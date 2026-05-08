use dioxus::prelude::*;

use inputforge_core::types::{AxisPolarity, HatDirection};

use crate::context::AppContext;
use crate::frame::mapping_list::source_label;

use super::analyzer::{LiveReadoutModel, PredicateDescriptor, PredicateKind};
use super::predicate::render_hat_glyphs;
use super::value_helpers::{
    AxisDisplay, ReadoutDisplay, format_percentage, hat_direction_label, hat_glyph_for,
    read_input_display,
};

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
    }
}

/// One binary button row in the readout grid.
#[component]
pub(super) fn ButtonReadoutRow(label: String, tag: String, pressed: bool, frozen: bool) -> Element {
    let row_class = button_row_class(frozen);
    let cell_class = button_cell_class(frozen);
    let dot_class = button_dot_class(pressed);
    let pill_class = button_pill_class(pressed);
    let state_label = button_state_label(pressed);

    rsx! {
        div { class: "{row_class}",
            div { class: "if-editor__readout-label", "{label}" }
            div { class: "if-editor__readout-tag", "{tag}" }
            div { class: "{cell_class}",
                span { class: "{dot_class}" }
                span { class: "{pill_class}", "{state_label}" }
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

fn button_row_class(frozen: bool) -> &'static str {
    if frozen {
        "if-editor__readout-row if-editor__readout-row--button if-editor__readout-row--frozen"
    } else {
        "if-editor__readout-row if-editor__readout-row--button"
    }
}

fn button_cell_class(frozen: bool) -> &'static str {
    if frozen {
        "if-editor__readout-button-cell if-editor__readout-button-cell--frozen"
    } else {
        "if-editor__readout-button-cell"
    }
}

fn button_dot_class(pressed: bool) -> &'static str {
    if pressed {
        "if-editor__readout-button-dot if-editor__readout-button-dot--live"
    } else {
        "if-editor__readout-button-dot"
    }
}

fn button_pill_class(pressed: bool) -> &'static str {
    if pressed {
        "if-editor__readout-button-pill if-editor__readout-button-pill--live"
    } else {
        "if-editor__readout-button-pill if-editor__readout-button-pill--idle"
    }
}

fn button_state_label(pressed: bool) -> &'static str {
    if pressed { "Pressed" } else { "Released" }
}

fn hat_row_class(frozen: bool) -> &'static str {
    if frozen {
        "if-editor__readout-row if-editor__readout-row--hat if-editor__readout-row--frozen"
    } else {
        "if-editor__readout-row if-editor__readout-row--hat"
    }
}

fn hat_compass_class(frozen: bool) -> &'static str {
    if frozen {
        "if-editor__readout-hat-compass if-editor__readout-hat-compass--frozen"
    } else {
        "if-editor__readout-hat-compass"
    }
}

fn hat_spoke_class(current: HatDirection, spoke: HatDirection) -> &'static str {
    if current == spoke {
        "if-editor__readout-hat-spoke if-editor__readout-hat-spoke--live"
    } else {
        "if-editor__readout-hat-spoke"
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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(
            READOUT_PREDICATE_LABEL_CLASS,
            "if-editor__readout-predicate-label"
        );
        assert_eq!(READOUT_CHIPS_CLASS, "if-editor__readout-chips");
    }

    #[test]
    fn predicate_chips_align_to_readout_grid() {
        let css = include_str!("../../../../assets/frame/mapping_editor.css");

        assert!(
            css_rule_contains(
                css,
                ".if-editor__readout-predicate-label",
                "grid-column: 1;"
            ),
            "predicate rows must reserve the readout label column"
        );
        assert!(
            css_rule_contains(css, ".if-editor__readout-chips", "grid-column: 2 / -1;"),
            "predicate chips must align to the shared readout grid instead of floating from an arbitrary offset"
        );
        assert!(
            !css_rule_contains(css, ".if-editor__readout-chips", "padding-left"),
            "predicate chips must not use old fixed padding; it drifts from the readout grid"
        );
    }

    #[test]
    fn readout_cells_claim_stable_grid_columns() {
        let css = include_str!("../../../../assets/frame/mapping_editor.css");
        for (selector, declaration) in [
            (".if-editor__readout-label", "grid-column: 1;"),
            (".if-editor__readout-tag", "grid-column: 2;"),
            (".if-editor__readout-bar", "grid-column: 3;"),
            (".if-editor__readout-hat-glyph", "grid-column: 3;"),
            (".if-editor__readout-kb-cell", "grid-column: 3;"),
            (".if-editor__readout-pct", "grid-column: 4;"),
            (".if-editor__readout-chevron", "grid-column: 5;"),
            (".if-editor__readout-chevron-spacer", "grid-column: 5;"),
        ] {
            assert!(
                css_rule_contains(css, selector, declaration),
                "{selector} must declare {declaration} so display: contents rows restart on the same readout grid columns"
            );
        }
    }

    #[test]
    fn readout_groups_share_one_scale_grid() {
        let css = include_str!("../../../../assets/frame/mapping_editor.css");
        for declaration in [
            "--if-editor__readout-label-col: 60px;",
            "--if-editor__readout-tag-col: clamp(120px, 18vw, 260px);",
            "--if-editor__readout-pct-col: 60px;",
            "--if-editor__readout-chevron-col: 24px;",
        ] {
            assert!(
                css_rule_contains(css, ".if-editor__readout", declaration),
                ".if-editor__readout must define {declaration} so IN and OUT readout bars share one scale"
            );
        }

        assert!(
            css_rule_contains(
                css,
                ".if-editor__readout-group",
                "grid-template-columns: var(--if-editor__readout-label-col) var(--if-editor__readout-tag-col) minmax(0, 1fr) var(--if-editor__readout-pct-col) var(--if-editor__readout-chevron-col);"
            ),
            ".if-editor__readout-group must reserve identical label, tag, bar, percentage, and chevron columns"
        );
        assert!(
            !css_rule_contains(css, ".if-editor__readout-group", "max-content"),
            ".if-editor__readout-group must not auto-size columns per group; that makes IN and OUT bars misalign"
        );
        assert!(
            css_rule_contains(css, ".if-editor__readout-tag", "min-width: 0;"),
            ".if-editor__readout-tag must truncate inside the shared tag column instead of expanding the scale"
        );
    }

    #[test]
    fn hat_compass_center_spoke_uses_compact_glyph() {
        let html = dioxus_ssr::render_element(rsx! {
            HatReadoutRow {
                label: "IN".to_owned(),
                tag: "Stick · Hat 0".to_owned(),
                direction: HatDirection::Center,
                frozen: false,
            }
        });

        assert!(
            html.contains("if-editor__readout-hat-state"),
            "hat row should keep the readable direction label outside the 3x3 spokes: {html}"
        );
        assert!(
            html.contains("Center \u{00b7}"),
            "readable state text should still name the centered direction: {html}"
        );
        assert!(
            !html.contains("if-editor__readout-hat-spoke\">Center</span>")
                && !html.contains(
                    "if-editor__readout-hat-spoke if-editor__readout-hat-spoke--live\">Center</span>"
                ),
            "the fixed 20px center spoke must not render the full Center label: {html}"
        );
    }

    fn css_rule_contains(css: &str, selector: &str, declaration: &str) -> bool {
        css.split('}').any(|block| {
            let Some((selectors, body)) = block.split_once('{') else {
                return false;
            };
            selectors
                .split(',')
                .any(|candidate| candidate.trim().ends_with(selector))
                && body.contains(declaration)
        })
    }
}
