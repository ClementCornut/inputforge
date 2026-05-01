// Rust guideline compliant 2026-05-01

//! Predicate editor stub. Task 26b replaces this with the real per-condition editor.

use dioxus::prelude::*;

use inputforge_core::action::{Action, Condition};

use crate::frame::MappingKey;
use crate::frame::mapping_editor::undo_log::StageId;

/// Minimal predicate display stub rendered by [`super::conditional::ConditionalBody`].
///
/// Shows the condition kind as a monospace badge with a placeholder note.
/// Task 26b replaces this component with a full per-condition editor widget.
#[component]
pub(crate) fn PredicateEditor(
    /// Mapping key, forwarded to Task 26b's real editor.
    mapping_key: MappingKey,
    /// Stage ID of the enclosing `Conditional`, forwarded to Task 26b.
    stage_id: StageId,
    /// The condition to display.
    condition: Condition,
    /// The `if_true` branch, forwarded for Task 26b context.
    if_true: Vec<Action>,
    /// The `if_false` branch, forwarded for Task 26b context.
    if_false: Option<Vec<Action>>,
    /// Full root-level actions vec, threaded unchanged per Task 20 convention.
    root_actions: Vec<Action>,
) -> Element {
    // Suppress unused-variable warnings: Task 26b will consume all of these.
    let _ = (mapping_key, stage_id, if_true, if_false, root_actions);

    let condition_label = match &condition {
        Condition::ButtonPressed { .. } => "ButtonPressed",
        Condition::ButtonReleased { .. } => "ButtonReleased",
        Condition::AxisInRange { .. } => "AxisInRange",
        Condition::HatDirection { .. } => "HatDirection",
        Condition::All { .. } => "All",
        Condition::Any { .. } => "Any",
        Condition::Not { .. } => "Not",
    };

    rsx! {
        div { class: "if-stage__predicate-stub",
            "Predicate: "
            span { class: "if-stage__predicate-kind", "{condition_label}" }
            " (editor lands in Task 26b)"
        }
    }
}
