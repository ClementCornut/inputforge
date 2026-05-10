use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, mpsc};

use dioxus::prelude::*;
use parking_lot::RwLock;

use inputforge_core::engine::EngineCommand;
use inputforge_core::pipeline::InputCache;
use inputforge_core::settings::StartupSettings;
use inputforge_core::snapshot::{SnapshotConfig, SnapshotId};
use inputforge_core::state::{
    AppState, DeviceState, EngineStatus, ProfileOrigin as CoreProfileOrigin,
};
use inputforge_core::types::{
    AxisPolarity, DeviceDiagnostics, DeviceId, DeviceInfo, HatDirection, InputAddress, InputId,
    OutputAddress, VJoyAxis, VirtualDeviceConfig,
};

/// Raw signal-free handles installed via `LaunchBuilder::with_context`.
///
/// `AppSettings` is no longer carried here: the engine state is the truth
/// source and the bridge polling task projects `state.snapshot_config`
/// directly into `Signal<SettingsSnapshot>` (F15).
#[derive(Clone, Debug)]
pub(crate) struct RawHandles {
    pub state: Arc<RwLock<AppState>>,
    pub commands: mpsc::Sender<EngineCommand>,
}

/// Polled projection of `AppSettings.snapshot` plus the count of unpinned
/// snapshots in the active profile.
///
/// `unpinned_snapshot_count` is derived from `AppState.active_snapshot_rows`
/// (the engine's projection of the active namespace, refreshed on every
/// snapshot mutation). The count is consumed by the F15 settings panel to
/// derive `would_prune` at commit time without an additional engine query
/// channel.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SettingsSnapshot {
    pub snapshot: SnapshotConfig,
    pub unpinned_snapshot_count: usize,
    pub startup: StartupSettings,
}

impl SettingsSnapshot {
    /// Project an `AppState` into a `SettingsSnapshot`.
    ///
    /// Reads `state.snapshot_config` (the engine's mirror of
    /// `AppSettings.snapshot`, populated on every settings mutation) and
    /// counts unpinned entries in `state.active_snapshot_rows` (refreshed
    /// by the engine after every snapshot-mutating command). No filesystem
    /// IO is performed; this runs on every polling tick.
    pub(crate) fn from_state(state: &AppState) -> Self {
        let snapshot = state.snapshot_config.clone();
        let unpinned_snapshot_count = state
            .active_snapshot_rows
            .iter()
            .filter(|row| !row.pinned)
            .count();
        let startup = state.startup.clone();
        Self {
            snapshot,
            unpinned_snapshot_count,
            startup,
        }
    }
}

