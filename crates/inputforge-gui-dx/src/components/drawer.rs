//! `Drawer`: docked panel anchored to one of the four edges of its
//! container, optionally collapsing along the cross-axis.
//!
//! Modeled on MUI's `Drawer`
//! (`packages/mui-material/src/Drawer/Drawer.js`). Two of MUI's three
//! variants are implemented today: `Permanent` (always rendered at full
//! size) and `Persistent` (animates between collapsed and expanded
//! along the anchor's cross-axis, releasing its space when closed).
//! The `Temporary` variant (Modal + Portal + Backdrop + `FocusTrap`) is
//! reserved for a follow-up; the existing `Portal` and
//! `ClickAwayListener` primitives will compose into it without changes
//! here.
//!
//! Persistent diverges from MUI in one place: MUI keeps the docked
//! container at full size and translates the Paper off-screen via
//! transform, so the docked region reserves space even when closed.
//! That suits a navigation rail next to scrolling content; it does
//! NOT suit an in-panel disclosure surface, where the closed state
//! should give back its space to the rest of the panel. So our
//! Persistent collapses the docked wrapper itself on the cross-axis
//! (`max-height` for Top/Bottom, `max-width` for Left/Right). See
//! `assets/components/drawer.css` for the anchor-specific rules.

use dioxus::prelude::*;

/// Edge of the parent container the drawer is anchored to. The drawer
/// expands inward from this edge when open. `Bottom` matches the
/// snapshot drawer's surface in the Profiles panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DrawerAnchor {
    Top,
    Right,
    #[default]
    Bottom,
    Left,
}

impl DrawerAnchor {
    fn class(self) -> &'static str {
        match self {
            Self::Top => "if-drawer--anchor-top",
            Self::Right => "if-drawer--anchor-right",
            Self::Bottom => "if-drawer--anchor-bottom",
            Self::Left => "if-drawer--anchor-left",
        }
    }
}

/// Drawer behaviour mode. See module-level docs for variant semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DrawerVariant {
    /// Always rendered at full size, ignores `open`. Use for
    /// surfaces that must always be visible (inspector, fixed
    /// list pane).
    Permanent,
    /// Docked, collapses along the anchor's cross-axis when
    /// `open` is false. Released space goes back to the parent.
    #[default]
    Persistent,
}

impl DrawerVariant {
    fn class(self) -> &'static str {
        match self {
            Self::Permanent => "if-drawer--permanent",
            Self::Persistent => "if-drawer--persistent",
        }
    }
}

