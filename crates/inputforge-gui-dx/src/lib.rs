//! Dioxus editor rendering for `InputForge`. Desktop hosting belongs to the app.
mod app;
pub mod bridge;
#[cfg(not(test))]
pub mod capture;
#[cfg(test)]
pub use inputforge_gui_dx::capture;
pub mod components;
mod context;
pub mod controllers;
mod frame;
pub mod icons;
#[cfg(not(test))]
pub mod keyboard_capture;
#[cfg(test)]
pub use inputforge_gui_dx::keyboard_capture;
#[cfg(test)]
#[path = "../../inputforge-app/tests/keyboard_capture/document.rs"]
mod keyboard_document;
#[cfg(not(test))]
pub mod mapping_edits;
#[cfg(test)]
pub use inputforge_gui_dx::mapping_edits;
#[cfg(not(test))]
pub mod mapping_rail;
#[cfg(test)]
pub use inputforge_gui_dx::mapping_rail;
pub mod patterns;
pub mod routing;
#[cfg(not(test))]
pub mod snapshots;
// Unit tests use the same value types as app-backed fixture preparation.
#[cfg(test)]
pub use inputforge_gui_dx::snapshots;
pub mod theme;
pub mod toast;

pub use frame::view_state::PanelSlot;
#[doc(hidden)]
pub use toast::ToastViewport;
pub use toast::{ToastLevel, ToastQueue, ToastState};

pub use context::RawHandles;
use dioxus::prelude::*;

/// Host-owned presentation state shared with the editor's panel controls.
#[derive(Clone, Copy, Debug)]
pub struct EditorNavigation {
    pub panel: Signal<PanelSlot>,
    pub calibration: Signal<bool>,
}

/// Render the editor inside a host-provided Dioxus scope.
/// The host owns desktop configuration, lifecycle and external events.
/// `start_polling` receives the GUI publication handle once per mounted scope.
/// Engine handles remain transitional inputs for workflows awaiting extraction.
///
/// # Errors
/// Propagates render errors from the editor root.
pub fn render_editor(
    handles: RawHandles,
    routing: routing::Bindings,
    controllers: controllers::Bindings,
    capture: capture::Bindings,
    keyboard: keyboard_capture::Bindings,
    mappings: mapping_edits::Bindings,
    start_polling: Callback<bridge::Publisher>,
) -> Element {
    use_context_provider(|| routing);
    use_context_provider(|| controllers);
    use_context_provider(|| capture);
    use_context_provider(|| keyboard);
    use_context_provider(|| mappings);
    use_context_provider(|| start_polling);
    use_context_provider(|| handles);
    app::app_root()
}
