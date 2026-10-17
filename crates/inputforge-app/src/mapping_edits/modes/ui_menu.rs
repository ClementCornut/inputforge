//! Mode menu activation and dismissal retain failed workflow feedback.
use super::{Controller, ui::transition};
use dioxus::prelude::*;
use inputforge_gui_dx::{
    mode_management::{Operation, Request, Scope, Transition, ui},
    overlays::CloseReason,
};
impl Controller {
    pub(super) fn mode_menu_action(
        &self,
        scope: Scope,
        operation: Operation,
        mut activation: Signal<Option<Transition>>,
        mut error: Signal<Option<String>>,
    ) -> bool {
        if error.try_peek().is_err() {
            return false;
        }
        match self.mode_action(Request { scope, operation }) {
            Ok(next) => {
                activation.set(Some(next));
                error.set(None);
                true
            }
            Err(message) => {
                activation.set(None);
                error.set(Some(message));
                false
            }
        }
    }
    pub(super) fn mode_menu_close(
        &self,
        mut p: ui::Publication,
        target: ui::MenuTarget,
        reason: CloseReason,
        activation: Signal<Option<Transition>>,
        mut error: Signal<Option<String>>,
    ) -> bool {
        if error.try_peek().is_err() || p.menu.try_peek().is_err() {
            return false;
        }
        let next = if matches!(reason, CloseReason::ItemActivated) {
            let Some(next) = activation.peek().clone() else {
                return false;
            };
            next
        } else {
            match self.mode_action(Request {
                scope: target.scope.clone(),
                operation: Operation::CloseMenu {
                    name: target.name,
                    tab: matches!(reason, CloseReason::Tab),
                },
            }) {
                Ok(next) => next,
                Err(message) => {
                    error.set(Some(message));
                    return false;
                }
            }
        };
        p.menu.set(None);
        transition(p, target.scope, next);
        true
    }
    pub(super) fn mode_delete(&self, request: ui::DeleteRequest) -> bool {
        let ui::DeleteRequest {
            mut shell,
            mut error,
            confirm,
        } = request;
        if error.try_peek().is_err() || shell.delete.try_peek().is_err() {
            return false;
        }
        let Some(target) = shell.delete.peek().clone() else {
            return true;
        };
        let operation = if confirm {
            Operation::Delete(target.clone())
        } else {
            Operation::Cancel(Some(target.name))
        };
        match self.mode_action(Request {
            scope: target.scope,
            operation,
        }) {
            Ok(Transition::Close(focus)) => {
                if let Some(name) = focus {
                    shell.focus.set(Some(name));
                }
                shell.delete.set(None);
                error.set(None);
                true
            }
            Ok(_) => false,
            Err(message) => {
                error.set(Some(message));
                false
            }
        }
    }
    pub(super) fn mode_key(
        &self,
        mut publication: ui::Publication,
        scope: Scope,
        name: String,
        dom_id: &str,
        event: &KeyboardEvent,
    ) {
        if event.key() == Key::Delete {
            event.prevent_default();
            event.stop_propagation();
            match self.mode_action(Request {
                scope: scope.clone(),
                operation: Operation::BeginDelete(name),
            }) {
                Ok(next) => {
                    publication.error.set(None);
                    transition(publication, scope, next);
                }
                Err(error) => publication.error.set(Some(error)),
            }
        } else if event.key() == Key::F10 && event.modifiers().shift() {
            event.prevent_default();
            event.stop_propagation();
            let controller = self.clone();
            let id = serde_json::to_string(dom_id).expect("DOM id is serializable");
            spawn(async move {
                let mut handle = document::eval(&format!(
                    "var el = document.getElementById({id}); if (!el) {{ dioxus.send(null); return; }} var r = el.getBoundingClientRect(); dioxus.send([r.left,r.bottom]);"
                ));
                if let Ok(Some([left, bottom])) = handle.recv::<Option<[f64; 2]>>().await {
                    controller.mode_menu_open(publication, scope, name, left, bottom);
                }
            });
        }
    }
}
