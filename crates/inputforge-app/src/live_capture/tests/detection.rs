use std::time::{Duration, Instant};

use inputforge_core::state::InputCacheEntry;
use inputforge_core::types::{
    AxisPolarity, AxisValue, DeviceId, InputAddress, InputId, InputValue,
};

use crate::live_capture::machine::{CoreState, DEBOUNCE_MS, step, step_session};
use inputforge_gui_dx::capture::CaptureFilter;

fn axis_addr(index: u8) -> InputAddress {
    InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index },
    }
}

fn button_addr(index: u8) -> InputAddress {
    InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index },
    }
}

fn axis_entry(index: u8, value: f64) -> InputCacheEntry {
    InputCacheEntry {
        address: axis_addr(index),
        value: InputValue::Axis {
            value: AxisValue::new(value),
            polarity: AxisPolarity::Bipolar,
        },
    }
}

fn button_entry(index: u8, pressed: bool) -> InputCacheEntry {
    InputCacheEntry {
        address: button_addr(index),
        value: InputValue::Button { pressed },
    }
}

fn fresh_state(filter: CaptureFilter) -> CoreState {
    CoreState {
        baseline: None,
        pending: None,
        filter,
    }
}

#[test]
fn first_tick_records_baseline_and_does_not_fire() {
    let prev = fresh_state(CaptureFilter::Any);
    let snapshot = vec![axis_entry(0, 0.3)];
    let now = Instant::now();

    let (next, fired) = step(prev, &snapshot, now);

    assert!(
        fired.is_none(),
        "first tick must never fire, only records baseline"
    );
    assert!(
        next.baseline.is_some(),
        "baseline must be populated after first step"
    );
    assert!(next.pending.is_none());
}

#[test]
fn joystick_already_displaced_no_false_fire() {
    let now0 = Instant::now();
    let (state_after_baseline, _) =
        step(fresh_state(CaptureFilter::Any), &[axis_entry(0, 0.3)], now0);

    let now1 = now0 + Duration::from_millis(16);
    let (state_after_wiggle, fired) = step(state_after_baseline, &[axis_entry(0, 0.32)], now1);

    assert!(fired.is_none(), "delta < deadband must not fire");
    assert!(
        state_after_wiggle.pending.is_none(),
        "no pending capture should open"
    );

    let now2 = now1 + Duration::from_millis(16);
    let (state_after_move, fired) = step(state_after_wiggle, &[axis_entry(0, 0.6)], now2);
    assert!(
        fired.is_none(),
        "first crossing only opens the debounce window"
    );
    assert!(
        state_after_move.pending.is_some(),
        "axis crossing must open a pending capture window",
    );
    let (_, t0) = state_after_move.pending.as_ref().expect("pending set");
    assert_eq!(*t0, now2, "pending t0 must equal the first-crossing tick");
}

#[test]
fn always_on_switch_baselines_correctly() {
    let now0 = Instant::now();
    let (state_after_baseline, _) = step(
        fresh_state(CaptureFilter::Any),
        &[button_entry(3, true)],
        now0,
    );

    let now1 = now0 + Duration::from_millis(16);
    let (state_unchanged, fired) = step(state_after_baseline, &[button_entry(3, true)], now1);
    assert!(fired.is_none(), "unchanged state must not fire");
    assert!(state_unchanged.pending.is_none());

    let now2 = now1 + Duration::from_millis(16);
    let (state_with_pending, fired) = step(state_unchanged, &[button_entry(3, false)], now2);
    assert!(
        fired.is_none(),
        "first toggle only opens the debounce window"
    );
    assert!(
        state_with_pending.pending.is_some(),
        "button toggle must open a pending capture window",
    );
}

#[test]
fn multi_axis_nudge_largest_delta_wins() {
    let t0 = Instant::now();
    let (state, _) = step(
        fresh_state(CaptureFilter::Any),
        &[axis_entry(0, 0.0), axis_entry(1, 0.0)],
        t0,
    );

    let t1 = t0 + Duration::from_millis(16);
    let (state, fired) = step(state, &[axis_entry(0, 0.2), axis_entry(1, 0.0)], t1);
    assert!(fired.is_none(), "first crossing only opens the window");
    assert_eq!(
        state.pending.as_ref().map(|(a, _)| a.clone()),
        Some(axis_addr(0)),
    );

    let t2 = t1 + Duration::from_millis(16);
    let (state, fired) = step(state, &[axis_entry(0, 0.2), axis_entry(1, 0.4)], t2);
    assert!(fired.is_none(), "still inside debounce window");
    assert_eq!(
        state.pending.as_ref().map(|(a, _)| a.clone()),
        Some(axis_addr(1)),
        "larger delta must replace the smaller one within the debounce window",
    );

    let t3 = t1 + Duration::from_millis(DEBOUNCE_MS + 5);
    let (state, fired) = step(state, &[axis_entry(0, 0.2), axis_entry(1, 0.4)], t3);
    assert_eq!(
        fired,
        Some(axis_addr(1)),
        "winner must be the largest-delta axis"
    );
    assert!(
        state.pending.is_none() && state.baseline.is_none(),
        "fire must reset both pending and baseline",
    );
}

