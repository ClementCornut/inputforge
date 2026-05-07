//! `ClickAwayListener`: fires a callback when a `mousedown` lands
//! outside a target element. Behaviour primitive only — renders
//! nothing of its own, just installs and tears down a document-level
//! event listener tied to the component lifecycle.
//!
//! Mirrors MUI's `ClickAwayListener`
//! (`mui-material/src/ClickAwayListener/ClickAwayListener.tsx`):
//! attach to `document`, check `event.composedPath().includes(node)`
//! (Shadow-DOM safe) with a `Node.contains` fallback, fire
//! `onClickAway` if outside. We use the leading event (`mousedown`)
//! per MUI's recommendation: the overlay dismisses BEFORE the outside
//! click reaches its target, so the same gesture closes the overlay
//! and activates the outside element.

use std::sync::atomic::{AtomicU64, Ordering};

use dioxus::prelude::*;

static LISTENER_ID: AtomicU64 = AtomicU64::new(0);

/// Fires `on_click_away` when a `mousedown` lands outside the
/// element identified by `target_id`. If `exclude_id` is provided,
/// clicks inside that element are ALSO treated as "inside" — used
/// to exclude the trigger button that opens the wrapped overlay,
/// preventing the open-and-immediately-close race a single
/// `mousedown` would otherwise produce.
///
/// Renders nothing of its own; returns `{children}` directly. The
/// listener is installed on mount via `document::eval` and removed
/// on unmount via `use_drop`.
#[component]
pub fn ClickAwayListener(
    target_id: String,
    #[props(default)] exclude_id: Option<String>,
    on_click_away: EventHandler<()>,
    children: Element,
) -> Element {
    let listener_id =
        use_hook(|| format!("__if_caw_{}", LISTENER_ID.fetch_add(1, Ordering::Relaxed)));
    let id_for_mount = listener_id.clone();
    let id_for_drop = listener_id.clone();
    let target_for_mount = target_id.clone();
    let exclude_for_mount = exclude_id.clone();

    use_effect(move || {
        let listener_id = id_for_mount.clone();
        let target_id = target_for_mount.clone();
        let exclude_id = exclude_for_mount.clone();
        let cb = on_click_away;
        spawn(async move {
            let exclude_js = exclude_id
                .as_ref()
                .map_or_else(|| "null".to_owned(), |s| format!("'{s}'"));
            let mut handle = document::eval(&format!(
                "const handler = (event) => {{\n\
                   const target = document.getElementById('{target_id}');\n\
                   if (!target) return;\n\
                   const path = event.composedPath ? event.composedPath() : null;\n\
                   const insideTarget = path ? path.includes(target) : target.contains(event.target);\n\
                   if (insideTarget) return;\n\
                   const excludeId = {exclude_js};\n\
                   if (excludeId) {{\n\
                     const exc = document.getElementById(excludeId);\n\
                     const insideExc = exc && (path ? path.includes(exc) : exc.contains(event.target));\n\
                     if (insideExc) return;\n\
                   }}\n\
                   dioxus.send('away');\n\
                 }};\n\
                 window['{listener_id}'] = handler;\n\
                 document.addEventListener('mousedown', handler);"
            ));
            while handle.recv::<String>().await.is_ok() {
                cb.call(());
            }
        });
    });

    use_drop(move || {
        let id = id_for_drop.clone();
        spawn(async move {
            let _ = document::eval(&format!(
                "const h = window['{id}'];\n\
                 if (h) {{\n\
                   document.removeEventListener('mousedown', h);\n\
                   delete window['{id}'];\n\
                 }}"
            ));
        });
    });

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
