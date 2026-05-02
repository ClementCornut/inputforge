// Rust guideline compliant 2026-05-02

//! 28x14 inline-SVG curve thumbnail used in F9's stage-header right slot.

use dioxus::prelude::*;

use inputforge_core::processing::curves::{ResponseCurve, sample_curve_path};

/// Number of sample points used to approximate the curve polyline.
///
/// 30 points balances visual fidelity at 28x14 px against the byte size of
/// the rendered `points` attribute. Increasing this past ~40 yields no visible
/// improvement at thumbnail scale while growing the serialized SVG.
const THUMBNAIL_SAMPLE_COUNT: usize = 30;

/// Renders a 28x14 inline SVG thumbnail of `curve`.
///
/// The thumbnail is intended for the F9 stage-header right slot. It shows a
/// single polyline with no grid, no anchors, and no interactive elements.
/// `aria-hidden="true"` hides it from assistive technology since the stage
/// header already carries a text description.
///
/// The `viewBox="-1.05 -1.05 2.1 2.1"` with `preserveAspectRatio="none"`
/// stretches the unit-square curve domain into the 2:1 thumbnail rectangle.
/// `vector-effect="non-scaling-stroke"` keeps the stroke width uniform so
/// vertical strokes are not rendered ~2x thicker than horizontal ones.
pub(crate) fn header_thumbnail(curve: &ResponseCurve) -> Element {
    let samples = sample_curve_path(curve, THUMBNAIL_SAMPLE_COUNT);
    // 4-decimal precision is byte-stable across platforms for snapshot
    // tests and is well below the 0.12 stroke width (no visible aliasing).
    let points = samples
        .iter()
        .map(|(x, y)| format!("{x:.4},{y:.4}"))
        .collect::<Vec<_>>()
        .join(" ");
    rsx! {
        svg {
            class: "if-curve__thumbnail",
            width: "28",
            height: "14",
            view_box: "-1.05 -1.05 2.1 2.1",
            preserve_aspect_ratio: "none",
            "aria-hidden": "true",
            g {
                transform: "scale(1, -1)",
                polyline {
                    points: "{points}",
                    fill: "none",
                    stroke: "currentColor",
                    "stroke-width": "0.12",
                    "stroke-linecap": "round",
                    "stroke-linejoin": "round",
                    // `preserveAspectRatio="none"` stretches the 2.1x2.1
                    // viewBox into 28x14 (2:1 ratio). Without
                    // non-scaling-stroke, vertical strokes become ~2x
                    // thicker than horizontal ones.
                    "vector-effect": "non-scaling-stroke",
                }
            }
        }
    }
}
