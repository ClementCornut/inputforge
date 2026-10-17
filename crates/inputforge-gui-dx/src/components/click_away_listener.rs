//! `ClickAwayListener`: fires a callback when a `mousedown` lands
//! outside a target element. Behaviour primitive only — renders
//! nothing of its own; app bindings own the document listener lifecycle.
//!
//! Mirrors MUI's `ClickAwayListener`
//! (`mui-material/src/ClickAwayListener/ClickAwayListener.tsx`):
//! attach to `document`, check `event.composedPath().includes(node)`
//! (Shadow-DOM safe) with a `Node.contains` fallback, fire
//! `onClickAway` if outside. We use the leading event (`mousedown`)
//! per MUI's recommendation: the overlay dismisses BEFORE the outside
//! click reaches its target, so the same gesture closes the overlay
//! and activates the outside element.

use dioxus::prelude::*;

/// Fires `on_click_away` when a `mousedown` lands outside the
/// element identified by `target_id`. If `exclude_id` is provided,
/// clicks inside that element are ALSO treated as "inside" — used
/// to exclude the trigger button that opens the wrapped overlay,
/// preventing the open-and-immediately-close race a single
/// `mousedown` would otherwise produce.
///
/// Renders nothing of its own; returns `{children}` directly. The
/// app-supplied hook installs the listener and disposes it synchronously on unmount.
#[component]
pub fn ClickAwayListener(
    target_id: String,
    #[props(default)] exclude_id: Option<String>,
    on_click_away: EventHandler<()>,
    children: Element,
) -> Element {
    (super::behavior().overlays.click_away)(target_id, exclude_id, on_click_away);

    rsx! { {children} }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus_ssr::render;

    fn harness() -> Element {
        rsx! {
            ClickAwayListener {
                target_id: "wrapped".to_owned(),
                on_click_away: move |()| {},
                div { id: "wrapped", "child-content" }
            }
        }
    }

    #[test]
    fn renders_only_children_no_wrapper() {
        let mut vdom = VirtualDom::new(harness);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(html.contains("child-content"));
        assert!(html.contains("id=\"wrapped\""));
        // No JS leaks into rendered HTML; document::eval runs at runtime,
        // not via SSR markup.
        assert!(!html.contains("addEventListener"));
        assert!(!html.contains("dioxus.send"));
    }

    #[test]
    fn no_dom_wrapper_around_children() {
        // Ensure the component does not introduce its own div /
        // section / span that consumers would have to style around.
        fn h() -> Element {
            rsx! {
                ClickAwayListener {
                    target_id: "x".to_owned(),
                    on_click_away: move |()| {},
                    span { id: "only-child", "x" }
                }
            }
        }
        let mut vdom = VirtualDom::new(h);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        // The outermost element in the rendered html should be the
        // child <span>, not a wrapper div.
        let trimmed = html.trim();
        assert!(
            trimmed.starts_with("<span"),
            "expected children-only output, got: {trimmed}"
        );
    }
}
