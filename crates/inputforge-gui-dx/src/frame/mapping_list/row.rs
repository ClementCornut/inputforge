//! Single mapping-list row. See spec sec. "Row anatomy".
#![allow(
    clippy::doc_markdown,
    reason = "Doc comments reference DnD as a domain term, not as code."
)]
use crate::components::sortable::{SortableHandle, SortableState};
use crate::components::{Chip, ChipVariant, SelectableRow};
use dioxus::prelude::*;
use inputforge_core::types::InputAddress;
#[path = "row_labels.rs"]
mod labels;
#[path = "row_qualifiers.rs"]
mod qualifiers;
use crate::context::{AppContext, MappingSummary};
use crate::frame::mapping_list::group::group_of;
use crate::frame::mapping_list::source_label;
use labels::{compact_output_label, live_intensity_for};
pub(super) use labels::{group_to_u32, row_dom_id};
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures."
)]
pub(crate) fn Row(
    summary: MappingSummary,
    is_active: bool,
    /// `Some(addr)` when this row's name is currently being inline-renamed.
    renaming: Signal<Option<InputAddress>>,
    /// Shared sortable state (drag source + drop indicator + live region).
    /// Owned by `MappingList`, threaded through every row.
    sortable: SortableState<u32>,
    /// `true` when the filter input has narrowed the visible set. While
    /// active, the drag handle is non-draggable.
    filter_active: bool,
    /// RMB / Shift+F10 fires this with `(input, x, y)` so the parent can
    /// open the context menu at the cursor. Coordinates are page-space.
    on_open_menu: EventHandler<(InputAddress, f64, f64)>,
    #[props(default)] on_drag_start: Option<Callback<MappingSummary>>,
) -> Element {
    tracing::trace!(target : "frame::render", region = "mapping_list::row");
    let ctx = use_context::<AppContext>();
    let drag_context = format!(
        "profile:{:?};mode:{}",
        ctx.meta.read().profile_id,
        summary.mode,
    );
    let live_intensity = {
        let live = ctx.live.read();
        let cfg = ctx.config.read();
        live_intensity_for(&summary.input, &live, &cfg)
    };
    let is_live_active = live_intensity > 0.0;
    let is_renaming = renaming
        .read()
        .as_ref()
        .is_some_and(|a| a == &summary.input);
    let (device_label, input_label) = source_label::split_label(&summary.input, &ctx.config.read());
    let navigation = use_context::<crate::mapping_edits::Bindings>().navigation;
    let profile = ctx.meta.read().profile_id.clone();
    let summary_for_click = summary.clone();
    let onclick = move |_| {
        navigation.dispatch.call(crate::navigation::Intent::Select {
            profile: profile.clone(),
            key: (
                summary_for_click.mode.clone(),
                summary_for_click.input.clone(),
            ),
        });
    };
    let summary_for_ctx = summary.clone();
    let on_open_menu_inner = on_open_menu;
    let ui = use_context::<crate::mapping_edits::Bindings>().ui;
    let oncontextmenu = move |evt| {
        ui.row_menu.call(crate::mapping_edits::ui::RowMenuRequest {
            input: summary_for_ctx.input.clone(),
            event: evt,
            send: on_open_menu_inner,
        });
    };
    let group_kind = group_of(&summary.input);
    let group_id = group_to_u32(group_kind);
    let subgroup_idx = ctx
        .config
        .read()
        .mappings
        .iter()
        .filter(|m| m.mode == summary.mode && group_of(&m.input) == group_kind)
        .position(|m| m.input == summary.input)
        .unwrap_or(0);
    let is_drag_source = sortable
        .drag_from
        .read()
        .is_some_and(|src_idx| src_idx == subgroup_idx)
        && sortable
            .drag_group
            .read()
            .is_some_and(|src_group| src_group == group_id);
    let live_style = format!("--row-live-intensity: {live_intensity:.3};");
    let merge_glyph = summary.glyphs.merge_secondary.as_ref().map(|secondary| {
        let cfg = ctx.config.read();
        source_label::format(secondary, &cfg)
    });
    let cond_glyph = summary
        .glyphs
        .first_input_predicate
        .as_ref()
        .map(|predicate| {
            let cfg = ctx.config.read();
            source_label::format(predicate, &cfg)
        });
    rsx! {
        SelectableRow {
            class: "if-row",
            id: row_dom_id(& summary.mode,& summary
                    .input),
            selected: is_active,
            live: is_live_active,
            dragging: is_drag_source,
            aria_label: summary.name.clone().unwrap_or_else(|| source_label::format(&summary.input, &ctx.config.read())),
            tabindex: if is_active { 0 } else { -1 },
            style: live_style,
            onclick,
            oncontextmenu,
            ondragstart: {
                let source = summary.clone();
                move |_| {
                    if let Some(callback) = on_drag_start {
                        callback.call(source.clone());
                    }
                }
            },
            data_sortable_row: true,
            SortableHandle {
                state: sortable,
                index: subgroup_idx,
                group: group_id,
                context: drag_context,
                draggable: !
                        filter_active,
            }
            if is_renaming {
                crate::frame::mapping_list::rename_inline::RenameInline {
                    summary: summary.clone(),
                    state: renaming,
                }
            } else {
                if let Some(name) = &summary.name {
                    div {
                        class: "if-row__name",
                        "{name}"
                    }
                }
            }
            div {
                class: "if-row__source",
                div {
                    class: "if-row__source-primary",
                    if !device_label.is_empty() {
                        span {
                            class: "if-row__source-device",
                            "{device_label}"
                        }
                    }
                    span {
                        class: "if-row__source-input",
                        "{input_label}"
                    }
                    if let Some(output) = &summary.first_vjoy_output {
                        span {
                            class: "if-row__source-arrow",
                            "aria-hidden": "true",
                            "\u{2192}"
                        }
                        Chip {
                            variant: ChipVariant::Output,
                            class: "if-row__output-chip".to_owned(),
                            title: compact_output_label(output),
                            span {
                                class: "if-row__output-chip-text",
                                "{compact_output_label(output)}"
                            }
                        }
                    }
                }
                qualifiers::Qualifiers { merge: merge_glyph, conditional: cond_glyph }
            }
        }
    }
}
