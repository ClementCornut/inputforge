//! Application-owned tray actions, status labels and availability recovery.
use dioxus::prelude::*;
use inputforge_gui_dx::routing::{Action, Bindings};
use std::{cell::RefCell, rc::Rc, sync::Arc};

use super::{
    AppTray,
    action::{TrayAction, TrayMenuIds},
    availability, events, window,
};
use crate::desktop_host::DesktopHandles;

thread_local! {
    // Native menu handles are main-thread-only; clone the host tray on remount.
    static TRAY: RefCell<Option<Rc<AppTray>>> = const { RefCell::new(None) };
}

pub(crate) fn install(tray: Option<AppTray>) {
    TRAY.with(|slot| *slot.borrow_mut() = tray.map(Rc::new));
}

pub(crate) fn clear() {
    TRAY.with(|slot| *slot.borrow_mut() = None);
}

pub(crate) fn use_controller(
    handles: DesktopHandles,
    routing: Bindings,
    mut on_tick: impl FnMut() + 'static,
) {
    let tray = use_hook(|| TRAY.with(|slot| slot.borrow().clone()));
    let Some(tray) = tray else {
        events::install_tick(on_tick);
        return;
    };
    let (show, toggle, quit) = tray.menu_item_ids();
    let toggle_item = tray.toggle_menu_item();
    let tick_state = Arc::clone(&handles.state);
    let mut monitor = use_hook(|| {
        availability::monitor(
            Arc::clone(&tray.available),
            handles.initially_available,
            Arc::clone(&handles.state),
        )
    });
    let mut last_label = None;
    events::install_native_handler(
        TrayMenuIds { show, toggle, quit },
        move |action| match action {
            TrayAction::Show => window::show_window(),
            TrayAction::Quit => window::request_quit(),
            TrayAction::Toggle => {
                let action = (routing.project)(&handles.state.read()).toggle;
                if let Some(action) = action {
                    routing.dispatch.call(action);
                }
            }
        },
        move || {
            on_tick();
            monitor();
            let Some(state) = tick_state.try_read() else {
                return;
            };
            let action = (routing.project)(&state).toggle;
            drop(state);
            let label = match action {
                Some(Action::ReviewDevices) => "Review controller changes",
                Some(action) => action.label(),
                None => "Routing unavailable",
            };
            let item = (label, action.is_some());
            if last_label != Some(item) {
                toggle_item.set_text(label);
                toggle_item.set_enabled(item.1);
                last_label = Some(item);
            }
        },
    );
}
