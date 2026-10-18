//! Entry actions wait for the profile panel that owns their native picker.
use super::Controller;
use dioxus::prelude::*;
use inputforge_gui_dx::{PanelSlot, profiles::Intent};
use std::rc::Rc;
impl Controller {
    pub(super) fn start(&self, mut panel: Signal<PanelSlot>, intent: Intent) {
        if !matches!(intent, Intent::New | Intent::Pick) {
            return;
        }
        let owner = self.session.borrow().as_ref().map(|s| s.owner);
        if let Some(owner) = owner {
            let _ = self.dispatch(owner, intent);
        } else {
            *self.entry.borrow_mut() = Some(intent);
            let entry = Rc::clone(&self.entry);
            // A different panel may win before the requested panel mounts.
            dioxus::core::queue_effect(move || {
                entry.borrow_mut().take();
            });
        }
        panel.set(PanelSlot::Profiles);
    }
}
