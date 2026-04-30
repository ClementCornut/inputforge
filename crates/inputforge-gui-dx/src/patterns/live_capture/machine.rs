//! Pure state-transition logic for the live-capture primitive. Lives
//! outside any Dioxus runtime so it can be unit-tested by feeding
//! hand-crafted snapshot sequences and `Instant`s.
//!
//! Items in this module are unused in the lib build until Task 8 wires
//! the Dioxus hook adapter — `#[allow(dead_code)]` annotations are
//! intentional and will be removed when that task lands.

#![allow(
    dead_code,
    reason = "consumed by Task 8 hook adapter; tests already exercise these"
)]

use std::time::{Duration, Instant};

use inputforge_core::state::InputCacheEntry;
use inputforge_core::types::{HatDirection, InputAddress, InputValue};

use super::CaptureFilter;

/// Axis movement threshold. A delta below this against baseline is
/// ignored — protects against sympathetic stick movement and analog
/// noise. Tunable, but no settings UI in F8.
pub(crate) const AXIS_DEADBAND: f64 = 0.15;

/// Debounce window. Within this many milliseconds of opening a capture
/// window, a larger crossing replaces the pending winner; on expiry,
/// the current winner fires.
pub(crate) const DEBOUNCE_MS: u64 = 50;

/// Internal kind discriminator for `InputCacheEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputKind {
    Axis,
    Button,
    Hat,
}

/// State the live-capture primitive carries between polling ticks.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct CoreState {
    /// Baseline snapshot taken on the first tick after `start()`.
    pub baseline: Option<Vec<InputCacheEntry>>,
    /// Current best candidate within the open debounce window:
    /// `(address, window_open_time)`. `None` when no window is open.
    pub pending: Option<(InputAddress, Instant)>,
    /// Active filter (`Any` / `AxesOnly` / `ButtonsOnly`).
    pub filter: CaptureFilter,
}

/// Pure state-transition fn — see F8 spec § "Internal mechanics".
///
/// **Tied-axis tiebreak rule:** When two axes cross deadband on the
/// same tick with identical absolute deltas, the first axis encountered
/// in `snapshot`'s iteration order wins. `InputCacheStore::clone_compact`
/// guarantees stable, deterministic order (Task 4 contract).
pub(crate) struct LiveCaptureCore;

impl LiveCaptureCore {
    #[allow(
        clippy::too_many_lines,
        reason = "single state-transition fn — splitting it would obscure the branch structure"
    )]
    pub(crate) fn step(
        prev: CoreState,
        snapshot: &[InputCacheEntry],
        now: Instant,
    ) -> (CoreState, Option<InputAddress>) {
        // Branch 1: first tick — record baseline, never fire.
        let Some(baseline) = prev.baseline.as_ref() else {
            return (
                CoreState {
                    baseline: Some(snapshot.to_vec()),
                    pending: None,
                    filter: prev.filter,
                },
                None,
            );
        };

        // Branch 2: collect crossings against baseline, scoped by filter.
        let crossings = collect_crossings(prev.filter, baseline, snapshot);

        // Branch 3: window-state evolution.
        match prev.pending {
            None if crossings.is_empty() => (
                CoreState {
                    baseline: prev.baseline,
                    pending: None,
                    filter: prev.filter,
                },
                None,
            ),
            None => {
                let winner = pick_winner(&crossings);
                (
                    CoreState {
                        baseline: prev.baseline,
                        pending: Some((winner, now)),
                        filter: prev.filter,
                    },
                    None,
                )
            }
            Some((pending_addr, t0)) => {
                if now.duration_since(t0) >= Duration::from_millis(DEBOUNCE_MS) {
                    return (
                        CoreState {
                            baseline: None,
                            pending: None,
                            filter: prev.filter,
                        },
                        Some(pending_addr),
                    );
                }
                let mut best_addr = pending_addr.clone();
                let mut best_delta = current_delta_for(
                    &pending_addr,
                    prev.baseline.as_ref().expect("baseline set"),
                    snapshot,
                );
                for (addr, d, _) in &crossings {
                    if *d > best_delta {
                        best_delta = *d;
                        best_addr = addr.clone();
                    }
                }
                (
                    CoreState {
                        baseline: prev.baseline,
                        pending: Some((best_addr, t0)),
                        filter: prev.filter,
                    },
                    None,
                )
            }
        }
    }
}

