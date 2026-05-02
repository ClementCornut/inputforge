// Rust guideline compliant 2026-05-01

//! Stage card: header + body container.
//!
//! Renders one action as a collapsible card. Category tint is applied via
//! BEM modifier classes (`is-processing`, `is-output`, `is-control`).
//! The drag handle (6-dot grip) is rendered in the stage header area and
//! wires `ondragstart` via `SortableHandle`.

use dioxus::prelude::*;

use inputforge_core::action::{Action, Condition, ModeChangeStrategy};
use inputforge_core::processing::ResponseCurve;
use inputforge_core::types::{
    InputAddress, KeyCombo, KeyModifier, OutputAddress, OutputId, VJoyAxis,
};

use crate::components::sortable::{SortableHandle, SortableState};
use crate::context::ConfigSnapshot;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::pipeline::stage_body;
use crate::frame::mapping_editor::pipeline::stage_header::StageHeader;
use crate::frame::mapping_editor::undo_log::{StageId, StageIdSegment};
use crate::frame::mapping_editor::{EditorState, StageMenuState};

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Derive the group-local index of a stage within its parent pipeline.
///
/// The last segment of a well-formed `StageId` is always `Index(i)`. Returns
/// `0` as a safe fallback when the path is malformed (empty or ends with a
/// branch segment); callers must handle this gracefully.
fn local_index_of(stage_id: &StageId) -> usize {
    match stage_id.0.last() {
        Some(StageIdSegment::Index(i)) => *i,
        _ => 0,
    }
}

