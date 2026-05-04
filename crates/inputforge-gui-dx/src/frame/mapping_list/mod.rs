//! F8 mapping list (left rail). See
//! `docs/superpowers/specs/2026-04-30-f8-mapping-list-design.md` for the
//! design rationale.
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
mod empty;
mod filter;
mod group;
mod keyboard;
mod rename_inline;
mod row;
pub(crate) mod source_label;

#[cfg(test)]
mod tests;

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;
use inputforge_core::types::{DeviceId, InputAddress};

use crate::components::sortable::{SortableGap, SortableLiveRegion, use_sortable_state};
use crate::components::{InputSize, TextInput};
use crate::context::{AppContext, MappingSummary};
use crate::frame::mapping_list::add_inline::AddInline;
use crate::frame::mapping_list::empty::{EmptyZeroFilterResults, EmptyZeroMappings};
use crate::frame::mapping_list::filter::{
    DeviceChip, device_chips_for_mode, matches_device_filter, matches_filter,
};
use crate::frame::mapping_list::group::{GroupKind, group_of};
use crate::frame::mapping_list::keyboard::{Intent, Key, ReorderDir, State, handle_key};
use crate::frame::mapping_list::row::{Row, group_to_u32};

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

    let editing = view.editing_mode;
    let filter_query: Signal<String> = use_signal(String::new);
    let filter_focused: Signal<bool> = use_signal(|| false);
    let renaming: Signal<Option<InputAddress>> = use_signal(|| None);
    let selected_device: Signal<Option<DeviceId>> = use_signal(|| None);
    let force_expand_add: Signal<bool> = use_signal(|| false);
    let menu_open: Signal<Option<(InputAddress, f64, f64)>> = use_signal(|| None);
    let delete_target: Signal<Option<MappingSummary>> = use_signal(|| None);
    let pending_duplicate: Signal<Option<MappingSummary>> = use_signal(|| None);
    // DnD state -- shared with every Row via the sortable primitive.
    // Holds the in-flight drag source index, source group, drop-target
    // indicator, and the AT live-region content. The primitive's
    // `SortableHandle.ondragstart` calls `data_transfer().set_data(...)`
    // natively (no JS bootstrap), so this rail no longer mounts a
    // document-level dragstart/dragover listener.
    let sortable = use_sortable_state::<u32>();

    // Single memo computes filtered, grouped rows AND total in-mode count.
    let view_state_memo = use_memo(move || {
        let cfg = ctx.config.read();
        let mode_now = editing.read().clone();
        let query = filter_query.read().clone();
        let selected = selected_device.read().clone();
        let mut total: usize = 0;
        let mut filtered: Vec<MappingSummary> = Vec::new();
        for m in cfg.mappings.iter().filter(|m| m.mode == mode_now) {
            total += 1;
            if matches_filter(m, &query, &cfg) && matches_device_filter(m, selected.as_ref()) {
                filtered.push(m.clone());
            }
        }
        (total, filtered)
    });

    let device_chips_memo = use_memo(move || {
        let cfg = ctx.config.read();
        let mode_now = editing.read().clone();
        device_chips_for_mode(&cfg.mappings, &mode_now, &cfg)
    });

    use_effect(move || {
        let chips = device_chips_memo.read();
        let selected = selected_device.read().clone();
        if let Some(device) = selected {
            if !chips.iter().any(|chip| chip.id == device) {
                let mut selected_device = selected_device;
                selected_device.set(None);
            }
        }
    });

    // Document-scoped keyboard listener, mirrors Task 8's
    // `document::eval` + `window.addEventListener` pattern. Keydown
    // events are routed through the pure `keyboard::handle_key`
    // dispatcher; the resulting `Intent` is translated into signal
    // writes / focus calls. When `LiveCapture.active == true` the
    // listener early-returns so Phase C's Esc listener wins.
    let kb_listener_mounted: Signal<bool> = use_signal(|| false);
    let kb_shutdown_signal: Signal<bool> = use_signal(|| false);

    // Stable `(mode, input)` projection of the visible filtered rows.
    // Recomputed when `view_state_memo` changes; consumed inside the
    // listener loop to drive Up/Down navigation through `handle_key`.
    let nav_rows_memo = use_memo(move || {
        let snapshot = view_state_memo.read();
        snapshot
            .1
            .iter()
            .map(|r| (r.mode.clone(), r.input.clone()))
            .collect::<Vec<MappingKey>>()
    });

    let cap_for_kb = use_context::<LiveCapture>();
    let mut filter_query_writer = filter_query;
    let mut sel_writer = view.selected_mapping;
    // Clone context handles needed inside the spawned listener so the
    // FnMut effect closure can re-fire without moving `ctx`.
    let cmd_for_kb = ctx.commands.clone();
    let ctx_for_kb = ctx.clone();

    use_effect(move || {
        let mut mounted = kb_listener_mounted;
        if *mounted.peek() {
            return; // already mounted, no re-install on render.
        }
        mounted.set(true);
        let mut sd = kb_shutdown_signal;
        sd.set(false);
        let cmd_kb = cmd_for_kb.clone();
        let ctx_kb = ctx_for_kb.clone();

        spawn(async move {
            // The keydown listener is installed in CAPTURE phase at window
            // level (third arg `true`), so it fires before any element-level
            // handler regardless of focus. That is intentional for the rail
            // shortcuts (Up/Down to nav rows, Cmd+F to focus filter, Esc to
            // clear filter, Enter to focus the editor) but conflicts with
            // any open menu surface, whose own onkeydown also navigates with
            // arrow keys. The early return on `[role="menu"]` defers to the
            // menu when one is mounted (covers MenuItems, AnchoredMenu, and
            // the legacy `.if-row-menu` in this file); the menu's own
            // element-level onkeydown is unaffected because we do not call
            // stopPropagation, we just opt this listener out.
            let mut handle = document::eval(
                "const h = (ev) => {\n\
                   if (document.querySelector('[role=\"menu\"]')) return;\n\
                   const meta = ev.metaKey ? 1 : 0;\n\
                   const ctrl = ev.ctrlKey ? 1 : 0;\n\
                   const alt  = ev.altKey  ? 1 : 0;\n\
                   dioxus.send([ev.key, meta, ctrl, alt]);\n\
                 };\n\
                 window.addEventListener('keydown', h, true);\n\
                 (async () => {\n\
                   while (true) {\n\
                     const msg = await dioxus.recv();\n\
                     if (msg === '__shutdown__') {\n\
                       window.removeEventListener('keydown', h, true);\n\
                       dioxus.send(['__ack__', 0, 0, 0]);\n\
                       return;\n\
                     }\n\
                   }\n\
                 })();\n\
                 ",
            );

            loop {
                if *kb_shutdown_signal.peek() {
                    let _ = handle.send("__shutdown__".to_owned());
                    let _ = handle.recv::<(String, u8, u8, u8)>().await;
                    break;
                }
                let Ok((key_str, meta, ctrl, alt)) = handle.recv::<(String, u8, u8, u8)>().await
                else {
                    break;
                };
                // Coordinate with Phase C, if capture is armed, defer to it.
                if *cap_for_kb.active.read() {
                    continue;
                }
                let key = match key_str.as_str() {
                    "ArrowUp" if alt == 1 => Key::AltArrowUp,
                    "ArrowDown" if alt == 1 => Key::AltArrowDown,
                    "ArrowUp" => Key::ArrowUp,
                    "ArrowDown" => Key::ArrowDown,
                    "Enter" => Key::Enter,
                    "Escape" => Key::Escape,
                    "f" | "F" if meta == 1 || ctrl == 1 => Key::FilterShortcut,
                    _ => continue,
                };
                let nav_rows = nav_rows_memo.read().clone();
                let visible_pairs: Vec<&MappingKey> = nav_rows.iter().collect();
                let sel_snapshot = sel_writer.peek().clone();
                let sel_view: Option<(&str, &InputAddress)> =
                    sel_snapshot.as_ref().map(|(m, i)| (m.as_str(), i));
                let state = State {
                    visible_rows: &visible_pairs,
                    selected: sel_view,
                    capture_armed: *cap_for_kb.active.read(),
                    filter_focused: *filter_focused.read(),
                    filter_query_empty: filter_query_writer.peek().trim().is_empty(),
                };
                match handle_key(key, state) {
                    Intent::Select((m, i)) => sel_writer.set(Some((m, i))),
                    Intent::FocusEditor => {
                        spawn(async move {
                            let mut h2 = document::eval(
                                "var el = document.querySelector('[data-editor-focus]'); \
                                 if (el) el.focus(); dioxus.send(true);",
                            );
                            let _ = h2.recv::<bool>().await;
                        });
                    }
                    Intent::FocusFilter => {
                        spawn(async move {
                            let mut h2 = document::eval(
                                "var el = document.querySelector('.if-rail__filter input'); \
                                 if (el) el.focus(); dioxus.send(true);",
                            );
                            let _ = h2.recv::<bool>().await;
                        });
                    }
                    Intent::ClearFilter => filter_query_writer.set(String::new()),
                    Intent::ReorderSelected { mode, input, dir } => {
                        // Read the active profile to compute group + subpos.
                        // Boundary / single-element-group / unknown-mapping
                        // checks happen here; the engine then re-validates
                        // and silent-no-ops if any of them slip through.
                        let cfg = ctx_kb.config.read();
                        let group_inputs: Vec<&InputAddress> = cfg
                            .mappings
                            .iter()
                            .filter(|m| m.mode == mode && group_of(&m.input) == group_of(&input))
                            .map(|m| &m.input)
                            .collect();
                        let group_len = group_inputs.len();
                        let cur = group_inputs.iter().position(|i| **i == input).unwrap_or(0);
                        drop(cfg);
                        if group_len < 2 {
                            continue;
                        }
                        let target = match dir {
                            ReorderDir::Up => {
                                if cur == 0 {
                                    continue;
                                }
                                cur - 1
                            }
                            ReorderDir::Down => {
                                if cur + 1 >= group_len {
                                    continue;
                                }
                                cur + 1
                            }
                        };
                        let _ = cmd_kb.send(EngineCommand::ReorderMapping {
                            input: input.clone(),
                            mode: mode.clone(),
                            target_index_in_group: target,
                        });
                        let mut live = sortable.live_announcement;
                        live.set(format_reorder_announcement(
                            target,
                            group_len,
                            group_of(&input),
                        ));
                        tracing::info!(
                            target: "f8::mapping_list",
                            action = "reorder_keyboard",
                            ?input,
                            mode = %mode,
                            from = cur,
                            to = target,
                            "dispatch ReorderMapping",
                        );
                    }
                    Intent::NoOp => {}
                }
            }

            let mut mounted = kb_listener_mounted;
            mounted.set(false);
        });
    });

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
            div { class: "if-rail",
                EmptyZeroMappings {
                    on_start_capture: move |()| {
                        let mut force = force_expand_add;
                        force.set(true);
                    }
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

    if (!query_empty || device_selected) && rows.is_empty() {
        return rsx! {
            Stylesheet { href: MAPPING_LIST_CSS }
            div { class: "if-rail",
                FilterInput { value: filter_query, focused: filter_focused }
                DeviceFilterRow {
                    chips: device_chips_memo.read().clone(),
                    selected: selected_device,
                }
                EmptyZeroFilterResults {
                    query: query.clone(),
                    on_clear: move |()| {
                        let mut q = filter_query;
                        q.set(String::new());
                    }
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

    // Capture the per-render values that the gap drop handler needs.
    // Cloned/copied once here so the per-group filter_map closure can
    // move owned copies into each handler without borrowing back into
    // the render scope.
    let cmd_for_drop = ctx.commands.clone();
    let config_for_drop = ctx.config;
    let drag_from_for_drop = sortable.drag_from;
    let live_writer_for_drop = sortable.live_announcement;
    let mode_for_drop = editing.read().clone();
    let filter_active = !query.trim().is_empty() || device_selected;

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

        // One drop handler per group, threaded into every gap in that
        // group. The handler reads `drag_from` to resolve the source's
        // group-local subpos at drop time, converts the gap_index (pre-
        // remove slot) to the engine's post-remove `target_index_in_
        // group`, dispatches `ReorderMapping`, and writes the AT live
        // announcement.
        let cmd_tx = cmd_for_drop.clone();
        let mode_handler = mode_for_drop.clone();
        let mut live_writer = live_writer_for_drop;
        let drop_handler = EventHandler::new(move |gap_index: usize| {
            let Some(src_subpos) = *drag_from_for_drop.peek() else {
                return;
            };
            // Convert pre-remove gap index to post-remove insertion slot.
            let to = if src_subpos < gap_index {
                gap_index - 1
            } else {
                gap_index
            };
            let src_input = {
                let cfg = config_for_drop.read();
                cfg.mappings
                    .iter()
                    .filter(|m| m.mode == mode_handler && group_of(&m.input) == group)
                    .nth(src_subpos)
                    .map(|m| m.input.clone())
            };
            let Some(src_input) = src_input else {
                return;
            };
            tracing::info!(
                target: "f8::mapping_list",
                action = "reorder_drop",
                source = ?src_input,
                mode = %mode_handler,
                gap_index,
                to,
                "dispatch ReorderMapping",
            );
            let _ = cmd_tx.send(EngineCommand::ReorderMapping {
                input: src_input,
                mode: mode_handler.clone(),
                target_index_in_group: to,
            });
            // The engine clamps `target_index_in_group` to group_len-1;
            // mirror that here so the announcement reflects the actual
            // landed position rather than the user's pre-clamp intent.
            let landed_subpos = to.min(group_len.saturating_sub(1));
            live_writer.set(format_reorder_announcement(landed_subpos, group_len, group));
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
            div { class: "if-rail__group",
                div { class: "if-rail__group-header", {group.header()} }
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
        div { class: "if-rail",
            FilterInput { value: filter_query, focused: filter_focused }
            DeviceFilterRow {
                chips: device_chips_memo.read().clone(),
                selected: selected_device,
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
fn DeviceFilterRow(chips: Vec<DeviceChip>, selected: Signal<Option<DeviceId>>) -> Element {
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
                            class: if active { "if-rail__device-chip is-active" } else { "if-rail__device-chip" },
                            r#type: "button",
                            "aria-pressed": if active { "true" } else { "false" },
                            title: "{label}",
                            onclick: move |_| {
                                let mut selected = selected;
                                if selected.peek().as_ref() == Some(&id) {
                                    selected.set(None);
                                } else {
                                    selected.set(Some(id.clone()));
                                }
                            },
                            "{label}"
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
fn FilterInput(value: Signal<String>, focused: Signal<bool>) -> Element {
    let mut value = value;
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
                    value.set(evt.value());
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
    delete_target: Signal<Option<MappingSummary>>,
    pending_duplicate: Signal<Option<MappingSummary>>,
    filter_query: Signal<String>,
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
    // Compute the target's group-local position and group length while we
    // still hold the cfg read lock. group_of bucketing matches what the
    // engine's reorder helper uses; consistent on both sides of the
    // command channel.
    let target_group = target.as_ref().map(|t| group_of(&t.input));
    let (current_subpos, group_len) = match (&target, target_group) {
        (Some(t), Some(g)) => {
            let group_inputs: Vec<&InputAddress> = cfg
                .mappings
                .iter()
                .filter(|m| m.mode == mode_now && group_of(&m.input) == g)
                .map(|m| &m.input)
                .collect();
            let len = group_inputs.len();
            let pos = group_inputs
                .iter()
                .position(|i| **i == t.input)
                .unwrap_or(0);
            (pos, len)
        }
        _ => (0usize, 0usize),
    };
    drop(cfg);
    let Some(target) = target else {
        let mut menu_open = menu_open;
        menu_open.set(None);
        return rsx! {};
    };

    let modes_all = ctx.meta.read().modes.clone();
    let other_modes: Vec<String> = modes_all
        .iter()
        .filter(|m| **m != mode_now)
        .cloned()
        .collect();
    let dup_to_mode_disabled = modes_all.len() <= 1;

    // Filter-active gate: reorder is disabled while the filter input is
    // non-empty (per spec). Same idiom as the F8 audit's
    // disabled-with-title pattern from tools_cluster/mod.rs.
    let filter_active = !filter_query.read().trim().is_empty();
    let move_up_disabled = filter_active || group_len < 2 || current_subpos == 0;
    let move_down_disabled = filter_active || group_len < 2 || current_subpos + 1 >= group_len;
    let move_disabled_reason = if filter_active {
        "Clear filter to reorder."
    } else {
        ""
    };

    let mut menu_open_writer = menu_open;
    let close = move |_| menu_open_writer.set(None);

    let target_for_rename = target.input.clone();
    let target_for_dup = target.clone();
    let target_for_dup_to = target.clone();
    let target_for_delete = target.clone();
    let target_for_move_up = target.clone();
    let target_for_move_down = target.clone();
    let cmd_for_dup_to = ctx.commands.clone();
    let cmd_for_move_up = ctx.commands.clone();
    let cmd_for_move_down = ctx.commands.clone();

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
                        ?target_for_dup.input,
                        mode = %target_for_dup.mode,
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
                                let cmd_for_each = cmd_for_dup_to.clone();
                                let ctx_for_each = ctx.clone();
                                let mut menu_open_each = menu_open;
                                rsx! {
                                    button {
                                        key: "{target_mode}",
                                        r#type: "button",
                                        role: "menuitem",
                                        class: "if-row-menu__item",
                                        onclick: move |_| {
                                            let cfg = ctx_for_each.config.read();
                                            let collision = cfg.mappings.iter().any(|m| {
                                                m.input == target_for_each.input
                                                    && m.mode == target_mode_clone
                                            });
                                            drop(cfg);
                                            if collision {
                                                let mut em = view.editing_mode;
                                                em.set(target_mode_clone.clone());
                                                let mut sel = view.selected_mapping;
                                                sel.set(Some((
                                                    target_mode_clone.clone(),
                                                    target_for_each.input.clone(),
                                                )));
                                            } else {
                                                let actions = ctx_for_each
                                                    .state
                                                    .read()
                                                    .active_profile
                                                    .as_ref()
                                                    .and_then(|p| {
                                                        p.find_mapping(
                                                            &target_for_each.input,
                                                            &target_for_each.mode,
                                                        )
                                                        .map(|m| m.actions.clone())
                                                    })
                                                    .unwrap_or_default();
                                                let _ = cmd_for_each.send(
                                                    EngineCommand::SetMapping {
                                                        input: target_for_each.input.clone(),
                                                        mode: target_mode_clone.clone(),
                                                        name: target_for_each.name.clone(),
                                                        actions,
                                                    },
                                                );
                                                tracing::info!(
                                                    target: "f8::mapping_list",
                                                    action = "duplicate_to_mode",
                                                    ?target_for_each.input,
                                                    mode = %target_mode_clone,
                                                    "dispatch SetMapping (duplicate_to_mode)",
                                                );
                                            }
                                            menu_open_each.set(None);
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
                    let new_subpos = current_subpos.saturating_sub(1);
                    let _ = cmd_for_move_up.send(EngineCommand::ReorderMapping {
                        input: target_for_move_up.input.clone(),
                        mode: target_for_move_up.mode.clone(),
                        target_index_in_group: new_subpos,
                    });
                    let mut live = live_announcement;
                    if let Some(g) = target_group {
                        live.set(format_reorder_announcement(new_subpos, group_len, g));
                    }
                    tracing::info!(
                        target: "f8::mapping_list",
                        action = "reorder_move_up",
                        ?target_for_move_up.input,
                        mode = %target_for_move_up.mode,
                        from = current_subpos,
                        to = new_subpos,
                        "dispatch ReorderMapping",
                    );
                    let mut menu_open = menu_open;
                    menu_open.set(None);
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
                    let new_subpos = current_subpos + 1;
                    let _ = cmd_for_move_down.send(EngineCommand::ReorderMapping {
                        input: target_for_move_down.input.clone(),
                        mode: target_for_move_down.mode.clone(),
                        target_index_in_group: new_subpos,
                    });
                    let mut live = live_announcement;
                    if let Some(g) = target_group {
                        live.set(format_reorder_announcement(new_subpos, group_len, g));
                    }
                    tracing::info!(
                        target: "f8::mapping_list",
                        action = "reorder_move_down",
                        ?target_for_move_down.input,
                        mode = %target_for_move_down.mode,
                        from = current_subpos,
                        to = new_subpos,
                        "dispatch ReorderMapping",
                    );
                    let mut menu_open = menu_open;
                    menu_open.set(None);
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
pub(crate) fn DeleteDialogMount(delete_target: Signal<Option<MappingSummary>>) -> Element {
    let ctx = use_context::<AppContext>();

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
        .and_then(|t| t.name.clone())
        .unwrap_or_else(|| "(unnamed)".to_owned());
    let target_clone = delete_target.read().clone();
    let cmd_for_delete = ctx.commands.clone();

    rsx! {
        crate::components::DialogRoot {
            open: dialog_open,
            onclose: move |()| {
                let mut dt = delete_target;
                dt.set(None);
            },
            crate::components::DialogTitle { "Delete mapping" }
            crate::components::DialogBody {
                "Delete '{display_name}'? Undo available this session only."
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
                            let _ = cmd_for_delete.send(EngineCommand::RemoveMapping {
                                input: target.input.clone(),
                                mode: target.mode.clone(),
                            });
                            tracing::info!(
                                target: "f8::mapping_list",
                                action = "remove",
                                ?target.input,
                                mode = %target.mode,
                                "dispatch RemoveMapping",
                            );
                        }
                        let mut dt = delete_target;
                        dt.set(None);
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
