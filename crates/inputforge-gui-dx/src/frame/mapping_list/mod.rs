//! F8 mapping list (left rail).
//!
//! Composition (inside-out, in dependency order):
//!   - `source_label::format`, `InputAddress` -> "TFM Throttle . Z" formatter
//!   - `group::group_of`     , bucketing by `InputId` kind
//!   - `filter::matches_filter`, name + source-label substring match
//!   - `row::Row`            , single row component
//!   - `rename_inline::RenameInline`, inline rename
//!   - `add_inline::AddInline`, `+ Add mapping` capture state machine
//!   - `empty::EmptyZeroMappings` / `empty::EmptyZeroFilterResults`
//!   - `keyboard::handle_key`, Up/Down/Enter/Cmd-F/Esc
//!   - `MappingList` (this fn), orchestrates everything

#![allow(
    dead_code,
    reason = "Sub-modules expose APIs that orchestrator + Tasks 20-22 consume; \
              clippy's reachability check loses some pub(crate) items here."
)]

mod add_inline;
mod add_inline_rendering;
mod empty;
mod filter;
mod group;
mod rename_inline;
mod row;
pub(crate) use crate::labels as source_label;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod workflow_events_tests;

use dioxus::prelude::*;

use inputforge_core::types::{DeviceId, InputAddress};

use crate::components::sortable::{SortableGap, SortableLiveRegion, use_sortable_state};
use crate::components::{Chip, ChipVariant, InputSize, TextInput};
use crate::context::{AppContext, MappingSummary};
use crate::frame::mapping_list::add_inline::AddInline;
use crate::frame::mapping_list::empty::{EmptyZeroFilterResults, EmptyZeroMappings};
use crate::frame::mapping_list::filter::DeviceChip;
use crate::frame::mapping_list::group::{GroupKind, group_of};
use crate::frame::mapping_list::row::{Row, group_to_u32};
use crate::{
    mapping_edits::Bindings,
    mapping_rail::{Group, Lookup, Menu, Operation, Request, Scope, Target},
};

use crate::frame::view_state::{MappingKey, ViewState};
use crate::patterns::live_capture::LiveCapture;

#[allow(
    dead_code,
    reason = "rsx! macro is opaque to rustc; constant is consumed by Stylesheet { href: MAPPING_LIST_CSS }"
)]
const MAPPING_LIST_CSS: Asset = asset!("/assets/frame/mapping_list.css");

