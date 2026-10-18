//! Named keyboard/pointer interaction surface; plot geometry stays with its owner.

use dioxus::prelude::*;

use crate::components::merge_class;

/// Focusable plot group with stable instruction/value references.
/// Supply a unique mounted `id`; callbacks retain all interaction policy.
#[component]
pub fn PlotFrame(
    id: String,
    label: String,
    #[props(default)] description: Option<String>,
    #[props(default)] instructions: Option<String>,
    #[props(default)] value_text: Option<String>,
    #[props(default)] hovered: bool,
    #[props(default)] dragging: bool,
    #[props(default)] class: Option<String>,
    #[props(default)] onmounted: Option<EventHandler<MountedEvent>>,
    #[props(default)] onkeydown: Option<EventHandler<KeyboardEvent>>,
    #[props(default)] onfocusout: Option<EventHandler<FocusEvent>>,
    #[props(default)] onpointerdown: Option<EventHandler<PointerEvent>>,
    #[props(default)] onpointermove: Option<EventHandler<PointerEvent>>,
    #[props(default)] onpointerup: Option<EventHandler<PointerEvent>>,
    #[props(default)] onpointercancel: Option<EventHandler<PointerEvent>>,
    children: Element,
) -> Element {
    let combined = merge_class("if-plot-frame", "", class.as_deref());
    let description_id = format!("{id}-description");
    let instructions_id = format!("{id}-instructions");
    let value_id = format!("{id}-value");
    let describedby = [
        description.as_ref().map(|_| description_id.as_str()),
        instructions.as_ref().map(|_| instructions_id.as_str()),
        value_text.as_ref().map(|_| value_id.as_str()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ");
    let onmounted_handler = move |event| {
        if let Some(handler) = onmounted {
            handler.call(event);
        }
    };
    let onkeydown_handler = move |event| {
        if let Some(handler) = onkeydown {
            handler.call(event);
        }
    };
    let onfocusout_handler = move |event| {
        if let Some(handler) = onfocusout {
            handler.call(event);
        }
    };
    let onpointerdown_handler = move |event| {
        if let Some(handler) = onpointerdown {
            handler.call(event);
        }
    };
    let onpointermove_handler = move |event| {
        if let Some(handler) = onpointermove {
            handler.call(event);
        }
    };
    let onpointerup_handler = move |event| {
        if let Some(handler) = onpointerup {
            handler.call(event);
        }
    };
    let onpointercancel_handler = move |event| {
        if let Some(handler) = onpointercancel {
            handler.call(event);
        }
    };
    rsx! {
        div {
            id, class: combined, role: "group", tabindex: "0",
            "aria-label": label,
            "aria-describedby": (!describedby.is_empty()).then_some(describedby),
            "data-hovered": hovered.to_string(), "data-dragging": dragging.to_string(),
            onmounted: onmounted_handler,
            onkeydown: onkeydown_handler,
            onfocusout: onfocusout_handler,
            onpointerdown: onpointerdown_handler,
            onpointermove: onpointermove_handler,
            onpointerup: onpointerup_handler,
            onpointercancel: onpointercancel_handler,
            {children}
            if let Some(description) = description {
                span { id: description_id, class: "if-sr-only", "{description}" }
            }
            if let Some(instructions) = instructions {
                span { id: instructions_id, class: "if-sr-only", "{instructions}" }
            }
            if let Some(value_text) = value_text {
                span { id: value_id, class: "if-sr-only", "{value_text}" }
            }
        }
    }
}

#[cfg(test)]
mod tests;
