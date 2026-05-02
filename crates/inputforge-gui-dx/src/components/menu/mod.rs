use std::sync::atomic::{AtomicU64, Ordering};

use dioxus::prelude::*;

use super::merge_class;

pub(crate) mod focus_walker;

pub(crate) use focus_walker::{FocusAction, focus_menu_item};

/// Per-MenuRoot ID counter. Used to give each menu instance a stable DOM id
/// so the trigger's `aria-controls` and the in-JS focus walker can address
/// the correct items wrapper.
static MENU_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Reason a menu was closed. Trigger-attached menus (`MenuRoot`) discard
/// this; cursor-anchored menus (`AnchoredMenu`) surface it through `on_close`
/// so the parent can decide whether to re-focus the originating element.
///
/// `Escape` and `ClickOutside` mean the user dismissed without picking
/// anything; the parent typically re-focuses the originating trigger.
/// `Tab` means the user pressed Tab to leave the menu; the parent must NOT
/// re-focus the trigger because the browser's natural Tab traversal is
/// moving focus to the next element. `ItemActivated` fires after a
/// `MenuItem` click; the parent's behaviour is item-specific (often
/// re-focus the trigger as a default landing spot before the activated
/// item's own follow-on focus takes over).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseReason {
    Escape,
    ClickOutside,
    Tab,
    ItemActivated,
}

/// Shared open-state context for menu compound. Both `MenuRoot` and
/// `AnchoredMenu` install one of these so `MenuItem` works under either.
#[derive(Clone, Copy)]
struct MenuState {
    /// Open-state signal. `MenuRoot` owns this directly; `AnchoredMenu`
    /// mirrors its prop into here so `MenuItems`'s hidden+focus logic
    /// keeps reading from one place.
    open: Signal<bool>,
    /// Stable DOM id for the items wrapper.
    menu_id: Signal<String>,
    /// Stable DOM id for the trigger button. `MenuItems` reads this id at
    /// open time to call `getBoundingClientRect()` on the trigger and
    /// position the floating list relative to the viewport (so the menu
    /// does not extend any ancestor's overflow box and is unaffected by
    /// transformed containers). `AnchoredMenu` does not consume this; its
    /// own `MenuAnchor` prop already supplies viewport coordinates.
    trigger_id: Signal<String>,
    /// Close dispatcher. `MenuRoot` provides one that flips `open` to
    /// false and discards the reason; `AnchoredMenu` provides one that
    /// fires its `on_close` handler with the reason.
    close: Callback<CloseReason>,
}

/// Where the dropdown attaches to its trigger horizontally. `Start` = left edge,
/// `Center` = under the trigger's centerline, `End` = right edge. `Start` matches
/// the historical default and is the right pick for small triggers (icon button,
/// label-and-caret); `Center` is the right pick for full-width triggers like the
/// `AddPalette` `+` slot, where left-anchoring would float the menu off the trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Anchor {
    #[default]
    Start,
    Center,
    End,
}

/// Anchor coordinates for `AnchoredMenu`. Values are page-space pixels
/// (the same coordinate system as `MouseEvent::page_coordinates`). The
/// menu renders with `position: fixed` at these coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuAnchor {
    pub x: f64,
    pub y: f64,
}

/// Final viewport-relative placement of a `MenuItems` floating list,
/// computed in pass 2 of the popper after the menu's own dimensions are
/// known. Values feed straight into a `position: fixed` inline style.
#[derive(Debug, Clone, Copy, PartialEq)]
struct MenuPlacement {
    left: f64,
    top: f64,
}

/// Padding (in CSS px) the popper leaves between the floating list and
/// the viewport edge when the trigger sits close to a side. Matches the
/// 4px gap used between the trigger and the menu vertically; chosen so a
/// clamped menu never sits flush against the viewport edge.
const VIEWPORT_PADDING: f64 = 4.0;

/// Vertical offset (in CSS px) between the trigger's edge and the menu
/// when the menu opens below or above. Mirrors the historical
/// `top: calc(100% + 4px)` of the pre-popper `.if-menu__items` wrapper.
const TRIGGER_GAP: f64 = 4.0;

