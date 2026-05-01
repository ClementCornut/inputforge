// Rust guideline compliant 2026-05-01

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
//! consumer owns a local `Signal<bool>` `is_armed_consumer` flag. Only the
//! consumer that set its flag to `true` reacts when `captured` fires; all
//! others see `false` and skip. This is the project-wide convention; deviating
//! from it causes the self-fire bug documented in Task 16.
//!
//! # Malformed hints (Amendment 1)
//!
//! On every render via `use_effect`, the component checks whether
//! `second_input == mapping_key.1` (secondary equals primary). When true it
//! writes a hint to `editor.malformed_hints`; otherwise it clears the entry
//! for this `stage_id`.
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

use crate::components::Select;
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::replace_at_path;
use crate::frame::mapping_editor::undo_log::{LabelArgs, StageId, UndoKind, format_undo_label};
use crate::frame::mapping_list::source_label;
use crate::patterns::live_capture::{CaptureFilter, LiveCapture};

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

    // --- Consumer-flag for LiveCapture race prevention ---
    // Set to true only while THIS component is waiting for a secondary-input
    // capture; cleared when capture arrives or is cancelled.
    let mut is_armed_consumer: Signal<bool> = use_signal(|| false);

    // Amendment 1: malformed-hint write / clear on every render.
    // When secondary == primary, the merge is a no-op and the stage is
    // flagged as malformed per spec lines 587-589.
    //
    // REACTIVE-LOOP CONCERN (Task 40): both branches call malformed.write(),
    // which marks the Signal dirty and could re-trigger effects that read
    // malformed_hints. In practice this is safe because use_effect captures
    // the hint values at call time (secondary_for_hint / primary_addr are
    // plain values, not Signal reads), so the effect does not re-subscribe
    // to malformed_hints and therefore cannot form a loop. A read-then-compare
    // guard would be more explicit but is not required for correctness here.
    let primary_addr = mapping_key.1.clone();
    let secondary_for_hint = second_input.clone();
    let stage_id_for_hint = stage_id.clone();
    let mut malformed = editor.malformed_hints;
    use_effect(move || {
        let mut map = malformed.write();
        if secondary_for_hint == primary_addr {
            map.insert(
                stage_id_for_hint.clone(),
                "Secondary input must differ from primary".to_owned(),
            );
        } else {
            map.remove(&stage_id_for_hint);
        }
    });

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

        // Only act when this component armed the capture.
        if !*is_armed_consumer.read() {
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
            is_armed_consumer.set(false);
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
            is_armed_consumer.set(false);
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
        is_armed_consumer.set(false);
        captured_mut.set(None);
    });

    // --- Rebind button click handler ---
    // Set armed flag BEFORE calling start so the captured effect cannot
    // fire before the flag is true (effects run synchronously in SSR).
    let start_cb = capture.start;
    let on_rebind = move |_: MouseEvent| {
        is_armed_consumer.set(true);
        start_cb.call(CaptureFilter::AxesOnly);
    };

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
    let op_value: Signal<String> = use_signal(|| op_to_str(operation).to_owned());
    let op_options: Vec<(String, String)> = ALL_OPS
        .iter()
        .map(|&op| {
            let s = op_to_str(op).to_owned();
            (s.clone(), s)
        })
        .collect();

    // --- Secondary source label ---
    let secondary_label = source_label::format(&second_input, &ctx.config.read());

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
                div { class: "if-rebind-composite",
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
