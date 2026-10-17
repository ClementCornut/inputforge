//! Snapshot settings render prepared controls and forward raw observations.
use super::{field_row::SettingsFieldRow, section::SettingsSection};
use crate::{
    components::{IntegerInput, Switch},
    settings::Intent,
};
use dioxus::prelude::*;
// Existing snapshot input affordances; core validates the same inclusive range.
const MAX_COUNT_MIN: usize = 1;
const MAX_COUNT_MAX: usize = 100;
const MAX_COUNT_ID: &str = "if-settings-snapshot-max-count";
const SKIP_UNCHANGED_ID: &str = "if-settings-snapshot-skip-unchanged";
#[component]
pub(crate) fn SnapshotsSection() -> Element {
    let c = super::context();
    let max_count_signal = use_memo(move || c.view.read().values.snapshot.max_count);
    let skip_local = use_memo(move || c.view.read().values.snapshot.skip_if_unchanged);
    let on_max_count_commit = move |v: usize| {
        let _ = c.send(Intent::Max(v.to_string()));
    };
    let on_max_count_invalid = move |error| {
        let _ = c.send(Intent::Invalid(error));
    };
    let on_skip_change = move |_| {
        let _ = c.send(Intent::Skip);
    };
    rsx! {
        SettingsSection {
            children: rsx! {
                SettingsFieldRow {
                    label: "Snapshot buffer size".to_owned(),
                    helper: "Maximum number of unpinned snapshots kept per profile. \
                             The oldest are auto-evicted. Pinned snapshots are kept regardless.".to_owned(),
                    control_id: MAX_COUNT_ID.to_owned(),
                    error: c.view.read().buffer_error.clone(),
                    control: rsx! {
                        IntegerInput {
                            id: Some(MAX_COUNT_ID.to_owned()),
                            value: max_count_signal,
                            min: MAX_COUNT_MIN,
                            max: MAX_COUNT_MAX,
                            class: "if-integer-input--inset".to_owned(),
                            oncommit: on_max_count_commit,
                            oninvalid: on_max_count_invalid,
                        }
                    },
                }

                SettingsFieldRow {
                    label: "Skip startup snapshot if unchanged".to_owned(),
                    helper: "Don't take a snapshot at app start when the active profile is \
                             identical to the most recent snapshot.".to_owned(),
                    control_id: SKIP_UNCHANGED_ID.to_owned(),
                    control: rsx! {
                        Switch {
                            id: Some(SKIP_UNCHANGED_ID.to_owned()),
                            checked: skip_local,
                            onchange: on_skip_change,
                        }
                    },
                }
            },
        }

    }
}
