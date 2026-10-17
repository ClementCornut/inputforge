//! Mounted rail shortcuts and field transitions using current app navigation.
use super::{
    Controller, listener,
    rail_keys::{Intent, Key, State, handle_key},
};
use dioxus::prelude::*;
use inputforge_gui_dx::{
    mapping_rail::{Lookup, Operation, Outcome, Request, Scope},
    navigation::{Listener, RailPublication},
};
use std::{cell::Cell, rc::Rc};
pub(super) fn mount(c: Controller, mut p: RailPublication) -> Listener {
    let key = listener::key("rail-keyboard");
    let script = include_str!("../../assets/rail_shortcuts.js").replace("__LISTENER_KEY__", &key);
    let mut handle = document::eval(&script);
    let closed = Rc::new(Cell::new(false));
    let close = listener::close(handle, key, Rc::clone(&closed));
    let query_closed = Rc::clone(&closed);
    let device_closed = Rc::clone(&closed);
    spawn(async move {
        while let Ok((key, meta, ctrl, alt, context)) =
            handle.recv::<(String, u8, u8, u8, String)>().await
        {
            if closed.get() || c.inner.borrow().disposed {
                break;
            }
            if p.capture.try_peek().is_ok_and(|a| *a) {
                continue;
            }
            route(&c, p, &key, meta, ctrl, alt, &context);
        }
    });
    Listener {
        close,
        query: Callback::new(move |q| {
            if !query_closed.get() && p.query.try_peek().is_ok() {
                p.query.set(q);
            }
        }),
        device: Callback::new(move |id| {
            if !device_closed.get() && p.device.try_peek().is_ok() {
                let next = if *p.device.peek() == id { None } else { id };
                p.device.set(next);
            }
        }),
    }
}
fn route(
    c: &Controller,
    mut p: RailPublication,
    key: &str,
    meta: u8,
    ctrl: u8,
    alt: u8,
    context: &str,
) {
    let Some(nav) = c.inner.borrow().chrome.publication else {
        return;
    };
    let Some(state) = c.state.try_read() else {
        return;
    };
    let profile = state.active_profile.as_ref().map(|p| p.id().clone());
    if context != format!("{profile:?}:{}", nav.editing_mode.peek()) {
        return;
    }
    drop(state);
    let key = match key {
        "ArrowUp" if alt == 1 => Key::AltArrowUp,
        "ArrowDown" if alt == 1 => Key::AltArrowDown,
        "ArrowUp" => Key::ArrowUp,
        "ArrowDown" => Key::ArrowDown,
        "Enter" => Key::Enter,
        "Escape" => Key::Escape,
        "f" | "F" if meta == 1 || ctrl == 1 => Key::FilterShortcut,
        _ => return,
    };
    let rows = p.rows.peek().clone();
    let refs = rows.iter().collect::<Vec<_>>();
    let selected = nav.selected_mapping.peek().clone();
    let intent = handle_key(
        key,
        State {
            visible_rows: &refs,
            selected: selected.as_ref().map(|(m, i)| (m.as_str(), i)),
            capture_armed: *p.capture.peek(),
            filter_focused: *p.focused.peek(),
            filter_query_empty: p.query.peek().trim().is_empty(),
        },
    );
    match intent {
        Intent::Select(key) => c.navigate_editor(inputforge_gui_dx::navigation::Intent::Select {
            profile: profile.clone(),
            key,
        }),
        Intent::FocusEditor => p.focus.call("[data-editor-focus]".into()),
        Intent::FocusFilter => p.focus.call(".if-rail__filter input".into()),
        Intent::ClearFilter => p.query.set(String::new()),
        Intent::ReorderSelected { mode, input, dir } => {
            let cfg = p.config.peek();
            let Some(row) = cfg
                .mappings
                .iter()
                .find(|m| m.mode == mode && m.input == input)
            else {
                return;
            };
            let filtered = !p.query.peek().trim().is_empty() || p.device.peek().is_some();
            let menu = c.rail_menu(Lookup {
                scope: Scope {
                    profile: p.meta.peek().profile_id.clone(),
                    mode,
                },
                input,
                name: row.name.clone(),
                filtered,
            });
            drop(cfg);
            if let Some(menu) = menu {
                match c.rail(Request {
                    target: menu.target,
                    operation: Operation::Reorder {
                        movement: dir,
                        order: menu.order,
                        group: menu.group,
                        filtered,
                    },
                }) {
                    Outcome::Moved { index, len, group } => {
                        p.announcement
                            .set(crate::ui::sortable::rail_announcement((index, len, group)));
                        p.feedback.set(None);
                    }
                    Outcome::Rejected(error) => p.feedback.set(Some(error)),
                    _ => {}
                }
            }
        }
        Intent::NoOp => {}
    }
}
