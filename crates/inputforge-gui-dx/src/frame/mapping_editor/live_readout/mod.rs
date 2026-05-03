// Rust guideline compliant 2026-05-01

//! Live readout: IN/OUT axis bars with merge-mapping layout.
//!
//! IN and OUT rows read directly from the live `ctx.live` snapshot (raw
//! input cache and engine output cache, respectively). Merged-IN runs
//! `evaluate_actions_through` over the input cache so curve/deadzone edits
//! preview live. When the engine is stopped, OUT freezes at the engine's
//! last-written value and the row carries an `--frozen` modifier class.
//!
//! **Layout rules (per F9 spec lines 42, 417)**
//! - Non-merge: `IN`, dashed divider, `OUT` (OUT omitted when no `MapToVJoy`).
//! - Merge:     `IN 1`, `IN 2`, dashed divider, merged `IN`, `OUT`
//!   (no extra divider before `OUT` in the merge case).

use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::processing::into_natural_domain;
use inputforge_core::state::EngineStatus;
use inputforge_core::types::{AxisPolarity, InputAddress, MergeOp, OutputAddress};

use crate::context::AppContext;
use crate::frame::mapping_list::source_label;

mod value_helpers;

use value_helpers::{
    AxisDisplay, axis_f64, format_output_label, format_percentage, merge_output_polarity,
    read_axis_display, read_output_display,
};

/// CSS modifier class applied to a `ReadoutRow` whose value is held
/// (engine stopped / paused). The component renders this suffix as part
/// of a static literal in `ReadoutRow`; this const exists so SSR tests
/// can `html.contains(FROZEN_ROW_CLASS)` rather than retyping the
/// modifier name. Keep the literal and the const in sync.
pub(super) const FROZEN_ROW_CLASS: &str = "if-editor__readout-row--frozen";

// ---------------------------------------------------------------------------
// Public component
// ---------------------------------------------------------------------------

