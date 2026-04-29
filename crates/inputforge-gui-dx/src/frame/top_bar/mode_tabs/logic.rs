//! Pure logic for mode-tabs runtime-marker derivation and name validation.

use inputforge_core::state::ForcedMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarkerColor {
    Natural,
    Forced,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeMarker {
    pub tab_index: Option<usize>,
    pub color: MarkerColor,
}

/// Compute where the runtime-marker dot should render.
///
/// `tab_index = None` when no profile is loaded or `current_mode` does not
/// match any tab (e.g., engine is running a mode that was renamed mid-flight).
/// In that case the renderer omits the dot entirely.
#[allow(
    dead_code,
    reason = "consumed by ModeTabs in Task 29 and inline editors in Task 31"
)]
pub(crate) fn runtime_marker(
    modes: &[String],
    current_mode: &str,
    mode_force: Option<&ForcedMode>,
) -> RuntimeMarker {
    let tab_index = modes.iter().position(|m| m == current_mode);
    let color = if mode_force.is_some() {
        MarkerColor::Forced
    } else {
        MarkerColor::Natural
    };
    RuntimeMarker { tab_index, color }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "consumed by ModeTabs in Task 29 and inline editors in Task 31"
)]
pub(crate) enum NameValidation {
    Valid(String),
    Empty,
    Duplicate { name: String },
}

/// Validate a candidate mode name for inline add or rename.
///
/// - Trims leading/trailing whitespace.
/// - Empty → `Empty`.
/// - Duplicate of any name in `existing` (excluding `self_name`) → `Duplicate`.
/// - Otherwise → `Valid(trimmed)`.
///
/// `self_name` lets rename exempt the source name from the duplicate check
/// (renaming `Combat` → `Combat` is a no-op clone, not a duplicate).
#[allow(
    dead_code,
    reason = "consumed by ModeTabs in Task 29 and inline editors in Task 31"
)]
pub(crate) fn validate_mode_name(
    raw: &str,
    existing: &[String],
    self_name: Option<&str>,
) -> NameValidation {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return NameValidation::Empty;
    }
    let is_duplicate = existing
        .iter()
        .filter(|n| self_name != Some(n.as_str()))
        .any(|n| n == trimmed);
    if is_duplicate {
        return NameValidation::Duplicate {
            name: trimmed.to_owned(),
        };
    }
    NameValidation::Valid(trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modes() -> Vec<String> {
        vec![
            "Default".to_owned(),
            "Combat".to_owned(),
            "Landing".to_owned(),
        ]
    }

    #[test]
    fn runtime_marker_no_force_with_match_returns_natural() {
        let m = runtime_marker(&modes(), "Combat", None);
        assert_eq!(m.tab_index, Some(1));
        assert_eq!(m.color, MarkerColor::Natural);
    }

    #[test]
    fn runtime_marker_with_force_returns_forced_color() {
        let f = ForcedMode {
            mode: "Combat".to_owned(),
        };
        let m = runtime_marker(&modes(), "Combat", Some(&f));
        assert_eq!(m.tab_index, Some(1));
        assert_eq!(m.color, MarkerColor::Forced);
    }

    #[test]
    fn runtime_marker_no_match_returns_none_index() {
        let m = runtime_marker(&modes(), "Mystery", None);
        assert!(m.tab_index.is_none());
    }

    #[test]
    fn runtime_marker_empty_modes_returns_none_index() {
        let m = runtime_marker(&[], "anything", None);
        assert!(m.tab_index.is_none());
    }

    #[test]
    fn validate_valid() {
        assert_eq!(
            validate_mode_name("Approach", &modes(), None),
            NameValidation::Valid("Approach".to_owned())
        );
    }

    #[test]
    fn validate_trims_whitespace() {
        assert_eq!(
            validate_mode_name("  Approach  ", &modes(), None),
            NameValidation::Valid("Approach".to_owned())
        );
    }

    #[test]
    fn validate_empty() {
        assert_eq!(
            validate_mode_name("", &modes(), None),
            NameValidation::Empty
        );
        assert_eq!(
            validate_mode_name("   ", &modes(), None),
            NameValidation::Empty
        );
    }

    #[test]
    fn validate_duplicate() {
        assert_eq!(
            validate_mode_name("Combat", &modes(), None),
            NameValidation::Duplicate {
                name: "Combat".to_owned(),
            }
        );
    }

    #[test]
    fn validate_rename_exempts_self_from_duplicate_check() {
        // Renaming Combat → Combat is a no-op, not a duplicate.
        assert_eq!(
            validate_mode_name("Combat", &modes(), Some("Combat")),
            NameValidation::Valid("Combat".to_owned())
        );
    }

    #[test]
    fn validate_rename_still_rejects_other_duplicates() {
        // Renaming Combat → Landing collides with the existing Landing.
        assert_eq!(
            validate_mode_name("Landing", &modes(), Some("Combat")),
            NameValidation::Duplicate {
                name: "Landing".to_owned(),
            }
        );
    }
}
