// Rust guideline compliant 2026-05-01

//! Editor header: h2 mapping name + subtitle line.
//!
//! Renders the mapping name as an `<h2>` (wrapped in a tooltip that shows
//! the full name on hover, per spec line 36), and a monospace subtitle line
//! reading `<source-label>` when no `MapToVJoy` action is present, or
//! `<source-label>  →  <output-label>` when one is found (DFS pre-order
//! through `Conditional` branches).

use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::types::{InputAddress, OutputAddress, OutputId, VJoyAxis};

use crate::components::{Tooltip, TooltipPlacement};
use crate::context::AppContext;
use crate::frame::mapping_list::source_label;

#[component]
pub(crate) fn Header(name: String, input: InputAddress) -> Element {
    let ctx = use_context::<AppContext>();
    let cfg = ctx.config.read();
    let src = source_label::format(&input, &cfg);

    let output_label = cfg
        .selected_mapping_actions
        .as_ref()
        .and_then(|actions| first_map_to_vjoy_label(actions));

    rsx! {
        div { class: "if-editor__header",
            Tooltip {
                content: name.clone(),
                placement: TooltipPlacement::Bottom,
                h2 { class: "if-editor__title", "{name}" }
            }
            div { class: "if-editor__subtitle",
                "{src}"
                if let Some(out) = output_label {
                    span { class: "if-editor__subtitle-arrow",
                        "\u{00a0}\u{00a0}\u{2192}\u{00a0}\u{00a0}"
                    }
                    "{out}"
                }
            }
        }
    }
}

/// Walk the action tree (DFS pre-order, including `Conditional` branches)
/// and return the formatted label for the first `MapToVJoy` found.
fn first_map_to_vjoy_label(actions: &[Action]) -> Option<String> {
    fn walk(actions: &[Action]) -> Option<&OutputAddress> {
        for action in actions {
            match action {
                Action::MapToVJoy { output } => return Some(output),
                Action::Conditional {
                    if_true, if_false, ..
                } => {
                    if let Some(found) = walk(if_true) {
                        return Some(found);
                    }
                    if let Some(branch) = if_false.as_deref() {
                        if let Some(found) = walk(branch) {
                            return Some(found);
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }
    walk(actions).map(format_output_label)
}

fn format_output_label(output: &OutputAddress) -> String {
    let suffix = match output.output {
        OutputId::Axis { id } => match id {
            VJoyAxis::X => "X axis",
            VJoyAxis::Y => "Y axis",
            VJoyAxis::Z => "Z axis",
            VJoyAxis::Rx => "Rx axis",
            VJoyAxis::Ry => "Ry axis",
            VJoyAxis::Rz => "Rz axis",
            VJoyAxis::Slider0 => "Slider 0",
            VJoyAxis::Slider1 => "Slider 1",
        }
        .to_owned(),
        OutputId::Button { id } => format!("Button {id}"),
        OutputId::Hat { id } => format!("Hat {id}"),
    };
    format!("vJoy {} \u{00b7} {}", output.device, suffix)
}
