//! Restore snapshots, in-flight command chains and isolated session stacks.
use super::grouping::Burst;
use inputforge_core::{
    action::Mapping,
    profile::{Profile, ProfileId},
    state::AppState,
};
use inputforge_gui_dx::{
    mapping_edits::{Entry as Label, Group},
    snapshots::MappingKey,
};
use std::{
    collections::{HashMap, VecDeque},
    time::Instant,
};
// Preserve the existing per-mapping FIFO history limit.
const CAP: usize = 50;
#[derive(Debug, Clone)]
pub(super) struct Entry {
    pub target: MappingKey,
    pub before: Option<Mapping>,
    pub after: Option<Mapping>,
    pub label: Label,
    pub structural: bool,
}
#[derive(Debug, Default)]
pub(super) struct Stack {
    pub undo: Vec<Entry>,
    pub redo: Vec<Entry>,
    // A restore breaks acceptance of snapshots captured before it.
    pub stale_start: usize,
}
#[derive(Debug)]
struct Delivery {
    mapping: Option<Mapping>,
    serial: u64,
    structural: bool,
}
#[derive(Debug)]
struct Pending {
    observed: Option<Mapping>,
    sent: VecDeque<Delivery>,
}
#[derive(Debug, Default)]
pub(super) struct History {
    pub profile: Option<ProfileId>,
    pub stacks: HashMap<MappingKey, Stack>,
    pending: HashMap<MappingKey, Pending>,
    pub burst: Option<Burst>,
}
pub(super) fn saved(state: &AppState, key: &MappingKey) -> Option<Mapping> {
    state
        .active_profile
        .as_ref()?
        .find_mapping(&key.1, &key.0)
        .cloned()
}
impl History {
    pub(super) fn reconcile(&mut self, state: &AppState, owner: u128) {
        let profile = state.active_profile.as_ref().map(Profile::id).cloned();
        if self.profile != profile {
            *self = Self {
                profile,
                ..Self::default()
            };
        }
        let confirmed = state
            .mapping_edit_barrier
            .filter(|(id, _)| *id == owner)
            .map_or(0, |(_, serial)| serial);
        let mut changed = vec![];
        for (key, pending) in &mut self.pending {
            while pending.sent.front().is_some_and(|d| d.serial <= confirmed) {
                pending.observed = pending
                    .sent
                    .pop_front()
                    .expect("confirmed delivery exists")
                    .mapping;
            }
            let actual = saved(state, key);
            if actual != pending.observed && !pending.sent.iter().any(|d| d.mapping == actual) {
                changed.push(key.clone());
            }
        }
        for key in changed {
            self.forget(&key);
        }
    }
    pub(super) fn current(&self, state: &AppState, key: &MappingKey) -> Option<Mapping> {
        self.pending.get(key).map_or_else(
            || saved(state, key),
            |p| p.sent.back().map_or(&p.observed, |d| &d.mapping).clone(),
        )
    }
    pub(super) fn watch(&mut self, state: &AppState, key: &MappingKey) {
        self.pending.entry(key.clone()).or_insert_with(|| Pending {
            observed: saved(state, key),
            sent: VecDeque::new(),
        });
    }
    pub(super) fn accepts_stale(&self, key: &MappingKey, before: &Mapping) -> bool {
        self.stacks.get(key).is_some_and(|stack| {
            stack.redo.is_empty()
                && stack
                    .undo
                    .iter()
                    .enumerate()
                    .skip(stack.stale_start)
                    .any(|(i, entry)| {
                        (entry.before.as_ref() == Some(before)
                            && stack.undo[i..].iter().all(|e| !e.structural))
                            || (entry.after.as_ref() == Some(before)
                                && stack.undo[i + 1..].iter().all(|e| !e.structural))
                    })
        })
    }
    pub(super) fn delivered(
        &mut self,
        state: &AppState,
        key: &MappingKey,
        mapping: Option<Mapping>,
        serial: u64,
        structural: bool,
    ) {
        self.pending
            .entry(key.clone())
            .or_insert_with(|| Pending {
                observed: saved(state, key),
                sent: VecDeque::new(),
            })
            .sent
            .push_back(Delivery {
                mapping,
                serial,
                structural,
            });
    }
    pub(super) fn structural_pending(&self, key: &MappingKey) -> bool {
        self.pending
            .get(key)
            .is_some_and(|p| p.sent.iter().any(|d| d.structural))
    }
    pub(super) fn record(
        &mut self,
        key: MappingKey,
        entry: Entry,
        group: Option<Group>,
        now: Instant,
    ) {
        self.stacks.retain(|owner, stack| {
            owner == &key
                || !stack
                    .undo
                    .iter()
                    .chain(&stack.redo)
                    .any(|e| e.target == entry.target)
        });
        let merge = group
            .as_ref()
            .is_some_and(|g| self.burst.as_ref().is_some_and(|b| b.matches(&key, g, now)));
        let stack = self.stacks.entry(key.clone()).or_default();
        stack.redo.clear();
        if merge
            && let Some(last) = stack.undo.last_mut()
            && last.target == entry.target
            && last.after == entry.before
        {
            last.after = entry.after;
            last.structural |= entry.structural;
        } else {
            stack.undo.push(entry);
            if stack.undo.len() > CAP {
                stack.undo.remove(0);
                stack.stale_start = stack.stale_start.saturating_sub(1);
            }
        }
        self.burst = group.map(|group| Burst {
            key,
            group,
            at: now,
        });
    }
    pub(super) fn forget(&mut self, key: &MappingKey) {
        self.stacks.retain(|owner, stack| {
            owner != key
                && !stack
                    .undo
                    .iter()
                    .chain(&stack.redo)
                    .any(|e| &e.target == key)
        });
        self.pending.remove(key);
        self.burst = None;
    }
}