#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant qualifications on event listeners."
)]
pub(crate) fn MappingList() -> Element {
    tracing::trace!(target: "frame::render", region = "mapping_list");
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();

    let bindings = use_context::<Bindings>();
    let feedback = use_signal(|| None::<String>);
    let drag_target: Signal<Option<Menu>> = use_signal(|| None);
    let editing = view.editing_mode;
    let filter_query: Signal<String> = use_signal(String::new);
    let filter_focused: Signal<bool> = use_signal(|| false);
    let renaming: Signal<Option<InputAddress>> = use_signal(|| None);
    let selected_device: Signal<Option<DeviceId>> = use_signal(|| None);
    let force_expand_add: Signal<bool> = use_signal(|| false);
    let menu_open: Signal<Option<(InputAddress, f64, f64)>> = use_signal(|| None);
    let delete_target: Signal<Option<Target>> = use_signal(|| None);
    let pending_duplicate: Signal<Option<Target>> = use_signal(|| None);
    // DnD state -- shared with every Row via the sortable primitive.
    // Holds the in-flight drag source index, source group, drop-target
    // indicator, and the AT live-region content. The primitive's
    // pointer transport forwards source/gap events and owns scroll geometry;
    // this rail retains prepared app targets and callback wiring.
    let sortable = use_sortable_state::<u32>(crate::components::behavior().rail_sortable);

    let publication = crate::mapping_edits::ui::Rail {
        filter: crate::mapping_edits::ui::Filter {
            query: filter_query,
            device: selected_device,
        },
        prepared: use_signal(|| None),
        menu: menu_open,
        rename: renaming,
        duplicate: pending_duplicate,
        delete: delete_target,
        error: feedback,
        live: sortable.live_announcement,
    };
    use_context_provider(|| publication);
    let navigation = bindings.navigation;
    let projection = use_memo(move || {
        navigation.filter.call(crate::navigation::FilterRequest {
            config: ctx.config.read().clone(),
            mode: editing.read().clone(),
            query: filter_query.read().clone(),
            device: selected_device.read().clone(),
        })
    });
    let view_state_memo = use_memo(move || {
        let p = projection.read();
        (p.total, p.rows.clone())
    });
    let device_chips_memo = use_memo(move || projection.read().chips.clone());
    use_effect(move || {
        let p = projection.read();
        let mut device = selected_device;
        if *device.peek() != p.device {
            device.set(p.device.clone());
        }
    });

    let nav_rows_memo = use_memo(move || {
        view_state_memo
            .read()
            .1
            .iter()
            .map(|r| (r.mode.clone(), r.input.clone()))
            .collect::<Vec<MappingKey>>()
    });
    let capture = use_context::<LiveCapture>();
    let keyboard = use_hook(|| {
        bindings
            .navigation
            .rail
            .call(crate::navigation::RailPublication {
                query: filter_query,
                focused: filter_focused,
                device: selected_device,
                rows: nav_rows_memo,
                config: ctx.config,
                meta: ctx.meta,
                capture: capture.active,
                feedback,
                announcement: sortable.live_announcement,
                focus: Callback::new(|selector: String| crate::frame::focus(&selector)),
            })
    });
    use_drop(move || keyboard.close.call(()));
    let navigation_context = format!(
        "{:?}:{}",
        ctx.meta.read().profile_id,
        view.editing_mode.read()
    );
    let (total, rows) = {
        let snapshot = view_state_memo.read();
        (snapshot.0, snapshot.1.clone())
    };
    let query = filter_query.read().clone();
    let filter_active = projection.read().filtered;
    let device_selected = selected_device.read().is_some();

    if total == 0 {
        return rsx! {
            Stylesheet { href: MAPPING_LIST_CSS }
            if let Some(error) = feedback.read().as_ref() { div { role: "alert", "{error}" } }
            div { class: "if-rail", "data-navigation-context":navigation_context.clone(),
                EmptyZeroMappings {}
                div { class: "if-rail__add-sticky",
                    AddInline {
                        force_expanded: force_expand_add,
                        pending_duplicate: pending_duplicate,
                    }
                }
            }
        };
    }

    if filter_active && rows.is_empty() {
        let selected_device_label = selected_device.read().as_ref().and_then(|selected| {
            device_chips_memo
                .read()
                .iter()
                .find(|chip| &chip.id == selected)
                .map(|chip| chip.label.clone())
        });
        return rsx! {
            Stylesheet { href: MAPPING_LIST_CSS }
            if let Some(error) = feedback.read().as_ref() { div { role: "alert", "{error}" } }
            div { class: "if-rail", "data-navigation-context":navigation_context.clone(),
                FilterInput { value: filter_query, focused: filter_focused, events:keyboard }
                DeviceFilterRow {
                    chips: device_chips_memo.read().clone(),
                    selected: selected_device, events:keyboard,
                }
                EmptyZeroFilterResults {
                    query: query.clone(),
                    device_label: selected_device_label,
                    on_clear_text: move |()| {
                        keyboard.query.call(String::new());
                    },
                    on_clear_device: device_selected.then(|| {
                        EventHandler::new(move |()| {
                            keyboard.device.call(None);
                        })
                    }),
                }
                div { class: "if-rail__add-sticky",
                    AddInline {
                        force_expanded: force_expand_add,
                        pending_duplicate: pending_duplicate,
                    }
                }
            }
        };
    }

    let drag_context = format!(
        "profile:{:?};mode:{}",
        ctx.meta.read().profile_id,
        editing.read().as_str()
    );
    let ctx_drag = ctx.clone();
    let on_drag_start = Callback::new(move |summary: MappingSummary| {
        bindings
            .ui
            .drag
            .call(crate::mapping_edits::ui::DragRequest {
                publication: drag_target,
                filter: publication.filter,
                lookup: Lookup {
                    scope: Scope {
                        profile: ctx_drag.meta.peek().profile_id.clone(),
                        mode: summary.mode,
                    },
                    input: summary.input,
                    name: summary.name,
                    filtered: filter_active,
                },
            });
    });
    let group_iter = GroupKind::ordered().into_iter().filter_map(|group| {
        let group_rows: Vec<MappingSummary> = rows
            .iter()
            .filter(|r| group_of(&r.input) == group)
            .cloned()
            .collect();
        if group_rows.is_empty() {
            return None;
        }
        let group_id = group_to_u32(group);
        let group_len = group_rows.len();

        let drop_handler = EventHandler::new(move |gap_index: usize| {
            bindings.ui.rail.call((publication,crate::mapping_edits::ui::RailEvent::Drop {drag:drag_target,group:match group {GroupKind::Axes=>Group::Axes,GroupKind::Buttons=>Group::Buttons,GroupKind::Hats=>Group::Hats},gap:gap_index}));
        });

        // Pre-render the row+gap pairs into a Vec so the rsx! `for` loop
        // walks one stream of children per row instead of needing nested
        // rsx fragments inside the iteration body.
        let row_items: Vec<(usize, MappingSummary, bool)> = group_rows
            .into_iter()
            .enumerate()
            .map(|(i, row)| {
                let is_active = view
                    .selected_mapping
                    .read()
                    .as_ref()
                    .is_some_and(|(m, x)| m == &row.mode && x == &row.input);
                (i, row, is_active)
            })
            .collect();

        Some(rsx! {
            div { class: "if-rail__group", "data-sortable-group": "{sortable.drag_from.id():?}", "data-sortable-context": "{drag_context}",
                div { class: "if-rail__group-header",
                    span { class: "if-rail__group-header__label", {group.header()} }
                    span { class: "if-rail__group-header__count", "{group_len}" }
                }
                SortableGap {
                    key: "gap-{group_id}-0",
                    state: sortable,
                    gap_index: 0_usize,
                    group: group_id,
                    on_drop: drop_handler,
                }
                for (i, row, is_active) in row_items {
                    {
                        rsx! {
                            Row {
                                key: "{row.input:?}-{row.mode}",
                                summary: row.clone(),
                                is_active,
                                renaming,
                                sortable,
                                filter_active,
                                on_drag_start: Some(on_drag_start),
                                on_open_menu: {let lookup=Lookup {scope:Scope {profile:ctx.meta.peek().profile_id.clone(),mode:row.mode.clone()},input:row.input.clone(),name:row.name.clone(),filtered:filter_active}; move |(_input,x,y):(InputAddress,f64,f64)| {
                                    bindings.ui.rail.call((publication,crate::mapping_edits::ui::RailEvent::Open {lookup:lookup.clone(),x,y}));
                                }},
                            }
                            SortableGap {
                                state: sortable,
                                gap_index: i + 1,
                                group: group_id,
                                            on_drop: drop_handler,
                            }
                        }
                    }
                }
            }
        })
    });

    rsx! {
        Stylesheet { href: MAPPING_LIST_CSS }
        if let Some(error) = feedback.read().as_ref() { div { role: "alert", "{error}" } }
        div { class: "if-rail", "data-navigation-context":navigation_context.clone(),
            FilterInput { value: filter_query, focused: filter_focused, events:keyboard }
            DeviceFilterRow {
                chips: device_chips_memo.read().clone(),
                selected: selected_device, events:keyboard,
            }
            div { class: "if-rail__scroll",
                { group_iter }
            }
            div { class: "if-rail__add-sticky",
                AddInline {
                    force_expanded: force_expand_add,
                    pending_duplicate: pending_duplicate,
                }
            }
            ContextMenuMount {filtered:projection.read().filtered}
            DeleteDialogMount { delete_target: delete_target }
            // sr-only live region. Mounted once at the rail root so AT
            // users hear every reorder action (drag-drop, context menu
            // Move up/down, keyboard Alt+Arrow). The text is overwritten
            // at each dispatch site through app-supplied announcements.
            SortableLiveRegion { state: sortable }
        }
    }
}

