use dioxus::prelude::*;
use inputforge_gui_dx::components::{
    Badge, BadgeVariant, Card, CardPadding, Icon, MenuGroup, MenuItem, MenuItems, MenuRoot,
    MenuTrigger, MenuTriggerPresentation, Separator, SeparatorOrientation, Stack, StatusBar,
    TabButton, TabItem, Tabs, TabsList, TabsRoot,
};
use inputforge_gui_dx::icons::Icon as IconKind;

#[component]
pub(super) fn NavigationGallery() -> Element {
    let mut compact_tab = use_signal(|| "compact-first".to_owned());
    let mut tabs_demo = use_signal(|| "first".to_owned());
    rsx! {
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
            h2 { "Tabs" }
            Card { padding: CardPadding::Md,
                Stack { gap: "--space-3".to_owned(),
                    p {
                        "Active tab: "
                        code { "{tabs_demo}" }
                        ", use Left/Right or Home/End to cycle."
                    }
                    Tabs {
                        items: vec![
                            TabItem {
                                id: "first".into(),
                                label: "First".into(),
                                controls: Some("first-panel".into()),
                                running: false,
                            },
                            TabItem {
                                id: "second".into(),
                                label: "Second".into(),
                                controls: Some("second-panel".into()),
                                running: false,
                            },
                            TabItem {
                                id: "third".into(),
                                label: "Third".into(),
                                controls: Some("third-panel".into()),
                                running: false,
                            },
                        ],
                        value: tabs_demo.read().clone(),
                        onchange: move |id: String| tabs_demo.set(id),
                    }
                    for (id, text) in [("first", "First"), ("second", "Second"), ("third", "Third")] {
                        div {
                            role: "tabpanel",
                            id: "{id}-panel",
                            "aria-labelledby": "tab-{id}",
                            hidden: tabs_demo.read().as_str() != id,
                            "{text} panel content."
                        }
                    }
                    p { "Disabled state:" }
                    Tabs {
                        items: vec![
                            TabItem {
                                id: "a".into(),
                                label: "Disabled A".into(),
                                controls: None,
                                running: false,
                            },
                            TabItem {
                                id: "b".into(),
                                label: "Disabled B".into(),
                                controls: None,
                                running: false,
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
                        middle: rsx! {
                            span { "2/3 devices" }
                        },
                        end: rsx! {
                            span { "Demo Profile" }
                        },
                    }
                }
                p { "Empty slots, verifies slot independence:" }
                Card { padding: CardPadding::Md,
                    StatusBar { start: rsx! {}, middle: rsx! {}, end: rsx! {} }
                }
            }
        }
        section {
            h2 { "Grouped icon menu and compact tabs" }
            MenuRoot {
                MenuTrigger {
                    presentation: MenuTriggerPresentation::Icon,
                    aria_label: "Preview actions",
                    Icon { name: IconKind::DotsVertical }
                }
                MenuItems {
                    MenuGroup { label: "Duplicate to mode",
                        MenuItem { compact: true, "Duplicate to mode Flight" }
                        MenuItem {
                            compact: true,
                            disabled: true,
                            title: "This mode is unavailable",
                            "Duplicate to mode Combat"
                        }
                    }
                    MenuItem { compact: true, danger: true, "Delete preview" }
                }
            }
            MenuRoot {
                MenuTrigger { presentation: MenuTriggerPresentation::Create, "Create preview" }
                MenuItems {
                    MenuItem { "New blank preview" }
                }
            }
            TabsRoot { value: compact_tab, onchange: move |id| compact_tab.set(id),
                TabsList {
                    compact: true,
                    scrollable: true,
                    aria_label: "Compact preview tabs",
                    TabButton {
                        id: "compact-first",
                        label: "A long preview tab label that truncates",
                        truncate: true,
                    }
                    TabButton {
                        id: "compact-second",
                        label: "Second",
                        truncate: true,
                    }
                }
            }
        }
    }
}
