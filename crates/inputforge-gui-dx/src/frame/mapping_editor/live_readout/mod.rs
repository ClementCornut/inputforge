// Rust guideline compliant 2026-05-04

//! Live readout orchestration for full action-tree analysis.
//!
//! The readout surfaces every pipeline input, every condition predicate,
//! and every terminal output with per-OUT expandable causal chains.

use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::state::EngineStatus;
use inputforge_core::types::InputAddress;

use crate::context::AppContext;

mod analyzer;
mod in_block;
mod out_block;
mod out_chain;
mod predicate;
mod value_helpers;

use in_block::InBlock;
use out_block::{DividerStrip, ExpandState, OutBlock};

/// CSS modifier class applied to readout rows whose value is held.
pub(super) const FROZEN_ROW_CLASS: &str = "if-editor__readout-row--frozen";

/// Live IN/OUT readout section, mounted beneath the input field.
///
/// The analyzer receives one coherent state/config snapshot per render.
#[component]
pub(crate) fn LiveReadout(primary: InputAddress, actions: Vec<Action>) -> Element {
    let ctx = use_context::<AppContext>();
    let model = {
        let state = ctx.state.read();
        let cfg = ctx.config.read();
        analyzer::analyze(&actions, &primary, &state, &cfg)
    };
    let engine_running = matches!(ctx.meta.read().engine_status, EngineStatus::Running);
    let outputs_len = model.outputs.len();

    let mut expand_state: Signal<ExpandState> = use_signal(ExpandState::default);
    let mut prev_outputs_len: Signal<usize> = use_signal(|| outputs_len);
    use_effect(move || {
        let prev = *prev_outputs_len.read();
        if prev != outputs_len {
            expand_state.with_mut(|s| {
                s.per_output = vec![false; outputs_len];
                s.expand_all = false;
            });
            prev_outputs_len.set(outputs_len);
        }
    });

    let model_for_in = model.clone();
    let model_for_divider = model.clone();
    let model_for_out = model;

    rsx! {
        div { class: "if-editor__readout",
            InBlock { model: model_for_in }
            DividerStrip { model: model_for_divider, expand_state }
            OutBlock { model: model_for_out, expand_state, engine_running }
        }
    }
}

#[cfg(test)]
mod tests {}
