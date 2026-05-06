#![cfg_attr(
    not(test),
    expect(dead_code, reason = "wired into keyboard listener in a later task")
)]

use dioxus::prelude::*;

use crate::context::SnapshotRowView;

#[component]
pub(crate) fn SnapshotDrawer(
    active_profile_name: String,
    rows: Vec<SnapshotRowView>,
    open: bool,
) -> Element {
    let count = rows.len();
    rsx! {
        section { class: "snapshot-drawer",
            div { class: "snapshot-drawer__header",
                button {
                    class: "snapshot-drawer__toggle",
                    "aria-expanded": "{open}",
                    span { class: "snapshot-drawer__chevron", if open { "v" } else { ">" } }
                    span { "Snapshots - {active_profile_name}" }
                    span { class: "badge", "{count}" }
                }
                button {
                    class: "icon-button",
                    "aria-label": "Snapshot now",
                    title: "Snapshot now",
                    "+"
                }
            }
            if open {
                div { class: "snapshot-drawer__ledger",
                    for row in rows {
                        article { class: "snapshot-row", "data-snapshot-id": "{row.id}",
                            span { class: "snapshot-row__kind", "{row.kind_label}" }
                            span { class: "snapshot-row__time", "{row.time_label}" }
                            if let Some(label) = &row.label { strong { "{label}" } }
                            if row.pinned { span { class: "badge", "Pinned" } }
                            button { class: "button button--primary", "Restore" }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FocusScope {
    Panel,
    TextInput,
    InlineRename,
    Menu,
    Dialog,
    OsPickerReturn,
}

pub(crate) fn should_handle_snapshot_shortcut(scope: FocusScope) -> bool {
    matches!(scope, FocusScope::Panel)
}
