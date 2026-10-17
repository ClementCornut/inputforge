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
    mapping_rail::{Group, Lookup, Menu, Move, Operation, Outcome, Request, Scope, Target},
};

/// Format the AT live-region phrase for a successful reorder. Shared
/// between the keyboard handler, the context menu items, and the row's
/// drop handler so all three reorder paths produce identical
/// announcements (per the F8-impeccable critique pass).
pub(crate) fn format_reorder_announcement(
    new_subpos: usize,
    group_len: usize,
    group: GroupKind,
) -> String {
    let phrase = match group {
        GroupKind::Axes => "axes",
        GroupKind::Buttons => "buttons",
        GroupKind::Hats => "hats",
    };
    format!(
        "Mapping moved to position {} of {} in {}.",
        new_subpos + 1,
        group_len,
        phrase
    )
}
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
    let sortable = use_sortable_state::<u32>();

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
    let query_empty = query.trim().is_empty();
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

    if (!query_empty || device_selected) && rows.is_empty() {
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

    let live_writer_for_drop = sortable.live_announcement;
    let filter_active = !query.trim().is_empty() || device_selected;
    let drag_context = format!(
        "profile:{:?};mode:{}",
        ctx.meta.read().profile_id,
        editing.read().as_str()
    );
    let ctx_drag = ctx.clone();
    let on_drag_start = Callback::new(move |summary: MappingSummary| {
        let mut drag_target = drag_target;
        let filtered = !filter_query.peek().trim().is_empty() || selected_device.peek().is_some();
        drag_target.set(prepare_menu(&ctx_drag, &summary, filtered));
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
            let mut drag_target = drag_target;
            let source = drag_target.peek().clone();
            drag_target.set(None);
            let Some(menu) = source else {
                return;
            };
            let filtered =
                !filter_query.peek().trim().is_empty() || selected_device.peek().is_some();
            let target_group = match group {
                GroupKind::Axes => Group::Axes,
                GroupKind::Buttons => Group::Buttons,
                GroupKind::Hats => Group::Hats,
            };
            run_rail(
                &bindings,
                Request {
                    target: menu.target,
                    operation: Operation::Reorder {
                        movement: Move::Gap(gap_index),
                        order: menu.order,
                        group: target_group,
                        filtered,
                    },
                },
                feedback,
                live_writer_for_drop,
            );
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

        // Bind the validator with an explicit type so Dioxus's prop
        // SuperInto accepts it as `Option<fn(&u32, &u32) -> bool>` (the
        // unique closure type wouldn't coerce through the macro's
        // generated trait bound).
        let rail_validator: Option<fn(&u32, &u32) -> bool> = Some(|s, t| s == t);
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
                    validate_drop: rail_validator,
                    on_drop: drop_handler,
                }
                for (i, row, is_active) in row_items {
                    {
                        let mut menu_setter = menu_open;
                        rsx! {
                            Row {
                                key: "{row.input:?}-{row.mode}",
                                summary: row.clone(),
                                is_active,
                                renaming,
                                sortable,
                                filter_active,
                                on_drag_start: Some(on_drag_start),
                                on_open_menu: move |(input, x, y): (InputAddress, f64, f64)| {
                                    menu_setter.set(Some((input, x, y)));
                                },
                            }
                            SortableGap {
                                state: sortable,
                                gap_index: i + 1,
                                group: group_id,
                                validate_drop: rail_validator,
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
            ContextMenuMount {
                menu_open: menu_open,
                renaming: renaming,
                delete_target: delete_target,
                pending_duplicate: pending_duplicate,
                filter_query: filter_query,
                selected_device,
                feedback,
                live_announcement: sortable.live_announcement,
            }
            DeleteDialogMount { delete_target: delete_target }
            // sr-only live region. Mounted once at the rail root so AT
            // users hear every reorder action (drag-drop, context menu
            // Move up/down, keyboard Alt+Arrow). The text is overwritten
            // at each dispatch site via `format_reorder_announcement`.
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
fn ContextMenuMount(
    menu_open: Signal<Option<(InputAddress, f64, f64)>>,
    renaming: Signal<Option<InputAddress>>,
    delete_target: Signal<Option<Target>>,
    pending_duplicate: Signal<Option<Target>>,
    filter_query: Signal<String>,
    selected_device: Signal<Option<DeviceId>>,
    feedback: Signal<Option<String>>,
    live_announcement: Signal<String>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();

    let Some((target_input, anchor_x, anchor_y)) = menu_open.read().clone() else {
        return rsx! {};
    };
    let mode_now = view.editing_mode.read().clone();
    let cfg = ctx.config.read();
    let target = cfg
        .mappings
        .iter()
        .find(|m| m.input == target_input && m.mode == mode_now)
        .cloned();
    drop(cfg);
    let Some(target) = target else {
        let mut menu_open = menu_open;
        menu_open.set(None);
        return rsx! {};
    };

    let filtered = !filter_query.read().trim().is_empty() || selected_device.read().is_some();
    let _ = ctx.meta.read();
    let Some(menu) = prepare_menu(&ctx, &target, filtered) else {
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

    let mut menu_open_writer = menu_open;
    let close = move |_| menu_open_writer.set(None);

    let target_for_rename = target.input.clone();
    let target_for_dup = menu.target.clone();
    let target_for_dup_to = menu.target.clone();
    let target_for_delete = menu.target.clone();
    let menu_up = menu.clone();
    let menu_down = menu.clone();

    rsx! {
        div { class: "if-row-menu-backdrop", onclick: close }
        div {
            class: "if-row-menu",
            role: "menu",
            style: "position: fixed; left: {anchor_x}px; top: {anchor_y}px;",
            button {
                r#type: "button",
                role: "menuitem",
                class: "if-row-menu__item",
                onclick: move |_| {
                    let mut renaming = renaming;
                    renaming.set(Some(target_for_rename.clone()));
                    let mut menu_open = menu_open;
                    menu_open.set(None);
                },
                "Rename"
            }
            button {
                r#type: "button",
                role: "menuitem",
                class: "if-row-menu__item",
                onclick: move |_| {
                    // Just set pending_duplicate; AddInline observes the
                    // rising edge, pre-fills the name with `<source.name>
                    // (copy)`, stashes the source for actions-resolution
                    // at commit, and arms LiveCapture itself.
                    let mut pd = pending_duplicate;
                    pd.set(Some(target_for_dup.clone()));
                    tracing::info!(
                        target: "f8::mapping_list",
                        action = "duplicate_arm",
                        ?target_for_dup.before.input,
                        mode = %target_for_dup.before.mode,
                        "duplicate flow armed; awaiting fresh capture",
                    );
                    let mut menu_open = menu_open;
                    menu_open.set(None);
                },
                "Duplicate"
            }
            div {
                class: "if-row-menu__item if-row-menu__item--submenu-host",
                "aria-disabled": "{dup_to_mode_disabled}",
                "Duplicate to mode\u{2026}"
                if !dup_to_mode_disabled {
                    div {
                        class: "if-row-menu__submenu",
                        role: "menu",
                        for target_mode in other_modes.iter().cloned() {
                            {
                                let target_mode_clone = target_mode.clone();
                                let target_for_each = target_for_dup_to.clone();
                                let mut menu_open_each = menu_open;
                                rsx! {
                                    button {
                                        key: "{target_mode}",
                                        r#type: "button",
                                        role: "menuitem",
                                        class: "if-row-menu__item",
                                        onclick: move |_| {
                                            if run_rail(&bindings, Request { target: target_for_each.clone(), operation: Operation::Duplicate(target_mode_clone.clone()) }, feedback, live_announcement) {
                                                menu_open_each.set(None);
                                            }
                                        },
                                        "{target_mode}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            button {
                r#type: "button",
                role: "menuitem",
                class: "if-row-menu__item",
                "aria-disabled": "{move_up_disabled}",
                title: if move_up_disabled { move_disabled_reason } else { "" },
                onclick: move |_| {
                    if move_up_disabled {
                        return;
                    }
                    if run_rail(&bindings, Request { target: menu_up.target.clone(), operation: Operation::Reorder { movement: Move::Up, order: menu_up.order.clone(), group: menu_up.group, filtered: !filter_query.peek().trim().is_empty() || selected_device.peek().is_some() } }, feedback, live_announcement) {
                        let mut menu_open = menu_open;
                        menu_open.set(None);
                    }
                },
                "Move up"
            }
            button {
                r#type: "button",
                role: "menuitem",
                class: "if-row-menu__item",
                "aria-disabled": "{move_down_disabled}",
                title: if move_down_disabled { move_disabled_reason } else { "" },
                onclick: move |_| {
                    if move_down_disabled {
                        return;
                    }
                    if run_rail(&bindings, Request { target: menu_down.target.clone(), operation: Operation::Reorder { movement: Move::Down, order: menu_down.order.clone(), group: menu_down.group, filtered: !filter_query.peek().trim().is_empty() || selected_device.peek().is_some() } }, feedback, live_announcement) {
                        let mut menu_open = menu_open;
                        menu_open.set(None);
                    }
                },
                "Move down"
            }
            button {
                r#type: "button",
                role: "menuitem",
                class: "if-row-menu__item if-row-menu__item--danger",
                onclick: move |_| {
                    let mut delete_target = delete_target;
                    delete_target.set(Some(target_for_delete.clone()));
                    let mut menu_open = menu_open;
                    menu_open.set(None);
                },
                "Delete"
            }
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
    let target_clone = delete_target.read().clone();
    let bindings = use_context::<Bindings>();
    let mut feedback = use_signal(|| None::<String>);

    rsx! {
        crate::components::DialogRoot {
            open: dialog_open,
            onclose: move |()| {
                let mut dt = delete_target;
                dt.set(None);
                true
            },
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
                    onclick: move |_| {
                        let mut dt = delete_target;
                        dt.set(None);
                    },
                    "Cancel"
                }
                crate::components::Button {
                    variant: crate::components::ButtonVariant::Danger,
                    onclick: move |_| {
                        if let Some(target) = &target_clone {
                            match bindings.rail.call(Request { target: target.clone(), operation: Operation::Delete }) {
                                Outcome::Delivered => { let mut dt = delete_target; dt.set(None); feedback.set(None); },
                                Outcome::Rejected(error) => feedback.set(Some(error)),
                                _ => {},
                            }
                        }
                    },
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
    mut feedback: Signal<Option<String>>,
    mut live: Signal<String>,
) -> bool {
    match bindings.rail.call(request) {
        Outcome::Moved { index, len, group } => {
            let group = match group {
                Group::Axes => GroupKind::Axes,
                Group::Buttons => GroupKind::Buttons,
                Group::Hats => GroupKind::Hats,
            };
            live.set(format_reorder_announcement(index, len, group));
            feedback.set(None);
            true
        }
        Outcome::Delivered | Outcome::Opened => {
            feedback.set(None);
            true
        }
        Outcome::Rejected(error) => {
            feedback.set(Some(error));
            false
        }
        Outcome::NoOp => false,
    }
}
