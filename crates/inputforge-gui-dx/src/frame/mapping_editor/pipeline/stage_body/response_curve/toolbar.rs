// Rust guideline compliant 2026-05-02

//! Toolbar above the plot: type segmented control + symmetric switch + reset.

use std::sync::mpsc::Sender;

use dioxus::prelude::*;

use inputforge_core::action::{Action, Mapping};
use inputforge_core::engine::EngineCommand;
use inputforge_core::processing::curves::ResponseCurve;

use crate::components::tabs::{TabItem, Tabs};
use crate::components::{Button, ButtonSize, ButtonVariant, Switch};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::replace_at_path;
use crate::frame::mapping_editor::undo_log::{StageId, UndoKind};

use super::CurveType;
use super::mutation;

/// Toolbar rendered above the response-curve plot.
///
/// Owns three controls: a type segmented control (`Tabs`), a symmetric
/// `Switch`, and a ghost `Button` to reset to the identity curve. Each
/// control dispatches `EngineCommand::SetMapping` and pushes an undo entry
/// via `dispatch_curve_edit`.
///
/// All signals needed are received as props so the component remains
/// SSR-testable in isolation (no implicit context reads for data).
#[component]
pub(crate) fn Toolbar(
    curve: ResponseCurve,
    stage_id: StageId,
    root_actions: Vec<Action>,
    mapping_key: MappingKey,
) -> Element {
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();
    let cmd_tx = ctx.commands.clone();
    let config_signal = ctx.config;
    let mut undo_log = editor.undo_log;

    let current_kind = match &curve {
        ResponseCurve::PiecewiseLinear { .. } => CurveType::PiecewiseLinear,
        ResponseCurve::CubicSpline { .. } => CurveType::CubicSpline,
        ResponseCurve::CubicBezier { .. } => CurveType::CubicBezier,
    };
    let current_kind_id = match current_kind {
        CurveType::PiecewiseLinear => "linear".to_owned(),
        CurveType::CubicSpline => "spline".to_owned(),
        CurveType::CubicBezier => "bezier".to_owned(),
    };
    let symmetric = matches!(
        &curve,
        ResponseCurve::PiecewiseLinear {
            symmetric: true,
            ..
        } | ResponseCurve::CubicSpline {
            symmetric: true,
            ..
        } | ResponseCurve::CubicBezier {
            symmetric: true,
            ..
        }
    );
    // `Switch::checked` expects `ReadSignal<bool>`. Derive a read-only
    // signal from the prop so the component stays stateless (the owning
    // body re-renders with the new curve on every edit).
    let symmetric_signal: ReadSignal<bool> = use_signal(|| symmetric).into();

    let curve_for_type = curve.clone();
    let actions_for_type = root_actions.clone();
    let key_for_type = mapping_key.clone();
    let stage_for_type = stage_id.clone();
    let cmd_for_type = cmd_tx.clone();
    let on_type_change = move |id: String| {
        let target = match id.as_str() {
            "linear" => CurveType::PiecewiseLinear,
            "spline" => CurveType::CubicSpline,
            "bezier" => CurveType::CubicBezier,
            _ => return,
        };
        if target == current_kind {
            return;
        }
        let Some(new) = mutation::convert_curve_type(&curve_for_type, target) else {
            return;
        };
        let name = config_signal
            .read()
            .mapping_names
            .get(&key_for_type.1)
            .cloned();
        dispatch_curve_edit(
            &actions_for_type,
            &stage_for_type,
            new,
            &key_for_type,
            name,
            &cmd_for_type,
            &mut undo_log,
            format!(
                "curve: type {} -> {}",
                kind_label(current_kind),
                kind_label(target),
            ),
        );
    };

    let curve_for_sym = curve.clone();
    let actions_for_sym = root_actions.clone();
    let key_for_sym = mapping_key.clone();
    let stage_for_sym = stage_id.clone();
    let cmd_for_sym = cmd_tx.clone();
    let on_symmetric_change = move |evt: FormEvent| {
        // Switch renders <input type="checkbox">. evt.value() returns the
        // static `value` attribute (always "on") regardless of checked
        // state; use evt.checked() for the actual bit.
        let new_state = evt.data().checked();
        if new_state == symmetric {
            return;
        }
        let Some(new) = mutation::apply_symmetry(&curve_for_sym, new_state) else {
            return;
        };
        let name = config_signal
            .read()
            .mapping_names
            .get(&key_for_sym.1)
            .cloned();
        dispatch_curve_edit(
            &actions_for_sym,
            &stage_for_sym,
            new,
            &key_for_sym,
            name,
            &cmd_for_sym,
            &mut undo_log,
            format!("curve: symmetric {}", if new_state { "on" } else { "off" }),
        );
    };

    let curve_for_reset = curve.clone();
    let actions_for_reset = root_actions.clone();
    let key_for_reset = mapping_key.clone();
    let stage_for_reset = stage_id.clone();
    let cmd_for_reset = cmd_tx.clone();
    let on_reset = move |_| {
        let new = mutation::default_identity_curve(&curve_for_reset);
        if new == curve_for_reset {
            return;
        }
        let name = config_signal
            .read()
            .mapping_names
            .get(&key_for_reset.1)
            .cloned();
        dispatch_curve_edit(
            &actions_for_reset,
            &stage_for_reset,
            new,
            &key_for_reset,
            name,
            &cmd_for_reset,
            &mut undo_log,
            "curve: reset".to_owned(),
        );
    };

    rsx! {
        div { class: "if-curve__toolbar",
            Tabs {
                value: current_kind_id,
                items: vec![
                    TabItem { id: "linear".to_owned(), label: "Linear".to_owned(), controls: None },
                    TabItem { id: "spline".to_owned(), label: "Spline".to_owned(), controls: None },
                    TabItem { id: "bezier".to_owned(), label: "Bezier".to_owned(), controls: None },
                ],
                onchange: on_type_change,
            }
            Switch {
                checked: symmetric_signal,
                onchange: on_symmetric_change,
                label: Some("Symmetric".to_owned()),
            }
            Button {
                variant: ButtonVariant::Ghost,
                size: ButtonSize::Sm,
                onclick: on_reset,
                "Reset"
            }
        }
    }
}

