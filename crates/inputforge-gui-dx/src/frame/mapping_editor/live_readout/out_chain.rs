// Rust guideline compliant 2026-05-04

//! Expanded-chain rows for terminal OUT destinations.
//!
//! The chain block renders the merge and conditional steps that explain
//! how a terminal OUT row was reached.

use dioxus::prelude::*;

use inputforge_core::processing::into_natural_domain;
use inputforge_core::types::{AxisPolarity, MergeOp};

use crate::context::AppContext;
use crate::frame::mapping_list::source_label;

use super::analyzer::{Branch, ChainStep, OutputDescriptor};
use super::value_helpers::{AxisDisplay, format_percentage};

/// Indented chain block for one output descriptor.
///
/// Merge rows use each step's running polarity, not the terminal output
/// polarity, so intermediate bars stay faithful when polarity promotes
/// partway through a chain.
#[component]
pub(super) fn OutChain(descriptor: OutputDescriptor) -> Element {
    let chain = descriptor.chain.clone();

    rsx! {
        div { class: "if-editor__readout-chain",
            for (idx, step) in chain.iter().cloned().enumerate() {
                ChainRow {
                    key: "{idx}",
                    step,
                    index_in_chain: idx,
                }
            }
        }
    }
}

#[component]
fn ChainRow(step: ChainStep, index_in_chain: usize) -> Element {
    let ctx = use_context::<AppContext>();

    match step {
        ChainStep::Merge {
            operation,
            secondary_input,
            encoded_value,
            polarity_at_step,
        } => {
            let cfg = ctx.config.read();
            let merge_n = index_in_chain + 1;
            let partner_label = source_label::format(&secondary_input, &cfg);
            let display = AxisDisplay {
                value: into_natural_domain(encoded_value, polarity_at_step),
                polarity: polarity_at_step,
            };
            let pct = format_percentage(&display);
            let bar_class = chain_bar_class(polarity_at_step);
            let bar_style = chain_bar_style(&display);
            let op_label = merge_operation_label(operation);
            let merge_tag = format!("{partner_label} \u{00b7} {op_label}");

            rsx! {
                div { class: "if-editor__readout-chain-row",
                    span { class: "if-editor__readout-chain-step", "MERGE {merge_n}" }
                    span { class: "if-editor__readout-chain-tag", "{merge_tag}" }
                    div { class: "{bar_class}",
                        div {
                            class: "if-editor__readout-chain-fill",
                            style: "{bar_style}",
                        }
                    }
                    span { class: "if-editor__readout-chain-pct", "{pct}" }
                }
            }
        }
        ChainStep::Conditional {
            condition_label,
            evaluated,
            branch,
        } => {
            let active = conditional_active(evaluated, branch);
            let outcome = if active {
                "active branch"
            } else {
                "inactive branch"
            };
            let row_class = conditional_row_class(active);
            let outcome_class = conditional_outcome_class(active);
            let outcome_text = format!("\u{2192} {outcome}");

            rsx! {
                div { class: "{row_class}",
                    span { class: "if-editor__readout-chain-step", "COND" }
                    span { class: "if-editor__readout-chain-tag", "{condition_label}" }
                    span { class: "{outcome_class}", "{outcome_text}" }
                }
            }
        }
    }
}

fn conditional_active(evaluated: bool, branch: Branch) -> bool {
    evaluated == matches!(branch, Branch::IfTrue)
}

fn conditional_row_class(active: bool) -> &'static str {
    if active {
        "if-editor__readout-chain-row is-cond"
    } else {
        "if-editor__readout-chain-row is-cond is-inactive"
    }
}

fn merge_operation_label(operation: MergeOp) -> &'static str {
    match operation {
        MergeOp::Bidirectional => "bidirectional",
        MergeOp::Average => "average",
        MergeOp::Maximum => "maximum",
    }
}

fn chain_bar_style(display: &AxisDisplay) -> String {
    let bipolar = matches!(display.polarity, AxisPolarity::Bipolar);
    let fill_pct = if bipolar {
        (display.value.abs() * 50.0).clamp(0.0, 50.0)
    } else {
        (display.value.abs() * 100.0).clamp(0.0, 100.0)
    };

    if bipolar && display.value < 0.0 {
        format!("left: auto; right: 50%; width: {fill_pct}%;")
    } else if bipolar {
        format!("left: 50%; right: auto; width: {fill_pct}%;")
    } else {
        format!("left: 0; right: auto; width: {fill_pct}%;")
    }
}

