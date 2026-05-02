// Rust guideline compliant 2026-05-02

//! Curve mutation operations, ported verbatim from
//! `E:\Git\Perso\inputforge-egui-ref\crates\inputforge-gui\src\widgets\curve_editor\mutation.rs`.
//!
//! Mechanical surface change: `egui_plot::PlotPoint` becomes `(f64, f64)`.
//! The egui implementation is already engine-native `(input, output)`, so
//! no swap is unwound here. The SVG render path applies y-down via a
//! `<g transform="scale(1, -1)">`, never via tuple swap.

use inputforge_core::processing::curves::{BezierSegment, ResponseCurve};

use super::CurveType;

/// Minimum x separation between adjacent control points when dragging.
///
/// Below this gap two points would be indistinguishable to the engine's
/// piecewise-linear interpolation and would violate the strictly-increasing
/// x invariant enforced by `ResponseCurve::piecewise_linear`.
const MIN_X_GAP: f64 = 0.001;

// ---------------------------------------------------------------------------
// Drag application
// ---------------------------------------------------------------------------

/// Returns the `(lo, hi)` x-coordinate clamp for dragging the point at
/// `index` in `curve`.
///
/// - First and last anchor points are locked to their current x (returns
///   identical lo/hi equal to the point's own x).
/// - The symmetric center point is locked to `(0.0, 0.0)`.
/// - All other points are clamped between their left and right neighbors
///   with a [`MIN_X_GAP`] buffer on each side.
/// - For Bezier curves the control handles are given full `(-1.0, 1.0)` range
///   because they do not need to stay ordered.
#[must_use]
pub(crate) fn adjacent_x_bounds(curve: &ResponseCurve, index: usize) -> (f64, f64) {
    let symmetric = match curve {
        ResponseCurve::PiecewiseLinear { symmetric, .. }
        | ResponseCurve::CubicSpline { symmetric, .. }
        | ResponseCurve::CubicBezier { symmetric, .. } => *symmetric,
    };
    match curve {
        ResponseCurve::PiecewiseLinear { points, .. }
        | ResponseCurve::CubicSpline { points, .. } => {
            let count = points.len();
            if index == 0 {
                return (points[0].0, points[0].0);
            }
            if index == count - 1 {
                return (points[count - 1].0, points[count - 1].0);
            }
            if symmetric && count % 2 == 1 && index == count / 2 {
                return (0.0, 0.0);
            }
            (
                points[index - 1].0 + MIN_X_GAP,
                points[index + 1].0 - MIN_X_GAP,
            )
        }
        ResponseCurve::CubicBezier { segments, .. } => {
            let seg_idx = index / 4;
            let local = index % 4;
            let last_seg = segments.len().saturating_sub(1);
            if seg_idx == 0 && local == 0 {
                return (-1.0, -1.0);
            }
            if seg_idx == last_seg && local == 3 {
                return (1.0, 1.0);
            }
            (-1.0, 1.0)
        }
    }
}

/// Moves the control point at `index` in `curve` to `new_pos`, clamping
/// x to `bounds` and y to `[-1.0, 1.0]`.
///
/// For symmetric piecewise/spline curves the mirror point is updated
/// automatically. The center point of a symmetric odd-length curve is
/// frozen and this function returns early without modifying the curve.
pub(crate) fn update_point_in_curve(
    curve: &mut ResponseCurve,
    index: usize,
    new_pos: (f64, f64),
    bounds: (f64, f64),
) {
    let new_x = new_pos.0.clamp(bounds.0, bounds.1);
    let new_y = new_pos.1.clamp(-1.0, 1.0);
    match curve {
        ResponseCurve::PiecewiseLinear {
            points, symmetric, ..
        }
        | ResponseCurve::CubicSpline {
            points, symmetric, ..
        } => {
            if *symmetric && points.len() % 2 == 1 && index == points.len() / 2 {
                return;
            }
            if let Some(pt) = points.get_mut(index) {
                pt.0 = new_x;
                pt.1 = new_y;
            }
            if *symmetric {
                let count = points.len();
                let mirror_idx = count - 1 - index;
                if mirror_idx != index {
                    if let Some(mirror_pt) = points.get_mut(mirror_idx) {
                        mirror_pt.0 = -new_x;
                        mirror_pt.1 = -new_y;
                    }
                }
            }
        }
        ResponseCurve::CubicBezier {
            segments,
            symmetric,
        } => {
            update_bezier_point(segments, *symmetric, index, new_x, new_y);
        }
    }
}