fn kind_label(k: CurveType) -> &'static str {
    match k {
        CurveType::PiecewiseLinear => "linear",
        CurveType::CubicSpline => "spline",
        CurveType::CubicBezier => "bezier",
    }
}

/// Dispatch a curve edit to the engine **without** recording an undo entry.
///
/// Used by the keyboard handler's same-key coalesce path (`MergeUndo`): the
/// engine must receive the updated curve but the undo log must not receive a
/// new entry. The first nudge of a coalesce burst already pushed an entry
/// (via [`dispatch_curve_edit`]) whose `mapping_before` captures the
/// pre-burst state, so `undo` restores correctly without the intermediate
/// entries.
///
/// `name` is threaded through to preserve the user-set mapping name (mirrors
/// the `dispatch_curve_edit` convention; see `name_field.rs:60-70`).
pub(crate) fn dispatch_curve_edit_no_undo(
    actions_before: &[Action],
    stage_id: &StageId,
    new_curve: ResponseCurve,
    mapping_key: &MappingKey,
    name: Option<String>,
    cmd_tx: &Sender<EngineCommand>,
) {
    let Some(new_actions) = replace_at_path(
        actions_before,
        stage_id,
        Action::ResponseCurve { curve: new_curve },
    ) else {
        return;
    };
    if cmd_tx
        .send(EngineCommand::SetMapping {
            input: mapping_key.1.clone(),
            mode: mapping_key.0.clone(),
            name,
            actions: new_actions,
        })
        .is_err()
    {
        tracing::warn!(
            target: "f10::response_curve",
            action = "set_mapping_no_undo_drop_offline",
            "dropped no-undo SetMapping command: receiver disconnected"
        );
    }
}

// `name` is resolved by the caller via
// `ctx.config.read().mapping_names.get(mapping_key).cloned()`. Both the
// undo `before` snapshot and the engine command must carry the same
// `Some(name)` to preserve the user-set mapping name (mirrors F9
// amendment #2; see `name_field.rs:60-70` and `input_field.rs:87-103`).
#[expect(
    clippy::too_many_arguments,
    reason = "F9 convention; matches dispatch_input_field_edit signature"
)]
pub(crate) fn dispatch_curve_edit(
    actions_before: &[Action],
    stage_id: &StageId,
    new_curve: ResponseCurve,
    mapping_key: &MappingKey,
    name: Option<String>,
    cmd_tx: &Sender<EngineCommand>,
    undo_log: &mut Signal<crate::frame::mapping_editor::undo_log::UndoLog>,
    label: String,
) {
    let Some(new_actions) = replace_at_path(
        actions_before,
        stage_id,
        Action::ResponseCurve { curve: new_curve },
    ) else {
        return;
    };
    let before = Mapping {
        input: mapping_key.1.clone(),
        mode: mapping_key.0.clone(),
        name: name.clone(),
        actions: actions_before.to_vec(),
    };
    if cmd_tx
        .send(EngineCommand::SetMapping {
            input: mapping_key.1.clone(),
            mode: mapping_key.0.clone(),
            name,
            actions: new_actions,
        })
        .is_err()
    {
        tracing::warn!(
            target: "f10::response_curve",
            action = "set_mapping_drop_offline",
            "dropped SetMapping command: receiver disconnected"
        );
        return;
    }
    undo_log
        .write()
        .push_edit(mapping_key.clone(), before, UndoKind::StageEdit, label);
}
