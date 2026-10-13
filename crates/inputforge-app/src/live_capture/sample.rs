//! Capture detection sampled by the existing editor polling task.
use super::{Controller, machine, policy};
use dioxus::prelude::*;
use inputforge_core::state::AppState;
use inputforge_gui_dx::snapshots::CaptureEpoch;
use std::time::Instant;

impl Controller {
    /// Sample capture under the polling bridge's coherent engine guard.
    pub fn sample(&self, state: &AppState, now: Instant) {
        let mut inner = self.inner.borrow_mut();
        let publication = inner.publication;
        let Some(armed) = inner.armed.as_mut() else {
            return;
        };
        if armed.cancelled {
            return;
        }
        if !armed
            .request
            .target
            .try_peek()
            .is_ok_and(|target| *target == armed.target)
            || !publication.is_some_and(|p| policy::navigation(p, &armed.target))
            || !policy::valid(state, &armed.target)
        {
            armed.cancelled = true;
            return;
        }
        let layouts = policy::layouts(state);
        let changed =
            armed.generation != Some(state.session.generation) || armed.layouts != layouts;
        if armed.candidate.is_some() {
            armed.cancelled = changed || !state.session.ready;
            return;
        }
        if changed {
            armed.core.baseline = None;
            armed.core.pending = None;
        }
        let epoch = CaptureEpoch {
            generation: state.session.generation,
            ready: state.session.ready,
            allowed: !state.session.offline,
        };
        let (core, candidate) = machine::step_session(
            std::mem::take(&mut armed.core),
            &state.input_cache.clone_compact(),
            now,
            armed.generation,
            epoch,
        );
        armed.core = core;
        armed.generation = Some(epoch.generation);
        armed.layouts = layouts;
        armed.candidate = candidate;
    }
}
