//! Pure banner-state derivation. No Dioxus runtime dependency.

use inputforge_core::state::ForcedMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BannerState {
    Hidden,
    Diverged {
        editing: String,
        current: String,
    },
    /// Forced + aligned: `editing == current == forced`. The override is
    /// active and the user is editing the mode the engine is currently
    /// running.
    Forced {
        forced: String,
    },
    ForcedAndDiverged {
        editing: String,
        forced: String,
    },
}

pub(crate) fn derive_banner_state(
    editing: &str,
    current: &str,
    mode_force: Option<&ForcedMode>,
) -> BannerState {
    match mode_force {
        None if editing == current => BannerState::Hidden,
        None => BannerState::Diverged {
            editing: editing.to_owned(),
            current: current.to_owned(),
        },
        Some(f) if f.mode == editing => BannerState::Forced {
            forced: f.mode.clone(),
        },
        Some(f) => BannerState::ForcedAndDiverged {
            editing: editing.to_owned(),
            forced: f.mode.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aligned_unforced_is_hidden() {
        assert_eq!(
            derive_banner_state("Default", "Default", None),
            BannerState::Hidden
        );
    }

    #[test]
    fn unforced_diverged_yields_diverged() {
        assert_eq!(
            derive_banner_state("Combat", "Default", None),
            BannerState::Diverged {
                editing: "Combat".to_owned(),
                current: "Default".to_owned(),
            }
        );
    }

    #[test]
    fn forced_and_aligned_yields_forced() {
        let f = ForcedMode {
            mode: "Combat".to_owned(),
        };
        assert_eq!(
            derive_banner_state("Combat", "Combat", Some(&f)),
            BannerState::Forced {
                forced: "Combat".to_owned(),
            }
        );
    }

    #[test]
    fn forced_and_diverged_yields_forced_and_diverged() {
        let f = ForcedMode {
            mode: "Combat".to_owned(),
        };
        assert_eq!(
            derive_banner_state("Landing", "Combat", Some(&f)),
            BannerState::ForcedAndDiverged {
                editing: "Landing".to_owned(),
                forced: "Combat".to_owned(),
            }
        );
    }
}