/// Inner helper that applies a drag to a single Bezier control point and
/// propagates joint-continuity to adjacent segments and the symmetric mirror.
fn update_bezier_point(
    segments: &mut [BezierSegment],
    symmetric: bool,
    index: usize,
    new_x: f64,
    new_y: f64,
) {
    let seg_idx = index / 4;
    let local = index % 4;
    // Freeze the join between the two halves of a symmetric curve.
    if symmetric && segments.len() % 2 == 0 {
        let center_seg = segments.len() / 2;
        if seg_idx == center_seg && local == 0 {
            return;
        }
        if seg_idx == center_seg - 1 && local == 3 {
            return;
        }
    }
    if let Some(seg) = segments.get_mut(seg_idx) {
        match local {
            0 => {
                seg.start = (new_x, new_y);
            }
            1 => {
                seg.control1 = (new_x, new_y);
            }
            2 => {
                seg.control2 = (new_x, new_y);
            }
            3 => {
                seg.end = (new_x, new_y);
            }
            _ => {}
        }
    }
    // Propagate shared endpoint to the adjacent segment.
    if local == 3 {
        if let Some(next) = segments.get_mut(seg_idx + 1) {
            next.start = (new_x, new_y);
        }
    } else if local == 0 && seg_idx > 0 {
        if let Some(prev) = segments.get_mut(seg_idx - 1) {
            prev.end = (new_x, new_y);
        }
    }
    if symmetric {
        let seg_count = segments.len();
        let mirror_seg_idx = seg_count - 1 - seg_idx;
        let mirror_local = 3 - local;
        let primary_synced_idx = match local {
            3 => Some(seg_idx + 1),
            0 if seg_idx > 0 => Some(seg_idx - 1),
            _ => None,
        };
        if mirror_seg_idx != seg_idx || mirror_local != local {
            if let Some(mirror_seg) = segments.get_mut(mirror_seg_idx) {
                match mirror_local {
                    0 => mirror_seg.start = (-new_x, -new_y),
                    1 => mirror_seg.control1 = (-new_x, -new_y),
                    2 => mirror_seg.control2 = (-new_x, -new_y),
                    3 => mirror_seg.end = (-new_x, -new_y),
                    _ => {}
                }
            }
            if mirror_local == 3 {
                let target = mirror_seg_idx + 1;
                if primary_synced_idx != Some(target) {
                    if let Some(next) = segments.get_mut(target) {
                        next.start = (-new_x, -new_y);
                    }
                }
            } else if mirror_local == 0 && mirror_seg_idx > 0 {
                let target = mirror_seg_idx - 1;
                if primary_synced_idx != Some(target) {
                    if let Some(prev) = segments.get_mut(target) {
                        prev.end = (-new_x, -new_y);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Reconstruction + identity reset
// ---------------------------------------------------------------------------

/// Re-validates `curve` through the engine constructors.
///
/// Returns the validator's error string when the curve is structurally invalid
/// (e.g., duplicate x-values in a piecewise curve).
pub(crate) fn reconstruct_curve(curve: &ResponseCurve) -> Result<ResponseCurve, String> {
    match curve {
        ResponseCurve::PiecewiseLinear { points, symmetric } => {
            ResponseCurve::piecewise_linear(points.clone(), *symmetric).map_err(|e| e.to_string())
        }
        ResponseCurve::CubicSpline { points, symmetric } => {
            ResponseCurve::cubic_spline(points.clone(), *symmetric).map_err(|e| e.to_string())
        }
        ResponseCurve::CubicBezier {
            segments,
            symmetric,
        } => ResponseCurve::cubic_bezier(segments.clone(), *symmetric).map_err(|e| e.to_string()),
    }
}

/// Returns the canonical identity curve of the same variant and `symmetric`
/// flag as `curve`.
///
/// For `PiecewiseLinear` and `CubicSpline` the identity is three points:
/// `(-1, -1)`, `(0, 0)`, `(1, 1)`. For `CubicBezier` it is the result of
/// [`symmetric_bezier_identity`].
#[must_use]
pub(crate) fn default_identity_curve(curve: &ResponseCurve) -> ResponseCurve {
    match curve {
        ResponseCurve::PiecewiseLinear { symmetric, .. } => {
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], *symmetric)
                .unwrap_or_else(|_| {
                    ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (1.0, 1.0)], false)
                        .expect("hardcoded identity is valid")
                })
        }
        ResponseCurve::CubicSpline { symmetric, .. } => {
            ResponseCurve::cubic_spline(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], *symmetric)
                .unwrap_or_else(|_| {
                    ResponseCurve::cubic_spline(vec![(-1.0, -1.0), (1.0, 1.0)], false)
                        .expect("hardcoded identity is valid")
                })
        }
        ResponseCurve::CubicBezier { symmetric, .. } => {
            let segs = symmetric_bezier_identity(*symmetric);
            ResponseCurve::cubic_bezier(segs, *symmetric).unwrap_or_else(|_| {
                ResponseCurve::cubic_bezier(symmetric_bezier_identity(false), false)
                    .expect("hardcoded bezier identity is valid")
            })
        }
    }
}

/// Constructs the control-point layout for an identity Bezier curve.
///
/// When `symmetric` is true the curve is split at the origin into two
/// segments so the center join can be frozen during dragging. When false
/// a single segment spanning `[-1, 1]` is returned.
fn symmetric_bezier_identity(symmetric: bool) -> Vec<BezierSegment> {
    if symmetric {
        vec![
            BezierSegment {
                start: (-1.0, -1.0),
                control1: (-2.0 / 3.0, -2.0 / 3.0),
                control2: (-1.0 / 3.0, -1.0 / 3.0),
                end: (0.0, 0.0),
            },
            BezierSegment {
                start: (0.0, 0.0),
                control1: (1.0 / 3.0, 1.0 / 3.0),
                control2: (2.0 / 3.0, 2.0 / 3.0),
                end: (1.0, 1.0),
            },
        ]
    } else {
        vec![BezierSegment {
            start: (-1.0, -1.0),
            control1: (-1.0 / 3.0, -1.0 / 3.0),
            control2: (1.0 / 3.0, 1.0 / 3.0),
            end: (1.0, 1.0),
        }]
    }
}

// ---------------------------------------------------------------------------
// Type conversion
// ---------------------------------------------------------------------------

/// Converts `curve` to `target` variant, preserving the `symmetric` flag.
///
/// The converted curve is always reset to the identity shape for the new
/// type. Returns `None` only if the engine rejects the constructed curve,
/// which should not happen for these hardcoded inputs.
#[must_use]
pub(crate) fn convert_curve_type(
    curve: &ResponseCurve,
    target: CurveType,
) -> Option<ResponseCurve> {
    let symmetric = match curve {
        ResponseCurve::PiecewiseLinear { symmetric, .. }
        | ResponseCurve::CubicSpline { symmetric, .. }
        | ResponseCurve::CubicBezier { symmetric, .. } => *symmetric,
    };
    match target {
        CurveType::PiecewiseLinear => {
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], symmetric)
                .ok()
        }
        CurveType::CubicSpline => {
            ResponseCurve::cubic_spline(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], symmetric).ok()
        }
        CurveType::CubicBezier => {
            ResponseCurve::cubic_bezier(symmetric_bezier_identity(symmetric), symmetric).ok()
        }
    }
}