/// Full per-window context: raw handles plus the four reactive signals.
///
/// Assembled inside `app_root` (signals must be created within the runtime).
#[derive(Clone, Debug)]
pub(crate) struct AppContext {
    pub state: Arc<RwLock<AppState>>,
    pub commands: mpsc::Sender<EngineCommand>,
    pub settings: Signal<SettingsSnapshot>,
    pub meta: Signal<MetaSnapshot>,
    pub config: Signal<ConfigSnapshot>,
    pub live: Signal<LiveSnapshot>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct MetaSnapshot {
    pub engine_status: EngineStatus,
    pub current_mode: String,
    pub profile_name: Option<String>,
    pub profile_path: Option<PathBuf>,
    pub warnings: Vec<String>,
    /// DFS pre-order names. Hierarchy queries (parent, descendants) are
    /// not surfaced through this field, components requiring tree shape
    /// read `ctx.state.active_profile.modes()` directly. The split is
    /// deliberate: this snapshot is cheap to clone-on-tick, and the
    /// only F7 consumer that needs hierarchy (delete-confirm preview)
    /// reads from raw state at dialog-open time, which is rare enough
    /// not to warrant projecting an `Arc<ModeTree>` here.
    pub modes: Vec<String>,
    pub startup_mode: Option<String>,
    pub active_profile_id: Option<String>,
    pub profile_rows: Vec<ProfileRowView>,
    pub snapshot_rows: Vec<SnapshotRowView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProfileRowOrigin {
    Library,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "profile row capabilities are explicit UI affordance flags"
)]
pub(crate) struct ProfileRowView {
    pub id: String,
    pub name: String,
    pub path_label: String,
    pub is_active: bool,
    pub origin: ProfileRowOrigin,
    /// Number of modes defined in the profile.
    pub mode_count: u32,
    /// Pre-formatted "last edited" label for display, when known.
    pub last_edited_label: Option<String>,
    pub can_open: bool,
    pub can_rename: bool,
    pub can_duplicate: bool,
    pub can_reveal: bool,
    pub can_delete: bool,
    pub can_add_to_library: bool,
    pub can_snapshot_now: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SnapshotRowView {
    pub id: SnapshotId,
    pub kind: inputforge_core::snapshot::SnapshotKind,
    pub kind_label: String,
    pub label: Option<String>,
    /// Glanceable relative time ("just now", "12m ago", "2d ago", or
    /// `YYYY-MM-DD` for older). Re-computed each projection cycle.
    pub time_relative: String,
    /// Tooltip / `<time datetime>` value. `YYYY-MM-DD HH:MM UTC`,
    /// no sub-second precision, no `T` separator.
    pub time_absolute: String,
    pub sort_key: i64,
    pub pinned: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ConfigSnapshot {
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
    pub selected_mapping_key: Option<crate::frame::MappingKey>,
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
    pub(crate) fn device_display_name(&self, id: &DeviceId) -> String {
        self.device_display_names
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.0.clone())
    }
}

/// One row's worth of state for the F8 mapping list. Populated by
/// `ConfigSnapshot::from_state` once per polling tick from the active
/// profile's `Mapping` entries; consumers in `frame::mapping_list` read
/// these as constant-time field accesses without re-walking the action
/// tree.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MappingSummary {
    pub input: InputAddress,
    pub mode: String,
    pub name: Option<String>,
    pub glyphs: GlyphFlags,
    pub referenced_devices: Vec<DeviceId>,
    pub first_vjoy_output: Option<OutputAddress>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DeviceCoverage {
    pub mapped: u8,
    pub total: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DeviceUsageSummary {
    pub axes: DeviceCoverage,
    pub buttons: DeviceCoverage,
    pub hats: DeviceCoverage,
    pub primary_mappings: usize,
    pub secondary_mappings: usize,
    pub touched_modes: Vec<String>,
    pub touched_mapping_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DevicePanelRow {
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
pub(crate) struct GlyphFlags {
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

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct LiveSnapshot {
    pub device_inputs: Vec<DeviceInputValues>,
    pub output_values: Vec<VjoyOutputValues>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct DeviceInputValues {
    pub axes: Vec<(f64, AxisPolarity)>,
    pub buttons: Vec<bool>,
    pub hats: Vec<HatDirection>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct VjoyOutputValues {
    pub axes: Vec<(VJoyAxis, f64)>,
    pub buttons: Vec<bool>,
    pub hats: Vec<HatDirection>,
}

impl MetaSnapshot {
    pub(crate) fn from_state(s: &AppState) -> Self {
        let active_profile_id = s
            .profile_path
            .as_ref()
            .map(|path| path.display().to_string());
        let mut profile_rows = s
            .profile_library_rows
            .iter()
            .map(|row| {
                let id = row.path.display().to_string();
                ProfileRowView {
                    id,
                    name: row.name.clone(),
                    path_label: row.path.display().to_string(),
                    is_active: row.is_active,
                    origin: ProfileRowOrigin::Library,
                    mode_count: profile_library_row_mode_count(row),
                    last_edited_label: profile_library_row_last_edited_label(row),
                    can_open: true,
                    can_rename: true,
                    can_duplicate: true,
                    can_reveal: true,
                    can_delete: true,
                    can_add_to_library: false,
                    can_snapshot_now: row.is_active,
                }
            })
            .collect::<Vec<_>>();
        if s.active_profile_origin == Some(CoreProfileOrigin::External) {
            if let Some(row) = active_profile_row_from_state(s, ProfileRowOrigin::External) {
                profile_rows.push(row);
            }
        }
        if profile_rows.is_empty() {
            if let Some(row) = active_profile_row_from_state(s, ProfileRowOrigin::Library) {
                profile_rows.push(row);
            }
        }
        let now_for_snapshots = chrono::Utc::now();
        let snapshot_rows = s
            .active_snapshot_rows
            .iter()
            .map(|row| SnapshotRowView {
                id: row.id,
                kind: row.kind,
                kind_label: match row.kind {
                    inputforge_core::snapshot::SnapshotKind::AutoSessionStart => "Session start",
                    inputforge_core::snapshot::SnapshotKind::AutoBeforeRestore => "Before restore",
                    // The user-visible surface that creates this kind is the
                    // "Batch map" tab; "bulk map" is engine vocabulary. Match
                    // the label to the surface so the kind badge does not
                    // stutter against the snapshot's own label which already
                    // says "Before batch map".
                    inputforge_core::snapshot::SnapshotKind::AutoBeforeBulkMap => {
                        "Before batch map"
                    }
                    inputforge_core::snapshot::SnapshotKind::Manual => "Manual",
                }
                .to_owned(),
                label: row.label.clone(),
                time_relative: format_relative_at(row.taken_at, now_for_snapshots),
                time_absolute: format_absolute_time(row.taken_at),
                sort_key: row.taken_at.timestamp_millis(),
                pinned: row.pinned,
            })
            .collect();
        Self {
            engine_status: s.engine_status,
            current_mode: s.current_mode.clone(),
            profile_name: s.active_profile.as_ref().map(|p| p.name().to_owned()),
            profile_path: s.profile_path.clone(),
            warnings: s.warnings.clone(),
            modes: s
                .active_profile
                .as_ref()
                .map(|p| {
                    p.modes()
                        .all_modes()
                        .into_iter()
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default(),
            startup_mode: s
                .active_profile
                .as_ref()
                .map(|p| p.settings().startup_mode().to_owned()),
            active_profile_id,
            profile_rows,
            snapshot_rows,
        }
    }
}

fn active_profile_row_from_state(s: &AppState, origin: ProfileRowOrigin) -> Option<ProfileRowView> {
    let path = s.profile_path.as_ref()?;
    let profile = s.active_profile.as_ref()?;
    let path_label = path.display().to_string();
    let is_external = origin == ProfileRowOrigin::External;
    let mode_count = u32::try_from(profile.modes().all_modes().len()).unwrap_or(u32::MAX);
    let last_edited_label = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .map(format_system_time_relative);

    Some(ProfileRowView {
        id: path_label.clone(),
        name: profile.name().to_owned(),
        path_label,
        is_active: true,
        origin,
        mode_count,
        last_edited_label,
        can_open: false,
        can_rename: !is_external,
        can_duplicate: !is_external,
        can_reveal: true,
        can_delete: !is_external,
        can_add_to_library: is_external,
        can_snapshot_now: true,
    })
}

/// Mode count for a `ProfileLibraryRow`. Adapter from the core
/// projection field into the GUI view model.
fn profile_library_row_mode_count(row: &inputforge_core::state::ProfileLibraryRow) -> u32 {
    row.mode_count
}

/// Pre-formatted "last edited" label for a `ProfileLibraryRow`.
fn profile_library_row_last_edited_label(
    row: &inputforge_core::state::ProfileLibraryRow,
) -> Option<String> {
    row.last_edited_at.map(format_chrono_time_relative)
}

/// Format a `DateTime<Utc>` as a short relative-time label, using
/// `now` as the reference. Pure function so tests can pin behaviour
/// deterministically without reading the wall clock.
///
/// Buckets: "just now" (under 1 min), "Nm ago" (under 1 hour),
/// "Nh ago" (under 1 day), "Nd ago" (under 1 week), `YYYY-MM-DD`
/// otherwise. Negative deltas (`taken_at` in the future, e.g. clock
/// skew) collapse to the date form so the user never sees "in 2m".
pub(crate) fn format_relative_at(
    taken_at: chrono::DateTime<chrono::Utc>,
    now: chrono::DateTime<chrono::Utc>,
) -> String {
    let delta = now.signed_duration_since(taken_at);
    if delta.num_seconds() < 0 {
        return taken_at.format("%Y-%m-%d").to_string();
    }
    let mins = delta.num_minutes();
    let hours = delta.num_hours();
    let days = delta.num_days();
    if mins < 1 {
        "just now".to_owned()
    } else if mins < 60 {
        format!("{mins}m ago")
    } else if hours < 24 {
        format!("{hours}h ago")
    } else if days < 7 {
        format!("{days}d ago")
    } else {
        taken_at.format("%Y-%m-%d").to_string()
    }
}

/// Wall-clock variant of [`format_relative_at`]. Production callers
/// pass through here; tests prefer the pure form.
fn format_chrono_time_relative(datetime: chrono::DateTime<chrono::Utc>) -> String {
    format_relative_at(datetime, chrono::Utc::now())
}

/// Format a `DateTime<Utc>` as a glanceable absolute timestamp suitable
/// for a tooltip / `<time datetime>` value. Drops sub-second precision
/// and the `T` separator; appends `UTC` so the user reads timezone
/// without parsing an offset string.
pub(crate) fn format_absolute_time(taken_at: chrono::DateTime<chrono::Utc>) -> String {
    taken_at.format("%Y-%m-%d %H:%M UTC").to_string()
}

/// Format a `SystemTime` as a short relative-time label suitable for a
/// dense profile row (e.g. "3m ago", "yesterday", "2026-04-01"). Falls
/// back to ISO 8601 date when the elapsed delta is large or the system
/// clock disagrees with the file mtime.
fn format_system_time_relative(time: std::time::SystemTime) -> String {
    let datetime: chrono::DateTime<chrono::Utc> = time.into();
    format_relative_at(datetime, chrono::Utc::now())
}

impl LiveSnapshot {
    /// Takes a pre-built `ConfigSnapshot` so device / virtual-device shape is
    /// read from a single coherent source.
    pub(crate) fn from_state(s: &AppState, cfg: &ConfigSnapshot) -> Self {
        let device_inputs: Vec<DeviceInputValues> = cfg
            .devices
            .iter()
            .map(|device| {
                let did = &device.info.id;
                DeviceInputValues {
                    axes: (0..device.info.axes)
                        .map(|i| {
                            let addr = InputAddress::Bound {
                                device: did.clone(),
                                input: InputId::Axis { index: i },
                            };
                            // Polarity source: device.info.axis_polarities
                            // (the lazy-classification table updated on
                            // re-probe). Fall back to Bipolar when the
                            // device entry is short, matching pre-Task-1
                            // behavior. The cache's polarity tag is
                            // unused here intentionally.
                            let (value, _cache_polarity) = s.input_cache.get_axis(&addr);
                            let pol = device
                                .info
                                .axis_polarities
                                .get(usize::from(i))
                                .copied()
                                .unwrap_or_default();
                            (value, pol)
                        })
                        .collect(),
                    buttons: (0..device.info.buttons)
                        .map(|i| {
                            let addr = InputAddress::Bound {
                                device: did.clone(),
                                input: InputId::Button { index: i },
                            };
                            s.input_cache.get_button(&addr)
                        })
                        .collect(),
                    hats: (0..device.info.hats)
                        .map(|i| {
                            let addr = InputAddress::Bound {
                                device: did.clone(),
                                input: InputId::Hat { index: i },
                            };
                            s.input_cache.get_hat(&addr)
                        })
                        .collect(),
                }
            })
            .collect();

        let output_values: Vec<VjoyOutputValues> = cfg
            .virtual_devices
            .iter()
            .map(|v| VjoyOutputValues {
                axes: v
                    .axes
                    .iter()
                    .map(|&a| (a, s.output_cache.get_axis(v.device_id, a)))
                    .collect(),
                buttons: (1..=v.button_count)
                    .map(|i| s.output_cache.get_button(v.device_id, i))
                    .collect(),
                hats: (0..v.hat_count)
                    .map(|i| s.output_cache.get_hat(v.device_id, i))
                    .collect(),
            })
            .collect();

        Self {
            device_inputs,
            output_values,
        }
    }
}

/// Walk an action tree in depth-first order, recording the first
/// `MergeAxis::second_input` and the first input-bearing `Conditional`
/// predicate. Returns early once both glyphs are populated, or after a
/// full traversal (whichever comes first).
fn derive_glyphs(actions: &[inputforge_core::action::Action]) -> GlyphFlags {
    let mut out = GlyphFlags::default();
    walk_actions(actions, &mut out);
    out
}

fn walk_actions(actions: &[inputforge_core::action::Action], out: &mut GlyphFlags) {
    use inputforge_core::action::Action;
    for action in actions {
        if out.merge_secondary.is_some() && out.first_input_predicate.is_some() {
            return;
        }
        match action {
            Action::MergeAxis { second_input, .. } if out.merge_secondary.is_none() => {
                out.merge_secondary = Some(second_input.clone());
            }
            Action::Conditional {
                condition,
                if_true,
                if_false,
            } => {
                if out.first_input_predicate.is_none() {
                    if let Some(addr) = first_input_predicate(condition) {
                        out.first_input_predicate = Some(addr);
                    }
                }
                walk_actions(if_true, out);
                walk_actions(if_false, out);
            }
            _ => {}
        }
    }
}

/// Recurse through `All | Any | Not` composites until an input-bearing
/// leaf (`ButtonPressed | ButtonReleased | AxisInRange | HatDirection`)
/// is found.
fn first_input_predicate(condition: &inputforge_core::action::Condition) -> Option<InputAddress> {
    use inputforge_core::action::Condition;
    match condition {
        Condition::ButtonPressed { input }
        | Condition::ButtonReleased { input }
        | Condition::AxisInRange { input, .. }
        | Condition::HatDirection { input, .. } => Some(input.clone()),
        Condition::All { conditions } | Condition::Any { conditions } => {
            conditions.iter().find_map(first_input_predicate)
        }
        Condition::Not { condition } => first_input_predicate(condition),
    }
}

fn derive_referenced_devices(
    primary: &InputAddress,
    actions: &[inputforge_core::action::Action],
) -> Vec<DeviceId> {
    fn push_addr(out: &mut Vec<DeviceId>, addr: &InputAddress) {
        if let Some(device) = addr.device() {
            if !out.iter().any(|existing| existing == device) {
                out.push(device.clone());
            }
        }
    }

    fn walk_condition(out: &mut Vec<DeviceId>, condition: &inputforge_core::action::Condition) {
        use inputforge_core::action::Condition;
        match condition {
            Condition::ButtonPressed { input }
            | Condition::ButtonReleased { input }
            | Condition::AxisInRange { input, .. }
            | Condition::HatDirection { input, .. } => push_addr(out, input),
            Condition::All { conditions } | Condition::Any { conditions } => {
                for child in conditions {
                    walk_condition(out, child);
                }
            }
            Condition::Not { condition } => walk_condition(out, condition),
        }
    }

    fn walk_actions(out: &mut Vec<DeviceId>, actions: &[inputforge_core::action::Action]) {
        use inputforge_core::action::Action;
        for action in actions {
            match action {
                Action::MergeAxis { second_input, .. } => push_addr(out, second_input),
                Action::Conditional {
                    condition,
                    if_true,
                    if_false,
                } => {
                    walk_condition(out, condition);
                    walk_actions(out, if_true);
                    walk_actions(out, if_false);
                }
                _ => {}
            }
        }
    }

    let mut out = Vec::new();
    push_addr(&mut out, primary);
    walk_actions(&mut out, actions);
    out
}

fn first_vjoy_output(actions: &[inputforge_core::action::Action]) -> Option<OutputAddress> {
    use inputforge_core::action::Action;
    for action in actions {
        match action {
            Action::MapToVJoy { output } => return Some(output.clone()),
            Action::Conditional {
                if_true, if_false, ..
            } => {
                if let Some(output) = first_vjoy_output(if_true) {
                    return Some(output);
                }
                if let Some(output) = first_vjoy_output(if_false) {
                    return Some(output);
                }
            }
            _ => {}
        }
    }
    None
}

fn build_device_panel_rows(s: &AppState) -> Vec<DevicePanelRow> {
    let mut rows = Vec::new();
    let mut live_ids = HashSet::new();

    for device in &s.devices {
        live_ids.insert(device.info.id.clone());
        rows.push(DevicePanelRow {
            device_id: device.info.id.clone(),
            display_name: inputforge_core::settings::display_name_for_device(
                &s.device_aliases,
                &device.info,
            ),
            alias: s
                .device_aliases
                .get(&device.info.id)
                .cloned()
                .unwrap_or_default(),
            hardware_name: device.info.name.clone(),
            connected: device.connected,
            info: device.info.clone(),
            diagnostics: device.diagnostics.clone(),
            usage: usage_for_device(&device.info.id, &device.info, s),
            last_seen_unix_ms: s
                .device_registry
                .get(&device.info.id)
                .and_then(|record| record.last_seen_unix_ms),
        });
    }

    for (device_id, record) in &s.device_registry {
        if live_ids.contains(device_id) {
            continue;
        }
        rows.push(DevicePanelRow {
            device_id: device_id.clone(),
            display_name: inputforge_core::settings::display_name_for_device(
                &s.device_aliases,
                &record.info,
            ),
            alias: s.device_aliases.get(device_id).cloned().unwrap_or_default(),
            hardware_name: record.info.name.clone(),
            connected: false,
            info: record.info.clone(),
            diagnostics: record.diagnostics.clone(),
            usage: usage_for_device(device_id, &record.info, s),
            last_seen_unix_ms: record.last_seen_unix_ms,
        });
    }

    rows.sort_by(|a, b| {
        b.connected.cmp(&a.connected).then_with(|| {
            a.display_name
                .to_ascii_lowercase()
                .cmp(&b.display_name.to_ascii_lowercase())
        })
    });
    rows
}

fn usage_for_device(device_id: &DeviceId, info: &DeviceInfo, s: &AppState) -> DeviceUsageSummary {
    let mut axes = HashSet::new();
    let mut buttons = HashSet::new();
    let mut hats = HashSet::new();
    let mut primary_mappings = 0;
    let mut secondary_mappings = 0;
    let mut touched_modes = Vec::new();
    let mut touched_mapping_names = Vec::new();

    if let Some(profile) = &s.active_profile {
        for mapping in profile.mappings() {
            let primary = mapping.input.device().is_some_and(|id| id == device_id);
            let referenced_devices = derive_referenced_devices(&mapping.input, &mapping.actions);
            let referenced = referenced_devices
                .iter()
                .any(|referenced| referenced == device_id);

            if primary {
                primary_mappings += 1;
                record_input_kind(&mapping.input, &mut axes, &mut buttons, &mut hats);
            }
            let action_referenced = record_referenced_input_kinds(
                device_id,
                &mapping.actions,
                &mut axes,
                &mut buttons,
                &mut hats,
            );
            if action_referenced {
                secondary_mappings += 1;
            }

            if primary || referenced || action_referenced {
                push_unique(&mut touched_modes, mapping.mode.clone());
                if let Some(name) = &mapping.name {
                    push_unique(&mut touched_mapping_names, name.clone());
                }
            }
        }
    }

    DeviceUsageSummary {
        axes: DeviceCoverage {
            mapped: set_len_as_u8(&axes),
            total: info.axes,
        },
        buttons: DeviceCoverage {
            mapped: set_len_as_u8(&buttons),
            total: info.buttons,
        },
        hats: DeviceCoverage {
            mapped: set_len_as_u8(&hats),
            total: info.hats,
        },
        primary_mappings,
        secondary_mappings,
        touched_modes,
        touched_mapping_names,
    }
}

fn record_input_kind(
    address: &InputAddress,
    axes: &mut HashSet<u8>,
    buttons: &mut HashSet<u8>,
    hats: &mut HashSet<u8>,
) {
    let InputAddress::Bound { input, .. } = address else {
        return;
    };
    match input {
        InputId::Axis { index } => {
            axes.insert(*index);
        }
        InputId::Button { index } => {
            buttons.insert(*index);
        }
        InputId::Hat { index } => {
            hats.insert(*index);
        }
    }
}

fn record_referenced_input_kinds(
    device_id: &DeviceId,
    actions: &[inputforge_core::action::Action],
    axes: &mut HashSet<u8>,
    buttons: &mut HashSet<u8>,
    hats: &mut HashSet<u8>,
) -> bool {
    use inputforge_core::action::Action;
    let mut found = false;
    for action in actions {
        match action {
            Action::MergeAxis { second_input, .. }
                if second_input.device().is_some_and(|id| id == device_id) =>
            {
                record_input_kind(second_input, axes, buttons, hats);
                found = true;
            }
            Action::Conditional {
                condition,
                if_true,
                if_false,
            } => {
                found |= record_condition_input_kinds(device_id, condition, axes, buttons, hats);
                found |= record_referenced_input_kinds(device_id, if_true, axes, buttons, hats);
                found |= record_referenced_input_kinds(device_id, if_false, axes, buttons, hats);
            }
            _ => {}
        }
    }
    found
}

fn record_condition_input_kinds(
    device_id: &DeviceId,
    condition: &inputforge_core::action::Condition,
    axes: &mut HashSet<u8>,
    buttons: &mut HashSet<u8>,
    hats: &mut HashSet<u8>,
) -> bool {
    use inputforge_core::action::Condition;
    match condition {
        Condition::ButtonPressed { input }
        | Condition::ButtonReleased { input }
        | Condition::AxisInRange { input, .. }
        | Condition::HatDirection { input, .. } => {
            if input.device().is_some_and(|id| id == device_id) {
                record_input_kind(input, axes, buttons, hats);
                true
            } else {
                false
            }
        }
        Condition::All { conditions } | Condition::Any { conditions } => {
            let mut found = false;
            for child in conditions {
                found |= record_condition_input_kinds(device_id, child, axes, buttons, hats);
            }
            found
        }
        Condition::Not { condition } => {
            record_condition_input_kinds(device_id, condition, axes, buttons, hats)
        }
    }
}

fn set_len_as_u8(set: &HashSet<u8>) -> u8 {
    set.len().try_into().unwrap_or(u8::MAX)
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

impl ConfigSnapshot {
    pub(crate) fn from_state(s: &AppState, selection: Option<&crate::frame::MappingKey>) -> Self {
        let mut mapped_inputs = HashSet::new();
        let mut mapping_names = HashMap::new();
        let mut mappings = Vec::new();
        let mut selected_mapping_actions: Option<Vec<inputforge_core::action::Action>> = None;
        if let Some(profile) = &s.active_profile {
            for mapping in profile.mappings() {
                mapped_inputs.insert(mapping.input.clone());
                if let Some(name) = &mapping.name {
                    mapping_names.insert(mapping.input.clone(), name.clone());
                }
                mappings.push(MappingSummary {
                    input: mapping.input.clone(),
                    mode: mapping.mode.clone(),
                    name: mapping.name.clone(),
                    glyphs: derive_glyphs(&mapping.actions),
                    referenced_devices: derive_referenced_devices(&mapping.input, &mapping.actions),
                    first_vjoy_output: first_vjoy_output(&mapping.actions),
                });
                if let Some((sel_mode, sel_input)) = selection {
                    if mapping.mode == *sel_mode && mapping.input == *sel_input {
                        selected_mapping_actions = Some(mapping.actions.clone());
                    }
                }
            }
        }
        Self {
            devices: s.devices.clone(),
            virtual_devices: s.virtual_devices.clone(),
            mapped_inputs,
            mapping_names,
            mappings,
            selected_mapping_actions,
            selected_mapping_key: selection.cloned(),
            device_panel_rows: build_device_panel_rows(s),
            device_display_names: build_device_display_names(s),
        }
    }
}

/// Build the precomputed `DeviceId -> display_name` map. Inserts
/// remembered devices first (`s.device_registry`), then connected
/// devices (`s.devices`), so a live entry overwrites a stale
/// remembered entry on key collision. The same device can therefore
/// appear in both maps; live identity wins.
fn build_device_display_names(s: &AppState) -> HashMap<DeviceId, String> {
    let mut out = HashMap::new();
    for (id, record) in &s.device_registry {
        out.insert(
            id.clone(),
            inputforge_core::settings::display_name_for_device(&s.device_aliases, &record.info),
        );
    }
    for device in &s.devices {
        out.insert(
            device.info.id.clone(),
            inputforge_core::settings::display_name_for_device(&s.device_aliases, &device.info),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_device(id: &str, name: &str, axes: u8, buttons: u8, hats: u8) -> DeviceInfo {
        DeviceInfo {
            id: DeviceId(id.to_owned()),
            name: name.to_owned(),
            axes,
            buttons,
            hats,
            instance_path: None,
            axis_polarities: vec![AxisPolarity::Bipolar; usize::from(axes)],
        }
    }

    fn profile_with_primary_merge_and_conditional_device_refs() -> inputforge_core::profile::Profile
    {
        use inputforge_core::action::{Action, Condition, Mapping};
        use inputforge_core::mode::ModeTree;
        use inputforge_core::profile::Profile;
        use inputforge_core::types::MergeOp;

        let modes =
            ModeTree::from_adjacency(&HashMap::from([("Default".to_owned(), vec![])])).unwrap();
        let device = DeviceId("dev-1".to_owned());
        let other = DeviceId("dev-2".to_owned());
        let third = DeviceId("dev-3".to_owned());
        let primary_button = InputAddress::Bound {
            device: device.clone(),
            input: InputId::Button { index: 0 },
        };
        let secondary_axis = InputAddress::Bound {
            device: device.clone(),
            input: InputId::Axis { index: 1 },
        };

        Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            vec![
                Mapping {
                    input: primary_button.clone(),
                    mode: "Default".to_owned(),
                    name: Some("Primary fire".to_owned()),
                    actions: vec![],
                },
                Mapping {
                    input: InputAddress::Bound {
                        device: other,
                        input: InputId::Axis { index: 0 },
                    },
                    mode: "Default".to_owned(),
                    name: Some("Merged axis".to_owned()),
                    actions: vec![Action::MergeAxis {
                        second_input: secondary_axis,
                        operation: MergeOp::Average,
                    }],
                },
                Mapping {
                    input: InputAddress::Bound {
                        device: third,
                        input: InputId::Button { index: 0 },
                    },
                    mode: "Default".to_owned(),
                    name: Some("Conditional fire".to_owned()),
                    actions: vec![Action::Conditional {
                        condition: Condition::ButtonPressed {
                            input: primary_button,
                        },
                        if_true: vec![],
                        if_false: vec![],
                    }],
                },
            ],
            vec![],
            "Default".to_owned(),
        )
    }

    #[test]
    fn meta_snapshot_default_is_empty() {
        let m = MetaSnapshot::default();
        assert_eq!(m.engine_status, EngineStatus::Stopped);
        assert!(m.current_mode.is_empty());
        assert!(m.profile_name.is_none());
        assert!(m.profile_path.is_none());
        assert!(m.warnings.is_empty());
        assert!(m.modes.is_empty());
        assert!(m.startup_mode.is_none());
    }

    #[test]
    fn config_snapshot_default_is_empty() {
        let c = ConfigSnapshot::default();
        assert!(c.devices.is_empty());
        assert!(c.virtual_devices.is_empty());
        assert!(c.mapped_inputs.is_empty());
        assert!(c.mapping_names.is_empty());
        assert!(c.mappings.is_empty());
        assert!(c.device_panel_rows.is_empty());
    }

    #[test]
    fn live_snapshot_default_is_empty() {
        let l = LiveSnapshot::default();
        assert!(l.device_inputs.is_empty());
        assert!(l.output_values.is_empty());
    }

    #[test]
    fn meta_from_state_extracts_lifecycle_fields() {
        let mut state = AppState::new();
        state.engine_status = EngineStatus::Running;
        state.current_mode = "FlightAssist".to_owned();
        state.warnings.push("HidHide unavailable".to_owned());
        state.profile_path = Some(PathBuf::from("/tmp/profile.json"));

        let meta = MetaSnapshot::from_state(&state);
        assert_eq!(meta.engine_status, EngineStatus::Running);
        assert_eq!(meta.current_mode, "FlightAssist");
        assert_eq!(meta.profile_name, None); // active_profile is None
        assert_eq!(meta.profile_path, Some(PathBuf::from("/tmp/profile.json")));
        assert_eq!(meta.warnings, vec!["HidHide unavailable".to_owned()]);
    }

    #[test]
    fn meta_from_state_with_active_profile_maps_name() {
        use inputforge_core::mode::ModeTree;
        use inputforge_core::profile::Profile;

        let mut map = HashMap::new();
        map.insert("Default".to_owned(), vec![]);
        let modes = ModeTree::from_adjacency(&map).unwrap();

        let profile = Profile::new(
            "Hornet".to_owned(),
            vec![],
            modes,
            vec![],
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);
        let meta = MetaSnapshot::from_state(&state);
        assert_eq!(meta.profile_name, Some("Hornet".to_owned()));
    }

    #[test]
    fn config_from_state_clones_devices_and_virtual_devices() {
        use inputforge_core::types::{DeviceDiagnostics, DeviceId, DeviceInfo};

        let mut state = AppState::new();
        state.devices.push(DeviceState {
            info: DeviceInfo {
                id: DeviceId("dev-1".to_owned()),
                name: "Throttle".to_owned(),
                axes: 1,
                buttons: 0,
                hats: 0,
                instance_path: None,
                axis_polarities: vec![AxisPolarity::Unipolar],
            },
            connected: true,
            diagnostics: DeviceDiagnostics::default(),
        });
        state.virtual_devices.push(VirtualDeviceConfig {
            device_id: 1,
            axes: vec![VJoyAxis::X],
            button_count: 4,
            hat_count: 0,
        });

        let cfg = ConfigSnapshot::from_state(&state, None);
        assert_eq!(cfg.devices.len(), 1);
        assert_eq!(cfg.devices[0].info.name, "Throttle");
        assert_eq!(cfg.virtual_devices.len(), 1);
        assert_eq!(cfg.virtual_devices[0].button_count, 4);
        assert!(cfg.mapped_inputs.is_empty()); // no profile loaded
        assert!(cfg.mapping_names.is_empty());
    }

    #[test]
    fn config_snapshot_merges_live_and_remembered_device_rows() {
        let live = DeviceState {
            info: test_device("dev-live", "Live Wheel", 4, 12, 1),
            connected: true,
            diagnostics: DeviceDiagnostics::default(),
        };
        let remembered = test_device("dev-old", "Old Pedals", 3, 0, 0);
        let mut state = AppState::new();
        state.devices.push(live);
        state
            .device_aliases
            .insert(DeviceId("dev-live".to_owned()), "Rig Wheel".to_owned());
        state.device_registry.insert(
            DeviceId("dev-old".to_owned()),
            inputforge_core::settings::DeviceRecord {
                info: remembered,
                diagnostics: DeviceDiagnostics::default(),
                last_seen_unix_ms: Some(1),
            },
        );

        let snapshot = ConfigSnapshot::from_state(&state, None);

        assert_eq!(snapshot.device_panel_rows.len(), 2);
        assert_eq!(snapshot.device_panel_rows[0].display_name, "Rig Wheel");
        assert!(snapshot.device_panel_rows[0].connected);
        assert_eq!(snapshot.device_panel_rows[1].display_name, "Old Pedals");
        assert!(!snapshot.device_panel_rows[1].connected);
    }

    #[test]
    fn config_snapshot_counts_primary_merge_and_conditional_usage() {
        let mut state =
            AppState::with_profile(profile_with_primary_merge_and_conditional_device_refs());
        state.devices.push(DeviceState {
            info: test_device("dev-1", "Stick", 6, 32, 1),
            connected: true,
            diagnostics: DeviceDiagnostics::default(),
        });

        let snapshot = ConfigSnapshot::from_state(&state, None);
        let row = snapshot
            .device_panel_rows
            .iter()
            .find(|row| row.device_id == DeviceId("dev-1".to_owned()))
            .expect("device row");

        assert_eq!(row.usage.primary_mappings, 1);
        assert_eq!(row.usage.secondary_mappings, 2);
        assert_eq!(row.usage.axes.mapped, 1);
        assert_eq!(row.usage.buttons.mapped, 1);
        assert_eq!(row.usage.hats.mapped, 0);
    }

    #[test]
    fn config_snapshot_counts_same_device_merge_axis_as_secondary_usage() {
        use inputforge_core::action::{Action, Mapping};
        use inputforge_core::mode::ModeTree;
        use inputforge_core::profile::Profile;
        use inputforge_core::types::MergeOp;

        let device = DeviceId("dev-1".to_owned());
        let primary = InputAddress::Bound {
            device: device.clone(),
            input: InputId::Button { index: 0 },
        };
        let merge_axis = InputAddress::Bound {
            device: device.clone(),
            input: InputId::Axis { index: 0 },
        };
        let modes =
            ModeTree::from_adjacency(&HashMap::from([("Default".to_owned(), vec![])])).unwrap();
        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            vec![Mapping {
                input: primary,
                mode: "Default".to_owned(),
                name: Some("Toe brake blend".to_owned()),
                actions: vec![Action::MergeAxis {
                    second_input: merge_axis,
                    operation: MergeOp::Average,
                }],
            }],
            vec![],
            "Default".to_owned(),
        );
        let mut state = AppState::with_profile(profile);
        state.devices.push(DeviceState {
            info: test_device("dev-1", "Sim Pedals", 4, 12, 0),
            connected: true,
            diagnostics: DeviceDiagnostics::default(),
        });

        let snapshot = ConfigSnapshot::from_state(&state, None);
        let row = snapshot
            .device_panel_rows
            .iter()
            .find(|row| row.device_id == device)
            .expect("device row");

        assert_eq!(row.usage.primary_mappings, 1);
        assert_eq!(row.usage.secondary_mappings, 1);
        assert_eq!(row.usage.axes.mapped, 1);
    }

    #[test]
    fn live_from_state_handles_empty_config() {
        let state = AppState::new();
        let cfg = ConfigSnapshot::from_state(&state, None);
        let live = LiveSnapshot::from_state(&state, &cfg);
        assert!(live.device_inputs.is_empty());
        assert!(live.output_values.is_empty());
    }

    #[test]
    fn live_from_state_reads_caches_per_device_shape() {
        use inputforge_core::state::DeviceState;
        use inputforge_core::types::{
            AxisValue, DeviceDiagnostics, DeviceId, DeviceInfo, InputId, InputValue,
        };

        let mut state = AppState::new();
        let did = DeviceId("dev-1".to_owned());

        state.devices.push(DeviceState {
            info: DeviceInfo {
                id: did.clone(),
                name: "Joystick".to_owned(),
                axes: 1,
                buttons: 1,
                hats: 1,
                instance_path: None,
                axis_polarities: vec![AxisPolarity::Bipolar],
            },
            connected: true,
            diagnostics: DeviceDiagnostics::default(),
        });
        state.virtual_devices.push(VirtualDeviceConfig {
            device_id: 1,
            axes: vec![VJoyAxis::X],
            button_count: 1,
            hat_count: 1,
        });

        state.input_cache.update(
            &InputAddress::Bound {
                device: did.clone(),
                input: InputId::Axis { index: 0 },
            },
            &InputValue::Axis {
                value: AxisValue::new(0.5),
                polarity: AxisPolarity::Bipolar,
            },
        );
        state.input_cache.update(
            &InputAddress::Bound {
                device: did.clone(),
                input: InputId::Button { index: 0 },
            },
            &InputValue::Button { pressed: true },
        );
        state.input_cache.update(
            &InputAddress::Bound {
                device: did,
                input: InputId::Hat { index: 0 },
            },
            &InputValue::Hat {
                direction: HatDirection::N,
            },
        );

        state.output_cache.set_axis(1, VJoyAxis::X, -0.25);
        state.output_cache.set_button(1, 1, true);
        state.output_cache.set_hat(1, 0, HatDirection::SE);

        let cfg = ConfigSnapshot::from_state(&state, None);
        let live = LiveSnapshot::from_state(&state, &cfg);

        assert_eq!(live.device_inputs.len(), 1);
        assert_eq!(live.device_inputs[0].axes.len(), 1);
        assert!((live.device_inputs[0].axes[0].0 - 0.5).abs() < f64::EPSILON);
        assert_eq!(live.device_inputs[0].axes[0].1, AxisPolarity::Bipolar);
        assert_eq!(live.device_inputs[0].buttons, vec![true]);
        assert_eq!(live.device_inputs[0].hats, vec![HatDirection::N]);

        assert_eq!(live.output_values.len(), 1);
        assert!((live.output_values[0].axes[0].1 - (-0.25)).abs() < f64::EPSILON);
        assert_eq!(live.output_values[0].axes[0].0, VJoyAxis::X);
        assert_eq!(live.output_values[0].buttons, vec![true]);
        assert_eq!(live.output_values[0].hats, vec![HatDirection::SE]);
    }

    #[test]
    fn config_from_state_populates_mapped_inputs_and_names() {
        use inputforge_core::action::Mapping;
        use inputforge_core::mode::ModeTree;
        use inputforge_core::profile::Profile;
        use inputforge_core::types::{DeviceId, InputId};

        let mut map = HashMap::new();
        map.insert("Default".to_owned(), vec![]);
        let modes = ModeTree::from_adjacency(&map).unwrap();

        let device_id = DeviceId("dev-1".to_owned());
        let addr_named = InputAddress::Bound {
            device: device_id.clone(),
            input: InputId::Button { index: 0 },
        };
        let addr_unnamed = InputAddress::Bound {
            device: device_id,
            input: InputId::Button { index: 1 },
        };

        let mappings = vec![
            Mapping {
                input: addr_named.clone(),
                mode: "Default".to_owned(),
                name: Some("Fire".to_owned()),
                actions: vec![],
            },
            Mapping {
                input: addr_unnamed.clone(),
                mode: "Default".to_owned(),
                name: None,
                actions: vec![],
            },
        ];

        let profile = Profile::new(
            "TestProfile".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);

        let cfg = ConfigSnapshot::from_state(&state, None);
        assert_eq!(cfg.mapped_inputs.len(), 2);
        assert!(cfg.mapped_inputs.contains(&addr_named));
        assert!(cfg.mapped_inputs.contains(&addr_unnamed));
        assert_eq!(cfg.mapping_names.len(), 1);
        assert_eq!(cfg.mapping_names.get(&addr_named), Some(&"Fire".to_owned()));
        assert!(!cfg.mapping_names.contains_key(&addr_unnamed));
    }

    #[test]
    fn f1_readout_data_binding_contract() {
        use inputforge_core::state::{AppState, DeviceState, EngineStatus};
        use inputforge_core::types::{
            AxisPolarity, DeviceDiagnostics, DeviceId, DeviceInfo, VJoyAxis, VirtualDeviceConfig,
        };

        let mut s = AppState::new();
        s.engine_status = EngineStatus::Running;
        "Combat".clone_into(&mut s.current_mode);
        s.warnings.push("low battery".to_owned());
        s.devices.push(DeviceState {
            info: DeviceInfo {
                id: DeviceId("dev-1".to_owned()),
                name: "Stick".to_owned(),
                axes: 2,
                buttons: 4,
                hats: 1,
                instance_path: None,
                axis_polarities: vec![AxisPolarity::Bipolar; 2],
            },
            connected: true,
            diagnostics: DeviceDiagnostics::default(),
        });
        s.virtual_devices.push(VirtualDeviceConfig {
            device_id: 1,
            axes: vec![VJoyAxis::X, VJoyAxis::Y],
            button_count: 4,
            hat_count: 1,
        });

        let meta = MetaSnapshot::from_state(&s);
        let cfg = ConfigSnapshot::from_state(&s, None);

        // The exact six snapshot fields the placeholder shell surfaces consume:
        assert_eq!(meta.engine_status, EngineStatus::Running);
        assert_eq!(meta.current_mode, "Combat");
        assert_eq!(meta.profile_name, None); // no profile loaded
        assert_eq!(cfg.devices.len(), 1);
        assert_eq!(cfg.virtual_devices.len(), 1);
        assert_eq!(meta.warnings.len(), 1);
    }

    #[test]
    fn meta_from_state_projects_modes_and_startup_mode() {
        use inputforge_core::mode::ModeTree;
        use inputforge_core::profile::Profile;

        let mut map = HashMap::new();
        map.insert(
            "Default".to_owned(),
            vec!["Combat".to_owned(), "Landing".to_owned()],
        );
        let modes = ModeTree::from_adjacency(&map).unwrap();
        let profile = Profile::new(
            "Modal".to_owned(),
            vec![],
            modes,
            vec![],
            vec![],
            "Combat".to_owned(),
        );
        let state = AppState::with_profile(profile);
        let meta = MetaSnapshot::from_state(&state);
        assert_eq!(meta.modes.len(), 3);
        assert_eq!(meta.modes[0], "Default", "DFS pre-order: root first");
        assert_eq!(meta.startup_mode, Some("Combat".to_owned()));
    }

    #[test]
    fn meta_from_state_with_no_profile_has_empty_modes_and_no_startup() {
        let state = AppState::new();
        let meta = MetaSnapshot::from_state(&state);
        assert!(meta.modes.is_empty());
        assert!(meta.startup_mode.is_none());
    }

    #[test]
    fn config_snapshot_populates_mappings_with_no_glyphs() {
        use inputforge_core::action::Mapping;
        use inputforge_core::mode::ModeTree;
        use inputforge_core::profile::Profile;
        use inputforge_core::types::{DeviceId, InputId};

        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();

        let addr = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 0 },
        };
        let mappings = vec![Mapping {
            input: addr.clone(),
            mode: "Default".to_owned(),
            name: Some("Fire".to_owned()),
            actions: vec![],
        }];

        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);
        let cfg = ConfigSnapshot::from_state(&state, None);

        assert_eq!(cfg.mappings.len(), 1);
        let s = &cfg.mappings[0];
        assert_eq!(s.input, addr);
        assert_eq!(s.mode, "Default");
        assert_eq!(s.name.as_deref(), Some("Fire"));
        assert!(s.glyphs.merge_secondary.is_none());
        assert!(s.glyphs.first_input_predicate.is_none());
    }

    #[test]
    fn config_snapshot_glyph_walker_finds_merge_axis() {
        use inputforge_core::action::{Action, Mapping};
        use inputforge_core::mode::ModeTree;
        use inputforge_core::profile::Profile;
        use inputforge_core::types::{DeviceId, InputId, MergeOp};

        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();

        let primary = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        let secondary = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 1 },
        };

        let mappings = vec![Mapping {
            input: primary.clone(),
            mode: "Default".to_owned(),
            name: None,
            actions: vec![Action::MergeAxis {
                second_input: secondary.clone(),
                operation: MergeOp::Average,
            }],
        }];

        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);
        let cfg = ConfigSnapshot::from_state(&state, None);

        let s = &cfg.mappings[0];
        assert_eq!(s.glyphs.merge_secondary.as_ref(), Some(&secondary));
        assert!(s.glyphs.first_input_predicate.is_none());
    }

    #[test]
    fn config_snapshot_glyph_walker_finds_input_conditional() {
        use inputforge_core::action::{Action, Condition, Mapping};
        use inputforge_core::mode::ModeTree;
        use inputforge_core::profile::Profile;
        use inputforge_core::types::{DeviceId, InputId};

        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();

        let trigger = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 0 },
        };
        let predicate = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 1 },
        };

        let mappings = vec![Mapping {
            input: trigger.clone(),
            mode: "Default".to_owned(),
            name: None,
            actions: vec![Action::Conditional {
                condition: Condition::ButtonPressed {
                    input: predicate.clone(),
                },
                if_true: vec![],
                if_false: Vec::new(),
            }],
        }];

        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);
        let cfg = ConfigSnapshot::from_state(&state, None);