/// Trigger bounding rect, viewport-relative pixels.
#[derive(Debug, Clone, Copy)]
struct TriggerRect {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

/// Floating menu's measured size (CSS px).
#[derive(Debug, Clone, Copy)]
struct MenuSize {
    width: f64,
    height: f64,
}

/// Viewport size in CSS px (`window.innerWidth/innerHeight`).
#[derive(Debug, Clone, Copy)]
struct Viewport {
    width: f64,
    height: f64,
}

/// Compute the final viewport-relative placement of a floating menu list.
/// Implements a small flip-and-clamp policy: prefer "below the trigger";
/// flip to "above" if below would clip and above fits; else clamp inside
/// the viewport.
fn compute_placement(
    trigger: TriggerRect,
    menu: MenuSize,
    viewport: Viewport,
    anchor: Anchor,
) -> MenuPlacement {
    // Horizontal: anchor decides the desired left edge of the menu;
    // clamp to keep the menu fully inside the viewport (with padding).
    let desired_left = match anchor {
        Anchor::Start => trigger.left,
        Anchor::Center => f64::midpoint(trigger.left, trigger.right) - menu.width / 2.0,
        Anchor::End => trigger.right - menu.width,
    };
    let max_left = (viewport.width - menu.width - VIEWPORT_PADDING).max(VIEWPORT_PADDING);
    let left = desired_left.clamp(VIEWPORT_PADDING, max_left);

    // Vertical: prefer below (`trigger.bottom + gap`); flip above if
    // below clips and above fits; else clamp.
    let below_top = trigger.bottom + TRIGGER_GAP;
    let above_top = trigger.top - menu.height - TRIGGER_GAP;
    let fits_below = below_top + menu.height <= viewport.height - VIEWPORT_PADDING;
    let fits_above = above_top >= VIEWPORT_PADDING;
    let top = if fits_below {
        below_top
    } else if fits_above {
        above_top
    } else {
        // Neither side fits cleanly. Pick the side with more room and
        // clamp to keep as much of the menu in the viewport as possible.
        let space_below = viewport.height - trigger.bottom;
        let space_above = trigger.top;
        if space_below >= space_above {
            (viewport.height - menu.height - VIEWPORT_PADDING).max(VIEWPORT_PADDING)
        } else {
            VIEWPORT_PADDING
        }
    };

    MenuPlacement { left, top }
}

#[component]
pub fn MenuRoot(
    /// Class extension for the OUTER wrapper (`.if-menu`). Use for layout-flow
    /// modifiers like `if-menu--block` (which flips the wrapper from
    /// `inline-flex` to `block`).
    #[props(default)]
    class: Option<String>,
    children: Element,
) -> Element {
    let open = use_signal(|| false);
    let menu_id = use_signal(|| {
        format!(
            "if-menu-{}",
            MENU_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
        )
    });
    // Derive a JS-string-safe trigger id from the same counter as menu_id,
    // so `MenuItems` can call `document.getElementById(trigger_id)` to
    // measure the trigger's viewport rect at open time.
    let trigger_id = use_signal(|| format!("{}-trigger", menu_id.read()));
    let close = use_callback(move |_reason: CloseReason| {
        let mut o = open;
        o.set(false);
    });
    let state = MenuState {
        open,
        menu_id,
        trigger_id,
        close,
    };
    use_context_provider(|| state);

    let combined = merge_class("if-menu", "", class.as_deref());
    rsx! { div { class: "{combined}", {children} } }
}

#[component]
pub fn MenuTrigger(
    #[props(default)] class: Option<String>,
    /// When `true`, the `if-menu__trigger` base class is omitted, so the
    /// caller's `class` is the only surface styling. Use for triggers that
    /// already carry a non-trivial visual treatment (e.g. `AddPalette`'s
    /// dashed-violet "next slot"). The structural attributes
    /// (`aria-haspopup`, `aria-expanded`, `aria-controls`) are unaffected.
    #[props(default)]
    unstyled: bool,
    /// Accessible name for icon-only triggers. When `Some`, an `aria-label`
    /// attribute is emitted; when `None`, the attribute is omitted entirely
    /// (Dioxus 0.7 skips `Option<String>` attribute values that are `None`).
    /// Required for any trigger whose visible content is an icon with no
    /// adjacent text, per WCAG 2.1 SC 4.1.2 (Name, Role, Value).
    #[props(default)]
    aria_label: Option<String>,
    children: Element,
) -> Element {
    let mut state = use_context::<MenuState>();
    let combined = if unstyled {
        class.as_deref().unwrap_or("").to_owned()
    } else {
        merge_class("if-menu__trigger", "", class.as_deref())
    };
    let menu_id = state.menu_id.read().clone();
    let trigger_id = state.trigger_id.read().clone();
    let onclick = move |_| {
        let now = !*state.open.read();
        state.open.set(now);
    };
    rsx! {
        button {
            class: "{combined}",
            id: "{trigger_id}",
            onclick,
            "aria-haspopup": "true",
            "aria-expanded": "{state.open.read()}",
            "aria-controls": "{menu_id}",
            "aria-label": aria_label,
            {children}
        }
    }
}

#[component]
#[expect(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures."
)]
pub fn MenuItems(
    /// Class extension for the floating list surface (`.if-menu__list`).
    /// Lands directly on the visible chrome layer; consumers can customise
    /// the list by writing rules against `.your-class` (which is now a
    /// peer of `.if-menu__list` on the same element). This differs from
    /// pre-popper `MenuItems`, which routed `class` to a positioned
    /// `.if-menu__items` wrapper that no longer exists; consumer CSS
    /// previously written as `.your-class .if-menu__list { ... }` should
    /// be flattened to `.your-class { ... }`.
    #[props(default)]
    class: Option<String>,
    /// Horizontal alignment of the floating list relative to its trigger.
    /// `Start` aligns the list's left edge with the trigger's left edge;
    /// `Center` centres the list under the trigger's horizontal midpoint;
    /// `End` aligns the list's right edge with the trigger's right edge.
    /// Alignment is realised by `compute_placement`, which adjusts the
    /// desired `left` coordinate before clamping to the viewport. No inline
    /// `transform` is emitted; the list's inline style contains only
    /// `position`, `left`, `top`, `visibility`, and `z-index`.
    #[props(default)]
    anchor: Anchor,
    children: Element,
) -> Element {
    let state = use_context::<MenuState>();
    let mut open_signal = state.open;
    let menu_id = state.menu_id.read().clone();
    let trigger_id = state.trigger_id.read().clone();

    // Two-pass popper placement. Pass 1: render the menu invisibly so it
    // takes layout space and we can read its size. Pass 2: spawn an eval
    // that measures both rects + viewport, compute optimal placement
    // (flip-above-on-overflow, clamp horizontally), commit it back into
    // this signal. While `placement` is `None` the menu renders with
    // `visibility: hidden` so the user does not see a flash at the
    // (0, 0) placeholder coordinates.
    let mut placement: Signal<Option<MenuPlacement>> = use_signal(|| None);

    // Effect 1: drive the placement signal off `open`. On open=true,
    // schedule the pass-2 measurement. On open=false, clear placement
    // so the next open starts from pass 1 again (otherwise a stale
    // placement would briefly flash at the previous coords).
    let menu_id_for_eval = menu_id.clone();
    let trigger_id_for_eval = trigger_id.clone();
    let anchor_for_eval = anchor;
    use_effect(move || {
        let is_open = *open_signal.read();
        if !is_open {
            placement.set(None);
            return;
        }
        let menu_id_owned = menu_id_for_eval.clone();
        let trigger_id_owned = trigger_id_for_eval.clone();
        spawn(async move {
            // Both rects + viewport size in one round-trip. The menu is
            // already in the DOM (pass-1 hidden render committed before
            // this effect ran), so its size is measurable. Eight numbers:
            // trigger left/top/right/bottom, menu width/height, viewport
            // width/height.
            let mut handle = document::eval(&format!(
                "var t = document.getElementById('{trigger_id_owned}');\n\
                 var m = document.getElementById('{menu_id_owned}');\n\
                 if (!t || !m) {{ dioxus.send([0, 0, 0, 0, 0, 0, 0, 0]); return; }}\n\
                 var tr = t.getBoundingClientRect();\n\
                 var mr = m.getBoundingClientRect();\n\
                 dioxus.send([tr.left, tr.top, tr.right, tr.bottom, \
                              mr.width, mr.height, \
                              window.innerWidth, window.innerHeight]);"
            ));
            if let Ok(value) = handle.recv::<[f64; 8]>().await {
                let [t_left, t_top, t_right, t_bottom, m_w, m_h, vp_w, vp_h] = value;
                // Guard against the trigger/menu-not-found path: width
                // and height of zero mean the eval found neither. Skip
                // the placement update so the menu stays hidden rather
                // than committing to a degenerate (0, 0) location.
                if m_w > 0.0 && m_h > 0.0 {
                    let p = compute_placement(
                        TriggerRect {
                            left: t_left,
                            top: t_top,
                            right: t_right,
                            bottom: t_bottom,
                        },
                        MenuSize {
                            width: m_w,
                            height: m_h,
                        },
                        Viewport {
                            width: vp_w,
                            height: vp_h,
                        },
                        anchor_for_eval,
                    );
                    placement.set(Some(p));
                }
            }
        });
    });

    let close = state.close;
    let target_id_for_keydown = menu_id.clone();
    let onkeydown = move |evt: KeyboardEvent| {
        let action = match evt.key() {
            Key::Escape => {
                close.call(CloseReason::Escape);
                return;
            }
            Key::Tab => {
                // Do NOT prevent_default; let the browser advance focus to
                // the next focusable element. CloseReason::Tab signals the
                // parent's intent (irrelevant for MenuRoot which discards
                // the reason, but keeps the contract consistent with
                // AnchoredMenu, where Tab close skips trigger re-focus).
                close.call(CloseReason::Tab);
                return;
            }
            Key::ArrowDown => FocusAction::Next,
            Key::ArrowUp => FocusAction::Prev,
            Key::Home => FocusAction::First,
            Key::End => FocusAction::Last,
            _ => return,
        };
        focus_menu_item(&target_id_for_keydown, action);
    };
    let backdrop_onclick = move |_| {
        open_signal.set(false);
    };

    // Auto-focus the first menuitem once the menu is mounted with a
    // committed placement. Tracks both `open` and `placement` so it
    // fires exactly once per open, on the placement None -> Some
    // transition that accompanies the menu becoming visible.
    let target_id_for_focus = menu_id.clone();
    use_effect(move || {
        let is_open = *open_signal.read();
        let has_placement = placement.read().is_some();
        if is_open && has_placement {
            focus_menu_item(&target_id_for_focus, FocusAction::First);
        }
    });

    let is_open = *open_signal.read();
    if !is_open {
        return rsx! {};
    }

    // Pass-1 vs pass-2 inline style. Pass 1 keeps the menu invisible at
    // (0, 0) so the eval can measure it without the user seeing a flash.
    // Pass 2 commits the computed placement and explicitly resets
    // `visibility: visible`. The explicit reset matters: Dioxus 0.7
    // diffs the `style` attribute by individual CSS properties, so a
    // pass-2 string that omitted `visibility` would leave pass-1's
    // `visibility: hidden` applied and the menu would never appear.
    let style = match *placement.read() {
        None => "position: fixed; left: 0; top: 0; visibility: hidden; z-index: 1001;".to_owned(),
        Some(p) => format!(
            "position: fixed; left: {left}px; top: {top}px; visibility: visible; z-index: 1001;",
            left = p.left,
            top = p.top,
        ),
    };

    let combined = merge_class("if-menu__list", "", class.as_deref());
    rsx! {
        // Backdrop: full-viewport pointer-event sink at z-index 1000.
        // Renders BEFORE the list in document order so the list paints on
        // top within the same z-index layer; any click outside the list
        // hits the backdrop and closes the menu.
        div {
            class: "if-menu__backdrop",
            "aria-hidden": "true",
            onclick: backdrop_onclick,
        }
        div {
            class: "{combined}",
            id: "{menu_id}",
            role: "menu",
            tabindex: "-1",
            style: "{style}",
            onkeydown,
            {children}
        }
    }
}

