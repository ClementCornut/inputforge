// Rust guideline compliant 2026-05-02

//! SVG rendering helpers for the F10 curve-editor body.
//!
//! All colors come from CSS custom properties on `.if-curve` (defined
//! in `assets/frame/response_curve.css`); render fns emit class names
//! only. The y-flip group ensures positive output points up; tick
//! labels render outside the flip so text is not mirrored.

use dioxus::prelude::*;

use inputforge_core::processing::curves::ResponseCurve;

use super::state::BodyState;
use crate::frame::mapping_editor::pipeline::stage_body::instruments::INSTR_GLOW_STDDEV;

/// Top-level plot. Composes the layered SVG and returns it as a single
/// `Element`. Layer ordering matches the spec's stack table.
pub(crate) fn render_plot(
    curve: &ResponseCurve,
    state: &BodyState,
    live_value: Option<f64>,
    plot_size_px: f64,
) -> Element {
    let bezier_handles = matches!(curve, ResponseCurve::CubicBezier { .. });
    let live_output = live_value.map(|v| (v, curve.evaluate(v)));
    let _ = plot_size_px; // size is owned by CSS via `aspect-ratio: 1 / 1`.
    rsx! {
        svg {
            class: "if-curve__plot",
            view_box: "-1.05 -1.05 2.1 2.1",
            preserve_aspect_ratio: "xMidYMid meet",
            // aria-label lives on the focusable wrapper <div> (Task 12),
            // not on the inner <svg>. Inner <svg> exposes a <title> for
            // screen readers that descend by default.
            title { "response curve" }
            // Filter defs.
            defs {
                filter {
                    id: "if-instr-glow",
                    x: "-50%", y: "-50%", width: "200%", height: "200%",
                    feGaussianBlur { std_deviation: "{INSTR_GLOW_STDDEV}" }
                }
            }
            // Background.
            rect {
                class: "if-curve__bg",
                x: "-1.05", y: "-1.05", width: "2.1", height: "2.1",
            }
            // Y-flipped layers.
            g {
                transform: "scale(1, -1)",
                {render_grid_major()}
                {render_axis_cross()}
                {render_identity()}
                if bezier_handles {
                    {render_bezier_handle_lines(curve)}
                }
                {render_curve_path(state)}
                {render_anchors(curve, state)}
                {render_handle_markers(curve, state)}
                {render_hover_ring(curve, state)}
                {render_drag_halo(curve, state)}
                {render_focus_ring(curve, state)}
                if let Some((input, output)) = live_output {
                    {render_live(input, output)}
                }
            }
            // Tick labels: outside the flip so text is upright.
            {render_tick_labels()}
        }
    }
}

