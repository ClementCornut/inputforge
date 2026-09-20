// Rust guideline compliant 2026-03-06

//! Pipeline output processing and mode change application.
//!
//! Contains pure functions that translate [`PipelineOutput`] values
//! into concrete I/O calls and mode state transitions. Kept separate
//! from the main loop for testability and readability.

use crate::action::ModeChangeStrategy;
use crate::callbacks::{CallbackRegistry, ReleaseCallback};
use crate::error::Result;
use crate::mode::{ModeState, Modes};
use crate::output::traits::{KeyboardSink, MouseSink, OutputSink};
use crate::pipeline::{self, PipelineContext, PipelineOutput};
use crate::state::{OutputActivityStore, OutputActivityValue, OutputCacheStore};
use crate::types::{AxisValue, InputAddress, InputValue, OutputId};

use super::output_state::{OutputAction, OutputEvent, OutputRuntimeState};

/// Result of processing pipeline outputs for a single event.
#[derive(Debug)]
pub(super) struct OutputResult {
    /// Whether the active mode changed during processing.
    pub mode_changed: bool,
}

/// Process all pipeline outputs for a single input event.
///
/// Dispatches each output to the appropriate sink. Mode changes are
/// applied to `mode_state` and callbacks are registered for temporary
/// modes. Returns whether the mode changed (for axis refresh).
///
/// Uses exhaustive matching on [`PipelineOutput`] so new variants
/// cause compile errors rather than silent no-ops.
#[expect(
    clippy::too_many_arguments,
    reason = "Engine run loop owns these mutable subsystems separately; grouping would obscure borrows."
)]
pub(super) fn process_pipeline_outputs(
    outputs: &[PipelineOutput],
    output_sink: &mut dyn OutputSink,
    keyboard: &mut dyn KeyboardSink,
    mouse: &mut dyn MouseSink,
    output_state: &mut OutputRuntimeState,
    mode_state: &mut ModeState,
    mode_list: &Modes,
    callbacks: &mut CallbackRegistry,
    triggering_input: &InputAddress,
) -> Result<OutputResult> {
    let mut mode_changed = false;

    for output in outputs {
        match output {
            PipelineOutput::SetAxis { output, value, .. } => {
                let OutputId::Axis { id } = &output.output else {
                    tracing::warn!(
                        output_id = ?output.output,
                        "SetAxis output has non-axis OutputId, skipping"
                    );
                    continue;
                };
                output_sink.set_axis(output.device, *id, *value)?;
            }
            PipelineOutput::SetHat {
                owner,
                output,
                direction,
            } => {
                let OutputId::Hat { id } = output.output else {
                    continue;
                };
                output_sink.set_hat(output.device, id, *direction)?;
                output_state.commit_set_button(
                    owner.clone(),
                    output.clone(),
                    *direction != crate::types::HatDirection::Center,
                );
            }
            PipelineOutput::SetButton {
                owner,
                output,
                pressed,
            } => {
                let OutputId::Button { id } = &output.output else {
                    tracing::warn!(
                        output_id = ?output.output,
                        "SetButton output has non-button OutputId, skipping"
                    );
                    continue;
                };
                output_sink.set_button(output.device, *id, *pressed)?;
                output_state.commit_set_button(owner.clone(), output.clone(), *pressed);
            }
            PipelineOutput::Keyboard {
                owner,
                key,
                behavior,
                active,
            } => {
                for action in
                    output_state.reconcile_keyboard(owner.clone(), key.clone(), *behavior, *active)
                {
                    dispatch_output_action(action, output_state, keyboard, mouse)?;
                }
            }
            PipelineOutput::Mouse {
                owner,
                target,
                behavior,
                active,
            } => {
                for action in
                    output_state.reconcile_mouse(owner.clone(), *target, *behavior, *active)
                {
                    dispatch_output_action(action, output_state, keyboard, mouse)?;
                }
            }
            PipelineOutput::ChangeMode { strategy } => {
                let old_mode = mode_state.current().to_owned();
                apply_mode_change(strategy, mode_state, mode_list, callbacks, triggering_input);
                if mode_state.current() != old_mode {
                    mode_changed = true;
                }
            }
        }
    }

    Ok(OutputResult { mode_changed })
}

