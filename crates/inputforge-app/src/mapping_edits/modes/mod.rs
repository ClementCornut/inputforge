//! Mode workflows share the mounted editor's navigation, queue and history.
mod dispatch;
mod edit;
pub(crate) mod policy;
use super::Controller;
use dioxus::prelude::{Callback, ReadableExt, WritableExt};
use inputforge_core::profile::ProfileId;
use inputforge_gui_dx::mode_management::Bindings;
#[derive(Debug)]
pub(super) struct Pending {
    pub profile: ProfileId,
    pub serial: u64,
}
impl Controller {
    pub(super) fn mode_pending(&self, state: &inputforge_core::state::AppState) -> bool {
        self.inner.borrow().modes.as_ref().is_some_and(|pending| {
            state
                .active_profile
                .as_ref()
                .is_some_and(|p| p.id() == &pending.profile)
                && !state
                    .mapping_edit_barrier
                    .is_some_and(|(owner, serial)| owner == self.owner && serial >= pending.serial)
        })
    }

    pub(super) fn mode_bindings(&self) -> Bindings {
        let dispatch = self.clone();
        let reconcile = self.clone();
        Bindings {
            dispatch: Callback::new(move |r| dispatch.mode_action(r)),
            reconcile: Callback::new(move |()| {
                reconcile.reconcile();
                let publication = reconcile.inner.borrow().publication;
                if let Some(mut p) = publication {
                    let stale = p
                        .selection
                        .peek()
                        .as_ref()
                        .is_some_and(|key| key.0 != *p.mode.peek());
                    if stale {
                        p.selection.set(None);
                    }
                }
            }),
        }
    }
    pub(super) fn reconcile_modes(&self) {
        let Some(state) = self.state.try_read() else {
            return;
        };
        let (publication, next) = {
            let mut inner = self.inner.borrow_mut();
            if inner.disposed {
                return;
            }
            let Some(p) = inner.publication else { return };
            if let Some(pending) = &inner.modes {
                if state
                    .active_profile
                    .as_ref()
                    .is_some_and(|p| p.id() == &pending.profile)
                    && !state.mapping_edit_barrier.is_some_and(|(owner, serial)| {
                        owner == self.owner && serial >= pending.serial
                    })
                {
                    return;
                }
                inner.modes = None;
            }
            let current = p.mode.peek().clone();
            let next = state.active_profile.as_ref().map(|profile| {
                if profile.modes().contains(&current) {
                    current.clone()
                } else if profile.modes().contains(profile.settings().startup_mode()) {
                    profile.settings().startup_mode().to_owned()
                } else {
                    profile.modes().first().to_owned()
                }
            });
            (p, next)
        };
        drop(state);
        if let Some(mode) = next {
            let mut p = publication;
            if *p.mode.peek() != mode {
                p.mode.set(mode);
                p.selection.set(None);
            }
        }
    }
}