fn render_grid_major() -> Element {
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
fn render_axis_cross() -> Element {
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

fn render_identity() -> Element {
    rsx! {
        line {
            class: "if-curve__identity",
            x1: "-1.0", y1: "-1.0", x2: "1.0", y2: "1.0",
        }
    }
}

fn render_bezier_handle_lines(curve: &ResponseCurve) -> Element {
    let ResponseCurve::CubicBezier { segments, .. } = curve else {
        return rsx! { g {} };
    };
    rsx! {
        g {
            for (i, seg) in segments.iter().enumerate() {
                line {
                    key: "h1-{i}",
                    class: "if-curve__handle-line",
                    x1: "{seg.start.0}", y1: "{seg.start.1}",
                    x2: "{seg.control1.0}", y2: "{seg.control1.1}",
                }
                line {
                    class: "if-curve__handle-line",
                    x1: "{seg.control2.0}", y1: "{seg.control2.1}",
                    x2: "{seg.end.0}", y2: "{seg.end.1}",
                }
            }
        }
    }
}

fn render_curve_path(state: &BodyState) -> Element {
    // 4-decimal precision = 0.0001 viewBox units, well below the 0.025
    // stroke width. Output is byte-stable across platforms (matters for
    // SSR snapshot assertions) and ~6 KB shorter than `{x},{y}` at 200
    // samples.
    let points = state
        .cached_path
        .iter()
        .map(|(x, y)| format!("{x:.4},{y:.4}"))
        .collect::<Vec<_>>()
        .join(" ");
    rsx! {
        polyline {
            class: "if-curve__path",
            points: "{points}",
            fill: "none",
        }
    }
}

// Spec layer 7: anchors only.
fn render_anchors(curve: &ResponseCurve, state: &BodyState) -> Element {
    let bezier = matches!(curve, ResponseCurve::CubicBezier { .. });
    rsx! {
        g {
            // Anchor radius 0.022 = 5px at 480px plot. Tighter than the
            // plan's 0.04 (which read as "massive blobs" against the new
            // 1.75px curve stroke). Hit-test still uses HIT_RADIUS_PX=10
            // so click targets stay generous regardless of visual size.
            for (i, &(x, y)) in state.cached_anchors.iter().enumerate() {
                if !(bezier && matches!(i % 4, 1 | 2)) {
                    circle {
                        key: "anchor-{i}",
                        class: "if-curve__anchor",
                        cx: "{x}", cy: "{y}", r: "0.022",
                    }
                }
            }
        }
    }
}

// Spec layer 8: bezier handle markers (diamonds), rendered AFTER
// anchors so handle markers stack on top when they coincide.
fn render_handle_markers(curve: &ResponseCurve, state: &BodyState) -> Element {
    let ResponseCurve::CubicBezier { .. } = curve else {
        return rsx! { g {} };
    };
    rsx! {
        g {
            // Handle marker 0.030 wide → 0.042 diagonal (≈9.6px at 480px plot).
            // Slightly larger than the 0.022-radius anchors so handles read
            // as discoverable hit targets without dominating the plot.
            for (i, &(x, y)) in state.cached_anchors.iter().enumerate() {
                if matches!(i % 4, 1 | 2) {
                    rect {
                        key: "handle-{i}",
                        class: "if-curve__handle-marker",
                        x: "{x - 0.015}", y: "{y - 0.015}",
                        width: "0.030", height: "0.030",
                        transform: "rotate(45 {x} {y})",
                    }
                }
            }
        }
    }
}

fn render_hover_ring(_curve: &ResponseCurve, state: &BodyState) -> Element {
    let Some(idx) = state.hovered_point else {
        return rsx! { g {} };
    };
    let Some(&(x, y)) = state.cached_anchors.get(idx) else {
        return rsx! { g {} };
    };
    rsx! {
        circle {
            class: "if-curve__hover-ring",
            cx: "{x}", cy: "{y}", r: "0.085",
            fill: "none",
        }
    }
}

fn render_drag_halo(_curve: &ResponseCurve, state: &BodyState) -> Element {
    let Some(drag) = state.dragging.as_ref() else {
        return rsx! { g {} };
    };
    let Some(&(x, y)) = state.cached_anchors.get(drag.point_index) else {
        return rsx! { g {} };
    };
    rsx! {
        circle {
            class: "if-curve__drag-halo",
            cx: "{x}", cy: "{y}", r: "0.07",
        }
    }
}

fn render_focus_ring(_curve: &ResponseCurve, state: &BodyState) -> Element {
    let Some(idx) = state.focused_point else {
        return rsx! { g {} };
    };
    let Some(&(x, y)) = state.cached_anchors.get(idx) else {
        return rsx! { g {} };
    };
    rsx! {
        circle {
            class: "if-curve__focus-ring",
            cx: "{x}", cy: "{y}", r: "0.105",
            fill: "none",
        }
    }
}

fn render_live(input: f64, output: f64) -> Element {
    rsx! {
        g {
            line {
                class: "if-curve__live-guide",
                x1: "-1.0", y1: "{output}", x2: "{input}", y2: "{output}",
            }
            circle {
                class: "if-curve__live-dot-halo",
                cx: "{input}", cy: "{output}", r: "0.07",
            }
            circle {
                class: "if-curve__live-dot",
                cx: "{input}", cy: "{output}", r: "0.04",
            }
        }
    }
}

fn render_tick_labels() -> Element {
    let xs = [
        (-1.0_f64, "-1"),
        (-0.5, "-.5"),
        (0.0, "0"),
        (0.5, ".5"),
        (1.0, "1"),
    ];
    let ys = [(-1.0_f64, "-1"), (0.0, "0"), (1.0, "1")];
    // Y-axis labels render INSIDE the plot near the left axis. The original
    // x="-1.04" + text-anchor="end" placed text at SVG pixel ~2 and grew it
    // leftward, falling outside the SVG drawing area's overflow:hidden bounds.
    // Anchoring at start with x=-1.0 + dx=0.015 nudges the text just inside
    // the plot frame; dominant-baseline="central" centers the glyph on the y
    // tick position so the "0" label sits on the y=0 grid line, not above it.
    rsx! {
        g {
            class: "if-curve__ticks",
            for (x, lbl) in xs.iter().copied() {
                text {
                    key: "tx-{x}",
                    class: "if-curve__tick-label",
                    x: "{x}", y: "1.04",
                    text_anchor: "middle",
                    "{lbl}"
                }
            }
            for (y, lbl) in ys.iter().copied() {
                text {
                    key: "ty-{y}",
                    class: "if-curve__tick-label",
                    x: "-0.98", y: "{-y}",
                    text_anchor: "start",
                    dominant_baseline: "central",
                    "{lbl}"
                }
            }
        }
    }
}