fn chain_bar_class(polarity: AxisPolarity) -> &'static str {
    if matches!(polarity, AxisPolarity::Bipolar) {
        "if-editor__readout-chain-bar if-editor__readout-chain-bar--bipolar"
    } else {
        "if-editor__readout-chain-bar"
    }
}

fn conditional_outcome_class(active: bool) -> &'static str {
    if active {
        "if-editor__readout-chain-outcome if-editor__readout-chain-outcome--active"
    } else {
        "if-editor__readout-chain-outcome if-editor__readout-chain-outcome--inactive"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conditional_active_matches_branch_outcome() {
        assert!(conditional_active(true, Branch::IfTrue));
        assert!(conditional_active(false, Branch::IfFalse));
        assert!(!conditional_active(false, Branch::IfTrue));
        assert!(!conditional_active(true, Branch::IfFalse));
    }

    #[test]
    fn conditional_row_class_uses_real_state_classes() {
        assert_eq!(
            conditional_row_class(true),
            "if-editor__readout-chain-row is-cond"
        );
        assert_eq!(
            conditional_row_class(false),
            "if-editor__readout-chain-row is-cond is-inactive"
        );
    }

    #[test]
    fn merge_operation_label_is_stable_ui_copy() {
        assert_eq!(
            merge_operation_label(MergeOp::Bidirectional),
            "bidirectional"
        );
        assert_eq!(merge_operation_label(MergeOp::Average), "average");
        assert_eq!(merge_operation_label(MergeOp::Maximum), "maximum");
    }

    #[test]
    fn chain_bar_style_anchors_by_polarity_and_sign() {
        assert_eq!(
            chain_bar_style(&AxisDisplay {
                value: -0.5,
                polarity: AxisPolarity::Bipolar,
            }),
            "left: auto; right: 50%; width: 25%;"
        );
        assert_eq!(
            chain_bar_style(&AxisDisplay {
                value: 0.5,
                polarity: AxisPolarity::Bipolar,
            }),
            "left: 50%; right: auto; width: 25%;"
        );
        assert_eq!(
            chain_bar_style(&AxisDisplay {
                value: 0.5,
                polarity: AxisPolarity::Unipolar,
            }),
            "left: 0; right: auto; width: 50%;"
        );
    }

    #[test]
    fn chain_bar_class_marks_bipolar_rows() {
        assert_eq!(
            chain_bar_class(AxisPolarity::Bipolar),
            "if-editor__readout-chain-bar if-editor__readout-chain-bar--bipolar"
        );
        assert_eq!(
            chain_bar_class(AxisPolarity::Unipolar),
            "if-editor__readout-chain-bar"
        );
    }

    #[test]
    fn chain_preview_rows_share_parent_readout_scale() {
        let css = include_str!("../../../../assets/frame/mapping_editor.css");

        assert!(
            css.contains(
                ".if-editor__readout-chain {\n    grid-column: 1 / -1;\n    display: grid;"
            ),
            "expanded chain block must establish a grid aligned to the parent readout"
        );
        assert!(
            css.contains(
                "grid-template-columns: var(--if-editor__readout-label-col) \
                 var(--if-editor__readout-tag-col) minmax(0, 1fr) \
                 var(--if-editor__readout-pct-col) \
                 var(--if-editor__readout-chevron-col);"
            ),
            "chain preview rows must use the parent readout column tokens"
        );
        for declaration in [
            ".if-editor__readout-chain-row {\n    display: contents;",
            ".if-editor__readout-chain-bar {\n    grid-column: 3;",
            ".if-editor__readout-chain-pct {\n    grid-column: 4;",
            ".if-editor__readout-chain-outcome {\n    grid-column: 3 / 5;",
            ".if-editor__readout-chain-step {\n    grid-column: 1;\n    padding-left: 8px;",
            "white-space: nowrap;",
        ] {
            assert!(
                css.contains(declaration),
                "missing chain alignment declaration: {declaration}"
            );
        }
        assert!(
            !css.contains("grid-template-columns: 80px minmax(0, 1fr) 1fr 56px;"),
            "chain preview rows must not keep an independent bar/value grid"
        );
        assert!(
            !css.contains("padding: 6px 0 6px 28px;"),
            "chain indentation must not move the preview bar scale"
        );
    }

    #[test]
    fn conditional_outcome_class_tracks_active_state() {
        assert_eq!(
            conditional_outcome_class(true),
            "if-editor__readout-chain-outcome if-editor__readout-chain-outcome--active"
        );
        assert_eq!(
            conditional_outcome_class(false),
            "if-editor__readout-chain-outcome if-editor__readout-chain-outcome--inactive"
        );
    }
}
