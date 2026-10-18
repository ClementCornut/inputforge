//! Shared action variants, sizes, and presentation choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Ghost,
    Danger,
    GhostDanger,
    GhostPrimary,
}

impl ButtonVariant {
    #[must_use]
    pub(crate) fn modifier(self) -> &'static str {
        match self {
            ButtonVariant::Primary => "primary",
            ButtonVariant::Secondary => "secondary",
            ButtonVariant::Ghost => "ghost",
            ButtonVariant::Danger => "danger",
            ButtonVariant::GhostDanger => "ghost-danger",
            ButtonVariant::GhostPrimary => "ghost-primary",
        }
    }

    /// Class string for any button-family component.
    /// `prefix` is the BEM block (e.g. `"if-button"`, `"if-icon-button"`).
    #[must_use]
    pub(crate) fn class_for(self, prefix: &str) -> String {
        format!("{prefix}--{}", self.modifier())
    }

    pub(super) fn class(self) -> String {
        self.class_for("if-button")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonSize {
    Sm,
    Md,
    Lg,
}

impl ButtonSize {
    #[must_use]
    pub(crate) fn modifier(self) -> &'static str {
        match self {
            ButtonSize::Sm => "sm",
            ButtonSize::Md => "md",
            ButtonSize::Lg => "lg",
        }
    }

    /// Class string for any button-family component.
    /// `prefix` is the BEM block (e.g. `"if-button"`, `"if-icon-button"`).
    #[must_use]
    pub(crate) fn class_for(self, prefix: &str) -> String {
        format!("{prefix}--{}", self.modifier())
    }

    pub(super) fn class(self) -> String {
        self.class_for("if-button")
    }
}

/// Visual composition for an action with native button semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonPresentation {
    #[default]
    Standard,
    Compact,
    Row,
    Disclosure,
    Navigation,
    Status,
    Create,
}
impl ButtonPresentation {
    pub(super) fn class(self) -> &'static str {
        match self {
            Self::Standard => "",
            Self::Compact => "if-button--compact",
            Self::Row => "if-button--row if-action-row",
            Self::Disclosure => "if-button--disclosure",
            Self::Navigation => "if-button--navigation",
            Self::Status => "if-button--status",
            Self::Create => "if-button--create",
        }
    }
}

/// Semantic status color for an action; the caller owns its state and label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActionTone {
    #[default]
    Neutral,
    Live,
    Warning,
    Error,
}
impl ActionTone {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Live => "live",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}
