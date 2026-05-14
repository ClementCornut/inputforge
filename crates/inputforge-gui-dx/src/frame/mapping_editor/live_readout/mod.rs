// Rust guideline compliant 2026-05-04

//! Live readout orchestration for full action-tree analysis.
//!
//! The readout surfaces every pipeline input, every condition predicate,
//! and every terminal output with per-OUT expandable causal chains.

use std::time::Instant;

use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::pipeline::OutputOwner;
use inputforge_core::state::{EngineStatus, OUTPUT_ACTIVITY_PREVIEW_LATCH};
use inputforge_core::types::InputAddress;

use crate::context::{AppContext, OutputActivitySnapshot};

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

#[derive(Debug, Clone, PartialEq)]
struct LatchedOutputActivity {
    owner: OutputOwner,
    value: inputforge_core::state::OutputActivityValue,
    expires_at: Instant,
}

fn apply_output_activity_latch(
    latched: &mut Vec<LatchedOutputActivity>,
    observed: &[OutputActivitySnapshot],
    now: Instant,
) -> Vec<OutputActivitySnapshot> {
    latched.retain(|entry| now <= entry.expires_at);

    for observed_entry in observed {
        if let Some(latched_entry) = latched
            .iter_mut()
            .find(|entry| entry.owner == observed_entry.owner)
        {
            latched_entry.value = observed_entry.value;
        } else {
            latched.push(LatchedOutputActivity {
                owner: observed_entry.owner.clone(),
                value: observed_entry.value,
                expires_at: now + OUTPUT_ACTIVITY_PREVIEW_LATCH,
            });
        }
    }

    let mut output_activity = observed.to_vec();
    for latched_entry in latched {
        if !observed
            .iter()
            .any(|observed_entry| observed_entry.owner == latched_entry.owner)
        {
            output_activity.push(OutputActivitySnapshot {
                owner: latched_entry.owner.clone(),
                value: latched_entry.value,
            });
        }
    }
    output_activity
}

fn next_latch_expiry(latched: &[LatchedOutputActivity]) -> Option<Instant> {
    latched.iter().map(|entry| entry.expires_at).min()
}

fn schedule_output_activity_latch_prune(
    mut latch: Signal<Vec<LatchedOutputActivity>>,
    mut scheduled_expiry: Signal<Option<Instant>>,
    expires_at: Instant,
) {
    spawn(async move {
        tokio::time::sleep(expires_at.saturating_duration_since(Instant::now())).await;
        let now = Instant::now();
        latch.with_mut(|entries| entries.retain(|entry| now <= entry.expires_at));
        let next_expiry = next_latch_expiry(&latch.read());
        scheduled_expiry.set(next_expiry);
        if let Some(next_expiry) = next_expiry {
            schedule_output_activity_latch_prune(latch, scheduled_expiry, next_expiry);
        }
    });
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
    let mut output_activity_latch: Signal<Vec<LatchedOutputActivity>> = use_signal(Vec::new);
    let mut output_activity_expiry: Signal<Option<Instant>> = use_signal(|| None);
    let model = {
        // Subscribe the analyzer owner to the live polling tick. IN/OUT rows
        // read `ctx.live` themselves, but expanded merge-chain previews are
        // derived by the analyzer from `ctx.state`; without this wake gate,
        // chain rows can stay on the value from the last structural render.
        let live = ctx.live.read();
        let observed_activity = live.output_activity.clone();
        drop(live);
        let now = Instant::now();
        let output_activity = output_activity_latch
            .with_mut(|latched| apply_output_activity_latch(latched, &observed_activity, now));
        let next_expiry = output_activity_latch.with(|latched| next_latch_expiry(latched));
        if *output_activity_expiry.peek() != next_expiry {
            output_activity_expiry.set(next_expiry);
            if let Some(next_expiry) = next_expiry {
                schedule_output_activity_latch_prune(
                    output_activity_latch,
                    output_activity_expiry,
                    next_expiry,
                );
            }
        }
        let state = ctx.state.read();
        let cfg = ctx.config.read();
        analyzer::analyze(&actions, &primary, &state, &cfg, &output_activity)
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
            if outputs_len > 0 {
                DividerStrip {}
                OutBlock { model: model_for_out, expand_state, engine_running }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use inputforge_core::action::{ActionBranch, OutputBehavior};
    use inputforge_core::pipeline::{ActionPathSegment, OutputDestination};
    use inputforge_core::state::OutputActivityValue;
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

    fn activity_owner() -> OutputOwner {
        OutputOwner {
            profile: "memory-profile".to_owned(),
            mode: "Default".to_owned(),
            input: axis_addr(0),
            action_path: vec![
                ActionPathSegment::Index(0),
                ActionPathSegment::Branch(ActionBranch::TapSingle),
                ActionPathSegment::Index(0),
            ],
            destination: OutputDestination::VJoy(OutputAddress {
                device: 1,
                output: OutputId::Button { id: 1 },
            }),
            behavior: OutputBehavior::Hold,
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

    #[test]
    fn observed_output_activity_latch_keeps_activity_after_snapshot_drops_it() {
        let owner = activity_owner();
        let now = Instant::now();
        let observed = vec![OutputActivitySnapshot {
            owner: owner.clone(),
            value: OutputActivityValue::Button(true),
        }];
        let mut latch = Vec::new();

        let first = apply_output_activity_latch(&mut latch, &observed, now);
        let before_expiry = (now + OUTPUT_ACTIVITY_PREVIEW_LATCH)
            .checked_sub(std::time::Duration::from_millis(1))
            .expect("test duration stays within Instant range");
        let second = apply_output_activity_latch(&mut latch, &[], before_expiry);

        assert_eq!(first, observed);
        assert_eq!(second, observed);
    }

    #[test]
    fn observed_output_activity_latch_expires_without_new_snapshot_activity() {
        let owner = activity_owner();
        let now = Instant::now();
        let observed = vec![OutputActivitySnapshot {
            owner,
            value: OutputActivityValue::Button(true),
        }];
        let mut latch = Vec::new();

        let _ = apply_output_activity_latch(&mut latch, &observed, now);
        let expired = apply_output_activity_latch(
            &mut latch,
            &[],
            now + OUTPUT_ACTIVITY_PREVIEW_LATCH + std::time::Duration::from_millis(1),
        );

        assert!(expired.is_empty());
    }

    #[test]
    fn observed_output_activity_latch_does_not_extend_unchanged_activity() {
        let owner = activity_owner();
        let now = Instant::now();
        let observed = vec![OutputActivitySnapshot {
            owner,
            value: OutputActivityValue::Button(true),
        }];
        let mut latch = Vec::new();

        let _ = apply_output_activity_latch(&mut latch, &observed, now);
        let _ = apply_output_activity_latch(
            &mut latch,
            &observed,
            now + std::time::Duration::from_millis(50),
        );
        let expired = apply_output_activity_latch(
            &mut latch,
            &[],
            now + OUTPUT_ACTIVITY_PREVIEW_LATCH + std::time::Duration::from_millis(1),
        );

        assert!(expired.is_empty());
    }
}
