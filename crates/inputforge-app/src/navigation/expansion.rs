//! Expansion and draft invalidation after accepted structural edits or navigation.
use super::Controller;
use dioxus::prelude::*;
use inputforge_gui_dx::{
    mapping_edits::{Edit, StageId, StageIdSegment},
    navigation::EditorPublication,
};
pub(super) fn reset(mut p: EditorPublication) {
    if p.expanded.try_peek().is_err() {
        return;
    }
    if !p.expanded.peek().is_empty() {
        p.expanded.write().clear();
    }
    if !p.hints.peek().is_empty() {
        p.hints.write().clear();
    }
    if !p.tags.peek().is_empty() {
        p.tags.write().clear();
    }
    if p.menu.peek().is_some() {
        p.menu.set(None);
    }
    if p.error.peek().is_some() {
        p.error.set(None);
    }
}
pub(super) fn invalidated(id: &StageId, parent: &[StageIdSegment], index: usize) -> bool {
    id.0.starts_with(parent)
        && matches!(id.0.get(parent.len()),Some(StageIdSegment::Index(i)) if *i>=index)
}
impl Controller {
    pub(in crate::mapping_edits) fn invalidate_editor(&self, edit: &Edit) {
        let Some(mut p) = self.inner.borrow().chrome.editor else {
            return;
        };
        let mut changes = vec![];
        let mut inserted = None;
        match edit {
            Edit::Insert { path, .. } => {
                changes.push(path.clone());
                inserted = Some(path.clone());
            }
            Edit::Remove(path) => changes.push(path.clone()),
            Edit::Move { from, to } => {
                changes.push(from.clone());
                changes.push(to.clone());
            }
            Edit::MoveGap { from, parent, gap } => {
                changes.push(from.clone());
                let mut target = parent.clone();
                target.0.push(StageIdSegment::Index(*gap));
                changes.push(target);
            }
            _ => return,
        }
        if p.expanded.try_peek().is_err() {
            return;
        }
        let invalidate = |id: &StageId| {
            changes.iter().any(|path| {
                let Some((StageIdSegment::Index(index), parent)) = path.0.split_last() else {
                    return false;
                };
                invalidated(id, parent, *index)
            })
        };
        p.expanded.write().retain(|id| !invalidate(id));
        p.hints.write().retain(|id, _| !invalidate(id));
        p.tags.write().retain(|id, _| !invalidate(id));
        if let Some(path) = inserted {
            p.expanded.write().insert(path);
        }
    }
}

impl Controller {
    pub(super) fn stage_event(&self, r: inputforge_gui_dx::navigation::StageRequest) {
        let (Some(nav), Some(mut e)) = ({
            let inner = self.inner.borrow();
            if inner.disposed {
                return;
            }
            (inner.chrome.publication, inner.chrome.editor)
        }) else {
            return;
        };
        let Some(state) = self.state.try_read() else {
            return;
        };
        if state
            .active_profile
            .as_ref()
            .map(inputforge_core::profile::Profile::id)
            != r.profile.as_ref()
            || nav.selected_mapping.peek().as_ref() != Some(&r.key)
        {
            return;
        }
        if self
            .inner
            .borrow()
            .history
            .current(&state, &r.key)
            .is_none_or(|m| {
                m.actions != r.actions || super::super::paths::at(&m.actions, &r.path.0).is_none()
            })
        {
            return;
        }
        drop(state);
        match r.intent {
            inputforge_gui_dx::navigation::StageIntent::Toggle => {
                let mut expanded = e.expanded.write();
                if !expanded.insert(r.path.clone()) {
                    expanded.remove(&r.path);
                }
            }
            inputforge_gui_dx::navigation::StageIntent::Menu(event) => {
                event.prevent_default();
                event.stop_propagation();
                let coords = event.client_coordinates();
                e.menu.set(Some(inputforge_gui_dx::navigation::StageMenu {
                    stage: r.path,
                    x: coords.x,
                    y: coords.y,
                }));
            }
        }
    }
}
