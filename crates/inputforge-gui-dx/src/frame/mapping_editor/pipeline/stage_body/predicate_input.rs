//! Predicate input capture and unchanged app edit delivery.
use crate::frame::mapping_editor::undo_log::{LabelArgs, UndoKind, format_undo_label};
use crate::{
    capture::{CaptureFilter, Target},
    context::AppContext,
    frame::{MappingKey, mapping_list::source_label},
    mapping_edits::StageId,
    patterns::live_capture::{CAPTURE_PROMPT, use_capture},
};
use dioxus::prelude::*;
use inputforge_core::{action::Action, types::InputAddress};
pub(super) fn commit_condition(
    field: crate::mapping_edits::Field,
    mapping_key: &MappingKey,
    stage_id: &StageId,
    root_actions: &[Action],
) -> bool {
    let label = format_undo_label(
        UndoKind::StageEdit,
        LabelArgs {
            stage_name: Some("Conditional"),
            field: Some("condition"),
            ..LabelArgs::default()
        },
    );
    super::super::instruments::stage_dispatch::dispatch_field(
        root_actions,
        stage_id,
        field,
        mapping_key,
        label,
    )
}
#[component]
pub(super) fn PredicateInputRow(
    input: InputAddress,
    on_input_change: EventHandler<InputAddress>,
    target: Target,
) -> Element {
    let ctx = use_context::<AppContext>();
    let capture = use_capture(
        target,
        CaptureFilter::Any,
        move |new_addr| {
            on_input_change.call(new_addr);
        },
        |()| {},
    );
    let source = source_label::format(&input, &ctx.config.read());
    let on_rebind = move |_: MouseEvent| capture.start.call(());
    let on_cancel = move |_: MouseEvent| capture.cancel.call(());
    let is_listening = *capture.listening.read();
    rsx! {
        div {
            class: "if-predicate__input-row",
            crate::components::RebindField {
                source_label: source,
                listening: is_listening,
                unbound: matches!(input, InputAddress::Unbound),
                prompt: CAPTURE_PROMPT,
                action_label: "Rebind predicate input",
                cancel_label: "Cancel predicate input capture",
                onclick: on_rebind,
                oncancel: on_cancel,
            }
        }
    }
}
