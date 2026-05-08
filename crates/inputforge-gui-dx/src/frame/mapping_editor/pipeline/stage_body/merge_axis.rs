// Rust guideline compliant 2026-05-02

//! `MergeAxis` body: operation picker + secondary input picker.
//!
//! # Controls
//!
//! Two rows:
//!
//! 1. **Operation picker** -- a [`Select`] with three options:
//!    `Bidirectional`, `Average`, `Maximum`. Dispatches on every change.
//! 2. **Secondary input picker** -- read-only source label + ghost `rebind`
//!    button that arms `LiveCapture::AxesOnly` via the consumer-flag pattern.
//!
//! # Capture-arming (consumer-flag pattern)
//!
//! `LiveCapture` is single-instance. Both the editor-frame `InputField` (Task
//! 16) and this component consume `LiveCapture.captured`. To prevent races each
//! consumer stores the `LiveCapture.session` it armed. Only the consumer whose
//! stored session matches the current session reacts when `captured` fires; a
//! newer session means another capture surface superseded it.
//!
//! # Malformed hints (Amendment 1, Task 9)
//!
//! At render time (NOT inside a `use_effect`, so SSR observes the write the
//! same frame the condition becomes invalid), the component picks at most
//! one hint to write into `editor.malformed_hints` for this `stage_id`,
//! using the priority:
//!
//! 1. `second_input.is_unbound()` -- writes [`HINT_MERGE_UNBOUND`]
//!    ("Bind a secondary input to complete this merge"). Top priority
//!    because no other validation is meaningful when no secondary input
//!    is bound yet.
//! 2. `second_input == mapping_key.1` (secondary equals primary) --
//!    writes "Secondary input must differ from primary".
//! 3. Otherwise -- clears the entry for this `stage_id`.
//!
//! # Name preservation (Amendment 2)
//!
//! Every dispatch reads the current name from `cfg.mapping_names` so
//! user-set names are never silently cleared.
//!
//! # Dispatch-before-undo (Amendment 3)
//!
//! `push_edit` is only called after `cmd_tx.send` succeeds. A closed channel
//! never generates phantom undo entries.

use dioxus::prelude::*;

use inputforge_core::action::{Action, Mapping};
use inputforge_core::engine::EngineCommand;
use inputforge_core::types::{InputAddress, MergeOp};

use crate::components::{Select, SelectOption};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::replace_at_path;
use crate::frame::mapping_editor::undo_log::{LabelArgs, StageId, UndoKind, format_undo_label};
use crate::frame::mapping_list::source_label;
use crate::patterns::live_capture::{
    CAPTURE_PROMPT, CaptureFilter, LiveCapture, is_current_capture_session, rebind_composite_class,
};

/// Malformed-hint message shown when the [`MergeAxisBody`] secondary input
/// is [`InputAddress::Unbound`].
///
/// Surfaced by [`MergeAxisBody`] as the highest-priority hint for the
/// stage. Pre-empts the existing "Secondary input must differ from primary"
/// hint because no other validation is meaningful when no secondary input
/// is bound yet.
const HINT_MERGE_UNBOUND: &str = "Bind a secondary input to complete this merge";

/// Convert a [`MergeOp`] to its stable string key used as the Select value.
const fn op_to_str(op: MergeOp) -> &'static str {
    match op {
        MergeOp::Bidirectional => "Bidirectional",
        MergeOp::Average => "Average",
        MergeOp::Maximum => "Maximum",
    }
}

/// Parse a Select option value string back to a [`MergeOp`]. Returns `None`
/// for unrecognised strings (should not occur under normal operation).
fn parse_op(s: &str) -> Option<MergeOp> {
    match s {
        "Bidirectional" => Some(MergeOp::Bidirectional),
        "Average" => Some(MergeOp::Average),
        "Maximum" => Some(MergeOp::Maximum),
        _ => None,
    }
}

/// All [`MergeOp`] variants in display order.
const ALL_OPS: [MergeOp; 3] = [MergeOp::Bidirectional, MergeOp::Average, MergeOp::Maximum];

