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

pub(crate) fn handle_pointer_down(
    mut state: BodyState,
    config: &DeadzoneConfig,
    cursor: (f64, f64),
    r: &PlotRect,
) -> HandlerOut {
    let Some(handle) = nearest_handle(cursor, config, r, HIT_RADIUS_PX) else {
        return (state, None, false);
    };
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
}
