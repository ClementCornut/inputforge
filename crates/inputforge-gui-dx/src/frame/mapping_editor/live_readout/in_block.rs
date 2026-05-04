use dioxus::prelude::*;

use inputforge_core::types::AxisPolarity;

use crate::context::AppContext;
use crate::frame::mapping_list::source_label;

use super::analyzer::{LiveReadoutModel, PredicateDescriptor, PredicateKind};
use super::predicate::render_hat_glyphs;
use super::value_helpers::{AxisDisplay, format_percentage, read_axis_display};

const READOUT_SECTION_CLASS: &str = "if-editor__readout-section";
const READOUT_SECTION_LABEL_CLASS: &str = "if-editor__readout-section-label";
const READOUT_GROUP_CLASS: &str = "if-editor__readout-group";
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
                    ReadoutRow {
                        key: "pipeline-{idx}",
                        label: input_row_label(idx, has_multiple_inputs),
                        tag: source_label::format(addr, &cfg),
                        display: read_axis_display(addr, &live, &cfg),
                        frozen: false,
                    }
                }
            }
        }
        if has_predicates {
            div { class: "{READOUT_SECTION_CLASS}",
                div { class: "{READOUT_SECTION_LABEL_CLASS}", "IN \u{00b7} predicates" }
                div { class: "{READOUT_CHIPS_CLASS}",
                    for (idx, predicate) in model.predicates.iter().enumerate() {
                        {
                            let chip_class = predicate_chip_class(predicate.state);
                            let dot_class = predicate_dot_class(predicate.state);
                            let label = predicate_chip_label(predicate);
                            rsx! {
                                div { key: "predicate-{idx}", class: "{chip_class}",
                                    span { class: "{dot_class}" }
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

fn input_row_label(index: usize, has_multiple_inputs: bool) -> String {
    if has_multiple_inputs {
        format!("IN {}", index + 1)
    } else {
        "IN".to_owned()
    }
}

fn predicate_chip_class(state: bool) -> &'static str {
    if state {
        "if-editor__readout-chip if-editor__readout-chip--live"
    } else {
        "if-editor__readout-chip if-editor__readout-chip--idle"
    }
}

fn predicate_dot_class(state: bool) -> &'static str {
    if state {
        "if-editor__readout-chip-dot"
    } else {
        "if-editor__readout-chip-dot if-editor__readout-chip-dot--hollow"
    }
}

fn predicate_chip_label(predicate: &PredicateDescriptor) -> String {
    match &predicate.kind {
        PredicateKind::ButtonPressed => predicate.label.clone(),
        PredicateKind::ButtonReleased => format!("{} (released)", predicate.label),
        PredicateKind::AxisInRange { min, max } => {
            format!("{} [{min:.2}..{max:.2}]", predicate.label)
        }
        PredicateKind::HatDirection { directions } => {
            format!("{} {}", predicate.label, render_hat_glyphs(directions))
        }
    }
}

/// One row in the readout grid: label, tag, bar, percentage text.
///
/// The bar fill is anchored at 50% for bipolar axes and at 0% for
/// unipolar axes, matching the live-green visual described in the spec.
/// Bipolar bars also carry a modifier class so the CSS can draw a center
/// tick that communicates polarity at idle.
///
/// `frozen` is true only on the OUT row when the engine is not running.
/// CSS dims the bar fill and percentage to signal a held value; label
/// and tag stay at full strength because they describe configuration.
#[component]
pub(super) fn ReadoutRow(
    label: String,
    tag: String,
    display: AxisDisplay,
    frozen: bool,
) -> Element {
    let pct_text = format_percentage(&display);
    let bipolar = matches!(display.polarity, AxisPolarity::Bipolar);

    // Bipolar bars are anchored at the 50% center and grow toward one
    // edge, so the visual maximum is half the container width. Unipolar
    // bars grow from the left edge across the full width.
    let fill_pct = if bipolar {
        (display.value.abs() * 50.0).clamp(0.0, 50.0)
    } else {
        (display.value.abs() * 100.0).clamp(0.0, 100.0)
    };

    // Always set both `left` and `right`, with `auto` for the off side,
    // so Dioxus's per-property style diffing cannot leave a stale anchor
    // from a previous render. Without this, switching from negative to
    // positive or vice versa keeps the prior side's `50%` set. CSS then
    // resolves `left: 50%; right: 50%; width: <pct>%;` by honoring
    // `left + width` regardless of sign, so both signs grow rightward and
    // overflow the container.
    let bar_style = if bipolar && display.value < 0.0 {
        format!("left: auto; right: 50%; width: {fill_pct}%;")
    } else if bipolar {
        format!("left: 50%; right: auto; width: {fill_pct}%;")
    } else {
        format!("left: 0; right: auto; width: {fill_pct}%;")
    };

    let bar_class = if bipolar {
        "if-editor__readout-bar if-editor__readout-bar--bipolar"
    } else {
        "if-editor__readout-bar"
    };

    let row_class = if frozen {
        "if-editor__readout-row if-editor__readout-row--frozen"
    } else {
        "if-editor__readout-row"
    };

    rsx! {
        div { class: "{row_class}",
            div { class: "if-editor__readout-label", "{label}" }
            div { class: "if-editor__readout-tag", "{tag}" }
            div { class: "{bar_class}",
                div {
                    class: "if-editor__readout-fill",
                    style: "{bar_style}",
                }
            }
            div { class: "if-editor__readout-pct", "{pct_text}" }
        }
    }
}

/// Section divider with an inline label.
///
/// Renders as a single grid cell that spans the full row and contains
/// dashed lines on either side of a small uppercase label, marking the
/// transition between the input section and the merged-or-output section.
#[component]
pub(super) fn ReadoutDivider(label: String) -> Element {
    rsx! {
        div { class: "if-editor__readout-divider",
            span { class: "if-editor__readout-divider-label", "{label}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use inputforge_core::types::HatDirection;

    fn predicate(kind: PredicateKind) -> PredicateDescriptor {
        PredicateDescriptor {
            kind,
            inputs: Vec::new(),
            state: false,
            label: "Button 1".to_owned(),
        }
    }

    #[test]
    fn predicate_chip_label_adds_variant_suffixes() {
        assert_eq!(
            predicate_chip_label(&predicate(PredicateKind::ButtonPressed)),
            "Button 1"
        );
        assert_eq!(
            predicate_chip_label(&predicate(PredicateKind::ButtonReleased)),
            "Button 1 (released)"
        );
        assert_eq!(
            predicate_chip_label(&predicate(PredicateKind::AxisInRange {
                min: -0.25,
                max: 0.75,
            })),
            "Button 1 [-0.25..0.75]"
        );
        assert_eq!(
            predicate_chip_label(&predicate(PredicateKind::HatDirection {
                directions: vec![HatDirection::E, HatDirection::N],
            })),
            "Button 1 \u{2191}\u{2192}"
        );
    }

    #[test]
    fn predicate_chip_classes_follow_live_state() {
        assert_eq!(
            predicate_chip_class(true),
            "if-editor__readout-chip if-editor__readout-chip--live"
        );
        assert_eq!(
            predicate_chip_class(false),
            "if-editor__readout-chip if-editor__readout-chip--idle"
        );
        assert_eq!(predicate_dot_class(true), "if-editor__readout-chip-dot");
        assert_eq!(
            predicate_dot_class(false),
            "if-editor__readout-chip-dot if-editor__readout-chip-dot--hollow"
        );
    }

    #[test]
    fn section_classes_match_readout_css_contract() {
        assert_eq!(READOUT_SECTION_CLASS, "if-editor__readout-section");
        assert_eq!(
            READOUT_SECTION_LABEL_CLASS,
            "if-editor__readout-section-label"
        );
        assert_eq!(READOUT_CHIPS_CLASS, "if-editor__readout-chips");
    }
}