        let s = &cfg.mappings[0];
        assert!(s.glyphs.merge_secondary.is_none());
        assert!(
            s.glyphs.first_input_predicate.is_some(),
            "input-bearing Conditional must populate first_input_predicate"
        );
    }

    #[test]
    fn config_snapshot_glyph_walker_finds_both_glyphs() {
        use inputforge_core::action::{Action, Condition, Mapping};
        use inputforge_core::mode::ModeTree;
        use inputforge_core::profile::Profile;
        use inputforge_core::types::{DeviceId, InputId, MergeOp};

        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();

        let primary = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        let secondary = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 1 },
        };
        let predicate = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 0 },
        };

        let mappings = vec![Mapping {
            input: primary.clone(),
            mode: "Default".to_owned(),
            name: None,
            actions: vec![
                Action::MergeAxis {
                    second_input: secondary.clone(),
                    operation: MergeOp::Average,
                },
                Action::Conditional {
                    condition: Condition::ButtonPressed {
                        input: predicate.clone(),
                    },
                    if_true: vec![],
                    if_false: Vec::new(),
                },
            ],
        }];

        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);
        let cfg = ConfigSnapshot::from_state(&state, None);

        let s = &cfg.mappings[0];
        assert_eq!(s.glyphs.merge_secondary.as_ref(), Some(&secondary));
        assert!(s.glyphs.first_input_predicate.is_some());
    }

    #[test]
    fn config_snapshot_glyph_walker_recurses_through_composite_conditions() {
        use inputforge_core::action::{Action, Condition, Mapping};
        use inputforge_core::mode::ModeTree;
        use inputforge_core::profile::Profile;
        use inputforge_core::types::{DeviceId, InputId};

        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();

        let trigger = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 0 },
        };
        let nested_predicate = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 5 },
        };

        let nested_condition = Condition::Not {
            condition: Box::new(Condition::Any {
                conditions: vec![Condition::All {
                    conditions: vec![Condition::ButtonReleased {
                        input: nested_predicate.clone(),
                    }],
                }],
            }),
        };

        let mappings = vec![Mapping {
            input: trigger.clone(),
            mode: "Default".to_owned(),
            name: None,
            actions: vec![Action::Conditional {
                condition: nested_condition,
                if_true: vec![],
                if_false: Vec::new(),
            }],
        }];

        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);
        let cfg = ConfigSnapshot::from_state(&state, None);

        let s = &cfg.mappings[0];
        assert!(
            s.glyphs.first_input_predicate.is_some(),
            "walker must recurse through Not -> Any -> All -> ButtonReleased"
        );
    }

    #[test]
    fn config_snapshot_glyph_walker_descends_into_nested_actions() {
        use inputforge_core::action::{Action, Condition, Mapping};
        use inputforge_core::mode::ModeTree;
        use inputforge_core::profile::Profile;
        use inputforge_core::types::{DeviceId, InputId, MergeOp};

        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();

        let primary = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        let secondary = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 1 },
        };
        let predicate = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 0 },
        };

        let mappings = vec![Mapping {
            input: primary.clone(),
            mode: "Default".to_owned(),
            name: None,
            actions: vec![Action::Conditional {
                condition: Condition::ButtonPressed {
                    input: predicate.clone(),
                },
                if_true: vec![Action::MergeAxis {
                    second_input: secondary.clone(),
                    operation: MergeOp::Average,
                }],
                if_false: Vec::new(),
            }],
        }];

        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);
        let cfg = ConfigSnapshot::from_state(&state, None);

        let s = &cfg.mappings[0];
        assert_eq!(
            s.glyphs.merge_secondary.as_ref(),
            Some(&secondary),
            "walker must descend into Conditional.if_true to find MergeAxis"
        );
    }

    #[test]
    fn mapping_summary_referenced_devices_dedupes_and_ignores_unbound() {
        use inputforge_core::action::{Action, Condition, Mapping};
        use inputforge_core::mode::ModeTree;
        use inputforge_core::profile::Profile;
        use inputforge_core::state::AppState;
        use inputforge_core::types::{DeviceId, InputAddress, InputId, MergeOp};

        let dev_a = DeviceId("dev-a".to_owned());
        let primary = InputAddress::Bound {
            device: dev_a.clone(),
            input: InputId::Axis { index: 0 },
        };
        let modes =
            ModeTree::from_adjacency(&HashMap::from([("Default".to_owned(), vec![])])).unwrap();
        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            vec![Mapping {
                input: primary.clone(),
                mode: "Default".to_owned(),
                name: None,
                actions: vec![
                    Action::MergeAxis {
                        second_input: InputAddress::Unbound,
                        operation: MergeOp::Average,
                    },
                    Action::Conditional {
                        condition: Condition::ButtonPressed { input: primary },
                        if_true: vec![],
                        if_false: vec![],
                    },
                ],
            }],
            vec![],
            "Default".to_owned(),
        );

        let cfg = ConfigSnapshot::from_state(&AppState::with_profile(profile), None);
        assert_eq!(cfg.mappings[0].referenced_devices, vec![dev_a]);
    }

    #[test]
    fn mapping_summary_finds_first_vjoy_output_in_preorder() {
        use inputforge_core::action::{Action, Condition, Mapping};
        use inputforge_core::mode::ModeTree;
        use inputforge_core::profile::Profile;
        use inputforge_core::state::AppState;
        use inputforge_core::types::{
            DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis,
        };

        let modes =
            ModeTree::from_adjacency(&HashMap::from([("Default".to_owned(), vec![])])).unwrap();
        let input = InputAddress::Bound {
            device: DeviceId("stick".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        let true_output = OutputAddress {
            device: 2,
            output: OutputId::Axis { id: VJoyAxis::Y },
        };
        let false_output = OutputAddress {
            device: 3,
            output: OutputId::Axis { id: VJoyAxis::Z },
        };
        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            vec![Mapping {
                input: input.clone(),
                mode: "Default".to_owned(),
                name: None,
                actions: vec![Action::Conditional {
                    condition: Condition::ButtonPressed { input },
                    if_true: vec![Action::MapToVJoy {
                        output: true_output.clone(),
                    }],
                    if_false: vec![Action::MapToVJoy {
                        output: false_output,
                    }],
                }],
            }],
            vec![],
            "Default".to_owned(),
        );

        let cfg = ConfigSnapshot::from_state(&AppState::with_profile(profile), None);
        assert_eq!(
            cfg.mappings[0].first_vjoy_output.as_ref(),
            Some(&true_output)
        );
    }

    #[test]
    fn config_from_state_with_selection_clones_actions() {
        use inputforge_core::action::{Action, Mapping};
        use inputforge_core::mode::ModeTree;
        use inputforge_core::profile::Profile;
        use inputforge_core::types::{DeviceId, InputId};

        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();

        let addr = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 0 },
        };
        let mappings = vec![Mapping {
            input: addr.clone(),
            mode: "Default".to_owned(),
            name: Some("Fire".to_owned()),
            actions: vec![Action::Invert],
        }];
        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);

        let sel = Some(("Default".to_owned(), addr.clone()));
        let cfg = ConfigSnapshot::from_state(&state, sel.as_ref());

        assert_eq!(cfg.selected_mapping_actions.as_ref().map(Vec::len), Some(1));
        assert_eq!(
            cfg.selected_mapping_key.as_ref(),
            Some(&("Default".to_owned(), addr.clone()))
        );
    }

    #[test]
    fn config_from_state_without_selection_actions_none() {
        let state = AppState::new();
        let cfg = ConfigSnapshot::from_state(&state, None);
        assert!(cfg.selected_mapping_actions.is_none());
        assert!(cfg.selected_mapping_key.is_none());
    }

    #[test]
    fn config_from_state_with_stale_selection_actions_none_key_present() {
        use inputforge_core::types::{DeviceId, InputId};

        let app = AppState::new();
        let stale_sel = Some((
            "Default".to_owned(),
            InputAddress::Bound {
                device: DeviceId("nonexistent".to_owned()),
                input: InputId::Button { index: 99 },
            },
        ));
        let cfg = ConfigSnapshot::from_state(&app, stale_sel.as_ref());
        assert!(cfg.selected_mapping_actions.is_none());
        assert_eq!(cfg.selected_mapping_key, stale_sel);
    }

    fn at(s: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339(s)
            .expect("rfc3339 timestamp")
            .with_timezone(&chrono::Utc)
    }

    #[test]
    fn relative_time_under_one_minute_is_just_now() {
        let now = at("2026-05-07T12:00:00Z");
        let taken = at("2026-05-07T11:59:30Z");
        assert_eq!(format_relative_at(taken, now), "just now");
    }

    #[test]
    fn relative_time_under_one_hour_is_minutes_ago() {
        let now = at("2026-05-07T12:00:00Z");
        assert_eq!(
            format_relative_at(at("2026-05-07T11:55:00Z"), now),
            "5m ago"
        );
        assert_eq!(
            format_relative_at(at("2026-05-07T11:00:30Z"), now),
            "59m ago"
        );
    }

    #[test]
    fn relative_time_under_one_day_is_hours_ago() {
        let now = at("2026-05-07T12:00:00Z");
        assert_eq!(
            format_relative_at(at("2026-05-07T10:00:00Z"), now),
            "2h ago"
        );
        assert_eq!(
            format_relative_at(at("2026-05-06T12:30:00Z"), now),
            "23h ago"
        );
    }

    #[test]
    fn relative_time_under_one_week_is_days_ago() {
        let now = at("2026-05-07T12:00:00Z");
        assert_eq!(
            format_relative_at(at("2026-05-05T12:00:00Z"), now),
            "2d ago"
        );
        assert_eq!(
            format_relative_at(at("2026-05-01T12:00:00Z"), now),
            "6d ago"
        );
    }

    #[test]
    fn relative_time_one_week_or_older_falls_back_to_iso_date() {
        let now = at("2026-05-07T12:00:00Z");
        assert_eq!(
            format_relative_at(at("2026-04-30T12:00:00Z"), now),
            "2026-04-30"
        );
    }

    #[test]
    fn relative_time_negative_delta_collapses_to_iso_date() {
        // Clock skew: snapshot timestamp is in the future relative to
        // `now`. We avoid showing "in 2m" (jarring); we show the date.
        let now = at("2026-05-07T12:00:00Z");
        assert_eq!(
            format_relative_at(at("2026-05-07T12:02:00Z"), now),
            "2026-05-07"
        );
    }

    #[test]
    fn absolute_time_drops_subseconds_and_offset_string() {
        let taken = at("2026-05-07T14:42:59.659262500Z");
        assert_eq!(format_absolute_time(taken), "2026-05-07 14:42 UTC");
    }

    #[test]
    fn device_display_name_returns_alias_when_present() {
        let mut state = AppState::new();
        state.devices.push(DeviceState {
            info: test_device("dev-live", "Generic Joystick", 4, 16, 0),
            connected: true,
            diagnostics: DeviceDiagnostics::default(),
        });
        state
            .device_aliases
            .insert(DeviceId("dev-live".to_owned()), "Rig Wheel".to_owned());
        let snapshot = ConfigSnapshot::from_state(&state, None);
        assert_eq!(
            snapshot.device_display_name(&DeviceId("dev-live".to_owned())),
            "Rig Wheel"
        );
    }

    #[test]
    fn device_display_name_falls_back_to_hardware_name_when_alias_blank() {
        let mut state = AppState::new();
        state.devices.push(DeviceState {
            info: test_device("dev-1", "Generic HID Joystick", 0, 0, 0),
            connected: true,
            diagnostics: DeviceDiagnostics::default(),
        });
        state
            .device_aliases
            .insert(DeviceId("dev-1".to_owned()), "   ".to_owned());
        let snapshot = ConfigSnapshot::from_state(&state, None);
        assert_eq!(
            snapshot.device_display_name(&DeviceId("dev-1".to_owned())),
            "Generic HID Joystick"
        );
    }

    #[test]
    fn device_display_name_returns_alias_for_remembered_disconnected_device() {
        let mut state = AppState::new();
        state.device_registry.insert(
            DeviceId("dev-old".to_owned()),
            inputforge_core::settings::DeviceRecord {
                info: test_device("dev-old", "Old Pedals", 0, 0, 0),
                diagnostics: DeviceDiagnostics::default(),
                last_seen_unix_ms: Some(1),
            },
        );
        state
            .device_aliases
            .insert(DeviceId("dev-old".to_owned()), "Track Pedals".to_owned());
        // Not in s.devices: the device is remembered but disconnected.
        let snapshot = ConfigSnapshot::from_state(&state, None);
        assert_eq!(
            snapshot.device_display_name(&DeviceId("dev-old".to_owned())),
            "Track Pedals"
        );
    }

    #[test]
    fn device_display_name_returns_id_for_unknown_device() {
        let state = AppState::new();
        let snapshot = ConfigSnapshot::from_state(&state, None);
        // Snapshot has not seen this device anywhere; fall back to id.
        assert_eq!(
            snapshot.device_display_name(&DeviceId("ghost".to_owned())),
            "ghost"
        );
    }

    #[test]
    fn device_display_name_connected_device_overrides_registry_record() {
        // Same DeviceId in both `device_registry` (with stale name)
        // and `s.devices` (live with current name). The connected
        // entry must win on collision so the user sees the
        // up-to-date hardware identity, not the stale one.
        let mut state = AppState::new();
        state.devices.push(DeviceState {
            info: test_device("dev-1", "Live Stick", 0, 0, 0),
            connected: true,
            diagnostics: DeviceDiagnostics::default(),
        });
        state.device_registry.insert(
            DeviceId("dev-1".to_owned()),
            inputforge_core::settings::DeviceRecord {
                info: test_device("dev-1", "Stale Stick", 0, 0, 0),
                diagnostics: DeviceDiagnostics::default(),
                last_seen_unix_ms: Some(1),
            },
        );
        let snapshot = ConfigSnapshot::from_state(&state, None);
        assert_eq!(
            snapshot.device_display_name(&DeviceId("dev-1".to_owned())),
            "Live Stick"
        );
    }

    #[test]
    fn settings_snapshot_default_is_zero_count() {
        let snap = SettingsSnapshot::default();
        assert_eq!(snap.unpinned_snapshot_count, 0);
    }

    #[test]
    fn settings_snapshot_from_state_no_profile_yields_zero_count() {
        use inputforge_core::state::AppState;
        let state = AppState::new();
        let snap = SettingsSnapshot::from_state(&state);
        assert_eq!(
            snap.unpinned_snapshot_count, 0,
            "no profile loaded must yield 0 unpinned"
        );
        assert_eq!(snap.snapshot, state.snapshot_config);
    }

    #[test]
    fn settings_snapshot_from_state_mirrors_snapshot_config_field() {
        use inputforge_core::snapshot::SnapshotConfig;
        use inputforge_core::state::AppState;

        let mut state = AppState::new();
        state.snapshot_config = SnapshotConfig {
            max_count: 42,
            skip_if_unchanged: false,
        };

        let snap = SettingsSnapshot::from_state(&state);
        assert_eq!(snap.snapshot.max_count, 42);
        assert!(!snap.snapshot.skip_if_unchanged);
        assert_eq!(snap.unpinned_snapshot_count, 0);
    }

    #[test]
    fn settings_snapshot_from_state_counts_unpinned_active_rows() {
        use chrono::DateTime;
        use inputforge_core::snapshot::{SnapshotId, SnapshotKind};
        use inputforge_core::state::{ActiveSnapshotRow, AppState};
        use ulid::Ulid;

        fn row(pinned: bool) -> ActiveSnapshotRow {
            ActiveSnapshotRow {
                id: SnapshotId(Ulid::nil()),
                kind: SnapshotKind::Manual,
                label: None,
                taken_at: DateTime::from_timestamp(0, 0).unwrap(),
                pinned,
            }
        }

        let mut state = AppState::new();
        state.active_snapshot_rows = vec![row(true), row(false), row(true), row(false)];

        let snap = SettingsSnapshot::from_state(&state);
        assert_eq!(
            snap.unpinned_snapshot_count, 2,
            "should count only unpinned rows"
        );
    }

    #[test]
    fn settings_snapshot_from_state_mirrors_startup() {
        use inputforge_core::settings::StartupSettings;
        use inputforge_core::state::AppState;

        let mut state = AppState::new();
        state.startup = StartupSettings {
            launch_at_startup: true,
            start_minimized_to_tray: true,
        };
        let snap = SettingsSnapshot::from_state(&state);
        assert_eq!(snap.startup, state.startup);
    }
}
