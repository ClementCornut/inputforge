//! Entry generation and command dispatch.
//!
//! `build_entries` walks the cross-product of committed rows and
//! selected modes, filtering out `(do not map)` rows and skip-on-
//! conflict (row, mode) pairs, and emits a `Vec<BulkMapEntry>` ready
//! for `EngineCommand::SetMappingsBulk`.
//!
//! `format_snapshot_label` produces the user-visible recovery
//! snapshot label.

use super::state::RowState;
use inputforge_core::action::BulkMapEntry;
use inputforge_core::profile::Profile;

pub(super) fn build_entries(
    profile: &Profile,
    rows: &[RowState],
    modes: &[String],
) -> Vec<BulkMapEntry> {
    let mut out = Vec::new();
    for row in rows {
        let Some(target) = row.target.clone() else {
            continue;
        };
        for mode in modes {
            let collides = profile.find_mapping(&row.input, mode).is_some();
            if collides && !row.replace {
                continue;
            }
            out.push(BulkMapEntry {
                input: row.input.clone(),
                mode: mode.clone(),
                output: target.clone(),
            });
        }
    }
    out
}

pub(super) fn format_snapshot_label(source_name: &str, target_id: u8) -> String {
    format!("Before batch map: {source_name} to vJoy {target_id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::bulk_map::state::RowKind;
    use inputforge_core::action::Action;
    use inputforge_core::types::{
        DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis,
    };

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
    fn excludes_do_not_map_rows() {
        let p = one_mode_profile();
        let rows = vec![axis_row(0, None, false)];
        assert!(build_entries(&p, &rows, &["Default".to_owned()]).is_empty());
    }

    #[test]
    fn excludes_skip_on_conflict_rows() {
        let mut p = one_mode_profile();
        let rows = vec![axis_row(0, Some(x_target()), false)];
        p.set_mapping(&rows[0].input, "Default", None, vec![Action::Invert]);
        assert!(build_entries(&p, &rows, &["Default".to_owned()]).is_empty());
    }

    #[test]
    fn includes_replace_rows_with_conflict() {
        let mut p = one_mode_profile();
        let rows = vec![axis_row(0, Some(x_target()), true)];
        p.set_mapping(&rows[0].input, "Default", None, vec![Action::Invert]);
        assert_eq!(build_entries(&p, &rows, &["Default".to_owned()]).len(), 1);
    }

    #[test]
    fn fans_out_across_modes_with_per_mode_conflict_filter() {
        let mut p = one_mode_profile();
        let rows = vec![axis_row(0, Some(x_target()), false)];
        p.set_mapping(&rows[0].input, "Default", None, vec![Action::Invert]);
        let entries = build_entries(&p, &rows, &["Default".to_owned(), "Combat".to_owned()]);
        assert_eq!(
            entries.len(),
            1,
            "Default skipped (conflict, replace=false); Combat created"
        );
        assert_eq!(entries[0].mode, "Combat");
    }

    #[test]
    fn includes_replace_rows_without_conflict_as_normal_create() {
        // replace=true with no existing mapping: the flag is irrelevant
        // (no mapping to replace), entry is emitted as a normal create.
        let p = one_mode_profile();
        let rows = vec![axis_row(0, Some(x_target()), true)];
        assert_eq!(build_entries(&p, &rows, &["Default".to_owned()]).len(), 1);
    }

    #[test]
    fn multi_row_mixed_state_single_mode() {
        // Three rows: replace-with-conflict, do-not-map, normal-create.
        // Expect two entries: the replace and the normal-create. The
        // do-not-map row is excluded, no entry generated.
        let mut p = one_mode_profile();
        let row_a = axis_row(0, Some(x_target()), true);
        let row_b = axis_row(1, None, false);
        let row_c = axis_row(
            2,
            Some(OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::Y },
            }),
            false,
        );
        p.set_mapping(&row_a.input, "Default", None, vec![Action::Invert]);
        let entries = build_entries(&p, &[row_a, row_b, row_c], &["Default".to_owned()]);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn label_format_matches_spec() {
        assert_eq!(
            format_snapshot_label("FlightStick", 1),
            "Before batch map: FlightStick to vJoy 1"
        );
    }
}
