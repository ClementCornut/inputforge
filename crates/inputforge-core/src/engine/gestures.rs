// Rust guideline compliant 2026-05-14
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "runtime integration is scheduled for Task 9")
)]

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use crate::action::{Action, ActionBranch, branch_actions};
use crate::pipeline::ActionPathSegment;
use crate::types::InputAddress;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct GestureKey {
    profile: String,
    mode: String,
    input: InputAddress,
    action_path: Vec<ActionPathSegment>,
}

impl GestureKey {
    pub(crate) fn new(
        profile: String,
        mode: String,
        input: InputAddress,
        action_path: Vec<ActionPathSegment>,
    ) -> Self {
        Self {
            profile,
            mode,
            input,
            action_path,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GestureRunPhase {
    Momentary,
    Active,
    Release,
}

#[derive(Debug, Clone)]
pub(crate) struct GestureRun {
    pub(crate) key: GestureKey,
    pub(crate) branch: ActionBranch,
    pub(crate) actions: Vec<Action>,
    pub(crate) phase: GestureRunPhase,
}

#[derive(Debug, Clone)]
struct ScheduledRun {
    due_at: Instant,
    run: GestureRun,
}

#[derive(Debug, Clone)]
struct TapState {
    first_release_at: Instant,
    single_due_at: Option<Instant>,
}

#[derive(Debug, Clone)]
struct PressState {
    pressed_at: Instant,
    long_fired: bool,
}

#[derive(Debug, Default)]
pub(crate) struct GestureDispatcher {
    taps: HashMap<GestureKey, TapState>,
    presses: HashMap<GestureKey, PressState>,
    scheduled: VecDeque<ScheduledRun>,
}

impl GestureDispatcher {
    pub(crate) fn observe_release(
        &mut self,
        key: GestureKey,
        action: &Action,
        now: Instant,
    ) -> Vec<GestureRun> {
        match action {
            Action::TapGesture {
                threshold_ms,
                fire_single_immediately,
                ..
            } => {
                self.observe_tap_release(key, action, *threshold_ms, *fire_single_immediately, now)
            }
            Action::PressGesture {
                threshold_ms,
                fire_long_when_threshold_crossed,
                ..
            } => self.observe_press_release(
                key,
                action,
                *threshold_ms,
                *fire_long_when_threshold_crossed,
                now,
            ),
            _ => Vec::new(),
        }
    }

    pub(crate) fn observe_press(
        &mut self,
        key: GestureKey,
        action: &Action,
        now: Instant,
    ) -> Vec<GestureRun> {
        let Action::PressGesture {
            threshold_ms,
            fire_long_when_threshold_crossed,
            ..
        } = action
        else {
            return Vec::new();
        };

        if self.presses.contains_key(&key) {
            return Vec::new();
        }

        self.presses.insert(
            key.clone(),
            PressState {
                pressed_at: now,
                long_fired: false,
            },
        );

        if *fire_long_when_threshold_crossed {
            self.schedule(
                now + Duration::from_millis(*threshold_ms),
                self.run_for(
                    &key,
                    ActionBranch::PressLong,
                    action,
                    GestureRunPhase::Active,
                ),
            );
        }

        Vec::new()
    }

    pub(crate) fn drain_due(&mut self, now: Instant) -> Vec<GestureRun> {
        let mut ready = Vec::new();
        let mut pending = VecDeque::new();

        while let Some(scheduled) = self.scheduled.pop_front() {
            if scheduled.due_at <= now {
                self.mark_due_run_drained(&scheduled);
                ready.push(scheduled.run);
            } else {
                pending.push_back(scheduled);
            }
        }

        self.scheduled = pending;
        ready
    }

    pub(crate) fn clear_all(&mut self) {
        self.taps.clear();
        self.presses.clear();
        self.scheduled.clear();
    }

    fn observe_tap_release(
        &mut self,
        key: GestureKey,
        action: &Action,
        threshold_ms: u64,
        fire_single_immediately: bool,
        now: Instant,
    ) -> Vec<GestureRun> {
        if let Some(first) = self.taps.remove(&key) {
            if now.duration_since(first.first_release_at) <= Duration::from_millis(threshold_ms) {
                if let Some(single_due_at) = first.single_due_at {
                    self.cancel_exact(&key, ActionBranch::TapSingle, single_due_at);
                }
                return vec![self.run_for(
                    &key,
                    ActionBranch::TapDouble,
                    action,
                    GestureRunPhase::Momentary,
                )];
            }

            if let Some(single_due_at) = first.single_due_at {
                self.cancel_exact(&key, ActionBranch::TapSingle, single_due_at);
                let mut runs = vec![self.run_for(
                    &key,
                    ActionBranch::TapSingle,
                    action,
                    GestureRunPhase::Momentary,
                )];
                runs.extend(self.start_tap_window(
                    key,
                    action,
                    threshold_ms,
                    fire_single_immediately,
                    now,
                ));
                return runs;
            }
        }

        self.start_tap_window(key, action, threshold_ms, fire_single_immediately, now)
    }

    fn start_tap_window(
        &mut self,
        key: GestureKey,
        action: &Action,
        threshold_ms: u64,
        fire_single_immediately: bool,
        now: Instant,
    ) -> Vec<GestureRun> {
        let single_due_at =
            (!fire_single_immediately).then(|| now + Duration::from_millis(threshold_ms));
        self.taps.insert(
            key.clone(),
            TapState {
                first_release_at: now,
                single_due_at,
            },
        );

        if fire_single_immediately {
            vec![self.run_for(
                &key,
                ActionBranch::TapSingle,
                action,
                GestureRunPhase::Momentary,
            )]
        } else {
            self.schedule(
                single_due_at.expect("delayed single tap has a due instant"),
                self.run_for(
                    &key,
                    ActionBranch::TapSingle,
                    action,
                    GestureRunPhase::Momentary,
                ),
            );
            Vec::new()
        }
    }

    fn observe_press_release(
        &mut self,
        key: GestureKey,
        action: &Action,
        threshold_ms: u64,
        fire_long_when_threshold_crossed: bool,
        now: Instant,
    ) -> Vec<GestureRun> {
        let Some(state) = self.presses.remove(&key) else {
            return Vec::new();
        };

        self.cancel(&key, ActionBranch::PressLong);

        if fire_long_when_threshold_crossed {
            if state.long_fired {
                return vec![self.run_for(
                    &key,
                    ActionBranch::PressLong,
                    action,
                    GestureRunPhase::Release,
                )];
            }

            if now.duration_since(state.pressed_at) >= Duration::from_millis(threshold_ms) {
                return vec![
                    self.run_for(
                        &key,
                        ActionBranch::PressLong,
                        action,
                        GestureRunPhase::Active,
                    ),
                    self.run_for(
                        &key,
                        ActionBranch::PressLong,
                        action,
                        GestureRunPhase::Release,
                    ),
                ];
            }

            return vec![self.run_for(
                &key,
                ActionBranch::PressShort,
                action,
                GestureRunPhase::Momentary,
            )];
        }

        if now.duration_since(state.pressed_at) >= Duration::from_millis(threshold_ms) {
            vec![self.run_for(
                &key,
                ActionBranch::PressLong,
                action,
                GestureRunPhase::Momentary,
            )]
        } else {
            vec![self.run_for(
                &key,
                ActionBranch::PressShort,
                action,
                GestureRunPhase::Momentary,
            )]
        }
    }

    fn run_for(
        &self,
        key: &GestureKey,
        branch: ActionBranch,
        action: &Action,
        phase: GestureRunPhase,
    ) -> GestureRun {
        let actions = branch_actions(action, branch)
            .map(<[Action]>::to_vec)
            .unwrap_or_default();

        GestureRun {
            key: key.clone(),
            branch,
            actions,
            phase,
        }
    }

    fn schedule(&mut self, due_at: Instant, run: GestureRun) {
        self.scheduled.push_back(ScheduledRun { due_at, run });
    }

    fn cancel(&mut self, key: &GestureKey, branch: ActionBranch) {
        self.scheduled
            .retain(|scheduled| &scheduled.run.key != key || scheduled.run.branch != branch);
    }

    fn cancel_exact(&mut self, key: &GestureKey, branch: ActionBranch, due_at: Instant) {
        self.scheduled.retain(|scheduled| {
            &scheduled.run.key != key
                || scheduled.run.branch != branch
                || scheduled.due_at != due_at
        });
    }

    fn mark_due_run_drained(&mut self, scheduled: &ScheduledRun) {
        if scheduled.run.branch == ActionBranch::TapSingle
            && self
                .taps
                .get(&scheduled.run.key)
                .is_some_and(|state| state.single_due_at == Some(scheduled.due_at))
        {
            self.taps.remove(&scheduled.run.key);
        }

        if scheduled.run.branch == ActionBranch::PressLong {
            if let Some(state) = self.presses.get_mut(&scheduled.run.key) {
                state.long_fired = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{GestureDispatcher, GestureKey, GestureRunPhase};
    use crate::action::{Action, ActionBranch};
    use crate::pipeline::ActionPathSegment;
    use crate::types::{DeviceId, InputAddress, InputId};
    use std::time::{Duration, Instant};

    fn button(index: u8) -> InputAddress {
        InputAddress::Bound {
            device: DeviceId("stick".to_owned()),
            input: InputId::Button { index },
        }
    }

    fn key(branch: ActionBranch) -> GestureKey {
        GestureKey::new(
            "profile".to_owned(),
            "Default".to_owned(),
            button(1),
            vec![
                ActionPathSegment::Index(0),
                ActionPathSegment::Branch(branch),
            ],
        )
    }

    #[test]
    fn delayed_single_tap_fires_after_threshold() {
        let now = Instant::now();
        let mut gestures = GestureDispatcher::default();
        let action = Action::TapGesture {
            threshold_ms: 500,
            fire_single_immediately: false,
            single_tap: vec![Action::Invert],
            double_tap: Vec::new(),
        };

        assert!(
            gestures
                .observe_release(key(ActionBranch::TapSingle), &action, now)
                .is_empty()
        );
        assert!(
            gestures
                .drain_due(now + Duration::from_millis(499))
                .is_empty()
        );

        let due = gestures.drain_due(now + Duration::from_millis(500));

        assert_eq!(due.len(), 1);
        assert_eq!(due[0].branch, ActionBranch::TapSingle);
        assert_eq!(due[0].actions, vec![Action::Invert]);
        assert!(matches!(due[0].phase, GestureRunPhase::Momentary));
    }

    #[test]
    fn double_tap_cancels_delayed_single() {
        let now = Instant::now();
        let mut gestures = GestureDispatcher::default();
        let action = Action::TapGesture {
            threshold_ms: 500,
            fire_single_immediately: false,
            single_tap: vec![Action::Invert],
            double_tap: vec![Action::Deadzone {
                config: crate::processing::DeadzoneConfig::default(),
            }],
        };
        let key = key(ActionBranch::TapSingle);

        assert!(
            gestures
                .observe_release(key.clone(), &action, now)
                .is_empty()
        );
        let immediate = gestures.observe_release(key, &action, now + Duration::from_millis(250));

        assert_eq!(immediate.len(), 1);
        assert_eq!(immediate[0].branch, ActionBranch::TapDouble);
        assert_eq!(immediate[0].actions.len(), 1);
        assert!(matches!(immediate[0].phase, GestureRunPhase::Momentary));
        assert!(
            gestures
                .drain_due(now + Duration::from_millis(500))
                .is_empty()
        );
    }

    #[test]
    fn tap_gesture_immediate_single_dispatches_on_first_release() {
        let now = Instant::now();
        let mut gestures = GestureDispatcher::default();
        let action = Action::TapGesture {
            threshold_ms: 500,
            fire_single_immediately: true,
            single_tap: vec![Action::Invert],
            double_tap: Vec::new(),
        };

        let immediate = gestures.observe_release(key(ActionBranch::TapSingle), &action, now);

        assert_eq!(immediate.len(), 1);
        assert_eq!(immediate[0].branch, ActionBranch::TapSingle);
        assert!(matches!(immediate[0].phase, GestureRunPhase::Momentary));
    }

    #[test]
    fn tap_gesture_immediate_single_allows_additive_double_tap() {
        let now = Instant::now();
        let mut gestures = GestureDispatcher::default();
        let action = Action::TapGesture {
            threshold_ms: 500,
            fire_single_immediately: true,
            single_tap: vec![Action::Invert],
            double_tap: vec![Action::Deadzone {
                config: crate::processing::DeadzoneConfig::default(),
            }],
        };
        let key = key(ActionBranch::TapSingle);

        assert_eq!(gestures.observe_release(key.clone(), &action, now).len(), 1);
        let second = gestures.observe_release(key, &action, now + Duration::from_millis(250));

        assert_eq!(second.len(), 1);
        assert_eq!(second[0].branch, ActionBranch::TapDouble);
        assert!(matches!(second[0].phase, GestureRunPhase::Momentary));
    }

    #[test]
    fn long_press_fires_when_threshold_crossed() {
        let now = Instant::now();
        let mut gestures = GestureDispatcher::default();
        let action = Action::PressGesture {
            threshold_ms: 500,
            fire_long_when_threshold_crossed: true,
            short_press: Vec::new(),
            long_press: vec![Action::Invert],
        };
        let key = key(ActionBranch::PressLong);

        assert!(gestures.observe_press(key.clone(), &action, now).is_empty());
        let due = gestures.drain_due(now + Duration::from_millis(500));

        assert_eq!(due.len(), 1);
        assert_eq!(due[0].branch, ActionBranch::PressLong);
        assert!(matches!(due[0].phase, GestureRunPhase::Active));

        let release = gestures.observe_release(key, &action, now + Duration::from_millis(550));

        assert_eq!(release.len(), 1);
        assert_eq!(release[0].branch, ActionBranch::PressLong);
        assert!(matches!(release[0].phase, GestureRunPhase::Release));
    }

    #[test]
    fn press_gesture_long_press_decides_on_release() {
        let now = Instant::now();
        let mut gestures = GestureDispatcher::default();
        let action = Action::PressGesture {
            threshold_ms: 500,
            fire_long_when_threshold_crossed: false,
            short_press: Vec::new(),
            long_press: vec![Action::Invert],
        };
        let key = key(ActionBranch::PressLong);

        assert!(gestures.observe_press(key.clone(), &action, now).is_empty());
        let release = gestures.observe_release(key, &action, now + Duration::from_millis(500));

        assert_eq!(release.len(), 1);
        assert_eq!(release[0].branch, ActionBranch::PressLong);
        assert!(matches!(release[0].phase, GestureRunPhase::Momentary));
    }

    #[test]
    fn press_gesture_short_release_cancels_scheduled_long() {
        let now = Instant::now();
        let mut gestures = GestureDispatcher::default();
        let action = Action::PressGesture {
            threshold_ms: 500,
            fire_long_when_threshold_crossed: true,
            short_press: vec![Action::Invert],
            long_press: vec![Action::Deadzone {
                config: crate::processing::DeadzoneConfig::default(),
            }],
        };
        let key = key(ActionBranch::PressLong);

        assert!(gestures.observe_press(key.clone(), &action, now).is_empty());
        let release = gestures.observe_release(key, &action, now + Duration::from_millis(250));

        assert_eq!(release.len(), 1);
        assert_eq!(release[0].branch, ActionBranch::PressShort);
        assert!(matches!(release[0].phase, GestureRunPhase::Momentary));
        assert!(
            gestures
                .drain_due(now + Duration::from_millis(500))
                .is_empty()
        );
    }

    #[test]
    fn expired_tap_timer_does_not_clear_newer_tap_window() {
        let now = Instant::now();
        let mut gestures = GestureDispatcher::default();
        let action = Action::TapGesture {
            threshold_ms: 500,
            fire_single_immediately: false,
            single_tap: vec![Action::Invert],
            double_tap: vec![Action::Deadzone {
                config: crate::processing::DeadzoneConfig::default(),
            }],
        };
        let key = key(ActionBranch::TapSingle);

        assert!(
            gestures
                .observe_release(key.clone(), &action, now)
                .is_empty()
        );
        let second_release =
            gestures.observe_release(key.clone(), &action, now + Duration::from_millis(600));
        assert_eq!(second_release.len(), 1);
        assert_eq!(second_release[0].branch, ActionBranch::TapSingle);

        let third_release =
            gestures.observe_release(key, &action, now + Duration::from_millis(700));

        assert_eq!(third_release.len(), 1);
        assert_eq!(third_release[0].branch, ActionBranch::TapDouble);
    }

    #[test]
    fn clear_all_drops_gesture_state_and_scheduled_runs() {
        let now = Instant::now();
        let mut gestures = GestureDispatcher::default();
        let tap = Action::TapGesture {
            threshold_ms: 500,
            fire_single_immediately: false,
            single_tap: vec![Action::Invert],
            double_tap: Vec::new(),
        };
        let press = Action::PressGesture {
            threshold_ms: 500,
            fire_long_when_threshold_crossed: true,
            short_press: Vec::new(),
            long_press: vec![Action::Invert],
        };

        assert!(
            gestures
                .observe_release(key(ActionBranch::TapSingle), &tap, now)
                .is_empty()
        );
        assert!(
            gestures
                .observe_press(key(ActionBranch::PressLong), &press, now)
                .is_empty()
        );

        gestures.clear_all();

        assert!(
            gestures
                .drain_due(now + Duration::from_millis(500))
                .is_empty()
        );
    }

    #[test]
    fn overdue_delayed_single_runs_before_new_tap_window() {
        let now = Instant::now();
        let mut gestures = GestureDispatcher::default();
        let action = Action::TapGesture {
            threshold_ms: 500,
            fire_single_immediately: false,
            single_tap: vec![Action::Invert],
            double_tap: vec![Action::Deadzone {
                config: crate::processing::DeadzoneConfig::default(),
            }],
        };
        let key = key(ActionBranch::TapSingle);

        assert!(
            gestures
                .observe_release(key.clone(), &action, now)
                .is_empty()
        );
        let second_release =
            gestures.observe_release(key.clone(), &action, now + Duration::from_millis(600));

        assert_eq!(second_release.len(), 1);
        assert_eq!(second_release[0].branch, ActionBranch::TapSingle);
        assert!(matches!(
            second_release[0].phase,
            GestureRunPhase::Momentary
        ));

        let third_release =
            gestures.observe_release(key, &action, now + Duration::from_millis(700));

        assert_eq!(third_release.len(), 1);
        assert_eq!(third_release[0].branch, ActionBranch::TapDouble);
        assert!(
            gestures
                .drain_due(now + Duration::from_millis(1_100))
                .is_empty()
        );
    }

    #[test]
    fn overdue_threshold_crossed_long_press_runs_active_then_release() {
        let now = Instant::now();
        let mut gestures = GestureDispatcher::default();
        let action = Action::PressGesture {
            threshold_ms: 500,
            fire_long_when_threshold_crossed: true,
            short_press: vec![Action::Deadzone {
                config: crate::processing::DeadzoneConfig::default(),
            }],
            long_press: vec![Action::Invert],
        };
        let key = key(ActionBranch::PressLong);

        assert!(gestures.observe_press(key.clone(), &action, now).is_empty());
        let release = gestures.observe_release(key, &action, now + Duration::from_millis(600));

        assert_eq!(release.len(), 2);
        assert_eq!(release[0].branch, ActionBranch::PressLong);
        assert!(matches!(release[0].phase, GestureRunPhase::Active));
        assert_eq!(release[1].branch, ActionBranch::PressLong);
        assert!(matches!(release[1].phase, GestureRunPhase::Release));
        assert!(
            gestures
                .drain_due(now + Duration::from_millis(600))
                .is_empty()
        );
    }

    #[test]
    fn duplicate_press_while_held_does_not_reset_long_press_state() {
        let now = Instant::now();
        let mut gestures = GestureDispatcher::default();
        let action = Action::PressGesture {
            threshold_ms: 500,
            fire_long_when_threshold_crossed: true,
            short_press: Vec::new(),
            long_press: vec![Action::Invert],
        };
        let key = key(ActionBranch::PressLong);

        assert!(gestures.observe_press(key.clone(), &action, now).is_empty());
        assert!(
            gestures
                .observe_press(key.clone(), &action, now + Duration::from_millis(250))
                .is_empty()
        );

        let due = gestures.drain_due(now + Duration::from_millis(500));
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].branch, ActionBranch::PressLong);
        assert!(matches!(due[0].phase, GestureRunPhase::Active));

        assert!(
            gestures
                .observe_press(key.clone(), &action, now + Duration::from_millis(550))
                .is_empty()
        );
        let release = gestures.observe_release(key, &action, now + Duration::from_millis(600));

        assert_eq!(release.len(), 1);
        assert_eq!(release[0].branch, ActionBranch::PressLong);
        assert!(matches!(release[0].phase, GestureRunPhase::Release));
    }
}
