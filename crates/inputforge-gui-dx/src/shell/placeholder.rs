use dioxus::prelude::*;

use crate::components::Tabs;
use crate::shell::status_bar_view::StatusBarView;

const PLACEHOLDER_SHELL_CSS: Asset = asset!("/assets/shell/placeholder-shell.css");

/// Disposable four-region shell grid. Mounted by `app_root` once F3 Task 18
/// wires the shell into the launch path; F5 replaces this entirely.
#[allow(dead_code, reason = "consumed by app_root in F3 Task 18 (shell mount)")]
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
                        ("mappings".into(), "Mappings".into()),
                        ("modes".into(),    "Modes".into()),
                    ],
                    value: center_tab.read().clone(),
                    onchange: move |id: String| center_tab.set(id),
                }
                div { class: "if-placeholder-shell__center-body",
                    "Center placeholder — F7+ owns content"
                }
            }
            StatusBarView {}
        }
    }
}
