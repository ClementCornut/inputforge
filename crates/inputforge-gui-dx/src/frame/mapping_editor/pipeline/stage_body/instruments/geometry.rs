//! Screen/viewbox projection and pointer hit measurements.
use crate::instruments::{Observation, Pointer};
use inputforge_core::{
    action::Action,
    processing::{curves::ResponseCurve, deadzone::DeadzoneConfig},
};
#[derive(Debug, Clone, Copy)]
pub(crate) struct PlotRect {
    pub x: f64,
    pub y: f64,
    pub size: f64,
}

/// Project a viewport pixel coordinate onto the plot's `viewBox`
/// coordinate system (`-1.05..1.05` square). The returned tuple is
/// engine-native `(input, output)`: the SVG y-flip is applied here
/// (output points up) so callers downstream see engine-native data.
///
/// Returns `None` when the plot rect is unmeasurable (`size <= 0.0`),
/// which happens during the brief window between mount and the first
/// `getBoundingClientRect()` capture. `(0.0, 0.0)` would be ambiguous
/// with a real click at the plot origin, so the contract is explicit.
#[must_use]
pub(crate) fn screen_to_viewbox(cursor: (f64, f64), r: &PlotRect) -> Option<(f64, f64)> {
    if r.size <= 0.0 {
        return None;
    }
    let nx = (cursor.0 - r.x) / r.size; // 0..1 left-to-right
    let ny = (cursor.1 - r.y) / r.size; // 0..1 top-to-bottom
    let input = -1.05 + nx * 2.1;
    // Top of plot is +output (1.05); bottom is -output (-1.05).
    let output = 1.05 - ny * 2.1;
    Some((input, output))
}

/// Project a viewBox `(input, output)` tuple onto viewport pixel
/// coordinates. Inverse of `screen_to_viewbox`.
pub(crate) fn viewbox_to_screen(p: (f64, f64), r: &PlotRect) -> (f64, f64) {
    let nx = (p.0 + 1.05) / 2.1;
    let ny = (1.05 - p.1) / 2.1;
    (r.x + nx * r.size, r.y + ny * r.size)
}

/// Find the anchor in `anchors` whose projected position is within
/// `radius_px` of `cursor`. Ties broken by lowest index.
#[must_use]
pub(crate) fn nearest_anchor(
    cursor: (f64, f64),
    anchors: &[(f64, f64)],
    r: &PlotRect,
    radius_px: f64,
) -> Option<usize> {
    let radius_sq = radius_px * radius_px;
    let mut best: Option<(usize, f64)> = None;
    for (i, a) in anchors.iter().enumerate() {
        let p = viewbox_to_screen(*a, r);
        let dx = p.0 - cursor.0;
        let dy = p.1 - cursor.1;
        let d2 = dx * dx + dy * dy;
        if d2 <= radius_sq {
            match best {
                Some((_, bd)) if bd <= d2 => {}
                _ => best = Some((i, d2)),
            }
        }
    }
    best.map(|(i, _)| i)
}

pub(crate) fn extract_anchors(curve: &ResponseCurve) -> Vec<(f64, f64)> {
    match curve {
        ResponseCurve::PiecewiseLinear { points, .. }
        | ResponseCurve::CubicSpline { points, .. } => points.clone(),
        ResponseCurve::CubicBezier { segments, .. } => {
            let mut pts = Vec::with_capacity(segments.len() * 4);
            for seg in segments {
                pts.push(seg.start);
                pts.push(seg.control1);
                pts.push(seg.control2);
                pts.push(seg.end);
            }
            pts
        }
    }
}

