//! Deadzone gesture, numeric validation and commit decisions.
use super::{Commit, Session, deadzone::interaction, keyboard};
use inputforge_core::{action::Action, processing::deadzone::DeadzoneConfig};
use inputforge_gui_dx::instruments::Event;
pub(super) fn event(session: &mut Session, event: &Event) -> Option<Commit> {
    let Action::Deadzone { config } = &session.value else {
        return None;
    };
    let config = config.clone();
    let label;
    let mut group = None;
    let candidate = match event {
        Event::Pointer { kind, observation } => match kind.as_str() {
            "down" if observation.position.is_some() => {
                session.zone =
                    interaction::handle_pointer_down(session.zone.clone(), &config, observation).0;
                return None;
            }
            "move" if observation.position.is_some() => {
                let (next, value, _) =
                    interaction::handle_pointer_move(session.zone.clone(), &config, observation);
                session.zone = next;
                if let Some(config) = value {
                    session.value = Action::Deadzone { config };
                    session.draft = true;
                }
                return None;
            }
            "up" if session.zone.dragging.is_some() || session.zone.pending_split.is_some() => {
                let (next, value, _) =
                    interaction::handle_pointer_up(session.zone.clone(), &config);
                session.zone = next;
                match value {
                    Ok(config) => {
                        label = "deadzone: drag".into();
                        Some(config)
                    }
                    Err(error) => {
                        session.cancel();
                        if !error.is_empty() {
                            session.error = Some(error);
                        }
                        return None;
                    }
                }
            }
            _ => return None,
        },
        Event::Key { key, shift } => {
            let input = keyboard::zone(key, *shift)?;
            session.prevent_default = key != "Tab";
            if key == "Escape" {
                session.cancel();
                return None;
            }
            if session.dragging() {
                return None;
            }
            let (next, value, outcome, _) =
                super::deadzone::keyboard::handle_key(session.zone.clone(), &config, input);
            session.zone = next;
            if let Some(kind) = input.nudge_kind() {
                group = Some(format!("{kind:?}:{:?}", session.zone.focused_handle));
            }
            let super::deadzone::keyboard::KeyOutcome::PushUndo { label: caption } = outcome?;
            label = caption;
            value
        }
        Event::Number { field, value } => {
            let (candidate, caption) = number(session, &config, *field, *value)?;
            label = caption;
            Some(candidate)
        }
        Event::Reset => {
            label = "deadzone: reset".into();
            Some(DeadzoneConfig::default())
        }
        _ => return None,
    }?;
    session.value = Action::Deadzone { config: candidate };
    session.draft = true;
    Some(Commit {
        action: session.value.clone(),
        label,
        group,
    })
}

fn number(
    session: &mut Session,
    config: &DeadzoneConfig,
    field: usize,
    value: f64,
) -> Option<(DeadzoneConfig, String)> {
    if !value.is_finite() {
        session.error = Some("Deadzone value must be finite.".into());
        return None;
    }
    let mut values = [
        config.low(),
        config.center_low(),
        config.center_high(),
        config.high(),
    ];
    let old = *values.get(field)?;
    values[field] = value;
    let name = ["low", "center_low", "center_high", "high"][field];
    let label = format!("deadzone: {name} {old:+.2} -> {value:+.2}");
    match DeadzoneConfig::new(values[0], values[1], values[2], values[3]) {
        Ok(value) => Some((value, label)),
        Err(error) => {
            session.error = Some(error.to_string());
            None
        }
    }
}
