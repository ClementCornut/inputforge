//! Selectable rows and toggle chips; domain identity and state remain caller-owned.
use crate::components::merge_class;
use dioxus::prelude::*;

/// Render a native toggle action, separate from the presentational Chip.
#[component]
pub fn ToggleChip(
    pressed: bool,
    #[props(default)] disabled: bool,
    #[props(default)] compact: bool,
    #[props(default)] title: Option<String>,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] class: Option<String>,
    onclick: EventHandler<MouseEvent>,
    children: Element,
) -> Element {
    let class = merge_class(
        "if-toggle-chip",
        if compact {
            "if-toggle-chip--compact"
        } else {
            ""
        },
        class.as_deref(),
    );
    let onclick = move |event| {
        if !disabled {
            onclick.call(event);
        }
    };
    rsx! {
        button {
            r#type: "button",
            class,
            title,
            "aria-label": aria_label,
            "aria-pressed": pressed.to_string(),
            disabled,
            onclick,
            {children}
        }
    }
}

/// Render a focusable row group that may contain editors and drag controls.
///
/// Use Button's Row presentation for rows whose entire content is one native action.
/// The caller supplies keyboard interpretation and roving tabindex for compound rows.
#[component]
pub fn SelectableRow(
    #[props(default)] selected: bool,
    #[props(default)] live: bool,
    #[props(default)] dragging: bool,
    #[props(default)] id: Option<String>,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] aria_describedby: Option<String>,
    #[props(default)] class: Option<String>,
    #[props(default)] style: Option<String>,
    #[props(default = 0)] tabindex: i32,
    #[props(default)] data_sortable_row: bool,
    #[props(default)] onclick: Option<EventHandler<MouseEvent>>,
    #[props(default)] onkeydown: Option<EventHandler<KeyboardEvent>>,
    #[props(default)] oncontextmenu: Option<EventHandler<MouseEvent>>,
    #[props(default)] ondragstart: Option<EventHandler<DragEvent>>,
    children: Element,
) -> Element {
    let class = merge_class("if-action-row", "if-action-row--compound", class.as_deref());
    let onclick = move |event| {
        if let Some(handler) = onclick {
            handler.call(event);
        }
    };
    let onkeydown = move |event| {
        if let Some(handler) = onkeydown {
            handler.call(event);
        }
    };
    let oncontextmenu = move |event| {
        if let Some(handler) = oncontextmenu {
            handler.call(event);
        }
    };
    let ondragstart = move |event| {
        if let Some(handler) = ondragstart {
            handler.call(event);
        }
    };
    rsx! {
        div {
            id,
            role: "group",
            tabindex,
            class,
            style,
            "aria-label": aria_label,
            "aria-describedby": aria_describedby,
            "data-selected": selected.to_string(),
            "data-live": live.to_string(),
            "data-dragging": dragging.to_string(),
            "data-sortable-row": data_sortable_row.then_some("true"),
            onclick,
            onkeydown,
            oncontextmenu,
            ondragstart,
            {children}
        }
    }
}
