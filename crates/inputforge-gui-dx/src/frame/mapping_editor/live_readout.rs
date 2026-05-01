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
use inputforge_core::types::{AxisPolarity, InputAddress, InputId, InputValue};

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
    let merge_secondary = first_merge_secondary(&actions);
    let merge_index = first_merge_index(&actions);
    let out_present = first_map_to_vjoy(&actions);

    // Brief read-lock on AppState for pipeline evaluation.
    // Two separate guard scopes so Rust can prove non-overlapping borrows.
    let merged_in_value = merge_index.map(|idx| {
        let state = ctx.state.read();
        let iv = inputforge_core::pipeline::evaluate_actions_through(
            &actions,
            &state,
            &primary,
            idx + 1,
        );
        AxisDisplay {
            value: axis_f64(&iv),
            polarity: primary_value.polarity,
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
            value: axis_f64(&iv),
            polarity: primary_value.polarity,
        }
    });

    rsx! {
        div { class: "if-editor__readout",
            if let Some(secondary_addr) = merge_secondary {
                // Merge layout: IN 1, IN 2, dashed divider, merged IN, OUT.
                // No extra divider before OUT in the merge case (spec line 417).
                ReadoutRow { label: "IN 1".to_owned(), display: primary_value }
                {
                    let secondary_val = read_axis_display(&secondary_addr, &live, &cfg);
                    rsx! { ReadoutRow { label: "IN 2".to_owned(), display: secondary_val } }
                }
                div { class: "if-editor__readout-divider-dashed" }
                ReadoutRow {
                    label: "IN".to_owned(),
                    display: merged_in_value.unwrap_or(primary_value),
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
    // Clamp to [0, 100] — values outside that range are always clipping.
    let fill_pct = (display.value.abs() * 100.0).clamp(0.0, 100.0);
    let bipolar = matches!(display.polarity, AxisPolarity::Bipolar);

    // Inline style positions the live-fill bar. Bipolar anchors at 50%,
    // negative values extend left, positive extend right. Unipolar always
    // extends from the left edge.
    let bar_style = if bipolar && display.value < 0.0 {
        format!("right: 50%; width: {fill_pct}%;")
    } else if bipolar {
        format!("left: 50%; width: {fill_pct}%;")
    } else {
        format!("left: 0; width: {fill_pct}%;")
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
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct AxisDisplay {
    /// Normalized value in [-1.0, 1.0].
    pub value: f64,
    pub polarity: AxisPolarity,
}

/// Read the raw axis value and polarity for `addr` from the live snapshot.
///
/// Falls back to `(0.0, Bipolar)` when the device or axis index is not
/// present in the snapshot (e.g. engine offline or non-axis input).
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
    if let Some(di) = dev_idx {
        if let Some(dev_inputs) = live.device_inputs.get(di) {
            if let Some(&(value, polarity)) = dev_inputs.axes.get(usize::from(index)) {
                return AxisDisplay { value, polarity };
            }
        }
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
        InputValue::Axis { value } => value.value(),
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

/// Index of the first top-level `MergeAxis` in `actions`.
///
/// Top-level only by design: merges nested inside `Conditional` do not
/// trigger the merge layout (acceptable per the task spec).
fn first_merge_index(actions: &[Action]) -> Option<usize> {
    actions
        .iter()
        .position(|a| matches!(a, Action::MergeAxis { .. }))
}

/// `second_input` of the first top-level `MergeAxis`, if present.
fn first_merge_secondary(actions: &[Action]) -> Option<InputAddress> {
    actions.iter().find_map(|a| {
        if let Action::MergeAxis { second_input, .. } = a {
            Some(second_input.clone())
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
// Formatting
// ---------------------------------------------------------------------------

/// Format a percentage string for the readout label.
///
/// Bipolar axes show a sign prefix (`+0.00` / `-0.00`) so the center is
/// unambiguous. Unipolar axes omit the sign.
fn format_percentage(display: &AxisDisplay) -> String {
    match display.polarity {
        AxisPolarity::Bipolar => format!("{:+.2}", display.value),
        AxisPolarity::Unipolar => format!("{:.2}", display.value),
    }
}
