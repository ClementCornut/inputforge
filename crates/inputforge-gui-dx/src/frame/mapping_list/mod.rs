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
                AddInline { force_expanded: force_expand_add }
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
                AddInline { force_expanded: force_expand_add }
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
            AddInline { force_expanded: force_expand_add }
            ContextMenuMount {
                menu_open: menu_open,
                renaming: renaming,
                delete_target: delete_target,
                pending_duplicate: pending_duplicate,
            }
            DeleteDialogMount { delete_target: delete_target }
            DuplicateWatcher { pending_duplicate: pending_duplicate }
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
                placeholder: "Filter mappings...".to_owned(),
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
    let cap = use_context::<LiveCapture>();

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
                    let mut pd = pending_duplicate;
                    pd.set(Some(target_for_dup.clone()));
                    cap.start.call(crate::patterns::live_capture::CaptureFilter::Any);
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
                "Duplicate to mode..."
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

#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant qualifications on event listeners."
)]
fn DuplicateWatcher(pending_duplicate: Signal<Option<MappingSummary>>) -> Element {
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let cap = use_context::<LiveCapture>();

    let editing = view.editing_mode;
    let ctx_for_cap = ctx.clone();
    use_effect(move || {
        let captured_now = cap.captured.read().clone();
        let Some(source) = pending_duplicate.read().clone() else {
            return;
        };
        let Some(captured_addr) = captured_now else {
            return;
        };
        let mode_now = editing.read().clone();
        let cfg = ctx_for_cap.config.read();
        let collision = cfg
            .mappings
            .iter()
            .any(|m| m.input == captured_addr && m.mode == mode_now);
        drop(cfg);

        if collision {
            let mut sel = view.selected_mapping;
            sel.set(Some((mode_now.clone(), captured_addr.clone())));
        } else {
            let actions = ctx_for_cap
                .state
                .read()
                .active_profile
                .as_ref()
                .and_then(|p| {
                    p.find_mapping(&source.input, &source.mode)
                        .map(|m| m.actions.clone())
                })
                .unwrap_or_default();
            let new_name = format!("{} (copy)", source.name.as_deref().unwrap_or("(unnamed)"),);
            let _ = ctx_for_cap.commands.send(EngineCommand::SetMapping {
                input: captured_addr.clone(),
                mode: mode_now.clone(),
                name: Some(new_name),
                actions,
            });
            let mut sel = view.selected_mapping;
            sel.set(Some((mode_now, captured_addr)));
            tracing::info!(
                target: "f8::mapping_list",
                action = "duplicate_capture_success",
                "dispatch SetMapping (duplicate-with-fresh-capture)",
            );
        }
        cap.cancel.call(());
        let mut pd = pending_duplicate;
        pd.set(None);
    });

    let pending = pending_duplicate.read().clone();
    if pending.is_none() || !*cap.active.read() {
        return rsx! {};
    }
    let source_name = pending
        .as_ref()
        .and_then(|s| s.name.clone())
        .unwrap_or_else(|| "(unnamed)".to_owned());
    rsx! {
        div { class: "if-add-inline if-add-inline--armed if-add-inline--duplicate",
            div { class: "if-add-inline__pad",
                "Press an input to bind the copy of "
                strong { "{source_name}" }
                "..."
            }
        }
    }
}
