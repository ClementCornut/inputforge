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
use crate::output::traits::{KeyboardSink, OutputSink};
use crate::pipeline::{self, PipelineContext, PipelineOutput};
use crate::state::{InputCacheStore, OutputCacheStore};
use crate::types::{AxisValue, InputAddress, InputValue, OutputId};

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
pub(super) fn process_pipeline_outputs(
    outputs: &[PipelineOutput],
    output_sink: &mut dyn OutputSink,
    keyboard: &mut dyn KeyboardSink,
    mode_state: &mut ModeState,
    mode_list: &Modes,
    callbacks: &mut CallbackRegistry,
    triggering_input: &InputAddress,
) -> Result<OutputResult> {
    let mut mode_changed = false;

    for output in outputs {
        match output {
            PipelineOutput::SetAxis { output, value } => {
                let OutputId::Axis { id } = &output.output else {
                    tracing::warn!(
                        output_id = ?output.output,
                        "SetAxis output has non-axis OutputId, skipping"
                    );
                    continue;
                };
                output_sink.set_axis(output.device, *id, *value)?;
            }
            PipelineOutput::SetButton { output, pressed } => {
                let OutputId::Button { id } = &output.output else {
                    tracing::warn!(
                        output_id = ?output.output,
                        "SetButton output has non-button OutputId, skipping"
                    );
                    continue;
                };
                output_sink.set_button(output.device, *id, *pressed)?;
            }
            PipelineOutput::Keyboard { key, active, .. } => {
                if *active {
                    keyboard.send_key(key)?;
                }
            }
            PipelineOutput::Mouse { .. } => {}
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
            } => {
                if let OutputId::Axis { id } = &addr.output {
                    cache.set_axis(addr.device, *id, *value);
                }
            }
            PipelineOutput::SetButton {
                output: addr,
                pressed,
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

/// Re-process all cached axis values through the active mode's pipelines.
///
/// Called after a mode change, on engine activation, and after saving a
/// mapping so that axis outputs reflect current mappings immediately,
/// without waiting for a physical input event.
pub(super) fn refresh_axes_for_mode_change(
    cache: &InputCacheStore,
    mappings: &[crate::action::Mapping],
    mode: &str,
    output_sink: &mut dyn OutputSink,
    output_cache: &mut OutputCacheStore,
) -> Result<()> {
    for (address, value, polarity) in cache.get_all_axis_entries() {
        if let Some(mapping) = mappings
            .iter()
            .find(|mapping| mapping.input == address && mapping.mode == *mode)
        {
            let input_value = InputValue::Axis {
                value: AxisValue::new(value),
                polarity,
            };
            let mut ctx = PipelineContext {
                current_value: value,
                input_value,
                outputs: Vec::new(),
                input_cache: cache,
            };
            pipeline::execute_pipeline(&mapping.actions, &mut ctx);

            // Apply only axis and button outputs during refresh.
            for output in &ctx.outputs {
                match output {
                    PipelineOutput::SetAxis {
                        output: addr,
                        value: v,
                    } => {
                        let OutputId::Axis { id } = &addr.output else {
                            tracing::warn!(
                                output_id = ?addr.output,
                                "SetAxis refresh has non-axis OutputId, skipping"
                            );
                            continue;
                        };
                        output_sink.set_axis(addr.device, *id, *v)?;
                        output_cache.set_axis(addr.device, *id, *v);
                    }
                    PipelineOutput::SetButton {
                        output: addr,
                        pressed,
                    } => {
                        let OutputId::Button { id } = &addr.output else {
                            tracing::warn!(
                                output_id = ?addr.output,
                                "SetButton refresh has non-button OutputId, skipping"
                            );
                            continue;
                        };
                        output_sink.set_button(addr.device, *id, *pressed)?;
                        output_cache.set_button(addr.device, *id, *pressed);
                    }
                    // Skip key presses (spurious from axis-to-keyboard
                    // mappings) and mode changes (avoid recursion).
                    PipelineOutput::Keyboard { .. }
                    | PipelineOutput::Mouse { .. }
                    | PipelineOutput::ChangeMode { .. } => {}
                }
            }
        }
    }
    Ok(())
}
