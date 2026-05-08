//! Tab leaf primitive. Renders the canonical `<button role="tab">`
//! with `if-tab` / `if-tab--active` class shape, the optional running
//! pip + sr-only sibling, focus-roving tabindex, and the full
//! per-tab ARIA + data-attribute surface `mode_tabs` needs.
//!
//! Reads the `TabsContext` published by `TabsRoot`: derives
//! `aria-selected` from `value`, fires `onchange` on click, registers
//! its `MountedData` ref in the shared registry on mount and removes
//! it on unmount via `use_drop`. The registry order is mount order,
//! which is what `TabsList`'s keyboard coordinator walks.
//!
//! Per-tab `onkeydown` and `oncontextmenu` props are intentionally
//! exposed: consumers like `mode_tabs` that bind Shift+F10, Delete, or a
//! right-click menu attach those handlers here. The user's handler
//! fires first (it sits on the button); if it calls `stop_propagation`,
//! the bubbling stops before `TabsList`'s wrapper-level handler runs.

use dioxus::prelude::*;

use super::merge_class;
use super::tabs_root::{TabRegistryEntry, TabsContext};

#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures (the macro suggests \
              shorthand-with-no-prop-name as a fix, which would erase the intent). \
              This is a macro-level artifact, not authored qualifications."
)]
pub fn TabButton(
    /// Stable id; identifies the tab in `onchange`, `focus_request`,
    /// and the registry. Distinct from the rendered DOM `id`, which
    /// defaults to `tab-{id}` but can be overridden via `dom_id`.
    id: String,
    /// Visible button text. Rendered after the optional running pip.
    label: String,
    /// Override the rendered DOM `id`. Used by `mode_tabs` to land the
    /// integer-derived `mode-tab-{idx}` shape that keeps JS-eval
    /// helpers safe against caller-typed mode names.
    #[props(default)]
    dom_id: Option<String>,
    /// Marks this tab as the runtime-live one. Renders a 6px
    /// `--color-live` pip before the label.
    #[props(default)]
    running: bool,
    /// Optional sr-only text rendered next to the running pip so AT
    /// users get the semantic that the visual dot alone cannot convey
    /// (e.g. "Engine running" in `mode_tabs`).
    #[props(default)]
    running_sr_label: Option<String>,
    /// Per-tab right-click handler. The platform menu is suppressed
    /// only if the consumer calls `evt.prevent_default()`.
    #[props(default)]
    oncontextmenu: Option<EventHandler<MouseEvent>>,
    /// Per-tab keyboard handler. Fires before `TabsList`'s wrapper
    /// coordinator; `evt.stop_propagation()` short-circuits the
    /// coordinator (`mode_tabs` uses this for Shift+F10 and Delete).
    #[props(default)]
    onkeydown: Option<EventHandler<KeyboardEvent>>,
    #[props(default)] aria_controls: Option<String>,
    #[props(default)] aria_haspopup: Option<String>,
    #[props(default)] aria_expanded: Option<bool>,
    #[props(default)] title: Option<String>,
    #[props(default)] data_mode: Option<String>,
    /// Per-tab disabled flag. Combined with the cluster-level
    /// `TabsRoot::disabled`; either being true marks this tab disabled.
    #[props(default)]
    disabled: bool,
    #[props(default)] extra_class: Option<String>,
) -> Element {
    let ctx = use_context::<TabsContext>();
    let is_active = *ctx.value.read() == id;
    let is_disabled = disabled || ctx.disabled;

    let dom_id_resolved = dom_id.clone().unwrap_or_else(|| format!("tab-{id}"));
    let variant_class = if is_active { "if-tab--active" } else { "" };
    let combined = merge_class("if-tab", variant_class, extra_class.as_deref());

    // Register the mount ref in the shared registry. ``mode_tabs`` swaps
    // a TabButton for `RenameInline` while a tab is being renamed; the
    // RenameInline does not register, so the registry shrinks for that
    // window and `TabsList`'s arrow nav skips the renaming slot for
    // free.
    let id_for_mount = id.clone();
    let mut registry = ctx.registry;
    let onmounted = move |evt: MountedEvent| {
        let entry = TabRegistryEntry {
            id: id_for_mount.clone(),
            disabled: is_disabled,
            mounted: evt.data(),
        };
        registry.write().push(entry);
    };

    // Deregister on unmount so stale `MountedData` refs (pointing at
    // detached DOM nodes) cannot poison subsequent focus moves.
    let id_for_drop = id.clone();
    use_drop(move || {
        registry.write().retain(|e| e.id != id_for_drop);
    });

    // Keep the registry's `disabled` flag in sync if the prop flips
    // mid-life (rare, but the contract is "registry reflects current
    // truth", not "mount-time snapshot").
    let id_for_sync = id.clone();
    use_effect(use_reactive!(|is_disabled| {
        let mut entries = registry.write();
        if let Some(entry) = entries.iter_mut().find(|e| e.id == id_for_sync) {
            entry.disabled = is_disabled;
        }
    }));

    let id_for_click = id.clone();
    let click_handler = move |_: MouseEvent| {
        if !is_disabled {
            ctx.onchange.call(id_for_click.clone());
        }
    };

    let onkeydown_user = onkeydown;
    let key_handler = move |evt: KeyboardEvent| {
        if let Some(h) = &onkeydown_user {
            h.call(evt);
        }
    };

    let oncontextmenu_user = oncontextmenu;
    let ctx_handler = move |evt: MouseEvent| {
        if let Some(h) = &oncontextmenu_user {
            h.call(evt);
        }
    };

    let aria_expanded_str = aria_expanded.map(|b| if b { "true" } else { "false" });

    rsx! {
        button {
            id: "{dom_id_resolved}",
            r#type: "button",
            class: "{combined}",
            role: "tab",
            "aria-selected": "{is_active}",
            "aria-controls": aria_controls.as_deref(),
            "aria-haspopup": aria_haspopup.as_deref(),
            "aria-expanded": aria_expanded_str,
            "data-mode": data_mode.as_deref(),
            title: title.as_deref(),
            tabindex: if is_active { "0" } else { "-1" },
            disabled: is_disabled,
            onclick: click_handler,
            oncontextmenu: ctx_handler,
            onkeydown: key_handler,
            onmounted,
            if running {
                span {
                    class: "if-tab__running-pip",
                    "aria-hidden": "true",
                }
                if let Some(label) = &running_sr_label {
                    span { class: "if-sr-only", "{label}" }
                }
            }
            "{label}"
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case, reason = "Dioxus components are PascalCase")]

    use dioxus::prelude::*;
    use dioxus_ssr::render;

    use super::super::tabs_list::TabsList;
    use super::super::tabs_root::TabsRoot;
    use super::TabButton;

    #[test]
    fn tab_button_renders_canonical_class_shape_for_active_and_inactive() {
        fn TestComponent() -> Element {
            rsx! {
                TabsRoot {
                    value: "second".to_owned(),
                    onchange: move |_: String| {},
                    TabsList {
                        TabButton { id: "first".to_owned(), label: "First".to_owned() }
                        TabButton { id: "second".to_owned(), label: "Second".to_owned() }
                    }
                }
            }
        }
        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        // Both buttons carry the base class.
        assert_eq!(html.matches("if-tab").count() >= 2, true, "{html}");
        // Only the active one carries the active modifier.
        assert_eq!(
            html.matches("if-tab--active").count(),
            1,
            "exactly one tab must carry --active: {html}",
        );
        // aria-selected reflects the context value.
        assert!(
            html.contains("aria-selected=\"true\""),
            "active tab missing aria-selected=true: {html}",
        );
        assert!(
            html.contains("aria-selected=\"false\""),
            "inactive tab missing aria-selected=false: {html}",
        );
    }

    #[test]
    fn tab_button_renders_running_pip_and_sr_only_label() {
        fn TestComponent() -> Element {
            rsx! {
                TabsRoot {
                    value: "default".to_owned(),
                    onchange: move |_: String| {},
                    TabsList {
                        TabButton {
                            id: "default".to_owned(),
                            label: "Default".to_owned(),
                            running: true,
                            running_sr_label: "Engine running".to_owned(),
                        }
                    }
                }
            }
        }
        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            html.contains("if-tab__running-pip"),
            "running pip element missing: {html}",
        );
        assert!(
            html.contains("if-sr-only") && html.contains("Engine running"),
            "sr-only label must render alongside the running pip: {html}",
        );
    }

    #[test]
    fn tab_button_omits_running_pip_and_sr_label_when_not_running() {
        fn TestComponent() -> Element {
            rsx! {
                TabsRoot {
                    value: "default".to_owned(),
                    onchange: move |_: String| {},
                    TabsList {
                        TabButton {
                            id: "default".to_owned(),
                            label: "Default".to_owned(),
                            running_sr_label: "Engine running".to_owned(),
                        }
                    }
                }
            }
        }
        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            !html.contains("if-tab__running-pip"),
            "running pip must be absent when running=false: {html}",
        );
        assert!(
            !html.contains("Engine running"),
            "sr-only label must NOT render when running=false even if label provided: {html}",
        );
    }

    #[test]
    fn tab_button_dom_id_defaults_to_tab_prefix_and_can_be_overridden() {
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
                            dom_id: "mode-tab-1".to_owned(),
                        }
                    }
                }
            }
        }
        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            html.contains("id=\"tab-a\""),
            "default dom_id must be `tab-{{id}}`: {html}",
        );
        assert!(
            html.contains("id=\"mode-tab-1\""),
            "dom_id override must reach the rendered button: {html}",
        );
    }

    #[test]
    fn tab_button_propagates_optional_aria_and_data_attrs() {
        fn TestComponent() -> Element {
            rsx! {
                TabsRoot {
                    value: "x".to_owned(),
                    onchange: move |_: String| {},
                    TabsList {
                        TabButton {
                            id: "x".to_owned(),
                            label: "X".to_owned(),
                            aria_controls: "panel-x".to_owned(),
                            aria_haspopup: "menu".to_owned(),
                            aria_expanded: true,
                            title: "X mode".to_owned(),
                            data_mode: "X".to_owned(),
                        }
                    }
                }
            }
        }
        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            html.contains("aria-controls=\"panel-x\""),
            "aria-controls must propagate: {html}",
        );
        assert!(
            html.contains("aria-haspopup=\"menu\""),
            "aria-haspopup must propagate: {html}",
        );
        assert!(
            html.contains("aria-expanded=\"true\""),
            "aria-expanded must serialize as `true`: {html}",
        );
        assert!(
            html.contains("title=\"X mode\""),
            "title must propagate: {html}",
        );
        assert!(
            html.contains("data-mode=\"X\""),
            "data-mode must propagate: {html}",
        );
    }
}