pub(crate) fn handle_positions(config: &DeadzoneConfig) -> [(f64, f64); 4] {
    [
        (config.low(), -1.0),
        (config.center_low(), 0.0),
        (config.center_high(), 0.0),
        (config.high(), 1.0),
    ]
}
pub(crate) fn observe(pointer: &Pointer, action: &Action) -> Observation {
    let rect = PlotRect {
        x: pointer.rl,
        y: pointer.rt,
        size: pointer.rs,
    };
    let cursor = (pointer.x, pointer.y);
    let position = screen_to_viewbox(cursor, &rect).filter(|(x, y)| x.is_finite() && y.is_finite());
    let points = match action {
        Action::ResponseCurve { curve } => extract_anchors(curve),
        Action::Deadzone { config } => handle_positions(config).to_vec(),
        _ => vec![],
    };
    // Preserve the existing 10 CSS-pixel hit radius at any plot size.
    let hit = position.and_then(|_| nearest_anchor(cursor, &points, &rect, 10.0));
    let near = |i: usize| {
        points.get(i).is_some_and(|&p| {
            let screen = viewbox_to_screen(p, &rect);
            (screen.0 - cursor.0).powi(2) + (screen.1 - cursor.1).powi(2) <= 100.0
        })
    };
    let centers = matches!(action, Action::Deadzone { .. }) && near(1) && near(2);
    let split_dx = points
        .get(1)
        .map_or(0.0, |&p| cursor.0 - viewbox_to_screen(p, &rect).0);
    Observation {
        position,
        hit,
        centers,
        split_dx,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn seed_curve() -> ResponseCurve {
        ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false).unwrap()
    }
    fn rect() -> PlotRect {
        PlotRect {
            x: 10.0,
            y: 20.0,
            size: 240.0,
        }
    }
    #[test]
    fn screen_to_viewbox_maps_corners() {
        let r = rect();
        // top-left of plot maps to (-1.05, -1.05) in viewBox; in engine
        // coords (input axis horizontal) that's (input=-1.05, output=+1.05)
        // because SVG-y is flipped at render time.
        let p = screen_to_viewbox((10.0, 20.0), &r).unwrap();
        assert!((p.0 - (-1.05)).abs() < 1e-6);
        assert!((p.1 - 1.05).abs() < 1e-6);
        // Center of plot maps to (0, 0).
        let p = screen_to_viewbox((10.0 + 120.0, 20.0 + 120.0), &r).unwrap();
        assert!(p.0.abs() < 1e-6 && p.1.abs() < 1e-6);
    }

    #[test]
    fn screen_to_viewbox_returns_none_for_zero_rect() {
        let r = PlotRect {
            x: 0.0,
            y: 0.0,
            size: 0.0,
        };
        assert!(screen_to_viewbox((10.0, 20.0), &r).is_none());
    }

    #[test]
    fn nearest_anchor_within_radius() {
        let curve = seed_curve();
        let anchors = extract_anchors(&curve);
        let r = rect();
        // Center anchor (0, 0) projects to plot center.
        let cursor = (10.0 + 120.0 + 4.0, 20.0 + 120.0);
        let hit = nearest_anchor(cursor, &anchors, &r, 10.0);
        assert_eq!(hit, Some(1));
    }

    #[test]
    fn nearest_anchor_outside_radius() {
        let curve = seed_curve();
        let anchors = extract_anchors(&curve);
        let r = rect();
        // Far from any anchor.
        let cursor = (10.0 + 120.0 + 60.0, 20.0 + 120.0);
        let hit = nearest_anchor(cursor, &anchors, &r, 10.0);
        assert!(hit.is_none());
    }

    #[test]
    fn nearest_anchor_at_bezier_junction_returns_lower_index() {
        // Two-segment bezier: anchors[3] (seg0.end) and anchors[4] (seg1.start)
        // coincide at (0, 0). A click at the junction must return the
        // lower index per `nearest_anchor`'s tie-breaking rule.
        let curve = ResponseCurve::cubic_bezier(
            vec![
                inputforge_core::processing::curves::BezierSegment {
                    start: (-1.0, -1.0),
                    control1: (-0.5, -0.5),
                    control2: (-0.25, -0.25),
                    end: (0.0, 0.0),
                },
                inputforge_core::processing::curves::BezierSegment {
                    start: (0.0, 0.0),
                    control1: (0.25, 0.25),
                    control2: (0.5, 0.5),
                    end: (1.0, 1.0),
                },
            ],
            false,
        )
        .unwrap();
        let anchors = extract_anchors(&curve);
        let r = rect();
        // Junction (0, 0) projects to plot center.
        let cursor = (10.0 + 120.0, 20.0 + 120.0);
        let hit = nearest_anchor(cursor, &anchors, &r, 10.0);
        assert_eq!(
            hit,
            Some(3),
            "junction tie must return lower index (3, not 4)"
        );
    }
}
