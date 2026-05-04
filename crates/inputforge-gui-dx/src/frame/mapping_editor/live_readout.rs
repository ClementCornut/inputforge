// Rust guideline compliant 2026-05-01

//! Live readout: IN/OUT axis bars with merge-mapping layout.
//!
//! IN and OUT rows read directly from the live `ctx.live` snapshot (raw
//! input cache and engine output cache, respectively). Merged-IN runs
//! `evaluate_actions_through` over the input cache so curve/deadzone edits
//! preview live. When the engine is stopped, OUT freezes at the engine's
//! last-written value and the row carries an `--frozen` modifier class.
//!
//! **Layout rules (per F9 spec lines 42, 417)**
//! - Non-merge: `IN`, dashed divider, `OUT` (OUT omitted when no `MapToVJoy`).
//! - Merge:     `IN 1`, `IN 2`, dashed divider, merged `IN`, `OUT`
//!   (no extra divider before `OUT` in the merge case).

use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::processing::into_natural_domain;
use inputforge_core::state::EngineStatus;
pub(crate) use inputforge_core::types::AxisPolarity;
use inputforge_core::types::{
    HatDirection, InputAddress, InputId, InputValue, MergeOp, OutputAddress, OutputId, VJoyAxis,
};

use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot};
use crate::frame::mapping_list::source_label;