/// Docked panel anchored to one edge of its parent container.
///
/// # Examples
///
/// Persistent bottom drawer driven by a sibling toggle:
///
/// ```ignore
/// let mut drawer_open = use_signal(|| true);
/// rsx! {
///     button {
///         onclick: move |_| { let n = !*drawer_open.read(); drawer_open.set(n); },
///         "Toggle"
///     }
///     Drawer {
///         anchor: DrawerAnchor::Bottom,
///         variant: DrawerVariant::Persistent,
///         open: *drawer_open.read(),
///         div { "drawer body" }
///     }
/// }
/// ```
///
/// Permanent right drawer (always visible, no animation):
///
/// ```ignore
/// rsx! {
///     Drawer {
///         anchor: DrawerAnchor::Right,
///         variant: DrawerVariant::Permanent,
///         div { "always-visible inspector" }
///     }
/// }
/// ```
///
/// # Sizing
///
/// The open-state cross-axis size is set by the `--if-drawer-size` CSS
/// custom property on the docked wrapper or any ancestor. Defaults are
/// `50%` for Top/Bottom and `320px` for Left/Right. Set the property on
/// a wrapping element to size per surface (e.g. `.snapshot-drawer
/// { --if-drawer-size: 40%; }`).
#[component]
pub fn Drawer(
    /// Whether the drawer is expanded. Ignored for `Permanent`.
    #[props(default = true)]
    open: bool,
    #[props(default)] anchor: DrawerAnchor,
    #[props(default)] variant: DrawerVariant,
    /// Optional extra class on the docked wrapper. The Paper class is
    /// internal so consumers cannot reach into the slide chrome.
    #[props(default)]
    class: Option<String>,
    /// `aria-label` on the Paper region. Prefer `aria_labelledby` to a
    /// heading inside the drawer when one exists.
    #[props(default)]
    aria_label: Option<String>,
    /// `aria-labelledby` on the Paper region. Should reference the id
    /// of a heading or label sibling to the drawer.
    #[props(default)]
    aria_labelledby: Option<String>,
    children: Element,
) -> Element {
    let mut combined = String::with_capacity(64);
    combined.push_str("if-drawer if-drawer--docked ");
    combined.push_str(anchor.class());
    combined.push(' ');
    combined.push_str(variant.class());
    if matches!(variant, DrawerVariant::Persistent) {
        combined.push(' ');
        combined.push_str(if open {
            "if-drawer--open"
        } else {
            "if-drawer--closed"
        });
    }
    if let Some(extra) = class.as_deref()
        && !extra.is_empty()
    {
        combined.push(' ');
        combined.push_str(extra);
    }

    rsx! {
        div { class: "{combined}",
            div {
                class: "if-drawer__paper",
                role: "region",
                aria_label: aria_label.clone(),
                aria_labelledby: aria_labelledby.clone(),
                {children}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus_ssr::render;

    fn render_with(builder: fn() -> Element) -> String {
        let mut vdom = VirtualDom::new(builder);
        vdom.rebuild_in_place();
        render(&vdom)
    }

    #[test]
    fn permanent_renders_full_size_without_state_class() {
        fn h() -> Element {
            rsx! {
                Drawer {
                    variant: DrawerVariant::Permanent,
                    anchor: DrawerAnchor::Right,
                    div { "permanent-body" }
                }
            }
        }
        let html = render_with(h);
        assert!(html.contains("if-drawer--permanent"));
        assert!(html.contains("if-drawer--anchor-right"));
        assert!(!html.contains("if-drawer--open"));
        assert!(!html.contains("if-drawer--closed"));
        assert!(html.contains("permanent-body"));
    }

    #[test]
    fn persistent_open_carries_open_class_and_mounts_body() {
        fn h() -> Element {
            rsx! {
                Drawer {
                    variant: DrawerVariant::Persistent,
                    anchor: DrawerAnchor::Bottom,
                    open: true,
                    div { "persistent-body" }
                }
            }
        }
        let html = render_with(h);
        assert!(html.contains("if-drawer--persistent"));
        assert!(html.contains("if-drawer--open"));
        assert!(!html.contains("if-drawer--closed"));
        assert!(html.contains("persistent-body"));
    }

    #[test]
    fn persistent_closed_carries_closed_class_and_keeps_body_mounted() {
        fn h() -> Element {
            rsx! {
                Drawer {
                    variant: DrawerVariant::Persistent,
                    anchor: DrawerAnchor::Bottom,
                    open: false,
                    div { "still-mounted-body" }
                }
            }
        }
        let html = render_with(h);
        assert!(html.contains("if-drawer--closed"));
        assert!(!html.contains("if-drawer--open"));
        // Body must remain in the DOM when closed: the Paper is hidden
        // by the wrapper's max-* collapse, not by removing the subtree.
        // Keeping it mounted preserves scroll position, focus history,
        // and avoids re-mount cost on every toggle.
        assert!(html.contains("still-mounted-body"));
    }

    #[test]
    fn all_four_anchors_emit_their_class() {
        fn top() -> Element {
            rsx! { Drawer { anchor: DrawerAnchor::Top, div {} } }
        }
        fn right() -> Element {
            rsx! { Drawer { anchor: DrawerAnchor::Right, div {} } }
        }
        fn bottom() -> Element {
            rsx! { Drawer { anchor: DrawerAnchor::Bottom, div {} } }
        }
        fn left() -> Element {
            rsx! { Drawer { anchor: DrawerAnchor::Left, div {} } }
        }
        assert!(render_with(top).contains("if-drawer--anchor-top"));
        assert!(render_with(right).contains("if-drawer--anchor-right"));
        assert!(render_with(bottom).contains("if-drawer--anchor-bottom"));
        assert!(render_with(left).contains("if-drawer--anchor-left"));
    }

    #[test]
    fn paper_carries_role_region() {
        fn h() -> Element {
            rsx! { Drawer { div {} } }
        }
        let html = render_with(h);
        assert!(html.contains("role=\"region\""));
    }

    #[test]
    fn aria_label_and_labelledby_pass_through_to_paper() {
        fn h() -> Element {
            rsx! {
                Drawer {
                    aria_label: "Snapshots",
                    aria_labelledby: "snapshot-bar-title",
                    div {}
                }
            }
        }
        let html = render_with(h);
        assert!(html.contains("aria-label=\"Snapshots\""));
        assert!(html.contains("aria-labelledby=\"snapshot-bar-title\""));
    }

    #[test]
    fn caller_class_appends_to_combined() {
        fn h() -> Element {
            rsx! {
                Drawer { class: "snapshot-drawer__body".to_owned(), div {} }
            }
        }
        let html = render_with(h);
        assert!(html.contains("if-drawer"));
        assert!(html.contains("snapshot-drawer__body"));
    }
}