/// Walk the snapshot, returning every input that has crossed deadband
/// against baseline (or, in the absence of a baseline entry, that has
/// moved away from a quiescent default). Filtered by the active
/// `CaptureFilter`.
fn collect_crossings(
    filter: CaptureFilter,
    baseline: &[InputCacheEntry],
    snapshot: &[InputCacheEntry],
) -> Vec<(InputAddress, f64, InputKind)> {
    let mut crossings: Vec<(InputAddress, f64, InputKind)> = Vec::new();
    for entry in snapshot {
        let kind = match entry.value {
            InputValue::Axis { .. } => InputKind::Axis,
            InputValue::Button { .. } => InputKind::Button,
            InputValue::Hat { .. } => InputKind::Hat,
        };
        if !filter_accepts(filter, kind) {
            continue;
        }
        let baseline_value = baseline.iter().find(|b| b.address == entry.address);
        let delta = match (&entry.value, baseline_value.map(|b| &b.value)) {
            (InputValue::Axis { value: cur }, Some(InputValue::Axis { value: base })) => {
                let d = (cur.value() - base.value()).abs();
                (d > AXIS_DEADBAND).then_some(d)
            }
            (InputValue::Axis { value: cur }, None) => {
                let d = cur.value().abs();
                (d > AXIS_DEADBAND).then_some(d)
            }
            (InputValue::Button { pressed: cur }, Some(InputValue::Button { pressed: base })) => {
                (cur != base).then_some(1.0)
            }
            (InputValue::Button { pressed: true }, None) => Some(1.0),
            (InputValue::Hat { direction: cur }, Some(InputValue::Hat { direction: base })) => {
                (cur != base).then_some(1.0)
            }
            (InputValue::Hat { direction }, None) => {
                (*direction != HatDirection::Center).then_some(1.0)
            }
            _ => None,
        };
        if let Some(d) = delta {
            crossings.push((entry.address.clone(), d, kind));
        }
    }
    crossings
}

fn filter_accepts(filter: CaptureFilter, kind: InputKind) -> bool {
    matches!(
        (filter, kind),
        (CaptureFilter::Any, _)
            | (CaptureFilter::AxesOnly, InputKind::Axis)
            | (CaptureFilter::ButtonsOnly, InputKind::Button)
    )
}

/// Pick the winning crossing.
///
/// - For axis crossings: largest absolute delta wins.
/// - **Tied absolute deltas:** the first axis encountered in
///   `crossings`' order wins (linear scan with strict `>` not `>=`).
/// - For non-axis (buttons/hats, all deltas = 1.0): first crossing wins.
fn pick_winner(crossings: &[(InputAddress, f64, InputKind)]) -> InputAddress {
    let any_axis = crossings.iter().any(|(_, _, k)| *k == InputKind::Axis);
    if any_axis {
        let mut best_idx = 0usize;
        let mut best_delta = crossings[0].1;
        for (i, (_, d, _)) in crossings.iter().enumerate().skip(1) {
            if *d > best_delta {
                best_delta = *d;
                best_idx = i;
            }
        }
        crossings[best_idx].0.clone()
    } else {
        crossings
            .first()
            .map(|(addr, _, _)| addr.clone())
            .expect("crossings non-empty")
    }
}

/// Recompute the absolute delta for a pending address against the
/// current snapshot — used when comparing newly-crossing inputs.
fn current_delta_for(
    addr: &InputAddress,
    baseline: &[InputCacheEntry],
    snapshot: &[InputCacheEntry],
) -> f64 {
    let snap = snapshot.iter().find(|e| &e.address == addr);
    let base = baseline.iter().find(|e| &e.address == addr);
    match (snap.map(|e| &e.value), base.map(|e| &e.value)) {
        (Some(InputValue::Axis { value: cur }), Some(InputValue::Axis { value: base })) => {
            (cur.value() - base.value()).abs()
        }
        (Some(InputValue::Axis { value: cur }), None) => cur.value().abs(),
        _ => 1.0,
    }
}
