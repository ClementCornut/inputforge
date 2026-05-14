// Rust guideline compliant 2026-05-14

//! `TapGesture` body: thin wrapper around the shared [`GestureBody`].

use dioxus::prelude::*;

use inputforge_core::action::Action;

use crate::frame::MappingKey;
use crate::frame::mapping_editor::pipeline::stage_body::gesture_body::{GestureBody, GestureKind};
use crate::frame::mapping_editor::undo_log::StageId;

#[component]
pub(crate) fn TapGestureBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    threshold_ms: u64,
    fire_single_immediately: bool,
    single_tap: Vec<Action>,
    double_tap: Vec<Action>,
    root_actions: Vec<Action>,
    depth: u8,
) -> Element {
    rsx! {
        GestureBody {
            kind: GestureKind::Tap,
            mapping_key,
            stage_id,
            threshold_ms,
            toggle: fire_single_immediately,
            branch_a_actions: single_tap,
            branch_b_actions: double_tap,
            root_actions,
            depth,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use inputforge_core::action::Action;
    use inputforge_core::engine::EngineCommand;
    use inputforge_core::types::InputAddress;

    use crate::frame::mapping_editor::pipeline::stage_body::gesture_body::{
        GestureKind, dispatch_gesture_edit_into, root_actions_with_state,
    };
    use crate::frame::mapping_editor::undo_log::{StageId, StageIdSegment, UndoLog};

    #[test]
    fn toggle_after_local_threshold_edit_records_current_threshold_in_undo() {
        let mapping_key = ("Default".to_owned(), InputAddress::Unbound);
        let stage_id = StageId(vec![StageIdSegment::Index(0)]);
        let stale_root_actions = vec![Action::TapGesture {
            threshold_ms: 250,
            fire_single_immediately: false,
            single_tap: Vec::new(),
            double_tap: Vec::new(),
        }];
        let actions_before_toggle = root_actions_with_state(
            GestureKind::Tap,
            &stale_root_actions,
            &stage_id,
            375,
            false,
            Vec::new(),
            Vec::new(),
        );
        let mut undo_log = UndoLog::default();
        let (tx, rx) = mpsc::channel();

        dispatch_gesture_edit_into(
            GestureKind::Tap,
            &mut undo_log,
            &mapping_key,
            &stage_id,
            &actions_before_toggle,
            None,
            &tx,
            "Tap gesture: single timing exclusive single/double -> immediate single".to_owned(),
            375,
            true,
            Vec::new(),
            Vec::new(),
        );

        let command = rx
            .try_recv()
            .expect("toggle dispatches current local state");
        assert!(matches!(
            command,
            EngineCommand::SetMapping {
                actions,
                ..
            } if matches!(
                actions.first(),
                Some(Action::TapGesture {
                    threshold_ms: 375,
                    fire_single_immediately: true,
                    ..
                })
            )
        ));
        let before = &undo_log.stacks[&mapping_key].undo[0].mapping_before.actions[0];
        assert!(matches!(
            before,
            Action::TapGesture {
                threshold_ms: 375,
                fire_single_immediately: false,
                ..
            }
        ));
    }
}
