//! Root snapshot shortcut ownership and current-profile validation.
mod listener;
use super::{Controller, Scope};
use dioxus::prelude::*;
use inputforge_core::engine::{EngineCommand, SnapshotAction, SnapshotRequest};
#[derive(Debug)]
pub(super) struct Session {
    owner: u64,
    generation: u64,
    scope: Scope,
    error: Callback<String>,
    listener: listener::Listener,
}
impl Controller {
    pub(in crate::profiles) fn mount_shortcut(&self, error: Callback<String>) -> u64 {
        if let Some(old) = self.shortcut.borrow_mut().take() {
            old.listener.close();
        }
        *self.serial.borrow_mut() += 1;
        let owner = *self.serial.borrow();
        let scope = self
            .state
            .try_read()
            .map(|s| Scope::from_state(&s))
            .unwrap_or_default();
        let listener = listener::listen(self.clone(), owner);
        *self.shortcut.borrow_mut() = Some(Session {
            owner,
            generation: 1,
            scope,
            error,
            listener,
        });
        owner
    }
    pub(in crate::profiles) fn unmount_shortcut(&self, owner: u64) {
        let mut shortcut = self.shortcut.borrow_mut();
        if shortcut.as_ref().is_some_and(|s| s.owner == owner) {
            let old = shortcut
                .take()
                .expect("shortcut owner exists after matching it");
            drop(shortcut);
            old.listener.close();
        }
    }
    pub(in crate::profiles) fn reconcile_shortcut(&self) {
        let Some(state) = self.state.try_read() else {
            return;
        };
        let scope = Scope::from_state(&state);
        drop(state);
        if let Some(shortcut) = self.shortcut.borrow_mut().as_mut()
            && shortcut.scope != scope
        {
            shortcut.scope = scope;
            shortcut.generation += 1;
            shortcut.listener.arm(shortcut.generation);
        }
    }
    fn shortcut_payload(&self, owner: u64, generation: u64) {
        let mut shortcuts = self.shortcut.borrow_mut();
        let Some(shortcut) = shortcuts
            .as_mut()
            .filter(|s| s.owner == owner && s.generation == generation)
        else {
            return;
        };
        let Some(state) = self.state.try_read() else {
            let feedback = shortcut.error;
            drop(shortcuts);
            feedback.call("Snapshots are busy. Try again.".into());
            return;
        };
        if shortcut.scope != Scope::from_state(&state) {
            return;
        }
        let command = shortcut
            .scope
            .profile
            .clone()
            .zip(shortcut.scope.path.clone())
            .map(|(profile, path)| {
                EngineCommand::Snapshot(Box::new(SnapshotRequest {
                    profile,
                    path,
                    action: SnapshotAction::Manual,
                }))
            });
        drop(state);
        let result = command
            .ok_or("Load a profile before creating a snapshot.")
            .and_then(|command| {
                self.commands
                    .send(command)
                    .map_err(|_closed| "Could not send snapshot action. Try again.")
            });
        let feedback = shortcut.error;
        drop(shortcuts);
        if let Err(error) = result {
            feedback.call(error.into());
        }
    }
}
