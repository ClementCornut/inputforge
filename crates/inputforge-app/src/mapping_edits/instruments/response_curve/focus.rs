//! Curve anchor focus traversal and duplicate Bezier junction filtering.
use inputforge_core::processing::curves::ResponseCurve;
pub(super) fn is_anchor_index(curve: &ResponseCurve, idx: usize) -> bool {
    match curve {
        ResponseCurve::PiecewiseLinear { .. } | ResponseCurve::CubicSpline { .. } => true,
        ResponseCurve::CubicBezier { .. } => matches!(idx % 4, 0 | 3),
    }
}

pub(super) fn advance_focus(
    curve: &ResponseCurve,
    anchors: &[(f64, f64)],
    current: Option<usize>,
    backward: bool,
) -> Option<usize> {
    if anchors.is_empty() {
        return None;
    }
    let len = anchors.len();
    if backward && current.is_none_or(|i| i == 0) {
        return None;
    }
    let start = current.map_or(0, |i| if backward { i.saturating_sub(1) } else { i + 1 });
    let order: Vec<usize> = if backward {
        (0..len).rev().collect()
    } else {
        (0..len).collect()
    };
    let visit_filter = |i: usize| -> bool {
        if let ResponseCurve::CubicBezier { .. } = curve
            && i.is_multiple_of(4)
            && i > 0
            && let Some(prev) = anchors.get(i.saturating_sub(1)).copied()
            && let Some(here) = anchors.get(i).copied()
            && (prev.0 - here.0).abs() < f64::EPSILON
            && (prev.1 - here.1).abs() < f64::EPSILON
        {
            return false;
        }
        true
    };
    order
        .iter()
        .skip_while(|&&i| if backward { i > start } else { i < start })
        .find(|&&i| visit_filter(i))
        .copied()
}
