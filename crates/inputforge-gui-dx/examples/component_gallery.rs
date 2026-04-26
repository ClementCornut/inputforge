//! Visual harness for all F2 primitives.
//!
//! Run via:
//!     dx serve --example `component_gallery` --platform desktop
//!
//! Mounts `ThemeProvider` directly — no engine state required.
//! Hot-reload friendly: editing CSS or RSX updates instantly.

use dioxus::prelude::*;
#[allow(
    unused_imports,
    reason = "Label is re-exported for consumers; Field uses it internally"
)]
use inputforge_gui_dx::components::{
    Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, Card, CardPadding, Checkbox, Cluster,
    Field, Icon, IconButton, InputSize, Label, MenuItem, MenuItems, MenuRoot, MenuTrigger,
    NumberInput, Select, Separator, SeparatorOrientation, Slider, Spinner, SpinnerSize, Stack,
    StatusBar, Switch, TabItem, Tabs, TextInput, Tooltip, TooltipPlacement,
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
    // Interactive demo state for NumberInput steppers — each non-disabled
    // demo wires its own signal so they're independently steppable.
    let mut number_demo = use_signal(|| 50.0_f64);
    let mut number_demo_b = use_signal(|| 50.0_f64);
    let mut number_demo_precision = use_signal(|| 0.123_456_f64);
    let mut number_demo_size = use_signal(|| 0.0_f64);
    let mut tabs_demo = use_signal(|| "first".to_owned());
    rsx! {
        ThemeProvider {
            main {
                Stack { gap: "--space-8".to_owned(), padding: "--space-6".to_owned(),
                    h1 { "InputForge — Component Gallery (F2)" }

                    section {
                        h2 { "Icon" }
                        Card { padding: CardPadding::Md,
                            Cluster { gap: "--space-4".to_owned(),
                                Icon { name: IconKind::Joystick, size: IconSize::Sm }
                                Icon { name: IconKind::Joystick, size: IconSize::Md }
                                Icon { name: IconKind::Joystick, size: IconSize::Lg }
                                Icon { name: IconKind::Settings }
                                Icon { name: IconKind::Save }
                                Icon { name: IconKind::Trash }
                            }
                        }
                    }

                    section {
                        h2 { "Button" }
                        Card { padding: CardPadding::Md,
                            Stack { gap: "--space-3".to_owned(),
                                Cluster { gap: "--space-3".to_owned(),
                                    Button { variant: ButtonVariant::Primary,   "Primary" }
                                    Button { variant: ButtonVariant::Secondary, "Secondary" }
                                    Button { variant: ButtonVariant::Ghost,     "Ghost" }
                                    Button { variant: ButtonVariant::Danger,    "Danger" }
                                    Button { disabled: true, "Disabled" }
                                }
                                Cluster { gap: "--space-3".to_owned(),
                                    Button { size: ButtonSize::Sm, "Small" }
                                    Button { size: ButtonSize::Md, "Medium" }
                                    Button { size: ButtonSize::Lg, "Large" }
                                }
                            }
                        }
                    }

                    section {
                        h2 { "IconButton" }
                        Card { padding: CardPadding::Md,
                            Cluster { gap: "--space-3".to_owned(),
                                IconButton { icon: IconKind::Settings, label: "Settings" }
                                IconButton { icon: IconKind::Save,     label: "Save",  variant: ButtonVariant::Primary }
                                IconButton { icon: IconKind::Trash,    label: "Delete", variant: ButtonVariant::Danger }
                                IconButton { icon: IconKind::Eye,      label: "Show",   disabled: true }
                            }
                        }
                    }

                    section {
                        h2 { "TextInput" }
                        Card { padding: CardPadding::Md,
                            // max-width belongs to the demo, not to Stack — keep on a wrapper div.
                            div { style: "max-width: 320px;",
                                Stack { gap: "--space-3".to_owned(),
                                    TextInput { value: "hello".to_owned(), placeholder: "Type here…".to_owned() }
                                    TextInput { value: String::new(), placeholder: "Disabled".to_owned(), disabled: true }
                                    TextInput { value: "wrong".to_owned(), invalid: true }
                                    TextInput { value: "Small".to_owned(),  size: InputSize::Sm }
                                    TextInput { value: "Medium".to_owned(), size: InputSize::Md }
                                    TextInput { value: "Large".to_owned(),  size: InputSize::Lg }
                                }
                            }
                        }
                    }

                    section {
                        h2 { "NumberInput" }
                        Card { padding: CardPadding::Md,
                            Stack { gap: "--space-3".to_owned(),
                                Cluster { gap: "--space-3".to_owned(),
                                    NumberInput {
                                        value: number_demo(),
                                        min: 0.0,
                                        max: 100.0,
                                        step: 1.0,
                                        onstep: move |v| number_demo.set(v),
                                    }
                                    NumberInput {
                                        value: number_demo_b(),
                                        min: 0.0,
                                        max: 100.0,
                                        step: 1.0,
                                        onstep: move |v| number_demo_b.set(v),
                                    }
                                    NumberInput { value: 0.0, disabled: true }
                                    NumberInput {
                                        value: number_demo_precision(),
                                        step: 0.01,
                                        precision: 2,
                                        onstep: move |v| number_demo_precision.set(v),
                                    }
                                }
                                Cluster { gap: "--space-3".to_owned(),
                                    NumberInput {
                                        value: number_demo_size(),
                                        step: 1.0,
                                        size: InputSize::Sm,
                                        onstep: move |v| number_demo_size.set(v),
                                    }
                                    NumberInput {
                                        value: number_demo_size(),
                                        step: 1.0,
                                        size: InputSize::Md,
                                        onstep: move |v| number_demo_size.set(v),
                                    }
                                    NumberInput {
                                        value: number_demo_size(),
                                        step: 1.0,
                                        size: InputSize::Lg,
                                        onstep: move |v| number_demo_size.set(v),
                                    }
                                }
                            }
                        }
                    }

                    section {
                        h2 { "Select" }
                        Card { padding: CardPadding::Md,
                            Stack { gap: "--space-3".to_owned(),
                                Cluster { gap: "--space-3".to_owned(),
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
                                Cluster { gap: "--space-3".to_owned(),
                                    Select {
                                        value: "alpha".to_owned(),
                                        options: vec![("alpha".to_owned(), "Small".to_owned())],
                                        size: InputSize::Sm,
                                    }
                                    Select {
                                        value: "alpha".to_owned(),
                                        options: vec![("alpha".to_owned(), "Medium".to_owned())],
                                        size: InputSize::Md,
                                    }
                                    Select {
                                        value: "alpha".to_owned(),
                                        options: vec![("alpha".to_owned(), "Large".to_owned())],
                                        size: InputSize::Lg,
                                    }
                                }
                            }
                        }
                    }

                    section {
                        h2 { "Slider" }
                        Card { padding: CardPadding::Md,
                            div { style: "max-width: 320px;",
                                Stack { gap: "--space-3".to_owned(),
                                    Slider { value: 0.5 }
                                    Slider { value: 0.25, disabled: true }
                                }
                            }
                        }
                    }

                    section {
                        h2 { "Switch" }
                        Card { padding: CardPadding::Md,
                            Cluster { gap: "--space-4".to_owned(),
                                Switch { checked: false, label: "Off".to_owned() }
                                Switch { checked: true,  label: "On".to_owned() }
                                Switch { checked: false, disabled: true, label: "Disabled".to_owned() }
                            }
                        }
                    }

                    section {
                        h2 { "Checkbox" }
                        Card { padding: CardPadding::Md,
                            Cluster { gap: "--space-4".to_owned(),
                                Checkbox { checked: false }
                                Checkbox { checked: true }
                                Checkbox { checked: false, indeterminate: true }
                                Checkbox { checked: false, disabled: true }
                            }
                        }
                    }

                    section {
                        h2 { "Card" }
                        Cluster { gap: "--space-3".to_owned(),
                            Card { padding: CardPadding::Sm, "Small padding" }
                            Card { padding: CardPadding::Md, "Medium padding" }
                            Card { padding: CardPadding::Lg, "Large padding" }
                        }
                    }

                    section {
                        h2 { "Badge" }
                        Cluster { gap: "--space-2".to_owned(),
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
                        // Vertical separator demo: explicit height keeps the separator
                        // visible without a surrounding flex stretch — not a Cluster fit.
                        div { style: "display: flex; gap: var(--space-3); align-items: center; height: 30px;",
                            span { "Left" }
                            Separator { orientation: SeparatorOrientation::Vertical }
                            span { "Right" }
                        }
                    }

                    section {
                        h2 { "Spinner" }
                        Cluster { gap: "--space-3".to_owned(),
                            Spinner { size: SpinnerSize::Sm }
                            Spinner { size: SpinnerSize::Md }
                            Spinner { size: SpinnerSize::Lg }
                        }
                    }

                    section {
                        h2 { "Tooltip" }
                        Card { padding: CardPadding::Md,
                            Cluster { gap: "--space-6".to_owned(),
                                Tooltip { content: "Hovers up".to_owned(),    Button { "Top" } }
                                Tooltip { content: "Hovers down".to_owned(),  placement: TooltipPlacement::Bottom, Button { "Bottom" } }
                                Tooltip { content: "Hovers left".to_owned(),  placement: TooltipPlacement::Left,   Button { "Left" } }
                                Tooltip { content: "Hovers right".to_owned(), placement: TooltipPlacement::Right,  Button { "Right" } }
                            }
                        }
                    }

                    section {
                        h2 { "Menu" }
                        Card { padding: CardPadding::Md,
                            MenuRoot {
                                MenuTrigger { "Open menu" }
                                MenuItems {
                                    MenuItem { "First action" }
                                    MenuItem { "Second action" }
                                    MenuItem { disabled: true, "Disabled action" }
                                }
                            }
                        }
                    }

                    section {
                        h2 { "Field + Label" }
                        Card { padding: CardPadding::Md,
                            div { style: "max-width: 320px;",
                                Stack { gap: "--space-4".to_owned(),
                                    // Field couples label↔input by passing the same string
                                    // to `for_id` (Field) and `id` (the wrapped input).
                                    Field {
                                        label: "Profile name".to_owned(),
                                        for_id: "profile-name".to_owned(),
                                        helper: "Used in dropdowns.".to_owned(),
                                        required: true,
                                        TextInput {
                                            id: "profile-name".to_owned(),
                                            value: String::new(),
                                            placeholder: "My profile".to_owned(),
                                        }
                                    }
                                    Field {
                                        label: "Sensitivity".to_owned(),
                                        for_id: "sensitivity".to_owned(),
                                        error: "Must be between 0 and 1.".to_owned(),
                                        NumberInput {
                                            id: "sensitivity".to_owned(),
                                            value: 1.5,
                                            min: 0.0,
                                            max: 1.0,
                                            step: 0.01,
                                            precision: 2,
                                        }
                                    }
                                }
                            }
                        }
                    }

                    section {
                        h2 { "Tabs" }
                        Card { padding: CardPadding::Md,
                            Stack { gap: "--space-3".to_owned(),
                                p {
                                    "Active tab: "
                                    code { "{tabs_demo}" }
                                    " — use Left/Right or Home/End to cycle."
                                }
                                Tabs {
                                    items: vec![
                                        TabItem {
                                            id: "first".into(),
                                            label: "First".into(),
                                            controls: Some("first-panel".into()),
                                        },
                                        TabItem {
                                            id: "second".into(),
                                            label: "Second".into(),
                                            controls: Some("second-panel".into()),
                                        },
                                        TabItem {
                                            id: "third".into(),
                                            label: "Third".into(),
                                            controls: Some("third-panel".into()),
                                        },
                                    ],
                                    value: tabs_demo.read().clone(),
                                    onchange: move |id: String| tabs_demo.set(id),
                                }
                                {
                                    match tabs_demo.read().as_str() {
                                        "first" => rsx! {
                                            div { role: "tabpanel",
                                                  id: "first-panel",
                                                  "aria-labelledby": "tab-first",
                                                  "First panel content." }
                                        },
                                        "second" => rsx! {
                                            div { role: "tabpanel",
                                                  id: "second-panel",
                                                  "aria-labelledby": "tab-second",
                                                  "Second panel content." }
                                        },
                                        "third" => rsx! {
                                            div { role: "tabpanel",
                                                  id: "third-panel",
                                                  "aria-labelledby": "tab-third",
                                                  "Third panel content." }
                                        },
                                        _ => rsx! { div {} },
                                    }
                                }
                                p { "Disabled state:" }
                                Tabs {
                                    items: vec![
                                        TabItem {
                                            id: "a".into(),
                                            label: "Disabled A".into(),
                                            controls: None,
                                        },
                                        TabItem {
                                            id: "b".into(),
                                            label: "Disabled B".into(),
                                            controls: None,
                                        },
                                    ],
                                    value: "a".to_owned(),
                                    onchange: move |_: String| {},
                                    disabled: true,
                                }
                            }
                        }
                    }

                    section {
                        h2 { "StatusBar" }
                        Stack { gap: "--space-3".to_owned(),
                            p { "Composed slots (Badge + Separator + Badge / text / span):" }
                            Card { padding: CardPadding::Md,
                                StatusBar {
                                    start: rsx! {
                                        Badge { variant: BadgeVariant::Success, "Running" }
                                        Separator { orientation: SeparatorOrientation::Vertical }
                                        Badge { variant: BadgeVariant::Neutral, "Default" }
                                    },
                                    middle: rsx! { span { "2/3 devices" } },
                                    end: rsx! { span { "Demo Profile" } },
                                }
                            }
                            p { "Empty slots — verifies slot independence:" }
                            Card { padding: CardPadding::Md,
                                StatusBar {
                                    start:  rsx! {},
                                    middle: rsx! {},
                                    end:    rsx! {},
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
