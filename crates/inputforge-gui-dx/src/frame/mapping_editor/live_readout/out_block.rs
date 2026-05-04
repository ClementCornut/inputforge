//! OUT-side rows for terminal live readout destinations.
//!
//! This module renders vJoy axis and button outputs as regular readout
//! rows. vJoy hats and keyboard combos use destination-specific rows
//! that preserve the surrounding section structure.

use std::rc::Rc;

use dioxus::prelude::*;

use inputforge_core::types::{AxisPolarity, HatDirection, OutputId};

use crate::components::Icon;
use crate::context::AppContext;
use crate::icons::{Icon as IconKind, IconSize};

use super::analyzer::{LiveReadoutModel, OutputDescriptor, OutputDestination};
use super::in_block::ReadoutRow;
use super::value_helpers::{
    AxisDisplay, format_key_combo, format_output_label, read_output_button, read_output_display,
    read_output_hat,
};

const READOUT_SECTION_CLASS: &str = "if-editor__readout-section";
const READOUT_SECTION_LABEL_CLASS: &str = "if-editor__readout-section-label";
const READOUT_GROUP_CLASS: &str = "if-editor__readout-group";

/// Per-readout expand state for output chains.
///
/// `per_output` is index-aligned with `LiveReadoutModel::outputs`.
/// `expand_all` acts as a global override for every output row.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ExpandState {
    pub expand_all: bool,
    pub per_output: Vec<bool>,
}

impl ExpandState {
    /// Return whether the output at `idx` is currently expanded.
    pub(super) fn is_expanded(&self, idx: usize) -> bool {
        self.expand_all || self.per_output.get(idx).copied().unwrap_or(false)
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

    rsx! {
        div { class: "{READOUT_SECTION_CLASS}",
            div { class: "{READOUT_SECTION_LABEL_CLASS}", "OUT" }
            div { class: "{READOUT_GROUP_CLASS}",
                for (idx, descriptor) in outputs.iter().cloned().enumerate() {
                    OutRow {
                        key: "output-{idx}",
                        descriptor,
                        idx,
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
    expand_state: Signal<ExpandState>,
    engine_running: bool,
) -> Element {
    let ctx = use_context::<AppContext>();
    let frozen = !engine_running || !descriptor.is_active;
    let has_chain = !descriptor.chain.is_empty();
    let expanded = expand_state.read().is_expanded(idx);
    let chevron_icon = chevron_icon(expanded);
    let chevron_label = chevron_label(expanded);

    let value_cell = match &descriptor.destination {
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
pub(super) fn DividerStrip(model: LiveReadoutModel, expand_state: Signal<ExpandState>) -> Element {
    let any_expandable = model.outputs.iter().any(|o| !o.chain.is_empty());
    let outputs_len = model.outputs.len();
    let expanded_now = expand_state.read().expand_all;
    let button_text = if expanded_now {
        "collapse all"
    } else {
        "expand all"
    };
    let onclick = move |_| {
        expand_state.with_mut(|s| {
            set_all_expanded(s, outputs_len, !s.expand_all);
        });
    };

    rsx! {
        div { class: "if-editor__readout-divider",
            span { class: "if-editor__readout-divider-spacer" }
            if any_expandable {
                button {
                    class: "if-editor__readout-expand-all",
                    "type": "button",
                    onclick,
                    "{button_text}"
                }
            }
        }
    }
}

fn toggle_output_expanded(state: &mut ExpandState, idx: usize) {
    if state.per_output.len() <= idx {
        state.per_output.resize(idx + 1, false);
    }
    state.per_output[idx] = !state.per_output[idx];
}

fn set_all_expanded(state: &mut ExpandState, outputs_len: usize, expanded: bool) {
    state.expand_all = expanded;
    state.per_output = vec![expanded; outputs_len];
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

    #[test]
    fn expand_state_applies_global_override_and_per_output_state() {
        let mut state = ExpandState {
            expand_all: false,
            per_output: vec![false, true],
        };

        assert!(!state.is_expanded(0));
        assert!(state.is_expanded(1));
        assert!(!state.is_expanded(2));

        state.expand_all = true;

        assert!(state.is_expanded(0));
        assert!(state.is_expanded(1));
        assert!(state.is_expanded(2));
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
    fn set_all_expanded_syncs_global_and_per_output_state() {
        let mut state = ExpandState {
            expand_all: false,
            per_output: vec![true],
        };

        set_all_expanded(&mut state, 3, true);

        assert!(state.expand_all);
        assert_eq!(state.per_output, vec![true, true, true]);

        set_all_expanded(&mut state, 2, false);

        assert!(!state.expand_all);
        assert_eq!(state.per_output, vec![false, false]);
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