#[component]
pub fn MenuItem(
    onclick: Option<EventHandler<MouseEvent>>,
    #[props(default)] disabled: bool,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let state = use_context::<MenuState>();
    let combined = merge_class("if-menu__item", "", class.as_deref());
    let handler = onclick;
    let close = state.close;
    let onclick = move |evt: MouseEvent| {
        // Synthetic clicks bypass the HTML `disabled` attribute, so guard
        // explicitly: a disabled item must neither invoke its handler nor
        // close the surrounding menu.
        if disabled {
            return;
        }
        if let Some(h) = &handler {
            h.call(evt);
        }
        close.call(CloseReason::ItemActivated);
    };
    rsx! {
        button {
            class: "{combined}",
            role: "menuitem",
            disabled,
            "aria-disabled": "{disabled}",
            onclick,
            {children}
        }
    }
}

/// Cursor-anchored menu (right-click style). The parent owns an
/// `Option<MenuAnchor>` signal: `None` = closed, `Some(coords)` = open at
/// those coordinates. `on_close` fires whenever the menu wants to close
/// (Escape, click-outside, Tab, item-activated); the parent decides what
/// to do (typically: clear its anchor signal to None, possibly re-focus
/// the originating element based on the `CloseReason`).
///
/// Inside, render `MenuItem`s as children; they auto-close via the same
/// `MenuState` mechanism `MenuRoot` uses. The wrapper handles backdrop,
/// keyboard navigation (Arrow keys, Home, End), Escape, Tab, and
/// auto-focuses the first non-disabled item on open. Space and Enter
/// activation on items works via native `<button>` semantics (each
/// `MenuItem` is a `<button>`); the keydown handler does not need to
/// handle them explicitly.
///
/// # Mount contract (auto-focus)
///
/// The parent MUST mount `AnchoredMenu` only when `open` is `Some(_)`
/// and unmount it when `open` is `None`. The canonical pattern is a
/// `let Some(coords) = anchor_signal.read().as_ref() else { return rsx! {}; };`
/// gate at the parent that returns empty rsx when the anchor is `None`,
/// so each open is a fresh mount. Auto-focus on open is driven by that
/// mount transition: the internal `open_signal` is `true` for the entire
/// lifetime of one mount, so the focus `use_effect` fires exactly once
/// per mount. A consumer that mounts `AnchoredMenu` unconditionally and
/// toggles its `open` prop between `None` and `Some` across renders is
/// hook-order safe (hooks are still allocated before the early return),
/// but auto-focus will only fire on the first open; subsequent re-opens
/// will not re-focus the first item because `open_signal` never toggles.
/// Tasks 4 to 6 consumers (`AddPalette`, `StageActionsMenu`,
/// `ModeTabContextMenu`) all mount on demand under a parent
/// `let Some(...)` gate and so satisfy the contract.
///
/// `aria_labelledby` is the DOM id of the element that named this menu
/// (typically the originating right-click target). When `Some`, written
/// to `aria-labelledby`; when `None`, the attribute is omitted entirely
/// (an empty `aria-labelledby` would point to a nonexistent element and
/// is invalid ARIA).
#[component]
#[expect(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures."
)]
pub fn AnchoredMenu(
    /// Anchor coordinates and open-state, fused into one Option so an
    /// open-with-no-coords state is unrepresentable.
    open: Option<MenuAnchor>,
    /// Fires when the menu wants to close, with the reason. The parent
    /// must clear its anchor signal in response; otherwise the menu
    /// stays rendered.
    on_close: EventHandler<CloseReason>,
    /// Optional id of the element that names this menu (written to
    /// `aria-labelledby`). Pass the originating trigger's DOM id.
    #[props(default)]
    aria_labelledby: Option<String>,
    /// Class extension for the inner LIST surface (`.if-menu__list`).
    /// NOTE: this is the visible chrome layer, NOT a wrapper. `AnchoredMenu`
    /// has no positioned outer wrapper because it applies `position: fixed`
    /// directly on the list. This matches `MenuItems.class` (which also
    /// lands directly on `.if-menu__list` post-popper) and differs from
    /// `MenuRoot.class` (which targets the outer `.if-menu` wrapper). If
    /// you need wrapper-level styling, lift it to a parent component.
    #[props(default)]
    class: Option<String>,
    children: Element,
) -> Element {
    // Hooks must run in the same order on every render, so allocate them
    // BEFORE the early-return below. This keeps hook-order safety
    // independent of the parent's mount strategy: even if a future
    // consumer mounted AnchoredMenu unconditionally and toggled `open`
    // between None / Some across renders, the hook count would stay
    // stable and not trip a hook-order panic. Note that hook-order
    // safety is the only thing this ordering buys us: auto-focus on
    // open is a separate contract, documented above, that REQUIRES the
    // mount-on-open pattern (the always-true `open_signal` below never
    // toggles, so `use_effect` would not re-fire on re-opens of an
    // unconditionally-mounted instance).

    // Allocate a stable menu id for this instance. Using a counter (not
    // anchor coordinates) so the id is consistent across re-renders even
    // if the anchor moves (the menu can be re-positioned without losing
    // its identity for ARIA / focus walking).
    let menu_id = use_signal(|| {
        format!(
            "if-menu-{}",
            MENU_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
        )
    });

    // Mirror open-state into a Signal so MenuItem (which reads
    // MenuState.open via context) sees a live signal. Stays `true` for
    // the lifetime of this AnchoredMenu mount; AnchoredMenu's own
    // visibility is gated by the parent's anchor signal (the
    // `let Some(coords) = open` early return below), not by this Signal.
    let open_signal = use_signal(|| true);
    // AnchoredMenu does not consume `MenuState.trigger_id` (its anchor
    // arrives through the `open: Option<MenuAnchor>` prop), but the
    // struct demands the field. Install an empty placeholder so child
    // `MenuItem`s see a well-formed context.
    let trigger_id = use_signal(String::new);
    let close_handler = on_close;
    let close = use_callback(move |reason: CloseReason| {
        close_handler.call(reason);
    });
    let state = MenuState {
        open: open_signal,
        menu_id,
        trigger_id,
        close,
    };
    use_context_provider(|| state);

    let menu_id_str_for_focus = menu_id.read().clone();
    use_effect(move || {
        if *open_signal.read() {
            focus_menu_item(&menu_id_str_for_focus, FocusAction::First);
        }
    });

    let Some(coords) = open else {
        return rsx! {};
    };

    let menu_id_str = menu_id.read().clone();
    let menu_id_for_keys = menu_id_str.clone();

    let onkeydown = move |evt: KeyboardEvent| match evt.key() {
        Key::Escape => {
            evt.prevent_default();
            close.call(CloseReason::Escape);
        }
        Key::ArrowDown => {
            evt.prevent_default();
            focus_menu_item(&menu_id_for_keys, FocusAction::Next);
        }
        Key::ArrowUp => {
            evt.prevent_default();
            focus_menu_item(&menu_id_for_keys, FocusAction::Prev);
        }
        Key::Home => {
            evt.prevent_default();
            focus_menu_item(&menu_id_for_keys, FocusAction::First);
        }
        Key::End => {
            evt.prevent_default();
            focus_menu_item(&menu_id_for_keys, FocusAction::Last);
        }
        Key::Tab => {
            // Do NOT prevent_default; let the browser advance focus
            // to the next focusable element. CloseReason::Tab tells
            // the parent to NOT re-focus the trigger so the user's
            // Tab traversal is honoured.
            close.call(CloseReason::Tab);
        }
        _ => {}
    };

    let backdrop_onclick = move |_| {
        close.call(CloseReason::ClickOutside);
    };

    let combined = merge_class("if-menu__list", "", class.as_deref());
    let style = format!(
        "position: fixed; left: {}px; top: {}px; z-index: 1001;",
        coords.x, coords.y
    );

    rsx! {
        // Backdrop sits at z-index 1000 (set on `.if-menu__backdrop`),
        // list at z-index 1001 (inline). Both fixed so they escape any
        // ancestor stacking context.
        div {
            class: "if-menu__backdrop",
            "aria-hidden": "true",
            onclick: backdrop_onclick,
        }
        div {
            class: "{combined}",
            id: "{menu_id_str}",
            role: "menu",
            tabindex: "-1",
            "aria-labelledby": aria_labelledby,
            style: "{style}",
            onkeydown,
            {children}
        }
    }
}
