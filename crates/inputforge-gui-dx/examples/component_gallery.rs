//! Visual harness for all F2 primitives.
//!
//! Run via:
//!     dx serve --example `component_gallery` --platform desktop
//!
//! Mounts `ThemeProvider` directly — no engine state required.
//! Hot-reload friendly: editing CSS or RSX updates instantly.

use dioxus::prelude::*;
use inputforge_gui_dx::components::{
    Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, Card, CardPadding, Checkbox, Icon,
    IconButton, NumberInput, Select, Separator, SeparatorOrientation, Slider, Spinner, SpinnerSize,
    Switch, TextInput,
};
use inputforge_gui_dx::icons::{Icon as IconKind, IconSize};
use inputforge_gui_dx::theme::ThemeProvider;

fn main() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    LaunchBuilder::desktop().launch(gallery_root);
}

#[allow(
    clippy::too_many_lines,
    reason = "gallery function intentionally lists all primitives in one place"
)]
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

                section {
                    h2 { "Switch" }
                    div { style: "display: flex; gap: var(--space-4);",
                        Switch { checked: false, label: "Off".to_owned() }
                        Switch { checked: true,  label: "On".to_owned() }
                        Switch { checked: false, disabled: true, label: "Disabled".to_owned() }
                    }
                }

                section {
                    h2 { "Checkbox" }
                    div { style: "display: flex; gap: var(--space-4); align-items: center;",
                        Checkbox { checked: false }
                        Checkbox { checked: true }
                        Checkbox { checked: false, indeterminate: true }
                        Checkbox { checked: false, disabled: true }
                    }
                }

                section {
                    h2 { "Card" }
                    div { style: "display: flex; gap: var(--space-3);",
                        Card { padding: CardPadding::Sm, "Small padding" }
                        Card { padding: CardPadding::Md, "Medium padding" }
                        Card { padding: CardPadding::Lg, "Large padding" }
                    }
                }

                section {
                    h2 { "Badge" }
                    div { style: "display: flex; gap: var(--space-2);",
                        Badge { variant: BadgeVariant::Neutral, "Neutral" }
                        Badge { variant: BadgeVariant::Info,    "Info" }
                        Badge { variant: BadgeVariant::Success, "Success" }
                        Badge { variant: BadgeVariant::Warning, "Warning" }
                        Badge { variant: BadgeVariant::Error,   "Error" }
                    }
                }

                section {
                    h2 { "Separator" }
                    Separator {}
                    div { style: "display: flex; gap: var(--space-3); align-items: center; height: 30px;",
                        span { "Left" }
                        Separator { orientation: SeparatorOrientation::Vertical }
                        span { "Right" }
                    }
                }

                section {
                    h2 { "Spinner" }
                    div { style: "display: flex; gap: var(--space-3); align-items: center;",
                        Spinner { size: SpinnerSize::Sm }
                        Spinner { size: SpinnerSize::Md }
                        Spinner { size: SpinnerSize::Lg }
                    }
                }
            }
        }
    }
}
