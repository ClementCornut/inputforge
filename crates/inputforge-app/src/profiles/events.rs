use super::Controller;
use dioxus::prelude::*;
use inputforge_gui_dx::profiles::Intent;
impl Controller {
    pub(super) fn rename_key(&self, owner: u64, workflow: u64, event: &KeyboardEvent) {
        match event.key() {
            Key::Enter => {
                event.prevent_default();
                event.stop_propagation();
                self.dispatch_captured(owner, workflow, Intent::Rename);
            }
            Key::Escape => {
                event.prevent_default();
                event.stop_propagation();
                self.dispatch_captured(owner, workflow, Intent::CancelRename);
            }
            _ => {}
        }
    }
    pub(super) fn focus(&self, owner: u64, workflow: u64, event: MountedEvent) {
        let c = self.clone();
        spawn(async move {
            if c.session.borrow().as_ref().is_some_and(|s| {
                s.owner == owner
                    && s.view.peek().workflow == workflow
                    && (s.view.peek().rename.is_some() || s.view.peek().delete.is_some())
            }) {
                let _ = event.set_focus(true).await;
            }
        });
    }
}
