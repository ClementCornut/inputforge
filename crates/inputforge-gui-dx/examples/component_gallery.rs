//! Visual harness for all F2 primitives.
//!
//! Run via:
//!     dx serve --example `component_gallery` --platform desktop
//!
//! Mounts `ThemeProvider` directly — no engine state required.
//! Hot-reload friendly: editing CSS or RSX updates instantly.

use dioxus::prelude::*;
use inputforge_gui_dx::components::{
    Button, ButtonSize, ButtonVariant, Icon, IconButton, NumberInput, Select, Slider, TextInput,
};
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

                section {
                    h2 { "Button" }
                    div {
                        style: "display: flex; gap: var(--space-3); flex-wrap: wrap; align-items: center;",
                        Button { variant: ButtonVariant::Primary,   "Primary" }
                        Button { variant: ButtonVariant::Secondary, "Secondary" }
                        Button { variant: ButtonVariant::Ghost,     "Ghost" }
                        Button { variant: ButtonVariant::Danger,    "Danger" }
                        Button { disabled: true, "Disabled" }
                    }
                    div {
                        style: "display: flex; gap: var(--space-3); margin-top: var(--space-3);",
                        Button { size: ButtonSize::Sm, "Small" }
                        Button { size: ButtonSize::Md, "Medium" }
                        Button { size: ButtonSize::Lg, "Large" }
                    }
                }

                section {
                    h2 { "IconButton" }
                    div {
                        style: "display: flex; gap: var(--space-3); align-items: center;",
                        IconButton { icon: IconKind::Settings, label: "Settings" }
                        IconButton { icon: IconKind::Save,     label: "Save",  variant: ButtonVariant::Primary }
                        IconButton { icon: IconKind::Trash,    label: "Delete", variant: ButtonVariant::Danger }
                        IconButton { icon: IconKind::Eye,      label: "Show",   disabled: true }
                    }
                }

                section {
                    h2 { "TextInput" }
                    div {
                        style: "display: flex; flex-direction: column; gap: var(--space-3); max-width: 320px;",
                        TextInput { value: "hello".to_owned(), placeholder: "Type here…".to_owned() }
                        TextInput { value: String::new(), placeholder: "Disabled".to_owned(), disabled: true }
                        TextInput { value: "wrong".to_owned(), invalid: true }
                    }
                }

                section {
                    h2 { "NumberInput" }
                    div {
                        style: "display: flex; gap: var(--space-3); align-items: center;",
                        NumberInput { value: 0.0, min: 0.0, max: 100.0, step: 1.0 }
                        NumberInput { value: 50.0, min: 0.0, max: 100.0, step: 1.0 }
                        NumberInput { value: 0.0, disabled: true }
                    }
                }

                section {
                    h2 { "Select" }
                    div {
                        style: "display: flex; gap: var(--space-3); align-items: center;",
                        Select {
                            value: "alpha".to_owned(),
                            options: vec![
                                ("alpha".to_owned(),   "Alpha".to_owned()),
                                ("beta".to_owned(),    "Beta".to_owned()),
                                ("gamma".to_owned(),   "Gamma".to_owned()),
                            ],
                        }
                        Select {
                            value: "alpha".to_owned(),
                            options: vec![("alpha".to_owned(), "Alpha".to_owned())],
                            disabled: true,
                        }
                    }
                }

                section {
                    h2 { "Slider" }
                    div {
                        style: "display: flex; flex-direction: column; gap: var(--space-3); max-width: 320px;",
                        Slider { value: 0.5 }
                        Slider { value: 0.25, disabled: true }
                    }
                }
            }
        }
    }
}
