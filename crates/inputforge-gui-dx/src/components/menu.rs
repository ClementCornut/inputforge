use dioxus::prelude::*;

use super::merge_class;

/// Shared open-state context for menu compound.
#[derive(Clone, Copy)]
struct MenuState {
    open: Signal<bool>,
}

#[component]
pub fn MenuRoot(#[props(default)] class: Option<String>, children: Element) -> Element {
    let state = MenuState {
        open: use_signal(|| false),
    };
    use_context_provider(|| state);

    let combined = merge_class("if-menu", "", class.as_deref());
    rsx! { div { class: "{combined}", {children} } }
}

#[component]
pub fn MenuTrigger(children: Element) -> Element {
    let mut state = use_context::<MenuState>();
    let onclick = move |_| {
        let now = !*state.open.read();
        state.open.set(now);
    };
    rsx! {
        button {
            class: "if-menu__trigger",
            onclick,
            "aria-haspopup": "true",
            "aria-expanded": "{state.open.read()}",
            {children}
        }
    }
}

#[component]
pub fn MenuItems(children: Element) -> Element {
    let state = use_context::<MenuState>();
    let mut open_signal = state.open;
    if !*open_signal.read() {
        return rsx! {};
    }
    let onkeydown = move |evt: KeyboardEvent| {
        if evt.key() == Key::Escape {
            open_signal.set(false);
        }
    };
    let onclick = move |_| {
        open_signal.set(false);
    };
    rsx! {
        div {
            class: "if-menu__items",
            role: "menu",
            tabindex: "-1",
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
    children: Element,
) -> Element {
    let mut state = use_context::<MenuState>();
    let handler = onclick;
    let onclick = move |evt: MouseEvent| {
        if let Some(h) = &handler {
            h.call(evt);
        }
        state.open.set(false);
    };
    rsx! {
        button {
            class: "if-menu__item",
            role: "menuitem",
            disabled,
            onclick,
            {children}
        }
    }
}
