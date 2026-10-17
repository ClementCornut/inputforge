//! Reconcile actual profile identity without overriding pending mode delivery.
use super::{Controller, expansion};
use dioxus::prelude::*;
impl Controller {
    pub(in crate::mapping_edits) fn reconcile_navigation(&self) {
        let Some(state) = self.state.try_read() else {
            return;
        };
        let pending = self.mode_pending(&state);
        let (mut p, profile_changed, mode_changed, mode, missing) = {
            let mut inner = self.inner.borrow_mut();
            if inner.disposed {
                return;
            }
            let current_selection = inner
                .chrome
                .publication
                .and_then(|p| p.selected_mapping.try_peek().ok().map(|s| s.clone()))
                .flatten();
            let current_mapping = current_selection
                .as_ref()
                .and_then(|key| inner.history.current(&state, key));
            let chrome = &mut inner.chrome;
            let Some(p) = chrome.publication else { return };
            if p.editing_mode.try_peek().is_err() {
                return;
            }
            let profile = state.active_profile.as_ref();
            let id = profile.map(|p| p.id().clone());
            let changed = chrome.profile != id;
            let mode = if changed {
                profile
                    .map_or("Default", |p| p.settings().startup_mode())
                    .to_owned()
            } else {
                p.editing_mode.peek().clone()
            };
            let mode_changed = chrome.mode != mode;
            let selection_changed = chrome.selection != current_selection;
            chrome.selection = current_selection;
            if (changed || mode_changed || selection_changed)
                && let Some(editor) = chrome.editor
            {
                expansion::reset(editor);
            }
            let missing = !pending
                && p.selected_mapping
                    .peek()
                    .as_ref()
                    .is_some_and(|key| key.0 != mode || current_mapping.is_none());
            chrome.profile = id;
            chrome.mode.clone_from(&mode);
            (p, changed, mode_changed, mode, missing)
        };
        drop(state);
        if profile_changed && *p.editing_mode.peek() != mode {
            p.editing_mode.set(mode);
        }
        if (profile_changed || mode_changed || missing) && p.selected_mapping.peek().is_some() {
            p.selected_mapping.set(None);
        }
    }
}
