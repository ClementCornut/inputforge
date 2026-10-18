use crate::{
    components::{
        Badge, BadgeVariant, Button, ButtonPresentation, ButtonSize, ButtonVariant, Cluster,
        Drawer, DrawerAnchor, DrawerVariant, Icon, IconButton, ReadoutCaption,
    },
    icons::Icon as IconKind,
    profiles::{Context, Intent as ProfileIntent, snapshots::Intent},
};
use dioxus::prelude::*;

#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures (the macro suggests \
              shorthand-with-no-prop-name as a fix, which would erase the intent). \
              This is a macro-level artifact, not authored qualifications."
)]
pub(crate) fn SnapshotDrawer() -> Element {
    let c = use_context::<Context>().capture();
    let view = c.view.read().snapshots.clone();
    let active_profile_name = view
        .profile
        .as_ref()
        .map(|p| p.name.clone())
        .unwrap_or_default();
    let profile = view.profile.clone();
    let rows = view.rows;
    let count = rows.len();
    let is_empty = rows.is_empty();
    let is_open = view.open;
    let create = profile.clone();
    let snapshot_now = move |_| {
        if let Some(profile) = create.clone() {
            let _ = c.send(ProfileIntent::Snapshot(Intent::Create(profile)));
        }
    };
    let toggle = move |_| {
        if let Some(profile) = profile.clone() {
            let _ = c.send(ProfileIntent::Snapshot(Intent::Toggle(profile)));
        }
    };
    rsx! {
        section { class: "snapshot-drawer",
            // Always-visible bar: toggle on the left, actions on the right.
            // Sibling of the Drawer so it stays put when the body collapses.
            div { class: "snapshot-drawer__bar",
                Button {
                    variant: ButtonVariant::Ghost, presentation: ButtonPresentation::Disclosure,
                    class: "snapshot-drawer__toggle",
                    id: "snapshot-drawer-bar-title",
                    aria_expanded: is_open,
                    aria_controls: "snapshot-drawer-body",
                    onclick: toggle,
                    span { class: "snapshot-drawer__chevron",
                        Icon { name: IconKind::ChevronUp }
                    }
                    span { class: "snapshot-drawer__title", "Snapshots · {active_profile_name}" }
                    Badge { variant: BadgeVariant::Info, count:true, "{count}" }
                }
                Cluster { class: "snapshot-drawer__actions", inline:true, nowrap:true, gap:"--space-1",
                    IconButton {
                        icon: IconKind::Plus,
                        label: "Snapshot now",
                        variant: ButtonVariant::Primary,
                        size: ButtonSize::Sm,
                        onclick: snapshot_now,
                    }
                }
            }
            Drawer {
                anchor: DrawerAnchor::Bottom,
                variant: DrawerVariant::Persistent,
                open: is_open,
                aria_labelledby: "snapshot-drawer-bar-title".to_owned(),
                class: "snapshot-drawer__drawer".to_owned(),
                div { id: "snapshot-drawer-body", class: "snapshot-drawer__body",
                    if is_empty {
                        div { class: "snapshot-drawer__empty",
                            ReadoutCaption { "No snapshots yet" }
                            span { class: "snapshot-drawer__empty-hint",
                                "Press Ctrl+S or use the + button to capture the current profile state."
                            }
                        }
                    }
                    for row in rows { super::snapshot_row::SnapshotRow { key: "{row.row.id}", row } }

                }
            }
        }
    }
}
