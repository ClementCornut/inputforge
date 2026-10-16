//! Curve topology edits.
use super::bezier::split_bezier_segment;
use inputforge_core::processing::curves::{BezierSegment, ResponseCurve};
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
