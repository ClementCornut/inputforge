// Rust guideline compliant 2026-05-03

//! Pure pointer-event handlers for F11. Each takes a snapshot of `BodyState`
//! and the current `DeadzoneConfig` and returns the next state plus an
//! optional candidate config (for the host to dispatch on commit).

use inputforge_core::processing::deadzone::DeadzoneConfig;

use crate::frame::mapping_editor::pipeline::stage_body::deadzone::mutation::{
    adjacent_bounds, handle_positions, with_handle,
};
use crate::frame::mapping_editor::pipeline::stage_body::deadzone::state::{
    BodyState, DragInProgress, HandleId,
};

/// Hit-test radius in screen pixels (matches F10's `HIT_RADIUS_PX`).
pub(crate) const HIT_RADIUS_PX: f64 = 10.0;

#[derive(Debug, Clone, Copy)]
pub(crate) struct PlotRect {
    pub x: f64,
    pub y: f64,
    pub size: f64,
}

pub(crate) type HandlerOut = (BodyState, Option<DeadzoneConfig>, bool);

#[must_use]
pub(crate) fn screen_to_viewbox(cursor: (f64, f64), r: &PlotRect) -> Option<(f64, f64)> {
    if r.size <= 0.0 {
        return None;
    }
    let nx = (cursor.0 - r.x) / r.size;
    let ny = (cursor.1 - r.y) / r.size;
    let input = -1.05 + nx * 2.1;
    let output = 1.05 - ny * 2.1;
    Some((input, output))
}

fn viewbox_to_screen(p: (f64, f64), r: &PlotRect) -> (f64, f64) {
    let nx = (p.0 + 1.05) / 2.1;
    let ny = (1.05 - p.1) / 2.1;
    (r.x + nx * r.size, r.y + ny * r.size)
}

#[must_use]
pub(crate) fn nearest_handle(
    cursor: (f64, f64),
    config: &DeadzoneConfig,
    r: &PlotRect,
    radius_px: f64,
) -> Option<HandleId> {
    let radius_sq = radius_px * radius_px;
    let positions = handle_positions(config);
    let mut best: Option<(HandleId, f64)> = None;
    for (handle, pos) in HandleId::ALL.iter().zip(positions.iter()) {
        let s = viewbox_to_screen(*pos, r);
        let dx = s.0 - cursor.0;
        let dy = s.1 - cursor.1;
        let d2 = dx * dx + dy * dy;
        if d2 <= radius_sq {
            match best {
                Some((_, bd)) if bd <= d2 => {}
                _ => best = Some((*handle, d2)),
            }
        }
    }
    best.map(|(h, _)| h)
}

/// Cursor must travel this far in screen pixels before a `pending_split`
/// commits to one of its two candidate handles. Below the threshold, a
/// pointer-down that lands on the stacked CL/CH pair stays unresolved so a
/// pixel-perfect click without motion does not silently commit a side.
const SPLIT_RESOLUTION_THRESHOLD_PX: f64 = 2.0;

/// Returns the other middle handle when called with a center-handle, else
/// `None`. Used by `handle_pointer_down` to detect the stacked CL/CH case.
const fn paired_center_handle(handle: HandleId) -> Option<HandleId> {
    match handle {
        HandleId::CenterLow => Some(HandleId::CenterHigh),
        HandleId::CenterHigh => Some(HandleId::CenterLow),
        _ => None,
    }
}

