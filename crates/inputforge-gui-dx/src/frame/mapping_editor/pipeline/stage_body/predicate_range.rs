//! Predicate axis range fields.
use super::input::{PredicateInputRow, commit_condition};
use crate::components::NumberInput;
use crate::{capture::Target, frame::MappingKey, mapping_edits::StageId};
use dioxus::prelude::*;
use inputforge_core::action::Action;
use inputforge_core::types::InputAddress;
#[component]
pub(super) fn AxisRange(
    input: InputAddress,
    mapping_key: MappingKey,
    stage_id: StageId,
    root_actions: Vec<Action>,
    capture_target: Target,
    min_sig: Signal<f64>,
    max_sig: Signal<f64>,
    field_prefix: String,
) -> Element {
    let mut min_sig = min_sig;
    let mut max_sig = max_sig;
    let input_addr = input;
    let mk_in = mapping_key.clone();
    let sid_in = stage_id.clone();
    let root_in = root_actions.clone();
    let addr_in = input_addr.clone();
    let mk_min = mapping_key.clone();
    let sid_min = stage_id.clone();
    let root_min = root_actions.clone();
    let mk_max = mapping_key.clone();
    let sid_max = stage_id.clone();
    let root_max = root_actions.clone();
    rsx! {
        PredicateInputRow {
            target: capture_target.clone(),
            input: addr_in,
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
            class: "if-predicate__range-row",
            crate::components::Label {
                class: "if-predicate__range-label",
                for_id: format!("{field_prefix}-min"),
                "min"
            }
            NumberInput {
                aria_label: "Minimum axis value",
                id: format!("{field_prefix}-min"),
                value: min_sig,
                min: -1.0,
                max: 1.0,
                step: 0.01,
                precision: 2_usize,
                oninput: move |evt: FormEvent| {
                    if let Some(v) = consume_context::<crate::mapping_edits::Bindings>()
                        .number
                        .call(evt.value())
                    {
                        min_sig.set(v);
                    }
                },
                onstep: move |v: f64| {
                    if commit_condition(
                        crate::mapping_edits::Field::ConditionMin(v.to_string()),
                        &mk_min.clone(),
                        &sid_min,
                        &root_min.clone(),
                    ) {
                        min_sig.set(v);
                    }
                },
            }
            crate::components::Label {
                class: "if-predicate__range-label",
                for_id: format!("{field_prefix}-max"),
                "max"
            }
            NumberInput {
                aria_label: "Maximum axis value",
                id: format!("{field_prefix}-max"),
                value: max_sig,
                min: -1.0,
                max: 1.0,
                step: 0.01,
                precision: 2_usize,
                oninput: move |evt: FormEvent| {
                    if let Some(v) = consume_context::<crate::mapping_edits::Bindings>()
                        .number
                        .call(evt.value())
                    {
                        max_sig.set(v);
                    }
                },
                onstep: move |v: f64| {
                    if commit_condition(
                        crate::mapping_edits::Field::ConditionMax(v.to_string()),
                        &mk_max.clone(),
                        &sid_max,
                        &root_max.clone(),
                    ) {
                        max_sig.set(v);
                    }
                },
            }
        }
    }
}
