//! Single mapping-list row. See spec sec. "Row anatomy".

#![allow(
    clippy::doc_markdown,
    reason = "Doc comments reference DnD as a domain term, not as code."
)]

use dioxus::prelude::*;

use inputforge_core::types::{
    HatDirection, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis,
};

use crate::components::sortable::{SortableHandle, SortableState};
use crate::components::{Chip, ChipVariant};
use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MappingSummary};
use crate::frame::mapping_editor::live_readout::{
    read_axis_display, read_button_pressed, read_hat_direction,
};
use crate::frame::mapping_list::group::{GroupKind, group_of};
use crate::frame::mapping_list::source_label;
use crate::frame::view_state::ViewState;

/// Stable u32 mapping for `GroupKind`. The sortable primitive's
/// validator is `fn(&u32, &u32) -> bool`; the consumer's group
/// discriminator passes through that function pointer's signature.
pub(crate) fn group_to_u32(group: GroupKind) -> u32 {
    match group {
        GroupKind::Axes => 0,
        GroupKind::Buttons => 1,
        GroupKind::Hats => 2,
    }
}

/// Live-input magnitude for `addr` in `[0.0, 1.0]`, drives the row's
/// `--row-live-intensity` custom property and gates `is-live-active`.
///
/// Axis intensity is `|value|` in the natural domain (bipolar
/// `[-1, 1]` -> `[0, 1]`; unipolar already `[0, 1]`). Buttons and hats
/// are binary: `1.0` when pressed / off-center, else `0.0`. Returns
/// `0.0` for `Unbound` or when the device or input index is missing
/// from the snapshot.
fn live_intensity_for(addr: &InputAddress, live: &LiveSnapshot, cfg: &ConfigSnapshot) -> f64 {
    match addr {
        InputAddress::Unbound => 0.0,
        InputAddress::Bound { input, .. } => match input {
            InputId::Axis { .. } => read_axis_display(addr, live, cfg)
                .value
                .abs()
                .clamp(0.0, 1.0),
            InputId::Button { .. } => {
                if read_button_pressed(addr, live, cfg) {
                    1.0
                } else {
                    0.0
                }
            }
            InputId::Hat { .. } => match read_hat_direction(addr, live, cfg) {
                HatDirection::Center => 0.0,
                _ => 1.0,
            },
        },
    }
}

fn compact_output_label(output: &OutputAddress) -> String {
    let suffix = match output.output {
        OutputId::Axis { id } => match id {
            VJoyAxis::X => "X",
            VJoyAxis::Y => "Y",
            VJoyAxis::Z => "Z",
            VJoyAxis::Rx => "Rx",
            VJoyAxis::Ry => "Ry",
            VJoyAxis::Rz => "Rz",
            VJoyAxis::Slider0 => "Slider 0",
            VJoyAxis::Slider1 => "Slider 1",
        }
        .to_owned(),
        OutputId::Button { id } => format!("Btn {id}"),
        OutputId::Hat { id } => format!("Hat {id}"),
    };
    format!("vJoy {} · {}", output.device, suffix)
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
) -> Element {
    tracing::trace!(target: "frame::render", region = "mapping_list::row");
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();

    // Subscribe to the 60 Hz live snapshot and derive this row's input
    // intensity in `[0.0, 1.0]`. Drives the `is-live-active` class and
    // the `--row-live-intensity` inline custom property; the CSS layers
    // an inset 1 px `--color-live` ring proportional to that value.
    let live_intensity = {
        let live = ctx.live.read();
        let cfg = ctx.config.read();
        live_intensity_for(&summary.input, &live, &cfg)
    };
    let is_live_active = live_intensity > 0.0;

    // Rename branch: when this row's input matches the parent's rename
    // selector, swap the name area for the inline editor while keeping
    // the source line and glyphs in place.
    let is_renaming = renaming
        .read()
        .as_ref()
        .is_some_and(|a| a == &summary.input);

    let (device_label, input_label) = source_label::split_label(&summary.input, &ctx.config.read());

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
    // active config. Used by the drag-source modifier and threaded into
    // `SortableHandle` for live-region announcements.
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

    // `if-sortable--dragging` dims this row while it is the active drag
    // source. Drop-target painting lives on the inter-row gaps, not on
    // rows: gaps own ondragover/ondrop, rows own only the drag source.
    let is_drag_source = sortable
        .drag_from
        .read()
        .is_some_and(|src_idx| src_idx == subgroup_idx)
        && sortable
            .drag_group
            .read()
            .is_some_and(|src_group| src_group == group_id);
    let mut class = String::from("if-row");
    if is_active {
        class.push_str(" is-active");
    }
    if is_live_active {
        class.push_str(" is-live-active");
    }
    if is_drag_source {
        class.push_str(" if-sortable--dragging");
    }
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
        div {
            class: "{class}",
            role: "button",
            tabindex: if is_active { "0" } else { "-1" },
            style: "{live_style}",
            onclick,
            oncontextmenu,
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
                if let Some(name) = &summary.name {
                    div { class: "if-row__name",
                        "{name}"
                    }
                }
            }
            div { class: "if-row__source",
                div { class: "if-row__source-primary",
                    if !device_label.is_empty() {
                        span { class: "if-row__source-device", "{device_label}" }
                        span {
                            class: "if-row__source-sep",
                            "aria-hidden": "true",
                            "\u{00b7}"
                        }
                    }
                    span { class: "if-row__source-input", "{input_label}" }
                    if let Some(output) = &summary.first_vjoy_output {
                        span { class: "if-row__source-arrow", "aria-hidden": "true", "\u{2192}" }
                        Chip {
                            variant: ChipVariant::Output,
                            class: "if-row__output-chip".to_owned(),
                            title: compact_output_label(output),
                            "{compact_output_label(output)}"
                        }
                    }
                }
                if merge_glyph.is_some() || cond_glyph.is_some() {
                    div { class: "if-row__source-qualifiers",
                        if let Some(secondary_label) = merge_glyph {
                            Chip {
                                variant: ChipVariant::Outline,
                                class: "if-row__qualifier glyph-merge".to_owned(),
                                title: format!("Merge: {secondary_label}"),
                                span { class: "if-row__qualifier-glyph", "+" }
                                span { class: "if-row__qualifier-text", "{secondary_label}" }
                            }
                        }
                        if let Some(predicate_label) = cond_glyph {
                            Chip {
                                variant: ChipVariant::Outline,
                                class: "if-row__qualifier glyph-cond".to_owned(),
                                title: format!("Condition: {predicate_label}"),
                                span { class: "if-row__qualifier-glyph", "\u{2295}" }
                                span { class: "if-row__qualifier-text", "{predicate_label}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
