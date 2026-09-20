//! Mapping failures are local; valid mappings keep routing.
use super::{Engine, output_state::OwnerScopeKey, validation};
use crate::{
    action::Action,
    error::Result,
    types::{InputAddress, OutputId},
};
impl Engine {
    pub(super) fn refresh_mapping_issues(&mut self) -> Result<()> {
        let issues = validation::issues(&self.state.read());
        let previous = self.state.read().session.mapping_issues.clone();
        for issue in &issues {
            if !previous
                .iter()
                .any(|old| old.input == issue.input && old.mode == issue.mode)
            {
                self.release_mapping(&issue.input, &issue.mode)?;
            }
        }
        if issues != previous {
            self.pending_output_refresh = true;
        }
        self.state.write().session.mapping_issues = issues;
        Ok(())
    }

    pub(super) fn release_mapping(&mut self, input: &InputAddress, mode: &str) -> Result<()> {
        let (profile, actions, output_active) = {
            let state = self.state.read();
            (
                state
                    .profile_path
                    .as_ref()
                    .map_or_else(|| "memory-profile".into(), |p| p.display().to_string()),
                state
                    .active_profile
                    .as_ref()
                    .and_then(|p| {
                        p.mappings()
                            .iter()
                            .find(|m| m.input == *input && m.mode == mode)
                    })
                    .map_or_else(Vec::new, |m| m.actions.clone()),
                state.session.output_active,
            )
        };
        let scope = OwnerScopeKey::new(profile, mode, input.clone());
        for action in self
            .output_state
            .reconcile_absent_owners_for_scope(&scope, &[])
        {
            super::output_handler::dispatch_output_action(
                action,
                &mut self.output_state,
                self.keyboard.as_mut(),
                self.mouse.as_mut(),
            )?;
        }
        for (owner, address) in self.output_state.absent_set_buttons_for_scope(&scope, &[]) {
            self.release_virtual_owner(owner, address)?;
        }
        if output_active {
            for address in axis_outputs(&actions) {
                if let OutputId::Axis { id } = address.output {
                    if self.state.read().output_cache.get_axis(address.device, id) == 0.0 {
                        continue;
                    }
                    self.output.set_axis(address.device, id, 0.0)?;
                    self.state
                        .write()
                        .output_cache
                        .set_axis(address.device, id, 0.0);
                }
            }
        }
        // A removed/invalid shift mapping must not leave a temporary mode held.
        let mode_before = self.mode_state.current().to_owned();
        for callback in self.callbacks.fire(input) {
            if matches!(
                callback,
                crate::callbacks::ReleaseCallback::PopTemporaryMode
            ) {
                self.mode_state.pop_temporary();
            }
        }
        if self.mode_state.current() != mode_before {
            self.release_all_held_outputs()?;
            self.gesture_dispatcher.clear_all();
            self.pending_output_refresh = true;
        }
        self.mode_state
            .current()
            .clone_into(&mut self.state.write().current_mode);
        self.clear_gesture_mapping(input, mode);
        Ok(())
    }
}
fn axis_outputs(actions: &[Action]) -> Vec<crate::types::OutputAddress> {
    let mut out = Vec::new();
    for action in actions {
        match action {
            Action::MapToVJoy { output } if matches!(output.output, OutputId::Axis { .. }) => {
                out.push(output.clone());
            }
            Action::Conditional {
                if_true, if_false, ..
            } => {
                out.extend(axis_outputs(if_true));
                out.extend(axis_outputs(if_false));
            }
            Action::TapGesture {
                single_tap,
                double_tap,
                ..
            } => {
                out.extend(axis_outputs(single_tap));
                out.extend(axis_outputs(double_tap));
            }
            Action::PressGesture {
                short_press,
                long_press,
                ..
            } => {
                out.extend(axis_outputs(short_press));
                out.extend(axis_outputs(long_press));
            }
            _ => {}
        }
    }
    out
}