/// `MergeAxis` body: operation picker and secondary-input source label with
/// rebind button.
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners (raw <button onclick>). Mirrors the \
              suppression used in header.rs and mapping_list/row.rs."
)]
pub(crate) fn MergeAxisBody(
    /// `(mode, InputAddress)` key for the mapping being edited. Named
    /// `mapping_key` to avoid collision with Dioxus's reserved `key` prop.
    mapping_key: MappingKey,
    stage_id: StageId,
    /// The secondary input address.
    second_input: InputAddress,
    /// The merge operation.
    operation: MergeOp,
    /// Full root-level action list for the mapping. Needed so that
    /// `replace_at_path` can build the new action tree on every edit.
    /// Named `root_actions` per the dispatcher convention (Amendment 5).
    root_actions: Vec<Action>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();
    let capture = use_context::<LiveCapture>();

    // --- Capture-session ownership for LiveCapture race prevention ---
    // Stores the LiveCapture session this component owns; a newer session
    // means another surface superseded this secondary-input capture.
    let mut armed_session: Signal<Option<u64>> = use_signal(|| None);

    // Task 9 + Amendment 1: malformed-hint write / clear on every render.
    // Priority: Unbound > secondary-equals-primary. Written during the
    // render phase (matching `MapToVJoyBody` / `MapToKeyboardBody`) so SSR
    // observes the hint and the user sees the guidance the same frame the
    // condition becomes invalid.
    //
    // REACTIVE-LOOP CONCERN (Task 40): both branches call malformed.write(),
    // marking the Signal dirty. No loop forms because the write value is
    // derived from the `second_input` and `mapping_key.1` props, neither of
    // which originates from malformed_hints. A read-then-compare guard
    // would be more explicit but is not required for correctness here.
    {
        let mut malformed = editor.malformed_hints;
        if second_input.is_unbound() {
            malformed
                .write()
                .insert(stage_id.clone(), HINT_MERGE_UNBOUND.to_owned());
        } else if second_input == mapping_key.1 {
            malformed.write().insert(
                stage_id.clone(),
                "Secondary input must differ from primary".to_owned(),
            );
        } else {
            malformed.write().remove(&stage_id);
        }
    }

    // --- Capture-and-commit for secondary input ---
    // When a LiveCapture fires and we are armed, dispatch SetMapping with
    // the new secondary address, then push an undo entry.
    // Pre-clone second_input for the cap closure; the original is still
    // needed by the op-change handler and the final rsx! label below.
    let second_input_for_cap = second_input.clone();
    let key_for_cap = mapping_key.clone();
    let stage_id_for_cap = stage_id.clone();
    let root_for_cap = root_actions.clone();
    let cmd_tx_cap = ctx.commands.clone();
    let mut undo_log_cap = editor.undo_log;
    let cfg_for_cap = ctx.config;
    let mut captured_mut = capture.captured;

    use_effect(move || {
        let captured_addr = captured_mut.read().clone();

        // Only act when this component owns the current capture session.
        if !is_current_capture_session(*armed_session.peek(), *capture.session.peek()) {
            return;
        }
        let Some(new_addr) = captured_addr else {
            return;
        };

        // Amendment 2: read current name from snapshot so user-set names
        // are never silently cleared.
        let cfg = cfg_for_cap.read();
        let current_name = cfg.mapping_names.get(&key_for_cap.1).cloned();

        // Build the secondary source labels for the undo entry before
        // dispatch so we can read the snapshot before it may update.
        let old_label = source_label::format(&second_input_for_cap, &cfg);
        let new_label = source_label::format(&new_addr, &cfg);
        drop(cfg);

        let new_action = Action::MergeAxis {
            second_input: new_addr.clone(),
            operation,
        };
        let Some(new_actions) = replace_at_path(&root_for_cap, &stage_id_for_cap, new_action)
        else {
            // Invalid path: skip edit so no phantom undo entry is created.
            armed_session.set(None);
            captured_mut.set(None);
            return;
        };

        let before = Mapping {
            input: key_for_cap.1.clone(),
            mode: key_for_cap.0.clone(),
            name: current_name.clone(),
            actions: root_for_cap.clone(),
        };

        // Amendment 3: dispatch first; push undo only on success.
        if cmd_tx_cap
            .send(EngineCommand::SetMapping {
                input: key_for_cap.1.clone(),
                mode: key_for_cap.0.clone(),
                name: current_name,
                actions: new_actions,
            })
            .is_err()
        {
            tracing::warn!(
                target: "f9::mapping_editor",
                action = "merge_axis_secondary_drop_offline",
                "secondary capture dropped: engine channel disconnected"
            );
            armed_session.set(None);
            captured_mut.set(None);
            return;
        }

        let label = format_undo_label(
            UndoKind::StageEdit,
            LabelArgs {
                stage_name: Some("Merge axis"),
                field: Some("secondary"),
                before_after: Some((&old_label, &new_label)),
                ..LabelArgs::default()
            },
        );
        undo_log_cap
            .write()
            .push_edit(key_for_cap.clone(), before, UndoKind::StageEdit, label);

        // Disarm and clear so stale effects do not re-fire.
        armed_session.set(None);
        captured_mut.set(None);
    });

    // --- Rebind button click handler ---
    // Set armed flag BEFORE calling start so the captured effect cannot
    // fire before the flag is true (effects run synchronously in SSR).
    let start_cb = capture.start;
    let on_rebind = move |_: MouseEvent| {
        start_cb.call(CaptureFilter::AxesOnly);
        armed_session.set(Some(*capture.session.peek()));
    };
    let cancel_cb = capture.cancel;
    let on_cancel_rebind = move |_: MouseEvent| {
        cancel_cb.call(());
        armed_session.set(None);
    };

    // External-cancel / supersede watcher. `active=false` handles Esc/cancel;
    // `session` mismatch handles another capture surface starting while the
    // global capture remains active.
    use_effect(move || {
        let active_now = *capture.active.read();
        let current_session = *capture.session.read();
        let owned_session = *armed_session.peek();
        if owned_session.is_none() {
            return;
        }
        if capture.captured.peek().is_some() {
            return;
        }
        if active_now && is_current_capture_session(owned_session, current_session) {
            return;
        }
        armed_session.set(None);
    });

    // --- Operation picker change handler ---
    let key_for_op = mapping_key.clone();
    let stage_id_for_op = stage_id.clone();
    let root_for_op = root_actions.clone();
    let second_for_op = second_input.clone();
    let cmd_tx_op = ctx.commands.clone();
    let mut undo_log_op = editor.undo_log;

    let on_op_change = move |evt: FormEvent| {
        let new_op_str = evt.value();
        let Some(new_op) = parse_op(&new_op_str) else {
            return;
        };
        if new_op == operation {
            return;
        }

        // Amendment 2: name from snapshot.
        let cfg = ctx.config.read();
        let current_name = cfg.mapping_names.get(&key_for_op.1).cloned();
        drop(cfg);

        let new_action = Action::MergeAxis {
            second_input: second_for_op.clone(),
            operation: new_op,
        };
        let Some(new_actions) = replace_at_path(&root_for_op, &stage_id_for_op, new_action) else {
            return;
        };

        let before = Mapping {
            input: key_for_op.1.clone(),
            mode: key_for_op.0.clone(),
            name: current_name.clone(),
            actions: root_for_op.clone(),
        };

        let before_op_str = op_to_str(operation);
        let new_op_str_ref = op_to_str(new_op);

        // Amendment 3: dispatch first; push undo only on success.
        if cmd_tx_op
            .send(EngineCommand::SetMapping {
                input: key_for_op.1.clone(),
                mode: key_for_op.0.clone(),
                name: current_name,
                actions: new_actions,
            })
            .is_err()
        {
            tracing::warn!(
                target: "f9::mapping_editor",
                action = "merge_axis_op_drop_offline",
                "operation change dropped: engine channel disconnected"
            );
            return;
        }

        let label = format_undo_label(
            UndoKind::StageEdit,
            LabelArgs {
                stage_name: Some("Merge axis"),
                field: Some("operation"),
                before_after: Some((before_op_str, new_op_str_ref)),
                ..LabelArgs::default()
            },
        );
        undo_log_op
            .write()
            .push_edit(key_for_op.clone(), before, UndoKind::StageEdit, label);
    };

    // --- Build Select value and options ---
    // Sync the Select's local Signal to the prop on every render.
    // `use_signal` is initialized once per component instance, so without
    // this peek-and-set the dropdown would keep displaying the original
    // operation after a SetMapping echo updated the prop.
    let op_str = op_to_str(operation).to_owned();
    let mut op_value: Signal<String> = use_signal(|| op_str.clone());
    if *op_value.peek() != op_str {
        op_value.set(op_str);
    }
    let op_options: Vec<SelectOption> = ALL_OPS
        .iter()
        .map(|&op| {
            let s = op_to_str(op).to_owned();
            SelectOption {
                value: s.clone(),
                label: s,
                disabled: false,
                class: None,
            }
        })
        .collect();

    // --- Secondary source label ---
    let secondary_label = source_label::format(&second_input, &ctx.config.read());

    // Compose the rebind-composite class so the placeholder label renders
    // muted/italic when the secondary input is `Unbound`, and flips to the
    // listening treatment while this component owns a capture.
    let is_secondary_listening =
        is_current_capture_session(*armed_session.read(), *capture.session.read());
    let composite_class = rebind_composite_class(&second_input, is_secondary_listening);

    rsx! {
        div { class: "if-stage__body-merge-axis",
            div { class: "if-stage__body-field",
                label { class: "if-stage__body-label", "Operation" }
                Select {
                    value: op_value,
                    options: op_options,
                    onchange: on_op_change,
                }
            }
            div { class: "if-stage__body-field",
                label { class: "if-stage__body-label", "Secondary input" }
                div { class: "{composite_class}",
                    if is_secondary_listening {
                        span {
                            class: "if-rebind-composite__listening",
                            role: "status",
                            "aria-live": "polite",
                            "{CAPTURE_PROMPT}"
                        }
                        button {
                            class: "if-rebind-composite__action",
                            r#type: "button",
                            onclick: on_cancel_rebind,
                            "Cancel"
                        }
                    } else {
                        span { class: "if-rebind-composite__label", "{secondary_label}" }
                        button {
                            class: "if-rebind-composite__action",
                            r#type: "button",
                            onclick: on_rebind,
                            "rebind"
                        }
                    }
                }
            }
        }
    }
}
