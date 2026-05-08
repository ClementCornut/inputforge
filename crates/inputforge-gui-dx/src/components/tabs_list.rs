//! Tablist primitive. Owns the `role="tablist"` wrapper, the canonical
//! `if-tabs` class shape (+ `if-tabs--disabled` when the cluster is
//! disabled), the `aria-orientation` + optional `aria-label`, and the
//! roving-tabindex keyboard coordinator.
//!
//! `TabsList` consumes the `TabsContext` published by `TabsRoot`. The
//! coordinator walks the context's `registry` (populated by mounted
//! `TabButton`s) to compute the next active id on Arrow / Home / End,
//! calls `onchange`, and moves focus to the new tab's `MountedData`
//! ref via `set_focus(true)`. Disabled registry entries are skipped.
//!
//! `TabsList` also watches the optional `focus_request` signal: when a
//! caller sets it to `Some(id)`, the watcher locates the matching ref
//! in the registry, focuses it, then clears the signal so subsequent
//! requests fire reliably.

use std::rc::Rc;

use dioxus::prelude::*;

use super::merge_class;
use super::tabs_root::TabsContext;

/// Tablist orientation. Horizontal honors `ArrowLeft` / `ArrowRight`;
/// vertical honors `ArrowUp` / `ArrowDown`. Home / End apply in both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabsOrientation {
    Horizontal,
    Vertical,
}

impl TabsOrientation {
    fn aria_value(self) -> &'static str {
        match self {
            TabsOrientation::Horizontal => "horizontal",
            TabsOrientation::Vertical => "vertical",
        }
    }
}

#[component]
pub fn TabsList(
    #[props(default)] aria_label: Option<String>,
    #[props(default = TabsOrientation::Horizontal)] orientation: TabsOrientation,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let ctx = use_context::<TabsContext>();

    if let Some(mut req) = ctx.focus_request {
        let registry = ctx.registry;
        use_effect(move || {
            let Some(target_id) = req.read().clone() else {
                return;
            };
            let entries = registry.read();
            if let Some(entry) = entries.iter().find(|e| e.id == target_id) {
                let node = Rc::clone(&entry.mounted);
                drop(entries);
                req.set(None);
                spawn(async move {
                    let _ = node.set_focus(true).await;
                });
            }
        });
    }

    let combined = merge_class(
        "if-tabs",
        if ctx.disabled {
            "if-tabs--disabled"
        } else {
            ""
        },
        class.as_deref(),
    );
    let orientation_str = orientation.aria_value();
    let cluster_disabled = ctx.disabled;

    let onkeydown = move |evt: KeyboardEvent| {
        if cluster_disabled {
            return;
        }
        let entries: Vec<_> = ctx.registry.read().clone();
        let len = entries.len();
        if len == 0 {
            return;
        }
        let current = ctx.value.read().clone();
        let cur_idx = entries.iter().position(|e| e.id == current).unwrap_or(0);
        let key = evt.key();
        let (start_idx, step): (usize, isize) = match key {
            Key::ArrowRight if orientation == TabsOrientation::Horizontal => {
                ((cur_idx + 1) % len, 1)
            }
            Key::ArrowLeft if orientation == TabsOrientation::Horizontal => {
                ((cur_idx + len - 1) % len, -1)
            }
            Key::ArrowDown if orientation == TabsOrientation::Vertical => ((cur_idx + 1) % len, 1),
            Key::ArrowUp if orientation == TabsOrientation::Vertical => {
                ((cur_idx + len - 1) % len, -1)
            }
            Key::Home => (0, 1),
            Key::End => (len - 1, -1),
            _ => return,
        };
        // Walk past disabled entries, bounded by len so we never loop
        // forever even if every tab is disabled.
        let mut target = start_idx;
        for _ in 0..len {
            if !entries[target].disabled {
                break;
            }
            target = if step > 0 {
                (target + 1) % len
            } else {
                (target + len - 1) % len
            };
        }
        if entries[target].disabled {
            return;
        }
        evt.prevent_default();
        let target_entry = &entries[target];
        ctx.onchange.call(target_entry.id.clone());
        let node = Rc::clone(&target_entry.mounted);
        spawn(async move {
            let _ = node.set_focus(true).await;
        });
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

#[cfg(test)]
mod tests {
    #![allow(non_snake_case, reason = "Dioxus components are PascalCase")]

    use dioxus::prelude::*;
    use dioxus_ssr::render;

    use super::super::tabs_root::TabsRoot;
    use super::{TabsList, TabsOrientation};

    /// Renders the canonical class shape, role, and orientation. The
    /// keyboard coordinator and registry-driven nav are exercised in
    /// the integration suite once TabButton lands.
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
    /// switching is integration-tested with TabButton.
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
}
