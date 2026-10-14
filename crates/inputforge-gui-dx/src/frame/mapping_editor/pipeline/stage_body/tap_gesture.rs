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
