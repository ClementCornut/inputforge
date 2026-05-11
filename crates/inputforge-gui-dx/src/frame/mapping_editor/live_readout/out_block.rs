//! OUT-side rows for terminal live readout destinations.
//!
//! This module renders vJoy axis and button outputs as regular readout
//! rows. vJoy hats and keyboard combos use destination-specific rows
//! that preserve the surrounding section structure.

use std::rc::Rc;

use dioxus::prelude::*;

use inputforge_core::action::OutputBehavior;
use inputforge_core::types::OutputId;

use crate::components::Icon;
use crate::context::AppContext;
use crate::icons::{Icon as IconKind, IconSize};

use super::analyzer::{LiveReadoutModel, OutputDescriptor, OutputDestination};
use super::in_block::{ButtonReadoutRow, HatReadoutRow, ReadoutRow};
use super::value_helpers::{
    ReadoutDisplay, format_key_combo, format_output_label, read_output_typed_display,
};

const READOUT_SECTION_CLASS: &str = "if-editor__readout-section";
const READOUT_SECTION_LABEL_CLASS: &str = "if-editor__readout-section-label";
const READOUT_GROUP_CLASS: &str = "if-editor__readout-group";

/// Per-readout expand state for output chains.
///
/// `per_output` is index-aligned with `LiveReadoutModel::outputs`.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ExpandState {
    pub per_output: Vec<bool>,
}

impl ExpandState {
    /// Return whether the output at `idx` is currently expanded.
    pub(super) fn is_expanded(&self, idx: usize) -> bool {
        self.per_output.get(idx).copied().unwrap_or(false)
    }
}

/// High-level OUT side of the live readout.
#[component]
pub(super) fn OutBlock(
    model: LiveReadoutModel,
    expand_state: Signal<ExpandState>,
    engine_running: bool,
) -> Element {
    if model.outputs.is_empty() {
        return rsx! {};
    }
    let outputs: Vec<Rc<OutputDescriptor>> =
        model.outputs.iter().map(|d| Rc::new(d.clone())).collect();
    let has_multiple_outputs = outputs.len() > 1;

    rsx! {
        div { class: "{READOUT_SECTION_CLASS}",
            div { class: "{READOUT_SECTION_LABEL_CLASS}", "OUT \u{00b7} destinations" }
            div { class: "{READOUT_GROUP_CLASS}",
                for (idx, descriptor) in outputs.iter().cloned().enumerate() {
                    OutRow {
                        key: "output-{idx}",
                        descriptor,
                        idx,
                        has_multiple_outputs,
                        expand_state,
                        engine_running,
                    }
                }
            }
        }
    }
}

/// Render one OUT row for an analyzed terminal destination.
#[component]
pub(super) fn OutRow(
    descriptor: Rc<OutputDescriptor>,
    idx: usize,
    has_multiple_outputs: bool,
    expand_state: Signal<ExpandState>,
    engine_running: bool,
) -> Element {
    let ctx = use_context::<AppContext>();
    let frozen = !engine_running || !descriptor.is_active;
    let has_chain = !descriptor.chain.is_empty();
    let expanded = expand_state.read().is_expanded(idx);
    let chevron_icon = chevron_icon(expanded);
    let chevron_label = chevron_label(expanded);
    let row_label = output_row_label(idx, has_multiple_outputs);

    let value_cell = match &descriptor.destination {
        OutputDestination::VJoy(out) => match out.output {
            OutputId::Axis { .. } => {
                let live = ctx.live.read();
                let cfg = ctx.config.read();
                let tag = format_output_label(out);
                let ReadoutDisplay::Axis(display) =
                    read_output_typed_display(out, &live, &cfg, descriptor.polarity)
                else {
                    unreachable!("axis output should produce axis readout display");
                };

                rsx! {
                    ReadoutRow {
                        label: row_label.clone(),
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
                let ReadoutDisplay::Button { pressed } =
                    read_output_typed_display(out, &live, &cfg, descriptor.polarity)
                else {
                    unreachable!("button output should produce button readout display");
                };

                rsx! {
                    ButtonReadoutRow {
                        label: row_label.clone(),
                        tag,
                        pressed,
                        frozen,
                    }
                }
            }
            OutputId::Hat { .. } => {
                let live = ctx.live.read();
                let cfg = ctx.config.read();
                let tag = format_output_label(out);
                let ReadoutDisplay::Hat { direction } =
                    read_output_typed_display(out, &live, &cfg, descriptor.polarity)
                else {
                    unreachable!("hat output should produce hat readout display");
                };

                rsx! {
                    HatReadoutRow {
                        label: row_label.clone(),
                        tag,
                        direction,
                        frozen,
                    }
                }
            }
        },
        OutputDestination::Keyboard {
            key,
            behavior,
            pressed,
        } => {
            let combo_text = format_key_combo(key);
            let tag = format!("Keyboard - {}", format_behavior(*behavior));
            let key_live = engine_running && descriptor.is_active && *pressed;
            let row_class = keyboard_row_class(frozen);
            let chip_class = keyboard_chip_class(key_live);
            let label = row_label.clone();

            rsx! {
                div { class: "{row_class}",
                    div { class: "if-editor__readout-label", "{label}" }
                    div { class: "if-editor__readout-tag", "{tag}" }
                    div { class: "if-editor__readout-kb-cell",
                        span { class: "{chip_class}", "{combo_text}" }
                    }
                    div { class: "if-editor__readout-pct" }
                }
            }
        }
        OutputDestination::Mouse {
            target,
            behavior,
            active,
        } => {
            let is_live = engine_running && descriptor.is_active && *active;
            let tag = if target.is_wheel() {
                target.label().to_owned()
            } else {
                format!("{} - {}", target.label(), format_behavior(*behavior))
            };

            rsx! {
                ButtonReadoutRow {
                    label: row_label.clone(),
                    tag,
                    pressed: if target.is_wheel() { false } else { is_live },
                    frozen,
                }
            }
        }
    };

    let chevron = if has_chain {
        let onclick = move |_| {
            expand_state.with_mut(|s| {
                toggle_output_expanded(s, idx);
            });
        };
        rsx! {
            button {
                class: "if-editor__readout-chevron",
                "type": "button",
                "aria-label": "{chevron_label}",
                "aria-expanded": "{expanded}",
                onclick,
                Icon {
                    name: chevron_icon,
                    size: IconSize::Sm,
                }
            }
        }
    } else {
        rsx! { div { class: "if-editor__readout-chevron-spacer" } }
    };

    let chain_block = if expanded && has_chain {
        rsx! { super::out_chain::OutChain { descriptor: descriptor.as_ref().clone() } }
    } else {
        rsx! {}
    };
    let row_wrap_class = row_wrap_class(frozen);

    rsx! {
        div { class: "{row_wrap_class}",
            {value_cell}
            {chevron}
            {chain_block}
        }
    }
}

/// Divider strip between IN and OUT sections.
#[component]
pub(super) fn DividerStrip() -> Element {
    rsx! {
        div { class: "if-editor__readout-divider" }
    }
}

fn toggle_output_expanded(state: &mut ExpandState, idx: usize) {
    if state.per_output.len() <= idx {
        state.per_output.resize(idx + 1, false);
    }
    state.per_output[idx] = !state.per_output[idx];
}

fn output_row_label(index: usize, has_multiple_outputs: bool) -> String {
    if has_multiple_outputs {
        format!("OUT {}", index + 1)
    } else {
        "OUT".to_owned()
    }
}

fn row_wrap_class(frozen: bool) -> &'static str {
    if frozen {
        "if-editor__readout-row-wrap if-editor__readout-row-wrap--frozen"
    } else {
        "if-editor__readout-row-wrap"
    }
}

fn chevron_icon(expanded: bool) -> IconKind {
    if expanded {
        IconKind::ChevronDown
    } else {
        IconKind::ChevronRight
    }
}

fn chevron_label(expanded: bool) -> &'static str {
    if expanded {
        "Collapse output merge preview"
    } else {
        "Expand output merge preview"
    }
}

