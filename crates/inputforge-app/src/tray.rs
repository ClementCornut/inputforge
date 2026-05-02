// Rust guideline compliant 2026-03-06

//! System tray icon management for `InputForge`.
//!
//! Provides [`AppTray`] which creates and manages the system tray icon,
//! context menu, and menu event polling.

use std::sync::Arc;

use anyhow::Result;
use parking_lot::RwLock;
use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use inputforge_core::state::AppState;

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

    /// Return cloned menu item IDs for (show, toggle, quit).
    ///
    /// Used to pass IDs to the GUI so it can poll `MenuEvent` while open.
    pub(crate) fn menu_item_ids(&self) -> (MenuId, MenuId, MenuId) {
        (
            self.show_item.id().clone(),
            self.toggle_item.id().clone(),
            self.quit_item.id().clone(),
        )
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
