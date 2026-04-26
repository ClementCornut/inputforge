//! Bridges `MetaSnapshot.warnings` (engine-side, polled every 16 ms by
//! `bridge::spawn_polling_task`) into the toast queue. The polling task
//! gates writes via `PartialEq` so this `use_effect` only re-runs on actual
//! snapshot changes; the length-diff guard makes spurious re-runs (e.g.
//! `engine_status` flip without a new warning) idempotent.

use dioxus::prelude::*;

use crate::context::AppContext;
use crate::toast::{ToastLevel, ToastQueue};

/// Returns a closure suitable for `use_effect`. Watches `ctx.meta` for new
/// tail entries on `warnings` and pushes them as Warning-level toasts.
///
/// `last_seen` is initialized by the caller to `ctx.meta.peek().warnings.len()`
/// so the first run is a no-op even if warnings accumulated before mount.
///
/// `Signal<T>` is `Copy`; the closure rebinds `last_seen` as `mut` so it can
/// call `.set(...)`. The same shape is used by the F1 polling task in
/// `bridge.rs`.
#[expect(dead_code, reason = "consumed by app_root in F4 Task 16")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "ctx is moved into the returned `'static` FnMut closure"
)]
pub(crate) fn install_warnings_bridge(
    ctx: AppContext,
    toasts: ToastQueue,
    last_seen: Signal<usize>,
) -> impl FnMut() + 'static {
    move || {
        let meta = ctx.meta.read();
        let len = meta.warnings.len();
        let mut seen = last_seen;
        let last = *seen.peek();
        if len > last {
            for msg in &meta.warnings[last..] {
                toasts.push(ToastLevel::Warning, msg.clone());
            }
            seen.set(len);
        } else if len < last {
            // Engine cleared/reset warnings — re-baseline.
            seen.set(len);
        }
    }
}
