//! Tray menu routing — pure functions, no Dioxus or tao dependencies.

use muda::{MenuEvent, MenuId};

/// Internal action set produced by the tray menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrayAction {
    Show,
    Toggle,
    Quit,
}

/// The three menu ids this app builds (cloned from `tray_icon::AppTray`).
#[derive(Debug, Clone)]
pub(crate) struct TrayMenuIds {
    pub show: MenuId,
    pub toggle: MenuId,
    pub quit: MenuId,
}

impl TrayAction {
    /// Pure routing function — testable without constructing a `MenuEvent`.
    pub(crate) fn from_id(id: &MenuId, ids: &TrayMenuIds) -> Option<Self> {
        if *id == ids.show {
            return Some(Self::Show);
        }
        if *id == ids.toggle {
            return Some(Self::Toggle);
        }
        if *id == ids.quit {
            return Some(Self::Quit);
        }
        None
    }

    /// Thin adapter for the live event-loop closure.
    pub(crate) fn from_event(ev: &MenuEvent, ids: &TrayMenuIds) -> Option<Self> {
        Self::from_id(&ev.id, ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_ids() -> TrayMenuIds {
        TrayMenuIds {
            show: MenuId::new("show-gui"),
            toggle: MenuId::new("toggle-activation"),
            quit: MenuId::new("quit"),
        }
    }

    #[test]
    fn from_id_routes_show() {
        let ids = fixture_ids();
        assert_eq!(
            TrayAction::from_id(&MenuId::new("show-gui"), &ids),
            Some(TrayAction::Show),
        );
    }

    #[test]
    fn from_id_routes_toggle() {
        let ids = fixture_ids();
        assert_eq!(
            TrayAction::from_id(&MenuId::new("toggle-activation"), &ids),
            Some(TrayAction::Toggle),
        );
    }

    #[test]
    fn from_id_routes_quit() {
        let ids = fixture_ids();
        assert_eq!(
            TrayAction::from_id(&MenuId::new("quit"), &ids),
            Some(TrayAction::Quit),
        );
    }

    #[test]
    fn from_id_returns_none_for_unknown() {
        let ids = fixture_ids();
        assert_eq!(TrayAction::from_id(&MenuId::new("not-our-id"), &ids), None,);
    }

    #[test]
    fn from_event_delegates_to_from_id() {
        let ids = fixture_ids();
        let ev = MenuEvent {
            id: MenuId::new("toggle-activation"),
        };
        assert_eq!(TrayAction::from_event(&ev, &ids), Some(TrayAction::Toggle));
    }

    /// Routing depends on `TrayMenuIds` equality, NOT on hardcoded
    /// `"show-gui"` / `"toggle-activation"` / `"quit"` string literals
    /// inside `from_id`. Use deliberately unconventional fixture strings
    /// to prove this — if `from_id` ever grew a string-content fallback,
    /// this test would catch it via the final `assert_eq!(.., None)`.
    #[test]
    fn from_id_routes_by_identity_not_string_content() {
        let ids = TrayMenuIds {
            show: MenuId::new("alpha"),
            toggle: MenuId::new("beta"),
            quit: MenuId::new("gamma"),
        };
        assert_eq!(
            TrayAction::from_id(&MenuId::new("alpha"), &ids),
            Some(TrayAction::Show),
        );
        assert_eq!(
            TrayAction::from_id(&MenuId::new("beta"), &ids),
            Some(TrayAction::Toggle),
        );
        assert_eq!(
            TrayAction::from_id(&MenuId::new("gamma"), &ids),
            Some(TrayAction::Quit),
        );
        assert_eq!(TrayAction::from_id(&MenuId::new("show-gui"), &ids), None,);
    }
}