pub(super) fn dispatch_output_action(
    action: OutputAction,
    output_state: &mut OutputRuntimeState,
    keyboard: &mut dyn KeyboardSink,
    mouse: &mut dyn MouseSink,
) -> Result<()> {
    match action {
        OutputAction::HoldStart { owner, event } => {
            if let Some(event) = event {
                dispatch_event(event, keyboard, mouse)?;
            }
            output_state.commit_hold(owner);
        }
        OutputAction::Pulse {
            owner,
            start,
            finish,
        } => {
            dispatch_event(start, keyboard, mouse)?;
            if let Some(finish) = finish
                && let Err(err) = dispatch_event(finish, keyboard, mouse)
            {
                output_state.mark_partial_pulse(owner);
                return Err(err);
            }
            output_state.commit_pulse(owner);
        }
        OutputAction::Release { owner, event } => {
            dispatch_event(event, keyboard, mouse)?;
            output_state.commit_release(&owner);
        }
    }

    Ok(())
}

fn dispatch_event(
    event: OutputEvent,
    keyboard: &mut dyn KeyboardSink,
    mouse: &mut dyn MouseSink,
) -> Result<()> {
    match event {
        OutputEvent::KeyDown(key) => keyboard.key_down(&key),
        OutputEvent::KeyUp(key) => keyboard.key_up(&key),
        OutputEvent::MouseDown(target) => mouse.button_down(target),
        OutputEvent::MouseUp(target) => mouse.button_up(target),
        OutputEvent::Wheel(target) => mouse.wheel(target),
    }
}

/// Apply a mode change strategy to the mode state.
///
/// Delegates to the appropriate [`ModeState`] method. For temporary
/// mode pushes, registers a [`ReleaseCallback::PopTemporaryMode`]
/// on the triggering input so releasing the button pops the mode.
///
/// Mode change errors (e.g., `ModeNotFound`, `ModeCycleDetected`)
/// are logged and skipped rather than propagated, because they
/// represent recoverable user-configuration issues that must not
/// terminate the engine loop.
fn apply_mode_change(
    strategy: &ModeChangeStrategy,
    mode_state: &mut ModeState,
    mode_list: &Modes,
    callbacks: &mut CallbackRegistry,
    triggering_input: &InputAddress,
) {
    match strategy {
        ModeChangeStrategy::SwitchTo { mode } => {
            if let Err(e) = mode_state.switch_to(mode, mode_list) {
                tracing::warn!(
                    mode,
                    error = %e,
                    "SwitchTo failed, skipping"
                );
            }
        }
        ModeChangeStrategy::Temporary { mode } => {
            match mode_state.push_temporary(mode, mode_list) {
                Ok(()) => {
                    callbacks.register(triggering_input.clone(), ReleaseCallback::PopTemporaryMode);
                }
                Err(e) => {
                    tracing::warn!(
                        mode,
                        error = %e,
                        "Temporary mode push failed, skipping"
                    );
                }
            }
        }
    }
}

