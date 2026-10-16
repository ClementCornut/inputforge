use crate::{
    components::{
        Badge, BadgeVariant, ButtonSize, ButtonVariant, Drawer, DrawerAnchor, DrawerVariant, Icon,
        IconButton,
    },
    icons::Icon as IconKind,
    profiles::{Context, Intent as ProfileIntent, snapshots::Intent},
};
use dioxus::prelude::*;
use inputforge_core::snapshot::SnapshotKind;

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
                button {
                    class: "if-button if-button--ghost if-button--md snapshot-drawer__toggle",
                    id: "snapshot-drawer-bar-title",
                    "aria-expanded": if is_open { "true" } else { "false" },
                    "aria-controls": "snapshot-drawer-body",
                    onclick: toggle,
                    span { class: "snapshot-drawer__chevron",
                        Icon { name: IconKind::ChevronUp }
                    }
                    span { class: "snapshot-drawer__title", "Snapshots · {active_profile_name}" }
                    Badge { variant: BadgeVariant::Info, "{count}" }
                }
                div { class: "snapshot-drawer__actions",
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
                            span { class: "snapshot-drawer__empty-eyebrow", "No snapshots yet" }
                            span { class: "snapshot-drawer__empty-hint",
                                "Press Ctrl+S or use the + button to capture the current profile state."
                            }
                        }
                    }
                    for row in rows {
                        {
                            let restore = row.target.clone();
                            let delete = row.target.clone();
                            let can_delete = row.can_delete;
                            let row = row.row;
                            let restore_click = move |_| { let _ = c.send(ProfileIntent::Snapshot(Intent::Restore(restore.clone()))); };
                            let delete_click = move |_| { let _ = c.send(ProfileIntent::Snapshot(Intent::Delete(delete.clone()))); };
                            // Map the kind to a single-glyph cue at the row's
                            // leading edge. The icon replaces the prior kind
                            // badge so the strong-weight label can be the row's
                            // anchor without a competing colored chip.
                            let icon_name = match row.kind {
                                SnapshotKind::AutoSessionStart => IconKind::Play,
                                SnapshotKind::AutoBeforeRestore => IconKind::Refresh,
                                SnapshotKind::AutoBeforeBulkMap => IconKind::Link,
                                SnapshotKind::Manual => IconKind::Save,
                            };
                            // The strong-weight label: prefer the engine /
                            // user supplied label, fall back to the kind name
                            // for autos that do not carry a custom label
                            // (Session start, Before restore).
                            let primary_label = row
                                .label
                                .clone()
                                .unwrap_or_else(|| row.kind_label.clone());
                            // Two-line row layout: the 320px-wide drawer
                            // does not have horizontal real estate for icon
                            // + label + time + 2 action buttons in a single
                            // flex line (the label gets squeezed to ~85px
                            // and ellipses out almost everything). Stacking
                            // label-on-line-1, time + actions-on-line-2 lets
                            // the label take ~280px, restoring the
                            // glanceable read.
                            rsx! {
                                article { class: "snapshot-row", "data-snapshot-id": "{row.id}",
                                    span { class: "snapshot-row__kind-icon", "aria-hidden": "true",
                                        Icon { name: icon_name }
                                    }
                                    div { class: "snapshot-row__main",
                                        // Line 1: label + Pinned badge.
                                        // `title` carries the full label text
                                        // so the ellipsis-truncated form
                                        // surfaces a native tooltip on hover.
                                        div { class: "snapshot-row__primary",
                                            strong {
                                                class: "snapshot-row__label",
                                                title: primary_label.clone(),
                                                "{primary_label}"
                                            }
                                            if row.pinned { Badge { variant: BadgeVariant::Success, "Pinned" } }
                                        }
                                        // Line 2: muted time on the left,
                                        // Restore + Delete icons on the
                                        // right. Restore uses
                                        // `clock-counter-clockwise` (Phosphor
                                        // restore-from-history glyph), stays
                                        // always-visible. Delete is the
                                        // ghost trash icon revealed on
                                        // row-hover or focus.
                                        div { class: "snapshot-row__secondary",
                                            time {
                                                class: "snapshot-row__time",
                                                "datetime": row.time_absolute.clone(),
                                                title: row.time_absolute.clone(),
                                                "{row.time_relative}"
                                            }
                                            div { class: "snapshot-row__actions",
                                                IconButton {
                                                    icon: IconKind::ClockCounterClockwise,
                                                    label: "Restore snapshot",
                                                    variant: ButtonVariant::Ghost,
                                                    size: ButtonSize::Sm,
                                                    class: "snapshot-row__restore".to_owned(),
                                                    onclick: restore_click,
                                                }
                                                IconButton {
                                                    icon: IconKind::Trash,
                                                    label: "Delete snapshot",
                                                    variant: ButtonVariant::Ghost,
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
                    }
                }
            }
        }
    }
}
