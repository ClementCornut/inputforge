//! Visual harness for all F2 primitives.
//!
//! Run via:
//!     dx serve --example `component_gallery` --platform desktop
//!
//! Mounts `ThemeProvider` directly — no engine state required.
//! Hot-reload friendly: editing CSS or RSX updates instantly.

use dioxus::prelude::*;
use inputforge_gui_dx::components::Icon;
use inputforge_gui_dx::icons::{Icon as IconKind, IconSize};
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
                style: "padding: var(--space-6); display: flex; flex-direction: column; gap: var(--space-8);",
                h1 { "InputForge — Component Gallery (F2)" }

                section {
                    h2 { "Icon" }
                    div {
                        style: "display: flex; gap: var(--space-4); align-items: center;",
                        Icon { name: IconKind::Joystick, size: IconSize::Sm }
                        Icon { name: IconKind::Joystick, size: IconSize::Md }
                        Icon { name: IconKind::Joystick, size: IconSize::Lg }
                        Icon { name: IconKind::Settings }
                        Icon { name: IconKind::Save }
                        Icon { name: IconKind::Trash }
                    }
                }
            }
        }
    }
}
