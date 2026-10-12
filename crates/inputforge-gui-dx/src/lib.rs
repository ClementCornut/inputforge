//! Dioxus editor rendering for `InputForge`. Desktop hosting belongs to the app.
mod app;
mod bridge;
pub mod components;
mod context;
mod frame;
pub mod icons;
pub mod patterns;
pub mod theme;
pub mod toast;

pub use frame::view_state::PanelSlot;
#[doc(hidden)]
pub use toast::ToastViewport;
pub use toast::{ToastLevel, ToastQueue, ToastState};

use dioxus::prelude::*;
use inputforge_core::{engine::EngineCommand, state::AppState};
use parking_lot::RwLock;
use std::sync::{Arc, mpsc};

/// Host-owned presentation state shared with the editor's panel controls.
#[derive(Clone, Copy, Debug)]
pub struct EditorNavigation {
    pub panel: Signal<PanelSlot>,
    pub calibration: Signal<bool>,
}

/// Render the editor inside a host-provided Dioxus scope.
/// The host owns desktop configuration, lifecycle and external events.
///
/// # Errors
/// Propagates render errors from the editor root.
pub fn render_editor(
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
) -> Element {
    use_context_provider(|| context::RawHandles { state, commands });
    app::app_root()
}
