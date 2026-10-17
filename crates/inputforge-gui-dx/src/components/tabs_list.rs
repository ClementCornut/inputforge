//! Tablist markup, rendering references and execution of app-selected focus targets.
//! Keyboard events and mounted availability observations go to the host callbacks.

use std::rc::Rc;

use dioxus::prelude::*;

use super::merge_class;
use super::tabs_root::TabsContext;

/// Tablist orientation. Horizontal honors `ArrowLeft` / `ArrowRight`;
/// vertical honors `ArrowUp` / `ArrowDown`. Home / End apply in both.
#[cfg(not(test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabsOrientation {
    Horizontal,
    Vertical,
}

#[cfg(test)]
pub use inputforge_gui_dx::components::TabsOrientation;

#[component]
pub fn TabsList(
    #[props(default)] aria_label: Option<String>,
    #[props(default = TabsOrientation::Horizontal)] orientation: TabsOrientation,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let ctx = use_context::<TabsContext>();

    let registry = ctx.registry;
    use_effect(move || {
        let request = ctx.focus_request.and_then(|r| r.read().clone());
        let _active = ctx.value.read();
        let _disabled = ctx.disabled.read();
        let _pending = ctx.behavior.pending.read().clone();
        let entries = registry.read().clone();
        let result = ctx.behavior.focus.call((request, tab_states(&entries)));
        if result.consumed
            && let Some(mut request) = ctx.focus_request
        {
            request.set(None);
        }
        if let Some(target) = result.target
            && let Some(entry) = entries.iter().find(|e| e.id == target)
        {
            let node = Rc::clone(&entry.mounted);
            spawn(async move {
                let _ = node.set_focus(true).await;
            });
        }
    });

    let combined = merge_class(
        "if-tabs",
        if (ctx.disabled)() {
            "if-tabs--disabled"
        } else {
            ""
        },
        class.as_deref(),
    );
    let orientation_str = match orientation {
        TabsOrientation::Horizontal => "horizontal",
        TabsOrientation::Vertical => "vertical",
    };
    let onkeydown = move |evt| {
        ctx.behavior
            .key
            .call((evt, tab_states(&ctx.registry.read()), orientation));
    };

    rsx! {
        div {
            class: "{combined}",
            role: "tablist",
            "aria-orientation": "{orientation_str}",
            "aria-label": aria_label.as_deref(),
            onkeydown,
            {children}
        }
    }
}

