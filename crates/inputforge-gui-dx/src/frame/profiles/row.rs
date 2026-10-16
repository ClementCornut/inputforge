//! Rendering and callback wiring for one prepared profile row.
use crate::{
    components::{
        Badge, BadgeVariant, Icon, InputSize, MenuItem, MenuItems, MenuRoot, MenuTrigger, TextInput,
    },
    icons::Icon as IconKind,
    profiles::{Context, Intent, Row},
    snapshots::ProfileRowOrigin,
};
use dioxus::prelude::*;
#[component]
pub(super) fn ProfileRow(row: Row) -> Element {
    let c = use_context::<Context>().capture();
    let v = c.view.read().clone();
    let r = &row.row;
    let t = row.target;
    let renaming = v.rename.as_ref().is_some_and(|draft| draft.path == t.path);
    let name = use_memo(move || c.view.read().name.clone());
    let mode_label = if r.mode_count == 1 {
        "1 mode".to_owned()
    } else {
        format!("{} modes", r.mode_count)
    };
    let open = t.clone();
    let add = t.clone();
    let reveal = t.clone();
    let duplicate = t.clone();
    let delete = t.clone();
    let snapshot = t.clone();
    let rename = t;
    rsx! {
        article {
            class: if r.is_active { "profile-row profile-row--active" } else { "profile-row" },
            "data-profile-id": "{r.id}",
            div {
                class: "profile-row__main",
                div {
                    class: "profile-row__title",
                    strong {
                        "{r.name}"
                    }
                    if r.is_active {
                        Badge {
                            variant: BadgeVariant::Success,
                            "Active"
                        }
                    }
                    if r.origin == ProfileRowOrigin::External {
                        Badge {
                            variant: BadgeVariant::Info,
                            "External"
                        }
                    }
                }
                div {
                    class: "profile-row__meta",
                    span {
                        class: "profile-row__mode-count",
                        "{mode_label}"
                    }
                    if let Some(last) = &r.last_edited_label {
                        span {
                            class: "profile-row__sep",
                            "·"
                        }
                        span {
                            class: "profile-row__last-edited",
                            "{last}"
                        }
                    }
                }
                if renaming {
                    div {
                        class: "profile-row__rename",
                        TextInput {
                            value: name,
                            size: InputSize::Sm,
                            placeholder: "Profile name".to_owned(),
                            oninput: move |e: FormEvent| {
                                let _ = c.send(Intent::Name(e.value()));
                            },
                            onkeydown: move |e| c.bindings.rename_key.call((c.owner, c.workflow, e)),
                            onblur: move |_| {
                                let _ = c.send(Intent::Rename);
                            },
                            onmounted: move |e| c.bindings.focus.call((c.owner, c.workflow, e)),
                        }
                    }
                    if let Some(error) = v.name_error {
                        div { class: "profile-row__rename-error", role: "alert", "{error}" }
                    }
                }
            }
            MenuRoot {
                class: "profile-row__menu".to_owned(),
                MenuTrigger {
                    class: "profile-row__menu-trigger".to_owned(),
                    Icon {
                        name: IconKind::DotsVertical,
                    }
                    span {
                        class: "profile-row__menu-label",
                        "Profile actions for {r.name}"
                    }
                }
                MenuItems {
                    if r.can_open && !r.is_active {
                        MenuItem {
                            onactivate: move |_| c.send(Intent::Open(open.clone())),
                            "Open"
                        }
                    }
                    if r.can_add_to_library {
                        MenuItem {
                            onactivate: move |_| c.send(Intent::AddRow(add.clone())),
                            "Add to library"
                        }
                    }
                    if r.can_snapshot_now {
                        MenuItem {
                            onactivate: move |_| c.send(Intent::Snapshot(crate::profiles::snapshots::Intent::Create(snapshot.clone()))),
                            "Snapshot now"
                        }
                    }
                    if r.can_rename {
                        MenuItem {
                            onactivate: move |_| c.send(Intent::BeginRename(rename.clone())),
                            "Rename"
                        }
                    }
                    if r.can_duplicate {
                        MenuItem {
                            onactivate: move |_| c.send(Intent::Duplicate(duplicate.clone())),
                            "Duplicate"
                        }
                    }
                    if r.can_reveal {
                        MenuItem {
                            onactivate: move |_| c.send(Intent::Reveal(reveal.clone())),
                            "Reveal"
                        }
                    }
                    if r.can_delete {
                        MenuItem {
                            onactivate: move |_| c.send(Intent::BeginDelete(delete.clone())),
                            class: "profile-row__danger-item".to_owned(),
                            "Delete"
                        }
                    }
                }
            }
        }
    }
}
