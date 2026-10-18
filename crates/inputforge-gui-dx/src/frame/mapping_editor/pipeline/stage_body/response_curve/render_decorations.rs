//! Pure SVG focus, interaction and live-signal layers.
use crate::frame::mapping_editor::pipeline::stage_body::response_curve::state::BodyState;
use dioxus::prelude::*;
use inputforge_core::processing::curves::ResponseCurve;
pub(super) fn render_hover_ring(_curve: &ResponseCurve, state: &BodyState) -> Element {
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

pub(super) fn render_drag_halo(_curve: &ResponseCurve, state: &BodyState) -> Element {
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

pub(super) fn render_focus_ring(_curve: &ResponseCurve, state: &BodyState) -> Element {
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

pub(super) fn render_live(input: f64, output: f64) -> Element {
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

pub(super) fn render_tick_labels() -> Element {
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
