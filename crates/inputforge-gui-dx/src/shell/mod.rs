//! Placeholder shell — disposable at F5.
//!
//! This module exists to give F3 a coherent four-region grid that the
//! tray-bridge lifecycle can be observed against (open the window, watch
//! the status bar reflect engine state, click tray Toggle, watch the badge
//! flip). F5 will redesign IA and may replace the entire grid template
//! (not just slot contents). Treat every line of CSS in
//! `assets/shell/placeholder-shell.css` and every grid-area definition in
//! `placeholder.rs` as scratch.

mod placeholder;
mod status_bar_view;

#[allow(
    unused_imports,
    reason = "consumed by app_root in F3 Task 18 (shell mount)"
)]
pub(crate) use placeholder::PlaceholderShell;
