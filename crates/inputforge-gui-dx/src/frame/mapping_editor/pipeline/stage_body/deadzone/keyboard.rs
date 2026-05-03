// Rust guideline compliant 2026-05-03

//! Pure keyboard handler for F11. Driven by `KeyInput` (a normalised event
//! shape) so the host body can route Dioxus `KeyboardEvent` through this
//! function without coupling tests to Dioxus types.

use inputforge_core::processing::deadzone::DeadzoneConfig;

use crate::frame::mapping_editor::pipeline::stage_body::deadzone::mutation::{
    adjacent_bounds, with_handle,
};
use crate::frame::mapping_editor::pipeline::stage_body::deadzone::state::{BodyState, HandleId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyInput {
    Tab,
    ShiftTab,
    ArrowLeft { shift: bool },
    ArrowRight { shift: bool },
    ArrowUp { shift: bool },
    ArrowDown { shift: bool },
    Home,
    End,
    Enter,
    Delete,
    Escape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyKind {
    ArrowLeft,
    ArrowRight,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum KeyOutcome {
    PushUndo { label: String },
    MergeUndo,
}

pub(crate) type KeyHandlerOut = (BodyState, Option<DeadzoneConfig>, Option<KeyOutcome>, bool);

const SMALL_STEP: f64 = 0.01;
const LARGE_STEP: f64 = 0.10;

pub(crate) fn handle_key(
    mut state: BodyState,
    config: &DeadzoneConfig,
    key: KeyInput,
    now_ms: u64,
) -> KeyHandlerOut {
    match key {
        KeyInput::Tab => {
            state.focused_handle = match state.focused_handle {
                None => Some(HandleId::Low),
                Some(h) => h.next(),
            };
            (state, None, None, false)
        }
        KeyInput::ShiftTab => {
            state.focused_handle = match state.focused_handle {
                None => Some(HandleId::Low),
                Some(h) => h.prev(),
            };
            (state, None, None, false)
        }
        KeyInput::Home => {
            state.focused_handle = Some(HandleId::Low);
            (state, None, None, false)
        }
        KeyInput::End => {
            state.focused_handle = Some(HandleId::High);
            (state, None, None, false)
        }
        KeyInput::Escape => {
            if let Some(prev) = state.pre_drag_config.take() {
                state.dragging = None;
                return (state, Some(prev), None, true);
            }
            (state, None, None, false)
        }
        KeyInput::ArrowLeft { shift } => {
            nudge(state, config, -step(shift), KeyKind::ArrowLeft, now_ms)
        }
        KeyInput::ArrowRight { shift } => {
            nudge(state, config, step(shift), KeyKind::ArrowRight, now_ms)
        }
        // Up/Down arrows are silently ignored: F11 is an X-axis-only
        // editor, so vertical nudges have no semantics. Enter and Delete
        // also fall through here since the body has no commit / remove
        // affordance for individual handles.
        KeyInput::ArrowUp { .. }
        | KeyInput::ArrowDown { .. }
        | KeyInput::Enter
        | KeyInput::Delete => (state, None, None, false),
    }
}

const fn step(shift: bool) -> f64 {
    if shift { LARGE_STEP } else { SMALL_STEP }
}

fn nudge(
    mut state: BodyState,
    config: &DeadzoneConfig,
    delta: f64,
    kind: KeyKind,
    now_ms: u64,
) -> KeyHandlerOut {
    let Some(handle) = state.focused_handle else {
        return (state, None, None, false);
    };
    let current_x = match handle {
        HandleId::Low => config.low(),
        HandleId::CenterLow => config.center_low(),
        HandleId::CenterHigh => config.center_high(),
        HandleId::High => config.high(),
    };
    let (lo, hi) = adjacent_bounds(handle, config);
    let target = (current_x + delta).min(hi).max(lo);
    if (target - current_x).abs() < f64::EPSILON {
        return (state, None, None, false);
    }
    let Ok(new_config) = with_handle(config, handle, target) else {
        return (state, None, None, false);
    };
    let merge = state.nudge_coalesce.should_merge(now_ms, kind);
    state.nudge_coalesce.record(now_ms, kind);
    let outcome = if merge {
        KeyOutcome::MergeUndo
    } else {
        let label_handle = match handle {
            HandleId::Low => "low",
            HandleId::CenterLow => "center_low",
            HandleId::CenterHigh => "center_high",
            HandleId::High => "high",
        };
        KeyOutcome::PushUndo {
            label: format!("deadzone: {label_handle} {current_x:+.2} -> {target:+.2}"),
        }
    };
    (state, Some(new_config), Some(outcome), true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::mapping_editor::pipeline::stage_body::deadzone::mutation::default_config;

    fn cfg() -> DeadzoneConfig {
        DeadzoneConfig::new(-0.85, -0.15, 0.15, 0.85).expect("valid")
    }

    fn seed() -> (DeadzoneConfig, BodyState) {
        let c = cfg();
        let s = BodyState {
            focused_handle: Some(HandleId::Low),
            ..BodyState::default()
        };
        (c, s)
    }

    #[test]
    fn tab_advances_focus() {
        let (c, mut s) = seed();
        s.focused_handle = Some(HandleId::CenterLow);
        let (next, _, _, _) = handle_key(s, &c, KeyInput::Tab, 0);
        assert_eq!(next.focused_handle, Some(HandleId::CenterHigh));
    }

    #[test]
    fn tab_at_high_yields_none_so_browser_advances_focus() {
        let (c, mut s) = seed();
        s.focused_handle = Some(HandleId::High);
        let (next, _, _, _) = handle_key(s, &c, KeyInput::Tab, 0);
        assert_eq!(next.focused_handle, None);
    }

    #[test]
    fn shift_tab_at_low_yields_none() {
        let (c, mut s) = seed();
        s.focused_handle = Some(HandleId::Low);
        let (next, _, _, _) = handle_key(s, &c, KeyInput::ShiftTab, 0);
        assert_eq!(next.focused_handle, None);
    }

    #[test]
    fn home_jumps_to_low() {
        let (c, mut s) = seed();
        s.focused_handle = Some(HandleId::High);
        let (next, _, _, _) = handle_key(s, &c, KeyInput::Home, 0);
        assert_eq!(next.focused_handle, Some(HandleId::Low));
    }

    #[test]
    fn arrow_left_nudges_low_by_step_and_pushes_undo() {
        let (c, s) = seed();
        let (_, new_cfg, outcome, changed) =
            handle_key(s, &c, KeyInput::ArrowLeft { shift: false }, 0);
        assert!(changed);
        let new = new_cfg.unwrap();
        assert!((new.low() - -0.86).abs() < 1e-9);
        assert!(matches!(outcome, Some(KeyOutcome::PushUndo { .. })));
    }

    #[test]
    fn shift_arrow_uses_large_step() {
        let (c, s) = seed();
        let (_, new_cfg, _, _) = handle_key(s, &c, KeyInput::ArrowRight { shift: true }, 0);
        let new = new_cfg.unwrap();
        assert!((new.low() - -0.75).abs() < 1e-9);
    }

    #[test]
    fn second_same_key_within_window_merges_undo() {
        let (c, s) = seed();
        let (s1, new1, _, _) = handle_key(s, &c, KeyInput::ArrowRight { shift: false }, 100);
        let new1 = new1.unwrap();
        let (_, _, outcome2, _) = handle_key(s1, &new1, KeyInput::ArrowRight { shift: false }, 200);
        assert_eq!(outcome2, Some(KeyOutcome::MergeUndo));
    }

    #[test]
    fn second_same_key_past_window_pushes_new_undo() {
        let (c, s) = seed();
        let (s1, new1, _, _) = handle_key(s, &c, KeyInput::ArrowRight { shift: false }, 100);
        let new1 = new1.unwrap();
        let (_, _, outcome2, _) =
            handle_key(s1, &new1, KeyInput::ArrowRight { shift: false }, 100 + 251);
        assert!(matches!(outcome2, Some(KeyOutcome::PushUndo { .. })));
    }

    #[test]
    fn arrow_at_clamp_boundary_is_noop() {
        let (c, s) = seed();
        // Drive the Low handle to the clamp bound first.
        let (s2, _, _, _) = handle_key(s, &c, KeyInput::ArrowLeft { shift: false }, 0);
        // Now nudge again from -1.0; nothing further is allowed.
        let mut s = s2;
        s.focused_handle = Some(HandleId::Low);
        let stuck = DeadzoneConfig::new(-1.0, -0.15, 0.15, 0.85).expect("valid");
        let (_, new_cfg, outcome, changed) =
            handle_key(s, &stuck, KeyInput::ArrowLeft { shift: false }, 1000);
        assert!(!changed);
        assert!(new_cfg.is_none());
        assert!(outcome.is_none());
    }

    #[test]
    fn arrow_up_and_down_silent_no_op() {
        let (c, s) = seed();
        let (_, new_cfg, outcome, changed) =
            handle_key(s, &c, KeyInput::ArrowUp { shift: false }, 0);
        assert!(!changed);
        assert!(new_cfg.is_none());
        assert!(outcome.is_none());
    }

    #[test]
    fn enter_and_delete_silent_no_op() {
        let (c, s) = seed();
        let (_, new_cfg, _, changed) = handle_key(s, &c, KeyInput::Enter, 0);
        assert!(!changed);
        assert!(new_cfg.is_none());
        let (c, s) = seed();
        let (_, new_cfg, _, changed) = handle_key(s, &c, KeyInput::Delete, 0);
        assert!(!changed);
        assert!(new_cfg.is_none());
    }

    #[test]
    fn escape_during_drag_reverts_to_pre_drag_config() {
        let (c, mut s) = seed();
        s.dragging = Some(
            crate::frame::mapping_editor::pipeline::stage_body::deadzone::state::DragInProgress {
                handle: HandleId::Low,
                bounds: adjacent_bounds(HandleId::Low, &c),
            },
        );
        s.pre_drag_config = Some(c.clone());
        let working = with_handle(&c, HandleId::Low, -0.5).unwrap();
        let (next, new_cfg, outcome, changed) = handle_key(s, &working, KeyInput::Escape, 0);
        assert!(changed);
        assert_eq!(new_cfg.unwrap(), c);
        assert!(next.dragging.is_none());
        assert!(next.pre_drag_config.is_none());
        assert!(outcome.is_none());
    }

    // `default_config` referenced by the body's Reset path.
    #[test]
    fn default_config_alias_exists() {
        let _ = default_config();
    }
}
