//! Application-owned engine polling and prepared editor snapshots.
mod actions;
mod config;
mod device_usage;
pub(crate) mod devices;
mod live;
pub(crate) mod meta;
mod profile_metadata;
mod readout;
pub(crate) mod settings;
#[cfg(test)]
mod tests;
mod time_labels;

use dioxus::prelude::spawn;
use inputforge_core::state::AppState;
use inputforge_gui_dx::{
    bridge::Publisher,
    snapshots::{Frame, MappingKey},
};
use parking_lot::RwLock;
use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};

// Preserve the editor bridge's existing ~60 Hz sampling and immediate first tick.
const POLL_INTERVAL: Duration = Duration::from_millis(16);

/// Start one editor-scoped polling task; scope teardown cancels it.
pub fn spawn_polling_task(
    state: Arc<RwLock<AppState>>,
    publisher: Publisher,
    capture: crate::live_capture::Controller,
) {
    spawn(async move {
        let mut poller = Poller::new(state, capture.clone());
        let mut tick = tokio::time::interval(POLL_INTERVAL);
        loop {
            tick.tick().await;
            if let Some(frame) = poller.sample(
                publisher.selected_mapping().as_ref(),
                tokio::time::Instant::now().into_std(),
                modified,
            ) {
                publisher.advance_notifications(
                    Some(frame.meta.warnings.clone()),
                    tokio::time::Instant::now().into_std(),
                );
                publisher.publish(frame);
                publisher.reconcile_history();
                capture.deliver();
            } else {
                publisher.advance_notifications(None, tokio::time::Instant::now().into_std());
                if let Some(live) = poller.readout.expire(
                    publisher.selected_mapping().as_ref(),
                    tokio::time::Instant::now().into_std(),
                ) {
                    publisher.publish_readout(match live {
                        readout::Change::Clear => None,
                        readout::Change::Value(value) => Some(*value),
                    });
                }
            }
        }
    });
}
fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}
struct Poller {
    state: Arc<RwLock<AppState>>,
    metadata: profile_metadata::Cache,
    capture: crate::live_capture::Controller,
    readout: readout::Activity,
}
impl Poller {
    fn new(state: Arc<RwLock<AppState>>, capture: crate::live_capture::Controller) -> Self {
        Self {
            state,
            metadata: profile_metadata::Cache::default(),
            capture,
            readout: readout::Activity::default(),
        }
    }
    fn sample(
        &mut self,
        selection: Option<&MappingKey>,
        now: Instant,
        read_metadata: impl FnOnce(&Path) -> Option<SystemTime>,
    ) -> Option<Frame> {
        let mut frame = {
            let state = self.state.try_read()?;
            self.capture.sample(&state, now);
            prepare_at(&state, selection, now)
        };
        self.metadata.complete(&mut frame.meta, now, read_metadata);
        Some(self.readout.sample(frame, now))
    }
}

/// Prepare a coherent editor frame without performing filesystem I/O.
#[must_use]
pub fn prepare(state: &AppState, selection: Option<&MappingKey>) -> Frame {
    let mut frame = prepare_at(state, selection, Instant::now());
    if let Some(readout) = frame.live.readout.as_mut() {
        readout::overlay(readout, &frame.live.output_activity);
    }
    frame
}
fn prepare_at(state: &AppState, selection: Option<&MappingKey>, now: Instant) -> Frame {
    let config = config::project(state, selection);
    let mut live = live::project_at(state, &config, now);
    readout::project(state, &config, &mut live);
    Frame {
        meta: meta::project(state),
        live,
        settings: settings::project(state),
        routing: crate::routing_policy::view(state),
        controllers: crate::controllers::view(state, &config.device_panel_rows),
        config,
    }
}