fn keyboard_row_class(frozen: bool) -> &'static str {
    if frozen {
        "if-editor__readout-row if-editor__readout-row--kb if-editor__readout-row--frozen"
    } else {
        "if-editor__readout-row if-editor__readout-row--kb"
    }
}

fn keyboard_chip_class(live: bool) -> &'static str {
    if live {
        "if-editor__readout-kb-chip if-editor__readout-kb-chip--live"
    } else {
        "if-editor__readout-kb-chip if-editor__readout-kb-chip--idle"
    }
}

fn format_behavior(behavior: OutputBehavior) -> &'static str {
    match behavior {
        OutputBehavior::Hold => "Hold",
        OutputBehavior::Pulse => "Pulse",
    }
}

#[cfg(test)]
mod tests {
    use super::super::value_helpers::hat_glyph_for;
    use super::*;
    use inputforge_core::types::HatDirection;

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
    fn keyboard_row_classes_include_kind_and_frozen_state() {
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
    fn keyboard_chip_class_tracks_live_state() {
        assert_eq!(
            keyboard_chip_class(true),
            "if-editor__readout-kb-chip if-editor__readout-kb-chip--live"
        );
        assert_eq!(
            keyboard_chip_class(false),
            "if-editor__readout-kb-chip if-editor__readout-kb-chip--idle"
        );
    }

    #[test]
    fn expand_state_applies_per_output_state() {
        let state = ExpandState {
            per_output: vec![false, true],
        };

        assert!(!state.is_expanded(0));
        assert!(state.is_expanded(1));
        assert!(!state.is_expanded(2));
    }

    #[test]
    fn toggle_output_expanded_resizes_and_flips_one_index() {
        let mut state = ExpandState::default();

        toggle_output_expanded(&mut state, 2);

        assert_eq!(state.per_output, vec![false, false, true]);
        assert!(state.is_expanded(2));

        toggle_output_expanded(&mut state, 2);

        assert_eq!(state.per_output, vec![false, false, false]);
        assert!(!state.is_expanded(2));
    }

    #[test]
    fn row_wrap_class_and_chevron_state_track_state() {
        assert_eq!(row_wrap_class(false), "if-editor__readout-row-wrap");
        assert_eq!(
            row_wrap_class(true),
            "if-editor__readout-row-wrap if-editor__readout-row-wrap--frozen"
        );
        assert_eq!(chevron_icon(false), IconKind::ChevronRight);
        assert_eq!(chevron_icon(true), IconKind::ChevronDown);
        assert_eq!(chevron_label(false), "Expand output merge preview");
        assert_eq!(chevron_label(true), "Collapse output merge preview");
    }
}
