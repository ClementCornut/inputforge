//! Prepare validated mapping snapshots while the engine guard is held.
use super::{
    Controller,
    history::{self, Entry},
    paths,
};
use dioxus::prelude::ReadableExt;
use inputforge_gui_dx::{
    mapping_edits::{Edit, Entry as Label, Request, UndoKind},
    snapshots::MappingKey,
};
impl Controller {
    pub(super) fn prepare(
        &self,
        request: &mut Request,
        captured: bool,
    ) -> Option<(MappingKey, Entry)> {
        let state = self.state.read();
        if self.mode_pending(&state) {
            return None;
        }
        let mut inner = self.inner.borrow_mut();
        inner.history.reconcile(&state, self.owner);
        let profile = state.active_profile.as_ref()?;
        if state.session.offline
            || request.profile.as_ref() != Some(profile.id())
            || !profile.modes().contains(&request.before.mode)
        {
            return None;
        }
        let key = paths::key(&request.before);
        if let Some(p) = inner.publication
            && (*p.mode.peek() != key.0
                || (request.editor && p.selection.peek().as_ref() != Some(&key)))
        {
            return None;
        }
        let actual = history::saved(&state, &key);
        if !request.editor && !matches!(request.edit, Edit::Rename(_)) {
            return None;
        }
        // Legacy rail dispatch only carries metadata; inline drafts carry a full target.
        if !request.editor && !captured {
            let current = actual.as_ref()?;
            if current.name != request.before.name {
                return None;
            }
            request.before.actions.clone_from(&current.actions);
        }
        let effective = inner.history.current(&state, &key);
        if actual.as_ref() != Some(&request.before)
            && effective.as_ref() != Some(&request.before)
            && !inner.history.accepts_stale(&key, &request.before)
        {
            return None;
        }
        if effective.as_ref() != Some(&request.before) && inner.history.structural_pending(&key) {
            return None;
        }
        let mut after = effective?;
        request.edit = super::editor::resolve(&request.edit, &after.actions, &state, &after.input)?;
        if !paths::preserves_children(&request.before.actions, &after.actions, &request.edit) {
            return None;
        }
        let structural = paths::structural(&after.actions, &request.edit);
        let actions = paths::apply(&after.actions, &request.edit)?;
        let kind = match &request.edit {
            Edit::Replace { .. } => UndoKind::StageEdit,
            Edit::Field { .. } | Edit::Add { .. } | Edit::Step { .. } => return None,
            Edit::Insert { .. } => UndoKind::StageAdd,
            Edit::Remove(_) => UndoKind::StageRemove,
            Edit::Move { .. } => UndoKind::StageReorder,
            Edit::Rename(name) => {
                let name = name.trim();
                if name.is_empty() {
                    return None;
                }
                after.name = Some(name.into());
                UndoKind::Rename
            }
            Edit::Rebind(input) => {
                if &after.input == input {
                    return None;
                }
                after.input = input.clone();
                UndoKind::Rebind
            }
        };
        let target = paths::key(&after);
        let before = inner.history.current(&state, &target);
        after.actions = actions;
        if inputforge_core::profile::validate_mapping_action_tree(&after.input, &after.actions)
            .is_err()
        {
            return None;
        }
        Some((
            key,
            Entry {
                target,
                before,
                after: Some(after),
                label: Label {
                    kind,
                    label: request.label.clone(),
                },
                structural,
            },
        ))
    }
    pub(super) fn prepare_restore(&self, key: &MappingKey, redo: bool) -> Option<Entry> {
        let state = self.state.read();
        if self.mode_pending(&state) {
            return None;
        }
        if state.session.offline {
            return None;
        }
        let mut inner = self.inner.borrow_mut();
        inner.history.reconcile(&state, self.owner);
        if let Some(p) = inner.publication
            && (p.selection.peek().as_ref() != Some(key) || *p.mode.peek() != key.0)
        {
            return None;
        }
        let stack = inner.history.stacks.get(key)?;
        let entry = (if redo { &stack.redo } else { &stack.undo }).last()?;
        let expected = if redo { &entry.before } else { &entry.after };
        if &inner.history.current(&state, &entry.target) != expected {
            return None;
        }
        Some(entry.clone())
    }
}