/// CSS modifier class applied to a `ReadoutRow` whose value is held
/// (engine stopped / paused). The component renders this suffix as part
/// of a static literal in `ReadoutRow`; this const exists so SSR tests
/// can `html.contains(FROZEN_ROW_CLASS)` rather than retyping the
/// modifier name. Keep the literal and the const in sync.
pub(super) const FROZEN_ROW_CLASS: &str = "if-editor__readout-row--frozen";

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
    let engine_running = matches!(ctx.meta.read().engine_status, EngineStatus::Running);

    let primary_value = read_axis_display(&primary, &live, &cfg);
    let merge_ctx = find_merge_context(&actions);
    let out_addr = first_map_to_vjoy_output(&actions);

    // Resolve tags for each row up front (single read-lock). Tags echo
    // info from the header subtitle and merge stage so the readout stays
    // self-explanatory when the user is watching the bars move; the
    // duplication is intentional, not redundant.
    let primary_tag = source_label::format(&primary, &cfg);
    let secondary_tag = merge_ctx
        .as_ref()
        .map(|c| source_label::format(&c.secondary, &cfg));
    let out_tag = out_addr.as_ref().map(format_output_label);

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

    // OUT reads directly from the engine output cache (projected into
    // `live.output_values` by `LiveSnapshot::from_state`). When the engine
    // isn't running it freezes at the last written value, by design.
    let out_value = out_addr
        .as_ref()
        .map(|out| read_output_display(out, &live, &cfg, output_polarity));

    rsx! {
        div { class: "if-editor__readout",
            if let Some(secondary) = secondary_display {
                // Merge layout: IN 1, IN 2, labeled divider, merged IN, OUT.
                // No extra divider before OUT in the merge case (spec line 417).
                // Each side of the divider is its own grid (`readout-group`)
                // so the tag column in the input section auto-sizes to the
                // longest source label, while the merged + OUT section
                // sizes independently. Bars within a group share an x
                // origin; bars across groups can differ.
                // `merged_in_value` is always Some when secondary_display is
                // Some (both derive from the same merge_ctx); the unwrap_or
                // is a defensive default that should never fire.
                div { class: "if-editor__readout-group",
                    ReadoutRow {
                        label: "IN 1".to_owned(),
                        tag: primary_tag.clone(),
                        display: primary_value,
                        frozen: false,
                    }
                    ReadoutRow {
                        label: "IN 2".to_owned(),
                        tag: secondary_tag.unwrap_or_default(),
                        display: secondary,
                        frozen: false,
                    }
                }
                ReadoutDivider { label: "merge".to_owned() }
                div { class: "if-editor__readout-group",
                    ReadoutRow {
                        label: "IN".to_owned(),
                        tag: "Merged".to_owned(),
                        display: merged_in_value.unwrap_or(AxisDisplay {
                            value: into_natural_domain(0.0, output_polarity),
                            polarity: output_polarity,
                        }),
                        frozen: false,
                    }
                    if let Some(out) = out_value {
                        ReadoutRow {
                            label: "OUT".to_owned(),
                            tag: out_tag.clone().unwrap_or_default(),
                            display: out,
                            frozen: !engine_running,
                        }
                    }
                }
            } else {
                // Non-merge layout: IN, optional labeled divider + OUT.
                // Each row sits in its own group: in this case there's no
                // alignment benefit (one row per group) but the markup
                // stays parallel to the merge case.
                div { class: "if-editor__readout-group",
                    ReadoutRow {
                        label: "IN".to_owned(),
                        tag: primary_tag.clone(),
                        display: primary_value,
                        frozen: false,
                    }
                }
                if let Some(out) = out_value {
                    ReadoutDivider { label: "out".to_owned() }
                    div { class: "if-editor__readout-group",
                        ReadoutRow {
                            label: "OUT".to_owned(),
                            tag: out_tag.clone().unwrap_or_default(),
                            display: out,
                            frozen: !engine_running,
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Sub-component
// ---------------------------------------------------------------------------

/// One row in the readout grid: label | tag | bar | percentage text.
///
/// The bar fill is anchored at 50% for bipolar axes and at 0% for
/// unipolar axes, matching the live-green visual described in the spec.
/// Bipolar bars also carry a `--bipolar` modifier class so the CSS can
/// draw a center tick that communicates polarity at idle.
///
/// `frozen` is true only on the OUT row when the engine is not running.
/// CSS dims the bar fill and percentage to signal "held value, not live";
/// label and tag stay at full strength because they describe configuration,
/// not telemetry.
#[component]
fn ReadoutRow(label: String, tag: String, display: AxisDisplay, frozen: bool) -> Element {
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

    let bar_class = if bipolar {
        "if-editor__readout-bar if-editor__readout-bar--bipolar"
    } else {
        "if-editor__readout-bar"
    };

    let row_class = if frozen {
        "if-editor__readout-row if-editor__readout-row--frozen"
    } else {
        "if-editor__readout-row"
    };

    rsx! {
        div { class: "{row_class}",
            div { class: "if-editor__readout-label", "{label}" }
            div { class: "if-editor__readout-tag", "{tag}" }
            div { class: "{bar_class}",
                div {
                    class: "if-editor__readout-fill",
                    style: "{bar_style}",
                }
            }
            div { class: "if-editor__readout-pct", "{pct_text}" }
        }
    }
}

/// Section divider with an inline label (e.g. `─── merge ───`).
///
/// Renders as a single grid cell that spans the full row and contains
/// dashed lines on either side of a small uppercase label, marking the
/// transition between the input section and the merged-or-output section.
#[component]
fn ReadoutDivider(label: String) -> Element {
    rsx! {
        div { class: "if-editor__readout-divider",
            span { class: "if-editor__readout-divider-label", "{label}" }
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
pub(crate) struct AxisDisplay {
    /// Normalized value in the polarity's natural domain.
    pub(crate) value: f64,
    pub(crate) polarity: AxisPolarity,
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
pub(crate) fn read_axis_display(
    addr: &InputAddress,
    live: &LiveSnapshot,
    cfg: &ConfigSnapshot,
) -> AxisDisplay {
    let Some(InputId::Axis { index }) = addr.input_id() else {
        return AxisDisplay {
            value: 0.0,
            polarity: AxisPolarity::Bipolar,
        };
    };
    let dev_idx = cfg
        .devices
        .iter()
        .position(|d| Some(&d.info.id) == addr.device());
    if let Some(di) = dev_idx
        && let Some(dev_inputs) = live.device_inputs.get(di)
        && let Some(&(raw, polarity)) = dev_inputs.axes.get(usize::from(*index))
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

/// Read whether the button at `addr` is currently pressed in the live
/// snapshot.
///
/// Returns `false` when the device or button index is not present
/// (engine offline, non-button input, or stale address).
pub(crate) fn read_button_pressed(
    addr: &InputAddress,
    live: &LiveSnapshot,
    cfg: &ConfigSnapshot,
) -> bool {
    let Some(InputId::Button { index }) = addr.input_id() else {
        return false;
    };
    let dev_idx = cfg
        .devices
        .iter()
        .position(|d| Some(&d.info.id) == addr.device());
    dev_idx
        .and_then(|di| live.device_inputs.get(di))
        .and_then(|dev_inputs| {
            // InputAddress stores buttons 0-indexed. Reconstruct the SDK
            // 1-indexed id, then convert back with checked_sub so malformed
            // or overflowing values fail closed.
            let one_indexed = index.checked_add(1)?;
            let zero_based = usize::from(one_indexed.checked_sub(1)?);
            dev_inputs.buttons.get(zero_based).copied()
        })
        .unwrap_or(false)
}

/// Read the hat direction at `addr` from the live snapshot. Returns
/// `HatDirection::Center` when the device or hat index is not present.
pub(crate) fn read_hat_direction(
    addr: &InputAddress,
    live: &LiveSnapshot,
    cfg: &ConfigSnapshot,
) -> HatDirection {
    let Some(InputId::Hat { index }) = addr.input_id() else {
        return HatDirection::Center;
    };
    let dev_idx = cfg
        .devices
        .iter()
        .position(|d| Some(&d.info.id) == addr.device());
    dev_idx
        .and_then(|di| live.device_inputs.get(di))
        .and_then(|dev_inputs| dev_inputs.hats.get(usize::from(*index)).copied())
        .unwrap_or(HatDirection::Center)
}

/// Read the engine output value for `out` from the live snapshot.
///
/// Mirrors `read_axis_display` but indexes into `live.output_values` (the
/// projection of the engine's output cache) instead of `device_inputs`.
/// The cache is written by the engine on tick and is never cleared by
/// `Activate`/`Deactivate`/`Pause`/`Resume`, so this naturally freezes at
/// the last engine value when the engine is stopped.
///
/// `polarity` is the inferred output polarity (from the merge op or, in
/// the no-merge case, from the primary input). The cache stores raw vJoy
/// floats (bipolar wire format); `into_natural_domain` remaps to the
/// row's polarity so a unipolar pedal mapped to vJoy Z still renders as
/// a unipolar bar.
///
/// Falls back to `value: 0.0` when the device or output id is not present
/// in the snapshot.
fn read_output_display(
    out: &OutputAddress,
    live: &LiveSnapshot,
    cfg: &ConfigSnapshot,
    polarity: AxisPolarity,
) -> AxisDisplay {
    let dev_idx = cfg
        .virtual_devices
        .iter()
        .position(|v| v.device_id == out.device);
    let raw = dev_idx
        .and_then(|di| live.output_values.get(di))
        .and_then(|vals| match out.output {
            OutputId::Axis { id } => vals
                .axes
                .iter()
                .find_map(|&(axis, value)| (axis == id).then_some(value)),
            OutputId::Button { id } => {
                // vJoy buttons are 1-indexed; `LiveSnapshot::from_state`
                // builds the buttons vec via `(1..=button_count)`.
                // `id == 0` is malformed input; treat it as "no value"
                // and let the outer fallback render an idle bar rather
                // than silently aliasing button 0 to button 1.
                let idx = usize::from(id.checked_sub(1)?);
                vals.buttons.get(idx).map(|&b| if b { 1.0 } else { 0.0 })
            }
            OutputId::Hat { .. } => None,
        })
        .unwrap_or(0.0);
    AxisDisplay {
        value: into_natural_domain(raw, polarity),
        polarity,
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

/// First `MapToVJoy` output address anywhere in the tree (including
/// inside `Conditional` branches), or `None` if no `MapToVJoy` exists.
///
/// Returns the address (not just a bool) so the OUT row's tag column
/// can show the destination label (e.g. `vJoy 1 · Y axis`) without a
/// second tree walk.
fn first_map_to_vjoy_output(actions: &[Action]) -> Option<OutputAddress> {
    for action in actions {
        match action {
            Action::MapToVJoy { output } => return Some(output.clone()),
            Action::Conditional {
                if_true, if_false, ..
            } => {
                if let Some(o) = first_map_to_vjoy_output(if_true) {
                    return Some(o);
                }
                if let Some(o) = first_map_to_vjoy_output(if_false) {
                    return Some(o);
                }
            }
            _ => {}
        }
    }
    None
}

/// Format a vJoy output address as `vJoy <device> · <axis|button|hat>`.
///
/// Mirrors the formatter in `header.rs` so the readout's OUT tag and
/// the header subtitle's output label read identically. Worth deduping
/// into a shared `output_label` module if a third call site appears.
fn format_output_label(output: &OutputAddress) -> String {
    let suffix = match output.output {
        OutputId::Axis { id } => match id {
            VJoyAxis::X => "X axis",
            VJoyAxis::Y => "Y axis",
            VJoyAxis::Z => "Z axis",
            VJoyAxis::Rx => "Rx axis",
            VJoyAxis::Ry => "Ry axis",
            VJoyAxis::Rz => "Rz axis",
            VJoyAxis::Slider0 => "Slider 0",
            VJoyAxis::Slider1 => "Slider 1",
        }
        .to_owned(),
        OutputId::Button { id } => format!("Button {id}"),
        OutputId::Hat { id } => format!("Hat {id}"),
    };
    format!("vJoy {} \u{00b7} {}", output.device, suffix)
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
        InputAddress::Bound {
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

    #[test]
    fn find_merge_context_skips_conditional_nested_merge() {
        use inputforge_core::action::Condition;
        // A merge nested inside a Conditional branch is intentionally
        // ignored; the merge layout only triggers for top-level merges.
        let actions = vec![Action::Conditional {
            condition: Condition::ButtonPressed {
                input: axis_addr(2),
            },
            if_true: vec![Action::MergeAxis {
                second_input: axis_addr(1),
                operation: MergeOp::Bidirectional,
            }],
            if_false: Vec::new(),
        }];
        assert!(find_merge_context(&actions).is_none());
    }
}
