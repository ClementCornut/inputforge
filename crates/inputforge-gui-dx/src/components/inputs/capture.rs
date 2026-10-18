//! Reusable capture presentation; consumers supply app-owned state and callbacks.
use crate::components::merge_class;
use dioxus::prelude::*;

/// Source readout and rebind/cancel action with a polite listening announcement.
#[component]
pub fn RebindField(
    source_label: String,
    listening: bool,
    action_label: String,
    cancel_label: String,
    onclick: EventHandler<MouseEvent>,
    oncancel: EventHandler<MouseEvent>,
    #[props(default)] unbound: bool,
    #[props(default = "Move any input…".to_owned())] prompt: String,
    #[props(default)] disabled: bool,
    #[props(default)] class: Option<String>,
) -> Element {
    let variants = merge_class(
        if listening {
            "if-rebind-field--listening"
        } else {
            ""
        },
        if unbound {
            "if-rebind-field--unbound"
        } else {
            ""
        },
        None,
    );
    let classes = merge_class("if-rebind-field", &variants, class.as_deref());
    rsx! {
        div { class: classes,
            if listening {
                span { class: "if-rebind-field__listening", role: "status", "aria-live": "polite", "{prompt}" }
                button { r#type: "button", class: "if-rebind-field__action", disabled, "aria-label": cancel_label, onclick: oncancel, "Cancel" }
            } else {
                span { class: "if-rebind-field__label", "{source_label}" }
                button { r#type: "button", class: "if-rebind-field__action", disabled, "aria-label": action_label, onclick, "rebind" }
            }
        }
    }
}

/// Stable physical-key capture button with externally supplied listening state and hint.
#[component]
pub fn KeyCaptureButton(
    value: String,
    aria_label: String,
    onclick: EventHandler<MouseEvent>,
    #[props(default)] listening: bool,
    #[props(default)] hint: Option<String>,
    #[props(default)] autofocus: bool,
    #[props(default)] data_key_capture: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] class: Option<String>,
    #[props(default)] onmounted: Option<EventHandler<MountedEvent>>,
) -> Element {
    let classes = merge_class("if-key-capture", "", class.as_deref());
    let surface = merge_class(
        "if-key-capture__surface",
        if listening {
            "if-key-capture__surface--listening"
        } else {
            ""
        },
        None,
    );
    let onmounted = move |event| {
        if let Some(handler) = onmounted {
            handler.call(event);
        }
    };
    rsx! {
        div { class: classes, "aria-live": "polite",
            button { r#type: "button", class: surface, disabled, autofocus,
                "aria-label": aria_label,
                "data-key-capture": data_key_capture,
                onclick,
                onmounted,
                span { class: "if-key-capture__value", "{value}" }
            }
            if let Some(hint) = hint { span { class: "if-key-capture__hint", role: "status", "{hint}" } }
        }
    }
}
