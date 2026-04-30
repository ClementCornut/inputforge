//! Single mapping-list row. See spec § "Row anatomy".

use dioxus::prelude::*;

use inputforge_core::types::{InputAddress, InputId};

use crate::context::{AppContext, MappingSummary};
use crate::frame::mapping_list::source_label;
use crate::frame::view_state::ViewState;

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

    let class = if is_active {
        "if-row is-active"
    } else {
        "if-row"
    };

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
            class,
            role: "button",
            tabindex: if is_active { "0" } else { "-1" },
            onclick,
            oncontextmenu,
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
