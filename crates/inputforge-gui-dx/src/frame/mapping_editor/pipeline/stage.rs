// Rust guideline compliant 2026-05-01

//! Stage card: header + body container.
//!
//! Renders one action as a collapsible card. Category tint is applied via
//! BEM modifier classes (`is-processing`, `is-output`, `is-control`).
//! The body region is a placeholder until Task 22 wires the dispatcher.

use dioxus::prelude::*;

use inputforge_core::action::{Action, Condition, ModeChangeStrategy};
use inputforge_core::processing::ResponseCurve;
use inputforge_core::types::{KeyCombo, KeyModifier, OutputAddress, OutputId, VJoyAxis};

use crate::context::ConfigSnapshot;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::stage_body;
use crate::frame::mapping_editor::pipeline::stage_header::StageHeader;
use crate::frame::mapping_editor::undo_log::StageId;

#[component]
pub(crate) fn Stage(
    stage_id: StageId,
    /// `(mode, InputAddress)` key for the mapping being edited. Named
    /// `mapping_key` to avoid collision with Dioxus's reserved `key` prop.
    mapping_key: MappingKey,
    action: Action,
    /// Mapping's root actions vec, threaded unchanged through every
    /// recursion. Bodies use this for tree mutators because `StageId`
    /// paths are root-relative. See `Pipeline` doc for rationale.
    root_actions: Vec<Action>,
    depth: u8,
) -> Element {
    let editor = use_context::<EditorState>();
    let expanded = editor.expanded_stages.read().contains(&stage_id);
    let ctx = use_context::<crate::context::AppContext>();
    let cfg = ctx.config.read().clone();

    let category_class = match &action {
        Action::ResponseCurve { .. } | Action::Deadzone { .. } | Action::Invert => "is-processing",
        Action::MapToVJoy { .. } | Action::MapToKeyboard { .. } | Action::MergeAxis { .. } => {
            "is-output"
        }
        Action::ChangeMode { .. } | Action::Conditional { .. } => "is-control",
    };

    let class = format!("if-stage {category_class}");
    let title = stage_title_for(&action).to_owned();
    let summary = stage_summary_for(&action, &cfg);
    let right_slot = stage_body::header_right_slot(&action, expanded);
    let body_id = format!("if-stage-body-{}", super::format_stage_id(&stage_id));

    rsx! {
        li {
            class: "{class}",
            "data-stage-id": "{super::format_stage_id(&stage_id)}",
            StageHeader {
                stage_id: stage_id.clone(),
                title,
                summary,
                expanded,
                right_slot,
            }
            if expanded {
                div {
                    id: "{body_id}",
                    class: "if-stage__body",
                    // Body dispatcher lands in Task 22.
                    div { class: "if-stage__body-placeholder", "(body)" }
                }
            }
        }
    }
}

/// Return the display title for an action variant.
///
/// Titles match spec § "Action surface coverage". Each variant maps to a
/// short, human-readable label shown in the stage header.
pub(crate) fn stage_title_for(action: &Action) -> &'static str {
    match action {
        Action::Invert => "Invert",
        Action::Deadzone { .. } => "Deadzone",
        Action::ResponseCurve { .. } => "Response curve",
        Action::MapToVJoy { .. } => "Map to vJoy",
        Action::MapToKeyboard { .. } => "Map to keyboard",
        Action::MergeAxis { .. } => "Merge axis",
        Action::ChangeMode { .. } => "Change mode",
        Action::Conditional { .. } => "Conditional",
    }
}

/// Return a one-line summary string for an action variant.
///
/// Shown in the collapsed stage header as secondary text. Empty for variants
/// whose configuration is fully conveyed by the title alone (`Invert`).
/// Looks up device names in `cfg` so the user sees friendly labels rather
/// than raw device IDs.
pub(crate) fn stage_summary_for(action: &Action, cfg: &ConfigSnapshot) -> String {
    match action {
        Action::Invert => String::new(),

        Action::Deadzone { config } => {
            // Show the outer low/high thresholds as percentages. The center
            // band is omitted here because it is already visualised in the
            // body widget (Task 27); the header needs only a glanceable hint.
            // Format directly from f64 with no fractional digits; avoids
            // lossy float-to-int casts.
            let low_pct = config.low().abs() * 100.0;
            let high_pct = config.high() * 100.0;
            format!("inner {low_pct:.0}% \u{00b7} outer {high_pct:.0}%")
        }

        Action::ResponseCurve { curve } => format_response_curve_summary(curve),

        Action::MapToVJoy { output } => format_output_summary(output),

        Action::MapToKeyboard { key } => format_key_combo(key),

        Action::MergeAxis {
            second_input,
            operation,
        } => {
            let device_name = cfg
                .devices
                .iter()
                .find(|d| d.info.id == second_input.device)
                .map_or_else(|| second_input.device.0.as_str(), |d| d.info.name.as_str());
            format!("{operation:?} \u{00b7} {device_name}")
        }

        Action::ChangeMode { strategy } => format_mode_strategy(strategy),

        Action::Conditional { condition, .. } => format_condition(condition, cfg),
    }
}

