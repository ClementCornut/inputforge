//! Pure SVG grid/axis layers.
use dioxus::prelude::*;
pub(super) fn render_grid_major() -> Element {
    // Origin (0.0) is excluded; render_axis_cross owns the x=0 and y=0
    // lines with slightly stronger ink.
    let majors = [-0.75_f64, -0.5, -0.25, 0.25, 0.5, 0.75];
    rsx! {
        g {
            for v in majors.iter().copied() {
                line {
                    key: "vmaj-{v}",
                    class: "if-curve__grid-major",
                    x1: "{v}", y1: "-1.0", x2: "{v}", y2: "1.0",
                }
                line {
                    class: "if-curve__grid-major",
                    x1: "-1.0", y1: "{v}", x2: "1.0", y2: "{v}",
                }
            }
        }
    }
}

/// Single-line cross at the origin (x=0 and y=0). The bipolar response
/// curve's origin is the most-referenced point: identity passes through it,
/// dead-center sits there, symmetric curves mirror around it. A slightly
/// stronger ink than the major grid earns its place by anchoring the eye
/// without competing with the curve stroke.
pub(super) fn render_axis_cross() -> Element {
    rsx! {
        g {
            line {
                class: "if-curve__axis-cross",
                x1: "0", y1: "-1.0", x2: "0", y2: "1.0",
            }
            line {
                class: "if-curve__axis-cross",
                x1: "-1.0", y1: "0", x2: "1.0", y2: "0",
            }
        }
    }
}

pub(super) fn render_identity() -> Element {
    rsx! {
        line {
            class: "if-curve__identity",
            x1: "-1.0", y1: "-1.0", x2: "1.0", y2: "1.0",
        }
    }
}
