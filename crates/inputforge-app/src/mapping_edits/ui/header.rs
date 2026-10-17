//! Header and inline rail rename keyboard, draft and completion policy.
use super::Controller;
use dioxus::prelude::*;
use inputforge_gui_dx::{
    mapping_edits::{Edit, Inline, ui},
    mapping_rail::{Scope, Target},
};
impl Controller {
    pub(super) fn header_ui(&self, request: ui::HeaderRequest) {
        let ui::HeaderRequest {
            mut publication,
            profile,
            target,
            event,
        } = request;
        if publication.value.try_peek().is_err() {
            return;
        }
        let arm = match &event {
            ui::HeaderEvent::ArmKey(event) => {
                if event.key() != Key::F2 {
                    return;
                }
                event.prevent_default();
                true
            }
            ui::HeaderEvent::ArmMouse(event) => {
                event.prevent_default();
                true
            }
            _ => false,
        };
        if arm {
            publication
                .value
                .set(target.name.clone().unwrap_or_default());
            publication.draft.set(target);
            publication.armed.set(true);
            return;
        }
        let cancel = match event {
            ui::HeaderEvent::Input(value) => {
                publication.value.set(value);
                return;
            }
            ui::HeaderEvent::Key(event) => match event.key() {
                Key::Enter => {
                    event.prevent_default();
                    let _ = document::eval(
                        "const el = document.activeElement; if (el && el instanceof HTMLInputElement) { el.blur(); }",
                    );
                    return;
                }
                Key::Escape => {
                    event.prevent_default();
                    true
                }
                _ => return,
            },
            ui::HeaderEvent::Blur => false,
            _ => return,
        };
        let before = publication.draft.peek().clone();
        let raw = publication.value.peek().clone();
        let result = self.inline(Inline {
            target: Some(Target {
                scope: Scope {
                    profile,
                    mode: before.mode.clone(),
                },
                before: before.clone(),
            }),
            edit: Edit::Rename(raw.clone()),
            label: format!(
                "rename: '{}' -> '{}'",
                before.name.as_deref().unwrap_or_default(),
                raw
            ),
            editor: true,
            cancel,
        });
        publication.error.set(result.clone().err());
        if result.is_ok() {
            if cancel {
                publication.value.set(before.name.unwrap_or_default());
            }
            publication.armed.set(false);
        }
    }
    pub(super) fn rename_ui(
        &self,
        mut p: ui::Rename,
        event: ui::RenameEvent,
    ) -> Result<(), String> {
        if p.value.try_peek().is_err() {
            return Err("This editor is no longer mounted.".into());
        }
        let cancel = match event {
            ui::RenameEvent::Input(value) => {
                p.value.set(value);
                return Ok(());
            }
            ui::RenameEvent::Key(event) => match event.key() {
                Key::Enter => {
                    event.prevent_default();
                    false
                }
                Key::Escape => {
                    event.prevent_default();
                    true
                }
                _ => return Ok(()),
            },
            ui::RenameEvent::Commit => false,
        };
        let raw = p.value.peek().clone();
        let result = self.inline(Inline {
            target: p.target.clone(),
            edit: Edit::Rename(raw.clone()),
            label: format!(
                "rename: '{}' -> '{}'",
                p.target
                    .as_ref()
                    .and_then(|t| t.before.name.as_deref())
                    .unwrap_or_default(),
                raw
            ),
            editor: false,
            cancel,
        });
        p.error.set(result.clone().err());
        if result.is_ok() {
            p.open.set(None);
        }
        result.map(|_| ())
    }
}
