//! Per-window presentation signals and transitional workflow handles.
pub(crate) use crate::snapshots::{
    ConfigSnapshot, DevicePanelRow, DeviceUsageSummary, LiveSnapshot, MappingSummary, MetaSnapshot,
    OutputActivitySnapshot, SettingsSnapshot,
};
use dioxus::prelude::*;
use inputforge_core::{engine::EngineCommand, state::AppState};
use parking_lot::RwLock;
use std::sync::{Arc, mpsc};
/// Transitional engine handles used by workflows awaiting extraction.
// shortcut: workflow reads and commands remain here, remove as findings 2-10 move to the app.
#[derive(Clone, Debug)]
pub struct RawHandles {
    pub state: Arc<RwLock<AppState>>,
    pub commands: mpsc::Sender<EngineCommand>,
}
/// Full per-window context: raw handles plus the four reactive signals.
///
/// Assembled inside `app_root` (signals must be created within the runtime).
#[derive(Clone, Debug)]
pub(crate) struct AppContext {
    pub state: Arc<RwLock<AppState>>,
    #[cfg(test)]
    pub commands: mpsc::Sender<EngineCommand>,
    pub settings: Signal<SettingsSnapshot>,
    pub meta: Signal<MetaSnapshot>,
    pub config: Signal<ConfigSnapshot>,
    pub live: Signal<LiveSnapshot>,
}

#[cfg(test)]
pub(crate) use crate::snapshots::{
    DeviceCoverage, DeviceInputValues, GlyphFlags, VjoyOutputValues,
};
