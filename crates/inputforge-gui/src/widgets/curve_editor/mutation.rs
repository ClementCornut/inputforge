// Rust guideline compliant 2026-03-04

//! Curve mutation operations for the curve editor.
//!
//! Contains functions that modify curve data: dragging control points,
//! adding/removing points, type conversion, and curve reconstruction.

use egui_plot::PlotPoint;

use inputforge_core::processing::curves::{BezierSegment, ResponseCurve};

use super::{CurveType, MIN_X_GAP};

// ---------------------------------------------------------------------------
// Drag application
// ---------------------------------------------------------------------------

/// Compute the allowed x range for moving control point at `index`.
///
/// Returns `(lower_bound, upper_bound)` exclusive so that the dragged point
/// keeps strictly between its neighbors. Edge points (first and last) have
/// their x position locked. In symmetric mode, the center point is frozen
/// at x = 0.
pub(super) fn adjacent_x_bounds(curve: &ResponseCurve, index: usize) -> (f64, f64) {
    let symmetric = match curve {
        ResponseCurve::PiecewiseLinear { symmetric, .. }
        | ResponseCurve::CubicSpline { symmetric, .. }
        | ResponseCurve::CubicBezier { symmetric, .. } => *symmetric,
    };

    match curve {
        ResponseCurve::PiecewiseLinear { points, .. }
        | ResponseCurve::CubicSpline { points, .. } => {
            let count = points.len();

            // Edge points: x locked at their current position.
            if index == 0 {
                return (points[0].0, points[0].0);
            }
            if index == count - 1 {
                return (points[count - 1].0, points[count - 1].0);
            }

            // Center point in symmetric mode: frozen at x = 0.
            if symmetric && count % 2 == 1 && index == count / 2 {
                return (0.0, 0.0);
            }

            let lower = points[index - 1].0 + MIN_X_GAP;
            let upper = points[index + 1].0 - MIN_X_GAP;
            (lower, upper)
        }
        ResponseCurve::CubicBezier { segments, .. } => {
            let seg_idx = index / 4;
            let local = index % 4;
            let last_seg = segments.len().saturating_sub(1);

            // Lock endpoint x: first segment start at x=-1, last segment end at x=1.
            if seg_idx == 0 && local == 0 {
                return (-1.0, -1.0);
            }
            if seg_idx == last_seg && local == 3 {
                return (1.0, 1.0);
            }

            // Handles are unconstrained in x; find_t_for_x handles
            // non-monotonic x(t) via coarse sampling.
            (-1.0, 1.0)
        }
    }
}

/// Update a single control point in the curve, clamping x to `bounds`.
///
/// For `PiecewiseLinear` / `CubicSpline` the matching point tuple is updated
/// directly.  For `CubicBezier` the point at the given index within the
/// interleaved `[start, c1, c2, end]` layout is updated in the corresponding
/// segment field.
///
/// When the curve is symmetric, the mirror point at `count - 1 - index` is
/// automatically updated to `(-x, -y)`, maintaining antisymmetry.
pub(super) fn update_point_in_curve(
    curve: &mut ResponseCurve,
    index: usize,
    new_pos: PlotPoint,
    bounds: (f64, f64),
) {
    let new_x = new_pos.x.clamp(bounds.0, bounds.1);
    let new_y = new_pos.y.clamp(-1.0, 1.0);

    match curve {
        ResponseCurve::PiecewiseLinear {
            points, symmetric, ..
        }
        | ResponseCurve::CubicSpline {
            points, symmetric, ..
        } => {
            // Center point is frozen at (0, 0) in symmetric mode.
            if *symmetric && points.len() % 2 == 1 && index == points.len() / 2 {
                return;
            }
            if let Some(pt) = points.get_mut(index) {
                pt.0 = new_x;
                pt.1 = new_y;
            }
            // Auto-mirror in symmetric mode.
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
        } => update_bezier_point(segments, *symmetric, index, new_x, new_y),
    }
}

