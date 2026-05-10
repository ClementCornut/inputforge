//! Pre-apply summary chip counts.
//!
//! Walks the wizard rows against the conflict map and reports the
//! `(create, replace, skip, excluded)` tuple shown in the summary
//! chip. With `apply_to_all_modes`, the tally fans out across every
//! mode in `modes`; otherwise the tally counts only `current_mode`.
//!
//! **Asymmetry note.** `excluded` counts each `(do not map)` row
//! exactly once: exclusion is a row-level decision and never reaches
//! a per-(row, mode) verdict. `create` / `replace` / `skip` fan out
//! across `modes` because they describe per-(row, mode) outcomes, so a
//! single row over five modes contributes up to five increments to
//! that triple. Test `excluded_does_not_fan_out_across_modes` locks
//! the asymmetry.

use super::state::RowState;
use inputforge_core::profile::Profile;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct SummaryCounts {
    pub create: usize,
    pub replace: usize,
    pub skip: usize,
    pub excluded: usize,
}

pub(super) fn tally(profile: &Profile, rows: &[RowState], modes: &[String]) -> SummaryCounts {
    let mut counts = SummaryCounts::default();
    for row in rows {
        if row.target.is_none() {
            counts.excluded += 1;
            continue;
        }
        for mode in modes {
            let collides = profile.find_mapping(&row.input, mode).is_some();
            match (collides, row.replace) {
                (false, _) => counts.create += 1,
                (true, true) => counts.replace += 1,
                (true, false) => counts.skip += 1,
            }
        }
    }
    counts
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
        let modes =
            inputforge_core::mode::Modes::new(vec!["Default".to_owned(), "Combat".to_owned()])
                .unwrap();
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
    fn create_only_on_clean_profile_one_mode() {
        let p = one_mode_profile();
        let rows = vec![axis_row(0, Some(x_target()), false)];
        let counts = tally(&p, &rows, &["Default".to_owned()]);
        assert_eq!(
            counts,
            SummaryCounts {
                create: 1,
                replace: 0,
                skip: 0,
                excluded: 0
            }
        );
    }

    #[test]
    fn excluded_when_target_is_none() {
        let p = one_mode_profile();
        let rows = vec![axis_row(0, None, false)];
        let counts = tally(&p, &rows, &["Default".to_owned()]);
        assert_eq!(counts.excluded, 1);
        assert_eq!(counts.create, 0);
    }

    #[test]
    fn skip_when_conflict_and_replace_false() {
        let mut p = one_mode_profile();
        let rows = vec![axis_row(0, Some(x_target()), false)];
        p.set_mapping(&rows[0].input, "Default", None, vec![Action::Invert]);
        let counts = tally(&p, &rows, &["Default".to_owned()]);
        assert_eq!(counts.skip, 1);
    }

    #[test]
    fn replace_when_conflict_and_replace_true() {
        let mut p = one_mode_profile();
        let rows = vec![axis_row(0, Some(x_target()), true)];
        p.set_mapping(&rows[0].input, "Default", None, vec![Action::Invert]);
        let counts = tally(&p, &rows, &["Default".to_owned()]);
        assert_eq!(counts.replace, 1);
    }

    #[test]
    fn fans_out_across_all_modes_when_apply_to_all_modes_is_active() {
        let p = one_mode_profile();
        let rows = vec![axis_row(0, Some(x_target()), false)];
        let counts = tally(&p, &rows, &["Default".to_owned(), "Combat".to_owned()]);
        assert_eq!(counts.create, 2, "one row times two modes = two creates");
    }

    #[test]
    fn excluded_does_not_fan_out_across_modes() {
        // Exclusion is a row-level decision; it counts once even when
        // multiple modes are selected. Locks the documented asymmetry.
        let p = one_mode_profile();
        let rows = vec![axis_row(0, None, false)];
        let counts = tally(&p, &rows, &["Default".to_owned(), "Combat".to_owned()]);
        assert_eq!(
            counts.excluded, 1,
            "excluded counts the row, not row-by-mode"
        );
        assert_eq!(counts.create, 0);
    }
}
