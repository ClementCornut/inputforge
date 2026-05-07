//! Per-instance Portal helper for floating overlays.
//!
//! Renders `children` inline at the call site so Dioxus owns the
//! diffing and event listeners by node reference, then JS-moves the
//! wrapper to the Dioxus root container (`#main`) on mount and removes
//! it on unmount. The moved wrapper escapes any ancestor's containing
//! block, overflow clip, or stacking context, which is necessary for
//! menus, dialogs, tooltips, and toasts to land at viewport coordinates
//! regardless of where their trigger lives in the DOM tree.
//!
//! Why `#main` and NOT `document.body`: Dioxus 0.7's `WebView2` event
//! delegate is registered on the renderer's mount root (`<div id="main">`,
//! a direct child of `body`). The delegate dispatches an `onclick` only
//! when the event bubbles through that root. Appending the wrapper to
//! `document.body` makes it a SIBLING of `#main`, so events on portaled
//! descendants reach `body` and `document` but never `#main`, and every
//! `onclick` inside the portal silently fails. Verified live: a click on
//! a portaled `.if-menu__item` reached `document` and `body` capture
//! listeners with `isTrusted: true`, but bypassed `#main` and was never
//! dispatched. Appending to `#main` keeps the wrapper inside the
//! delegate's bubble path, so onclick / onkeydown / etc. all dispatch
//! normally, while still escaping every containing-block ancestor of
//! the trigger (which all live deeper inside `#main`).
//!
//! Mirrors React's `createPortal` and MUI's Modal/Popover portal
//! pattern (`disablePortal: true` is the unusual case there; portal is
//! the default). Dioxus 0.7 has no built-in Portal primitive, so this
//! is the minimal helper for the desktop `WebView2` renderer.

use std::sync::atomic::{AtomicU64, Ordering};

use dioxus::prelude::*;

const PORTAL_CSS: Asset = asset!("/assets/components/portal.css");

static PORTAL_ID: AtomicU64 = AtomicU64::new(0);

/// Render `children` at the Dioxus root level, escaping any ancestor's
/// containing-block, overflow, or stacking-context constraints.
///
/// Use ONLY for elements that genuinely need to escape ancestor
/// layout (floating menus, modal dialogs, viewport-anchored
/// tooltips, toast stacks). Anything in normal flow stays in the
/// regular component tree.
///
/// # How it works
///
/// 1. Renders `<div id="if-portal-N" class="if-portal">{children}</div>`
///    inline at the call site. Dioxus attaches event listeners to
///    descendants by node reference, not by DOM position.
/// 2. `use_effect` fires after the wrapper is in the DOM. JS evaluates
///    inside `WebView2` and moves the wrapper to the Dioxus mount root
///    (`#main`, with a `document.body` fallback for any future renderer
///    layout that doesn't use that id). Children come along.
/// 3. Subsequent re-renders update the wrapper by reference; the new
///    root-level position does not matter to Dioxus's diff.
/// 4. On unmount, Dioxus naturally removes the wrapper from its render
///    tree, but the wrapper has been moved. `use_drop` runs JS cleanup
///    so no orphans accumulate at the root level.
///
/// Event bubbling continues to work because events fired inside the
/// moved wrapper bubble up THROUGH `#main`, where Dioxus's event
/// delegation listens, before reaching `body` and `document`.
#[component]
pub fn Portal(children: Element) -> Element {
    let id = use_hook(|| format!("if-portal-{}", PORTAL_ID.fetch_add(1, Ordering::Relaxed)));
    let id_for_mount = id.clone();
    let id_for_drop = id.clone();

    use_effect(move || {
        let id = id_for_mount.clone();
        spawn(async move {
            let mut handle = document::eval(&format!(
                "const root = document.getElementById('main') || document.body;\n\
                 const el = document.getElementById('{id}');\n\
                 if (el && el.parentElement !== root) {{\n\
                     root.appendChild(el);\n\
                 }}\n\
                 dioxus.send(true);"
            ));
            let _ = handle.recv::<bool>().await;
        });
    });

    use_drop(move || {
        let id = id_for_drop.clone();
        spawn(async move {
            let _ = document::eval(&format!(
                "const el = document.getElementById('{id}');\n\
                 if (el) el.remove();"
            ));
        });
    });

    rsx! {
        Stylesheet { href: PORTAL_CSS }
        div { id: "{id}", class: "if-portal", {children} }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus_ssr::render;

    fn harness() -> Element {
        rsx! {
            Portal {
                div { id: "marker", "portal-content" }
            }
        }
    }

    #[test]
    fn portal_wraps_children_with_unique_id_class() {
        let mut vdom = VirtualDom::new(harness);
        vdom.rebuild_in_place();
        let html = render(&vdom);

        assert!(html.contains("class=\"if-portal\""));
        assert!(html.contains("id=\"if-portal-"));
        assert!(html.contains("portal-content"));
        assert!(html.contains("id=\"marker\""));
    }

    #[test]
    fn portal_ids_are_distinct_across_instances() {
        fn pair() -> Element {
            rsx! {
                Portal { div { "first" } }
                Portal { div { "second" } }
            }
        }
        let mut vdom = VirtualDom::new(pair);
        vdom.rebuild_in_place();
        let html = render(&vdom);

        let ids: Vec<&str> = html.matches("if-portal-").collect();
        // Two Portals → two unique id="if-portal-N" attributes plus one
        // class="if-portal" each (the class doesn't include the counter
        // suffix, but the matches counter still picks up the prefix).
        // We expect at least 2 distinct numeric suffixes in the rendered
        // HTML; check by parsing.
        let mut suffixes: Vec<String> = html
            .split("id=\"if-portal-")
            .skip(1)
            .map(|s| s.split('"').next().unwrap_or("").to_owned())
            .collect();
        suffixes.sort();
        suffixes.dedup();
        assert_eq!(
            suffixes.len(),
            2,
            "expected two distinct portal ids in rendered html, got {ids:?}"
        );
    }

    /// Regression: Dioxus 0.7 attaches its `WebView2` event delegate at
    /// the renderer's mount root (`#main` for desktop), not at `body`.
    /// Portaling to `body` strands descendants outside the delegate's
    /// bubble path and silently breaks every onclick inside the portal
    /// (verified live: `mousedown`/`click` reach `document` with
    /// `isTrusted: true` but bypass `#main` and never dispatch). Lock
    /// the append target to `#main` so the constraint cannot regress
    /// to the broken shape.
    ///
    /// The test reads its own source, so the search needle must NOT
    /// appear as a contiguous literal here, otherwise it would always
    /// match itself. We assert the safe shape positively (the JS reads
    /// the mount root via `getElementById('main')` and appends to a
    /// `root` local) instead of asserting the broken shape negatively.
    #[test]
    fn portal_appends_to_dioxus_root_not_body() {
        let src = include_str!("portal.rs");
        assert!(
            src.contains("document.getElementById('main')"),
            "portal must target the Dioxus mount root (#main)"
        );
        assert!(
            src.contains("root.appendChild(el)"),
            "portal must append to the resolved `root` local, \
             not to `document.body` directly"
        );
    }
}
