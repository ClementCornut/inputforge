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
    let close = use_callback(move |_reason: CloseReason| {
        let mut o = open;
        o.set(false);
    });
    let state = MenuState {
        open,
        menu_id,
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
    let onclick = move |_| {
        let now = !*state.open.read();
        state.open.set(now);
    };
    rsx! {
        button {
            class: "{combined}",
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
pub fn MenuItems(
    /// Class extension for the OUTER positioned container (`.if-menu__items`),
    /// NOT the visible list. The visible chrome (background, border, shadow,
    /// `min-width`) lives on the inner `.if-menu__list`. If you need to
    /// customise the list surface, use a descendant selector
    /// (e.g. `.your-class .if-menu__list { ... }`) rather than expecting
    /// `your-class` to land on the surface itself.
    #[props(default)]
    class: Option<String>,
    /// Horizontal alignment of the dropdown relative to its trigger.
    /// Defaults to `Start` (the historical behaviour). `Center` and `End`
    /// switch on CSS modifier classes that override the default `left: 0`.
    #[props(default)]
    anchor: Anchor,
    children: Element,
) -> Element {
    let state = use_context::<MenuState>();
    let mut open_signal = state.open;
    let menu_id = state.menu_id.read().clone();
    let anchor_class = match anchor {
        Anchor::Start => "",
        Anchor::Center => "if-menu__items--center",
        Anchor::End => "if-menu__items--end",
    };
    let combined = merge_class("if-menu__items", anchor_class, class.as_deref());

    let target_id_for_keydown = menu_id.clone();
    let onkeydown = move |evt: KeyboardEvent| {
        let action = match evt.key() {
            Key::Escape => {
                open_signal.set(false);
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
    let onclick = move |_| {
        open_signal.set(false);
    };

    // Auto-focus the first menuitem when the menu opens. use_effect tracks
    // open_signal reads, so this fires on the false→true transition. The
    // eval is queued post-render so the [hidden] flip is applied first.
    let target_id_for_focus = menu_id.clone();
    use_effect(move || {
        if *open_signal.read() {
            focus_menu_item(&target_id_for_focus, FocusAction::First);
        }
    });

    let is_open = *open_signal.read();
    rsx! {
        div {
            class: "{combined}",
            id: "{menu_id}",
            role: "menu",
            tabindex: "-1",
            hidden: !is_open,
            onkeydown,
            div {
                class: "if-menu__backdrop",
                onclick,
            }
            div { class: "if-menu__list", {children} }
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
    /// directly on the list. This differs from `MenuItems.class` (which
    /// targets the outer `.if-menu__items` wrapper) and from `MenuRoot.class`
    /// (which targets the outer `.if-menu` wrapper). If you need wrapper-level
    /// styling, lift it to a parent component.
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
    let close_handler = on_close;
    let close = use_callback(move |reason: CloseReason| {
        close_handler.call(reason);
    });
    let state = MenuState {
        open: open_signal,
        menu_id,
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
        // Backdrop sits at z-index 1000, list at 1001. Both fixed so they
        // escape any ancestor stacking context.
        div {
            class: "if-menu__backdrop if-menu__backdrop--anchored",
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
