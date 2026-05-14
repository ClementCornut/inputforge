// Rust guideline compliant 2026-05-14

//! Short-lived output activity used by live preview surfaces.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::pipeline::OutputOwner;
use crate::types::HatDirection;

/// Duration used to keep momentary outputs visible in the GUI.
///
/// Gesture taps can emit press and release in one engine tick. The output sink
/// still receives the exact events, but the preview needs a tiny display-only
/// latch so users can see that the gesture fired.
pub const OUTPUT_ACTIVITY_PREVIEW_LATCH: Duration = Duration::from_millis(150);

/// Last observed output value for the live preview.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputActivityValue {
    Axis(f64),
    Button(bool),
    Hat(HatDirection),
    Keyboard(bool),
    Mouse(bool),
}

impl OutputActivityValue {
    #[must_use]
    pub fn released_like(&self) -> Self {
        match self {
            Self::Axis(_) => Self::Axis(0.0),
            Self::Button(_) => Self::Button(false),
            Self::Hat(_) => Self::Hat(HatDirection::Center),
            Self::Keyboard(_) => Self::Keyboard(false),
            Self::Mouse(_) => Self::Mouse(false),
        }
    }

    #[must_use]
    pub fn is_active(self) -> bool {
        match self {
            Self::Axis(value) => value.abs() > f64::EPSILON,
            Self::Button(pressed) | Self::Keyboard(pressed) | Self::Mouse(pressed) => pressed,
            Self::Hat(direction) => direction != HatDirection::Center,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct OutputActivityEntry {
    value: OutputActivityValue,
    expires_at: Option<Instant>,
}

/// Display-only store of recently active outputs.
#[derive(Debug, Default)]
pub struct OutputActivityStore {
    entries: HashMap<OutputOwner, OutputActivityEntry>,
}

impl OutputActivityStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(
        &mut self,
        owner: OutputOwner,
        value: OutputActivityValue,
        now: Instant,
        latch_inactive: bool,
    ) {
        if value.is_active() {
            self.entries.insert(
                owner,
                OutputActivityEntry {
                    value,
                    expires_at: None,
                },
            );
            return;
        }

        if latch_inactive
            && self
                .entries
                .get(&owner)
                .is_some_and(|entry| entry.value.is_active())
        {
            if let Some(entry) = self.entries.get_mut(&owner) {
                entry.expires_at = Some(now + OUTPUT_ACTIVITY_PREVIEW_LATCH);
            }
        } else {
            self.entries.remove(&owner);
        }
    }

    #[must_use]
    pub fn get(&self, owner: &OutputOwner, now: Instant) -> Option<OutputActivityValue> {
        let entry = self.entries.get(owner)?;
        entry
            .expires_at
            .is_none_or(|expires_at| now <= expires_at)
            .then_some(entry.value)
    }

    pub fn active_entries(
        &self,
        now: Instant,
    ) -> impl Iterator<Item = (&OutputOwner, OutputActivityValue)> {
        self.entries.iter().filter_map(move |(owner, entry)| {
            entry
                .expires_at
                .is_none_or(|expires_at| now <= expires_at)
                .then_some((owner, entry.value))
        })
    }

    pub fn prune(&mut self, now: Instant) {
        self.entries
            .retain(|_, entry| entry.expires_at.is_none_or(|expires_at| now <= expires_at));
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