/// Write axis and button values from pipeline outputs into the output cache.
///
/// Iterates each output and updates the corresponding entry in the cache.
/// Non-output intent variants (`Keyboard`, `Mouse`, `ChangeMode`) are ignored.
pub(super) fn record_outputs_to_cache(outputs: &[PipelineOutput], cache: &mut OutputCacheStore) {
    for output in outputs {
        match output {
            PipelineOutput::SetAxis {
                output: addr,
                value,
                ..
            } => {
                if let OutputId::Axis { id } = &addr.output {
                    cache.set_axis(addr.device, *id, *value);
                }
            }
            PipelineOutput::SetHat {
                output: addr,
                direction,
                ..
            } => {
                if let OutputId::Hat { id } = addr.output {
                    cache.set_hat(addr.device, id, *direction);
                }
            }
            PipelineOutput::SetButton {
                output: addr,
                pressed,
                ..
            } => {
                if let OutputId::Button { id } = &addr.output {
                    cache.set_button(addr.device, *id, *pressed);
                }
            }
            PipelineOutput::Keyboard { .. }
            | PipelineOutput::Mouse { .. }
            | PipelineOutput::ChangeMode { .. } => {}
        }
    }
}

pub(super) fn record_outputs_to_activity(
    outputs: &[PipelineOutput],
    activity: &mut OutputActivityStore,
    now: std::time::Instant,
    latch_inactive: bool,
) {
    activity.prune(now);
    for output in outputs {
        match output {
            PipelineOutput::SetAxis { owner, value, .. } => {
                activity.record(
                    owner.clone(),
                    OutputActivityValue::Axis(*value),
                    now,
                    latch_inactive,
                );
            }
            PipelineOutput::SetHat {
                owner, direction, ..
            } => {
                activity.record(
                    owner.clone(),
                    OutputActivityValue::Hat(*direction),
                    now,
                    latch_inactive,
                );
            }
            PipelineOutput::SetButton { owner, pressed, .. } => {
                activity.record(
                    owner.clone(),
                    OutputActivityValue::Button(*pressed),
                    now,
                    latch_inactive,
                );
            }
            PipelineOutput::Keyboard { owner, active, .. } => {
                activity.record(
                    owner.clone(),
                    OutputActivityValue::Keyboard(*active),
                    now,
                    latch_inactive,
                );
            }
            PipelineOutput::Mouse {
                owner,
                target,
                active,
                ..
            } => {
                let active = *active && !target.is_wheel();
                activity.record(
                    owner.clone(),
                    OutputActivityValue::Mouse(active),
                    now,
                    latch_inactive,
                );
            }
            PipelineOutput::ChangeMode { .. } => {}
        }
    }
}

/// Re-process all cached axis values through the active mode's pipelines.
///
/// Called after a mode change, on engine activation, and after saving a
/// mapping so that axis outputs reflect current mappings immediately,
/// without waiting for a physical input event.
pub(super) fn refresh_axes_for_state(
    state: &mut crate::state::AppState,
    mappings: &[crate::action::Mapping],
    mode: &str,
    output_sink: &mut dyn OutputSink,
) -> Result<()> {
    let values = crate::state::InputValues::from_parts(&state.input_cache, &state.calibrations);
    for (address, _, _) in state.input_cache.get_all_axis_entries() {
        if let Some(mapping) = mappings
            .iter()
            .find(|mapping| mapping.input == address && mapping.mode == *mode)
        {
            let (value, polarity) = pipeline::InputCache::get_axis(&values, &address);
            let input_value = InputValue::Axis {
                value: AxisValue::new(value),
                polarity,
            };
            let mut ctx = PipelineContext {
                current_value: value,
                input_value,
                outputs: Vec::new(),
                input_cache: &values,
            };
            pipeline::execute_pipeline(&mapping.actions, &mut ctx);

            // Refresh continuous outputs without inventing button/key edges or mode changes.
            for output in &ctx.outputs {
                let PipelineOutput::SetAxis {
                    output: addr,
                    value,
                    ..
                } = output
                else {
                    continue;
                };
                let OutputId::Axis { id } = &addr.output else {
                    tracing::warn!(
                        output_id = ?addr.output,
                        "SetAxis refresh has non-axis OutputId, skipping"
                    );
                    continue;
                };
                output_sink.set_axis(addr.device, *id, *value)?;
                state.output_cache.set_axis(addr.device, *id, *value);
            }
        }
    }
    Ok(())
}
