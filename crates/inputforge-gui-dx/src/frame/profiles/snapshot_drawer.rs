#![cfg_attr(
    not(test),
    expect(dead_code, reason = "wired into keyboard listener in a later task")
)]

use dioxus::prelude::*;

use crate::components::{
    Badge, BadgeVariant, BottomDrawer, Button, ButtonSize, ButtonVariant, IconButton,
};
use crate::context::AppContext;
use crate::context::SnapshotRowView;
use crate::frame::profiles::actions::{
    create_manual_snapshot_action, snapshot_delete_action, snapshot_restore_action,
};
use crate::icons::Icon as IconKind;

#[component]
pub(crate) fn SnapshotDrawer(
    active_profile_name: String,
    rows: Vec<SnapshotRowView>,
    open: bool,
) -> Element {
    let ctx = use_context::<AppContext>();
    let mut drawer_open = use_signal(|| open);
    let count = rows.len();
    let commands = ctx.commands.clone();
    let snapshot_now = move |_| {
        let _ = commands.send(create_manual_snapshot_action());
    };
    rsx! {
        section { class: "snapshot-drawer",
            BottomDrawer {
                open: *drawer_open.read(),
                title: format!("Snapshots · {active_profile_name}"),
                count,
                on_toggle: move |_| {
                    let next = !*drawer_open.read();
                    drawer_open.set(next);
                },
                actions: rsx! {
                    IconButton {
                        icon: IconKind::Plus,
                        label: "Snapshot now",
                        variant: ButtonVariant::Primary,
                        size: ButtonSize::Sm,
                        onclick: snapshot_now,
                    }
                },
                for row in rows {
                    {
                        let commands = ctx.commands.clone();
                        let restore_id = row.id;
                        let restore_click = move |_| {
                            let _ = commands.send(snapshot_restore_action(restore_id).command);
                        };
                        let commands = ctx.commands.clone();
                        let delete_id = row.id;
                        let delete_click = move |_| {
                            let _ = commands.send(snapshot_delete_action(delete_id).command);
                        };
                        rsx! {
                            article { class: "snapshot-row", "data-snapshot-id": "{row.id}",
                                div { class: "snapshot-row__main",
                                    div { class: "snapshot-row__title",
                                        Badge { variant: BadgeVariant::Info, "{row.kind_label}" }
                                        if row.pinned { Badge { variant: BadgeVariant::Success, "Pinned" } }
                                        if let Some(label) = &row.label { strong { "{label}" } }
                                    }
                                    span { class: "snapshot-row__time", "{row.time_label}" }
                                }
                                div { class: "snapshot-row__actions",
                                    Button { variant: ButtonVariant::Primary, size: ButtonSize::Sm, onclick: restore_click, "Restore" }
                                    Button { variant: ButtonVariant::Danger, size: ButtonSize::Sm, onclick: delete_click, "Delete" }
                                }
                            }
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
