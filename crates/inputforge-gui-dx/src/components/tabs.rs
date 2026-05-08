//! Items-array facade over the decomposed `TabsRoot` + `TabsList` +
//! `TabButton` primitives. Lets simple consumers (response-curve
//! toolbar, component gallery) keep the original
//! `Tabs { value, onchange, items: Vec<TabItem> }` call shape while
//! the underlying canon lives in the headless leaves.
//!
//! For consumers that need per-tab event handlers, dom-id overrides,
//! ARIA menu attributes, or the rename-swap pattern, compose the
//! decomposed primitives directly (see `mode_tabs` for the canonical
//! example).

use dioxus::prelude::*;

use super::tab_button::TabButton;
use super::tabs_list::TabsList;
use super::tabs_root::TabsRoot;

/// One entry in a `Tabs` tablist.
///
/// `id` is the stable identifier the caller passes back via `onchange` and
/// matches against `value`. `label` is the visible button text. `controls`,
/// when set, is the DOM `id` of the tab's panel, it wires `aria-controls` on
/// the tab to a `role="tabpanel"` element the caller renders elsewhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabItem {
    pub id: String,
    pub label: String,
    pub controls: Option<String>,
    /// `true` marks this tab as the runtime-live one (orthogonal to
    /// `value`/`is_active`). Renders a 6px `--color-live` pip before
    /// the label when set. Default `false` for consumers that do not
    /// need the indicator.
    pub running: bool,
}

/// WAI-ARIA Tabs primitive with focus-roving and automatic activation.
/// Thin wrapper over `TabsRoot` + `TabsList` + `TabButton`; rendered HTML
/// is identical to the pre-decomposition primitive so existing consumers
/// require no changes.
#[component]
pub fn Tabs(
    /// Stable id of the active tab.
    value: String,
    onchange: EventHandler<String>,
    /// Tabs in display order. Each item carries its own id, label, and
    /// optional `controls` panel id.
    items: Vec<TabItem>,
    #[props(default)] class: Option<String>,
    #[props(default)] disabled: bool,
) -> Element {
    rsx! {
        TabsRoot {
            value: value,
            onchange: onchange,
            disabled: disabled,
            TabsList { class: class,
                for item in items.iter().cloned() {
                    TabButton {
                        key: "{item.id}",
                        id: item.id.clone(),
                        label: item.label,
                        running: item.running,
                        aria_controls: item.controls,
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case, reason = "Dioxus components are PascalCase")]

    use dioxus::prelude::*;
    use dioxus_ssr::render;

    use super::{TabItem, Tabs};

    #[test]
    fn tabs_renders_running_pip_when_tab_running_is_true() {
        fn TestComponent() -> Element {
            let items = vec![
                TabItem {
                    id: "default".to_owned(),
                    label: "Default".to_owned(),
                    controls: None,
                    running: true,
                },
                TabItem {
                    id: "combat".to_owned(),
                    label: "Combat".to_owned(),
                    controls: None,
                    running: false,
                },
            ];
            rsx! {
                Tabs {
                    value: "combat".to_owned(),
                    onchange: move |_: String| {},
                    items,
                }
            }
        }
        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            html.contains("if-tab__running-pip"),
            "running pip element missing on the running tab: {html}",
        );
    }

    #[test]
    fn tabs_does_not_render_running_pip_for_non_running_tabs() {
        fn TestComponent() -> Element {
            let items = vec![TabItem {
                id: "default".to_owned(),
                label: "Default".to_owned(),
                controls: None,
                running: false,
            }];
            rsx! {
                Tabs {
                    value: "default".to_owned(),
                    onchange: move |_: String| {},
                    items,
                }
            }
        }
        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            !html.contains("if-tab__running-pip"),
            "running pip must NOT appear when no tab is running: {html}",
        );
    }
}
