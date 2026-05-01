// Rust guideline compliant 2026-05-01

//! Variant-body dispatcher. Each `Action` variant has its own body
//! component; this module dispatches based on the variant. F10/F11/F14
//! replace only their variant's branch in `StageBody` and
//! `header_right_slot()`: the dispatcher itself, `StageHeader`, and the
//! `EditorState` provider are invariant.

use dioxus::prelude::*;

use inputforge_core::action::Action;

use crate::frame::MappingKey;
use crate::frame::mapping_editor::undo_log::StageId;

mod conditional;
mod invert;
mod map_to_keyboard;
mod map_to_vjoy;
mod merge_axis;
pub(crate) mod predicate;
// Placeholders for ResponseCurve, Deadzone, ChangeMode land in task 27.

#[component]
pub(crate) fn StageBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    action: Action,
    /// The mapping's outermost actions vec, threaded unchanged through
    /// every recursion. Bodies use this for tree mutators because `StageId`
    /// paths are root-relative. See Task 20 / Task 11.
    root_actions: Vec<Action>,
) -> Element {
    match &action {
        Action::Invert => rsx! { invert::InvertBody {} },
        Action::MapToVJoy { output } => rsx! {
            map_to_vjoy::MapToVJoyBody {
                mapping_key: mapping_key.clone(),
                stage_id: stage_id.clone(),
                output: output.clone(),
                root_actions: root_actions.clone(),
            }
        },
        Action::MapToKeyboard { key } => rsx! {
            map_to_keyboard::MapToKeyboardBody {
                mapping_key: mapping_key.clone(),
                stage_id: stage_id.clone(),
                combo: key.clone(),
                root_actions: root_actions.clone(),
            }
        },
        Action::MergeAxis {
            second_input,
            operation,
        } => rsx! {
            merge_axis::MergeAxisBody {
                mapping_key: mapping_key.clone(),
                stage_id: stage_id.clone(),
                second_input: second_input.clone(),
                operation: *operation,
                root_actions: root_actions.clone(),
            }
        },
        Action::Conditional {
            condition,
            if_true,
            if_false,
        } => rsx! {
            conditional::ConditionalBody {
                mapping_key: mapping_key.clone(),
                stage_id: stage_id.clone(),
                condition: condition.clone(),
                if_true: if_true.clone(),
                if_false: if_false.clone(),
                root_actions: root_actions.clone(),
                depth: u8::try_from(
                    stage_id
                        .0
                        .iter()
                        .filter(|s| {
                            matches!(
                                s,
                                crate::frame::mapping_editor::undo_log::StageIdSegment::IfTrue
                                    | crate::frame::mapping_editor::undo_log::StageIdSegment::IfFalse
                            )
                        })
                        .count(),
                )
                // Nesting depth cannot exceed MAX_CONDITION_DEPTH (32) in practice;
                // saturate on overflow so pathological inputs do not panic.
                .unwrap_or(u8::MAX),
            }
        },
        // Stub for other variants until task 27.
        _ => rsx! { div { class: "if-stage__body-stub", "(body coming soon)" } },
    }
}

/// Per-variant `right_slot` for `StageHeader`. Called from `Stage::render`.
/// F9-owned variants all return the default chevron-down SVG (the visual
/// affordance for expand/collapse). F10/F11/F14 override their variants
/// here to return their 28x14 preview thumbnail. Per spec lines 325-326,
/// the `IconButton`'s 32x32 hit area, `aria-expanded`, and `aria-controls`
/// remain invariant: only the visual content of the slot changes.
#[allow(
    clippy::match_same_arms,
    reason = "Named arms are the F10/F11/F14 override seam (spec lines 325-326). \
              Each will return a preview thumbnail when those tasks land; \
              collapsing into wildcard would erase the seam."
)]
pub(crate) fn header_right_slot(action: &Action, _expanded: bool) -> Element {
    match action {
        // F10 will override (preview = curve thumbnail):
        Action::ResponseCurve { .. } => default_chevron(),
        // F11 will override (preview = deadzone visualization):
        Action::Deadzone { .. } => default_chevron(),
        // F14 will override (preview = mode badge):
        Action::ChangeMode { .. } => default_chevron(),
        // F9-owned variants: chevron only.
        _ => default_chevron(),
    }
}

fn default_chevron() -> Element {
    rsx! {
        // Chevron-down SVG; rotation is CSS-driven via `aria-expanded`.
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            width: "16",
            height: "16",
            view_box: "0 0 16 16",
            fill: "currentColor",
            "aria-hidden": "true",
            path { d: "M3.5 5.5L8 10l4.5-4.5z" }
        }
    }
}