fn tab_states(entries: &[super::tabs_root::TabRegistryEntry]) -> Vec<crate::controls::TabState> {
    entries
        .iter()
        .map(|entry| crate::controls::TabState {
            id: entry.id.clone(),
            disabled: entry.disabled,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case, reason = "Dioxus components are PascalCase")]

    use dioxus::prelude::*;
    use dioxus_ssr::render;

    use super::super::tabs_root::TabsRoot;
    use super::{TabsList, TabsOrientation};

    /// Renders the canonical class shape, role, and orientation. The
    /// keyboard coordinator and registry-driven nav are exercised in
    /// the integration suite once `TabButton` lands.
    #[test]
    fn tabs_list_renders_canonical_tablist_shape() {
        fn TestComponent() -> Element {
            rsx! {
                TabsRoot {
                    value: "x".to_owned(),
                    onchange: move |_: String| {},
                    TabsList { aria_label: "Demo".to_owned(),
                        span { "child" }
                    }
                }
            }
        }
        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(html.contains("if-tabs"), "if-tabs class missing: {html}");
        assert!(
            html.contains("role=\"tablist\""),
            "role=tablist missing: {html}",
        );
        assert!(
            html.contains("aria-orientation=\"horizontal\""),
            "aria-orientation default must be horizontal: {html}",
        );
        assert!(
            html.contains("aria-label=\"Demo\""),
            "aria-label must propagate: {html}",
        );
        assert!(
            !html.contains("if-tabs--disabled"),
            "if-tabs--disabled must be absent when cluster is enabled: {html}",
        );
    }

    /// Cluster-level `disabled` adds the `if-tabs--disabled` class to
    /// the wrapper and gates the keyboard handler (the gate itself is
    /// integration-tested in commit 6).
    #[test]
    fn tabs_list_emits_disabled_class_when_cluster_disabled() {
        fn TestComponent() -> Element {
            rsx! {
                TabsRoot {
                    value: "x".to_owned(),
                    onchange: move |_: String| {},
                    disabled: true,
                    TabsList {}
                }
            }
        }
        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            html.contains("if-tabs--disabled"),
            "if-tabs--disabled must render when cluster disabled: {html}",
        );
    }

    /// Vertical orientation flips the aria attribute. Arrow-key axis
    /// switching is integration-tested with `TabButton`.
    #[test]
    fn tabs_list_vertical_orientation_propagates_aria() {
        fn TestComponent() -> Element {
            rsx! {
                TabsRoot {
                    value: "x".to_owned(),
                    onchange: move |_: String| {},
                    TabsList { orientation: TabsOrientation::Vertical }
                }
            }
        }
        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            html.contains("aria-orientation=\"vertical\""),
            "vertical orientation must reach aria-orientation: {html}",
        );
    }

    /// An empty `TabsList` (no `TabButton` children) must still render
    /// the canonical wrapper without panicking, the registry is just
    /// empty and the keyboard handler short-circuits on len==0.
    #[test]
    fn tabs_list_renders_safely_with_no_children() {
        fn TestComponent() -> Element {
            rsx! {
                TabsRoot {
                    value: "noop".to_owned(),
                    onchange: move |_: String| {},
                    TabsList {}
                }
            }
        }
        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            html.contains("role=\"tablist\""),
            "empty TabsList still renders the tablist wrapper: {html}",
        );
    }

    /// A sibling rendered alongside `TabsList` under the same
    /// `TabsRoot` must land OUTSIDE the `role="tablist"` container.
    /// This is the composition pattern `mode_tabs`'s tail `+` and
    /// context menu rely on so AT tab counts stay honest.
    #[test]
    fn tabs_root_sibling_renders_outside_tablist_wrapper() {
        use super::super::tab_button::TabButton;
        fn TestComponent() -> Element {
            rsx! {
                TabsRoot {
                    value: "a".to_owned(),
                    onchange: move |_: String| {},
                    TabsList {
                        TabButton { id: "a".to_owned(), label: "A".to_owned() }
                    }
                    button {
                        r#type: "button",
                        class: "if-mode-tab--add",
                        "aria-label": "Add mode",
                        "+"
                    }
                }
            }
        }
        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        let tablist_open = html.find("role=\"tablist\"").expect("tablist must render");
        let tablist_close_relative = html[tablist_open..].find("</div>").expect("tablist closes");
        let tablist_close = tablist_open + tablist_close_relative;
        let plus_idx = html
            .find("aria-label=\"Add mode\"")
            .expect("Add mode button must render");
        assert!(
            plus_idx > tablist_close,
            "Sibling-of-tablist composition must place the sibling AFTER the tablist closes. \
             tablist_close={tablist_close}, plus_idx={plus_idx}",
        );
    }

    /// Locks the focus-roving tabindex contract: the active tab gets
    /// tabindex=0, every other tab gets tabindex=-1. This is what
    /// keeps Tab key from cycling through every tab when the user
    /// expects `ArrowKeys` to handle it.
    #[test]
    fn tabs_list_focus_roving_assigns_tabindex_zero_only_to_active_tab() {
        use super::super::tab_button::TabButton;
        fn TestComponent() -> Element {
            rsx! {
                TabsRoot {
                    value: "second".to_owned(),
                    onchange: move |_: String| {},
                    TabsList {
                        TabButton { id: "first".to_owned(), label: "First".to_owned() }
                        TabButton { id: "second".to_owned(), label: "Second".to_owned() }
                        TabButton { id: "third".to_owned(), label: "Third".to_owned() }
                    }
                }
            }
        }
        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert_eq!(
            html.matches("tabindex=\"0\"").count(),
            1,
            "exactly one tab must carry tabindex=0 (the active one): {html}",
        );
        assert_eq!(
            html.matches("tabindex=\"-1\"").count(),
            2,
            "every other tab must carry tabindex=-1: {html}",
        );
    }

    /// A disabled `TabButton` carries the HTML `disabled` attribute, so
    /// click activation and tab-key focus both respect the flag at the
    /// browser layer (independent of the keyboard coordinator's
    /// registry-based skip).
    #[test]
    fn tabs_list_disabled_tab_button_renders_disabled_attribute() {
        use super::super::tab_button::TabButton;
        fn TestComponent() -> Element {
            rsx! {
                TabsRoot {
                    value: "a".to_owned(),
                    onchange: move |_: String| {},
                    TabsList {
                        TabButton { id: "a".to_owned(), label: "A".to_owned() }
                        TabButton {
                            id: "b".to_owned(),
                            label: "B".to_owned(),
                            disabled: true,
                        }
                    }
                }
            }
        }
        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            html.contains("disabled"),
            "disabled TabButton must emit the HTML disabled attribute: {html}",
        );
    }

    /// Cluster-level `TabsRoot::disabled` must propagate to every
    /// `TabButton` so the whole strip is non-interactive at the
    /// browser layer, not just at the keyboard coordinator.
    #[test]
    fn tabs_root_cluster_disabled_disables_every_tab_button() {
        use super::super::tab_button::TabButton;
        fn TestComponent() -> Element {
            rsx! {
                TabsRoot {
                    value: "a".to_owned(),
                    onchange: move |_: String| {},
                    disabled: true,
                    TabsList {
                        TabButton { id: "a".to_owned(), label: "A".to_owned() }
                        TabButton { id: "b".to_owned(), label: "B".to_owned() }
                    }
                }
            }
        }
        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            html.matches("disabled").count() >= 2,
            "every TabButton under a disabled TabsRoot must render `disabled`: {html}",
        );
    }
}
