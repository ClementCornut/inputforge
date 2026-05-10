//! Pure logic for mode-tabs runtime-marker derivation and name validation.

use inputforge_core::engine::MAX_MODE_NAME_GRAPHEMES;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeMarker {
    pub tab_index: Option<usize>,
}

/// Compute where the runtime-marker dot should render.
///
/// `tab_index = None` when no profile is loaded or `current_mode` does not
/// match any tab (e.g., engine is running a mode that was renamed mid-flight).
/// In that case the renderer omits the dot entirely.
pub(crate) fn runtime_marker(modes: &[String], current_mode: &str) -> RuntimeMarker {
    let tab_index = modes.iter().position(|m| m == current_mode);
    RuntimeMarker { tab_index }
}

/// Whether the Delete action should be disabled for the named tab.
///
/// Spec: delete is disabled when the tab is the first mode or the startup mode.
/// Pure function, no Dioxus runtime, no profile access.
pub(crate) fn delete_disabled_for_tab(name: &str, modes: &[String], startup: Option<&str>) -> bool {
    let is_first = modes.first().is_some_and(|first| first == name);
    let is_startup = startup.is_some_and(|startup| startup == name);
    is_first || is_startup
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
    TooLong { len: usize, max: usize },
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
    // Grapheme-aware length check sits between empty and duplicate so
    // an over-length name surfaces a length error rather than a
    // duplicate-with-truncation false positive. `graphemes(_, true)`
    // requests extended grapheme clusters (UAX #29 extended), which
    // matches the user-visible "character" count for emoji + ZWJ
    // sequences and combining marks.
    let grapheme_count = UnicodeSegmentation::graphemes(trimmed, true).count();
    if grapheme_count > MAX_MODE_NAME_GRAPHEMES {
        return NameValidation::TooLong {
            len: grapheme_count,
            max: MAX_MODE_NAME_GRAPHEMES,
        };
    }
    let is_duplicate = existing
        .iter()
        .filter(|n| self_name != Some(n.as_str()))
        .any(|n| n.eq_ignore_ascii_case(trimmed));
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
    fn runtime_marker_returns_index_for_match() {
        let m = runtime_marker(&modes(), "Combat");
        assert_eq!(m.tab_index, Some(1));
    }

    #[test]
    fn runtime_marker_no_match_returns_none_index() {
        let m = runtime_marker(&modes(), "Mystery");
        assert!(m.tab_index.is_none());
    }

    #[test]
    fn runtime_marker_empty_modes_returns_none_index() {
        let m = runtime_marker(&[], "anything");
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
    fn validate_duplicate_is_ascii_case_insensitive() {
        assert_eq!(
            validate_mode_name("combat", &modes(), None),
            NameValidation::Duplicate {
                name: "combat".to_owned(),
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

    #[test]
    fn validate_too_long() {
        // 65 ASCII chars = 65 graphemes, one past the cap.
        let raw = "x".repeat(65);
        assert_eq!(
            validate_mode_name(&raw, &modes(), None),
            NameValidation::TooLong {
                len: 65,
                max: MAX_MODE_NAME_GRAPHEMES,
            }
        );
    }

    #[test]
    fn validate_at_cap_is_valid() {
        // Exactly 64 graphemes is on the boundary and must validate.
        let raw = "x".repeat(MAX_MODE_NAME_GRAPHEMES);
        assert_eq!(
            validate_mode_name(&raw, &modes(), None),
            NameValidation::Valid(raw.clone())
        );
    }

    #[test]
    fn delete_disabled_for_root_tab() {
        assert!(delete_disabled_for_tab("Default", &modes(), Some("Combat")));
    }

    #[test]
    fn delete_disabled_when_tab_is_startup() {
        assert!(delete_disabled_for_tab("Combat", &modes(), Some("Combat")));
    }

    #[test]
    fn delete_enabled_for_non_first_non_startup_tab() {
        assert!(!delete_disabled_for_tab(
            "Landing",
            &modes(),
            Some("Combat")
        ));
    }

    #[test]
    fn delete_enabled_when_no_startup_set() {
        assert!(!delete_disabled_for_tab("Landing", &modes(), None));
    }

    #[test]
    fn validate_grapheme_aware_emoji() {
        // "👨‍👩‍👧‍👦" is one extended grapheme cluster (family ZWJ
        // sequence, 7 codepoints, 25 bytes) and "🇫🇷" is one (regional-
        // indicator pair, 2 codepoints, 8 bytes). Together with three
        // ASCII chars the input has 5 user-visible graphemes, well
        // under the cap, so a grapheme-aware checker accepts it. A
        // naive `chars().count()` would yield 12 (still under the cap
        // but over-counting), and a byte-count comparison would yield
        // 38 (still under for this string but the wrong yardstick).
        // The test pins the contract: graphemes are what we count.
        let raw = "ab👨\u{200D}👩\u{200D}👧\u{200D}👦c🇫🇷";
        let grapheme_count = UnicodeSegmentation::graphemes(raw, true).count();
        assert_eq!(grapheme_count, 5);
        // chars() over-counts (12 codepoints vs 5 graphemes), proves
        // the input is non-trivially multi-codepoint.
        assert!(raw.chars().count() > grapheme_count);
        assert_eq!(
            validate_mode_name(raw, &modes(), None),
            NameValidation::Valid(raw.to_owned())
        );
    }
}
