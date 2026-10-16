//! Pixel fixtures for preserved pointer regression tests.
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

pub(crate) fn near(cursor: (f64, f64), p: (f64, f64), r: &PlotRect, radius: f64) -> bool {
    let q = viewbox_to_screen(p, r);
    (q.0 - cursor.0).powi(2) + (q.1 - cursor.1).powi(2) <= radius * radius
}