/// Walk `root_actions` to the slice addressed by `parent_pipeline_path` and
/// return its length. Returns the length of `root_actions` itself when
/// `parent_pipeline_path` is empty (the stage lives in the outer pipeline).
///
/// Returns `0` on an invalid path so callers degrade gracefully rather than
/// panic. The validator in `validate_pipeline_drop` prevents mis-directed
/// drops in the common case, so this fallback is a last resort only.
fn parent_pipeline_len(root_actions: &[Action], parent_pipeline_path: &StageId) -> usize {
    if parent_pipeline_path.0.is_empty() {
        return root_actions.len();
    }
    // The parent_pipeline_path segments describe the path from root down to the
    // branch that contains this stage. The path always ends with a branch
    // segment (IfTrue / IfFalse), not an Index, because Pipeline strips the
    // terminal Index when constructing path_prefix.
    let mut cursor: &[Action] = root_actions;
    let mut last_action: Option<&Action> = None;
    for seg in &parent_pipeline_path.0 {
        match seg {
            StageIdSegment::Index(i) => {
                let Some(a) = cursor.get(*i) else { return 0 };
                last_action = Some(a);
            }
            StageIdSegment::IfTrue => match last_action {
                Some(Action::Conditional { if_true, .. }) => cursor = if_true.as_slice(),
                _ => return 0,
            },
            StageIdSegment::IfFalse => match last_action {
                Some(Action::Conditional { if_false, .. }) => {
                    cursor = if_false.as_slice();
                }
                _ => return 0,
            },
        }
    }
    cursor.len()
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

#[component]
#[allow(
    clippy::too_many_lines,
    reason = "Stage integrates header, body, context-menu, and DnD in one component \
              by design; splitting would require threading many more props."
)]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures."
)]
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
    /// The `StageId` of the parent pipeline (i.e., `path_prefix` from
    /// `Pipeline`). Used as the sortable group discriminator so that only
    /// stages within the same pipeline can be reordered together.
    parent_pipeline_path: StageId,
    depth: u8,
) -> Element {
    let editor = use_context::<EditorState>();
    let expanded = editor.expanded_stages.read().contains(&stage_id);
    let ctx = use_context::<crate::context::AppContext>();
    let cfg = ctx.config.read().clone();

    // Read the shared sortable state installed by MappingEditor.
    let sortable = use_context::<SortableState<StageId>>();

    let category_class = match &action {
        Action::ResponseCurve { .. } | Action::Deadzone { .. } | Action::Invert => "is-processing",
        Action::MapToVJoy { .. } | Action::MapToKeyboard { .. } | Action::MergeAxis { .. } => {
            "is-output"
        }
        Action::ChangeMode { .. } | Action::Conditional { .. } => "is-control",
    };

    // Derive the group-local index and parent pipeline length for the sortable
    // primitive. Both are O(n) but bounded by pipeline depth, not total stages.
    let local_index = local_index_of(&stage_id);
    let group_len = parent_pipeline_len(&root_actions, &parent_pipeline_path);

    // Drag-source modifier. The sortable group discriminator is the full
    // parent_pipeline_path, so group comparison is by StageId equality
    // (Vec<StageIdSegment> PartialEq). Drop-target painting lives on the
    // inter-stage gaps in the gap-drop-zone primitive, not on stages
    // themselves.
    let is_drag_source = sortable
        .drag_from
        .read()
        .is_some_and(|src_idx| src_idx == local_index)
        && sortable
            .drag_group
            .read()
            .as_ref()
            .is_some_and(|src_group| src_group == &parent_pipeline_path);

    let mut base_class = format!("if-stage {category_class}");
    if is_drag_source {
        base_class.push_str(" if-sortable--dragging");
    }

    // Task 35: look up any validation hint written by the body for this stage.
    // When a hint exists the summary slot shows it instead of the normal
    // summary, and the title receives an error-tint class.
    let malformed_hint: Option<String> = editor.malformed_hints.read().get(&stage_id).cloned();
    let is_malformed = malformed_hint.is_some();

    let title = stage_title_for(&action).to_owned();
    let summary = malformed_hint.unwrap_or_else(|| stage_summary_for(&action, &cfg));
    let right_slot = stage_body::header_right_slot(&action, expanded);
    let body_id = format!("if-stage-body-{}", super::format_stage_id(&stage_id));

    // Context-menu handler: writes cursor coordinates + target stage id into
    // `EditorState::stage_menu`.
    let oncontextmenu = {
        let stage_id = stage_id.clone();
        let mut stage_menu = editor.stage_menu;
        move |evt: MouseEvent| {
            evt.prevent_default();
            evt.stop_propagation();
            let coords = evt.client_coordinates();
            stage_menu.set(Some(StageMenuState {
                stage: stage_id.clone(),
                x: coords.x,
                y: coords.y,
            }));
        }
    };

    rsx! {
        li {
            class: "{base_class}",
            "data-stage-id": "{super::format_stage_id(&stage_id)}",
            oncontextmenu,
            SortableHandle {
                state: sortable,
                index: local_index,
                group: parent_pipeline_path.clone(),
                group_len,
            }
            StageHeader {
                stage_id: stage_id.clone(),
                title,
                summary,
                expanded,
                is_malformed,
                right_slot,
            }
            if expanded {
                div {
                    id: "{body_id}",
                    class: "if-stage__body",
                    stage_body::StageBody {
                        mapping_key: mapping_key.clone(),
                        stage_id: stage_id.clone(),
                        action: action.clone(),
                        root_actions: root_actions.clone(),
                    }
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
            // DeadzoneConfig defines five zones on [-1, 1] (per its doc):
            //   below `low`           -> saturates to -1.0  (outer dead band, neg side)
            //   [low, center_low]     -> linear ramp to 0
            //   [center_low,
            //    center_high]         -> dead center        (inner dead band)
            //   [center_high, high]   -> linear ramp to +1
            //   above `high`          -> saturates to +1.0  (outer dead band, pos side)
            //
            // For the header we surface two glanceable percentages:
            //   inner = width of the dead-center band
            //         = (center_high - center_low) * 100
            //   outer = width of the positive-side saturation band
            //         = (1.0 - high) * 100
            // The body widget (Task 27) shows the full picture; the header
            // only needs a hint. Format directly from f64 with `{:.0}` to
            // avoid the lossy float-to-int casts that clippy rejects.
            let inner_pct = (config.center_high() - config.center_low()) * 100.0;
            let outer_pct = (1.0 - config.high()) * 100.0;
            format!("inner {inner_pct:.0}% \u{00b7} outer {outer_pct:.0}%")
        }

        Action::ResponseCurve { curve } => format_response_curve_summary(curve),

        Action::MapToVJoy { output } => format_output_summary(output),

        Action::MapToKeyboard { key } => format_key_combo(key),

        Action::MergeAxis {
            second_input,
            operation,
        } => match second_input.device() {
            Some(device_id) => {
                let device_name = cfg
                    .devices
                    .iter()
                    .find(|d| &d.info.id == device_id)
                    .map_or_else(|| device_id.0.as_str(), |d| d.info.name.as_str());
                format!("{operation:?} \u{00b7} {device_name}")
            }
            None => format!("{operation:?} \u{00b7} Unbound"),
        },

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
        ModeChangeStrategy::SwitchTo { mode } => format!("Set {mode}"),
        ModeChangeStrategy::Temporary { mode } => format!("Hold {mode}"),
        ModeChangeStrategy::Previous => "Pop".to_owned(),
        ModeChangeStrategy::Cycle { modes } => {
            let labels = modes.modes().join(" \u{2192} ");
            format!("Cycle {labels}")
        }
    }
}

/// Format a [`Condition`] to a short label, using `cfg` for device names.
fn format_condition(condition: &Condition, cfg: &ConfigSnapshot) -> String {
    match condition {
        Condition::ButtonPressed { input } => {
            let dev = predicate_device_label(cfg, input);
            format!("Button pressed \u{00b7} {dev}")
        }
        Condition::ButtonReleased { input } => {
            let dev = predicate_device_label(cfg, input);
            format!("Button released \u{00b7} {dev}")
        }
        Condition::AxisInRange { input, min, max } => {
            let dev = predicate_device_label(cfg, input);
            // Format directly from f64 with no fractional digits to avoid
            // lossy float-to-int casts.
            let min_pct = *min * 100.0;
            let max_pct = *max * 100.0;
            format!("Axis {min_pct:.0}%\u{2013}{max_pct:.0}% \u{00b7} {dev}")
        }
        Condition::HatDirection { input, directions } => {
            let dev = predicate_device_label(cfg, input);
            let dir_count = directions.len();
            format!("Hat ({dir_count} dir) \u{00b7} {dev}")
        }
        Condition::All { conditions } => format!("All ({} conditions)", conditions.len()),
        Condition::Any { conditions } => format!("Any ({} conditions)", conditions.len()),
        Condition::Not { .. } => "Not".to_owned(),
    }
}

/// Resolve a predicate's input address to a device-name label, falling back to
/// the literal `"Unbound"` when the predicate has no binding selected yet.
fn predicate_device_label<'a>(cfg: &'a ConfigSnapshot, addr: &'a InputAddress) -> &'a str {
    match addr.device() {
        Some(id) => device_label(cfg, id),
        None => "Unbound",
    }
}

/// Format a [`ResponseCurve`] summary: kind name and point/segment count.
fn format_response_curve_summary(curve: &ResponseCurve) -> String {
    match curve {
        ResponseCurve::PiecewiseLinear { points, symmetric } => {
            let sym = if *symmetric { " \u{00b7} sym" } else { "" };
            format!("Linear \u{00b7} {} pts{sym}", points.len())
        }
        ResponseCurve::CubicSpline { points, symmetric } => {
            let sym = if *symmetric { " \u{00b7} sym" } else { "" };
            format!("Spline \u{00b7} {} pts{sym}", points.len())
        }
        ResponseCurve::CubicBezier {
            segments,
            symmetric,
        } => {
            let sym = if *symmetric { " \u{00b7} sym" } else { "" };
            format!("Bezier \u{00b7} {} seg{sym}", segments.len())
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
