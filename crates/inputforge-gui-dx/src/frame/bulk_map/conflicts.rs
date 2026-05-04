//! Per-(row, mode) conflict detection.
//!
//! The wizard treats a row as "conflicted" in a given mode when the
//! active profile already contains a mapping for `(row.input, mode)`.
//! With `apply_to_all_modes`, conflict checks fan out across every
//! profile mode and produce a per-(row, mode) verdict.

use inputforge_core::profile::Profile;
use inputforge_core::types::InputAddress;

/// Returns the existing mapping name (or `""` for unnamed) when
/// `(input, mode)` collides, `None` otherwise.
pub(super) fn existing_name_for(
    profile: &Profile,
    input: &InputAddress,
    mode: &str,
) -> Option<String> {
    profile
        .find_mapping(input, mode)
        .map(|m| m.name.clone().unwrap_or_default())
}

/// Returns the list of modes (from `modes`) where `input` already has
/// a mapping in `profile`.
pub(super) fn conflicting_modes(
    profile: &Profile,
    input: &InputAddress,
    modes: &[String],
) -> Vec<String> {
    modes
        .iter()
        .filter(|m| profile.find_mapping(input, m).is_some())
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::action::Action;
    use inputforge_core::types::{DeviceId, InputId};

    fn one_mode_profile() -> Profile {
        let map =
            std::collections::HashMap::from([("Default".to_owned(), vec!["Combat".to_owned()])]);
        let modes = inputforge_core::mode::ModeTree::from_adjacency(&map).unwrap();
        Profile::new(
            "T".to_owned(),
            vec![],
            modes,
            vec![],
            vec![],
            "Default".to_owned(),
        )
    }

    fn axis_zero() -> InputAddress {
        InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        }
    }

    #[test]
    fn no_conflict_returns_none() {
        let profile = one_mode_profile();
        assert!(existing_name_for(&profile, &axis_zero(), "Default").is_none());
    }

    #[test]
    fn existing_named_mapping_returns_name() {
        let mut profile = one_mode_profile();
        profile.set_mapping(
            &axis_zero(),
            "Default",
            Some("Throttle".to_owned()),
            vec![Action::Invert],
        );
        assert_eq!(
            existing_name_for(&profile, &axis_zero(), "Default").as_deref(),
            Some("Throttle")
        );
    }

    #[test]
    fn existing_unnamed_mapping_returns_empty_string() {
        let mut profile = one_mode_profile();
        profile.set_mapping(&axis_zero(), "Default", None, vec![Action::Invert]);
        assert_eq!(
            existing_name_for(&profile, &axis_zero(), "Default").as_deref(),
            Some("")
        );
    }

    #[test]
    fn conflicting_modes_lists_only_collisions() {
        let mut profile = one_mode_profile();
        profile.set_mapping(&axis_zero(), "Default", None, vec![Action::Invert]);
        let modes = vec!["Default".to_owned(), "Combat".to_owned()];
        let collisions = conflicting_modes(&profile, &axis_zero(), &modes);
        assert_eq!(collisions, vec!["Default".to_owned()]);
    }
}