// ---------------------------------------------------------------------------
// Private formatting helpers
// ---------------------------------------------------------------------------

/// Format an [`OutputAddress`] as "vJoy {device} \u{00b7} {output-label}".
fn format_output_summary(output: &OutputAddress) -> String {
    let output_label = match &output.output {
        OutputId::Axis { id } => format_vjoy_axis(*id).to_owned(),
        OutputId::Button { id } => format!("Button {id}"),
        OutputId::Hat { id } => format!("Hat {id}"),
    };
    format!("vJoy {} \u{00b7} {output_label}", output.device)
}

/// Map a [`VJoyAxis`] to its short display name.
const fn format_vjoy_axis(axis: VJoyAxis) -> &'static str {
    match axis {
        VJoyAxis::X => "X",
        VJoyAxis::Y => "Y",
        VJoyAxis::Z => "Z",
        VJoyAxis::Rx => "Rx",
        VJoyAxis::Ry => "Ry",
        VJoyAxis::Rz => "Rz",
        VJoyAxis::Slider0 => "Slider 0",
        VJoyAxis::Slider1 => "Slider 1",
    }
}

/// Format a [`KeyCombo`] as "Mod + Mod + Key", e.g. "Ctrl + Shift + Q".
fn format_key_combo(key: &KeyCombo) -> String {
    let mut parts: Vec<&str> = key
        .modifiers
        .iter()
        .map(|m| match m {
            KeyModifier::Ctrl => "Ctrl",
            KeyModifier::Shift => "Shift",
            KeyModifier::Alt => "Alt",
            KeyModifier::Win => "Win",
        })
        .collect();
    parts.push(key.key.as_str());
    parts.join(" + ")
}

/// Format a [`ModeChangeStrategy`] to a concise one-line description.
fn format_mode_strategy(strategy: &ModeChangeStrategy) -> String {
    match strategy {
        ModeChangeStrategy::SwitchTo { mode } => format!("set {mode}"),
        ModeChangeStrategy::Temporary { mode } => format!("hold {mode}"),
        ModeChangeStrategy::Previous => "pop".to_owned(),
        ModeChangeStrategy::Cycle { modes } => {
            let labels = modes.modes().join(" \u{2192} ");
            format!("cycle {labels}")
        }
    }
}

/// Format a [`Condition`] to a short label, using `cfg` for device names.
fn format_condition(condition: &Condition, cfg: &ConfigSnapshot) -> String {
    match condition {
        Condition::ButtonPressed { input } => {
            let dev = device_label(cfg, &input.device);
            format!("button pressed \u{00b7} {dev}")
        }
        Condition::ButtonReleased { input } => {
            let dev = device_label(cfg, &input.device);
            format!("button released \u{00b7} {dev}")
        }
        Condition::AxisInRange { input, min, max } => {
            let dev = device_label(cfg, &input.device);
            // Format directly from f64 with no fractional digits to avoid
            // lossy float-to-int casts.
            let min_pct = *min * 100.0;
            let max_pct = *max * 100.0;
            format!("axis {min_pct:.0}%\u{2013}{max_pct:.0}% \u{00b7} {dev}")
        }
        Condition::HatDirection { input, directions } => {
            let dev = device_label(cfg, &input.device);
            let dir_count = directions.len();
            format!("hat ({dir_count} dir) \u{00b7} {dev}")
        }
        Condition::All { conditions } => format!("all ({} conditions)", conditions.len()),
        Condition::Any { conditions } => format!("any ({} conditions)", conditions.len()),
        Condition::Not { .. } => "not".to_owned(),
    }
}

/// Format a [`ResponseCurve`] summary: kind name and point/segment count.
fn format_response_curve_summary(curve: &ResponseCurve) -> String {
    match curve {
        ResponseCurve::PiecewiseLinear { points, symmetric } => {
            let sym = if *symmetric { " \u{00b7} sym" } else { "" };
            format!("linear \u{00b7} {} pts{sym}", points.len())
        }
        ResponseCurve::CubicSpline { points, symmetric } => {
            let sym = if *symmetric { " \u{00b7} sym" } else { "" };
            format!("spline \u{00b7} {} pts{sym}", points.len())
        }
        ResponseCurve::CubicBezier {
            segments,
            symmetric,
        } => {
            let sym = if *symmetric { " \u{00b7} sym" } else { "" };
            format!("bezier \u{00b7} {} seg{sym}", segments.len())
        }
    }
}

/// Look up the human-readable name for a device ID in the config snapshot.
///
/// Falls back to the raw device ID string when the device is not present in
/// the snapshot (e.g. disconnected devices whose actions are still persisted).
fn device_label<'a>(cfg: &'a ConfigSnapshot, id: &'a inputforge_core::types::DeviceId) -> &'a str {
    cfg.devices
        .iter()
        .find(|d| &d.info.id == id)
        .map_or(id.0.as_str(), |d| d.info.name.as_str())
}

/// Suppress unused-variable warning for `depth` and `mapping_key` until
/// Task 26a and Task 22 consume them respectively.
const _: () = {
    fn _assert_depth_used(_d: u8) {}
    fn _assert_mk_used(_k: &MappingKey) {}
};