// ---------------------------------------------------------------------------
// Add / remove control points
// ---------------------------------------------------------------------------

/// Inserts a new control point at `pos` into `curve`.
///
/// For piecewise/spline curves the point is appended, its symmetric mirror
/// added when applicable, and the list sorted by x. Returns `false` if the
/// resulting point list would violate the strictly-increasing-x invariant.
///
/// For Bezier curves the enclosing segment is split via de Casteljau at the
/// parametric t closest to `pos.x`. Returns `false` if no enclosing segment
/// contains `pos.x`.
pub(crate) fn add_control_point(curve: &mut ResponseCurve, pos: (f64, f64)) -> bool {
    let x = pos.0.clamp(-1.0, 1.0);
    let y = pos.1.clamp(-1.0, 1.0);
    match curve {
        ResponseCurve::PiecewiseLinear {
            points, symmetric, ..
        }
        | ResponseCurve::CubicSpline {
            points, symmetric, ..
        } => {
            let original = points.clone();
            points.push((x, y));
            if *symmetric && x.abs() > 0.0 {
                points.push((-x, -y));
            }
            points.sort_by(|a, b| a.0.total_cmp(&b.0));
            if points.windows(2).all(|w| w[0].0 < w[1].0) {
                true
            } else {
                *points = original;
                false
            }
        }
        ResponseCurve::CubicBezier {
            segments,
            symmetric,
        } => {
            let Some(seg_idx) = segments.iter().position(|s| s.start.0 <= x && x <= s.end.0) else {
                return false;
            };
            let seg = &segments[seg_idx];
            let dx = seg.end.0 - seg.start.0;
            if dx.abs() < f64::EPSILON {
                return false;
            }
            let t = ((x - seg.start.0) / dx).clamp(0.05, 0.95);
            let (left, right) = split_bezier_segment(seg, t);
            segments.splice(seg_idx..=seg_idx, [left, right]);
            if *symmetric {
                let pre_splice_count = segments.len() - 1;
                let mut mirror_seg = pre_splice_count - 1 - seg_idx;
                if mirror_seg >= seg_idx {
                    mirror_seg += 1;
                }
                if mirror_seg != seg_idx && mirror_seg != seg_idx + 1 {
                    let mirror_x = -x;
                    let m_seg = &segments[mirror_seg];
                    let m_dx = m_seg.end.0 - m_seg.start.0;
                    let mirror_t = if m_dx.abs() < f64::EPSILON {
                        0.5
                    } else {
                        ((mirror_x - m_seg.start.0) / m_dx).clamp(0.05, 0.95)
                    };
                    let (ml, mr) = split_bezier_segment(&segments[mirror_seg], mirror_t);
                    segments.splice(mirror_seg..=mirror_seg, [ml, mr]);
                }
            }
            true
        }
    }
}

