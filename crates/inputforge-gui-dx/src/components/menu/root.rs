//! Menu rendering and app-supplied handlers.
use super::{MENU_ID_COUNTER, MenuState, merge_class};
use dioxus::prelude::*;
use std::sync::atomic::Ordering;
/// Render a trigger-attached menu with app-controlled open state.
#[component]
pub fn MenuRoot(#[props(default)] class: Option<String>, children: Element) -> Element {
    let open = use_signal(|| false);
    let menu_id = use_signal(|| {
        format!(
            "if-menu-{}",
            MENU_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
        )
    });
    let trigger_id = use_signal(|| format!("{}-trigger", menu_id.read()));
    let behavior = (super::super::behavior().overlays.menu)(crate::overlays::MenuProps {
        open,
        trigger: trigger_id.read().clone(),
        onclose: None,
    });
    let state = MenuState {
        open,
        menu_id,
        trigger_id,
        behavior,
    };
    use_context_provider(|| state);

    let combined = merge_class("if-menu", "", class.as_deref());
    rsx! { div { class: "{combined}", {children} } }
}

/// Render the accessible native trigger and forward its activation.
#[component]
pub fn MenuTrigger(
    #[props(default)] class: Option<String>,
    #[props(default)] unstyled: bool,
    #[props(default)] aria_label: Option<String>,
    children: Element,
) -> Element {
    let state = use_context::<MenuState>();
    let combined = if unstyled {
        class.as_deref().unwrap_or("").to_owned()
    } else {
        merge_class("if-menu__trigger", "", class.as_deref())
    };
    let menu_id = state.menu_id.read().clone();
    let trigger_id = state.trigger_id.read().clone();
    let onclick = state.behavior.toggle;
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

/// Render an item; a rejected `onactivate` keeps the menu open.
#[component]
pub fn MenuItem(
    onclick: Option<EventHandler<MouseEvent>>,
    onactivate: Option<Callback<MouseEvent, bool>>,
    #[props(default)] disabled: bool,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let state = use_context::<MenuState>();
    let combined = merge_class("if-menu__item", "", class.as_deref());
    let activate = state.behavior.activate;
    let onclick = move |event| {
        activate.call(crate::overlays::Activation {
            event,
            disabled,
            onclick,
            onactivate,
        });
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
