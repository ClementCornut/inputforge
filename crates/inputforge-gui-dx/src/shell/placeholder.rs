use dioxus::prelude::*;

use crate::components::{TabItem, Tabs};
use crate::shell::status_bar_view::StatusBarView;

#[allow(dead_code, reason = "shell/ stays compilable until Task 32 removes it")]
const PLACEHOLDER_SHELL_CSS: Asset = asset!("/assets/shell/placeholder-shell.css");

/// Disposable four-region shell grid. Mounted by `app_root`; F5 replaces
/// this entirely.
#[component]
pub(crate) fn PlaceholderShell() -> Element {
    let mut center_tab = use_signal(|| "mappings".to_owned());

    rsx! {
        Stylesheet { href: PLACEHOLDER_SHELL_CSS }
        div { class: "if-placeholder-shell",
            div { class: "if-placeholder-shell__top",
                "Top toolbar (F5 owns contents)"
            }
            div { class: "if-placeholder-shell__left",
                "Left panel — devices (F6)"
            }
            div { class: "if-placeholder-shell__center",
                Tabs {
                    items: vec![
                        TabItem {
                            id: "mappings".into(),
                            label: "Mappings".into(),
                            controls: Some("mappings-panel".into()),
                        },
                        TabItem {
                            id: "modes".into(),
                            label: "Modes".into(),
                            controls: Some("modes-panel".into()),
                        },
                    ],
                    value: center_tab.read().clone(),
                    onchange: move |id: String| center_tab.set(id),
                }
                {
                    match center_tab.read().as_str() {
                        "mappings" => rsx! {
                            div { class: "if-placeholder-shell__center-body",
                                role: "tabpanel",
                                id: "mappings-panel",
                                "aria-labelledby": "tab-mappings",
                                "Mappings placeholder — F7 owns content"
                            }
                        },
                        "modes" => rsx! {
                            div { class: "if-placeholder-shell__center-body",
                                role: "tabpanel",
                                id: "modes-panel",
                                "aria-labelledby": "tab-modes",
                                "Modes placeholder — F11 owns content"
                            }
                        },
                        _ => rsx! { div {} },
                    }
                }
            }
            StatusBarView {}
        }
    }
}
