//! Menu policy shared by trigger and cursor anchored surfaces.
use dioxus::prelude::*;
use inputforge_gui_dx::overlays::{Activation, CloseReason, FocusAction, MenuProps, MenuView};
fn focus(menu: &str, action: FocusAction) {
    let action = match action {
        FocusAction::Next => "down",
        FocusAction::Prev => "up",
        FocusAction::First => "first",
        FocusAction::Last => "last",
    };
    let _ = document::eval(&format!(
        "({})({menu:?}, {action:?});",
        include_str!("../../../assets/menu_focus.js")
    ));
}
pub(super) fn use_focus(open: bool, ready: bool, id: String) {
    use_effect(use_reactive!(|(open, ready, id)| {
        if open && ready {
            focus(&id, FocusAction::First);
        }
    }));
}
pub(super) fn use_menu(props: MenuProps) -> MenuView {
    let mut open = props.open;
    let close = use_callback(move |reason| {
        if let Some(handler) = props.onclose {
            handler.call(reason);
        } else {
            open.set(false);
            if matches!(reason, CloseReason::Escape) {
                let _ = document::eval(&format!(
                    "document.getElementById({:?})?.focus();",
                    props.trigger
                ));
            }
        }
    });
    MenuView {
        toggle: use_callback(move |_| {
            let next = !open();
            open.set(next);
        }),
        close,
        key: use_callback(move |(event, id): (KeyboardEvent, String)| {
            let action = match event.key() {
                Key::Escape => {
                    event.prevent_default();
                    event.stop_propagation();
                    close.call(CloseReason::Escape);
                    return;
                }
                Key::Tab => {
                    event.stop_propagation();
                    close.call(CloseReason::Tab);
                    return;
                }
                Key::ArrowDown => FocusAction::Next,
                Key::ArrowUp => FocusAction::Prev,
                Key::Home => FocusAction::First,
                Key::End => FocusAction::Last,
                _ => return,
            };
            event.prevent_default();
            event.stop_propagation();
            focus(&id, action);
        }),
        activate: use_callback(move |request: Activation| {
            let Activation {
                event,
                disabled,
                onclick,
                onactivate,
            } = request;
            if disabled || !open() {
                return;
            }
            if let Some(action) = onactivate {
                if !action.call(event) {
                    return;
                }
            } else if let Some(action) = onclick {
                action.call(event);
            }
            close.call(CloseReason::ItemActivated);
        }),
    }
}
