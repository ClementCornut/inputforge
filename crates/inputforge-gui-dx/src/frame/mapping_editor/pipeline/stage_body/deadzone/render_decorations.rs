//! Pure SVG focus, interaction and live-signal layers.
use crate::frame::mapping_editor::pipeline::stage_body::{
    deadzone::state::BodyState, instruments::geometry::handle_positions,
};
use dioxus::prelude::*;
use inputforge_core::processing::deadzone::DeadzoneConfig;
pub(super) fn render_hover_ring(config: &DeadzoneConfig, state: &BodyState) -> Element {
    // Pending-split takes precedence: when a pointer-down landed on the
    // stacked center pair, draw a hover ring on BOTH candidates so the
    // user reads "drag direction will pick" before they commit.
    if let Some((a, b)) = state.pending_split {
        let p = handle_positions(config);
        let idx_a = a;
        let idx_b = b;
        let pos_a = p[idx_a];
        let pos_b = p[idx_b];
        return rsx! {
            circle { class: "if-deadzone__hover-ring", cx: "{pos_a.0}", cy: "{pos_a.1}", r: "0.085", fill: "none" }
            circle { class: "if-deadzone__hover-ring", cx: "{pos_b.0}", cy: "{pos_b.1}", r: "0.085", fill: "none" }
        };
    }
    let Some(handle) = state.hovered_handle else {
        return rsx! {};
    };
    let p = handle_positions(config);
    let idx = handle;
    let (cx, cy) = p[idx];
    rsx! {
        circle { class: "if-deadzone__hover-ring", cx: "{cx}", cy: "{cy}", r: "0.085", fill: "none" }
    }
}

pub(super) fn render_drag_halo(config: &DeadzoneConfig, state: &BodyState) -> Element {
    let Some(ref drag) = state.dragging else {
        return rsx! {};
    };
    let p = handle_positions(config);
    let idx = drag.handle;
    let (cx, cy) = p[idx];
    rsx! {
        circle { class: "if-deadzone__drag-halo", cx: "{cx}", cy: "{cy}", r: "0.07" }
    }
}

pub(super) fn render_focus_ring(config: &DeadzoneConfig, state: &BodyState) -> Element {
    let Some(handle) = state.focused_handle else {
        return rsx! {};
    };
    let p = handle_positions(config);
    let idx = handle;
    let (cx, cy) = p[idx];
    rsx! {
        circle { class: "if-deadzone__focus-ring", cx: "{cx}", cy: "{cy}", r: "0.105", fill: "none" }
    }
}

pub(super) fn render_live(input: f64, output: f64) -> Element {
    rsx! {
        // Horizontal guide.
        line { class: "if-deadzone__live-guide", x1: "-1.0", y1: "{output}", x2: "{input}", y2: "{output}" }
        // Vertical guide anchored at the axis cross (deadzone bipolarity).
        line { class: "if-deadzone__live-guide", x1: "{input}", y1: "0", x2: "{input}", y2: "{output}" }
        circle { class: "if-deadzone__live-dot-halo", cx: "{input}", cy: "{output}", r: "0.07" }
        circle { class: "if-deadzone__live-dot", cx: "{input}", cy: "{output}", r: "0.04" }
    }
}

pub(super) fn render_tick_labels() -> Element {
    // Hardcoded label table mirrors F10 (`response_curve/rendering.rs:284-292`).
    // The previous algorithmic version produced asymmetric output for negatives
    // (e.g. -0.5 -> "-0.5", but 0.5 -> ".5") because trim_start_matches('0')
    // stripped nothing past the leading '-'. Lookup table avoids the trap.
    let xs = [
        (-1.0_f64, "-1"),
        (-0.5, "-.5"),
        (0.0, "0"),
        (0.5, ".5"),
        (1.0, "1"),
    ];
    let ys = [(-1.0_f64, "-1"), (0.0, "0"), (1.0, "1")];
    rsx! {
        g {
            class: "if-deadzone__ticks",
            for (x, lbl) in xs.iter().copied() {
                text {
                    key: "tx-{x}",
                    class: "if-deadzone__tick-label",
                    x: "{x}", y: "1.04",
                    text_anchor: "middle",
                    "{lbl}"
                }
            }
            for (y, lbl) in ys.iter().copied() {
                text {
                    key: "ty-{y}",
                    class: "if-deadzone__tick-label",
                    x: "-1.0", dx: "0.015", y: "{-y}",
                    text_anchor: "start",
                    dominant_baseline: "central",
                    "{lbl}"
                }
            }
        }
    }
}
