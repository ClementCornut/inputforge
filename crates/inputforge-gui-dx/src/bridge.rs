use std::time::Duration;

use dioxus::prelude::*;

use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot};

/// Spawn the ~60Hz state-bridge polling task (16ms tick interval).
///
/// Each tick: non-blocking `try_read()` of `AppState`, rebuild the three
/// snapshots, write each via `Signal::set` only when `PartialEq` differs.
/// Idle state produces no wake-ups even while ticking.
///
/// The task is bound to the Dioxus runtime: it is auto-cancelled when the
/// runtime tears down on window close.
pub(crate) fn spawn_polling_task(ctx: AppContext) {
    spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_millis(16));
        loop {
            tick.tick().await;

            // Non-blocking: if the engine is currently writing, skip this tick.
            // One missed tick at 60Hz is imperceptible.
            let Some(guard) = ctx.state.try_read() else {
                continue;
            };

            let meta = MetaSnapshot::from_state(&guard);
            let config = ConfigSnapshot::from_state(&guard);
            let live = LiveSnapshot::from_state(&guard, &config);
            // Release the read lock before calling Signal::set; reactive re-reads of
            // ctx.state from subscribers must not contend with the held guard.
            drop(guard);

            // peek() reads without subscribing — the diff gate doesn't wake any component.
            let mut meta_signal = ctx.meta;
            let mut config_signal = ctx.config;
            let mut live_signal = ctx.live;
            if *meta_signal.peek() != meta {
                meta_signal.set(meta);
            }
            if *config_signal.peek() != config {
                config_signal.set(config);
            }
            if *live_signal.peek() != live {
                live_signal.set(live);
            }
        }
    });
}
