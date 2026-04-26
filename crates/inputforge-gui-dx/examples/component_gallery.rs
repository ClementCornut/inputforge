//! Visual harness for all F2 primitives.
//!
//! Run via:
//!     dx serve --example `component_gallery` --platform desktop
//!
//! Mounts `ThemeProvider` directly — no engine state required.
//! Hot-reload friendly: editing CSS or RSX updates instantly.

use dioxus::prelude::*;
use inputforge_gui_dx::theme::ThemeProvider;

fn main() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    LaunchBuilder::desktop().launch(gallery_root);
}

fn gallery_root() -> Element {
    rsx! {
        ThemeProvider {
            main {
                style: "padding: 24px;",
                h1 { "InputForge — Component Gallery (F2)" }
                p { "Primitives appear in sections below as Phase 4 lands them." }
            }
        }
    }
}
