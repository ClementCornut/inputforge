//! Fixed preview deadlines driven by the existing bridge tick.
use inputforge_core::{pipeline::OutputOwner, state::OUTPUT_ACTIVITY_PREVIEW_LATCH};
use inputforge_gui_dx::{
    readouts::{Key, Readout},
    snapshots::{Frame, MappingKey, OutputActivitySnapshot},
};
use std::time::Instant;
#[derive(Debug, PartialEq)]
pub(in crate::bridge) enum Change {
    Clear,
    Value(Box<Readout>),
}
#[derive(Default)]
pub(in crate::bridge) struct Activity {
    key: Option<Key>,
    entries: Vec<(OutputActivitySnapshot, Instant)>,
    observed: Vec<OutputOwner>,
    base: Option<Readout>,
    stale_at: Option<Instant>,
}
impl Activity {
    pub(in crate::bridge) fn sample(&mut self, mut frame: Frame, now: Instant) -> Frame {
        let key = frame.live.readout.as_ref().map(|r| r.key.clone());
        if self.key != key {
            self.entries.clear();
            self.observed.clear();
            self.key = key;
        }
        self.entries.retain(|(_, expiry)| now <= *expiry);
        for entry in &frame.live.output_activity {
            if let Some((held, _)) = self
                .entries
                .iter_mut()
                .find(|(held, _)| held.owner == entry.owner)
            {
                held.value = entry.value;
            } else if !self.observed.contains(&entry.owner) {
                self.entries
                    .push((entry.clone(), now + OUTPUT_ACTIVITY_PREVIEW_LATCH));
            }
        }
        self.observed = frame
            .live
            .output_activity
            .iter()
            .map(|entry| entry.owner.clone())
            .collect();
        self.base.clone_from(&frame.live.readout);
        // A last observed live value also needs a deadline if later reads are contended.
        self.stale_at = self
            .base
            .as_ref()
            .filter(|readout| {
                readout
                    .model
                    .outputs
                    .iter()
                    .any(|output| self.observed.contains(&output.owner))
            })
            .map(|_| now + OUTPUT_ACTIVITY_PREVIEW_LATCH);
        let mut activity = frame.live.output_activity.clone();
        for (held, _) in &self.entries {
            if !activity.iter().any(|entry| entry.owner == held.owner) {
                activity.push(held.clone());
            }
        }
        if let Some(readout) = frame.live.readout.as_mut() {
            super::overlay(readout, &activity);
        }
        frame
    }
    pub(in crate::bridge) fn expire(
        &mut self,
        selection: Option<&MappingKey>,
        now: Instant,
    ) -> Option<Change> {
        if self
            .key
            .as_ref()
            .is_some_and(|key| Some(&key.mapping) != selection)
        {
            self.key = None;
            self.base = None;
            self.stale_at = None;
            self.entries.clear();
            self.observed.clear();
            return Some(Change::Clear);
        }
        let previous = self.entries.len();
        self.entries.retain(|(_, expiry)| now <= *expiry);
        let stale = self.stale_at.is_some_and(|expiry| now > expiry);
        if stale {
            self.stale_at = None;
        }
        if previous == self.entries.len() && !stale {
            return None;
        }
        let mut readout = self.base.clone()?;
        let activity = self
            .entries
            .iter()
            .map(|(held, _)| held.clone())
            .collect::<Vec<_>>();
        super::overlay(&mut readout, &activity);
        Some(Change::Value(Box::new(readout)))
    }
}
