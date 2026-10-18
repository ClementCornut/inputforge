//! Predicate hat direction controls.
use super::HAT_VARIANTS;
use super::input::{PredicateInputRow, commit_condition};
use crate::{capture::Target, frame::MappingKey, mapping_edits::StageId};
use dioxus::prelude::*;
use inputforge_core::action::Action;
use inputforge_core::types::InputAddress;
#[component]
pub(super) fn HatDirections(
    input: InputAddress,
    mapping_key: MappingKey,
    stage_id: StageId,
    root_actions: Vec<Action>,
    capture_target: Target,
    directions: Vec<inputforge_core::types::HatDirection>,
) -> Element {
    let input_addr = input;
    let dirs = directions;
    let mk_in = mapping_key.clone();
    let sid_in = stage_id.clone();
    let root_in = root_actions.clone();
    rsx! {
        PredicateInputRow {
            target: capture_target.clone(),
            input: input_addr.clone(),
            on_input_change: move |new_addr: InputAddress| {
                commit_condition(
                    crate::mapping_edits::Field::ConditionInput(new_addr),
                    &mk_in.clone(),
                    &sid_in,
                    &root_in.clone(),
                );
            },
        }
        div {
            class: "if-hat-direction-grid",
            for (dir, label) in HAT_VARIANTS {
                {
                    let is_checked = dirs.contains(&dir);
                    let mk_cb = mapping_key.clone();
                    let sid_cb = stage_id.clone();
                    let root_cb = root_actions.clone();
                    rsx! {
                        crate::components::Checkbox {
                            class: "if-hat-direction-grid__cell",
                            checked: is_checked,
                            onchange: move |_| {
                                commit_condition(
                                    crate::mapping_edits::Field::ConditionDirection(dir),
                                    &mk_cb.clone(),
                                    &sid_cb,
                                    &root_cb.clone(),
                                );
                            },
                            "{label}"
                        }
                    }
                }
            }
        }
    }
}