#[test]
fn axes_only_filter_rejects_button_toggle() {
    let t0 = Instant::now();
    let (state, _) = step(
        fresh_state(CaptureFilter::AxesOnly),
        &[button_entry(0, false)],
        t0,
    );

    let t1 = t0 + Duration::from_millis(16);
    let (state, fired) = step(state, &[button_entry(0, true)], t1);
    assert!(fired.is_none(), "AxesOnly must not fire on button toggle");
    assert!(state.pending.is_none());
}

#[test]
fn buttons_only_filter_rejects_axis_crossing() {
    let t0 = Instant::now();
    let (state, _) = step(
        fresh_state(CaptureFilter::ButtonsOnly),
        &[axis_entry(0, 0.0)],
        t0,
    );

    let t1 = t0 + Duration::from_millis(16);
    let (state, fired) = step(state, &[axis_entry(0, 0.8)], t1);
    assert!(
        fired.is_none(),
        "ButtonsOnly must not fire on axis crossing"
    );
    assert!(state.pending.is_none());
}

#[test]
fn cancel_mid_window_resets_baseline_and_pending() {
    let t0 = Instant::now();
    let (state, _) = step(fresh_state(CaptureFilter::Any), &[axis_entry(0, 0.0)], t0);
    let t1 = t0 + Duration::from_millis(16);
    let (state, _) = step(state, &[axis_entry(0, 0.8)], t1);
    assert!(state.pending.is_some());

    let cleared = CoreState {
        baseline: None,
        pending: None,
        filter: state.filter,
    };
    let t2 = t1 + Duration::from_millis(16);
    let (after, fired) = step(cleared, &[axis_entry(0, 0.8)], t2);
    assert!(
        fired.is_none(),
        "post-cancel first tick must only re-baseline"
    );
    assert!(after.baseline.is_some());
    assert!(after.pending.is_none());
}

#[test]
fn multi_axis_tie_first_encountered_wins() {
    let t0 = Instant::now();
    let (state, _) = step(
        fresh_state(CaptureFilter::Any),
        &[axis_entry(0, 0.0), axis_entry(1, 0.0)],
        t0,
    );

    let t1 = t0 + Duration::from_millis(16);
    let (state, fired) = step(state, &[axis_entry(0, 0.4), axis_entry(1, 0.4)], t1);
    assert!(fired.is_none(), "first crossing only opens the window");
    assert_eq!(
        state.pending.as_ref().map(|(a, _)| a.clone()),
        Some(axis_addr(0)),
        "tied deltas → first axis in iteration order wins (axis 0)",
    );
}

#[test]
fn replacement_snapshot_clears_pending_capture_even_if_values_are_unchanged() {
    use crate::live_capture::machine::CoreState;
    use inputforge_gui_dx::snapshots::CaptureEpoch;
    let now = Instant::now();
    let address = InputAddress::Bound {
        device: DeviceId("evdev:v1:test".into()),
        input: InputId::Button { index: 0 },
    };
    let entries = vec![InputCacheEntry {
        address: address.clone(),
        value: InputValue::Button { pressed: true },
    }];
    let pending = CoreState {
        baseline: Some(entries.clone()),
        pending: Some((address, now)),
        filter: CaptureFilter::Any,
    };
    let epoch = CaptureEpoch {
        generation: 2,
        ready: true,
        allowed: true,
    };
    let (fresh, fired) = step_session(
        pending,
        &entries,
        now + Duration::from_millis(100),
        Some(1),
        epoch,
    );
    assert!(fired.is_none());
    assert!(fresh.pending.is_none());
    assert_eq!(fresh.baseline, Some(entries.clone()));
    let (blocked, fired) = step_session(
        fresh,
        &entries,
        now,
        Some(2),
        CaptureEpoch {
            ready: false,
            ..epoch
        },
    );
    assert!(blocked.baseline.is_none());
    assert!(fired.is_none());
}
