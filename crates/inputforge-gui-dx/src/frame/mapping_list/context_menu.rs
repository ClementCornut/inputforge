//! Shared cursor menu with visible destinations and acknowledged closure.
use super::row::row_dom_id;
use crate::{
    components::{AnchoredMenu, MenuAnchor, MenuGroup, MenuItem},
    mapping_edits::{Bindings, ui::RailEvent},
    mapping_rail::{Operation, Request},
};
use dioxus::prelude::*;
#[component]
pub(super) fn ContextMenuMount(filtered: bool) -> Element {
    let publication = use_context::<crate::mapping_edits::ui::Rail>();
    let Some((_, x, y)) = publication.menu.read().clone() else {
        return rsx! {};
    };
    let Some(menu) = publication.prepared.read().clone() else {
        return rsx! {};
    };
    let bindings = use_context::<Bindings>();
    let target = menu.target.clone();
    let return_focus = row_dom_id(&target.scope.mode, &target.before.input);
    let rename = target.clone();
    let duplicate = target.clone();
    let delete = target.clone();
    let up = menu.clone();
    let down = menu.clone();
    let up_reason = if filtered {
        "Clear filter to reorder."
    } else {
        "Already first in this group."
    };
    let down_reason = if filtered {
        "Clear filter to reorder."
    } else {
        "Already last in this group."
    };
    rsx! {
        AnchoredMenu {
            open: Some(MenuAnchor { x, y }),
            return_focus,
            on_close: move |_| {
                bindings.ui.rail.call((publication, RailEvent::Close))
            },
            MenuItem {
                compact: true,
                onactivate: move |_| bindings.ui.rail.call((publication, RailEvent::Rename(rename.clone()))),
                "Rename"
            }
            MenuItem {
                compact: true,
                onactivate: move |_| {
                    bindings.ui.rail.call((publication, RailEvent::Duplicate(duplicate.clone())))
                },
                "Duplicate"
            }
            MenuGroup {
                label: "Duplicate to mode",
                if menu.other_modes.is_empty() {
                    MenuItem {
                        compact: true,
                        disabled: true,
                        title: "No other modes are available.",
                        "No other modes"
                    }
                }
                for mode in menu.other_modes {
                    {
                        let destination = mode.clone();
                        let target = target.clone();
                        rsx! {
                            MenuItem {
                                key: "{mode}",
                                compact: true,
                                onactivate: move |_| {
                                    bindings
                                        .ui
                                        .rail
                                        .call((
                                            publication,
                                            RailEvent::MenuDispatch(Request {
                                                target: target.clone(),
                                                operation: Operation::Duplicate(destination.clone()),
                                            }),
                                        ))
                                },
                                "Duplicate to mode {mode}"
                            }
                        }
                    }
                }
            }
            MenuItem {
                compact: true,
                disabled: filtered || !menu.can_up,
                title: (filtered || !menu.can_up).then_some(up_reason.to_owned()),
                onactivate: move |_| {
                    bindings
                        .ui
                        .rail
                        .call((
                            publication,
                            RailEvent::Step {
                                menu: up.clone(),
                                down: false,
                            },
                        ))
                },
                "Move up"
            }
            MenuItem {
                compact: true,
                disabled: filtered || !menu.can_down,
                title: (filtered || !menu.can_down).then_some(down_reason.to_owned()),
                onactivate: move |_| {
                    bindings
                        .ui
                        .rail
                        .call((
                            publication,
                            RailEvent::Step {
                                menu: down.clone(),
                                down: true,
                            },
                        ))
                },
                "Move down"
            }
            MenuItem {
                compact: true,
                danger: true,
                onactivate: move |_| {
                    bindings.ui.rail.call((publication, RailEvent::BeginDelete(delete.clone())))
                },
                "Delete"
            }
        }
    }
}
