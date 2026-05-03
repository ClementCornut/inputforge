// Rust guideline compliant 2026-05-03

//! Shared `SetMapping` dispatch + undo bookkeeping for instrument bodies.
//! Generic over the new `Action` payload: F10 passes
//! `Action::ResponseCurve { curve }`, F11 passes `Action::Deadzone { config }`.
//!
//! Two-layer design: `dispatch_stage_edit_into` is the pure helper
//! (`&mut UndoLog`); `dispatch_stage_edit` is a Signal-wrapping wrapper for
//! Dioxus call sites. Tests target the helper so they do not need a
//! `VirtualDom` or a runtime-context `Signal`.

use std::sync::mpsc::Sender;

use dioxus::prelude::{Signal, WritableExt};

use inputforge_core::action::{Action, Mapping};
use inputforge_core::engine::EngineCommand;

use crate::frame::MappingKey;
use crate::frame::mapping_editor::pipeline::replace_at_path;
use crate::frame::mapping_editor::undo_log::{StageId, UndoKind, UndoLog};

/// Pure helper: takes `&mut UndoLog` directly. Test-friendly.
#[expect(
    clippy::too_many_arguments,
    reason = "F9 convention; matches dispatch_input_field_edit signature"
)]
pub(crate) fn dispatch_stage_edit_into(
    undo_log: &mut UndoLog,
    actions_before: &[Action],
    stage_id: &StageId,
    new_action: Action,
    mapping_key: &MappingKey,
    name: Option<String>,
    cmd_tx: &Sender<EngineCommand>,
    label: String,
) {
    let Some(new_actions) = replace_at_path(actions_before, stage_id, new_action) else {
        return;
    };
    let before = Mapping {
        input: mapping_key.1.clone(),
        mode: mapping_key.0.clone(),
        name: name.clone(),
        actions: actions_before.to_vec(),
    };
    if cmd_tx
        .send(EngineCommand::SetMapping {
            input: mapping_key.1.clone(),
            mode: mapping_key.0.clone(),
            name,
            actions: new_actions,
        })
        .is_err()
    {
        tracing::warn!(
            target: "instruments::stage_dispatch",
            action = "set_mapping_drop_offline",
            "dropped SetMapping command: receiver disconnected"
        );
        return;
    }
    undo_log.push_edit(mapping_key.clone(), before, UndoKind::StageEdit, label);
}

/// Signal-wrapping public form. Body call sites pass their `Signal<UndoLog>`
/// here; the wrapper takes the `write()` borrow once and threads it into the
/// helper.
#[expect(
    clippy::too_many_arguments,
    reason = "matches dispatch_stage_edit_into signature plus the Signal handle"
)]
pub(crate) fn dispatch_stage_edit(
    actions_before: &[Action],
    stage_id: &StageId,
    new_action: Action,
    mapping_key: &MappingKey,
    name: Option<String>,
    cmd_tx: &Sender<EngineCommand>,
    undo_log: &mut Signal<UndoLog>,
    label: String,
) {
    let mut guard = undo_log.write();
    dispatch_stage_edit_into(
        &mut guard,
        actions_before,
        stage_id,
        new_action,
        mapping_key,
        name,
        cmd_tx,
        label,
    );
}

pub(crate) fn dispatch_stage_edit_no_undo(
    actions_before: &[Action],
    stage_id: &StageId,
    new_action: Action,
    mapping_key: &MappingKey,
    name: Option<String>,
    cmd_tx: &Sender<EngineCommand>,
) {
    let Some(new_actions) = replace_at_path(actions_before, stage_id, new_action) else {
        return;
    };
    if cmd_tx
        .send(EngineCommand::SetMapping {
            input: mapping_key.1.clone(),
            mode: mapping_key.0.clone(),
            name,
            actions: new_actions,
        })
        .is_err()
    {
        tracing::warn!(
            target: "instruments::stage_dispatch",
            action = "set_mapping_no_undo_drop_offline",
            "dropped no-undo SetMapping command: receiver disconnected"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::processing::curves::ResponseCurve;
    use inputforge_core::types::InputAddress;
    use std::sync::mpsc;

    use crate::frame::mapping_editor::undo_log::StageIdSegment;

    fn linear_curve() -> ResponseCurve {
        ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (1.0, 1.0)], false)
            .expect("linear curve is valid")
    }

    #[test]
    fn dispatch_with_undo_sends_set_mapping_and_pushes_undo() {
        let (tx, rx) = mpsc::channel::<EngineCommand>();
        let key: MappingKey = ("default".into(), InputAddress::Unbound);
        let actions_before = vec![Action::ResponseCurve {
            curve: linear_curve(),
        }];
        let mut undo_log = UndoLog::default();
        let stage_id = StageId(vec![StageIdSegment::Index(0)]);

        dispatch_stage_edit_into(
            &mut undo_log,
            &actions_before,
            &stage_id,
            Action::ResponseCurve {
                curve: linear_curve(),
            },
            &key,
            None,
            &tx,
            "test: dispatch".to_owned(),
        );

        let cmd = rx.try_recv().expect("SetMapping should be sent");
        match cmd {
            EngineCommand::SetMapping { actions, .. } => {
                assert_eq!(actions.len(), 1);
            }
            _ => panic!("expected SetMapping"),
        }
        let entries = undo_log.stacks.get(&key).map_or(0, |h| h.undo.len());
        assert_eq!(entries, 1);
    }

    #[test]
    fn dispatch_no_undo_sends_command_but_skips_undo() {
        let (tx, rx) = mpsc::channel::<EngineCommand>();
        let key: MappingKey = ("default".into(), InputAddress::Unbound);
        let actions_before = vec![Action::ResponseCurve {
            curve: linear_curve(),
        }];
        let stage_id = StageId(vec![StageIdSegment::Index(0)]);

        dispatch_stage_edit_no_undo(
            &actions_before,
            &stage_id,
            Action::ResponseCurve {
                curve: linear_curve(),
            },
            &key,
            None,
            &tx,
        );

        assert!(matches!(
            rx.try_recv(),
            Ok(EngineCommand::SetMapping { .. })
        ));
    }

    #[test]
    fn dispatch_with_invalid_path_drops_silently() {
        let (tx, rx) = mpsc::channel::<EngineCommand>();
        let key: MappingKey = ("default".into(), InputAddress::Unbound);
        let actions_before: Vec<Action> = vec![];
        let mut undo_log = UndoLog::default();
        let stage_id = StageId(vec![StageIdSegment::Index(99)]);

        dispatch_stage_edit_into(
            &mut undo_log,
            &actions_before,
            &stage_id,
            Action::Invert,
            &key,
            None,
            &tx,
            "test: bad path".to_owned(),
        );

        assert!(rx.try_recv().is_err(), "no SetMapping for invalid path");
        let entries = undo_log.stacks.get(&key).map_or(0, |h| h.undo.len());
        assert_eq!(entries, 0);
    }
}
