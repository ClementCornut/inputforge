// Rust guideline compliant 2026-05-01

//! Live readout: IN/OUT axis bars with merge-mapping layout.
//!
//! Renders a compact two-row grid (label | bar | percentage) driven by the
//! live `ctx.live` snapshot for raw values and
//! `evaluate_actions_through` for pipeline-evaluated values.
//!
//! **Layout rules (per F9 spec lines 42, 417)**
//! - Non-merge: `IN`, dashed divider, `OUT` (OUT omitted when no `MapToVJoy`).
//! - Merge:     `IN 1`, `IN 2`, dashed divider, merged `IN`, `OUT`
//!   (no extra divider before `OUT` in the merge case).

use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::processing::into_natural_domain;
use inputforge_core::types::{AxisPolarity, InputAddress, InputId, InputValue, MergeOp};

use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot};

// ---------------------------------------------------------------------------
// Public component
// ---------------------------------------------------------------------------

/// Live IN/OUT readout section, wired beneath the input field.
///
/// # Props
/// - `primary`  — address of the primary (mapped) input axis.
/// - `actions`  — the full action pipeline for the selected mapping.
#[component]
pub(crate) fn LiveReadout(primary: InputAddress, actions: Vec<Action>) -> Element {
    let ctx = use_context::<AppContext>();
    let live = ctx.live.read();
    let cfg = ctx.config.read();

    let primary_value = read_axis_display(&primary, &live, &cfg);
    let merge_ctx = find_merge_context(&actions);
    let out_present = first_map_to_vjoy(&actions);

    // Resolve secondary axis display once. Used for the IN 2 row and to
    // infer the merge result's output polarity (consumed by the IN row
    // and OUT row).
    let secondary_display = merge_ctx
        .as_ref()
        .map(|c| read_axis_display(&c.secondary, &live, &cfg));

    // Polarity of the merged result (or, in the no-merge case, of the
    // OUT row): primary inherits when there is no merge; otherwise the
    // merge op's natural output polarity per `merge_output_polarity`.
    let output_polarity = match (&merge_ctx, secondary_display) {
        (Some(c), Some(secondary)) => {
            merge_output_polarity(c.op, primary_value.polarity, secondary.polarity)
        }
        _ => primary_value.polarity,
    };

    // Brief read-lock on AppState for pipeline evaluation.
    // Two separate guard scopes so Rust can prove non-overlapping borrows.
    let merged_in_value = merge_ctx.as_ref().map(|c| {
        let state = ctx.state.read();
        let iv = inputforge_core::pipeline::evaluate_actions_through(
            &actions,
            &state,
            &primary,
            c.index + 1,
        );
        AxisDisplay {
            value: into_natural_domain(axis_f64(&iv), output_polarity),
            polarity: output_polarity,
        }
    });

    let out_value = out_present.then(|| {
        let state = ctx.state.read();
        let iv = inputforge_core::pipeline::evaluate_actions_through(
            &actions,
            &state,
            &primary,
            actions.len(),
        );
        AxisDisplay {
            value: into_natural_domain(axis_f64(&iv), output_polarity),
            polarity: output_polarity,
        }
    });

    rsx! {
        div { class: "if-editor__readout",
            if let Some(secondary) = secondary_display {
                // Merge layout: IN 1, IN 2, dashed divider, merged IN, OUT.
                // No extra divider before OUT in the merge case (spec line 417).
                // `merged_in_value` is always Some when secondary_display is
                // Some (both derive from the same merge_ctx); the unwrap_or
                // is a defensive default that should never fire.
                ReadoutRow { label: "IN 1".to_owned(), display: primary_value }
                ReadoutRow { label: "IN 2".to_owned(), display: secondary }
                div { class: "if-editor__readout-divider-dashed" }
                ReadoutRow {
                    label: "IN".to_owned(),
                    display: merged_in_value.unwrap_or(AxisDisplay {
                        value: into_natural_domain(0.0, output_polarity),
                        polarity: output_polarity,
                    }),
                }
                if let Some(out) = out_value {
                    ReadoutRow { label: "OUT".to_owned(), display: out }
                }
            } else {
                // Non-merge layout: IN, optional dashed divider + OUT.
                ReadoutRow { label: "IN".to_owned(), display: primary_value }
                if let Some(out) = out_value {
                    div { class: "if-editor__readout-divider-dashed" }
                    ReadoutRow { label: "OUT".to_owned(), display: out }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Sub-component
// ---------------------------------------------------------------------------

/// One row in the readout grid: label | bar | percentage text.
///
/// The bar fill is anchored at 50% for bipolar axes and at 0% for
/// unipolar axes, matching the live-green visual described in the spec.
#[component]
fn ReadoutRow(label: String, display: AxisDisplay) -> Element {
    let pct_text = format_percentage(&display);
    let bipolar = matches!(display.polarity, AxisPolarity::Bipolar);

    // Bipolar bars are anchored at the 50% center and grow toward one
    // edge, so the visual maximum is half the container width. Unipolar
    // bars grow from the left edge across the full width.
    let fill_pct = if bipolar {
        (display.value.abs() * 50.0).clamp(0.0, 50.0)
    } else {
        (display.value.abs() * 100.0).clamp(0.0, 100.0)
    };

    // Always set BOTH `left` and `right` (with `auto` for the off side)
    // so Dioxus's per-property style diffing cannot leave a stale anchor
    // from a previous render. Without this, switching from negative to
    // positive (or vice versa) keeps the prior side's `50%` set, which
    // CSS resolves into `left: 50%; right: 50%; width: <pct>%;` and
    // honors `left + width` regardless of sign — so both signs grow
    // rightward and overflow the container.
    let bar_style = if bipolar && display.value < 0.0 {
        format!("left: auto; right: 50%; width: {fill_pct}%;")
    } else if bipolar {
        format!("left: 50%; right: auto; width: {fill_pct}%;")
    } else {
        format!("left: 0; right: auto; width: {fill_pct}%;")
    };

    rsx! {
        div { class: "if-editor__readout-row",
            div { class: "if-editor__readout-label", "{label}" }
            div { class: "if-editor__readout-bar",
                div {
                    class: "if-editor__readout-fill",
                    style: "{bar_style}",
                }
            }
            div { class: "if-editor__readout-pct", "{pct_text}" }
        }
    }
}

// ---------------------------------------------------------------------------
// Value helpers
// ---------------------------------------------------------------------------

/// Thin display value carried through the readout component tree.
///
/// `value` is normalized to the polarity's natural domain:
/// - `Bipolar`: `[-1.0, 1.0]`, where 0 is centered.
/// - `Unipolar`: `[0.0, 1.0]`, where 0 is idle and 1 is fully pressed.
#[derive(Clone, Copy, PartialEq)]
struct AxisDisplay {
    /// Normalized value in the polarity's natural domain.
    value: f64,
    polarity: AxisPolarity,
}

/// Read the raw axis value and polarity for `addr` from the live snapshot.
///
/// Falls back to `(0.0, Bipolar)` when the device or axis index is not
/// present in the snapshot (e.g. engine offline or non-axis input).
///
/// Hardware reports both bipolar and unipolar axes in the bipolar-encoded
/// range `[-1.0, 1.0]`. For unipolar axes (pedals, throttles, brakes) we
/// remap to the natural `[0.0, 1.0]` domain via `into_natural_domain` so a
/// Thrustmaster pedal idle reads `0.00` (not `-1.00`) and the unipolar
/// bar fill grows monotonically with press depth.
fn read_axis_display(
    addr: &InputAddress,
    live: &LiveSnapshot,
    cfg: &ConfigSnapshot,
) -> AxisDisplay {
    let InputId::Axis { index } = addr.input else {
        return AxisDisplay {
            value: 0.0,
            polarity: AxisPolarity::Bipolar,
        };
    };
    let dev_idx = cfg.devices.iter().position(|d| d.info.id == addr.device);
    if let Some(di) = dev_idx
        && let Some(dev_inputs) = live.device_inputs.get(di)
        && let Some(&(raw, polarity)) = dev_inputs.axes.get(usize::from(index))
    {
        return AxisDisplay {
            value: into_natural_domain(raw, polarity),
            polarity,
        };
    }
    AxisDisplay {
        value: 0.0,
        polarity: AxisPolarity::Bipolar,
    }
}

/// Extract a scalar f64 from any `InputValue`.
///
/// Buttons map to 0.0 / 1.0; hats are not meaningful as axis bars and
/// return 0.0.
fn axis_f64(v: &InputValue) -> f64 {
    match v {
        InputValue::Axis { value, .. } => value.value(),
        InputValue::Button { pressed } => {
            if *pressed {
                1.0
            } else {
                0.0
            }
        }
        InputValue::Hat { .. } => 0.0,
    }
}

// ---------------------------------------------------------------------------
// Action-tree walkers (top-level only for merge; recursive for MapToVJoy)
// ---------------------------------------------------------------------------

/// Resolved context for the first top-level `MergeAxis` in the pipeline.
///
/// Top-level only by design: merges nested inside `Conditional` do not
/// trigger the merge layout (acceptable per the F9 task spec).
#[derive(Clone, PartialEq)]
struct MergeContext {
    /// Position in the action list, used as the `stop_at` for the
    /// pipeline subset that produces the merged IN value.
    index: usize,
    /// Merge operator (drives the polarity inference table).
    op: MergeOp,
    /// Secondary input address (the IN 2 row).
    secondary: InputAddress,
}

/// Find the first top-level `MergeAxis` and return its index, op, and
/// secondary input.
fn find_merge_context(actions: &[Action]) -> Option<MergeContext> {
    actions.iter().enumerate().find_map(|(idx, a)| {
        if let Action::MergeAxis {
            second_input,
            operation,
        } = a
        {
            Some(MergeContext {
                index: idx,
                op: *operation,
                secondary: second_input.clone(),
            })
        } else {
            None
        }
    })
}

/// `true` if `actions` contains a `MapToVJoy` anywhere in the tree
/// (including inside `Conditional` branches).
fn first_map_to_vjoy(actions: &[Action]) -> bool {
    actions.iter().any(|a| match a {
        Action::MapToVJoy { .. } => true,
        Action::Conditional {
            if_true, if_false, ..
        } => first_map_to_vjoy(if_true) || if_false.as_deref().is_some_and(first_map_to_vjoy),
        _ => false,
    })
}

// ---------------------------------------------------------------------------
// Polarity inference
// ---------------------------------------------------------------------------

/// Infer the natural polarity of a merge result from the operator and
/// each input's polarity.
///
/// See `docs/superpowers/plans/2026-05-01-f9-merge-polarity-followup.md`
/// for the truth table and reasoning. Summary:
/// - `Bidirectional`: always Bipolar (a difference can swing through zero).
/// - `Average` / `Maximum`: preserve when both inputs match; Bipolar on mixed.
#[must_use]
fn merge_output_polarity(
    op: MergeOp,
    primary: AxisPolarity,
    secondary: AxisPolarity,
) -> AxisPolarity {
    match op {
        MergeOp::Bidirectional => AxisPolarity::Bipolar,
        MergeOp::Average | MergeOp::Maximum => {
            if primary == secondary {
                primary
            } else {
                AxisPolarity::Bipolar
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Formatting
// ---------------------------------------------------------------------------

/// Format a percentage string for the readout label.
///
/// Bipolar axes show a sign prefix (`+0.00` / `-0.00`) so the center is
/// unambiguous. Unipolar axes omit the sign.
///
/// Sub-precision noise (raw values slightly outside `[-1, 1]` from
/// device calibration drift, IEEE -0.0 from arithmetic, etc.) can round
/// to "-0.00" at two-decimal precision and look wrong to the user. Snap
/// any value whose absolute magnitude rounds to zero at this precision
/// to a literal `0.0` so the output is always `0.00` / `+0.00` at idle.
fn format_percentage(display: &AxisDisplay) -> String {
    let value = if display.value.abs() < 0.005 {
        0.0
    } else {
        display.value
    };
    match display.polarity {
        AxisPolarity::Bipolar => format!("{value:+.2}"),
        AxisPolarity::Unipolar => format!("{value:.2}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- merge_output_polarity ---------------------------------------------

    #[test]
    fn bidirectional_always_bipolar() {
        for primary in [AxisPolarity::Bipolar, AxisPolarity::Unipolar] {
            for secondary in [AxisPolarity::Bipolar, AxisPolarity::Unipolar] {
                assert_eq!(
                    merge_output_polarity(MergeOp::Bidirectional, primary, secondary),
                    AxisPolarity::Bipolar,
                    "Bidirectional should always be bipolar (got {primary:?} + {secondary:?})"
                );
            }
        }
    }

    #[test]
    fn average_unipolar_pair_is_unipolar() {
        assert_eq!(
            merge_output_polarity(
                MergeOp::Average,
                AxisPolarity::Unipolar,
                AxisPolarity::Unipolar
            ),
            AxisPolarity::Unipolar
        );
    }

    #[test]
    fn average_bipolar_pair_is_bipolar() {
        assert_eq!(
            merge_output_polarity(
                MergeOp::Average,
                AxisPolarity::Bipolar,
                AxisPolarity::Bipolar
            ),
            AxisPolarity::Bipolar
        );
    }

    #[test]
    fn average_mixed_is_bipolar() {
        assert_eq!(
            merge_output_polarity(
                MergeOp::Average,
                AxisPolarity::Bipolar,
                AxisPolarity::Unipolar
            ),
            AxisPolarity::Bipolar
        );
        assert_eq!(
            merge_output_polarity(
                MergeOp::Average,
                AxisPolarity::Unipolar,
                AxisPolarity::Bipolar
            ),
            AxisPolarity::Bipolar
        );
    }

    #[test]
    fn maximum_unipolar_pair_is_unipolar() {
        assert_eq!(
            merge_output_polarity(
                MergeOp::Maximum,
                AxisPolarity::Unipolar,
                AxisPolarity::Unipolar
            ),
            AxisPolarity::Unipolar
        );
    }

    #[test]
    fn maximum_bipolar_pair_is_bipolar() {
        assert_eq!(
            merge_output_polarity(
                MergeOp::Maximum,
                AxisPolarity::Bipolar,
                AxisPolarity::Bipolar
            ),
            AxisPolarity::Bipolar
        );
    }

    #[test]
    fn maximum_mixed_is_bipolar() {
        assert_eq!(
            merge_output_polarity(
                MergeOp::Maximum,
                AxisPolarity::Bipolar,
                AxisPolarity::Unipolar
            ),
            AxisPolarity::Bipolar
        );
        assert_eq!(
            merge_output_polarity(
                MergeOp::Maximum,
                AxisPolarity::Unipolar,
                AxisPolarity::Bipolar
            ),
            AxisPolarity::Bipolar
        );
    }

    #[test]
    fn average_and_maximum_are_commutative() {
        for op in [MergeOp::Average, MergeOp::Maximum] {
            for a in [AxisPolarity::Bipolar, AxisPolarity::Unipolar] {
                for b in [AxisPolarity::Bipolar, AxisPolarity::Unipolar] {
                    assert_eq!(
                        merge_output_polarity(op, a, b),
                        merge_output_polarity(op, b, a),
                        "{op:?}({a:?}, {b:?}) should equal {op:?}({b:?}, {a:?})"
                    );
                }
            }
        }
    }

    // -- find_merge_context ---------------------------------------------------

    fn axis_addr(index: u8) -> InputAddress {
        use inputforge_core::types::DeviceId;
        InputAddress {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index },
        }
    }

    #[test]
    fn find_merge_context_returns_none_for_no_merge() {
        let actions = vec![Action::Invert];
        assert!(find_merge_context(&actions).is_none());
    }

    #[test]
    fn find_merge_context_picks_first_top_level_merge() {
        let actions = vec![
            Action::Invert,
            Action::MergeAxis {
                second_input: axis_addr(1),
                operation: MergeOp::Bidirectional,
            },
        ];
        let ctx = find_merge_context(&actions).expect("expected merge context");
        assert_eq!(ctx.index, 1);
        assert_eq!(ctx.op, MergeOp::Bidirectional);
        assert_eq!(ctx.secondary, axis_addr(1));
    }
}
