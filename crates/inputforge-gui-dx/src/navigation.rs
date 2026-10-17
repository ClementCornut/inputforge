//! Prepared editor navigation signals and application callbacks.
use crate::{PanelSlot, snapshots::MappingKey};
use dioxus::prelude::*;
/// Primary workspace selected by the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Surface {
    #[default]
    Mappings,
    BulkMap,
}
/// Renderer-owned editor navigation signals.
#[derive(Debug, Clone, Copy)]
pub struct Publication {
    pub main_surface: Signal<Surface>,
    pub editing_mode: Signal<String>,
    pub panel_slot: Signal<PanelSlot>,
    pub via_calibration: Signal<bool>,
    pub selected_mapping: Signal<Option<MappingKey>>,
}
/// Raw navigation interaction.
#[derive(Debug, Clone)]
pub enum Intent {
    Surface(Surface),
    TogglePanel(PanelSlot),
    OpenPanel(PanelSlot),
    Select {
        profile: Option<inputforge_core::profile::ProfileId>,
        key: MappingKey,
    },
}
/// Application-owned navigation lifecycle and transitions.
#[derive(Debug, Clone, Copy)]
pub struct Bindings {
    pub install_editor: Callback<EditorPublication>,
    pub editor: Callback<EditorPublication, Listener>,
    pub rail: Callback<RailPublication, Listener>,
    pub stage: Callback<StageRequest>,
    pub filter: Callback<FilterRequest, FilterView>,
    pub install: Callback<Publication>,
    pub dispatch: Callback<Intent>,
    pub reconcile: Callback<()>,
}

/// Raw rail filter inputs.
#[derive(Debug, Clone)]
pub struct FilterRequest {
    pub config: crate::snapshots::ConfigSnapshot,
    pub mode: String,
    pub query: String,
    pub device: Option<inputforge_core::types::DeviceId>,
}
/// Prepared rail rows and device filters.
#[derive(Debug, Clone, PartialEq)]
pub struct FilterView {
    pub filtered: bool,
    pub total: usize,
    pub rows: Vec<crate::snapshots::MappingSummary>,
    pub chips: Vec<DeviceChip>,
    pub device: Option<inputforge_core::types::DeviceId>,
}
/// Device filter label supplied by the application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceChip {
    pub id: inputforge_core::types::DeviceId,
    pub label: String,
}

/// Mounted listener cleanup and raw rail field event forwarding.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Listener {
    pub close: Callback<()>,
    pub query: Callback<String>,
    pub device: Callback<Option<inputforge_core::types::DeviceId>>,
}
/// Renderer-owned rail fields and supplied DOM focus execution.
#[derive(Debug, Clone, Copy)]
pub struct RailPublication {
    pub query: Signal<String>,
    pub focused: Signal<bool>,
    pub device: Signal<Option<inputforge_core::types::DeviceId>>,
    pub rows: Memo<Vec<MappingKey>>,
    pub config: Signal<crate::snapshots::ConfigSnapshot>,
    pub meta: Signal<crate::snapshots::MetaSnapshot>,
    pub capture: Signal<bool>,
    pub feedback: Signal<Option<String>>,
    pub announcement: Signal<String>,
    pub focus: Callback<String>,
}
/// Rendering state for one stage menu.
#[derive(Debug, Clone, PartialEq)]
pub struct StageMenu {
    pub stage: crate::mapping_edits::StageId,
    pub x: f64,
    pub y: f64,
}
/// Renderer-owned expansion, diagnostics and menu signals.
#[derive(Debug, Clone, Copy)]
pub struct EditorPublication {
    pub expanded: Signal<std::collections::HashSet<crate::mapping_edits::StageId>>,
    pub menu: Signal<Option<StageMenu>>,
    pub hints: Signal<std::collections::HashMap<crate::mapping_edits::StageId, String>>,
    pub tags: Signal<std::collections::HashMap<crate::mapping_edits::StageId, String>>,
    pub error: Signal<Option<String>>,
    pub focus: Callback<String>,
}

/// Captured stage event target.
#[derive(Debug, Clone)]
pub struct StageRequest {
    pub actions: Vec<inputforge_core::action::Action>,
    pub profile: Option<inputforge_core::profile::ProfileId>,
    pub key: MappingKey,
    pub path: crate::mapping_edits::StageId,
    pub intent: StageIntent,
}
/// Raw stage header and context-menu interactions.
#[derive(Debug, Clone)]
pub enum StageIntent {
    Toggle,
    Menu(MouseEvent),
}
