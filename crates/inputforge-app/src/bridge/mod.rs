//! Application-owned engine polling and prepared editor snapshots.
mod actions;
mod config;
mod device_usage;
mod devices;
mod live;
mod meta;
mod profile_metadata;
mod settings;
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
pub fn spawn_polling_task(state: Arc<RwLock<AppState>>, publisher: Publisher) {
    spawn(async move {
        let mut poller = Poller::new(state);
        let mut tick = tokio::time::interval(POLL_INTERVAL);
        loop {
            tick.tick().await;
            if let Some(frame) = poller.sample(
                publisher.selected_mapping().as_ref(),
                Instant::now(),
                modified,
            ) {
                publisher.publish(frame);
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
}
impl Poller {
    fn new(state: Arc<RwLock<AppState>>) -> Self {
        Self {
            state,
            metadata: profile_metadata::Cache::default(),
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
            prepare_at(&state, selection, now)
        };
        self.metadata.complete(&mut frame.meta, now, read_metadata);
        Some(frame)
    }
}

/// Prepare a coherent editor frame without performing filesystem I/O.
#[must_use]
pub fn prepare(state: &AppState, selection: Option<&MappingKey>) -> Frame {
    prepare_at(state, selection, Instant::now())
}
fn prepare_at(state: &AppState, selection: Option<&MappingKey>, now: Instant) -> Frame {
    let config = config::project(state, selection);
    Frame {
        meta: meta::project(state),
        live: live::project_at(state, &config, now),
        settings: settings::project(state),
        routing: crate::routing_policy::view(state),
        controllers: crate::controllers::view(state, &config.device_panel_rows),
        config,
    }
}
