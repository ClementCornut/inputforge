//! Existing keyboard edit delivery and display helpers.
use crate::frame::MappingKey;
use crate::frame::mapping_editor::pipeline::at_path;
use crate::frame::mapping_editor::undo_log::{LabelArgs, StageId, UndoKind, format_undo_label};
use dioxus::prelude::*;
use inputforge_core::action::{Action, Mapping, OutputBehavior};
use inputforge_core::types::KeyCombo;
pub(super) fn use_combo_sync(combo: &KeyCombo, capturing: bool, local_combo: Signal<KeyCombo>) {
    let previous = use_signal(|| combo.clone());
    consume_context::<crate::mapping_edits::Bindings>()
        .ui
        .combo_sync
        .call((combo.clone(), capturing, previous, local_combo));
}

pub(super) fn format_key_combo(combo: &KeyCombo) -> String {
    let mut parts: Vec<String> = combo
        .modifiers
        .iter()
        .map(|modifier| modifier.display_label())
        .map(str::to_owned)
        .collect();
    parts.push(combo.key.display_label().into_owned());
    parts.join(" + ")
}

#[allow(
    clippy::too_many_arguments,
    reason = "Capture commit handling needs the same dispatch context as the field handlers."
)]
pub(super) fn commit_capture_combo(
    new_combo: KeyCombo,
    behavior: OutputBehavior,
    mut local_combo: Signal<KeyCombo>,
    mapping_key: &MappingKey,
    stage_id: &StageId,
    root_actions: &[Action],
    before: &Mapping,
) {
    if dispatch_keyboard(
        new_combo.clone(),
        behavior,
        "key",
        mapping_key,
        stage_id,
        root_actions,
        before,
    ) {
        local_combo.set(new_combo);
    }
}

/// Wire field values and their display label to app-owned dispatch and history.
#[allow(
    clippy::too_many_arguments,
    reason = "All arguments are logically distinct; grouping them into a struct \
              would introduce indirection without simplifying call sites, which \
              each already own the captured values independently."
)]
pub(super) fn dispatch_keyboard(
    new_combo: KeyCombo,
    new_behavior: OutputBehavior,
    field_label: &'static str,
    mapping_key: &MappingKey,
    stage_id: &StageId,
    root_actions: &[Action],
    before: &Mapping,
) -> bool {
    let Some(Action::MapToKeyboard { key, behavior }) = at_path(&before.actions, stage_id) else {
        return false;
    };
    let (before_value, after_value) = if field_label == "key" {
        (format_key_combo(key), format_key_combo(&new_combo))
    } else {
        (format!("{behavior:?}"), format!("{new_behavior:?}"))
    };
    let label = format_undo_label(
        UndoKind::StageEdit,
        LabelArgs {
            stage_name: Some("Map to keyboard"),
            field: Some(field_label),
            before_after: Some((&before_value, &after_value)),
            ..LabelArgs::default()
        },
    );
    super::super::instruments::stage_dispatch::dispatch_field(
        root_actions,
        stage_id,
        if field_label == "key" {
            crate::mapping_edits::Field::Keyboard(new_combo)
        } else {
            crate::mapping_edits::Field::Behavior(new_behavior)
        },
        mapping_key,
        label,
    )
}
