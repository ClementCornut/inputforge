// Rust guideline compliant 2026-05-14

//! `PressGesture` body: thin wrapper around the shared [`GestureBody`].

use dioxus::prelude::*;

use inputforge_core::action::Action;

use crate::frame::MappingKey;
use crate::frame::mapping_editor::pipeline::stage_body::gesture_body::{GestureBody, GestureKind};
use crate::frame::mapping_editor::undo_log::StageId;

#[component]
pub(crate) fn PressGestureBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    threshold_ms: u64,
    fire_long_when_threshold_crossed: bool,
    short_press: Vec<Action>,
    long_press: Vec<Action>,
    root_actions: Vec<Action>,
    depth: u8,
) -> Element {
    rsx! {
        GestureBody {
            kind: GestureKind::Press,
            mapping_key,
            stage_id,
            threshold_ms,
            toggle: fire_long_when_threshold_crossed,
            branch_a_actions: short_press,
            branch_b_actions: long_press,
            root_actions,
            depth,
        }
    }
}
