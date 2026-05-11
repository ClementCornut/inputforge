// Rust guideline compliant 2026-05-11

use std::collections::{HashMap, HashSet};

use crate::action::{MouseTarget, OutputBehavior};
use crate::pipeline::{OutputDestination, OutputOwner};
use crate::types::{InputAddress, KeyCombo};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OutputEvent {
    KeyDown(KeyCombo),
    KeyUp(KeyCombo),
    KeyPulse(KeyCombo),
    MouseDown(MouseTarget),
    MouseUp(MouseTarget),
    MousePulse(MouseTarget),
    Wheel(MouseTarget),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OutputAction {
    Immediate(OutputEvent),
    Release {
        owner: OutputOwner,
        event: OutputEvent,
    },
}

impl OutputAction {
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "Task 6 will convert actions for sink dispatch")
    )]
    pub(crate) fn into_event(self) -> OutputEvent {
        match self {
            Self::Immediate(event) | Self::Release { event, .. } => event,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct OwnerScopeKey {
    profile: String,
    mode: String,
    input: InputAddress,
}

impl OwnerScopeKey {
    pub(crate) fn new(
        profile: impl Into<String>,
        mode: impl Into<String>,
        input: InputAddress,
    ) -> Self {
        Self {
            profile: profile.into(),
            mode: mode.into(),
            input,
        }
    }

    pub(crate) fn from_owner(owner: &OutputOwner) -> Self {
        Self {
            profile: owner.profile.clone(),
            mode: owner.mode.clone(),
            input: owner.input.clone(),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct OutputRuntimeState {
    active_owners: HashSet<OutputOwner>,
    hold_counts: HashMap<OutputDestination, usize>,
}

impl OutputRuntimeState {
    pub(crate) fn reconcile_keyboard(
        &mut self,
        owner: OutputOwner,
        key: KeyCombo,
        behavior: OutputBehavior,
        active: bool,
    ) -> Vec<OutputAction> {
        let destination = OutputDestination::Keyboard(key.clone());
        match behavior {
            OutputBehavior::Hold => self.reconcile_hold(
                owner,
                destination,
                active,
                OutputEvent::KeyDown(key.clone()),
                OutputEvent::KeyUp(key),
            ),
            OutputBehavior::Pulse => {
                self.reconcile_pulse(owner, active, OutputEvent::KeyPulse(key))
            }
        }
    }

    pub(crate) fn reconcile_mouse(
        &mut self,
        owner: OutputOwner,
        target: MouseTarget,
        behavior: OutputBehavior,
        active: bool,
    ) -> Vec<OutputAction> {
        if target.is_wheel() {
            return self.reconcile_pulse(owner, active, OutputEvent::Wheel(target));
        }

        let destination = OutputDestination::Mouse(target);
        match behavior {
            OutputBehavior::Hold => self.reconcile_hold(
                owner,
                destination,
                active,
                OutputEvent::MouseDown(target),
                OutputEvent::MouseUp(target),
            ),
            OutputBehavior::Pulse => {
                self.reconcile_pulse(owner, active, OutputEvent::MousePulse(target))
            }
        }
    }

    pub(crate) fn reconcile_absent_owners_for_scope(
        &mut self,
        scope: &OwnerScopeKey,
        current: &[OutputOwner],
    ) -> Vec<OutputAction> {
        let current: HashSet<&OutputOwner> = current.iter().collect();
        self.active_owners
            .iter()
            .filter(|owner| OwnerScopeKey::from_owner(owner) == *scope && !current.contains(owner))
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .filter_map(|owner| self.stage_release_owner(owner))
            .collect()
    }

    pub(crate) fn release_all(&mut self) -> Vec<OutputAction> {
        self.active_owners
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .filter_map(|owner| self.stage_release_owner(owner))
            .collect()
    }

    pub(crate) fn commit_release(&mut self, owner: &OutputOwner) {
        if !self.active_owners.remove(owner) {
            return;
        }

        if owner.behavior == OutputBehavior::Hold {
            let count = self
                .hold_counts
                .get_mut(&owner.destination)
                .expect("active hold owner must have a destination count");
            *count -= 1;
            if *count == 0 {
                self.hold_counts.remove(&owner.destination);
            }
        }
    }

    fn reconcile_hold(
        &mut self,
        owner: OutputOwner,
        destination: OutputDestination,
        active: bool,
        down: OutputEvent,
        up: OutputEvent,
    ) -> Vec<OutputAction> {
        if active {
            if !self.active_owners.insert(owner) {
                return Vec::new();
            }

            let count = self.hold_counts.entry(destination).or_default();
            *count += 1;
            if *count == 1 {
                vec![OutputAction::Immediate(down)]
            } else {
                Vec::new()
            }
        } else {
            self.stage_release_owner_with_event(owner, up)
                .into_iter()
                .collect()
        }
    }

    fn reconcile_pulse(
        &mut self,
        owner: OutputOwner,
        active: bool,
        pulse: OutputEvent,
    ) -> Vec<OutputAction> {
        if active {
            if self.active_owners.insert(owner) {
                vec![OutputAction::Immediate(pulse)]
            } else {
                Vec::new()
            }
        } else {
            self.active_owners.remove(&owner);
            Vec::new()
        }
    }

    fn stage_release_owner(&mut self, owner: OutputOwner) -> Option<OutputAction> {
        let event = match &owner.destination {
            OutputDestination::Keyboard(key) => OutputEvent::KeyUp(key.clone()),
            OutputDestination::Mouse(target) => OutputEvent::MouseUp(*target),
        };
        self.stage_release_owner_with_event(owner, event)
    }

    fn stage_release_owner_with_event(
        &mut self,
        owner: OutputOwner,
        event: OutputEvent,
    ) -> Option<OutputAction> {
        if !self.active_owners.contains(&owner) {
            return None;
        }

        if owner.behavior != OutputBehavior::Hold {
            self.active_owners.remove(&owner);
            return None;
        }

        let count = self
            .hold_counts
            .get(&owner.destination)
            .copied()
            .expect("active hold owner must have a destination count");
        if count > 1 {
            self.commit_release(&owner);
            None
        } else {
            Some(OutputAction::Release { owner, event })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::pipeline::ActionPathSegment;

    fn combo() -> KeyCombo {
        KeyCombo {
            key: "F1".to_owned(),
            modifiers: Vec::new(),
        }
    }

    fn owner(
        destination: OutputDestination,
        index: usize,
        behavior: OutputBehavior,
    ) -> OutputOwner {
        OutputOwner {
            profile: "profile".to_owned(),
            mode: "mode".to_owned(),
            input: InputAddress::Unbound,
            action_path: vec![ActionPathSegment::Index(index)],
            destination,
            behavior,
        }
    }

    fn events(actions: Vec<OutputAction>) -> Vec<OutputEvent> {
        actions.into_iter().map(OutputAction::into_event).collect()
    }

    #[test]
    fn owner_scope_new_matches_owner_scope() {
        let mut owner = owner(
            OutputDestination::Keyboard(combo()),
            0,
            OutputBehavior::Hold,
        );
        owner.profile = "profile-a".to_owned();
        owner.mode = "Default".to_owned();

        assert_eq!(
            OwnerScopeKey::new("profile-a", "Default", InputAddress::Unbound),
            OwnerScopeKey::from_owner(&owner),
        );
    }

    #[test]
    fn hold_sends_down_once_and_up_once() {
        let mut state = OutputRuntimeState::default();
        let key = combo();
        let owner = owner(
            OutputDestination::Keyboard(key.clone()),
            0,
            OutputBehavior::Hold,
        );

        assert_eq!(
            events(state.reconcile_keyboard(
                owner.clone(),
                key.clone(),
                OutputBehavior::Hold,
                true,
            )),
            vec![OutputEvent::KeyDown(key.clone())],
        );
        assert!(
            state
                .reconcile_keyboard(owner.clone(), key.clone(), OutputBehavior::Hold, true)
                .is_empty()
        );

        let release =
            state.reconcile_keyboard(owner.clone(), key.clone(), OutputBehavior::Hold, false);
        assert_eq!(
            events(release.clone()),
            vec![OutputEvent::KeyUp(key.clone())]
        );

        state.commit_release(&owner);
        assert!(
            state
                .reconcile_keyboard(owner, key, OutputBehavior::Hold, false)
                .is_empty()
        );
    }

    #[test]
    fn two_hold_owners_ref_count_destination() {
        let mut state = OutputRuntimeState::default();
        let key = combo();
        let first = owner(
            OutputDestination::Keyboard(key.clone()),
            0,
            OutputBehavior::Hold,
        );
        let second = owner(
            OutputDestination::Keyboard(key.clone()),
            1,
            OutputBehavior::Hold,
        );

        assert_eq!(
            events(state.reconcile_keyboard(
                first.clone(),
                key.clone(),
                OutputBehavior::Hold,
                true,
            )),
            vec![OutputEvent::KeyDown(key.clone())],
        );
        assert!(
            state
                .reconcile_keyboard(second.clone(), key.clone(), OutputBehavior::Hold, true)
                .is_empty()
        );
        assert!(
            state
                .reconcile_keyboard(first, key.clone(), OutputBehavior::Hold, false)
                .is_empty()
        );

        let release =
            state.reconcile_keyboard(second.clone(), key.clone(), OutputBehavior::Hold, false);
        assert_eq!(events(release), vec![OutputEvent::KeyUp(key)]);
        state.commit_release(&second);
    }

    #[test]
    fn pulse_fires_once_per_rising_edge() {
        let mut state = OutputRuntimeState::default();
        let key = combo();
        let owner = owner(
            OutputDestination::Keyboard(key.clone()),
            0,
            OutputBehavior::Pulse,
        );

        assert_eq!(
            events(state.reconcile_keyboard(
                owner.clone(),
                key.clone(),
                OutputBehavior::Pulse,
                true,
            )),
            vec![OutputEvent::KeyPulse(key.clone())],
        );
        assert!(
            state
                .reconcile_keyboard(owner.clone(), key.clone(), OutputBehavior::Pulse, true)
                .is_empty()
        );
        assert!(
            state
                .reconcile_keyboard(owner.clone(), key.clone(), OutputBehavior::Pulse, false)
                .is_empty()
        );
        assert_eq!(
            events(state.reconcile_keyboard(owner, key.clone(), OutputBehavior::Pulse, true)),
            vec![OutputEvent::KeyPulse(key)],
        );
    }

    #[test]
    fn missing_owner_releases_hold_destination() {
        let mut state = OutputRuntimeState::default();
        let key = combo();
        let owner = owner(
            OutputDestination::Keyboard(key.clone()),
            0,
            OutputBehavior::Hold,
        );
        let scope = OwnerScopeKey::from_owner(&owner);

        assert_eq!(
            events(state.reconcile_keyboard(
                owner.clone(),
                key.clone(),
                OutputBehavior::Hold,
                true,
            )),
            vec![OutputEvent::KeyDown(key.clone())],
        );

        let release = state.reconcile_absent_owners_for_scope(&scope, &[]);
        assert_eq!(events(release), vec![OutputEvent::KeyUp(key)]);
        state.commit_release(&owner);
    }
}
