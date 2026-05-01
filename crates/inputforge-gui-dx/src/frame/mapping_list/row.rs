//! Single mapping-list row. See spec § "Row anatomy".

#![allow(
    clippy::doc_markdown,
    reason = "Doc comments reference DnD as a domain term, not as code."
)]

use std::rc::Rc;

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;
use inputforge_core::types::{InputAddress, InputId};

use crate::components::sortable::{
    SortableHandle, SortableItemConfig, SortableSide, SortableState, use_sortable_item,
};
use crate::context::{AppContext, MappingSummary};
use crate::frame::mapping_list::group::{GroupKind, group_of};
use crate::frame::mapping_list::source_label;
use crate::frame::view_state::ViewState;

/// Stable u32 mapping for `GroupKind`. The sortable primitive's
/// validator is `fn(u32, u32) -> bool`; the consumer's group
/// discriminator passes through that function pointer's signature.
fn group_to_u32(group: GroupKind) -> u32 {
    match group {
        GroupKind::Axes => 0,
        GroupKind::Buttons => 1,
        GroupKind::Hats => 2,
    }
}

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
    /// Task 14 only forwards the prop; Task 15 introduces the rename branch.
    renaming: Signal<Option<InputAddress>>,
    /// Shared sortable state (drag source / drop indicator / live region).
    /// Owned by `MappingList`, threaded through every row.
    sortable: SortableState<u32>,
    /// `true` when the filter input has narrowed the visible set. While
    /// active, the drag handle is non-draggable (drop-target rendering
    /// is also gated to ignore any in-flight drag).
    filter_active: bool,
    /// RMB / Shift+F10 fires this with `(input, x, y)` so the parent can
    /// open the context menu at the cursor. Coordinates are page-space.
    on_open_menu: EventHandler<(InputAddress, f64, f64)>,
) -> Element {
    tracing::trace!(target: "frame::render", region = "mapping_list::row");
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();

    // Rename branch, when this row's input matches the parent's
    // rename selector, swap the name area for the inline editor while
    // keeping the source line and glyphs in place. The source line is
    // the user's only handle on which row they are renaming, so it must
    // stay visible during the rename.
    let is_renaming = renaming
        .read()
        .as_ref()
        .is_some_and(|a| a == &summary.input);

    let (device_label, input_label) = source_label::split_label(&summary.input, &ctx.config.read());
    let kind_class = match summary.input.input {
        InputId::Axis { .. } => "axis",
        InputId::Button { .. } => "button",
        InputId::Hat { .. } => "hat",
    };

    let mut sel = view.selected_mapping;
    let summary_for_click = summary.clone();
    let onclick = move |_| {
        sel.set(Some((
            summary_for_click.mode.clone(),
            summary_for_click.input.clone(),
        )));
    };
    let summary_for_ctx = summary.clone();
    let on_open_menu_inner = on_open_menu;
    let oncontextmenu = move |evt: MouseEvent| {
        evt.prevent_default();
        evt.stop_propagation();
        let coords = evt.client_coordinates();
        on_open_menu_inner.call((summary_for_ctx.input.clone(), coords.x, coords.y));
    };

    // Compute this row's group + group-local index + group_len from the
    // active config. Cached locally so the sortable primitive's handlers
    // don't re-walk `cfg.mappings` on every mouse move.
    let group_kind = group_of(&summary.input);
    let group_id = group_to_u32(group_kind);
    let (subgroup_idx, group_len) = {
        let cfg = ctx.config.read();
        let group_inputs: Vec<&InputAddress> = cfg
            .mappings
            .iter()
            .filter(|m| m.mode == summary.mode && group_of(&m.input) == group_kind)
            .map(|m| &m.input)
            .collect();
        let len = group_inputs.len();
        let pos = group_inputs
            .iter()
            .position(|i| **i == summary.input)
            .unwrap_or(0);
        (pos, len)
    };

    // Build classlist. `if-sortable--dragging` dims this row while it is
    // the active drag source. `if-sortable--drop-before` /
    // `--drop-after` / `--drop-invalid` paint the insertion bar on the
    // appropriate side via the primitive's `::before` / `::after`
    // pseudo-elements.
    let is_drag_source = sortable
        .drag_from
        .read()
        .is_some_and(|src_idx| src_idx == subgroup_idx)
        && sortable
            .drag_group
            .read()
            .is_some_and(|src_group| src_group == group_id);
    let drop_marker = sortable.drop_target.read();
    // Match on `(index, group)` together: group-local indices are not
    // unique across groups (Axes idx 0 and Buttons idx 0 both exist). A
    // filter on `index` alone would paint the indicator on every row
    // whose subgroup-index matches, regardless of group.
    let (drop_before, drop_after, drop_invalid) = drop_marker
        .as_ref()
        .filter(|d| d.index == subgroup_idx && d.group == group_id)
        .map_or((false, false, false), |d| match (d.side, d.invalid) {
            (SortableSide::Before, false) => (true, false, false),
            (SortableSide::After, false) => (false, true, false),
            (SortableSide::Before, true) => (true, false, true),
            (SortableSide::After, true) => (false, true, true),
        });
    drop(drop_marker);
    let mut class = String::from("if-row");
    if is_active {
        class.push_str(" is-active");
    }
    if is_drag_source {
        class.push_str(" if-sortable--dragging");
    }
    if drop_before {
        class.push_str(" if-sortable--drop-before");
    }
    if drop_after {
        class.push_str(" if-sortable--drop-after");
    }
    if drop_invalid {
        class.push_str(" if-sortable--drop-invalid");
    }

    // Element ref for the cursor-Y midpoint computation in
    // `use_sortable_item.ondragover`. Set by the row's `onmounted`.
    let mut item_ref: Signal<Option<Rc<MountedData>>> = use_signal(|| None);

    // DnD wiring -- delegated to the sortable primitive. The validator
    // forbids cross-group drops (mappings can't change their input
    // kind). The on_drop callback dispatches the engine command and
    // writes the AT live-region announcement.
    //
    // Source vs target: the primitive's per-row `on_drop` closure fires
    // on the **target** row (the row whose `ondrop` event was hit), but
    // `ReorderMapping.input` must identify the **source** mapping (the
    // dragged row). The closure resolves the source's `InputAddress`
    // by reading `sortable.drag_from` (still populated when the
    // callback runs; the primitive clears it after the closure returns)
    // and looking it up in the active config under the same
    // (mode, group_kind) filter the row used to compute its own
    // `subgroup_idx`. Dispatching the target's input here would move
    // the wrong mapping.
    let cmd_for_drop = ctx.commands.clone();
    let summary_for_drop = summary.clone();
    let mut live_writer = sortable.live_announcement;
    let config_for_drop = ctx.config;
    let drag_from_for_drop = sortable.drag_from;
    let handlers = use_sortable_item(SortableItemConfig {
        state: sortable,
        index: subgroup_idx,
        group: group_id,
        group_len,
        item_ref,
        validate_drop: Some(|src: &u32, tgt: &u32| src == tgt),
        on_drop: move |to: usize, _side: SortableSide| {
            // Read the source's group-local subpos from the primitive's
            // shared state. `drag_from` is populated on dragstart and
            // cleared by the primitive after this callback returns, so
            // it is guaranteed to be `Some` here unless an upstream
            // invariant was violated; bail defensively.
            let Some(src_subpos) = *drag_from_for_drop.peek() else {
                return;
            };
            // Resolve the source row's `InputAddress` from the active
            // config. Same (mode, group_kind) filter the row used at
            // render time to compute `subgroup_idx`, so the index
            // domains match. Drop the read guard before dispatching so
            // we don't hold it across the engine-channel send.
            let src_input = {
                let cfg = config_for_drop.read();
                cfg.mappings
                    .iter()
                    .filter(|m| m.mode == summary_for_drop.mode && group_of(&m.input) == group_kind)
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
                target = ?summary_for_drop.input,
                mode = %summary_for_drop.mode,
                to,
                "dispatch ReorderMapping",
            );
            let _ = cmd_for_drop.send(EngineCommand::ReorderMapping {
                input: src_input,
                mode: summary_for_drop.mode.clone(),
                target_index_in_group: to,
            });
            // The engine clamps `target_index_in_group` to group_len-1;
            // mirror that here so the announcement reflects the actual
            // landed position rather than the user's pre-clamp intent.
            let landed_subpos = to.min(group_len.saturating_sub(1));
            live_writer.set(crate::frame::mapping_list::format_reorder_announcement(
                landed_subpos,
                group_len,
                group_kind,
            ));
        },
    });

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
        div {
            class: "{class}",
            role: "button",
            tabindex: if is_active { "0" } else { "-1" },
            onclick,
            oncontextmenu,
            ondragover: handlers.ondragover,
            ondragleave: handlers.ondragleave,
            ondragend: handlers.ondragend,
            ondrop: handlers.ondrop,
            onmounted: move |evt: MountedEvent| {
                item_ref.set(Some(evt.data()));
            },
            SortableHandle {
                state: sortable,
                index: subgroup_idx,
                group: group_id,
                group_len,
                draggable: !filter_active,
            }
            if is_renaming {
                crate::frame::mapping_list::rename_inline::RenameInline {
                    summary: summary.clone(),
                    state: renaming,
                }
            } else {
                div { class: "if-row__name",
                    if let Some(name) = &summary.name {
                        "{name}"
                    } else {
                        em { class: "if-row__name--unnamed", "(unnamed)" }
                    }
                }
            }
            div { class: "if-row__source",
                span { class: "if-row__source-device", "{device_label}" }
                span {
                    class: "if-row__source-input",
                    "data-kind": kind_class,
                    "{input_label}"
                }
                if let Some(secondary_label) = merge_glyph {
                    span {
                        class: "glyph-merge",
                        title: "MergeAxis",
                        "+ "
                    }
                    em { "{secondary_label}" }
                }
                if let Some(predicate_label) = cond_glyph {
                    span {
                        class: "glyph-cond",
                        title: "{predicate_label}",
                        "\u{2295} "
                    }
                    em { "{predicate_label}" }
                }
            }
        }
    }
}
