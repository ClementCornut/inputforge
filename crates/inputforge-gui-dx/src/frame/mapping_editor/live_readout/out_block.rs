//! OUT-side rows for terminal live readout destinations.
//!
//! This module renders vJoy axis and button outputs as regular readout
//! rows. vJoy hats and keyboard combos use destination-specific rows
//! that preserve the surrounding section structure.

use std::rc::Rc;

use dioxus::prelude::*;
#[path = "output_row.rs"]
mod output_row;
pub(super) use output_row::DividerStrip;
use output_row::OutRow;
#[path = "out_block_presentation.rs"]
mod presentation;

use super::value_helpers::ReadoutDisplay;
use crate::readouts::{LiveReadoutModel, OutputDescriptor};

const READOUT_SECTION_CLASS: &str = "if-editor__readout-section";
const READOUT_SECTION_LABEL_CLASS: &str = "if-editor__readout-section-label";
const READOUT_GROUP_CLASS: &str = "if-editor__readout-group";

use crate::readouts::{ExpandState, Key};
/// High-level OUT side of the live readout.
#[component]
pub(super) fn OutBlock(
    model: LiveReadoutModel,
    expand_state: Signal<ExpandState>,
    engine_running: bool,
    values: Vec<ReadoutDisplay>,
    readout_key: Key,
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
                        display:values[idx],
                        readout_key:readout_key.clone(),
                    }
                }
            }
        }
    }
}
