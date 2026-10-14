//! Group-local reorder validation shared by keyboard, menu and drag requests.
use super::super::Controller;
use inputforge_core::{
    action::Mapping,
    state::AppState,
    types::{InputAddress, InputId},
};
use inputforge_gui_dx::mapping_rail::{Group, Move, Outcome, Scope};

pub(super) fn group(input: &InputAddress) -> Option<Group> {
    input.input_id().map(|input| match input {
        InputId::Axis { .. } => Group::Axes,
        InputId::Button { .. } => Group::Buttons,
        InputId::Hat { .. } => Group::Hats,
    })
}
pub(super) fn order(state: &AppState, mode: &str, kind: Option<Group>) -> Vec<InputAddress> {
    state.active_profile.as_ref().map_or_else(Vec::new, |p| {
        p.mappings()
            .iter()
            .filter(|m| m.mode == mode && kind.is_some() && group(&m.input) == kind)
            .map(|m| m.input.clone())
            .collect()
    })
}
impl Controller {
    pub(super) fn reorder_blocked(&self, state: &AppState, scope: &Scope, group: Group) -> bool {
        let inner = self.inner.borrow();
        let confirmed = state
            .mapping_edit_barrier
            .filter(|(owner, _)| *owner == self.owner)
            .map_or(0, |(_, serial)| serial);
        inner.history.membership_pending(state, &scope.mode)
            || inner
                .reorders
                .get(&(scope.mode.clone(), group))
                .is_some_and(|(profile, serial)| {
                    Some(profile) == scope.profile.as_ref() && *serial > confirmed
                })
    }
}
pub(super) fn prepare(
    state: &AppState,
    source: &Mapping,
    movement: Move,
    expected: &[InputAddress],
    kind: Group,
    filtered: bool,
) -> Outcome {
    if filtered {
        return Outcome::NoOp;
    }
    if group(&source.input) != Some(kind) {
        return super::rejected();
    }
    let order = order(state, &source.mode, Some(kind));
    if order != expected {
        return super::rejected();
    }
    let Some(from) = order.iter().position(|input| input == &source.input) else {
        return super::rejected();
    };
    let to = match movement {
        Move::Up => from.checked_sub(1),
        Move::Down => (from + 1 < order.len()).then_some(from + 1),
        Move::Gap(gap) if gap <= order.len() => Some(if from < gap { gap - 1 } else { gap }),
        Move::Gap(_) => return super::rejected(),
    };
    let Some(to) = to.filter(|to| *to != from) else {
        return Outcome::NoOp;
    };
    // The caller drops its guard before this prepared command is delivered.
    Outcome::Moved {
        index: to,
        len: order.len(),
        group: kind,
    }
}
