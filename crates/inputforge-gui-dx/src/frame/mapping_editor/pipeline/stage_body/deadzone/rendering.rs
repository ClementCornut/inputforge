// Rust guideline compliant 2026-05-03

//! F11 SVG layer stack. Renders zone-banded background, grid, axis cross,
//! identity diagonal, deadzone curve polyline, four handles, hover/drag/focus
//! decorations, optional live tracking dot, and tick labels.

use dioxus::prelude::*;
#[path = "render_decorations.rs"]
mod render_decorations;
use render_decorations::{
    render_drag_halo, render_focus_ring, render_hover_ring, render_live, render_tick_labels,
};

use inputforge_core::processing::deadzone::DeadzoneConfig;

use super::super::instruments::geometry::handle_positions;
use crate::frame::mapping_editor::pipeline::stage_body::deadzone::state::BodyState;
use crate::frame::mapping_editor::pipeline::stage_body::instruments::INSTR_GLOW_STDDEV;

pub(crate) fn render_plot(
    config: &DeadzoneConfig,
    state: &BodyState,
    live_value: Option<f64>,
    glow_id: &str,
) -> Element {
    let live_pair = live_value.map(|v| (v, config.apply(v)));
    rsx! {
        svg {
            class: "if-deadzone__plot",
            style: format!("--instrument-glow: url(#{glow_id});"),
            view_box: "-1.05 -1.05 2.1 2.1",
            preserve_aspect_ratio: "xMidYMid meet",
            title { "deadzone curve" }
            defs {
                filter {
                    id: glow_id,
                    x: "-50%", y: "-50%", width: "200%", height: "200%",
                    feGaussianBlur { std_deviation: "{INSTR_GLOW_STDDEV}" }
                }
            }
            // Layer 1: background (root, not flipped).
            rect {
                class: "if-deadzone__bg",
                x: "-1.05", y: "-1.05", width: "2.1", height: "2.1",
            }
            g {
                transform: "scale(1, -1)",
                {render_zone_bands(config)}
                {render_grid(config)}
                {render_axis_cross()}
                {render_identity()}
                {render_curve(config)}
                // Live signal sits BELOW the handles so a coincident input
                // value (input ≈ 0 over the stacked CL/CH at default config,
                // or held against a saturation rail) does not occlude the
                // handle the user wants to grab. The 0.07-radius halo extends
                // ~0.05 past the 0.022 handle perimeter, so the green wash
                // still reads as the live channel without hiding the disc.
                if let Some((input, output)) = live_pair {
                    {render_live(input, output)}
                }
                {render_handles(config, state)}
                {render_hover_ring(config, state)}
                {render_drag_halo(config, state)}
                {render_focus_ring(config, state)}
            }
            {render_tick_labels()}
        }
    }
}

fn render_zone_bands(config: &DeadzoneConfig) -> Element {
    let l = config.low();
    let cl = config.center_low();
    let ch = config.center_high();
    let h = config.high();
    // Bands are anchored at the live `-1..1` region (NOT the `-1.05..1.05`
    // viewBox padding). At default config (low=-1, high=1) this collapses
    // the saturated bands to width 0 instead of leaking a visible 0.05-unit
    // tint outside the live region. `(l - -1.0).max(0.0)` guards against
    // any future config where low briefly exceeds -1 mid-validation.
    rsx! {
        // Outer saturated bands.
        rect { class: "if-deadzone__zone if-deadzone__zone--sat", x: "-1.0", y: "-1.0", width: "{(l - -1.0).max(0.0)}", height: "2.0" }
        rect { class: "if-deadzone__zone if-deadzone__zone--sat", x: "{h}", y: "-1.0", width: "{(1.0 - h).max(0.0)}", height: "2.0" }
        // Ramp bands.
        rect { class: "if-deadzone__zone if-deadzone__zone--ramp", x: "{l}", y: "-1.0", width: "{(cl - l).max(0.0)}", height: "2.0" }
        rect { class: "if-deadzone__zone if-deadzone__zone--ramp", x: "{ch}", y: "-1.0", width: "{(h - ch).max(0.0)}", height: "2.0" }
        // Dead band.
        rect { class: "if-deadzone__zone if-deadzone__zone--dead", x: "{cl}", y: "-1.0", width: "{(ch - cl).max(0.0)}", height: "2.0" }
    }
}

fn render_grid(_config: &DeadzoneConfig) -> Element {
    let lines = [-0.75, -0.5, -0.25, 0.25, 0.5, 0.75];
    rsx! {
        g {
            class: "if-deadzone__grid",
            for x in lines.iter() {
                line { class: "if-deadzone__grid-major", x1: "{x}", y1: "-1.0", x2: "{x}", y2: "1.0" }
                line { class: "if-deadzone__grid-major", x1: "-1.0", y1: "{x}", x2: "1.0", y2: "{x}" }
            }
        }
    }
}

fn render_axis_cross() -> Element {
    rsx! {
        g {
            line { class: "if-deadzone__axis-cross", x1: "0", y1: "-1.0", x2: "0", y2: "1.0" }
            line { class: "if-deadzone__axis-cross", x1: "-1.0", y1: "0", x2: "1.0", y2: "0" }
        }
    }
}

fn render_identity() -> Element {
    rsx! {
        line {
            class: "if-deadzone__identity",
            x1: "-1.0", y1: "-1.0", x2: "1.0", y2: "1.0",
        }
    }
}

fn render_curve(config: &DeadzoneConfig) -> Element {
    let l = config.low();
    let cl = config.center_low();
    let ch = config.center_high();
    let h = config.high();
    let pts = format!("-1,-1 {l},-1 {cl},0 {ch},0 {h},1 1,1");
    rsx! {
        polyline {
            class: "if-deadzone__path",
            points: "{pts}",
        }
    }
}

fn render_handles(config: &DeadzoneConfig, _state: &BodyState) -> Element {
    let p = handle_positions(config);
    rsx! {
        g {
            for (px, py) in p.iter() {
                circle { class: "if-deadzone__handle", cx: "{px}", cy: "{py}", r: "0.022" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus_ssr::render;

    #[test]
    fn render_plot_emits_expected_class_count() {
        let cfg = DeadzoneConfig::default();
        let state = BodyState::default();
        let html = {
            let mut dom = VirtualDom::new_with_props(
                |props: RenderProps| render_plot(&props.cfg, &props.state, None, "test-instr-glow"),
                RenderProps {
                    cfg: cfg.clone(),
                    state: state.clone(),
                },
            );
            dom.rebuild_in_place();
            render(&dom)
        };
        assert!(html.contains("if-deadzone__bg"));
        assert!(html.contains("if-deadzone__zone--sat"));
        assert!(html.contains("if-deadzone__zone--ramp"));
        assert!(html.contains("if-deadzone__zone--dead"));
        assert!(html.contains("if-deadzone__path"));
        assert!(html.contains("if-deadzone__handle"));
        assert!(html.contains("test-instr-glow"));
    }

    #[derive(Props, Clone, PartialEq)]
    struct RenderProps {
        cfg: DeadzoneConfig,
        state: BodyState,
    }
}