/// Live IN/OUT readout section, wired beneath the input field.
///
/// # Props
/// - `primary`  — address of the primary (mapped) input axis.
/// - `actions`  — the full action pipeline for the selected mapping.
#[component]
pub(crate) fn LiveReadout(primary: InputAddress, actions: Vec<Action>) -> Element {
    let ctx = use_context::<AppContext>();
    let live = ctx.live.read();
    let cfg = ctx.config.read();
    let engine_running = matches!(ctx.meta.read().engine_status, EngineStatus::Running);

    let primary_value = read_axis_display(&primary, &live, &cfg);
    let merge_ctx = find_merge_context(&actions);
    let out_addr = first_map_to_vjoy_output(&actions);

    // Resolve tags for each row up front (single read-lock). Tags echo
    // info from the header subtitle and merge stage so the readout stays
    // self-explanatory when the user is watching the bars move; the
    // duplication is intentional, not redundant.
    let primary_tag = source_label::format(&primary, &cfg);
    let secondary_tag = merge_ctx
        .as_ref()
        .map(|c| source_label::format(&c.secondary, &cfg));
    let out_tag = out_addr.as_ref().map(format_output_label);

    // Resolve secondary axis display once. Used for the IN 2 row and to
    // infer the merge result's output polarity (consumed by the IN row
    // and OUT row).
    let secondary_display = merge_ctx
        .as_ref()
        .map(|c| read_axis_display(&c.secondary, &live, &cfg));

    // Polarity of the merged result (or, in the no-merge case, of the
    // OUT row): primary inherits when there is no merge; otherwise the
    // merge op's natural output polarity per `merge_output_polarity`.
    let output_polarity = match (&merge_ctx, secondary_display) {
        (Some(c), Some(secondary)) => {
            merge_output_polarity(c.op, primary_value.polarity, secondary.polarity)
        }
        _ => primary_value.polarity,
    };

    // Brief read-lock on AppState for pipeline evaluation.
    // Two separate guard scopes so Rust can prove non-overlapping borrows.
    let merged_in_value = merge_ctx.as_ref().map(|c| {
        let state = ctx.state.read();
        let iv = inputforge_core::pipeline::evaluate_actions_through(
            &actions,
            &state,
            &primary,
            c.index + 1,
        );
        AxisDisplay {
            value: into_natural_domain(axis_f64(&iv), output_polarity),
            polarity: output_polarity,
        }
    });

    // OUT reads directly from the engine output cache (projected into
    // `live.output_values` by `LiveSnapshot::from_state`). When the engine
    // isn't running it freezes at the last written value, by design.
    let out_value = out_addr
        .as_ref()
        .map(|out| read_output_display(out, &live, &cfg, output_polarity));

    rsx! {
        div { class: "if-editor__readout",
            if let Some(secondary) = secondary_display {
                // Merge layout: IN 1, IN 2, labeled divider, merged IN, OUT.
                // No extra divider before OUT in the merge case (spec line 417).
                // Each side of the divider is its own grid (`readout-group`)
                // so the tag column in the input section auto-sizes to the
                // longest source label, while the merged + OUT section
                // sizes independently. Bars within a group share an x
                // origin; bars across groups can differ.
                // `merged_in_value` is always Some when secondary_display is
                // Some (both derive from the same merge_ctx); the unwrap_or
                // is a defensive default that should never fire.
                div { class: "if-editor__readout-group",
                    ReadoutRow {
                        label: "IN 1".to_owned(),
                        tag: primary_tag.clone(),
                        display: primary_value,
                        frozen: false,
                    }
                    ReadoutRow {
                        label: "IN 2".to_owned(),
                        tag: secondary_tag.unwrap_or_default(),
                        display: secondary,
                        frozen: false,
                    }
                }
                ReadoutDivider { label: "merge".to_owned() }
                div { class: "if-editor__readout-group",
                    ReadoutRow {
                        label: "IN".to_owned(),
                        tag: "Merged".to_owned(),
                        display: merged_in_value.unwrap_or(AxisDisplay {
                            value: into_natural_domain(0.0, output_polarity),
                            polarity: output_polarity,
                        }),
                        frozen: false,
                    }
                    if let Some(out) = out_value {
                        ReadoutRow {
                            label: "OUT".to_owned(),
                            tag: out_tag.clone().unwrap_or_default(),
                            display: out,
                            frozen: !engine_running,
                        }
                    }
                }
            } else {
                // Non-merge layout: IN, optional labeled divider + OUT.
                // Each row sits in its own group: in this case there's no
                // alignment benefit (one row per group) but the markup
                // stays parallel to the merge case.
                div { class: "if-editor__readout-group",
                    ReadoutRow {
                        label: "IN".to_owned(),
                        tag: primary_tag.clone(),
                        display: primary_value,
                        frozen: false,
                    }
                }
                if let Some(out) = out_value {
                    ReadoutDivider { label: "out".to_owned() }
                    div { class: "if-editor__readout-group",
                        ReadoutRow {
                            label: "OUT".to_owned(),
                            tag: out_tag.clone().unwrap_or_default(),
                            display: out,
                            frozen: !engine_running,
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Sub-component
// ---------------------------------------------------------------------------

/// One row in the readout grid: label | tag | bar | percentage text.
///
/// The bar fill is anchored at 50% for bipolar axes and at 0% for
/// unipolar axes, matching the live-green visual described in the spec.
/// Bipolar bars also carry a `--bipolar` modifier class so the CSS can
/// draw a center tick that communicates polarity at idle.
///
/// `frozen` is true only on the OUT row when the engine is not running.
/// CSS dims the bar fill and percentage to signal "held value, not live";
/// label and tag stay at full strength because they describe configuration,
/// not telemetry.
#[component]
fn ReadoutRow(label: String, tag: String, display: AxisDisplay, frozen: bool) -> Element {
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

    // Always set BOTH `left` and `right` (with `auto` for the off side)
    // so Dioxus's per-property style diffing cannot leave a stale anchor
    // from a previous render. Without this, switching from negative to
    // positive (or vice versa) keeps the prior side's `50%` set, which
    // CSS resolves into `left: 50%; right: 50%; width: <pct>%;` and
    // honors `left + width` regardless of sign — so both signs grow
    // rightward and overflow the container.
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

/// Section divider with an inline label (e.g. `─── merge ───`).
///
/// Renders as a single grid cell that spans the full row and contains
/// dashed lines on either side of a small uppercase label, marking the
/// transition between the input section and the merged-or-output section.
#[component]
fn ReadoutDivider(label: String) -> Element {
    rsx! {
        div { class: "if-editor__readout-divider",
            span { class: "if-editor__readout-divider-label", "{label}" }
        }
    }
}

// ---------------------------------------------------------------------------
// Action-tree walkers (top-level only for merge; recursive for MapToVJoy)
// ---------------------------------------------------------------------------

/// Resolved context for the first top-level `MergeAxis` in the pipeline.
///
/// Top-level only by design: merges nested inside `Conditional` do not
/// trigger the merge layout (acceptable per the F9 task spec).
#[derive(Clone, PartialEq)]
struct MergeContext {
    /// Position in the action list, used as the `stop_at` for the
    /// pipeline subset that produces the merged IN value.
    index: usize,
    /// Merge operator (drives the polarity inference table).
    op: MergeOp,
    /// Secondary input address (the IN 2 row).
    secondary: InputAddress,
}

/// Find the first top-level `MergeAxis` and return its index, op, and
/// secondary input.
fn find_merge_context(actions: &[Action]) -> Option<MergeContext> {
    actions.iter().enumerate().find_map(|(idx, a)| {
        if let Action::MergeAxis {
            second_input,
            operation,
        } = a
        {
            Some(MergeContext {
                index: idx,
                op: *operation,
                secondary: second_input.clone(),
            })
        } else {
            None
        }
    })
}

/// First `MapToVJoy` output address anywhere in the tree (including
/// inside `Conditional` branches), or `None` if no `MapToVJoy` exists.
///
/// Returns the address (not just a bool) so the OUT row's tag column
/// can show the destination label (e.g. `vJoy 1 · Y axis`) without a
/// second tree walk.
fn first_map_to_vjoy_output(actions: &[Action]) -> Option<OutputAddress> {
    for action in actions {
        match action {
            Action::MapToVJoy { output } => return Some(output.clone()),
            Action::Conditional {
                if_true, if_false, ..
            } => {
                if let Some(o) = first_map_to_vjoy_output(if_true) {
                    return Some(o);
                }
                if let Some(o) = first_map_to_vjoy_output(if_false) {
                    return Some(o);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::types::InputId;

    fn axis_addr(index: u8) -> InputAddress {
        use inputforge_core::types::DeviceId;
        InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index },
        }
    }

    #[test]
    fn find_merge_context_returns_none_for_no_merge() {
        let actions = vec![Action::Invert];
        assert!(find_merge_context(&actions).is_none());
    }

    #[test]
    fn find_merge_context_picks_first_top_level_merge() {
        let actions = vec![
            Action::Invert,
            Action::MergeAxis {
                second_input: axis_addr(1),
                operation: MergeOp::Bidirectional,
            },
        ];
        let ctx = find_merge_context(&actions).expect("expected merge context");
        assert_eq!(ctx.index, 1);
        assert_eq!(ctx.op, MergeOp::Bidirectional);
        assert_eq!(ctx.secondary, axis_addr(1));
    }

    #[test]
    fn find_merge_context_skips_conditional_nested_merge() {
        use inputforge_core::action::Condition;
        // A merge nested inside a Conditional branch is intentionally
        // ignored; the merge layout only triggers for top-level merges.
        let actions = vec![Action::Conditional {
            condition: Condition::ButtonPressed {
                input: axis_addr(2),
            },
            if_true: vec![Action::MergeAxis {
                second_input: axis_addr(1),
                operation: MergeOp::Bidirectional,
            }],
            if_false: Vec::new(),
        }];
        assert!(find_merge_context(&actions).is_none());
    }
}
