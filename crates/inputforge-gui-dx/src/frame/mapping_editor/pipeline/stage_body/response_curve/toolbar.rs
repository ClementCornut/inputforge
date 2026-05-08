// Rust guideline compliant 2026-05-02

//! Toolbar above the plot: type segmented control + symmetric switch + reset.

use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::processing::curves::ResponseCurve;

use crate::components::tabs::{TabItem, Tabs};
use crate::components::{Button, ButtonSize, ButtonVariant, Switch};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::stage_body::instruments::stage_dispatch::dispatch_stage_edit;
use crate::frame::mapping_editor::undo_log::StageId;

use super::CurveType;
use super::mutation;

/// Toolbar rendered above the response-curve plot.
///
/// Owns three controls: a type segmented control (`Tabs`), a symmetric
/// `Switch`, and a ghost `Button` to reset to the identity curve. Each
/// control dispatches `EngineCommand::SetMapping` and pushes an undo entry
/// via `instruments::stage_dispatch::dispatch_stage_edit`.
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
        dispatch_stage_edit(
            &actions_for_type,
            &stage_for_type,
            Action::ResponseCurve { curve: new },
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
        dispatch_stage_edit(
            &actions_for_sym,
            &stage_for_sym,
            Action::ResponseCurve { curve: new },
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
        dispatch_stage_edit(
            &actions_for_reset,
            &stage_for_reset,
            Action::ResponseCurve { curve: new },
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
                    TabItem { id: "linear".to_owned(), label: "Linear".to_owned(), controls: None, running: false },
                    TabItem { id: "spline".to_owned(), label: "Spline".to_owned(), controls: None, running: false },
                    TabItem { id: "bezier".to_owned(), label: "Bezier".to_owned(), controls: None, running: false },
                ],
                onchange: on_type_change,
            }
            Switch {
                checked: symmetric_signal,
                onchange: on_symmetric_change,
                label: Some("Symmetric".to_owned()),
            }
            // Secondary (elevated-navy fill + strong border) so the button has
            // resting affordance per DESIGN.md / Buttons. Ghost gives no
            // resting border or fill and reads as a static label next to the
            // checked Switch (same UX failure mode as the rebind button).
            Button {
                variant: ButtonVariant::Secondary,
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
