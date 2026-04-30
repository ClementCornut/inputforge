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

use inputforge_core::types::InputAddress;

use crate::components::{InputSize, TextInput};
use crate::context::{AppContext, MappingSummary};
use crate::frame::mapping_list::add_inline::AddInline;
use crate::frame::mapping_list::empty::{EmptyZeroFilterResults, EmptyZeroMappings};
use crate::frame::mapping_list::filter::matches_filter;
use crate::frame::mapping_list::group::{GroupKind, group_of};
use crate::frame::mapping_list::row::Row;
use crate::frame::view_state::ViewState;

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

// Stub mounts — actual content arrives in Tasks 20 and 21.
#[component]
#[allow(dead_code, reason = "Replaced in Task 20")]
fn ContextMenuMount(
    menu_open: Signal<Option<(InputAddress, f64, f64)>>,
    renaming: Signal<Option<InputAddress>>,
    delete_target: Signal<Option<MappingSummary>>,
    pending_duplicate: Signal<Option<MappingSummary>>,
) -> Element {
    let _ = (menu_open, renaming, delete_target, pending_duplicate);
    rsx! {}
}

#[component]
#[allow(dead_code, reason = "Replaced in Task 21")]
pub(crate) fn DeleteDialogMount(delete_target: Signal<Option<MappingSummary>>) -> Element {
    let _ = delete_target;
    rsx! {}
}

#[component]
#[allow(dead_code, reason = "Replaced in Task 20")]
fn DuplicateWatcher(pending_duplicate: Signal<Option<MappingSummary>>) -> Element {
    let _ = pending_duplicate;
    rsx! {}
}
