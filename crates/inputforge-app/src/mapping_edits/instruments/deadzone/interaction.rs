//! Deadzone interaction behavior.
use super::mutation;
use super::state::{BodyState, DragInProgress, HandleId};
use inputforge_core::processing::deadzone::DeadzoneConfig;
use inputforge_gui_dx::instruments::Observation;
pub(crate) type HandlerOut = (BodyState, Option<DeadzoneConfig>, bool);
pub(crate) fn handle_pointer_down(
    mut state: BodyState,
    config: &DeadzoneConfig,
    observation: &Observation,
) -> HandlerOut {
    let Some(handle) = observation.hit.and_then(|i| HandleId::ALL.get(i).copied()) else {
        return (state, None, false);
    };

    if matches!(handle, HandleId::CenterLow | HandleId::CenterHigh) && observation.centers {
        state.pending_split = Some((HandleId::CenterLow, HandleId::CenterHigh));
        state.pre_drag_config = Some(config.clone());
        return (state, None, false);
    }

    state.dragging = Some(DragInProgress {
        handle,
        bounds: mutation::adjacent_bounds(handle, config),
    });
    state.pre_drag_config = Some(config.clone());
    (state, None, false)
}

pub(crate) fn handle_pointer_move(
    mut state: BodyState,
    config: &DeadzoneConfig,
    observation: &Observation,
) -> HandlerOut {
    if let Some((left, right)) = state.pending_split {
        let dx = observation.split_dx;
        if dx.abs() < 2.0 {
            return (state, None, false);
        }
        let chosen = if dx > 0.0 { right } else { left };
        state.pending_split = None;
        state.dragging = Some(DragInProgress {
            handle: chosen,
            bounds: mutation::adjacent_bounds(chosen, config),
        });
    }

    if let Some(ref drag) = state.dragging {
        let Some((vx, _vy)) = observation.position else {
            return (state, None, false);
        };
        let clamped = vx.min(drag.bounds.1).max(drag.bounds.0);
        return match mutation::with_handle(config, drag.handle, clamped) {
            Ok(new_config) => (state, Some(new_config), true),
            Err(_) => (state, None, false),
        };
    }
    state.hovered_handle = observation.hit.and_then(|i| HandleId::ALL.get(i).copied());
    (state, None, false)
}

pub(crate) fn handle_pointer_up(
    mut state: BodyState,
    working_config: &DeadzoneConfig,
) -> (BodyState, Result<DeadzoneConfig, String>, bool) {
    if state.pending_split.is_some() {
        state.pending_split = None;
        state.pre_drag_config = None;
        return (state, Err(String::new()), false);
    }
    if state.dragging.is_none() {
        return (state, Err(String::new()), false);
    }
    state.dragging = None;
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
    use super::super::super::pointer_fixture::*;
    use super::*;

    use super::super::mutation::{adjacent_bounds, handle_positions, with_handle};

    fn observe(cursor: (f64, f64), r: &PlotRect, points: &[(f64, f64)]) -> Observation {
        let hit = nearest_anchor(cursor, points, r, 10.0);
        let centers = points.len() == 4 && [1, 2].iter().all(|&i| near(cursor, points[i], r, 10.0));
        let split_dx = points
            .get(1)
            .map_or(0.0, |p| cursor.0 - viewbox_to_screen(*p, r).0);
        Observation {
            position: screen_to_viewbox(cursor, r),
            hit,
            centers,
            split_dx,
        }
    }
    fn handle_pointer_down_pixels(
        state: BodyState,
        value: &DeadzoneConfig,
        cursor: (f64, f64),
        r: &PlotRect,
    ) -> HandlerOut {
        handle_pointer_down(state, value, &observe(cursor, r, &handle_positions(value)))
    }
    fn handle_pointer_move_pixels(
        state: BodyState,
        value: &DeadzoneConfig,
        cursor: (f64, f64),
        r: &PlotRect,
    ) -> HandlerOut {
        handle_pointer_move(state, value, &observe(cursor, r, &handle_positions(value)))
    }

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
        let (s, _, _) = handle_pointer_down_pixels(BodyState::default(), &c, cursor, &r);
        assert_eq!(s.dragging.unwrap().handle, HandleId::Low);
        assert_eq!(s.pre_drag_config, Some(c));
    }

    #[test]
    fn pointer_down_off_handle_no_drag() {
        let c = cfg();
        let r = rect();
        let (s, _, _) = handle_pointer_down_pixels(BodyState::default(), &c, (0.0, 0.0), &r);
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
        let (_, new_cfg, changed) = handle_pointer_move_pixels(s, &c, cursor, &r);
        assert!(changed);
        assert!(new_cfg.is_some());
    }

    #[test]
    fn pointer_move_idle_updates_hover_only() {
        let c = cfg();
        let r = rect();
        let cursor = screen_for(HandleId::High, &c, &r);
        let (s, new_cfg, _) = handle_pointer_move_pixels(BodyState::default(), &c, cursor, &r);
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
        let (s, _, _) = handle_pointer_down_pixels(BodyState::default(), &c, cursor, &r);
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
        let (next, new_cfg, _) = handle_pointer_move_pixels(s, &c, cursor, &r);
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
        let (next, _, _) = handle_pointer_move_pixels(s, &c, cursor, &r);
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
        let (next, new_cfg, changed) = handle_pointer_move_pixels(s, &c, cursor, &r);
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
