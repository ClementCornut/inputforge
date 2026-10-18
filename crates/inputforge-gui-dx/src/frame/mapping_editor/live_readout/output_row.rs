//! A terminal output readout with optional chain expansion.
use super::presentation::{
    chevron_icon, chevron_label, format_behavior, keyboard_row_class, row_wrap_class,
};
use crate::frame::mapping_editor::live_readout::{
    in_block::{ButtonReadoutRow, HatReadoutRow, ReadoutRow},
    value_helpers::{ReadoutDisplay, format_key_combo, format_output_label},
};
use crate::readouts::{ExpandState, Expansion, Key, OutputDescriptor, OutputDestination};
use dioxus::prelude::*;
use inputforge_core::types::OutputId;
use std::rc::Rc;
#[component]
pub(super) fn OutRow(
    descriptor: Rc<OutputDescriptor>,
    idx: usize,
    has_multiple_outputs: bool,
    expand_state: Signal<ExpandState>,
    engine_running: bool,
    display: ReadoutDisplay,
    readout_key: Key,
) -> Element {
    let bindings = use_context::<crate::mapping_edits::Bindings>().readouts;
    let frozen = !engine_running || !descriptor.is_active;
    let has_chain = !descriptor.chain.is_empty();
    let expanded = expand_state.read().is_expanded(idx);
    let chevron_icon = chevron_icon(expanded);
    let chevron_label = chevron_label(expanded);
    let row_label = output_row_label(idx, has_multiple_outputs);

    let value_cell = match &descriptor.destination {
        OutputDestination::VJoy(out) => match out.output {
            OutputId::Axis { .. } => {
                let tag = format_output_label(out);
                let ReadoutDisplay::Axis(display) = display else {
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
                let tag = format_output_label(out);
                let ReadoutDisplay::Button { pressed } = display else {
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
                let tag = format_output_label(out);
                let ReadoutDisplay::Hat { direction } = display else {
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
        OutputDestination::Keyboard { key, behavior, .. } => {
            let combo_text = format_key_combo(key);
            let tag = format!("Keyboard - {}", format_behavior(*behavior));
            let key_live = matches!(display, ReadoutDisplay::Button { pressed: true });
            let row_class = keyboard_row_class(frozen);
            let label = row_label.clone();

            rsx! {
                div { class: "{row_class}",
                    div { class: "if-editor__readout-label", "{label}" }
                    div { class: "if-editor__readout-tag", "{tag}" }
                    div { class: "if-editor__readout-kb-cell",
                        crate::components::Badge { variant: if key_live { crate::components::BadgeVariant::Success } else { crate::components::BadgeVariant::Neutral }, "{combo_text}" }
                    }
                    div { class: "if-editor__readout-pct" }
                }
            }
        }
        OutputDestination::Mouse {
            target, behavior, ..
        } => {
            let is_live = matches!(display, ReadoutDisplay::Button { pressed: true });
            let tag = if target.is_wheel() {
                target.label().to_owned()
            } else {
                format!("{} - {}", target.label(), format_behavior(*behavior))
            };

            rsx! {
                ButtonReadoutRow {
                    label: row_label.clone(),
                    tag,
                    pressed:is_live,
                    frozen,
                }
            }
        }
    };

    let chevron = if has_chain {
        let outputs = expand_state.peek().per_output.len();
        let onclick = move |_| {
            let next = bindings.expand.call(Expansion {
                state: expand_state.peek().clone(),
                key: readout_key.clone(),
                outputs,
                toggle: Some(idx),
            });
            expand_state.set(next);
        };
        rsx! {
            crate::components::IconButton {
                class: "if-editor__readout-chevron", size: crate::components::ButtonSize::Sm,
                variant: crate::components::ButtonVariant::Ghost,
                label: chevron_label, aria_expanded: expanded, icon: chevron_icon, onclick,
            }
        }
    } else {
        rsx! { div { class: "if-editor__readout-chevron-spacer" } }
    };

    let chain_block = if expanded && has_chain {
        rsx! { super::super::out_chain::OutChain { descriptor: descriptor.as_ref().clone() } }
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
pub(in crate::frame::mapping_editor::live_readout) fn DividerStrip() -> Element {
    rsx! {
        div { class: "if-editor__readout-divider" }
    }
}

fn output_row_label(index: usize, has_multiple_outputs: bool) -> String {
    if has_multiple_outputs {
        format!("OUT {}", index + 1)
    } else {
        "OUT".to_owned()
    }
}