/// Update a single bezier control point and enforce symmetric mirroring.
///
/// Handles center-freeze, endpoint sync between consecutive segments,
/// and antisymmetric mirroring when `symmetric` is enabled.
fn update_bezier_point(
    segments: &mut [BezierSegment],
    symmetric: bool,
    index: usize,
    new_x: f64,
    new_y: f64,
) {
    let seg_idx = index / 4;
    let local = index % 4;

    // Center junction point is frozen at (0, 0) in symmetric mode.
    // For N segments, the center is at segment N/2, local 0 (= start).
    if symmetric && segments.len() % 2 == 0 {
        let center_seg = segments.len() / 2;
        if seg_idx == center_seg && local == 0 {
            return;
        }
        // Also block the alias: previous segment's end (local 3).
        if seg_idx == center_seg - 1 && local == 3 {
            return;
        }
    }

    if let Some(seg) = segments.get_mut(seg_idx) {
        match local {
            0 => {
                seg.start.0 = new_x;
                seg.start.1 = new_y;
            }
            1 => {
                seg.control1.0 = new_x;
                seg.control1.1 = new_y;
            }
            2 => {
                seg.control2.0 = new_x;
                seg.control2.1 = new_y;
            }
            3 => {
                seg.end.0 = new_x;
                seg.end.1 = new_y;
            }
            _ => {}
        }
    }

    // Sync shared endpoints between consecutive segments:
    // segment N's end == segment N+1's start.
    if local == 3 {
        if let Some(next) = segments.get_mut(seg_idx + 1) {
            next.start.0 = new_x;
            next.start.1 = new_y;
        }
    } else if local == 0 && seg_idx > 0 {
        if let Some(prev) = segments.get_mut(seg_idx - 1) {
            prev.end.0 = new_x;
            prev.end.1 = new_y;
        }
    }

    // Auto-mirror in symmetric mode: mirror the corresponding
    // point in the opposite segment. For N segments, segment i
    // mirrors to segment (N - 1 - i), with local positions
    // swapped (0<->3, 1<->2).
    if symmetric {
        let seg_count = segments.len();
        let mirror_seg_idx = seg_count - 1 - seg_idx;
        let mirror_local = 3 - local;

        // Track which segment the primary endpoint sync touched,
        // so we skip overlapping mirror endpoint sync.
        let primary_synced_idx = match local {
            3 => Some(seg_idx + 1),
            0 if seg_idx > 0 => Some(seg_idx - 1),
            _ => None,
        };

        // Only mirror if it is a different point (not the center).
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
            // Sync shared endpoints for the mirrored segment,
            // skipping if the primary sync already wrote to this segment.
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
// Curve reconstruction after drag
// ---------------------------------------------------------------------------

/// Reconstruct a fully validated curve from the current (potentially dirty)
/// internal state.
///
/// Returns `None` when the state fails validation (e.g., points out of order
/// after a drag). The caller should revert to a safe default in that case.
pub(super) fn reconstruct_curve(curve: &ResponseCurve) -> Option<ResponseCurve> {
    match curve {
        ResponseCurve::PiecewiseLinear { points, symmetric } => {
            ResponseCurve::piecewise_linear(points.clone(), *symmetric).ok()
        }
        ResponseCurve::CubicSpline { points, symmetric } => {
            ResponseCurve::cubic_spline(points.clone(), *symmetric).ok()
        }
        ResponseCurve::CubicBezier {
            segments,
            symmetric,
        } => ResponseCurve::cubic_bezier(segments.clone(), *symmetric).ok(),
    }
}

/// Return a safe identity fallback with the same type and symmetry as `curve`.
///
/// Symmetric curves store all points on both sides of the origin.
pub(super) fn default_identity_curve(curve: &ResponseCurve) -> ResponseCurve {
    match curve {
        ResponseCurve::PiecewiseLinear { symmetric, .. } => {
            let pts = vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)];
            ResponseCurve::piecewise_linear(pts, *symmetric).unwrap_or_else(|_| {
                ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (1.0, 1.0)], false)
                    .expect("hardcoded identity is valid")
            })
        }
        ResponseCurve::CubicSpline { symmetric, .. } => {
            let pts = vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)];
            ResponseCurve::cubic_spline(pts, *symmetric).unwrap_or_else(|_| {
                ResponseCurve::cubic_spline(vec![(-1.0, -1.0), (1.0, 1.0)], false)
                    .expect("hardcoded identity is valid")
            })
        }
        ResponseCurve::CubicBezier { symmetric, .. } => {
            let segs = if *symmetric {
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
            };
            ResponseCurve::cubic_bezier(segs, *symmetric).unwrap_or_else(|_| {
                let fallback_seg = BezierSegment {
                    start: (-1.0, -1.0),
                    control1: (-1.0 / 3.0, -1.0 / 3.0),
                    control2: (1.0 / 3.0, 1.0 / 3.0),
                    end: (1.0, 1.0),
                };
                ResponseCurve::cubic_bezier(vec![fallback_seg], false)
                    .expect("hardcoded bezier identity is valid")
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Curve type conversion
// ---------------------------------------------------------------------------

/// Convert a curve to a different [`CurveType`] by creating fresh defaults.
///
/// Matches `JoystickGremlin` behavior: switching types resets the curve to
/// a clean identity rather than attempting to preserve arbitrary point
/// configurations. Preserves the symmetric flag and applies enforcement if
/// symmetric.
pub(super) fn convert_curve_type(
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
            let segs = if symmetric {
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
            };
            ResponseCurve::cubic_bezier(segs, symmetric).ok()
        }
    }
}

// ---------------------------------------------------------------------------
// Add / remove control points
// ---------------------------------------------------------------------------

/// Add a control point at `pos` on the curve. For symmetric curves, also
/// adds the mirror point at `(-x, -y)`.
///
/// Returns `true` when the point was added successfully.
pub(super) fn add_control_point(curve: &mut ResponseCurve, pos: PlotPoint) -> bool {
    let x = pos.x.clamp(-1.0, 1.0);
    let y = pos.y.clamp(-1.0, 1.0);

    match curve {
        ResponseCurve::PiecewiseLinear {
            points, symmetric, ..
        }
        | ResponseCurve::CubicSpline {
            points, symmetric, ..
        } => {
            let original_points = points.clone();
            points.push((x, y));
            if *symmetric {
                // Add mirror point (skip if at origin).
                if x.abs() > 0.0 {
                    points.push((-x, -y));
                }
            }
            points.sort_by(|a, b| a.0.total_cmp(&b.0));
            // Validate: check x values are strictly increasing after sort.
            if points.windows(2).all(|w| w[0].0 < w[1].0) {
                true
            } else {
                // Rollback: restore original points on validation failure.
                *points = original_points;
                false
            }
        }
        ResponseCurve::CubicBezier {
            segments,
            symmetric,
        } => {
            // Find the segment containing x.
            let Some(seg_idx) = segments.iter().position(|s| s.start.0 <= x && x <= s.end.0) else {
                return false;
            };

            // Compute t parameter (linear approximation).
            let seg = &segments[seg_idx];
            let dx = seg.end.0 - seg.start.0;
            if dx.abs() < f64::EPSILON {
                return false;
            }
            let t = ((x - seg.start.0) / dx).clamp(0.05, 0.95);

            // De Casteljau split.
            let (left, right) = split_bezier_segment(seg, t);
            segments.splice(seg_idx..=seg_idx, [left, right]);

            // Mirror in symmetric mode.
            if *symmetric {
                // segments.len() is post-splice (original + 1).
                let pre_splice_count = segments.len() - 1;
                let mut mirror_seg = pre_splice_count - 1 - seg_idx;
                // Adjust mirror index for the splice insertion at seg_idx.
                if mirror_seg >= seg_idx {
                    mirror_seg += 1;
                }
                if mirror_seg != seg_idx && mirror_seg != seg_idx + 1 {
                    // Compute mirror t from the mirror segment's geometry
                    // so the split point lands at (-x, -y) for antisymmetry.
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

/// Remove control point at `index`. For symmetric curves, also removes the
/// mirror point. Edge points and center point cannot be removed.
///
/// Returns `true` when the point was removed successfully.
pub(super) fn remove_control_point(curve: &mut ResponseCurve, index: usize) -> bool {
    match curve {
        ResponseCurve::PiecewiseLinear {
            points, symmetric, ..
        }
        | ResponseCurve::CubicSpline {
            points, symmetric, ..
        } => {
            let count = points.len();

            // Cannot remove edge points.
            if index == 0 || index == count - 1 {
                return false;
            }
            // Cannot remove center point in symmetric mode.
            if *symmetric && count % 2 == 1 && index == count / 2 {
                return false;
            }
            // Need at least 2 points after removal.
            let removals = if *symmetric { 2 } else { 1 };
            if count <= removals + 1 {
                return false;
            }

            if *symmetric {
                let mirror_idx = count - 1 - index;
                // Center point (index == mirror_idx) is already blocked above.
                debug_assert_ne!(index, mirror_idx, "center removal should be caught earlier");
                // Remove higher index first to avoid shifting.
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

            // Only junction points (endpoints shared between segments) can be removed.
            // Control handles (local 1, 2) cannot be removed independently.
            if local == 1 || local == 2 {
                return false;
            }

            // Determine which two segments share this junction.
            let (left_idx, right_idx) = if local == 3 {
                (seg_idx, seg_idx + 1)
            } else {
                // local == 0
                if seg_idx == 0 {
                    return false; // First start point, edge.
                }
                (seg_idx - 1, seg_idx)
            };

            let seg_count = segments.len();
            if right_idx >= seg_count {
                return false; // Last end point, edge.
            }
            // Need at least 1 segment after merge.
            if seg_count < 2 {
                return false;
            }

            // Cannot remove center junction in symmetric mode.
            if *symmetric && seg_count % 2 == 0 {
                let center_seg = seg_count / 2;
                if (local == 3 && seg_idx == center_seg - 1)
                    || (local == 0 && seg_idx == center_seg)
                {
                    return false;
                }
            }

            // Merge: keep left's start+control1, right's control2+end.
            let merged = BezierSegment {
                start: segments[left_idx].start,
                control1: segments[left_idx].control1,
                control2: segments[right_idx].control2,
                end: segments[right_idx].end,
            };
            segments.splice(left_idx..=right_idx, [merged]);

            // Mirror in symmetric mode.
            if *symmetric {
                // The primary merge replaced 2 segments with 1, so
                // pre_merge_count = current + 1.
                let pre_merge_count = segments.len() + 1;
                let mut mirror_left = pre_merge_count - 2 - left_idx;
                // Adjust: the merge at left_idx reduced indices above it by 1.
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

/// Linearly interpolate between two 2D points.
fn lerp_point(a: (f64, f64), b: (f64, f64), t: f64) -> (f64, f64) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// Split a cubic Bezier segment at parameter `t` using De Casteljau's algorithm.
///
/// Returns the two sub-segments `(left, right)` whose union equals the original.
fn split_bezier_segment(seg: &BezierSegment, t: f64) -> (BezierSegment, BezierSegment) {
    // Level 1: interpolate between adjacent original control points.
    let ab = lerp_point(seg.start, seg.control1, t);
    let bc = lerp_point(seg.control1, seg.control2, t);
    let cd = lerp_point(seg.control2, seg.end, t);
    // Level 2: interpolate between level-1 results.
    let abc = lerp_point(ab, bc, t);
    let bcd = lerp_point(bc, cd, t);
    // Level 3: the point on the curve at parameter t.
    let mid = lerp_point(abc, bcd, t);

    let left = BezierSegment {
        start: seg.start,
        control1: ab,
        control2: abc,
        end: mid,
    };
    let right = BezierSegment {
        start: mid,
        control1: bcd,
        control2: cd,
        end: seg.end,
    };
    (left, right)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Convenience constructors for test curves.
    fn identity_piecewise() -> ResponseCurve {
        ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false).unwrap()
    }

    fn identity_spline() -> ResponseCurve {
        ResponseCurve::cubic_spline(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false).unwrap()
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

    // -----------------------------------------------------------------------

    #[test]
    fn convert_curve_type_preserves_identity() {
        let curve = identity_piecewise();
        let tolerance = 0.02; // Conversion may introduce slight shape differences.

        // Piecewise → Spline.
        let spline = convert_curve_type(&curve, CurveType::CubicSpline)
            .expect("conversion to spline must succeed");
        assert!((spline.evaluate(0.0)).abs() < tolerance);
        assert!((spline.evaluate(1.0) - 1.0).abs() < tolerance);

        // Piecewise → Bezier.
        let bezier = convert_curve_type(&curve, CurveType::CubicBezier)
            .expect("conversion to bezier must succeed");
        assert!((bezier.evaluate(0.0)).abs() < tolerance);

        // Piecewise → Piecewise (identity conversion).
        let same = convert_curve_type(&curve, CurveType::PiecewiseLinear)
            .expect("same-type conversion must succeed");
        assert!((same.evaluate(0.5) - 0.5).abs() < tolerance);
    }

    #[test]
    fn convert_spline_to_bezier() {
        let curve = identity_spline();
        let bezier = convert_curve_type(&curve, CurveType::CubicBezier)
            .expect("spline-to-bezier conversion must succeed");
        assert!((bezier.evaluate(-1.0) - (-1.0)).abs() < 0.01);
        assert!((bezier.evaluate(1.0) - 1.0).abs() < 0.01);
    }

    #[test]
    fn adjacent_x_bounds_locks_edge_points() {
        let curve = identity_piecewise();
        // First point: x locked at -1.0.
        let (lo, hi) = adjacent_x_bounds(&curve, 0);
        assert!((lo - (-1.0)).abs() < f64::EPSILON);
        assert!((hi - (-1.0)).abs() < f64::EPSILON);
        // Last point: x locked at 1.0.
        let (lo, hi) = adjacent_x_bounds(&curve, 2);
        assert!((lo - 1.0).abs() < f64::EPSILON);
        assert!((hi - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn adjacent_x_bounds_middle_point() {
        let curve = identity_piecewise();
        let (lo, hi) = adjacent_x_bounds(&curve, 1);
        // Lower bound is the x of point[0] + MIN_X_GAP.
        assert!((lo - (-1.0 + MIN_X_GAP)).abs() < 1e-10);
        // Upper bound is the x of point[2] - MIN_X_GAP.
        assert!((hi - (1.0 - MIN_X_GAP)).abs() < 1e-10);
    }

    #[test]
    fn default_identity_curve_is_valid_for_each_type() {
        let pw = default_identity_curve(&identity_piecewise());
        let sp = default_identity_curve(&identity_spline());
        let bz = default_identity_curve(&identity_bezier());
        // Must evaluate without panicking and produce finite values.
        assert!(pw.evaluate(0.0).is_finite());
        assert!(sp.evaluate(0.0).is_finite());
        assert!(bz.evaluate(0.0).is_finite());
    }

    #[test]
    fn adjacent_x_bounds_edge_points_locked() {
        let curve = identity_piecewise();
        // First point: x locked at -1.0.
        let (lo, hi) = adjacent_x_bounds(&curve, 0);
        assert!((lo - (-1.0)).abs() < f64::EPSILON, "first point lo = -1.0");
        assert!((hi - (-1.0)).abs() < f64::EPSILON, "first point hi = -1.0");
        // Last point: x locked at 1.0.
        let (lo, hi) = adjacent_x_bounds(&curve, 2);
        assert!((lo - 1.0).abs() < f64::EPSILON, "last point lo = 1.0");
        assert!((hi - 1.0).abs() < f64::EPSILON, "last point hi = 1.0");
    }

    #[test]
    fn adjacent_x_bounds_symmetric_locks_center() {
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
        // Center point (index 2) must be locked at x = 0.
        let (lo, hi) = adjacent_x_bounds(&curve, 2);
        assert!((lo - 0.0).abs() < f64::EPSILON, "center lo must be 0.0");
        assert!((hi - 0.0).abs() < f64::EPSILON, "center hi must be 0.0");
    }

    #[test]
    fn center_point_frozen_in_symmetric_mode() {
        use egui_plot::PlotPoint;

        let mut curve = ResponseCurve::PiecewiseLinear {
            points: vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)],
            symmetric: true,
        };
        // Try to drag center point (index 1) to (0.3, 0.5).
        let bounds = adjacent_x_bounds(&curve, 1);
        update_point_in_curve(&mut curve, 1, PlotPoint::new(0.3, 0.5), bounds);

        if let ResponseCurve::PiecewiseLinear { points, .. } = &curve {
            assert!(
                points[1].0.abs() < f64::EPSILON,
                "center x must stay at 0, got {}",
                points[1].0
            );
            assert!(
                points[1].1.abs() < f64::EPSILON,
                "center y must stay at 0, got {}",
                points[1].1
            );
        } else {
            panic!("expected PiecewiseLinear");
        }
    }
}
