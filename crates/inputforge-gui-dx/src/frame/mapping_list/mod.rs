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
    reason = "Tasks 11-18 wire consumers of these sub-modules; until then they are scaffolding."
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

#[allow(
    dead_code,
    reason = "rsx! macro is opaque to rustc; constant is consumed by Stylesheet { href: MAPPING_LIST_CSS }"
)]
const MAPPING_LIST_CSS: Asset = asset!("/assets/frame/mapping_list.css");

#[component]
pub(crate) fn MappingList() -> Element {
    tracing::trace!(target: "frame::render", region = "mapping_list");
    rsx! {
        Stylesheet { href: MAPPING_LIST_CSS }
        div { class: "if-rail",
            // Stub — Task 19 wires filter / rows / empty states / inline editor.
        }
    }
}