pub(crate) fn handle_pointer_down(
    mut state: BodyState,
    config: &DeadzoneConfig,
    cursor: (f64, f64),
    r: &PlotRect,
) -> HandlerOut {
    let Some(handle) = nearest_handle(cursor, config, r, HIT_RADIUS_PX) else {
        return (state, None, false);
    };

    // Stacked-center disambiguation: if the cursor sits inside hit-radius of
    // BOTH center handles (the default config keeps them at x=0; any tighter
    // config can do the same), defer the drag start until the user's first
    // significant move picks a direction. Without this, `nearest_handle`'s
    // tie-break would silently commit to whichever handle iterates first in
    // `HandleId::ALL`, trapping the cursor inside that handle's adjacent
    // bounds and refusing a drag toward the opposite side.
    if let Some(twin) = paired_center_handle(handle) {
        let positions = handle_positions(config);
        let twin_idx = HandleId::ALL.iter().position(|h| *h == twin).unwrap();
        let twin_screen = viewbox_to_screen(positions[twin_idx], r);
        let dx = twin_screen.0 - cursor.0;
        let dy = twin_screen.1 - cursor.1;
        let radius_sq = HIT_RADIUS_PX * HIT_RADIUS_PX;
        if dx * dx + dy * dy <= radius_sq {
            // Pin the pair in canonical order (CL first, CH second) so the
            // move handler maps unambiguously: leftward picks the first
            // (CL), rightward picks the second (CH).
            state.pending_split = Some((HandleId::CenterLow, HandleId::CenterHigh));
            state.pre_drag_config = Some(config.clone());
            return (state, None, false);
        }
    }

    state.dragging = Some(DragInProgress {
        handle,
        bounds: adjacent_bounds(handle, config),
    });
    state.pre_drag_config = Some(config.clone());
    (state, None, false)
}

pub(crate) fn handle_pointer_move(
    mut state: BodyState,
    config: &DeadzoneConfig,
    cursor: (f64, f64),
    r: &PlotRect,
) -> HandlerOut {
    // Resolve a deferred stacked-center split before any drag work. The two
    // candidates share a screen position at pointer-down (that is the whole
    // reason we deferred), so either anchor produces the same `dx`. The
    // first candidate is canonically the leftward handle (CL).
    if let Some((left, right)) = state.pending_split {
        let positions = handle_positions(config);
        let left_idx = HandleId::ALL.iter().position(|h| *h == left).unwrap();
        let anchor = viewbox_to_screen(positions[left_idx], r);
        let dx = cursor.0 - anchor.0;
        if dx.abs() < SPLIT_RESOLUTION_THRESHOLD_PX {
            return (state, None, false);
        }
        let chosen = if dx > 0.0 { right } else { left };
        state.pending_split = None;
        state.dragging = Some(DragInProgress {
            handle: chosen,
            bounds: adjacent_bounds(chosen, config),
        });
        // Fall through to the standard drag-move branch below so the same
        // move event also produces the first candidate config.
    }

    if let Some(ref drag) = state.dragging {
        let Some((vx, _vy)) = screen_to_viewbox(cursor, r) else {
            return (state, None, false);
        };
        let clamped = vx.min(drag.bounds.1).max(drag.bounds.0);
        return match with_handle(config, drag.handle, clamped) {
            Ok(new_config) => (state, Some(new_config), true),
            Err(_) => (state, None, false),
        };
    }
    state.hovered_handle = nearest_handle(cursor, config, r, HIT_RADIUS_PX);
    (state, None, false)
}

