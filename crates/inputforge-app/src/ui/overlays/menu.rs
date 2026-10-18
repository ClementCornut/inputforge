//! Menu policy shared by trigger and cursor anchored surfaces.
use dioxus::prelude::*;
use inputforge_gui_dx::overlays::{Activation, CloseReason, FocusAction, MenuProps, MenuView};
fn focus_script(menu: &str, action: FocusAction) -> String {
    let action = match action {
        FocusAction::Next => "down",
        FocusAction::Prev => "up",
        FocusAction::First => "first",
        FocusAction::Last => "last",
    };
    format!(
        "({})({menu:?}, {action:?});",
        include_str!("../../../assets/menu_focus.js")
    )
}
fn focus(menu: &str, action: FocusAction) {
    let _ = document::eval(&focus_script(menu, action));
}
pub(super) fn use_focus(open: bool, ready: bool, id: String) {
    use_effect(use_reactive!(|(open, ready, id)| {
        if open && ready {
            let script = focus_script(&id, FocusAction::First);
            let _ = document::eval(&format!("requestAnimationFrame(() => {{ {script} }});"));
        }
    }));
}
pub(super) fn use_menu(props: MenuProps) -> MenuView {
    let mut open = props.open;
    let close = use_callback(move |reason| {
        if let Some(handler) = props.onclose {
            if !handler.call(reason) {
                return;
            }
        } else {
            open.set(false);
        }
        if matches!(reason, CloseReason::Escape | CloseReason::ItemActivated)
            && !props.trigger.is_empty()
        {
            let _ = document::eval(&format!(
                "document.getElementById({:?})?.focus();",
                props.trigger
            ));
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

#[cfg(test)]
#[path = "menu_tests.rs"]
mod tests;
