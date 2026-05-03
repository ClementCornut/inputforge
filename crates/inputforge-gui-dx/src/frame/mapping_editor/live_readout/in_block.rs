use dioxus::prelude::*;

use inputforge_core::types::AxisPolarity;

use super::value_helpers::{AxisDisplay, format_percentage};

/// One row in the readout grid: label, tag, bar, percentage text.
///
/// The bar fill is anchored at 50% for bipolar axes and at 0% for
/// unipolar axes, matching the live-green visual described in the spec.
/// Bipolar bars also carry a modifier class so the CSS can draw a center
/// tick that communicates polarity at idle.
///
/// `frozen` is true only on the OUT row when the engine is not running.
/// CSS dims the bar fill and percentage to signal a held value; label
/// and tag stay at full strength because they describe configuration.
#[component]
pub(super) fn ReadoutRow(
    label: String,
    tag: String,
    display: AxisDisplay,
    frozen: bool,
) -> Element {
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

    // Always set both `left` and `right`, with `auto` for the off side,
    // so Dioxus's per-property style diffing cannot leave a stale anchor
    // from a previous render. Without this, switching from negative to
    // positive or vice versa keeps the prior side's `50%` set. CSS then
    // resolves `left: 50%; right: 50%; width: <pct>%;` by honoring
    // `left + width` regardless of sign, so both signs grow rightward and
    // overflow the container.
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

/// Section divider with an inline label.
///
/// Renders as a single grid cell that spans the full row and contains
/// dashed lines on either side of a small uppercase label, marking the
/// transition between the input section and the merged-or-output section.
#[component]
pub(super) fn ReadoutDivider(label: String) -> Element {
    rsx! {
        div { class: "if-editor__readout-divider",
            span { class: "if-editor__readout-divider-label", "{label}" }
        }
    }
}