/// Removes the control point at `index` from `curve`.
///
/// Returns `false` (no-op) when:
/// - `index` is the first or last anchor of a piecewise/spline curve,
/// - `index` is the symmetric center of an odd-length symmetric curve,
/// - `index` addresses a Bezier handle (local position 1 or 2),
/// - `index` addresses the join between the two halves of a symmetric Bezier,
/// - removing would leave fewer than the minimum required control points.
pub(crate) fn remove_control_point(curve: &mut ResponseCurve, index: usize) -> bool {
    match curve {
        ResponseCurve::PiecewiseLinear {
            points, symmetric, ..
        }
        | ResponseCurve::CubicSpline {
            points, symmetric, ..
        } => {
            let count = points.len();
            if index == 0 || index == count - 1 {
                return false;
            }
            if *symmetric && count % 2 == 1 && index == count / 2 {
                return false;
            }
            let removals = if *symmetric { 2 } else { 1 };
            if count <= removals + 1 {
                return false;
            }
            if *symmetric {
                let mirror_idx = count - 1 - index;
                debug_assert_ne!(index, mirror_idx);
                let (first, second) = if index > mirror_idx {
                    (index, mirror_idx)
                } else {
                    (mirror_idx, index)
                };
                points.remove(first);
                points.remove(second);
            } else {
                points.remove(index);
            }
            true
        }
        ResponseCurve::CubicBezier {
            segments,
            symmetric,
        } => {
            let seg_idx = index / 4;
            let local = index % 4;
            // Bezier handles (local 1 or 2) cannot be removed, only anchors can.
            if local == 1 || local == 2 {
                return false;
            }
            let (left_idx, right_idx) = if local == 3 {
                (seg_idx, seg_idx + 1)
            } else {
                if seg_idx == 0 {
                    return false;
                }
                (seg_idx - 1, seg_idx)
            };
            let seg_count = segments.len();
            if right_idx >= seg_count || seg_count < 2 {
                return false;
            }
            if *symmetric && seg_count % 2 == 0 {
                let center_seg = seg_count / 2;
                if (local == 3 && seg_idx == center_seg - 1)
                    || (local == 0 && seg_idx == center_seg)
                {
                    return false;
                }
            }
            let merged = BezierSegment {
                start: segments[left_idx].start,
                control1: segments[left_idx].control1,
                control2: segments[right_idx].control2,
                end: segments[right_idx].end,
            };
            segments.splice(left_idx..=right_idx, [merged]);
            if *symmetric {
                let pre_merge_count = segments.len() + 1;
                let mut mirror_left = pre_merge_count - 2 - left_idx;
                if mirror_left > left_idx {
                    mirror_left -= 1;
                }
                let new_count = segments.len();
                if mirror_left < new_count && mirror_left != left_idx {
                    let mirror_right = mirror_left + 1;
                    if mirror_right < new_count {
                        let mirror_merged = BezierSegment {
                            start: segments[mirror_left].start,
                            control1: segments[mirror_left].control1,
                            control2: segments[mirror_right].control2,
                            end: segments[mirror_right].end,
                        };
                        segments.splice(mirror_left..=mirror_right, [mirror_merged]);
                    }
                }
            }
            true
        }
    }
}

