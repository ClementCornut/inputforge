use dioxus::prelude::*;
#[path = "in_block_presentation.rs"]
mod presentation;
pub(super) use presentation::ReadoutRow;
use presentation::{
    button_cell_class, button_row_class, button_state_label, hat_compass_class, hat_row_class,
    hat_spoke_class, input_row_label, predicate_chip_label,
};

use inputforge_core::types::HatDirection;

use crate::context::AppContext;
use crate::frame::mapping_list::source_label;

use super::value_helpers::{
    ReadoutDisplay, hat_direction_label, hat_glyph_for, read_input_display,
};
use crate::readouts::LiveReadoutModel;

const READOUT_SECTION_CLASS: &str = "if-editor__readout-section";
const READOUT_SECTION_LABEL_CLASS: &str = "if-editor__readout-section-label";
const READOUT_GROUP_CLASS: &str = "if-editor__readout-group";
const READOUT_PREDICATE_LABEL_CLASS: &str = "if-editor__readout-predicate-label";
const READOUT_CHIPS_CLASS: &str = "if-editor__readout-chips";

/// High-level IN side of the live readout.
///
/// Pipeline inputs are rendered as regular axis rows. Predicate chips
/// are rendered below them when the analyzer found conditional leaves.
#[component]
pub(super) fn InBlock(model: LiveReadoutModel) -> Element {
    let ctx = use_context::<AppContext>();
    let live = ctx.live.read();
    let cfg = ctx.config.read();
    let has_multiple_inputs = model.pipeline_inputs.len() > 1;
    let has_predicates = !model.predicates.is_empty();

    rsx! {
        div { class: "{READOUT_SECTION_CLASS}",
            if has_multiple_inputs {
                div { class: "{READOUT_SECTION_LABEL_CLASS}", "IN \u{00b7} pipeline" }
            }
            div { class: "{READOUT_GROUP_CLASS}",
                for (idx, addr) in model.pipeline_inputs.iter().enumerate() {
                    {
                        let label = input_row_label(idx, has_multiple_inputs);
                        let tag = source_label::format(addr, &cfg);
                        match read_input_display(addr, &live, &cfg) {
                            ReadoutDisplay::Axis(display) => rsx! {
                                ReadoutRow {
                                    key: "pipeline-{idx}",
                                    label,
                                    tag,
                                    display,
                                    frozen: false,
                                }
                            },
                            ReadoutDisplay::Button { pressed } => rsx! {
                                ButtonReadoutRow {
                                    key: "pipeline-{idx}",
                                    label,
                                    tag,
                                    pressed,
                                    frozen: false,
                                }
                            },
                            ReadoutDisplay::Hat { direction } => rsx! {
                                HatReadoutRow {
                                    key: "pipeline-{idx}",
                                    label,
                                    tag,
                                    direction,
                                    frozen: false,
                                }
                            },
                        }
                    }
                }
            }
        }
        if has_predicates {
            div { class: "{READOUT_SECTION_CLASS}",
                div { class: "{READOUT_SECTION_LABEL_CLASS}", "IN \u{00b7} predicates" }
                div { class: "{READOUT_GROUP_CLASS}",
                    div { class: "if-editor__readout-predicate-row",
                        div { class: "{READOUT_PREDICATE_LABEL_CLASS}", "IF" }
                        div { class: "{READOUT_CHIPS_CLASS}",
                            for (idx, predicate) in model.predicates.iter().enumerate() {
                                {
                                    let label = predicate_chip_label(predicate);
                                    rsx! {
                                        crate::components::Badge { key: "predicate-{idx}", variant: if predicate.state { crate::components::BadgeVariant::Success } else { crate::components::BadgeVariant::Neutral },
                                            crate::components::StatusDot { variant: if predicate.state { crate::components::BadgeVariant::Success } else { crate::components::BadgeVariant::Neutral }, hollow: !predicate.state }
                                            span { class: "if-editor__readout-chip-label", "{label}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// One binary button row in the readout grid.
#[component]
pub(super) fn ButtonReadoutRow(label: String, tag: String, pressed: bool, frozen: bool) -> Element {
    let row_class = button_row_class(frozen);
    let cell_class = button_cell_class(frozen);
    let state_label = button_state_label(pressed);
    let visual_live = pressed && !frozen;

    rsx! {
        div { class: "{row_class}",
            div { class: "if-editor__readout-label", "{label}" }
            div { class: "if-editor__readout-tag", "{tag}" }
            div { class: "{cell_class}",
                crate::components::StatusDot { variant: if visual_live { crate::components::BadgeVariant::Success } else { crate::components::BadgeVariant::Neutral }, hollow: !pressed }
                crate::components::Badge { variant: if visual_live { crate::components::BadgeVariant::Success } else { crate::components::BadgeVariant::Neutral }, "{state_label}" }
            }
            div { class: "if-editor__readout-pct" }
        }
    }
}

/// One 8-way hat row in the readout grid.
#[component]
pub(super) fn HatReadoutRow(
    label: String,
    tag: String,
    direction: HatDirection,
    frozen: bool,
) -> Element {
    let row_class = hat_row_class(frozen);
    let compass_class = hat_compass_class(frozen);
    let direction_label = hat_direction_label(direction);
    let direction_glyph = hat_glyph_for(direction);
    let center_glyph = hat_glyph_for(HatDirection::Center);

    rsx! {
        div { class: "{row_class}",
            div { class: "if-editor__readout-label", "{label}" }
            div { class: "if-editor__readout-tag", "{tag}" }
            div { class: "{compass_class}", "aria-label": "Hat direction {direction_label}",
                span { class: "{hat_spoke_class(direction, HatDirection::NW)}", "\u{2196}" }
                span { class: "{hat_spoke_class(direction, HatDirection::N)}", "\u{2191}" }
                span { class: "{hat_spoke_class(direction, HatDirection::NE)}", "\u{2197}" }
                span { class: "{hat_spoke_class(direction, HatDirection::W)}", "\u{2190}" }
                span { class: "{hat_spoke_class(direction, HatDirection::Center)}", "{center_glyph}" }
                span { class: "{hat_spoke_class(direction, HatDirection::E)}", "\u{2192}" }
                span { class: "{hat_spoke_class(direction, HatDirection::SW)}", "\u{2199}" }
                span { class: "{hat_spoke_class(direction, HatDirection::S)}", "\u{2193}" }
                span { class: "{hat_spoke_class(direction, HatDirection::SE)}", "\u{2198}" }
                span { class: "if-editor__readout-hat-state", "{direction_label} {direction_glyph}" }
            }
            div { class: "if-editor__readout-pct" }
        }
    }
}
