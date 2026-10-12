//! Shared routing effects and editor bindings for Windows and Linux.
pub(crate) use inputforge_app::routing_policy as policy;
#[cfg(test)]
mod tests;

use crate::desktop_host::DesktopHandles;
use anyhow::Result;
use dioxus::prelude::*;
use inputforge_core::{engine::EngineCommand, state::AppState};
use inputforge_gui_dx::{
    EditorNavigation, PanelSlot,
    routing::{Action, Bindings},
};
use parking_lot::RwLock;
use std::sync::mpsc;

pub(crate) fn bindings(handles: DesktopHandles, mut navigation: EditorNavigation) -> Bindings {
    Bindings {
        dispatch: Callback::new(move |action| {
            if let Err(error) = dispatch(action, &handles.state, &handles.commands, || {
                crate::tray::window::show_window();
                navigation.calibration.set(false);
                navigation.panel.set(PanelSlot::Devices);
            }) {
                tracing::error!(name: "routing.dispatch.failed", ?action, %error, "routing command delivery failed");
            }
        }),
    }
}

fn dispatch(
    action: Action,
    state: &RwLock<AppState>,
    commands: &mpsc::Sender<EngineCommand>,
    review: impl FnOnce(),
) -> Result<()> {
    let action = policy::resolve(&state.read(), action);
    let Some(action) = action else {
        return Ok(());
    };
    let command = match action {
        Action::Start => EngineCommand::Activate,
        Action::Stop => EngineCommand::Deactivate,
        Action::Retry => EngineCommand::Retry,
        Action::Refresh => EngineCommand::RefreshInput,
        Action::ReviewDevices => {
            review();
            return Ok(());
        }
    };
    commands.send(command)?;
    Ok(())
}
