//! App-owned forward rail workflows, sharing editor history and navigation.
mod add;
mod dispatch;
mod reorder;
use super::{Controller, history};
use dioxus::prelude::{ReadableExt, WritableExt};
use inputforge_core::{action::Mapping, state::AppState};
use inputforge_gui_dx::{
    mapping_rail::{Lookup, Menu, Outcome, Scope, Target},
    snapshots::MappingKey,
};

impl Controller {
    fn valid_scope(&self, state: &AppState, scope: &Scope) -> bool {
        !self.inner.borrow().disposed
            && !state.session.offline
            && state.active_profile.as_ref().is_some_and(|p| {
                scope.profile.as_ref() == Some(p.id()) && p.modes().contains(&scope.mode)
            })
            && self
                .inner
                .borrow()
                .publication
                .is_none_or(|p| *p.mode.peek() == scope.mode)
    }
    fn current_target(&self, state: &AppState, target: &Target) -> Option<Mapping> {
        if !self.valid_scope(state, &target.scope) || target.before.mode != target.scope.mode {
            return None;
        }
        let mut inner = self.inner.borrow_mut();
        inner.history.reconcile(state, self.owner);
        let key = (target.before.mode.clone(), target.before.input.clone());
        let current = inner.history.current(state, &key)?;
        if current != target.before
            && history::saved(state, &key).as_ref() != Some(&target.before)
            && !inner.history.accepts_stale(&key, &target.before)
        {
            return None;
        }
        Some(current)
    }
    /// Resolve current source metadata and prepare menu availability.
    #[must_use]
    pub fn rail_menu(&self, lookup: Lookup) -> Option<Menu> {
        let state = self.state.read();
        if !self.valid_scope(&state, &lookup.scope) {
            return None;
        }
        let profile = state.active_profile.as_ref()?;
        let mut inner = self.inner.borrow_mut();
        inner.history.reconcile(&state, self.owner);
        let key = (lookup.scope.mode.clone(), lookup.input.clone());
        let before = inner.history.current(&state, &key)?;
        if before.name != lookup.name
            && history::saved(&state, &key).is_none_or(|m| m.name != lookup.name)
        {
            return None;
        }
        let group = reorder::group(&before.input);
        let order = reorder::order(&state, &before.mode, group);
        let index = order.iter().position(|input| input == &before.input);
        drop(inner);
        let ready = group.is_some_and(|group| !self.reorder_blocked(&state, &lookup.scope, group));
        Some(Menu {
            target: Target {
                scope: lookup.scope,
                before,
            },
            group: group.unwrap_or(inputforge_gui_dx::mapping_rail::Group::Buttons),
            can_up: ready && !lookup.filtered && index.is_some_and(|i| i > 0),
            can_down: ready && !lookup.filtered && index.is_some_and(|i| i + 1 < order.len()),
            order,
            other_modes: profile
                .modes()
                .as_slice()
                .iter()
                .filter(|m| **m != key.0)
                .cloned()
                .collect(),
        })
    }
    fn navigate(&self, key: &MappingKey) {
        let publication = self.inner.borrow().publication;
        self.end_group();
        if let Some(mut p) = publication {
            p.mode.set(key.0.clone());
            p.selection.set(Some(key.clone()));
        }
    }
}
fn rejected() -> Outcome {
    Outcome::Rejected("This mapping changed or is no longer available. Reopen the action.".into())
}
