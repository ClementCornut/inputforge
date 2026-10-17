//! Mode inline draft completion follows the existing validated transition.
use super::{Controller, ui::transition};
use dioxus::prelude::*;
use inputforge_gui_dx::mode_management::{Operation, Request, Scope, Transition, ui};
impl Controller {
    pub(super) fn mode_draft(
        &self,
        mut p: ui::Publication,
        scope: Scope,
        mut draft: ui::Draft,
        blur: bool,
        cancel: bool,
    ) -> bool {
        if draft.value.try_peek().is_err() {
            return false;
        }
        let operation = if cancel {
            Operation::Cancel(draft.from.clone())
        } else if let Some(from) = &draft.from {
            Operation::Rename {
                from: from.clone(),
                raw: draft.value.peek().clone(),
                blur,
            }
        } else {
            Operation::Add {
                raw: draft.value.peek().clone(),
                blur,
            }
        };
        match self.mode_action(Request {
            scope: scope.clone(),
            operation,
        }) {
            Ok(next @ Transition::Close(_)) => {
                transition(p, scope, next);
                if draft.from.is_some() {
                    p.renaming.set(None);
                } else {
                    p.adding.set(None);
                    draft.value.set(String::new());
                }
                draft.error.set(None);
                true
            }
            Ok(_) => true,
            Err(error) => {
                draft.error.set(Some(error));
                false
            }
        }
    }
}
