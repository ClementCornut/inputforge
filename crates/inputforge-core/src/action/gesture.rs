use crate::action::Action;
use crate::error::{EngineError, Result};

/// Default gesture threshold in milliseconds.
pub const DEFAULT_GESTURE_THRESHOLD_MS: u64 = 500;
/// Minimum gesture threshold in milliseconds.
pub const MIN_GESTURE_THRESHOLD_MS: u64 = 1;
/// Maximum gesture threshold in milliseconds.
pub const MAX_GESTURE_THRESHOLD_MS: u64 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionBranch {
    ConditionalTrue,
    ConditionalFalse,
    TapSingle,
    TapDouble,
    PressShort,
    PressLong,
}

impl ActionBranch {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ConditionalTrue => "if true",
            Self::ConditionalFalse => "if false",
            Self::TapSingle => "Single tap",
            Self::TapDouble => "Double tap",
            Self::PressShort => "Short press",
            Self::PressLong => "Long press",
        }
    }
}

#[must_use]
pub const fn default_gesture_threshold_ms() -> u64 {
    DEFAULT_GESTURE_THRESHOLD_MS
}

/// Validates that a gesture threshold is within the supported range.
///
/// # Errors
/// Returns [`EngineError::InvalidConfig`] when the threshold is out of range.
pub fn validate_gesture_threshold_ms(threshold_ms: u64) -> Result<()> {
    if (MIN_GESTURE_THRESHOLD_MS..=MAX_GESTURE_THRESHOLD_MS).contains(&threshold_ms) {
        Ok(())
    } else {
        Err(EngineError::InvalidConfig {
            reason: format!(
                "gesture threshold {threshold_ms}ms is outside {MIN_GESTURE_THRESHOLD_MS}..={MAX_GESTURE_THRESHOLD_MS}ms"
            ),
        })
    }
}

#[must_use]
pub fn branch_actions(action: &Action, branch: ActionBranch) -> Option<&[Action]> {
    match (action, branch) {
        (Action::Conditional { if_true, .. }, ActionBranch::ConditionalTrue) => {
            Some(if_true.as_slice())
        }
        (Action::Conditional { if_false, .. }, ActionBranch::ConditionalFalse) => {
            Some(if_false.as_slice())
        }
        (Action::TapGesture { single_tap, .. }, ActionBranch::TapSingle) => {
            Some(single_tap.as_slice())
        }
        (Action::TapGesture { double_tap, .. }, ActionBranch::TapDouble) => {
            Some(double_tap.as_slice())
        }
        (Action::PressGesture { short_press, .. }, ActionBranch::PressShort) => {
            Some(short_press.as_slice())
        }
        (Action::PressGesture { long_press, .. }, ActionBranch::PressLong) => {
            Some(long_press.as_slice())
        }
        _ => None,
    }
}

pub fn branch_actions_mut(action: &mut Action, branch: ActionBranch) -> Option<&mut Vec<Action>> {
    match (action, branch) {
        (Action::Conditional { if_true, .. }, ActionBranch::ConditionalTrue) => Some(if_true),
        (Action::Conditional { if_false, .. }, ActionBranch::ConditionalFalse) => Some(if_false),
        (Action::TapGesture { single_tap, .. }, ActionBranch::TapSingle) => Some(single_tap),
        (Action::TapGesture { double_tap, .. }, ActionBranch::TapDouble) => Some(double_tap),
        (Action::PressGesture { short_press, .. }, ActionBranch::PressShort) => Some(short_press),
        (Action::PressGesture { long_press, .. }, ActionBranch::PressLong) => Some(long_press),
        _ => None,
    }
}
