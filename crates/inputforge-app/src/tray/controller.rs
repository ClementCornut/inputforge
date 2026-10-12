//! Application-owned tray actions, status labels and availability recovery.
use dioxus::prelude::*;
use inputforge_core::{engine::EngineCommand, state::EngineStatus};
use inputforge_gui_dx::{EditorNavigation, PanelSlot};
use std::{cell::RefCell, rc::Rc, sync::Arc};

use super::{
    AppTray,
    action::{TrayAction, TrayMenuIds},
    availability, events, policy, window,
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
    mut navigation: EditorNavigation,
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
                if policy::controller_changes_pending(&handles.state.read()) {
                    window::show_window();
                    navigation.calibration.set(false);
                    navigation.panel.set(PanelSlot::Devices);
                    return;
                }
                let command = match handles.state.read().engine_status {
                    EngineStatus::Running | EngineStatus::Starting => EngineCommand::Deactivate,
                    EngineStatus::Stopped | EngineStatus::Faulted => EngineCommand::Activate,
                };
                let _ = handles.commands.send(command);
            }
        },
        move || {
            on_tick();
            monitor();
            let Some(state) = tick_state.try_read() else {
                return;
            };
            let label = policy::toggle_label(
                state.engine_status,
                policy::controller_changes_pending(&state),
            );
            drop(state);
            if last_label != Some(label) {
                toggle_item.set_text(label);
                last_label = Some(label);
            }
        },
    );
}
