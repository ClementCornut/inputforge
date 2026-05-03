// Rust guideline compliant 2026-05-03

//! Project the engine's live-axis reading through the F10/F11 pipeline so
//! each instrument can render the live tracking dot at the right viewBox
//! coordinate. Gates: top-level stage, bound input, connected device, axis
//! input. Any failed gate returns `None` (no dot, no guides).

use dioxus::prelude::ReadableExt;

use inputforge_core::action::Action;
use inputforge_core::pipeline::evaluate_actions_through;
use inputforge_core::types::{InputAddress, InputValue};

use crate::context::AppContext;
use crate::frame::mapping_editor::undo_log::{StageId, StageIdSegment};

/// Internal gate exposed for unit testing. Returns the top-level stage index
/// when `stage_id` is exactly `[Index(n)]`, else `None`.
///
/// Nested stages are gated out because `evaluate_actions_through` walks only
/// the root action list; a nested stage would require a sub-slice that is not
/// yet threaded here.
pub(crate) fn gate_top_level(stage_id: &StageId) -> Option<usize> {
    match stage_id.0.as_slice() {
        [StageIdSegment::Index(n)] => Some(*n),
        _ => None,
    }
}

/// Project the live axis reading for a top-level stage through the pipeline
/// and return the input value as `Some(f64)`, or `None` when any gate fails.
///
/// # Gates
///
/// 1. `stage_id` must be exactly `[Index(n)]`. Nested stages are gated out
///    because `evaluate_actions_through` walks only the root action list; a
///    nested stage would require a sub-slice that is not yet threaded here.
/// 2. `addr` must be `InputAddress::Bound` (has a real device). Unbound
///    mappings have no device to read from.
/// 3. The device must be in `state.devices` with `connected: true`.
/// 4. The evaluated `InputValue` must be `Axis`. Button and Hat inputs are
///    not projected because `ResponseCurve` stages operate on scalar values.
///
/// `ctx.live` is read (not peeked) to subscribe the calling component to the
/// engine's ~60 Hz polling tick, ensuring the dot re-renders on every poll.
pub(crate) fn compute_live_axis_value(
    stage_id: &StageId,
    addr: &InputAddress,
    ctx: &AppContext,
    actions: &[Action],
) -> Option<f64> {
    // Gate 1: top-level only.
    let stop_at = gate_top_level(stage_id)?;
    // Gate 2: bound input only.
    let device_id = addr.device()?;
    // Subscribe to the ~60 Hz polling tick. The actual values come from
    // `ctx.state`, not from `ctx.live`; this read is solely for reactivity.
    let _ = ctx.live.read();
    let state_guard = ctx.state.try_read()?;
    // Gate 3: device must be connected.
    let device_present = state_guard
        .devices
        .iter()
        .any(|d| &d.info.id == device_id && d.connected);
    if !device_present {
        return None;
    }
    let value = evaluate_actions_through(actions, &state_guard, addr, stop_at);
    drop(state_guard);
    // Gate 4: axis inputs only.
    match value {
        InputValue::Axis { value, .. } => Some(value.value()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nested_stage_id() -> StageId {
        StageId(vec![StageIdSegment::Index(0), StageIdSegment::IfTrue])
    }

    fn unbound_addr() -> InputAddress {
        InputAddress::Unbound
    }

    #[test]
    fn nested_stage_returns_none() {
        let id = nested_stage_id();
        // ctx and actions intentionally not constructed: the gate trips before they are read.
        let result = gate_top_level(&id);
        assert!(result.is_none());
    }

    #[test]
    fn unbound_addr_returns_none() {
        let addr = unbound_addr();
        let device = addr.device();
        assert!(device.is_none());
    }
}
