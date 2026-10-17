//! Coherent action-tree readouts and deadline-based preview publication.
mod activity;
mod analyze;
#[cfg(test)]
mod analyzer_tests;
mod inputs;
mod outputs;
mod polarity;
mod predicates;
mod terminal;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod values_tests;
mod walk;
pub(super) use activity::{Activity, Change};
use inputforge_core::{
    pipeline::evaluate_actions_through,
    state::{AppState, EngineStatus},
    types::InputValue,
};
use inputforge_gui_dx::{
    readouts::{Key, Readout, ReadoutDisplay},
    snapshots::{ConfigSnapshot, LiveSnapshot},
};
/// Complete display values using only data from the existing engine guard.
pub(super) fn project(state: &AppState, cfg: &ConfigSnapshot, live: &mut LiveSnapshot) {
    inputs::project(cfg, live);
    let Some(mapping) = cfg.selected_mapping_key.as_ref() else {
        return;
    };
    let Some(actions) = cfg.selected_mapping_actions.as_ref() else {
        return;
    };
    let model = analyze::analyze(actions, &mapping.1, state, cfg);
    for address in &model.pipeline_inputs {
        live.input_readouts
            .entry(address.clone())
            .or_insert_with(|| inputs::missing(address));
    }
    let connected = mapping.1.device().is_some_and(|id| {
        state
            .devices
            .iter()
            .any(|d| &d.info.id == id && d.connected)
    });
    let instruments = actions
        .iter()
        .enumerate()
        .map(|(index, action)| {
            if !connected
                || !matches!(
                    action,
                    inputforge_core::action::Action::ResponseCurve { .. }
                        | inputforge_core::action::Action::Deadzone { .. }
                )
            {
                return None;
            }
            match evaluate_actions_through(actions, state, &mapping.1, index) {
                InputValue::Axis { value, .. } => Some(value.value()),
                _ => None,
            }
        })
        .collect();
    let mut readout = Readout {
        key: Key {
            profile: state.active_profile.as_ref().map(|p| p.id().clone()),
            path: state.profile_path.clone(),
            mapping: mapping.clone(),
            actions: actions.clone(),
            generation: state.session.generation,
        },
        model,
        outputs: Vec::new(),
        instruments,
        running: matches!(state.engine_status, EngineStatus::Running),
    };
    complete(&mut readout, live, cfg);
    live.readout = Some(readout);
}
fn complete(readout: &mut Readout, live: &LiveSnapshot, cfg: &ConfigSnapshot) {
    use inputforge_gui_dx::readouts::OutputDestination;
    readout.outputs = readout
        .model
        .outputs
        .iter()
        .map(|output| match &output.destination {
            OutputDestination::VJoy(address) => {
                outputs::read_output_typed_display(address, live, cfg, output.polarity)
            }
            OutputDestination::Keyboard { pressed, .. } => ReadoutDisplay::Button {
                pressed: readout.running && output.is_active && *pressed,
            },
            OutputDestination::Mouse { target, active, .. } => ReadoutDisplay::Button {
                pressed: !target.is_wheel() && readout.running && output.is_active && *active,
            },
        })
        .collect();
}

pub(super) fn overlay(
    readout: &mut Readout,
    activity: &[inputforge_gui_dx::snapshots::OutputActivitySnapshot],
) {
    use inputforge_core::{processing::into_natural_domain, state::OutputActivityValue};
    use inputforge_gui_dx::readouts::AxisDisplay;
    for (output, display) in readout.model.outputs.iter_mut().zip(&mut readout.outputs) {
        let Some(value) = activity
            .iter()
            .find(|entry| entry.owner == output.owner)
            .map(|entry| entry.value)
        else {
            continue;
        };
        output.activity = Some(value);
        output.is_active = true;
        *display = match value {
            OutputActivityValue::Axis(value) => ReadoutDisplay::Axis(AxisDisplay {
                value: into_natural_domain(value, output.polarity),
                polarity: output.polarity,
            }),
            OutputActivityValue::Button(pressed) => ReadoutDisplay::Button { pressed },
            OutputActivityValue::Hat(direction) => ReadoutDisplay::Hat { direction },
            OutputActivityValue::Keyboard(pressed) | OutputActivityValue::Mouse(pressed) => {
                ReadoutDisplay::Button {
                    pressed: readout.running
                        && pressed
                        && !matches!(output.destination,inputforge_gui_dx::readouts::OutputDestination::Mouse {target,..} if target.is_wheel()),
                }
            }
        };
    }
}
