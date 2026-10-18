//! Snapshot settings render prepared controls and forward raw observations.
use super::section::SettingsSection;
use crate::{
    components::{Field, IntegerInput, Switch},
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
    let buffer_error = c.view.read().buffer_error.clone();
    let invalid = buffer_error.is_some();
    let message_id = format!(
        "{MAX_COUNT_ID}__{}",
        if invalid { "error" } else { "helper" }
    );
    rsx! {
        SettingsSection {
            children: rsx! {
                Field {
                    horizontal: true,
                    label: "Snapshot buffer size".to_owned(),
                    helper: "Maximum number of unpinned snapshots kept per profile. \
                             The oldest are auto-evicted. Pinned snapshots are kept regardless.".to_owned(),
                    for_id: MAX_COUNT_ID.to_owned(),
                    helper_id: format!("{}__helper", MAX_COUNT_ID),
                    error: buffer_error,
                    error_id: format!("{MAX_COUNT_ID}__error"),
                        IntegerInput {
                            id: Some(MAX_COUNT_ID.to_owned()),
                            aria_describedby: message_id.clone(),
                            aria_errormessage: invalid.then(|| message_id.clone()),
                            invalid,
                            value: max_count_signal,
                            min: MAX_COUNT_MIN,
                            max: MAX_COUNT_MAX,
                            oncommit: on_max_count_commit,
                            oninvalid: on_max_count_invalid,
                        }
                }

                Field {
                    horizontal: true,
                    label: "Skip startup snapshot if unchanged".to_owned(),
                    helper: "Don't take a snapshot at app start when the active profile is \
                             identical to the most recent snapshot.".to_owned(),
                    for_id: SKIP_UNCHANGED_ID.to_owned(),
                    helper_id: format!("{}__helper", SKIP_UNCHANGED_ID),
                        Switch {
                            id: Some(SKIP_UNCHANGED_ID.to_owned()),
                            aria_describedby: format!("{}__helper", SKIP_UNCHANGED_ID),
                            checked: skip_local,
                            onchange: on_skip_change,
                        }
                }
            },
        }

    }
}
