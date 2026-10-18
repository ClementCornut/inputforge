//! Menu actions and visible labelled groups.
use super::{MenuState, merge_class};
use dioxus::prelude::*;

/// Render an item; a rejected `onactivate` keeps the menu open.
#[component]
pub fn MenuItem(
    #[props(default)] onclick: Option<EventHandler<MouseEvent>>,
    #[props(default)] onactivate: Option<Callback<MouseEvent, bool>>,
    #[props(default)] disabled: bool,
    #[props(default)] compact: bool,
    #[props(default)] danger: bool,
    #[props(default)] title: Option<String>,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let state = use_context::<MenuState>();
    let variants = format!(
        "{} {}",
        if compact {
            "if-menu__item--compact"
        } else {
            ""
        },
        if danger { "if-menu__item--danger" } else { "" }
    );
    let combined = merge_class("if-menu__item", variants.trim(), class.as_deref());
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
            r#type: "button",
            class: combined,
            role: "menuitem",
            disabled,
            title,
            "aria-disabled": disabled.to_string(),
            onclick,
            {children}
        }
    }
}

/// Render visible contextual actions in the parent menu's keyboard traversal.
///
/// Destination actions should include the destination in their accessible text.
#[component]
pub fn MenuGroup(label: String, children: Element) -> Element {
    rsx! {
        div { role: "group", class: "if-menu__group", "aria-label": label.clone(),
            div { class: "if-menu__group-label", "aria-hidden": "true", "{label}" }
            {children}
        }
    }
}
