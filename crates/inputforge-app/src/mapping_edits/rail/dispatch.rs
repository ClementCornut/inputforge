//! Guard-free command delivery before history invalidation and navigation.
use super::{Controller, rejected};
use dioxus::prelude::{ReadableExt, WritableExt};
use inputforge_core::{action::Mapping, engine::EngineCommand};
use inputforge_gui_dx::{
    mapping_rail::{Operation, Outcome, Request},
    snapshots::MappingKey,
};
impl Controller {
    /// Validate and deliver a forward rail operation without optimistic engine mutation.
    #[must_use]
    pub fn rail(&self, request: Request) -> Outcome {
        if let Operation::Reorder {
            movement,
            order,
            group,
            filtered,
        } = &request.operation
        {
            let outcome = {
                let state = self.state.read();
                let Some(source) = self.current_target(&state, &request.target) else {
                    return rejected();
                };
                if self.reorder_blocked(&state, &request.target.scope, *group) {
                    return Outcome::NoOp;
                }
                super::reorder::prepare(&state, &source, *movement, order, *group, *filtered)
            };
            if let Outcome::Moved { index, .. } = outcome {
                if self
                    .commands
                    .send(EngineCommand::ReorderMapping {
                        input: request.target.before.input.clone(),
                        mode: request.target.before.mode.clone(),
                        target_index_in_group: index,
                    })
                    .is_err()
                {
                    return Outcome::Rejected(
                        "Mapping command could not be delivered. Try again.".into(),
                    );
                }
                let Some(serial) = self.fence() else {
                    return Outcome::Rejected(
                        "Mapping command could not be delivered. Try again.".into(),
                    );
                };
                let Some(profile) = request.target.scope.profile.clone() else {
                    return rejected();
                };
                self.inner.borrow_mut().reorders.insert(
                    (request.target.scope.mode.clone(), *group),
                    (profile, serial),
                );
                self.end_group();
            }
            return outcome;
        }
        let prepared = {
            let state = self.state.read();
            let Some(source) = self.current_target(&state, &request.target) else {
                return rejected();
            };
            match request.operation {
                Operation::Duplicate(mode) => {
                    let Some(profile) = state.active_profile.as_ref() else {
                        return rejected();
                    };
                    if mode == source.mode || !profile.modes().contains(&mode) {
                        return rejected();
                    }
                    let key = (mode.clone(), source.input.clone());
                    if self.inner.borrow().history.current(&state, &key).is_some() {
                        drop(state);
                        self.navigate(&key);
                        return Outcome::Opened;
                    }
                    let mapping = Mapping { mode, ..source };
                    ((mapping.mode.clone(), mapping.input.clone()), Some(mapping))
                }
                Operation::Delete => ((source.mode.clone(), source.input.clone()), None),
                Operation::Reorder { .. } => {
                    unreachable!("reorder handled before mutation preparation")
                }
            }
        };
        let (key, mapping) = prepared;
        if !self.forward(&key, mapping.as_ref()) {
            return Outcome::Rejected("Mapping command could not be delivered. Try again.".into());
        }
        let publication = self.inner.borrow().publication;
        if mapping.is_none()
            && let Some(mut p) = publication
            && p.selection.peek().as_ref() == Some(&key)
        {
            p.selection.set(None);
        }
        Outcome::Delivered
    }
    pub(super) fn forward(&self, key: &MappingKey, mapping: Option<&Mapping>) -> bool {
        if mapping.is_some_and(|m| {
            inputforge_core::profile::validate_mapping_action_tree(&m.input, &m.actions).is_err()
        }) {
            return false;
        }
        let command = mapping.map_or_else(
            || EngineCommand::RemoveMapping {
                mode: key.0.clone(),
                input: key.1.clone(),
            },
            |m| EngineCommand::SetMapping {
                mode: m.mode.clone(),
                input: m.input.clone(),
                name: m.name.clone(),
                actions: m.actions.clone(),
            },
        );
        if self.commands.send(command).is_err() {
            return false;
        }
        let Some(serial) = self.fence() else {
            return false;
        };
        {
            let state = self.state.read();
            let mut inner = self.inner.borrow_mut();
            inner.history.forward(&state, key, mapping.cloned(), serial);
        };
        self.publish();
        true
    }
}
