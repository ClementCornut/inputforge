//! Menu/dialog interpretation and scope-bound document services.
mod dialog;
mod escape;
mod listener;
mod menu;
use dioxus::prelude::*;
use inputforge_gui_dx::overlays::Bindings;
pub(super) fn bindings() -> Bindings {
    Bindings {
        menu: menu::use_menu,
        focus: menu::use_focus,
        idle_escape: escape::use_idle_escape,
        click_away: listener::use_click_away,
        dialog: dialog::use_dialog,
        resolve: Callback::new(|(mut open, action): (Signal<bool>, Callback<(), bool>)| {
            if open() && action.call(()) {
                open.set(false);
            }
        }),
    }
}
