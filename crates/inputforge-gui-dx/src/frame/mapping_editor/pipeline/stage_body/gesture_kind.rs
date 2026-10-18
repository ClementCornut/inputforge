//! Static gesture labels, IDs and branch taxonomy.
use inputforge_core::action::ActionBranch;
/// Discriminator carrying everything that differs between `TapGesture` and
/// `PressGesture` rendering: action constructor, IDs, labels, branch tags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GestureKind {
    Tap,
    Press,
}

impl GestureKind {
    pub(super) fn stage_name(self) -> &'static str {
        match self {
            Self::Tap => "Tap gesture",
            Self::Press => "Press gesture",
        }
    }

    pub(super) fn threshold_id_suffix(self) -> &'static str {
        match self {
            Self::Tap => "tap-threshold",
            Self::Press => "press-threshold",
        }
    }

    pub(super) fn toggle_id_suffix(self) -> &'static str {
        match self {
            Self::Tap => "tap-immediate",
            Self::Press => "press-long-fire",
        }
    }

    pub(super) fn toggle_label(self) -> &'static str {
        match self {
            Self::Tap => "Fire single tap immediately",
            Self::Press => "Fire long press when threshold is crossed",
        }
    }

    pub(super) fn edit_field_label(self) -> &'static str {
        match self {
            Self::Tap => "single timing",
            Self::Press => "long timing",
        }
    }

    pub(super) fn timing_label(self, value: bool) -> String {
        match (self, value) {
            (Self::Tap, true) => "immediate single".to_owned(),
            (Self::Tap, false) => "exclusive single/double".to_owned(),
            (Self::Press, true) => "long fires while held".to_owned(),
            (Self::Press, false) => "decide on release".to_owned(),
        }
    }

    pub(super) fn branch_a(self) -> ActionBranch {
        match self {
            Self::Tap => ActionBranch::TapSingle,
            Self::Press => ActionBranch::PressShort,
        }
    }

    pub(super) fn branch_b(self) -> ActionBranch {
        match self {
            Self::Tap => ActionBranch::TapDouble,
            Self::Press => ActionBranch::PressLong,
        }
    }

    pub(super) fn branch_a_label(self) -> (&'static str, &'static str) {
        match self {
            Self::Tap => ("Single tap", "single tap branch"),
            Self::Press => ("Short press", "short press branch"),
        }
    }

    pub(super) fn branch_b_label(self) -> (&'static str, &'static str) {
        match self {
            Self::Tap => ("Double tap", "double tap branch"),
            Self::Press => ("Long press", "long press branch"),
        }
    }
}
