//! Headless tabs root primitive. Owns the active value, the optional
//! imperative-focus channel, and the registry of mounted `TabButton`
//! refs. Renders no DOM of its own (`{children}` only); siblings like a
//! trailing `+` affordance ride alongside `TabsList` under the same
//! parent.
//!
//! Compound shape parallels MUI Lab's `TabContext` and Base UI's
//! `Tabs.Root`: parent owns coordination state, leaves consume it via
//! context. The `Tabs` items-array facade in `tabs.rs` wraps this plus
//! `TabsList` + `TabButton` for simple consumers.

use std::rc::Rc;

use dioxus::prelude::*;

/// One entry in the tablist's registry. Inserted by `TabButton` on
/// mount; removed by `TabButton`'s `use_drop`. Order is mount order,
/// which equals render order for stable consumers and feeds keyboard
/// navigation in `TabsList`.
#[derive(Clone)]
#[allow(
    dead_code,
    reason = "Fields read by TabsList + TabButton in subsequent commits of this migration"
)]
pub(crate) struct TabRegistryEntry {
    pub id: String,
    pub disabled: bool,
    pub mounted: Rc<MountedData>,
}

/// Context published by `TabsRoot`. Consumed by `TabsList` (keyboard
/// coordinator + `focus_request` watcher) and `TabButton` (renders
/// aria-selected, registers its mount ref, calls `onchange` on click).
#[derive(Clone, Copy)]
#[allow(
    dead_code,
    reason = "Fields consumed by TabsList + TabButton in subsequent commits of this migration"
)]
pub(crate) struct TabsContext {
    pub value: ReadSignal<String>,
    pub onchange: EventHandler<String>,
    pub disabled: bool,
    /// Caller-owned signal: setting it to `Some(id)` asks `TabsList` to
    /// move focus to that tab. `TabsList` clears the signal once it has
    /// focused (or skipped) the request, so it is safe to set repeatedly.
    pub focus_request: Option<Signal<Option<String>>>,
    pub registry: Signal<Vec<TabRegistryEntry>>,
}

/// Root of the decomposed tabs primitive. Provides `TabsContext` and
/// renders only its children (no wrapper DOM); the tablist semantics
/// live in `TabsList`. Siblings of `TabsList` rendered as additional
/// children, e.g. an "Add tab" affordance, sit OUTSIDE the tablist
/// for honest screen-reader tab counts.
#[component]
pub fn TabsRoot(
    /// Stable id of the active tab. Accepts a literal `String` or a
    /// signal; Dioxus auto-wraps the literal so `TabButton` can react
    /// to changes pushed by the caller.
    value: ReadSignal<String>,
    /// Fires with the new tab id when a click or keyboard navigation
    /// activates a tab.
    onchange: EventHandler<String>,
    /// Optional imperative focus channel. Caller sets this signal to a
    /// tab id; `TabsList` watches the signal, calls `set_focus(true)`
    /// on the matching `TabButton`'s mount ref, then clears the signal.
    /// Replaces hand-rolled `use_effect` chains in consumers that need
    /// to focus a newly-mounted tab (e.g. `mode_tabs` after `AddMode`).
    #[props(default)]
    focus_request: Option<Signal<Option<String>>>,
    /// Disables every descendant `TabButton`'s click and keyboard
    /// activation. Per-tab disable lives on `TabButton::disabled`.
    #[props(default)]
    disabled: bool,
    children: Element,
) -> Element {
    let registry = use_signal(Vec::<TabRegistryEntry>::new);
    let ctx = TabsContext {
        value,
        onchange,
        disabled,
        focus_request,
        registry,
    };
    use_context_provider(|| ctx);
    rsx! { {children} }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case, reason = "Dioxus components are PascalCase")]

    use dioxus::prelude::*;
    use dioxus_ssr::render;

    use super::{TabsContext, TabsRoot};

    /// TabsRoot publishes a TabsContext that descendants can consume.
    /// The publish-by-context contract is what TabsList and TabButton
    /// will rely on; lock it here so a future API change cannot drop
    /// the context provider silently.
    #[test]
    fn tabs_root_publishes_context_to_descendants() {
        fn Probe() -> Element {
            // Reads the context, panics with a recognisable token if it
            // is not provided, and otherwise emits a marker class.
            let ctx = use_context::<TabsContext>();
            let active = ctx.value.read().clone();
            rsx! { span { class: "probe-active-{active}" } }
        }
        fn TestComponent() -> Element {
            rsx! {
                TabsRoot {
                    value: "second".to_owned(),
                    onchange: move |_: String| {},
                    Probe {}
                }
            }
        }
        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            html.contains("probe-active-second"),
            "TabsRoot must publish its `value` via TabsContext: {html}",
        );
    }

    /// TabsRoot itself must NOT render any wrapper DOM. The compound
    /// shape relies on TabsList being the wrapper; injecting a
    /// TabsRoot-level div would break the sibling-of-tablist pattern
    /// callers depend on (e.g. mode_tabs's trailing `+` button).
    #[test]
    fn tabs_root_renders_only_children_with_no_wrapper() {
        fn TestComponent() -> Element {
            rsx! {
                TabsRoot {
                    value: "x".to_owned(),
                    onchange: move |_: String| {},
                    span { class: "child-marker", "child" }
                }
            }
        }
        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        let trimmed = html.trim();
        assert!(
            trimmed.starts_with("<span"),
            "TabsRoot must render children directly, not wrap them: {html}",
        );
        assert!(
            trimmed.contains("child-marker"),
            "child marker missing: {html}",
        );
    }
}
