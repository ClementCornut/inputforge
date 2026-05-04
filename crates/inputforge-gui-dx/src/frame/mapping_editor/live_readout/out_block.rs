//! OUT-side rows for terminal live readout destinations.
//!
//! This module currently renders vJoy axis outputs as regular readout
//! rows. Other output kinds keep a minimal row-shaped placeholder so
//! later tasks can fill in their destination-specific visuals without
//! changing the surrounding section structure.

use dioxus::prelude::*;

use inputforge_core::state::EngineStatus;
use inputforge_core::types::OutputId;

use crate::context::AppContext;

use super::analyzer::{LiveReadoutModel, OutputDescriptor, OutputDestination};
use super::in_block::ReadoutRow;
use super::value_helpers::{format_key_combo, format_output_label, read_output_display};

const READOUT_SECTION_CLASS: &str = "if-editor__readout-section";
const READOUT_SECTION_LABEL_CLASS: &str = "if-editor__readout-section-label";
const READOUT_GROUP_CLASS: &str = "if-editor__readout-group";

/// High-level OUT side of the live readout.
#[component]
pub(super) fn OutBlock(model: LiveReadoutModel) -> Element {
    if model.outputs.is_empty() {
        return rsx! {};
    }

    rsx! {
        div { class: "{READOUT_SECTION_CLASS}",
            div { class: "{READOUT_SECTION_LABEL_CLASS}", "OUT" }
            div { class: "{READOUT_GROUP_CLASS}",
                for (idx, descriptor) in model.outputs.iter().cloned().enumerate() {
                    OutRow {
                        key: "output-{idx}",
                        descriptor,
                    }
                }
            }
        }
    }
}

/// Render one OUT row for an analyzed terminal destination.
#[component]
pub(super) fn OutRow(descriptor: OutputDescriptor) -> Element {
    let ctx = use_context::<AppContext>();
    let engine_running = matches!(ctx.meta.read().engine_status, EngineStatus::Running);
    let frozen = !engine_running || !descriptor.is_active;

    match &descriptor.destination {
        OutputDestination::VJoy(out) => match out.output {
            OutputId::Axis { .. } => {
                let live = ctx.live.read();
                let cfg = ctx.config.read();
                let tag = format_output_label(out);
                let display = read_output_display(out, &live, &cfg, descriptor.polarity);

                rsx! {
                    ReadoutRow {
                        label: "OUT".to_owned(),
                        tag,
                        display,
                        frozen,
                    }
                }
            }
            OutputId::Button { .. } | OutputId::Hat { .. } => {
                let tag = format_output_label(out);
                render_placeholder_row(&tag, frozen)
            }
        },
        OutputDestination::Keyboard(combo) => {
            let tag = format_key_combo(combo);
            render_placeholder_row(&tag, frozen)
        }
    }
}

fn render_placeholder_row(tag: &str, frozen: bool) -> Element {
    let row_class = if frozen {
        "if-editor__readout-row if-editor__readout-row--frozen if-editor__readout-row--placeholder"
    } else {
        "if-editor__readout-row if-editor__readout-row--placeholder"
    };

    rsx! {
        div { class: "{row_class}",
            div { class: "if-editor__readout-label", "OUT" }
            div { class: "if-editor__readout-tag", "{tag}" }
            div { class: "if-editor__readout-bar if-editor__readout-bar--placeholder" }
            div { class: "if-editor__readout-pct" }
        }
    }
}
