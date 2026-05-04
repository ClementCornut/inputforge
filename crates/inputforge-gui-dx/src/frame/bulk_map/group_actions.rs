//! Per-group bulk-action chip predicates.
//!
//! Each predicate inspects a slice of rows of one kind (Axes, Buttons,
//! Hats) plus the conflict mode list. Returns `true` when the
//! corresponding chip should render on the group header.

use super::state::RowState;

/// `skip all conflicts` chip: surfaces when at least one row in the
/// group is in replace-state and is conflict-driven (i.e., the
/// existing mapping is what triggered the replace state).
pub(super) fn show_skip_all_conflicts(rows: &[&RowState], conflicting: &[bool]) -> bool {
    rows.iter()
        .zip(conflicting.iter())
        .any(|(r, &c)| r.replace && c)
}

/// `replace all conflicts` chip: surfaces when at least one row in
/// the group is in skip-state with a conflict.
pub(super) fn show_replace_all_conflicts(rows: &[&RowState], conflicting: &[bool]) -> bool {
    rows.iter()
        .zip(conflicting.iter())
        .any(|(r, &c)| !r.replace && c && r.target.is_some())
}

/// `include all` chip: surfaces when at least one row in the group is
/// `(do not map)`.
pub(super) fn show_include_all(rows: &[&RowState]) -> bool {
    rows.iter().any(|r| r.target.is_none())
}

/// `exclude all` chip: surfaces when at least one row in the group
/// has a target set.
pub(super) fn show_exclude_all(rows: &[&RowState]) -> bool {
    rows.iter().any(|r| r.target.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::bulk_map::state::RowKind;
    use inputforge_core::types::{
        DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis,
    };

    fn axis_row(idx: u8, target: Option<OutputAddress>, replace: bool) -> RowState {
        RowState {
            kind: RowKind::Axis,
            source_index: idx,
            input: InputAddress::Bound {
                device: DeviceId("dev-1".to_owned()),
                input: InputId::Axis { index: idx },
            },
            target,
            replace,
        }
    }

    fn x_target() -> OutputAddress {
        OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::X },
        }
    }

    #[test]
    fn skip_all_conflicts_on_when_any_row_is_in_replace_with_conflict() {
        let rows = [axis_row(0, Some(x_target()), true)];
        let refs: Vec<&RowState> = rows.iter().collect();
        assert!(show_skip_all_conflicts(&refs, &[true]));
    }

    #[test]
    fn replace_all_conflicts_off_when_no_conflict_present() {
        let rows = [axis_row(0, Some(x_target()), false)];
        let refs: Vec<&RowState> = rows.iter().collect();
        assert!(!show_replace_all_conflicts(&refs, &[false]));
    }

    #[test]
    fn replace_all_conflicts_on_when_skip_state_with_conflict() {
        // Skip state (replace=false) + conflicting=true + target set
        // is exactly the condition the chip is designed to surface.
        let rows = [axis_row(0, Some(x_target()), false)];
        let refs: Vec<&RowState> = rows.iter().collect();
        assert!(show_replace_all_conflicts(&refs, &[true]));
    }

    #[test]
    fn include_all_on_when_any_row_is_do_not_map() {
        let rows = [axis_row(0, None, false)];
        let refs: Vec<&RowState> = rows.iter().collect();
        assert!(show_include_all(&refs));
    }

    #[test]
    fn exclude_all_on_when_any_row_has_a_target() {
        let rows = [axis_row(0, Some(x_target()), false)];
        let refs: Vec<&RowState> = rows.iter().collect();
        assert!(show_exclude_all(&refs));
    }
}