// ---------------------------------------------------------------------------
// Bezier helpers
// ---------------------------------------------------------------------------

/// Linear interpolation between two `(f64, f64)` points.
fn lerp_point(a: (f64, f64), b: (f64, f64), t: f64) -> (f64, f64) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// Splits a cubic Bezier segment at parameter `t` using de Casteljau's algorithm.
///
/// Returns `(left, right)` where `left.end == right.start == point at t`.
fn split_bezier_segment(seg: &BezierSegment, t: f64) -> (BezierSegment, BezierSegment) {
    let ab = lerp_point(seg.start, seg.control1, t);
    let bc = lerp_point(seg.control1, seg.control2, t);
    let cd = lerp_point(seg.control2, seg.end, t);
    let abc = lerp_point(ab, bc, t);
    let bcd = lerp_point(bc, cd, t);
    let mid = lerp_point(abc, bcd, t);
    (
        BezierSegment {
            start: seg.start,
            control1: ab,
            control2: abc,
            end: mid,
        },
        BezierSegment {
            start: mid,
            control1: bcd,
            control2: cd,
            end: seg.end,
        },
    )
}

// ---------------------------------------------------------------------------
// Symmetry enforcement
// ---------------------------------------------------------------------------

/// Apply a symmetry change. Enabling enforces antisymmetry through the
/// origin (mirrors positive-half points to negative side); disabling just
/// clears the flag. Ported from egui `widgets/curve_editor/symmetry.rs`.
#[must_use]
pub(crate) fn apply_symmetry(curve: &ResponseCurve, symmetric: bool) -> Option<ResponseCurve> {
    if symmetric {
        enforce_symmetry(curve)
    } else {
        let mut result = curve.clone();
        result.set_symmetric(false);
        Some(result)
    }
}

fn enforce_symmetry(curve: &ResponseCurve) -> Option<ResponseCurve> {
    match curve {
        ResponseCurve::PiecewiseLinear { points, .. } => {
            ResponseCurve::piecewise_linear(enforce_symmetry_points(points), true).ok()
        }
        ResponseCurve::CubicSpline { points, .. } => {
            ResponseCurve::cubic_spline(enforce_symmetry_points(points), true).ok()
        }
        ResponseCurve::CubicBezier { segments, .. } => {
            ResponseCurve::cubic_bezier(enforce_symmetry_bezier(segments), true).ok()
        }
    }
}

fn enforce_symmetry_points(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut positive: Vec<(f64, f64)> = points.iter().filter(|(x, _)| *x >= 0.0).copied().collect();
    positive.sort_by(|a, b| a.0.total_cmp(&b.0));
    if positive.is_empty() || positive[0].0 > 0.0 {
        positive.insert(0, (0.0, 0.0));
    } else {
        positive[0].1 = 0.0;
    }
    if positive.len() < 2 {
        positive.push((1.0, 1.0));
    }
    let mut result: Vec<(f64, f64)> = positive
        .iter()
        .filter(|(x, _)| *x > 0.0)
        .map(|(x, y)| (-x, -y))
        .collect();
    result.reverse();
    result.extend_from_slice(&positive);
    result
}