pub(crate) fn handle_pointer_up(
    mut state: BodyState,
    working_config: &DeadzoneConfig,
) -> (BodyState, Result<DeadzoneConfig, String>, bool) {
    // A still-set `pending_split` means the user clicked the stacked center
    // pair but never moved past the resolution threshold. Treat it as a
    // no-op: clear the deferred state and the snapshot, return the empty
    // sentinel so the bridge skips dispatch.
    if state.pending_split.is_some() {
        state.pending_split = None;
        state.pre_drag_config = None;
        return (state, Err(String::new()), false);
    }
    if state.dragging.is_none() {
        return (state, Err(String::new()), false);
    }
    state.dragging = None;
    // Working copy was built via `with_handle` which already runs through
    // `DeadzoneConfig::new`, so this re-validate is defensive only.
    match DeadzoneConfig::new(
        working_config.low(),
        working_config.center_low(),
        working_config.center_high(),
        working_config.high(),
    ) {
        Ok(valid) => {
            state.pre_drag_config = None;
            (state, Ok(valid), true)
        }
        Err(err) => (state, Err(err.to_string()), false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> DeadzoneConfig {
        DeadzoneConfig::new(-0.85, -0.15, 0.15, 0.85).expect("valid")
    }

    fn rect() -> PlotRect {
        PlotRect {
            x: 0.0,
            y: 0.0,
            size: 200.0,
        }
    }

    fn screen_for(handle: HandleId, c: &DeadzoneConfig, r: &PlotRect) -> (f64, f64) {
        let positions = handle_positions(c);
        let idx = HandleId::ALL.iter().position(|h| *h == handle).unwrap();
        viewbox_to_screen(positions[idx], r)
    }

    #[test]
    fn pointer_down_on_low_handle_starts_drag_and_snapshots() {
        let c = cfg();
        let r = rect();
        let cursor = screen_for(HandleId::Low, &c, &r);
        let (s, _, _) = handle_pointer_down(BodyState::default(), &c, cursor, &r);
        assert_eq!(s.dragging.unwrap().handle, HandleId::Low);
        assert_eq!(s.pre_drag_config, Some(c));
    }

    #[test]
    fn pointer_down_off_handle_no_drag() {
        let c = cfg();
        let r = rect();
        let (s, _, _) = handle_pointer_down(BodyState::default(), &c, (0.0, 0.0), &r);
        assert!(s.dragging.is_none());
    }

    #[test]
    fn pointer_move_during_drag_produces_candidate() {
        let c = cfg();
        let r = rect();
        let s = BodyState {
            dragging: Some(DragInProgress {
                handle: HandleId::CenterLow,
                bounds: adjacent_bounds(HandleId::CenterLow, &c),
            }),
            ..BodyState::default()
        };
        let cursor = screen_for(HandleId::CenterHigh, &c, &r);
        let (_, new_cfg, changed) = handle_pointer_move(s, &c, cursor, &r);
        assert!(changed);
        assert!(new_cfg.is_some());
    }

    #[test]
    fn pointer_move_idle_updates_hover_only() {
        let c = cfg();
        let r = rect();
        let cursor = screen_for(HandleId::High, &c, &r);
        let (s, new_cfg, _) = handle_pointer_move(BodyState::default(), &c, cursor, &r);
        assert!(new_cfg.is_none());
        assert_eq!(s.hovered_handle, Some(HandleId::High));
    }

    #[test]
    fn pointer_up_after_drag_validates() {
        let c = cfg();
        let s = BodyState {
            dragging: Some(DragInProgress {
                handle: HandleId::CenterLow,
                bounds: adjacent_bounds(HandleId::CenterLow, &c),
            }),
            pre_drag_config: Some(c.clone()),
            ..BodyState::default()
        };
        let working = with_handle(&c, HandleId::CenterLow, -0.05).unwrap();
        let (next, result, changed) = handle_pointer_up(s, &working);
        assert!(changed);
        result.unwrap();
        assert!(next.dragging.is_none());
        assert!(next.pre_drag_config.is_none());
    }

    #[test]
    fn pointer_up_without_drag_is_noop() {
        let c = cfg();
        let (_, result, changed) = handle_pointer_up(BodyState::default(), &c);
        assert!(matches!(result, Err(ref e) if e.is_empty()));
        assert!(!changed);
    }

    /// Default config has CL = CH = 0, so both center handles share a
    /// screen position and a single click hits both. The pointer-down must
    /// defer the drag start by setting `pending_split` instead of pinning
    /// to whichever handle wins `nearest_handle`'s tie-break.
    #[test]
    fn pointer_down_on_stacked_centers_defers_drag() {
        let c = DeadzoneConfig::default();
        let r = rect();
        let cursor = screen_for(HandleId::CenterLow, &c, &r);
        let (s, _, _) = handle_pointer_down(BodyState::default(), &c, cursor, &r);
        assert!(s.dragging.is_none());
        assert_eq!(
            s.pending_split,
            Some((HandleId::CenterLow, HandleId::CenterHigh))
        );
        assert_eq!(s.pre_drag_config, Some(c));
    }

    /// Once `pending_split` is set, a move event whose cursor has traveled
    /// past the resolution threshold to the right picks CH (the second
    /// handle in the canonical pair) and starts a real drag in the same
    /// move event.
    #[test]
    fn pending_split_resolves_to_high_side_on_rightward_move() {
        let c = DeadzoneConfig::default();
        let r = rect();
        let anchor = screen_for(HandleId::CenterLow, &c, &r);
        let s = BodyState {
            pending_split: Some((HandleId::CenterLow, HandleId::CenterHigh)),
            pre_drag_config: Some(c.clone()),
            ..BodyState::default()
        };
        let cursor = (anchor.0 + 10.0, anchor.1);
        let (next, new_cfg, _) = handle_pointer_move(s, &c, cursor, &r);
        assert!(next.pending_split.is_none());
        let drag = next.dragging.expect("drag should have started");
        assert_eq!(drag.handle, HandleId::CenterHigh);
        assert!(
            new_cfg.is_some(),
            "first move should also produce candidate"
        );
    }

    /// Mirror of the rightward-move test: a leftward move resolves to CL.
    #[test]
    fn pending_split_resolves_to_low_side_on_leftward_move() {
        let c = DeadzoneConfig::default();
        let r = rect();
        let anchor = screen_for(HandleId::CenterLow, &c, &r);
        let s = BodyState {
            pending_split: Some((HandleId::CenterLow, HandleId::CenterHigh)),
            pre_drag_config: Some(c.clone()),
            ..BodyState::default()
        };
        let cursor = (anchor.0 - 10.0, anchor.1);
        let (next, _, _) = handle_pointer_move(s, &c, cursor, &r);
        assert!(next.pending_split.is_none());
        let drag = next.dragging.expect("drag should have started");
        assert_eq!(drag.handle, HandleId::CenterLow);
    }

    /// A move whose cursor has not yet traveled past the resolution
    /// threshold leaves `pending_split` set and produces no candidate so
    /// jittery hands do not silently pick a side.
    #[test]
    fn pending_split_holds_below_resolution_threshold() {
        let c = DeadzoneConfig::default();
        let r = rect();
        let anchor = screen_for(HandleId::CenterLow, &c, &r);
        let s = BodyState {
            pending_split: Some((HandleId::CenterLow, HandleId::CenterHigh)),
            pre_drag_config: Some(c.clone()),
            ..BodyState::default()
        };
        let cursor = (anchor.0 + 1.0, anchor.1);
        let (next, new_cfg, changed) = handle_pointer_move(s, &c, cursor, &r);
        assert!(next.pending_split.is_some());
        assert!(next.dragging.is_none());
        assert!(new_cfg.is_none());
        assert!(!changed);
    }

    /// Pointer-up on an unresolved `pending_split` clears the deferred
    /// state and the pre-drag snapshot, and signals no-op via the empty
    /// error string so the bridge skips dispatch.
    #[test]
    fn pointer_up_with_pending_split_clears_state_and_skips_dispatch() {
        let c = DeadzoneConfig::default();
        let s = BodyState {
            pending_split: Some((HandleId::CenterLow, HandleId::CenterHigh)),
            pre_drag_config: Some(c.clone()),
            ..BodyState::default()
        };
        let (next, result, changed) = handle_pointer_up(s, &c);
        assert!(next.pending_split.is_none());
        assert!(next.dragging.is_none());
        assert!(next.pre_drag_config.is_none());
        assert!(matches!(result, Err(ref e) if e.is_empty()));
        assert!(!changed);
    }
}
