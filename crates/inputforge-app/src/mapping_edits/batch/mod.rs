//! Batch wizard lifecycle, current-state validation and rendering publication.
mod dispatch;
mod edit;
mod policy;
mod rows;
use super::Controller;
use dioxus::prelude::{Callback, ReadableExt, WritableExt};
use inputforge_core::{
    profile::{Profile, ProfileId},
    state::AppState,
};
use inputforge_gui_dx::batch_mapping::{Bindings, Publication, Request, View};
#[derive(Debug)]
pub(super) struct Wizard {
    publication: Publication,
    view: View,
    stamp: Stamp,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct Stamp {
    profile: Option<ProfileId>,
    navigation: String,
    modes: Vec<String>,
    mode: String,
    all_modes: bool,
    source: Option<(inputforge_core::types::DeviceId, u8, u8, u8)>,
    binding: Option<inputforge_core::profile::controllers::DeviceBinding>,
    target: Option<inputforge_core::types::VirtualDeviceConfig>,
}
impl Stamp {
    fn new(state: &AppState, view: &View, navigation: String) -> Self {
        Self {
            profile: state.active_profile.as_ref().map(Profile::id).cloned(),
            navigation,
            mode: view.mode.clone(),
            all_modes: view.all_modes,
            modes: state
                .active_profile
                .as_ref()
                .map_or_else(Vec::new, |p| p.modes().as_slice().to_vec()),
            source: state
                .devices
                .iter()
                .find(|d| d.connected && Some(&d.info.id) == view.source.as_ref())
                .map(|d| (d.info.id.clone(), d.info.axes, d.info.buttons, d.info.hats)),
            binding: view
                .source
                .as_ref()
                .and_then(|id| rows::binding(state, id))
                .cloned(),
            target: state
                .virtual_devices
                .iter()
                .find(|d| Some(d.device_id) == view.target)
                .cloned(),
        }
    }
}
impl Controller {
    pub(super) fn batch_bindings(&self) -> Bindings {
        let install = self.clone();
        let dispatch = self.clone();
        let dispose = self.clone();
        Bindings {
            install: Callback::new(move |publication: Publication| {
                let owner = uuid::Uuid::new_v4().as_u128();
                let state = install.state.read();
                let mode = publication.mode.peek().clone();
                let mut view = View {
                    owner,
                    mode: mode.clone(),
                    ..View::default()
                };
                rows::reconcile(&state, &mut view);
                let stamp = Stamp::new(&state, &view, mode);
                install.inner.borrow_mut().batch = Some(Wizard {
                    publication,
                    view,
                    stamp,
                });
                drop(state);
                install.batch_refresh();
                owner
            }),
            dispatch: Callback::new(move |request| dispatch.batch_action(request)),
            dispose: Callback::new(move |owner| {
                let mut inner = dispose.inner.borrow_mut();
                if inner.batch.as_ref().is_some_and(|b| b.view.owner == owner) {
                    inner.batch = None;
                }
            }),
        }
    }
    pub(super) fn batch_refresh(&self) {
        let publication = {
            let Some(state) = self.state.try_read() else {
                return;
            };
            let mut inner = self.inner.borrow_mut();
            inner.history.reconcile(&state, self.owner);
            let Some(mut batch) = inner.batch.take() else {
                return;
            };
            let navigation = batch.publication.mode.peek().clone();
            let stamp = Stamp::new(&state, &batch.view, navigation.clone());
            if stamp.profile != batch.stamp.profile || stamp.navigation != batch.stamp.navigation {
                batch.view.mode = navigation;
                batch.view.all_modes = false;
                batch.view.rows.clear();
                batch.view.source = None;
                batch.view.target = None;
            }
            rows::reconcile(&state, &mut batch.view);
            let stamp = Stamp::new(&state, &batch.view, batch.publication.mode.peek().clone());
            if batch.stamp != stamp {
                batch.view.generation += 1;
                batch.view.error = None;
                batch.stamp = stamp;
            }
            policy::project(&state, &inner.history, &mut batch.view);
            let publication = (batch.publication, batch.view.clone());
            inner.batch = Some(batch);
            publication
        };
        publish(publication.0, publication.1);
    }
    fn batch_action(&self, request: Request) {
        let delivery = {
            let state = self.state.read();
            let mode_pending = self.mode_pending(&state);
            let mut inner = self.inner.borrow_mut();
            if inner.disposed {
                return;
            }
            inner.history.reconcile(&state, self.owner);
            let Some(mut batch) = inner.batch.take() else {
                return;
            };
            if request.owner != batch.view.owner {
                inner.batch = Some(batch);
                return;
            }
            let current = Stamp::new(&state, &batch.view, batch.publication.mode.peek().clone());
            let valid = !mode_pending
                && request.generation == batch.view.generation
                && current == batch.stamp
                && current.profile.is_some()
                && !state.session.offline
                && current.modes.contains(&batch.view.mode)
                && rows::valid(&state, &batch.view);
            let delivery = if valid {
                edit::apply(&state, &inner.history, &mut batch, request.intent)
            } else {
                batch.view.error = Some("This batch context changed or is no longer available. Review the wizard and try again.".into());
                None
            };
            policy::project(&state, &inner.history, &mut batch.view);
            let publication = (batch.publication, batch.view.clone());
            inner.batch = Some(batch);
            (delivery, publication)
        };
        publish(delivery.1.0, delivery.1.1);
        if let Some((entries, label)) = delivery.0 {
            self.batch_deliver(request.owner, entries, label);
        }
    }
}
fn publish(publication: Publication, view: View) {
    let mut signal = publication.view;
    if signal.try_peek().is_ok_and(|old| *old != view)
        && let Ok(mut writer) = signal.try_write()
    {
        *writer = view;
    }
}