fn enforce_symmetry_bezier(segments: &[BezierSegment]) -> Vec<BezierSegment> {
    let positive: Vec<_> = segments
        .iter()
        .filter(|s| s.start.0 >= 0.0)
        .cloned()
        .collect();
    let positive = if positive.is_empty() {
        vec![BezierSegment {
            start: (0.0, 0.0),
            control1: (1.0 / 3.0, 1.0 / 3.0),
            control2: (2.0 / 3.0, 2.0 / 3.0),
            end: (1.0, 1.0),
        }]
    } else {
        positive
    };
    let mut mirrored: Vec<BezierSegment> = positive
        .iter()
        .rev()
        .map(|seg| BezierSegment {
            start: (-seg.end.0, -seg.end.1),
            control1: (-seg.control2.0, -seg.control2.1),
            control2: (-seg.control1.0, -seg.control1.1),
            end: (-seg.start.0, -seg.start.1),
        })
        .collect();
    mirrored.extend_from_slice(&positive);
    mirrored
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_piecewise() -> ResponseCurve {
        ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false).unwrap()
    }

    fn identity_bezier() -> ResponseCurve {
        let seg = BezierSegment {
            start: (-1.0, -1.0),
            control1: (-1.0 / 3.0, -1.0 / 3.0),
            control2: (1.0 / 3.0, 1.0 / 3.0),
            end: (1.0, 1.0),
        };
        ResponseCurve::cubic_bezier(vec![seg], false).unwrap()
    }

    #[test]
    fn adjacent_x_bounds_locks_first_and_last() {
        let curve = identity_piecewise();
        let (lo, hi) = adjacent_x_bounds(&curve, 0);
        assert!((lo - (-1.0)).abs() < f64::EPSILON);
        assert!((hi - (-1.0)).abs() < f64::EPSILON);
        let (lo, hi) = adjacent_x_bounds(&curve, 2);
        assert!((lo - 1.0).abs() < f64::EPSILON);
        assert!((hi - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn adjacent_x_bounds_locks_symmetric_center() {
        let curve = ResponseCurve::piecewise_linear(
            vec![
                (-1.0, -1.0),
                (-0.5, -0.3),
                (0.0, 0.0),
                (0.5, 0.3),
                (1.0, 1.0),
            ],
            true,
        )
        .unwrap();
        let (lo, hi) = adjacent_x_bounds(&curve, 2);
        assert!((lo - 0.0).abs() < f64::EPSILON);
        assert!((hi - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn update_point_freezes_symmetric_center() {
        let mut curve = ResponseCurve::PiecewiseLinear {
            points: vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)],
            symmetric: true,
        };
        let bounds = adjacent_x_bounds(&curve, 1);
        update_point_in_curve(&mut curve, 1, (0.3, 0.5), bounds);
        if let ResponseCurve::PiecewiseLinear { points, .. } = &curve {
            assert!(points[1].0.abs() < f64::EPSILON, "center x stays at 0");
            assert!(points[1].1.abs() < f64::EPSILON, "center y stays at 0");
        } else {
            panic!("expected PiecewiseLinear");
        }
    }

    #[test]
    fn update_point_mirrors_in_symmetric() {
        let mut curve = ResponseCurve::PiecewiseLinear {
            points: vec![
                (-1.0, -1.0),
                (-0.5, -0.5),
                (0.0, 0.0),
                (0.5, 0.5),
                (1.0, 1.0),
            ],
            symmetric: true,
        };
        let bounds = adjacent_x_bounds(&curve, 3);
        update_point_in_curve(&mut curve, 3, (0.4, 0.7), bounds);
        if let ResponseCurve::PiecewiseLinear { points, .. } = &curve {
            assert!((points[3].0 - 0.4).abs() < 1e-9);
            assert!((points[3].1 - 0.7).abs() < 1e-9);
            // Mirror at index 1.
            assert!((points[1].0 - (-0.4)).abs() < 1e-9);
            assert!((points[1].1 - (-0.7)).abs() < 1e-9);
        }
    }

    #[test]
    fn convert_curve_type_preserves_symmetric_flag() {
        let curve =
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], true)
                .unwrap();
        let bezier = convert_curve_type(&curve, CurveType::CubicBezier).unwrap();
        match bezier {
            ResponseCurve::CubicBezier {
                symmetric,
                segments,
            } => {
                assert!(symmetric);
                assert_eq!(segments.len(), 2, "symmetric bezier has 2 segments");
            }
            _ => panic!("expected CubicBezier"),
        }
    }

    #[test]
    fn add_control_point_inserts_between_neighbors() {
        let mut curve = identity_piecewise();
        let added = add_control_point(&mut curve, (0.5, 0.7));
        assert!(added);
        if let ResponseCurve::PiecewiseLinear { points, .. } = &curve {
            assert_eq!(points.len(), 4);
            assert!(points.windows(2).all(|w| w[0].0 < w[1].0));
        }
    }

    #[test]
    fn remove_control_point_refuses_edges_and_handles() {
        let mut curve = ResponseCurve::piecewise_linear(
            vec![(-1.0, -1.0), (0.0, 0.0), (0.5, 0.5), (1.0, 1.0)],
            false,
        )
        .unwrap();
        assert!(
            !remove_control_point(&mut curve, 0),
            "first edge cannot be removed"
        );
        assert!(
            !remove_control_point(&mut curve, 3),
            "last edge cannot be removed"
        );
        // Bezier handle (local 1 or 2) cannot be removed.
        let mut bz = identity_bezier();
        assert!(
            !remove_control_point(&mut bz, 1),
            "bezier handle cannot be removed"
        );
        assert!(
            !remove_control_point(&mut bz, 2),
            "bezier handle cannot be removed"
        );
    }

    #[test]
    fn reconstruct_curve_returns_validated() {
        let curve = identity_piecewise();
        reconstruct_curve(&curve).unwrap();
    }

    #[test]
    fn reconstruct_curve_returns_error_for_duplicate_x() {
        let invalid = ResponseCurve::PiecewiseLinear {
            points: vec![(-1.0, -1.0), (0.0, 0.0), (0.0, 0.5), (1.0, 1.0)],
            symmetric: false,
        };
        let result = reconstruct_curve(&invalid);
        assert!(result.is_err(), "duplicate x must reject");
        let err = result.unwrap_err();
        assert!(!err.is_empty(), "error string must not be empty");
    }

    #[test]
    fn default_identity_curve_preserves_type_and_symmetric() {
        let curve = ResponseCurve::cubic_bezier(
            vec![BezierSegment {
                start: (-1.0, -1.0),
                control1: (-0.5, -0.5),
                control2: (0.5, 0.5),
                end: (1.0, 1.0),
            }],
            true,
        )
        .unwrap();
        let reset = default_identity_curve(&curve);
        match reset {
            ResponseCurve::CubicBezier {
                symmetric: true,
                segments,
            } => {
                assert_eq!(segments.len(), 2, "symmetric reset is 2 segments");
            }
            _ => panic!("expected symmetric CubicBezier"),
        }
    }

    #[test]
    fn apply_symmetry_enabling_enforces_antisymmetric_points() {
        let curve =
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false)
                .unwrap();
        let result = apply_symmetry(&curve, true).expect("enable symmetry on identity");
        if let ResponseCurve::PiecewiseLinear { points, symmetric } = result {
            assert!(symmetric);
            assert!(points.len() >= 3);
            let center = points.iter().find(|(x, _)| x.abs() < f64::EPSILON);
            assert!(center.is_some(), "origin must be present");
            assert!(center.unwrap().1.abs() < f64::EPSILON);
        } else {
            panic!("expected PiecewiseLinear");
        }
    }

    #[test]
    fn apply_symmetry_two_point_default_curve() {
        let curve = ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (1.0, 1.0)], false).unwrap();
        let result = apply_symmetry(&curve, true).expect("enable symmetry on 2-point");
        if let ResponseCurve::PiecewiseLinear { points, symmetric } = result {
            assert!(symmetric);
            assert!(points.len() >= 3);
            assert!(points[0].0 < 0.0);
            assert!(points[points.len() - 1].0 > 0.0);
        }
    }

    #[test]
    fn apply_symmetry_disabling_keeps_all_points() {
        let curve = ResponseCurve::piecewise_linear(
            vec![(-1.0, -1.0), (0.0, 0.0), (0.5, 0.2), (1.0, 1.0)],
            true,
        )
        .unwrap();
        let result = apply_symmetry(&curve, false).expect("disable symmetry");
        if let ResponseCurve::PiecewiseLinear { points, symmetric } = result {
            assert!(!symmetric);
            assert_eq!(points.len(), 4);
        }
    }

    #[test]
    fn apply_symmetry_bezier_round_trip() {
        let curve = ResponseCurve::cubic_bezier(
            vec![BezierSegment {
                start: (-1.0, -1.0),
                control1: (-0.5, -0.5),
                control2: (0.5, 0.5),
                end: (1.0, 1.0),
            }],
            false,
        )
        .unwrap();
        let sym = apply_symmetry(&curve, true).expect("enable bezier symmetry");
        if let ResponseCurve::CubicBezier {
            segments,
            symmetric: true,
        } = sym
        {
            assert!(segments.len() >= 2);
        } else {
            panic!("expected symmetric CubicBezier");
        }
    }
}
