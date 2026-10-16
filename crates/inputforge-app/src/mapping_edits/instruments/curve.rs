//! Curve gesture, toolbar and commit decisions.
use super::{
    Commit, Session, keyboard,
    response_curve::{CurveType, interaction, mutation},
};
use inputforge_core::action::Action;
use inputforge_gui_dx::instruments::Event;
pub(super) fn event(session: &mut Session, event: &Event) -> Option<Commit> {
    let Action::ResponseCurve { curve } = &session.value else {
        return None;
    };
    let curve = curve.clone();
    let label;
    let mut group = None;
    let candidate = match event {
        Event::Pointer { kind, observation } => {
            let (value, caption) = pointer(session, &curve, kind, observation)?;
            label = caption;
            Some(value)
        }
        Event::Key { key, shift } => {
            let key_input = keyboard::curve(key, *shift)?;
            session.prevent_default = key != "Tab";
            if key == "Escape" {
                session.cancel();
                return None;
            }
            if session.dragging() {
                return None;
            }
            let (next, value, outcome, _) = super::response_curve::keyboard::handle_key(
                session.curve.clone(),
                &curve,
                key_input,
            );
            session.curve = next;
            if let Some(kind) = key_input.nudge_kind() {
                group = Some(format!("{kind:?}:{:?}", session.curve.focused_point));
            }
            let super::response_curve::keyboard::KeyOutcome::PushUndo { label: caption } = outcome?;
            label = caption;
            value
        }
        Event::Type(raw) => {
            let (current, current_label) = kind(&curve);
            let (target, target_label) = match raw.as_str() {
                "linear" => (CurveType::PiecewiseLinear, "linear"),
                "spline" => (CurveType::CubicSpline, "spline"),
                "bezier" => (CurveType::CubicBezier, "bezier"),
                _ => return None,
            };
            if target == current {
                return None;
            }
            label = format!("curve: type {current_label} -> {target_label}");
            mutation::convert_curve_type(&curve, target)
        }
        Event::Symmetric(enabled) => {
            label = format!("curve: symmetric {}", if *enabled { "on" } else { "off" });
            mutation::apply_symmetry(&curve, *enabled)
        }
        Event::Reset => {
            label = "curve: reset".into();
            Some(mutation::default_identity_curve(&curve))
        }
        _ => return None,
    }?;
    match mutation::reconstruct_curve(&candidate) {
        Ok(value) => {
            session.value = Action::ResponseCurve { curve: value };
            session.draft = true;
            session.anchors();
            Some(Commit {
                action: session.value.clone(),
                label,
                group,
            })
        }
        Err(error) => {
            session.cancel();
            session.error = Some(error);
            None
        }
    }
}
fn kind(curve: &inputforge_core::processing::curves::ResponseCurve) -> (CurveType, &'static str) {
    use inputforge_core::processing::curves::ResponseCurve;
    match curve {
        ResponseCurve::PiecewiseLinear { .. } => (CurveType::PiecewiseLinear, "linear"),
        ResponseCurve::CubicSpline { .. } => (CurveType::CubicSpline, "spline"),
        ResponseCurve::CubicBezier { .. } => (CurveType::CubicBezier, "bezier"),
    }
}

fn pointer(
    session: &mut Session,
    curve: &inputforge_core::processing::curves::ResponseCurve,
    kind: &str,
    observation: &inputforge_gui_dx::instruments::Observation,
) -> Option<(inputforge_core::processing::curves::ResponseCurve, String)> {
    let label;
    let candidate = match kind {
        "down" if observation.position.is_some() => {
            session.curve =
                interaction::handle_pointer_down(session.curve.clone(), curve, observation).0;
            return None;
        }
        "move" if observation.position.is_some() => {
            let (next, value, _) =
                interaction::handle_pointer_move(session.curve.clone(), curve, observation);
            session.curve = next;
            if let Some(value) = value {
                session.value = Action::ResponseCurve { curve: value };
                session.draft = true;
                session.anchors();
            }
            return None;
        }
        "up" if session.curve.dragging.is_some() => {
            let (next, result, _) = interaction::handle_pointer_up(session.curve.clone(), curve);
            session.curve = next;
            match result {
                Ok(value) => {
                    label = "curve: drag".into();
                    Some(value)
                }
                Err(error) => {
                    session.cancel();
                    session.error = Some(error);
                    return None;
                }
            }
        }
        "dbl" => {
            label = observation.position.map_or_else(
                || "curve: add point".into(),
                |(x, y)| format!("curve: add point at ({x:.2}, {y:.2})"),
            );
            let (next, value, _) =
                interaction::handle_double_click(session.curve.clone(), curve, observation);
            session.curve = next;
            value
        }
        "ctx" => {
            session.curve.hovered_point = observation
                .hit
                .filter(|&i| i < session.curve.cached_anchors.len());
            let (next, value, _) = interaction::handle_context_menu(session.curve.clone(), curve);
            session.curve = next;
            label = "curve: remove point".into();
            value
        }
        _ => return None,
    }?;
    Some((candidate, label))
}
