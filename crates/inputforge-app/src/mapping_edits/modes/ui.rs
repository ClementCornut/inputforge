//! Existing mode workflow presentation transitions and native observations.
use super::Controller;
use dioxus::prelude::*;
use inputforge_gui_dx::mode_management::{Operation, Request, Scope, Transition, ui};

pub(super) fn transition(mut p: ui::Publication, scope: Scope, next: Transition) {
    match next {
        Transition::Add => p.adding.set(Some(scope)),
        Transition::Rename(name) => p.renaming.set(Some((scope, name))),
        Transition::Delete(target) => p.delete.set(Some(target)),
        Transition::Close(Some(name)) => p.focus.set(Some(name)),
        Transition::Close(None) => {}
    }
}
impl Controller {
    pub(super) fn mode_scope_current(&self, scope: &Scope) -> bool {
        let state = self.state.read();
        self.inner
            .borrow()
            .publication
            .is_none_or(|p| *p.mode.peek() == scope.editing)
            && !self.inner.borrow().disposed
            && state.mode_revision == scope.revision
            && state.active_profile.as_ref().is_some_and(|p| {
                scope.profile.as_ref() == Some(p.id()) && p.modes().contains(&scope.editing)
            })
    }
    pub(super) fn mode_ui(&self, request: ui::Request) -> bool {
        let ui::Request {
            mut publication,
            scope,
            event,
        } = request;
        if publication.error.try_peek().is_err() {
            return false;
        }
        match event {
            ui::Event::BeginAdd | ui::Event::Select(_) => {
                let operation = if let ui::Event::Select(name) = event {
                    Operation::Select(name)
                } else {
                    Operation::BeginAdd
                };
                match self.mode_action(Request {
                    scope: scope.clone(),
                    operation,
                }) {
                    Ok(next) => {
                        publication.error.set(None);
                        transition(publication, scope, next);
                        true
                    }
                    Err(error) => {
                        publication.error.set(Some(error));
                        false
                    }
                }
            }
            ui::Event::Context(name, event) => {
                event.prevent_default();
                event.stop_propagation();
                let point = event.client_coordinates();
                self.mode_menu_open(publication, scope, name, point.x, point.y);
                true
            }
            ui::Event::Key {
                name,
                dom_id,
                event,
            } => {
                self.mode_key(publication, scope, name, &dom_id, &event);
                true
            }
            ui::Event::Input { mut draft, value } => {
                if draft.value.try_peek().is_err() {
                    return false;
                }
                draft.value.set(value);
                draft.error.set(None);
                true
            }
            ui::Event::DraftKey { draft, event } => match event.key() {
                Key::Enter => {
                    event.prevent_default();
                    self.mode_draft(publication, scope, draft, false, false)
                }
                Key::Escape => {
                    event.prevent_default();
                    self.mode_draft(publication, scope, draft, false, true)
                }
                _ => true,
            },
            ui::Event::Commit { draft, blur } => {
                self.mode_draft(publication, scope, draft, blur, false)
            }
            ui::Event::CancelDraft(draft) => {
                self.mode_draft(publication, scope, draft, false, true)
            }
            ui::Event::MenuAction {
                operation,
                activation,
                error,
            } => self.mode_menu_action(scope, operation, activation, error),
            ui::Event::MenuClose {
                target,
                reason,
                activation,
                error,
            } => self.mode_menu_close(publication, target, reason, activation, error),
        }
    }
    pub(super) fn mode_menu_open(
        &self,
        mut p: ui::Publication,
        scope: Scope,
        name: String,
        left: f64,
        bottom: f64,
    ) {
        if p.menu.try_peek().is_err() || !self.mode_scope_current(&scope) {
            return;
        }
        p.menu.set(Some(ui::MenuTarget {
            scope,
            name,
            left,
            bottom,
        }));
    }
}
