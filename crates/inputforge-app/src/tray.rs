// Rust guideline compliant 2026-03-06

//! System tray icon management for `InputForge`.
//!
//! Provides [`AppTray`] which creates and manages the system tray icon,
//! context menu, and menu event polling.

use std::sync::Arc;

use anyhow::Result;
use parking_lot::RwLock;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use inputforge_core::state::AppState;
use inputforge_core::state::EngineStatus;

/// Actions that can be triggered from the tray context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrayAction {
    /// Open (or reopen) the GUI window.
    ShowGui,
    /// Toggle engine between active and inactive.
    ToggleActivation,
    /// Quit the application entirely.
    Quit,
}

/// Manages the system tray icon and its context menu.
pub(crate) struct AppTray {
    /// Held to keep the tray icon alive; dropped on shutdown.
    _icon: TrayIcon,
    show_item: MenuItem,
    toggle_item: MenuItem,
    quit_item: MenuItem,
    state: Arc<RwLock<AppState>>,
}

impl std::fmt::Debug for AppTray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppTray")
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl AppTray {
    /// Create a new tray icon with context menu.
    ///
    /// The icon appears immediately in the system tray.
    pub(crate) fn new(state: Arc<RwLock<AppState>>) -> Result<Self> {
        let show_item = MenuItem::new("Show GUI", true, None);
        let toggle_item = MenuItem::new("Activate", true, None);
        let quit_item = MenuItem::new("Quit", true, None);

        let menu = Menu::new();
        menu.append(&show_item)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&toggle_item)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&quit_item)?;

        let icon = load_icon()?;

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("InputForge")
            .with_icon(icon)
            .build()?;

        Ok(Self {
            _icon: tray,
            show_item,
            toggle_item,
            quit_item,
            state,
        })
    }

    /// Poll for a pending tray menu event.
    ///
    /// Returns `None` if no event is queued.
    pub(crate) fn poll_event(&self) -> Option<TrayAction> {
        let event = MenuEvent::receiver().try_recv().ok()?;
        if event.id == *self.show_item.id() {
            return Some(TrayAction::ShowGui);
        }
        if event.id == *self.toggle_item.id() {
            return Some(TrayAction::ToggleActivation);
        }
        if event.id == *self.quit_item.id() {
            return Some(TrayAction::Quit);
        }
        None
    }

    /// Update the toggle menu item text to match current engine status.
    pub(crate) fn refresh_toggle_label(&self) {
        let status = self.state.read().engine_status;
        let label = match status {
            EngineStatus::Running => "Deactivate",
            EngineStatus::Paused | EngineStatus::Stopped => "Activate",
        };
        self.toggle_item.set_text(label);
    }
}

/// Icon dimensions for the tray icon.
const ICON_SIZE: u32 = 32;

/// Load the embedded tray icon from the compiled-in RGBA data.
fn load_icon() -> Result<Icon> {
    let rgba = include_bytes!("../assets/icon.rgba").to_vec();
    let icon = Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE)?;
    Ok(icon)
}
