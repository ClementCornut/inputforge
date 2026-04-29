//! Placeholder shell — disposable at F5.
//!
//! This module exists to give F3 a coherent four-region grid that the
//! tray-bridge lifecycle can be observed against (open the window, watch
//! the status bar reflect engine state, click tray Toggle, watch the badge
//! flip). F5 will redesign IA and may replace the entire grid template
//! (not just slot contents). Treat every line of CSS in
//! `assets/shell/placeholder-shell.css` and every grid-area definition in
//! `placeholder.rs` as scratch.
//!
//! ## CSS mount choice
//!
//! `placeholder-shell.css` mounts via `Stylesheet { href: ... }` inside
//! `PlaceholderShell` itself, NOT in `theme/mod.rs` alongside the other
//! component CSS files. This is intentional: the CSS is shell-scoped (only
//! meaningful when the placeholder is rendered) and component-disposable.
//! Keeping the asset reference local means deleting this module at F5
//! cleans up its CSS without touching `ThemeProvider`.

mod placeholder;
mod status_bar_view;

#[allow(
    unused_imports,
    reason = "shell/ stays compilable until Task 32 removes it"
)]
pub(crate) use placeholder::PlaceholderShell;