#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant qualifications on event listeners."
)]
fn DeviceFilterRow(
    chips: Vec<DeviceChip>,
    selected: Signal<Option<DeviceId>>,
    events: crate::navigation::Listener,
) -> Element {
    if chips.is_empty() {
        return rsx! {};
    }
    rsx! {
        div {
            class: "if-rail__device-filter",
            role: "group",
            "aria-label": "Filter mappings by device",
            for chip in chips {
                {
                    let active = selected.read().as_ref() == Some(&chip.id);
                    let id = chip.id.clone();
                    let label = chip.label.clone();
                    rsx! {
                        button {
                            r#type: "button",
                            class: "if-rail__device-chip",
                            "aria-pressed": if active { "true" } else { "false" },
                            title: "{label}",
                            onclick: move |_| {
                                events.device.call(Some(id.clone()));
                            },
                            Chip {
                                variant: ChipVariant::Outline,
                                "{label}"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant qualifications on event listeners."
)]
fn FilterInput(
    value: Signal<String>,
    focused: Signal<bool>,
    events: crate::navigation::Listener,
) -> Element {
    let mut focused = focused;
    rsx! {
        div {
            class: "if-rail__filter",
            onfocusin: move |_| focused.set(true),
            onfocusout: move |_| focused.set(false),
            TextInput {
                value: ReadSignal::from(value),
                size: InputSize::Sm,
                placeholder: "Filter mappings\u{2026}".to_owned(),
                oninput: move |evt: FormEvent| {
                    events.query.call(evt.value());
                },
            }
        }
    }
}

// Stub mount, actual content arrives in Task 21.
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant qualifications on event listeners."
)]
fn ContextMenuMount(filtered: bool) -> Element {
    let publication = use_context::<crate::mapping_edits::ui::Rail>();
    let Some((_target_input, anchor_x, anchor_y)) = publication.menu.read().clone() else {
        return rsx! {};
    };
    let Some(menu) = publication.prepared.read().clone() else {
        return rsx! {};
    };
    let bindings = use_context::<Bindings>();
    let other_modes = menu.other_modes.clone();
    let dup_to_mode_disabled = other_modes.is_empty();
    let move_up_disabled = !menu.can_up;
    let move_down_disabled = !menu.can_down;
    let move_disabled_reason = if filtered {
        "Clear filter to reorder."
    } else {
        ""
    };

    let close = move |_| {
        bindings
            .ui
            .rail
            .call((publication, crate::mapping_edits::ui::RailEvent::Close));
    };
    let target_for_rename = menu.target.clone();
    let target_for_dup = menu.target.clone();
    let target_for_delete = menu.target.clone();
    let target_for_dup_to = menu.target.clone();
    let menu_up = menu.clone();
    let menu_down = menu;
    rsx! {
        div {class:"if-row-menu-backdrop",onclick:close}
        div {class:"if-row-menu",role:"menu",style:"position: fixed; left: {anchor_x}px; top: {anchor_y}px;",
            button {r#type:"button",role:"menuitem",class:"if-row-menu__item",onclick:move |_| {bindings.ui.rail.call((publication,crate::mapping_edits::ui::RailEvent::Rename(target_for_rename.clone())));},"Rename"}
            button {r#type:"button",role:"menuitem",class:"if-row-menu__item",onclick:move |_| {bindings.ui.rail.call((publication,crate::mapping_edits::ui::RailEvent::Duplicate(target_for_dup.clone())));},"Duplicate"}
            div {class:"if-row-menu__item if-row-menu__item--submenu-host","aria-disabled":"{dup_to_mode_disabled}","Duplicate to mode…"
                if !dup_to_mode_disabled {
                    div {class:"if-row-menu__submenu",role:"menu",
                        for target_mode in other_modes.iter().cloned() {
                            {let target=target_for_dup_to.clone();let mode=target_mode.clone();rsx!{
                                button {key:"{target_mode}",r#type:"button",role:"menuitem",class:"if-row-menu__item",onclick:move |_| {bindings.ui.rail.call((publication,crate::mapping_edits::ui::RailEvent::MenuDispatch(Request {target:target.clone(),operation:Operation::Duplicate(mode.clone())})));},"{target_mode}"}
                            }}
                        }
                    }
                }
            }
            button {r#type:"button",role:"menuitem",class:"if-row-menu__item","aria-disabled":"{move_up_disabled}",title:if move_up_disabled {move_disabled_reason}else{""},
                onclick:move |_| {bindings.ui.rail.call((publication,crate::mapping_edits::ui::RailEvent::Step {menu:menu_up.clone(),down:false}));},"Move up"}
            button {r#type:"button",role:"menuitem",class:"if-row-menu__item","aria-disabled":"{move_down_disabled}",title:if move_down_disabled {move_disabled_reason}else{""},
                onclick:move |_| {bindings.ui.rail.call((publication,crate::mapping_edits::ui::RailEvent::Step {menu:menu_down.clone(),down:true}));},"Move down"}
            button {r#type:"button",role:"menuitem",class:"if-row-menu__item if-row-menu__item--danger",onclick:move |_| {bindings.ui.rail.call((publication,crate::mapping_edits::ui::RailEvent::BeginDelete(target_for_delete.clone())));},"Delete"}
        }
    }
}

#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant qualifications on event listeners."
)]
pub(crate) fn DeleteDialogMount(delete_target: Signal<Option<Target>>) -> Element {
    let mut dialog_open: Signal<bool> = use_signal(|| false);
    use_effect(move || {
        let want = delete_target.read().is_some();
        if *dialog_open.peek() != want {
            dialog_open.set(want);
        }
    });

    let display_name = delete_target
        .read()
        .as_ref()
        .and_then(|t| t.before.name.clone())
        .unwrap_or_else(|| "(unnamed)".to_owned());
    let bindings = use_context::<Bindings>();
    let feedback = use_signal(|| None::<String>);

    rsx! {
        crate::components::DialogRoot {
            open: dialog_open,
            onclose: move |()| bindings.ui.delete.call(crate::mapping_edits::ui::DeleteRequest {target:delete_target,error:feedback,confirm:false}),
            crate::components::DialogTitle { "Delete mapping" }
            crate::components::DialogBody {
                "Delete '{display_name}'?"
                if let Some(error) = feedback.read().as_ref() { div { role: "alert", "{error}" } }
            }
            crate::components::DialogFooter {
                crate::components::Button {
                    variant: crate::components::ButtonVariant::Ghost,
                    onmounted: move |evt: MountedEvent| {
                        spawn(async move {
                            let _ = evt.data().set_focus(true).await;
                        });
                    },
                    onclick:move |_| {bindings.ui.delete.call(crate::mapping_edits::ui::DeleteRequest {target:delete_target,error:feedback,confirm:false});},
                    "Cancel"
                }
                crate::components::Button {
                    variant: crate::components::ButtonVariant::Danger,
                    onclick:move |_| {bindings.ui.delete.call(crate::mapping_edits::ui::DeleteRequest {target:delete_target,error:feedback,confirm:true});},
                    "Delete"
                }
            }
        }
    }
}

// `DuplicateWatcher` was removed: the Duplicate flow now reuses the
// AddInline pad shell via the `pending_duplicate` prop. AddInline
// observes the rising edge, pre-fills the name with `<source.name>
// (copy)`, stashes the source for actions-resolution at commit, and
// goes through the normal Pad{Capturing} -> Pad{Captured} -> Add
// dispatch flow. The user can edit the name and press the refresh
// icon to recapture, exactly like a fresh add.

#[cfg(test)]
mod rename_tests;

fn prepare_menu(ctx: &AppContext, summary: &MappingSummary, filtered: bool) -> Option<Menu> {
    consume_context::<Bindings>().menu.call(Lookup {
        scope: Scope {
            profile: ctx.meta.peek().profile_id.clone(),
            mode: summary.mode.clone(),
        },
        input: summary.input.clone(),
        name: summary.name.clone(),
        filtered,
    })
}
fn run_rail(
    bindings: &Bindings,
    request: Request,
    feedback: Signal<Option<String>>,
    live: Signal<String>,
) -> bool {
    let publication = consume_context::<crate::mapping_edits::ui::Rail>();
    bindings.ui.rail.call((
        crate::mapping_edits::ui::Rail {
            error: feedback,
            live,
            ..publication
        },
        crate::mapping_edits::ui::RailEvent::Dispatch(request),
    ))
}
