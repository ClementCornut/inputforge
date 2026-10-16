//! Curve bezier edits.
use inputforge_core::processing::curves::BezierSegment;
pub(super) fn update_bezier_point(
    segments: &mut [BezierSegment],
    symmetric: bool,
    index: usize,
    new_x: f64,
    new_y: f64,
) {
    let seg_idx = index / 4;
    let local = index % 4;
    // Freeze the join between the two halves of a symmetric curve.
    if symmetric && segments.len().is_multiple_of(2) {
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
    } else if local == 0
        && seg_idx > 0
        && let Some(prev) = segments.get_mut(seg_idx - 1)
    {
        prev.end = (new_x, new_y);
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
                if primary_synced_idx != Some(target)
                    && let Some(next) = segments.get_mut(target)
                {
                    next.start = (-new_x, -new_y);
                }
            } else if mirror_local == 0 && mirror_seg_idx > 0 {
                let target = mirror_seg_idx - 1;
                if primary_synced_idx != Some(target)
                    && let Some(prev) = segments.get_mut(target)
                {
                    prev.end = (-new_x, -new_y);
                }
            }
        }
    }
}
fn lerp_point(a: (f64, f64), b: (f64, f64), t: f64) -> (f64, f64) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

pub(super) fn split_bezier_segment(seg: &BezierSegment, t: f64) -> (BezierSegment, BezierSegment) {
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
