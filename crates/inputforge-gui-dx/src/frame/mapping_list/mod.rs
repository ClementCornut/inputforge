//! F8 mapping list (left rail). See
//! `docs/superpowers/specs/2026-04-30-f8-mapping-list-design.md` for the
//! design rationale.
//!
//! Composition (inside-out, in dependency order):
//!   - `source_label::format` — `InputAddress` -> "TFM Throttle . Z" formatter
//!   - `group::group_of`      — bucketing by `InputId` kind
//!   - `filter::matches_filter` — name + source-label substring match
//!   - `row::Row`             — single row component
//!   - `rename_inline::RenameInline` — inline rename
//!   - `add_inline::AddInline` — `+ Add mapping` capture state machine
//!   - `empty::EmptyZeroMappings` / `empty::EmptyZeroFilterResults`
//!   - `keyboard::handle_key` — Up/Down/Enter/Cmd-F/Esc
//!   - `MappingList` (this fn) — orchestrates everything

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
mod source_label;

#[cfg(test)]
mod tests;

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;
use inputforge_core::types::InputAddress;

use crate::components::{InputSize, TextInput};
use crate::context::{AppContext, MappingSummary};
use crate::frame::mapping_list::add_inline::AddInline;
use crate::frame::mapping_list::empty::{EmptyZeroFilterResults, EmptyZeroMappings};
use crate::frame::mapping_list::filter::matches_filter;
use crate::frame::mapping_list::group::{GroupKind, group_of};
use crate::frame::mapping_list::keyboard::{Intent, Key, State, handle_key};
use crate::frame::mapping_list::row::Row;
use crate::frame::view_state::ViewState;
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
    let force_expand_add: Signal<bool> = use_signal(|| false);
    let menu_open: Signal<Option<(InputAddress, f64, f64)>> = use_signal(|| None);
    let delete_target: Signal<Option<MappingSummary>> = use_signal(|| None);
    let pending_duplicate: Signal<Option<MappingSummary>> = use_signal(|| None);

    // Single memo computes filtered, grouped rows AND total in-mode count.
    let view_state_memo = use_memo(move || {
        let cfg = ctx.config.read();
        let mode_now = editing.read().clone();
        let query = filter_query.read().clone();
        let mut total: usize = 0;
        let mut filtered: Vec<MappingSummary> = Vec::new();
        for m in cfg.mappings.iter().filter(|m| m.mode == mode_now) {
            total += 1;
            if matches_filter(m, &query, &cfg) {
                filtered.push(m.clone());
            }
        }
        (total, filtered)
    });

    // Document-scoped keyboard listener — mirrors Task 8's
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
            .collect::<Vec<(String, InputAddress)>>()
    });

    let cap_for_kb = use_context::<LiveCapture>();
    let mut filter_query_writer = filter_query;
    let mut sel_writer = view.selected_mapping;

    use_effect(move || {
        let mut mounted = kb_listener_mounted;
        if *mounted.peek() {
            return; // already mounted — no re-install on render.
        }
        mounted.set(true);
        let mut sd = kb_shutdown_signal;
        sd.set(false);

        spawn(async move {
            let mut handle = document::eval(
                "const h = (ev) => {\n\
                   const meta = ev.metaKey ? 1 : 0;\n\
                   const ctrl = ev.ctrlKey ? 1 : 0;\n\
                   dioxus.send([ev.key, meta, ctrl]);\n\
                 };\n\
                 window.addEventListener('keydown', h, true);\n\
                 (async () => {\n\
                   while (true) {\n\
                     const msg = await dioxus.recv();\n\
                     if (msg === '__shutdown__') {\n\
                       window.removeEventListener('keydown', h, true);\n\
                       dioxus.send(['__ack__', 0, 0]);\n\
                       return;\n\
                     }\n\
                   }\n\
                 })();\n\
                 ",
            );

            loop {
                if *kb_shutdown_signal.peek() {
                    let _ = handle.send("__shutdown__".to_owned());
                    let _ = handle.recv::<(String, u8, u8)>().await;
                    break;
                }
                let Ok((key_str, meta, ctrl)) = handle.recv::<(String, u8, u8)>().await else {
                    break;
                };
                // Coordinate with Phase C — if capture is armed, defer to it.
                if *cap_for_kb.active.read() {
                    continue;
                }
                let key = match key_str.as_str() {
                    "ArrowUp" => Key::ArrowUp,
                    "ArrowDown" => Key::ArrowDown,
                    "Enter" => Key::Enter,
                    "Escape" => Key::Escape,
                    "f" | "F" if meta == 1 || ctrl == 1 => Key::FilterShortcut,
                    _ => continue,
                };
                let nav_rows = nav_rows_memo.read().clone();
                let visible_pairs: Vec<&(String, InputAddress)> = nav_rows.iter().collect();
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
                AddInline {
                    force_expanded: force_expand_add,
                    pending_duplicate: pending_duplicate,
                }
            }
        };
    }

    if !query_empty && rows.is_empty() {
        return rsx! {
            Stylesheet { href: MAPPING_LIST_CSS }
            div { class: "if-rail",
                FilterInput { value: filter_query, focused: filter_focused }
                EmptyZeroFilterResults {
                    query: query.clone(),
                    on_clear: move |()| {
                        let mut q = filter_query;
                        q.set(String::new());
                    }
                }
                AddInline {
                    force_expanded: force_expand_add,
                    pending_duplicate: pending_duplicate,
                }
            }
        };
    }

    let group_iter = GroupKind::ordered().into_iter().filter_map(|group| {
        let group_rows: Vec<MappingSummary> = rows
            .iter()
            .filter(|r| group_of(&r.input) == group)
            .cloned()
            .collect();
        if group_rows.is_empty() {
            return None;
        }
        Some(rsx! {
            div { class: "if-rail__group",
                div { class: "if-rail__group-header", {group.header()} }
                for row in group_rows {
                    {
                        let is_active = view
                            .selected_mapping
                            .read()
                            .as_ref()
                            .is_some_and(|(m, i)| m == &row.mode && i == &row.input);
                        let mut menu_setter = menu_open;
                        rsx! {
                            Row {
                                key: "{row.input:?}-{row.mode}",
                                summary: row.clone(),
                                is_active: is_active,
                                renaming: renaming,
                                on_open_menu: move |(input, x, y): (InputAddress, f64, f64)| {
                                    menu_setter.set(Some((input, x, y)));
                                },
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
            { group_iter }
            AddInline {
                force_expanded: force_expand_add,
                pending_duplicate: pending_duplicate,
            }
            ContextMenuMount {
                menu_open: menu_open,
                renaming: renaming,
                delete_target: delete_target,
                pending_duplicate: pending_duplicate,
            }
            DeleteDialogMount { delete_target: delete_target }
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

// Stub mount — actual content arrives in Task 21.
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

    let modes_all = ctx.meta.read().modes.clone();
    let other_modes: Vec<String> = modes_all
        .iter()
        .filter(|m| **m != mode_now)
        .cloned()
        .collect();
    let dup_to_mode_disabled = modes_all.len() <= 1;

    let mut menu_open_writer = menu_open;
    let close = move |_| menu_open_writer.set(None);

    let target_for_rename = target.input.clone();
    let target_for_dup = target.clone();
    let target_for_dup_to = target.clone();
    let target_for_delete = target.clone();
    let cmd_for_dup_to = ctx.commands.clone();

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
