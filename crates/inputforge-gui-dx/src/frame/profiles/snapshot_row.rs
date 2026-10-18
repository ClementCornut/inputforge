//! Snapshot row rendering and app-owned restore/delete callbacks.
use crate::{
    components::{Badge, BadgeVariant, ButtonSize, ButtonVariant, Cluster, Icon, IconButton},
    icons::Icon as IconKind,
    profiles::{
        Context, Intent as ProfileIntent,
        snapshots::{Intent, Row},
    },
};
use dioxus::prelude::*;
use inputforge_core::snapshot::SnapshotKind;
#[component]
pub(super) fn SnapshotRow(row: Row) -> Element {
    let c = use_context::<Context>().capture();
    let restore = row.target.clone();
    let delete = row.target.clone();
    let can_delete = row.can_delete;
    let row = row.row;
    let restore_click = move |_| {
        let _ = c.send(ProfileIntent::Snapshot(Intent::Restore(restore.clone())));
    };
    let delete_click = move |_| {
        let _ = c.send(ProfileIntent::Snapshot(Intent::Delete(delete.clone())));
    };
    let icon_name = match row.kind {
        SnapshotKind::AutoSessionStart => IconKind::Play,
        SnapshotKind::AutoBeforeRestore => IconKind::Refresh,
        SnapshotKind::AutoBeforeBulkMap => IconKind::Link,
        SnapshotKind::Manual => IconKind::Save,
    };
    let primary_label = row.label.clone().unwrap_or_else(|| row.kind_label.clone());
    rsx! {
        article { class: "snapshot-row", "data-snapshot-id": "{row.id}",
            span { class: "snapshot-row__kind-icon", "aria-hidden": "true",
                Icon { name: icon_name }
            }
            div { class: "snapshot-row__main",
                div { class: "snapshot-row__primary",
                    strong {
                        class: "snapshot-row__label",
                        title: primary_label.clone(),
                        "{primary_label}"
                    }
                    if row.pinned { Badge { variant: BadgeVariant::Success, "Pinned" } }
                }
                div { class: "snapshot-row__secondary",
                    time {
                        class: "snapshot-row__time",
                        "datetime": row.time_absolute.clone(),
                        title: row.time_absolute.clone(),
                        "{row.time_relative}"
                    }
                    Cluster { class: "snapshot-row__actions", inline:true, nowrap:true, gap:"--space-1",
                        IconButton {
                            icon: IconKind::ClockCounterClockwise,
                            label: "Restore snapshot",
                            variant: ButtonVariant::GhostPrimary,
                            size: ButtonSize::Sm,
                            class: "snapshot-row__restore".to_owned(),
                            onclick: restore_click,
                        }
                        IconButton {
                            icon: IconKind::Trash,
                            label: "Delete snapshot",
                            variant: ButtonVariant::GhostDanger,
                            size: ButtonSize::Sm,
                            class: "snapshot-row__delete".to_owned(),
                            disabled: !can_delete,
                            onclick: delete_click,
                        }
                    }
                }
            }
        }
    }
}
