//! F-bulk-map: side-panel bulk mapping wizard. See
//! `docs/superpowers/specs/2026-05-03-bulk-mapping-design.md`.

#![allow(
    dead_code,
    reason = "Module is wired progressively across tasks 9-18; final exports settle in task 18."
)]

mod apply;
mod auto_map;
mod conflicts;
mod empty_state;
mod group_actions;
mod row_readout;
mod state;
mod summary;

#[cfg(test)]
mod tests;

use dioxus::prelude::*;

const BULK_MAP_CSS: Asset = asset!("/assets/frame/bulk_map.css");

/// Bulk-map wizard panel. Mounts inside `<aside class="if-panel-slot">`
/// when `view.panel_slot == PanelSlot::BulkMap`.
#[component]
pub(crate) fn BulkMapPanel() -> Element {
    tracing::trace!(target: "frame::render", region = "bulk_map");
    rsx! {
        Stylesheet { href: BULK_MAP_CSS }
        section { class: "if-bulk-map", "aria-label": "Bulk-map device wizard",
            // Real layout assembled in task 18.
            "Bulk-map wizard (under construction)"
        }
    }
}
