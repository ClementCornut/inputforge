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
pub(crate) use inputforge_core::types::AxisPolarity;
pub(crate) use out_block::ExpandState;
use out_block::{DividerStrip, OutBlock};
pub(crate) use value_helpers::{read_axis_display, read_button_pressed, read_hat_direction};

/// CSS modifier class applied to readout rows whose value is held.
pub(super) const FROZEN_ROW_CLASS: &str = "if-editor__readout-row--frozen";

#[derive(Debug, Clone, PartialEq)]
struct ResetKey {
    primary: InputAddress,
    actions: Vec<Action>,
    outputs_len: usize,
}

impl ResetKey {
    fn new(primary: &InputAddress, actions: &[Action], outputs_len: usize) -> Self {
        Self {
            primary: primary.clone(),
            actions: actions.to_vec(),
            outputs_len,
        }
    }
}

/// Live IN/OUT readout section, mounted beneath the input field.
///
/// The analyzer receives one coherent state/config snapshot per render.
#[component]
pub(crate) fn LiveReadout(primary: InputAddress, actions: Vec<Action>) -> Element {
    let expand_state: Signal<ExpandState> = use_signal(ExpandState::default);

    rsx! {
        LiveReadoutInner { primary, actions, expand_state }
    }
}

#[cfg(test)]
#[component]
pub(crate) fn LiveReadoutTest(
    primary: InputAddress,
    actions: Vec<Action>,
    expand_state: Signal<ExpandState>,
) -> Element {
    rsx! {
        LiveReadoutInner { primary, actions, expand_state }
    }
}

#[component]
fn LiveReadoutInner(
    primary: InputAddress,
    actions: Vec<Action>,
    expand_state: Signal<ExpandState>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let model = {
        // Subscribe the analyzer owner to the live polling tick. IN/OUT rows
        // read `ctx.live` themselves, but expanded merge-chain previews are
        // derived by the analyzer from `ctx.state`; without this wake gate,
        // chain rows can stay on the value from the last structural render.
        let _live_tick = ctx.live.read();
        let state = ctx.state.read();
        let cfg = ctx.config.read();
        analyzer::analyze(&actions, &primary, &state, &cfg)
    };
    let engine_running = matches!(ctx.meta.read().engine_status, EngineStatus::Running);
    let outputs_len = model.outputs.len();
    let reset_key = ResetKey::new(&primary, &actions, outputs_len);

    let mut prev_reset_key: Signal<ResetKey> = use_signal(|| reset_key.clone());
    use_effect(use_reactive!(|(reset_key, outputs_len)| {
        let prev = prev_reset_key.read().clone();
        if prev != reset_key {
            expand_state.with_mut(|s| {
                s.per_output = vec![false; outputs_len];
            });
            prev_reset_key.set(reset_key.clone());
        }
    }));

    let model_for_in = model.clone();
    let model_for_out = model;

    rsx! {
        div { class: "if-editor__readout",
            InBlock { model: model_for_in }
            DividerStrip {}
            OutBlock { model: model_for_out, expand_state, engine_running }
        }
    }
}

#[cfg(test)]
mod tests {
    use inputforge_core::types::{DeviceId, InputId, OutputAddress, OutputId, VJoyAxis};

    use super::*;

    fn axis_addr(index: u8) -> InputAddress {
        InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index },
        }
    }

    fn map_x() -> Action {
        Action::MapToVJoy {
            output: OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::X },
            },
        }
    }

    #[test]
    fn reset_key_changes_for_different_primary_with_same_output_count() {
        let actions = vec![map_x()];

        assert_ne!(
            ResetKey::new(&axis_addr(0), &actions, 1),
            ResetKey::new(&axis_addr(1), &actions, 1)
        );
    }
}
