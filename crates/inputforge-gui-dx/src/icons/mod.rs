//! Icon enum + SVG registry. SVGs sourced from Phosphor Icons (MIT).
//! Phosphor release tag used: v2.0.8
//!
//! Each variant maps to a `.svg` file under `src/icons/svg/` via
//! `include_str!()`. SVG content is compile-time embedded, so adding
//! a new icon = drop a `.svg` file + add a variant + add a match arm.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    Joystick,
    Device,
    Axis,
    Button,
    Hat,
    Mode,
    Profile,
    Save,
    Copy,
    Eye,
    EyeOff,
    Link,
    Plus,
    Minus,
    Trash,
    Settings,
    ChevronDown,
    ChevronUp,
    ChevronLeft,
    ChevronRight,
    X,
    Check,
    Warning,
    Info,
    Error,
    Play,
    Pause,
    Refresh,
    /// Phosphor `clock-counter-clockwise`. Used as the "restore from
    /// history" affordance on snapshot rows. Distinct glyph from
    /// `Refresh` (two-arrow cycle) so it does not collide with the
    /// `AutoBeforeRestore` kind icon when rendered side-by-side.
    ClockCounterClockwise,
    DragHandle,
    DotsVertical,
    /// Phosphor `folder-open`. Used as the "Open file..." affordance
    /// trailing the Profiles filter input. The folder-with-open-flap
    /// glyph reads as "load from disk" without conflating with `Profile`
    /// (single profile glyph) or `Plus` (create-new affordance).
    FolderOpen,
}

impl Icon {
    #[must_use]
    pub fn svg(&self) -> &'static str {
        match self {
            Icon::Joystick => include_str!("svg/joystick.svg"),
            Icon::Device => include_str!("svg/device.svg"),
            Icon::Axis => include_str!("svg/axis.svg"),
            Icon::Button => include_str!("svg/button.svg"),
            Icon::Hat => include_str!("svg/hat.svg"),
            Icon::Mode => include_str!("svg/mode.svg"),
            Icon::Profile => include_str!("svg/profile.svg"),
            Icon::Save => include_str!("svg/save.svg"),
            Icon::Copy => include_str!("svg/copy.svg"),
            Icon::Eye => include_str!("svg/eye.svg"),
            Icon::EyeOff => include_str!("svg/eye-off.svg"),
            Icon::Link => include_str!("svg/link.svg"),
            Icon::Plus => include_str!("svg/plus.svg"),
            Icon::Minus => include_str!("svg/minus.svg"),
            Icon::Trash => include_str!("svg/trash.svg"),
            Icon::Settings => include_str!("svg/settings.svg"),
            Icon::ChevronDown => include_str!("svg/chevron-down.svg"),
            Icon::ChevronUp => include_str!("svg/chevron-up.svg"),
            Icon::ChevronLeft => include_str!("svg/chevron-left.svg"),
            Icon::ChevronRight => include_str!("svg/chevron-right.svg"),
            Icon::X => include_str!("svg/x.svg"),
            Icon::Check => include_str!("svg/check.svg"),
            Icon::Warning => include_str!("svg/warning.svg"),
            Icon::Info => include_str!("svg/info.svg"),
            Icon::Error => include_str!("svg/error.svg"),
            Icon::Play => include_str!("svg/play.svg"),
            Icon::Pause => include_str!("svg/pause.svg"),
            Icon::Refresh => include_str!("svg/refresh.svg"),
            Icon::ClockCounterClockwise => include_str!("svg/clock-counter-clockwise.svg"),
            Icon::DragHandle => include_str!("svg/drag-handle.svg"),
            Icon::DotsVertical => include_str!("svg/dots-vertical.svg"),
            Icon::FolderOpen => include_str!("svg/folder-open.svg"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconSize {
    Sm,
    Md,
    Lg,
}

impl IconSize {
    #[must_use]
    pub fn class(self) -> &'static str {
        match self {
            IconSize::Sm => "if-icon--sm",
            IconSize::Md => "if-icon--md",
            IconSize::Lg => "if-icon--lg",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: &[Icon] = &[
        Icon::Joystick,
        Icon::Device,
        Icon::Axis,
        Icon::Button,
        Icon::Hat,
        Icon::Mode,
        Icon::Profile,
        Icon::Save,
        Icon::Copy,
        Icon::Eye,
        Icon::EyeOff,
        Icon::Link,
        Icon::Plus,
        Icon::Minus,
        Icon::Trash,
        Icon::Settings,
        Icon::ChevronDown,
        Icon::ChevronUp,
        Icon::ChevronLeft,
        Icon::ChevronRight,
        Icon::X,
        Icon::Check,
        Icon::Warning,
        Icon::Info,
        Icon::Error,
        Icon::Play,
        Icon::Pause,
        Icon::Refresh,
        Icon::DragHandle,
        Icon::DotsVertical,
        Icon::FolderOpen,
    ];

    #[test]
    fn every_variant_returns_non_empty_svg() {
        for icon in ALL {
            let svg = icon.svg();
            assert!(!svg.is_empty(), "{icon:?} svg is empty");
        }
    }

    #[test]
    fn every_variant_returns_well_formed_svg() {
        for icon in ALL {
            let svg = icon.svg();
            // Defensive: strip optional UTF-8 BOM (trim_start does NOT strip \u{FEFF}).
            let head = svg.trim_start_matches('\u{FEFF}').trim_start();
            assert!(
                head.starts_with("<svg") || head.starts_with("<?xml"),
                "{icon:?} does not start with <svg or <?xml prologue (got: {:?})",
                &svg[..svg.len().min(40)]
            );
            assert!(
                svg.contains("viewBox"),
                "{icon:?} missing viewBox attribute"
            );
        }
    }

    #[test]
    fn icon_size_class_names() {
        assert_eq!(IconSize::Sm.class(), "if-icon--sm");
        assert_eq!(IconSize::Md.class(), "if-icon--md");
        assert_eq!(IconSize::Lg.class(), "if-icon--lg");
    }
}
