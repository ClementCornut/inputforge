// Rust guideline compliant 2026-05-03

//! F11 deadzone body. See spec
//! `docs/superpowers/specs/2026-05-02-f11-deadzone-editor-design.md`.

pub(crate) mod interaction;
pub(crate) mod keyboard;
pub(crate) mod mutation;
pub(crate) mod rendering;
pub(crate) mod state;
pub(crate) mod thumbnail;
pub(crate) mod toolbar;

#[cfg(test)]
mod tests;

use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::processing::deadzone::DeadzoneConfig;

use crate::frame::MappingKey;
use crate::frame::mapping_editor::undo_log::StageId;

/// Body component for an `Action::Deadzone` pipeline stage. Stub for now;
/// fully wired up in Task 14 once interaction / keyboard / rendering land.
#[component]
pub(crate) fn DeadzoneBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    config: DeadzoneConfig,
    root_actions: Vec<Action>,
) -> Element {
    let _ = (mapping_key, stage_id, config, root_actions);
    rsx! { div { class: "if-deadzone", "deadzone body (under construction)" } }
}
