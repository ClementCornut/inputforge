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
//!    button wired to app-owned axis capture.
//!
//! The app owns exclusive capture, target revalidation, confirmation and delivery.
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
//! App dispatch preserves the exact target mapping's optional name.
//!
//! # Dispatch-before-undo (Amendment 3)
//!
//! App history is recorded after successful dispatch. Closed channels create no history.

use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::types::{InputAddress, MergeOp};

use crate::capture::{CaptureFilter, Target};
use crate::components::{Select, SelectOption};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::undo_log::{LabelArgs, StageId, UndoKind, format_undo_label};
use crate::frame::mapping_list::source_label;
use crate::patterns::live_capture::{CAPTURE_PROMPT, rebind_composite_class, use_capture};

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

    use_context::<crate::mapping_edits::Bindings>()
        .ui
        .stage_hint
        .call(crate::mapping_edits::hints::Request {
            path: stage_id.clone(),
            input: crate::mapping_edits::hints::Input::Merge(
                second_input.clone(),
                mapping_key.1.clone(),
            ),
            hints: editor.malformed_hints,
            tags: None,
        });

    // --- Capture-and-commit for secondary input ---
    // On app capture completion, dispatch SetMapping with
    // the new secondary address, then push an undo entry.
    // Pre-clone second_input for the cap closure; the original is still
    // needed by the op-change handler and the final rsx! label below.
    let second_input_for_cap = second_input.clone();
    let key_for_cap = mapping_key.clone();
    let stage_id_for_cap = stage_id.clone();
    let root_for_cap = root_actions.clone();
    let cfg_for_cap = ctx.config;

    let capture = use_capture(
        Target {
            profile: ctx.meta.read().profile_id.clone(),
            mode: mapping_key.0.clone(),
            mapping: Some(mapping_key.1.clone()),
            name: ctx
                .config
                .read()
                .mappings
                .iter()
                .find(|mapping| mapping.input == mapping_key.1 && mapping.mode == mapping_key.0)
                .and_then(|mapping| mapping.name.clone()),
            // Reject invalid paths before the app confirms an ambiguous native control.
            actions: super::super::at_path(&root_actions, &stage_id)
                .filter(|action| matches!(action, Action::MergeAxis { .. }))
                .map(|_| root_actions.clone()),
            field: format!("secondary:{stage_id:?}"),
        },
        CaptureFilter::AxesOnly,
        move |new_addr| {
            // Amendment 2: read current name from snapshot so user-set names
            // are never silently cleared.
            let cfg = cfg_for_cap.read();
            // Build the secondary source labels for the undo entry before
            // dispatch so we can read the snapshot before it may update.
            let old_label = source_label::format(&second_input_for_cap, &cfg);
            let new_label = source_label::format(&new_addr, &cfg);
            drop(cfg);

            let label = format_undo_label(
                UndoKind::StageEdit,
                LabelArgs {
                    stage_name: Some("Merge axis"),
                    field: Some("secondary"),
                    before_after: Some((&old_label, &new_label)),
                    ..LabelArgs::default()
                },
            );
            super::instruments::stage_dispatch::dispatch_field(
                &root_for_cap,
                &stage_id_for_cap,
                crate::mapping_edits::Field::MergeInput(new_addr),
                &key_for_cap,
                label,
            );
        },
        |()| {},
    );
    let on_rebind = move |_: MouseEvent| capture.start.call(());
    let on_cancel_rebind = move |_: MouseEvent| capture.cancel.call(());

    // --- Operation picker change handler ---
    let key_for_op = mapping_key.clone();
    let stage_id_for_op = stage_id.clone();
    let root_for_op = root_actions.clone();
    let on_op_change = move |evt: FormEvent| {
        let new_op_str = evt.value();
        let before_op_str = op_to_str(operation);
        let new_op_str_ref = new_op_str.as_str();
        let label = format_undo_label(
            UndoKind::StageEdit,
            LabelArgs {
                stage_name: Some("Merge axis"),
                field: Some("operation"),
                before_after: Some((before_op_str, new_op_str_ref)),
                ..LabelArgs::default()
            },
        );
        super::instruments::stage_dispatch::dispatch_field(
            &root_for_op,
            &stage_id_for_op,
            crate::mapping_edits::Field::MergeOperation(new_op_str),
            &key_for_op,
            label,
        );
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
    let is_secondary_listening = *capture.listening.read();
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
                            "aria-label": "Cancel secondary input capture",
                            onclick: on_cancel_rebind,
                            "Cancel"
                        }
                    } else {
                        span { class: "if-rebind-composite__label", "{secondary_label}" }
                        button {
                            class: "if-rebind-composite__action",
                            r#type: "button",
                            "aria-label": "Rebind secondary input",
                            onclick: on_rebind,
                            "rebind"
                        }
                    }
                }
            }
        }
    }
}
