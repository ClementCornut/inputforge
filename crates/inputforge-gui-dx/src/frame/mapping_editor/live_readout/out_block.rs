//! OUT-side rows for terminal live readout destinations.
//!
//! This module renders vJoy axis and button outputs as regular readout
//! rows. vJoy hats and keyboard combos use destination-specific rows
//! that preserve the surrounding section structure.

use dioxus::prelude::*;

use inputforge_core::state::EngineStatus;
use inputforge_core::types::{AxisPolarity, HatDirection, OutputId};

use crate::context::AppContext;

use super::analyzer::{LiveReadoutModel, OutputDescriptor, OutputDestination};
use super::in_block::ReadoutRow;
use super::value_helpers::{
    AxisDisplay, format_key_combo, format_output_label, read_output_button, read_output_display,
    read_output_hat,
};

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
            OutputId::Button { .. } => {
                let live = ctx.live.read();
                let cfg = ctx.config.read();
                let tag = format_output_label(out);
                let pressed = read_output_button(out, &live, &cfg);
                let display = AxisDisplay {
                    value: if pressed { 1.0 } else { 0.0 },
                    polarity: AxisPolarity::Unipolar,
                };

                rsx! {
                    ReadoutRow {
                        label: "OUT".to_owned(),
                        tag,
                        display,
                        frozen,
                    }
                }
            }
            OutputId::Hat { .. } => {
                let live = ctx.live.read();
                let cfg = ctx.config.read();
                let tag = format_output_label(out);
                let glyph = hat_glyph_for(read_output_hat(out, &live, &cfg));
                let row_class = hat_row_class(frozen);

                rsx! {
                    div { class: "{row_class}",
                        div { class: "if-editor__readout-label", "OUT" }
                        div { class: "if-editor__readout-tag", "{tag}" }
                        div { class: "if-editor__readout-hat-glyph", "{glyph}" }
                        div { class: "if-editor__readout-pct" }
                    }
                }
            }
        },
        OutputDestination::Keyboard(combo) => {
            let combo_text = format_key_combo(combo);
            let row_class = keyboard_row_class(frozen);
            let chip_class = keyboard_chip_class(frozen);

            rsx! {
                div { class: "{row_class}",
                    div { class: "if-editor__readout-label", "OUT" }
                    div { class: "if-editor__readout-tag", "Keyboard" }
                    div { class: "if-editor__readout-kb-cell",
                        span { class: "{chip_class}", "{combo_text}" }
                    }
                    div { class: "if-editor__readout-pct" }
                }
            }
        }
    }
}

fn hat_row_class(frozen: bool) -> &'static str {
    if frozen {
        "if-editor__readout-row if-editor__readout-row--hat if-editor__readout-row--frozen"
    } else {
        "if-editor__readout-row if-editor__readout-row--hat"
    }
}

fn keyboard_row_class(frozen: bool) -> &'static str {
    if frozen {
        "if-editor__readout-row if-editor__readout-row--kb if-editor__readout-row--frozen"
    } else {
        "if-editor__readout-row if-editor__readout-row--kb"
    }
}

fn keyboard_chip_class(frozen: bool) -> &'static str {
    if frozen {
        "if-editor__readout-kb-chip if-editor__readout-kb-chip--idle"
    } else {
        "if-editor__readout-kb-chip if-editor__readout-kb-chip--live"
    }
}

fn hat_glyph_for(direction: HatDirection) -> char {
    match direction {
        HatDirection::N => '\u{2191}',
        HatDirection::NE => '\u{2197}',
        HatDirection::E => '\u{2192}',
        HatDirection::SE => '\u{2198}',
        HatDirection::S => '\u{2193}',
        HatDirection::SW => '\u{2199}',
        HatDirection::W => '\u{2190}',
        HatDirection::NW => '\u{2196}',
        HatDirection::Center => '\u{00b7}',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hat_glyph_for_maps_all_directions() {
        assert_eq!(hat_glyph_for(HatDirection::N), '\u{2191}');
        assert_eq!(hat_glyph_for(HatDirection::NE), '\u{2197}');
        assert_eq!(hat_glyph_for(HatDirection::E), '\u{2192}');
        assert_eq!(hat_glyph_for(HatDirection::SE), '\u{2198}');
        assert_eq!(hat_glyph_for(HatDirection::S), '\u{2193}');
        assert_eq!(hat_glyph_for(HatDirection::SW), '\u{2199}');
        assert_eq!(hat_glyph_for(HatDirection::W), '\u{2190}');
        assert_eq!(hat_glyph_for(HatDirection::NW), '\u{2196}');
        assert_eq!(hat_glyph_for(HatDirection::Center), '\u{00b7}');
    }

    #[test]
    fn hat_and_keyboard_row_classes_include_kind_and_frozen_state() {
        assert_eq!(
            hat_row_class(false),
            "if-editor__readout-row if-editor__readout-row--hat"
        );
        assert_eq!(
            hat_row_class(true),
            "if-editor__readout-row if-editor__readout-row--hat if-editor__readout-row--frozen"
        );
        assert_eq!(
            keyboard_row_class(false),
            "if-editor__readout-row if-editor__readout-row--kb"
        );
        assert_eq!(
            keyboard_row_class(true),
            "if-editor__readout-row if-editor__readout-row--kb if-editor__readout-row--frozen"
        );
    }

    #[test]
    fn keyboard_chip_class_tracks_frozen_state() {
        assert_eq!(
            keyboard_chip_class(false),
            "if-editor__readout-kb-chip if-editor__readout-kb-chip--live"
        );
        assert_eq!(
            keyboard_chip_class(true),
            "if-editor__readout-kb-chip if-editor__readout-kb-chip--idle"
        );
    }
}
