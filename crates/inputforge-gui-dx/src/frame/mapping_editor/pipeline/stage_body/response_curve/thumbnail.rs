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
    // 4-decimal precision is byte-stable across platforms for snapshot tests
    // and is well below the rendered stroke (rounded to ~1.5 CSS px), so no
    // visible aliasing.
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
            // Recessed instrument plate. Mirrors the main plot's
            // `bg-sunken` canvas at thumbnail scale so the same cyan-blue
            // stroke gains the same contrast lift it has on the main plot.
            // Sharp edges match the main `.if-curve__plot-frame` (no radius).
            rect {
                class: "if-curve__thumbnail-bg",
                x: "-1.05",
                y: "-1.05",
                width: "2.1",
                height: "2.1",
            }
            g {
                transform: "scale(1, -1)",
                polyline {
                    points: "{points}",
                    fill: "none",
                    stroke: "currentColor",
                    // With `vector-effect: non-scaling-stroke` this value
                    // resolves in CSS pixels regardless of the 13.33x6.67
                    // viewBox stretch. 1.5 mirrors the main plot's 1.75px
                    // proportionally and is the minimum that reads cleanly
                    // at 28x14 against the recessed plate.
                    "stroke-width": "1.5",
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
