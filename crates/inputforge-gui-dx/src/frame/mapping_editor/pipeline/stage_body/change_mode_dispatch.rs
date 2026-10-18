//! Existing mode-change dispatch and strategy labels.
use crate::frame::MappingKey;
use crate::frame::mapping_editor::pipeline::stage_body::instruments::stage_dispatch::dispatch_field;
use crate::frame::mapping_editor::undo_log::{LabelArgs, StageId, UndoKind, format_undo_label};
use inputforge_core::action::{Action, ModeChangeStrategy};
pub(crate) const TOOLTIP_HOLD_NOT_BUTTON: &str = "Hold requires a button input.";

/// Which pill the user clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StrategyTarget {
    Set,
    Hold,
}

/// Wire a Set/Hold edit and its display label to the app controller.
///
/// The app preserves the effective target mode across strategy changes.
pub(crate) fn dispatch_strategy_change(
    target: StrategyTarget,
    mapping_key: &MappingKey,
    stage_id: &StageId,
    root_actions: &[Action],
) -> Option<String> {
    let (before, after) = match target {
        StrategyTarget::Set => ("Hold", "Set"),
        StrategyTarget::Hold => ("Set", "Hold"),
    };
    let label = format_undo_label(
        UndoKind::StageEdit,
        LabelArgs {
            stage_name: Some("Change mode"),
            field: Some("strategy"),
            before_after: Some((before, after)),
            ..LabelArgs::default()
        },
    );
    let delivered = dispatch_field(
        root_actions,
        stage_id,
        crate::mapping_edits::Field::ModeHold(matches!(target, StrategyTarget::Hold)),
        mapping_key,
        label.clone(),
    );
    delivered.then_some(label)
}

/// Wire a target-mode change to the app controller.
/// Returns the formatted undo label that was committed (`None` when the
/// new mode equals the current mode, i.e. picking the same option twice).
///
/// The `<unset>` token in the undo label preserves the empty-mode signal
/// to the user (prior strategy with `mode: ""` becomes `target <unset> -> X`).
pub(crate) fn dispatch_target_change(
    new_mode: &str,
    current_strategy: &ModeChangeStrategy,
    mapping_key: &MappingKey,
    stage_id: &StageId,
    root_actions: &[Action],
) -> Option<String> {
    let mode_before = match current_strategy {
        ModeChangeStrategy::SwitchTo { mode } | ModeChangeStrategy::Temporary { mode } => {
            mode.as_str()
        }
    };
    let before_label = if mode_before.is_empty() {
        "<unset>"
    } else {
        mode_before
    };
    let label = format_undo_label(
        UndoKind::StageEdit,
        LabelArgs {
            stage_name: Some("Change mode"),
            field: Some("target"),
            before_after: Some((before_label, new_mode)),
            ..LabelArgs::default()
        },
    );
    let delivered = dispatch_field(
        root_actions,
        stage_id,
        crate::mapping_edits::Field::ModeTarget(new_mode.into()),
        mapping_key,
        label.clone(),
    );
    delivered.then_some(label)
}
