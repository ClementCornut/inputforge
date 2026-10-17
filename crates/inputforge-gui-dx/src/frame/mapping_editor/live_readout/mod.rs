//! Render the application's coherent completed readout model.
use crate::snapshots::MappingKey;
use crate::{context::AppContext, readouts::Expansion};
use dioxus::prelude::*;
use inputforge_core::action::Action;
mod in_block;
mod out_block;
mod out_chain;
mod predicate;
mod value_helpers;
pub(crate) use crate::readouts::ExpandState;
use in_block::InBlock;
pub(crate) use inputforge_core::types::AxisPolarity;
use out_block::{DividerStrip, OutBlock};
pub(crate) use value_helpers::{read_axis_display, read_button_pressed, read_hat_direction};
pub(super) const FROZEN_ROW_CLASS: &str = "if-editor__readout-row--frozen";
#[component]
pub(crate) fn LiveReadout(mapping_key: MappingKey, actions: Vec<Action>) -> Element {
    let expand_state = use_signal(ExpandState::default);
    rsx! {LiveReadoutInner {mapping_key,actions,expand_state}}
}
#[cfg(test)]
#[component]
pub(crate) fn LiveReadoutTest(
    mapping_key: MappingKey,
    actions: Vec<Action>,
    expand_state: Signal<ExpandState>,
) -> Element {
    rsx! {LiveReadoutInner {mapping_key,actions,expand_state}}
}
#[component]
fn LiveReadoutInner(
    mapping_key: MappingKey,
    actions: Vec<Action>,
    mut expand_state: Signal<ExpandState>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let bindings = use_context::<crate::mapping_edits::Bindings>().readouts;
    let prepared = ctx.live.read().readout.clone();
    let key = prepared.as_ref().map(|r| r.key.clone());
    let outputs = prepared.as_ref().map_or(0, |r| r.model.outputs.len());
    use_effect(use_reactive!(|(key, outputs)| {
        if let Some(key) = key {
            let next = bindings.expand.call(Expansion {
                state: expand_state.peek().clone(),
                key: key.clone(),
                outputs,
                toggle: None,
            });
            if *expand_state.peek() != next {
                expand_state.set(next);
            }
        }
    }));
    let Some(readout) =
        prepared.filter(|r| r.key.mapping == mapping_key && r.key.actions == actions)
    else {
        return rsx! {};
    };
    rsx! {
        div {class:"if-editor__readout",
            InBlock {model:readout.model.clone()}
            if outputs>0 {
                DividerStrip {}
                OutBlock {model:readout.model,values:readout.outputs,expand_state,engine_running:readout.running,readout_key:readout.key}
            }
        }
    }
}

#[cfg(test)]
mod events_tests;
