use std::sync::atomic::{AtomicU64, Ordering};

use dioxus::prelude::*;

use super::merge_class;

pub(crate) mod focus_walker;

pub(crate) use focus_walker::{FocusAction, focus_menu_item};

/// Per-MenuRoot ID counter. Used to give each menu instance a stable DOM id
/// so the trigger's `aria-controls` and the in-JS focus walker can address
/// the correct items wrapper.
static MENU_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Shared open-state context for menu compound.
#[derive(Clone, Copy)]
struct MenuState {
    open: Signal<bool>,
    /// Stable DOM id for the items wrapper. Shared so `MenuTrigger` can advertise
    /// `aria-controls=<id>` and `MenuItems` can render `id=<id>` matching.
    menu_id: Signal<String>,
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

#[component]
pub fn MenuRoot(#[props(default)] class: Option<String>, children: Element) -> Element {
    let state = MenuState {
        open: use_signal(|| false),
        menu_id: use_signal(|| {
            format!(
                "if-menu-{}",
                MENU_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
            )
        }),
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
    let mut state = use_context::<MenuState>();
    let combined = merge_class("if-menu__item", "", class.as_deref());
    let handler = onclick;
    let onclick = move |evt: MouseEvent| {
        if let Some(h) = &handler {
            h.call(evt);
        }
        state.open.set(false);
    };
    rsx! {
        button {
            class: "{combined}",
            role: "menuitem",
            disabled,
            onclick,
            {children}
        }
    }
}
