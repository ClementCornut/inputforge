//! Leaf and nested predicate composition.
use super::input::{PredicateInputRow, commit_condition};
use super::{PredicateEditor, hat::HatDirections, range::AxisRange};
use crate::{capture::Target, frame::MappingKey, mapping_edits::StageId};
use dioxus::prelude::*;
use inputforge_core::action::{Action, Condition};
use inputforge_core::types::InputAddress;
#[component]
pub(super) fn Operand(
    mapping_key: MappingKey,
    stage_id: StageId,
    condition: Condition,
    if_true: Vec<Action>,
    if_false: Vec<Action>,
    root_actions: Vec<Action>,
    capture_target: Target,
    min_sig: Signal<f64>,
    max_sig: Signal<f64>,
    field_prefix: String,
) -> Element {
    match &condition {
        Condition::ButtonPressed { input } | Condition::ButtonReleased { input } => {
            let input_clone = input.clone();
            let mk = mapping_key.clone();
            let sid = stage_id.clone();
            let root = root_actions.clone();
            rsx! {
                PredicateInputRow {
                    target: capture_target.clone(),
                    input: input_clone,
                    on_input_change: move |new_addr: InputAddress| {
                        commit_condition(
                            crate::mapping_edits::Field::ConditionInput(new_addr),
                            &mk.clone(),
                            &sid,
                            &root.clone(),
                        );
                    },
                }
            }
        }
        Condition::AxisInRange { input, .. } => {
            rsx! {
                AxisRange {
                    input: input.clone(),
                    mapping_key: mapping_key.clone(),
                    stage_id: stage_id.clone(),
                    root_actions: root_actions.clone(),
                    capture_target: capture_target.clone(),
                    min_sig,
                    max_sig,
                    field_prefix: field_prefix.clone(),
                }
            }
        }
        Condition::HatDirection { input, directions } => {
            rsx! {
                HatDirections {
                    input: input.clone(),
                    mapping_key: mapping_key.clone(),
                    stage_id: stage_id.clone(),
                    root_actions: root_actions.clone(),
                    capture_target: capture_target.clone(),
                    directions: directions.clone(),
                }
            }
        }
        Condition::All { conditions } | Condition::Any { conditions } => {
            let is_all = matches!(&condition, Condition::All { .. });
            let sub_conditions = conditions.clone();
            let sub_count = sub_conditions.len();
            rsx! {
                div {
                    class: "if-predicate__nested-list",
                    for (idx, sub) in sub_conditions.iter().enumerate() {
                        {
                            let sub_cond = sub.clone();
                            let mk = mapping_key.clone();
                            let sid = stage_id.clone();
                            let root = root_actions.clone();
                            let it = if_true.clone();
                            let iff = if_false.clone();
                            rsx! {
                                div {
                                    class: "if-predicate__nested-item",
                                    key: "{idx}",
                                    PredicateEditor {
                                        mapping_key: mk,
                                        stage_id: sid,
                                        condition: sub_cond,
                                        if_true: it,
                                        if_false: iff,
                                        root_actions: root,
                                    }
                                }
                            }
                        }
                    }
                    if sub_count == 0 {
                        span {
                            class: "if-predicate__nested-empty",
                            if is_all {
                                "No conditions (vacuously true)"
                            } else {
                                "No conditions (vacuously false)"
                            }
                        }
                    }
                }
            }
        }
        Condition::Not { condition: inner } => {
            let inner_cond = *inner.clone();
            let mk = mapping_key.clone();
            let sid = stage_id.clone();
            let root = root_actions.clone();
            let it = if_true.clone();
            let iff = if_false.clone();
            rsx! {
                div {
                    class: "if-predicate__nested-list",
                    div {
                        class: "if-predicate__nested-item",
                        PredicateEditor {
                            mapping_key: mk,
                            stage_id: sid,
                            condition: inner_cond,
                            if_true: it,
                            if_false: iff,
                            root_actions: root,
                        }
                    }
                }
            }
        }
    }
}
