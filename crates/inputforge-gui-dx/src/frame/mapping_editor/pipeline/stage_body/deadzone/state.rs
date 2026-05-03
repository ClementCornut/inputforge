// Rust guideline compliant 2026-05-03

//! F11 deadzone body local state. No Signals here; pure types so the body's
//! interaction / keyboard handlers stay unit-testable without Dioxus.

use inputforge_core::processing::deadzone::DeadzoneConfig;

use crate::frame::mapping_editor::pipeline::stage_body::deadzone::keyboard::KeyKind;
use crate::frame::mapping_editor::pipeline::stage_body::instruments::nudge_coalesce::NudgeCoalesce;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum HandleId {
    Low,
    CenterLow,
    CenterHigh,
    High,
}

impl HandleId {
    pub(crate) const ALL: [HandleId; 4] = [
        HandleId::Low,
        HandleId::CenterLow,
        HandleId::CenterHigh,
        HandleId::High,
    ];

    pub(crate) const fn next(self) -> Option<HandleId> {
        match self {
            HandleId::Low => Some(HandleId::CenterLow),
            HandleId::CenterLow => Some(HandleId::CenterHigh),
            HandleId::CenterHigh => Some(HandleId::High),
            HandleId::High => None,
        }
    }

    pub(crate) const fn prev(self) -> Option<HandleId> {
        match self {
            HandleId::Low => None,
            HandleId::CenterLow => Some(HandleId::Low),
            HandleId::CenterHigh => Some(HandleId::CenterLow),
            HandleId::High => Some(HandleId::CenterHigh),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DragInProgress {
    pub handle: HandleId,
    /// Inclusive viewBox-x bounds derived once at drag start from the
    /// neighbour thresholds; the candidate config is built only after
    /// clamping the cursor X to this interval.
    pub bounds: (f64, f64),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct BodyState {
    pub dragging: Option<DragInProgress>,
    pub hovered_handle: Option<HandleId>,
    pub focused_handle: Option<HandleId>,
    pub pre_drag_config: Option<DeadzoneConfig>,
    pub nudge_coalesce: NudgeCoalesce<KeyKind>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_next_chain_hits_each_id_then_none() {
        assert_eq!(HandleId::Low.next(), Some(HandleId::CenterLow));
        assert_eq!(HandleId::CenterLow.next(), Some(HandleId::CenterHigh));
        assert_eq!(HandleId::CenterHigh.next(), Some(HandleId::High));
        assert_eq!(HandleId::High.next(), None);
    }

    #[test]
    fn handle_prev_chain_hits_each_id_then_none() {
        assert_eq!(HandleId::High.prev(), Some(HandleId::CenterHigh));
        assert_eq!(HandleId::CenterHigh.prev(), Some(HandleId::CenterLow));
        assert_eq!(HandleId::CenterLow.prev(), Some(HandleId::Low));
        assert_eq!(HandleId::Low.prev(), None);
    }

    #[test]
    fn body_state_default_is_idle() {
        let s = BodyState::default();
        assert!(s.dragging.is_none());
        assert!(s.hovered_handle.is_none());
        assert!(s.focused_handle.is_none());
        assert!(s.pre_drag_config.is_none());
    }
}
