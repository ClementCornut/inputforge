//! Prepared devices and mappings for the editor.
use inputforge_core::{
    state::DeviceState,
    types::{
        DeviceDiagnostics, DeviceId, DeviceInfo, InputAddress, OutputAddress, VirtualDeviceConfig,
    },
};
use std::collections::{HashMap, HashSet};
/// Prepared controller, device, and mapping configuration.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConfigSnapshot {
    pub controllers: Option<inputforge_core::profile::controllers::ControllerConfig>,
    pub devices: Vec<DeviceState>,
    pub virtual_devices: Vec<VirtualDeviceConfig>,
    pub mapped_inputs: HashSet<InputAddress>,
    pub mapping_names: HashMap<InputAddress, String>,
    pub mappings: Vec<MappingSummary>,
    /// Cloned `Vec<Action>` for the currently-selected mapping, if any.
    /// Cheap because only one mapping's actions are cloned per tick.
    pub selected_mapping_actions: Option<Vec<inputforge_core::action::Action>>,
    /// The (mode, input) key recorded at the same tick. Allows the editor
    /// to detect cross-window conflicts: selection still refers to a key
    /// that the engine no longer holds.
    pub selected_mapping_key: Option<super::MappingKey>,
    pub device_panel_rows: Vec<DevicePanelRow>,
    /// Pre-resolved display name for every device the user might
    /// reference (connected devices first, then remembered devices
    /// from `device_registry`). Built once per snapshot tick so call
    /// sites collapse to a single map lookup; see
    /// [`ConfigSnapshot::device_display_name`].
    pub device_display_names: HashMap<DeviceId, String>,
}

impl ConfigSnapshot {
    /// Resolve a `DeviceId` to its user-facing display name (alias if
    /// set, else hardware name, else the raw id string). Falls back to
    /// `id.0` when the snapshot has not seen the device at all
    /// (deterministic behaviour for stale references).
    ///
    /// This is the single accessor every user-facing surface should
    /// use; never read `info.name` directly. The precedence rule
    /// itself lives in
    /// [`inputforge_core::settings::display_name_for_device`].
    #[must_use]
    pub fn device_display_name(&self, id: &DeviceId) -> String {
        self.device_display_names
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.0.clone())
    }
}

/// One row's worth of state for the F8 mapping list. Populated by
/// the app projection once per polling tick from the active
/// profile's `Mapping` entries; consumers in `frame::mapping_list` read
/// these as constant-time field accesses without re-walking the action
/// tree.
#[derive(Debug, Clone, PartialEq)]
pub struct MappingSummary {
    pub input: InputAddress,
    pub mode: String,
    pub name: Option<String>,
    pub glyphs: GlyphFlags,
    pub referenced_devices: Vec<DeviceId>,
    pub first_vjoy_output: Option<OutputAddress>,
}

/// Mapped and available controls of one input kind.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceCoverage {
    pub mapped: u8,
    pub total: u8,
}

/// Prepared mapping usage for one device.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceUsageSummary {
    pub axes: DeviceCoverage,
    pub buttons: DeviceCoverage,
    pub hats: DeviceCoverage,
    pub primary_mappings: usize,
    pub secondary_mappings: usize,
    pub touched_modes: Vec<String>,
    pub touched_mapping_names: Vec<String>,
}

/// Prepared device identity, diagnostics, and usage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevicePanelRow {
    pub device_id: DeviceId,
    pub display_name: String,
    pub alias: String,
    pub hardware_name: String,
    pub connected: bool,
    pub info: DeviceInfo,
    pub diagnostics: DeviceDiagnostics,
    pub usage: DeviceUsageSummary,
    pub last_seen_unix_ms: Option<u64>,
}

/// Pre-computed glyph state for a `MappingSummary`. The walker stops on
/// the first match per glyph, so both fields hold the *first*
/// occurrence found by depth-first traversal of the action tree.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GlyphFlags {
    /// `Some(addr)` if the action tree contains an `Action::MergeAxis`
    /// whose `second_input` is `addr`, the secondary input shown after
    /// the gold `+` glyph.
    pub merge_secondary: Option<InputAddress>,
    /// `Some(addr)` if the action tree contains an `Action::Conditional`
    /// whose `condition` references at least one `InputAddress` (via
    /// `ButtonPressed | ButtonReleased | AxisInRange | HatDirection`,
    /// possibly nested under `All | Any | Not`). The violet `+` glyph
    /// hover-tooltip in `row.rs` runs this address through
    /// `source_label::format` to produce the human-readable predicate
    /// label (identical path to `merge_secondary`).
    pub first_input_predicate: Option<InputAddress>,
}
