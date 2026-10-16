//! Current context validation and guard-free delivery before presentation changes.
use super::{Controller, Pending, policy};
use dioxus::prelude::{ReadableExt, WritableExt};
use inputforge_core::engine::EngineCommand;
use inputforge_gui_dx::mode_management::{Operation, Request, Transition};
pub(super) const STALE: &str = "This mode or profile changed. Reopen the action.";
impl Controller {
    /// Validate the current mode context and deliver the requested operation.
    ///
    /// # Errors
    /// Returns visible feedback for stale targets, core rejection or failed delivery.
    pub fn mode_action(&self, request: Request) -> Result<Transition, String> {
        if let Some(transition) = self.mode_dismiss(&request) {
            return Ok(transition);
        }
        let (command, transition, changes, affected, navigation) = {
            let state = self.state.read();
            let mut inner = self.inner.borrow_mut();
            inner.history.reconcile(&state, self.owner);
            let profile = inner.history.mode_profile(&state).ok_or(STALE)?;
            if inner.disposed
                || state.session.offline
                || request.scope.revision != state.mode_revision
                || request.scope.profile.as_ref() != Some(profile.id())
                || inner
                    .publication
                    .is_some_and(|p| *p.mode.peek() != request.scope.editing)
            {
                return Err(STALE.into());
            }
            let pending = inner.modes.as_ref().is_some_and(|p| {
                p.profile == *profile.id()
                    && !state
                        .mapping_edit_barrier
                        .is_some_and(|(owner, serial)| owner == self.owner && serial >= p.serial)
            });
            if !(profile.modes().contains(&request.scope.editing)
                || pending && matches!(request.operation, Operation::Select(_)))
            {
                return Err(STALE.into());
            }
            if pending && !matches!(request.operation, Operation::Select(_)) {
                return Err("A mode change is still pending. Try again after it finishes.".into());
            }
            let (command, transition) =
                super::edit::prepare(&profile, &policy::profile_view(&state, &profile), &request)?;
            let Some(command) = command else {
                if let Operation::Select(name) = &request.operation {
                    let name = name.clone();
                    drop(inner);
                    drop(state);
                    self.mode_navigate(&name);
                }
                return Ok(transition);
            };
            let affected = match &command {
                EngineCommand::RenameMode { from, .. } => Some(from.clone()),
                EngineCommand::DeleteMode { name } => Some(name.clone()),
                _ => None,
            };
            let navigation = match &command {
                EngineCommand::AddMode { name } => Some(name.clone()),
                EngineCommand::RenameMode { from, to } if request.scope.editing == *from => {
                    Some(to.clone())
                }
                EngineCommand::DeleteMode { name } if request.scope.editing == *name => {
                    Some(profile.settings().startup_mode().to_owned())
                }
                _ => None,
            };
            let changes = inner.history.mode_changes(&state, &command);
            (command, transition, changes, affected, navigation)
        };
        let structural = matches!(
            command,
            EngineCommand::AddMode { .. }
                | EngineCommand::RenameMode { .. }
                | EngineCommand::DeleteMode { .. }
        );
        self.commands
            .send(command)
            .map_err(|error| format!("Mode command could not be delivered: {error}. Try again."))?;
        if structural {
            let serial = self
                .fence()
                .ok_or("Mode command could not be delivered. Try again.")?;
            {
                let state = self.state.read();
                let mut inner = self.inner.borrow_mut();
                inner
                    .history
                    .mode_forward(&state, changes, affected.as_deref(), serial);
                inner.modes = request
                    .scope
                    .profile
                    .map(|profile| Pending { profile, serial });
            };
            self.publish();
        }
        if let Some(mode) = navigation {
            self.mode_navigate(&mode);
        }
        Ok(transition)
    }
    fn mode_navigate(&self, mode: &str) {
        let publication = self.inner.borrow().publication;
        if let Some(mut p) = publication
            && *p.mode.peek() != mode
        {
            p.mode.set(mode.to_owned());
            p.selection.set(None);
            self.end_group();
        }
    }
    fn mode_dismiss(&self, request: &Request) -> Option<Transition> {
        if matches!(
            request.operation,
            Operation::Cancel(_) | Operation::CloseMenu { .. }
        ) {
            let state = self.state.read();
            let name = match &request.operation {
                Operation::Cancel(name) => name.clone(),
                Operation::CloseMenu { name, tab } => (!tab).then(|| name.clone()),
                _ => None,
            };
            let focus = name.filter(|name| {
                !self.inner.borrow().disposed
                    && state.active_profile.as_ref().is_some_and(|p| {
                        request.scope.profile.as_ref() == Some(p.id()) && p.modes().contains(name)
                    })
            });
            return Some(Transition::Close(focus));
        }
        None
    }
}
