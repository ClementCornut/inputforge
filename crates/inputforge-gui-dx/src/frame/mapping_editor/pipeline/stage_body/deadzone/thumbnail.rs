// Rust guideline compliant 2026-05-03

//! F11 stage-header right-slot thumbnail. A 28x14 SVG mini zone bar that
//! replaces the default chevron when the stage is collapsed (and scales as
//! the right-slot content for the F2 `IconButton`'s invariant 32x32 hit area).

use dioxus::prelude::*;

use inputforge_core::processing::deadzone::DeadzoneConfig;

/// Map a [-1, 1] viewBox-x value to the thumbnail's [0, 28] x range.
fn x_for(v: f64) -> f64 {
    (v + 1.0) * 14.0
}

pub(crate) fn header_thumbnail(config: &DeadzoneConfig) -> Element {
    let l = x_for(config.low());
    let cl = x_for(config.center_low());
    let ch = x_for(config.center_high());
    let h = x_for(config.high());
    rsx! {
        svg {
            class: "if-deadzone-thumb",
            view_box: "0 0 28 14",
            width: "28", height: "14",
            // Saturated outer bands (red, 55%).
            rect { fill: "var(--color-error)", fill_opacity: "0.55", x: "0", y: "0", width: "{l}", height: "14" }
            rect { fill: "var(--color-error)", fill_opacity: "0.55", x: "{h}", y: "0", width: "{28.0 - h}", height: "14" }
            // Ramp bands (blue, 30%).
            rect { fill: "var(--color-primary)", fill_opacity: "0.30", x: "{l}", y: "0", width: "{cl - l}", height: "14" }
            rect { fill: "var(--color-primary)", fill_opacity: "0.30", x: "{ch}", y: "0", width: "{h - ch}", height: "14" }
            // Dead band (sunken).
            rect { fill: "var(--color-bg-sunken)", x: "{cl}", y: "0", width: "{ch - cl}", height: "14" }
            // Threshold marks: 0.4px white-with-50% lines at the four positions.
            line { stroke: "white", stroke_opacity: "0.5", stroke_width: "0.4", x1: "{l}",  y1: "0", x2: "{l}",  y2: "14" }
            line { stroke: "white", stroke_opacity: "0.5", stroke_width: "0.4", x1: "{cl}", y1: "0", x2: "{cl}", y2: "14" }
            line { stroke: "white", stroke_opacity: "0.5", stroke_width: "0.4", x1: "{ch}", y1: "0", x2: "{ch}", y2: "14" }
            line { stroke: "white", stroke_opacity: "0.5", stroke_width: "0.4", x1: "{h}",  y1: "0", x2: "{h}",  y2: "14" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus_ssr::render;

    fn html_for(cfg: DeadzoneConfig) -> String {
        let mut dom = VirtualDom::new_with_props(
            |props: HtmlProps| header_thumbnail(&props.cfg),
            HtmlProps { cfg },
        );
        dom.rebuild_in_place();
        render(&dom)
    }

    #[derive(Props, Clone, PartialEq)]
    struct HtmlProps {
        cfg: DeadzoneConfig,
    }

    #[test]
    fn default_config_renders_full_outer_dead() {
        let html = html_for(DeadzoneConfig::default());
        // Default: low=-1, cl=0, ch=0, high=1.
        // Saturated rects collapse to width 0 at the edges, ramp covers
        // the full halves, dead band is zero-width. Exactly THREE rects
        // should have width=0 (left sat, right sat, dead band).
        assert_eq!(html.matches(r#"width="0""#).count(), 3);
    }

    #[test]
    fn aggressive_config_has_visible_outer_sat() {
        let cfg = DeadzoneConfig::new(-0.5, -0.1, 0.1, 0.5).expect("valid");
        let html = html_for(cfg);
        // x_for(-0.5) = 7.0, so left sat rect starts at width 7.
        assert!(html.contains(r#"width="7""#));
    }

    #[test]
    fn wide_dead_band_is_visible() {
        let cfg = DeadzoneConfig::new(-0.85, -0.3, 0.3, 0.85).expect("valid");
        let html = html_for(cfg);
        // Dead band: cl=-0.3 -> 9.8, ch=0.3 -> 18.2, width 8.4.
        // Approximate: the rendered width attr is 8.4 or 8.4000... depending on Rust f64 fmt.
        assert!(html.contains("8.4"));
    }
}
