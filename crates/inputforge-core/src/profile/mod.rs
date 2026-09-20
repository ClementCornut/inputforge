// Rust guideline compliant 2026-03-06

pub mod controllers;
pub mod library;
pub mod manager;
mod reconcile;
mod types;

#[doc(inline)]
pub use library::{
    LibraryProfile, add_external_profile_to_library, duplicate_library_profile,
    rename_library_profile,
};
pub use types::{CalibrationEntry, DeviceEntry, ProfileId, ProfileSettings};

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::action::{Action, Mapping, validate_gesture_threshold_ms};
use crate::error::{EngineError, Result};
use crate::mode::Modes;
use crate::types::{InputAddress, InputId};

// Keeps authored action trees bounded enough for validation and UI traversal.
const MAX_ACTION_BRANCH_DEPTH: usize = 8;

#[derive(Debug)]
struct MigratedProfileToml {
    value: toml::Value,
    changed: bool,
}

fn migrate_profile_toml(input: &str) -> Result<MigratedProfileToml> {
    let mut value: toml::Value = toml::from_str(input)?;
    let changed = migrate_profile_value(&mut value)?;
    Ok(MigratedProfileToml { value, changed })
}

fn migrate_profile_value(value: &mut toml::Value) -> Result<bool> {
    let Some(mappings) = value
        .get_mut("mappings")
        .and_then(toml::Value::as_array_mut)
    else {
        return Ok(false);
    };

    let mut changed = false;
    for mapping in mappings {
        if let Some(actions) = mapping.get_mut("actions") {
            changed |= migrate_actions(actions)?;
        }
    }
    Ok(changed)
}

fn migrate_actions(actions: &mut toml::Value) -> Result<bool> {
    let Some(actions) = actions.as_array_mut() else {
        return Ok(false);
    };

    let mut changed = false;
    for action in actions {
        changed |= migrate_action(action)?;
    }
    Ok(changed)
}

fn migrate_action(action: &mut toml::Value) -> Result<bool> {
    let Some(action) = action.as_table_mut() else {
        return Ok(false);
    };

    let mut changed = false;
    if action.get("type").and_then(toml::Value::as_str) == Some("map_to_keyboard")
        && let Some(key) = action.get_mut("key")
    {
        changed |= migrate_keyboard_combo(key)?;
    }

    if action.get("type").and_then(toml::Value::as_str) == Some("conditional") {
        if let Some(if_true) = action.get_mut("if_true") {
            changed |= migrate_actions(if_true)?;
        }
        if let Some(if_false) = action.get_mut("if_false") {
            changed |= migrate_actions(if_false)?;
        }
    }

    Ok(changed)
}

fn migrate_keyboard_combo(combo: &mut toml::Value) -> Result<bool> {
    let Some(combo) = combo.as_table_mut() else {
        return Ok(false);
    };

    let mut changed = false;
    if let Some(key) = combo.get_mut("key") {
        changed |= migrate_keyboard_key(key)?;
    }
    if let Some(modifiers) = combo
        .get_mut("modifiers")
        .and_then(toml::Value::as_array_mut)
    {
        for modifier in modifiers {
            changed |= migrate_keyboard_modifier(modifier);
        }
    }

    Ok(changed)
}

fn migrate_keyboard_key(key: &mut toml::Value) -> Result<bool> {
    let Some(name) = key.as_str() else {
        return Ok(false);
    };
    let Some(migrated) = legacy_keyboard_key_name(name)? else {
        return Ok(false);
    };

    *key = toml::Value::String(migrated.to_owned());
    Ok(true)
}

fn migrate_keyboard_modifier(modifier: &mut toml::Value) -> bool {
    let migrated = match modifier.as_str() {
        Some("Ctrl") => "ControlLeft",
        Some("Shift") => "ShiftLeft",
        Some("Alt") => "AltLeft",
        Some("Win") => "MetaLeft",
        _ => return false,
    };

    *modifier = toml::Value::String(migrated.to_owned());
    true
}

fn legacy_keyboard_key_name(name: &str) -> Result<Option<&'static str>> {
    if name.len() == 1 {
        let byte = name.as_bytes()[0];
        if byte.is_ascii_alphabetic() {
            return Ok(Some(match byte.to_ascii_uppercase() {
                b'A' => "KeyA",
                b'B' => "KeyB",
                b'C' => "KeyC",
                b'D' => "KeyD",
                b'E' => "KeyE",
                b'F' => "KeyF",
                b'G' => "KeyG",
                b'H' => "KeyH",
                b'I' => "KeyI",
                b'J' => "KeyJ",
                b'K' => "KeyK",
                b'L' => "KeyL",
                b'M' => "KeyM",
                b'N' => "KeyN",
                b'O' => "KeyO",
                b'P' => "KeyP",
                b'Q' => "KeyQ",
                b'R' => "KeyR",
                b'S' => "KeyS",
                b'T' => "KeyT",
                b'U' => "KeyU",
                b'V' => "KeyV",
                b'W' => "KeyW",
                b'X' => "KeyX",
                b'Y' => "KeyY",
                b'Z' => "KeyZ",
                _ => unreachable!("checked is_ascii_alphabetic"),
            }));
        }
        if byte.is_ascii_digit() {
            return Ok(Some(match byte {
                b'0' => "Digit0",
                b'1' => "Digit1",
                b'2' => "Digit2",
                b'3' => "Digit3",
                b'4' => "Digit4",
                b'5' => "Digit5",
                b'6' => "Digit6",
                b'7' => "Digit7",
                b'8' => "Digit8",
                b'9' => "Digit9",
                _ => unreachable!("checked is_ascii_digit"),
            }));
        }
    }

    if let Some(suffix) = name.strip_prefix('F')
        && !suffix.is_empty()
        && suffix.bytes().all(|b| b.is_ascii_digit())
    {
        let canonical = suffix.parse::<u32>().ok().and_then(|n| match n {
            1 => Some("F1"),
            2 => Some("F2"),
            3 => Some("F3"),
            4 => Some("F4"),
            5 => Some("F5"),
            6 => Some("F6"),
            7 => Some("F7"),
            8 => Some("F8"),
            9 => Some("F9"),
            10 => Some("F10"),
            11 => Some("F11"),
            12 => Some("F12"),
            _ => None,
        });
        return match canonical {
            Some(canon) => Ok(Some(canon)),
            None => Err(EngineError::InvalidConfig {
                reason: format!("unsupported legacy keyboard key: {name}"),
            }),
        };
    }

    Ok(match name {
        "Space" => Some("Space"),
        "Enter" | "Return" => Some("Enter"),
        "Tab" => Some("Tab"),
        "Escape" | "Esc" => Some("Escape"),
        "Backspace" => Some("Backspace"),
        "Delete" | "Del" => Some("Delete"),
        "Insert" => Some("Insert"),
        "Up" => Some("ArrowUp"),
        "Down" => Some("ArrowDown"),
        "Left" => Some("ArrowLeft"),
        "Right" => Some("ArrowRight"),
        "Home" => Some("Home"),
        "End" => Some("End"),
        "PageUp" | "PgUp" => Some("PageUp"),
        "PageDown" | "PgDn" => Some("PageDown"),
        _ => None,
    })
}

/// Discriminator for `reorder_mapping_in_group` that mirrors the GUI's
/// visual bucketing of inputs (Axes / Buttons / Hats). Engine-side
/// reorder operates within one of these buckets at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputGroup {
    Axis,
    Button,
    Hat,
}

fn group_of_input(addr: &InputAddress) -> InputGroup {
    let input = addr
        .input_id()
        .expect("invariant: mapping primaries are always Bound (set_mapping / add_inline / F8 capture); group_of_input only called on Profile::mappings entries");
    match input {
        InputId::Axis { .. } => InputGroup::Axis,
        InputId::Button { .. } => InputGroup::Button,
        InputId::Hat { .. } => InputGroup::Hat,
    }
}

/// A complete input mapping profile.
///
/// Profiles are persisted as TOML files. The file structure matches:
///
/// ```toml
/// modes = ["Default", "Combat", "Landing", "Missiles", "Guns"]
///
/// [profile]
/// id = "550e8400-e29b-41d4-a716-446655440000"
/// name = "My Profile"
/// startup_mode = "Default"
///
/// [[devices]]
/// id = "030000005e040000ea02000000007801"
/// name = "VKB Gladiator NXT Left"
///
/// [[mappings]]
/// mode = "Default"
/// # ...
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    id: ProfileId,
    name: String,
    devices: Vec<DeviceEntry>,
    modes: Modes,
    mappings: Vec<Mapping>,
    calibrations: Vec<CalibrationEntry>,
    settings: ProfileSettings,
    controllers: Option<controllers::ControllerConfig>,
}

/// Internal TOML-level representation of the `[profile]` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProfileMeta {
    id: ProfileId,
    name: String,
    startup_mode: String,
}

/// Internal TOML-level representation of the full file.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProfileRaw {
    modes: Modes,
    profile: ProfileMeta,
    #[serde(default)]
    devices: Vec<DeviceEntry>,
    #[serde(default)]
    mappings: Vec<Mapping>,
    #[serde(default)]
    calibrations: Vec<CalibrationEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    controllers: Option<controllers::ControllerConfig>,
    #[serde(default, skip_serializing)]
    linux: Option<controllers::ControllerConfig>,
}

impl Profile {
    /// Create a new profile with a generated ID.
    #[must_use]
    pub fn new(
        name: String,
        devices: Vec<DeviceEntry>,
        modes: Modes,
        mappings: Vec<Mapping>,
        calibrations: Vec<CalibrationEntry>,
        startup_mode: String,
    ) -> Self {
        Self {
            id: ProfileId::new(),
            name,
            devices,
            modes,
            mappings,
            calibrations,
            settings: ProfileSettings { startup_mode },
            controllers: None,
        }
    }

    /// Parse a profile from a TOML string.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::ProfileParse`] on invalid TOML, or
    /// [`EngineError::InvalidConfig`] if validation fails (e.g., startup
    /// mode not in modes, mapping references unknown mode).
    pub fn from_toml(s: &str) -> Result<Self> {
        let migrated = migrate_profile_toml(s)?;
        let raw: ProfileRaw = migrated.value.try_into()?;
        Self::from_raw(raw)
    }

    /// Serialize the profile to a TOML string.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::ProfileWrite`] on serialization failure.
    pub fn to_toml(&self) -> Result<String> {
        validate_profile_actions(self)?;
        let raw = self.to_raw();
        Ok(toml::to_string(&raw)?)
    }

    /// Load a profile from a TOML file.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::ProfileNotFound`] if the file does not
    /// exist, [`EngineError::Io`] on read errors, or parse/validation
    /// errors.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(EngineError::ProfileNotFound {
                path: path.to_path_buf(),
            });
        }
        let contents = std::fs::read_to_string(path)?;
        let migrated = migrate_profile_toml(&contents)?;
        let raw: ProfileRaw = migrated.value.try_into()?;
        let profile = Self::from_raw(raw)?;
        if migrated.changed {
            profile.save(path)?;
        }
        Ok(profile)
    }

    /// Save the profile to a TOML file.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::ProfileWrite`] on serialization failure,
    /// or [`EngineError::Io`] on write errors.
    pub fn save(&self, path: &Path) -> Result<()> {
        let toml_str = self.to_toml()?;
        std::fs::write(path, toml_str)?;
        Ok(())
    }

    #[must_use]
    pub fn controllers(&self) -> Option<&controllers::ControllerConfig> {
        self.controllers.as_ref()
    }

    /// Replace Linux configuration after validation. Does not acquire hardware.
    /// # Errors
    /// Returns an error for an invalid native control table.
    pub fn set_controllers(&mut self, linux: controllers::ControllerConfig) -> Result<()> {
        linux.validate()?;
        self.controllers = Some(linux);
        Ok(())
    }

    /// Return the profile ID.
    #[must_use]
    pub fn id(&self) -> &ProfileId {
        &self.id
    }

    /// Return the profile name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the device entries.
    #[must_use]
    pub fn devices(&self) -> &[DeviceEntry] {
        &self.devices
    }

    /// Return the modes.
    #[must_use]
    pub fn modes(&self) -> &Modes {
        &self.modes
    }

    /// Return the mappings.
    #[must_use]
    pub fn mappings(&self) -> &[Mapping] {
        &self.mappings
    }

    /// Return the profile settings.
    #[must_use]
    pub fn settings(&self) -> &ProfileSettings {
        &self.settings
    }

    /// Return the calibration entries.
    #[must_use]
    pub fn calibrations(&self) -> &[CalibrationEntry] {
        &self.calibrations
    }

    /// Find a mapping by input address and mode.
    #[must_use]
    pub fn find_mapping(&self, input: &InputAddress, mode: &str) -> Option<&Mapping> {
        self.mappings
            .iter()
            .find(|m| m.input == *input && m.mode == mode)
    }

    /// Set or update a mapping for a specific input and mode.
    ///
    /// Empty `actions` is a valid mapping (e.g., a freshly added entry
    /// awaiting an action editor in F9). Use `remove_mapping` to delete.
    pub fn set_mapping(
        &mut self,
        input: &InputAddress,
        mode: &str,
        name: Option<String>,
        actions: Vec<Action>,
    ) {
        if let Some(existing) = self
            .mappings
            .iter_mut()
            .find(|m| m.input == *input && m.mode == mode)
        {
            existing.name = name;
            existing.actions = actions;
        } else {
            self.mappings.push(Mapping {
                name,
                input: input.clone(),
                mode: mode.to_owned(),
                actions,
            });
        }
    }

    /// Apply a batch of upserts in a single in-memory pass.
    ///
    /// Each entry produces a mapping with `name: None` and exactly one
    /// `Action::MapToVJoy { output: entry.output }`. Existing mappings
    /// for `(entry.input, entry.mode)` are replaced. Empty `entries`
    /// is a no-op.
    ///
    /// **No file save.** The engine handler is responsible for
    /// persistence; see `EngineCommand::SetMappingsBulk`.
    pub fn set_mappings_bulk(&mut self, entries: &[crate::action::BulkMapEntry]) {
        for entry in entries {
            if matches!(entry.input, InputAddress::Unbound) {
                continue;
            }
            let actions = vec![Action::MapToVJoy {
                output: entry.output.clone(),
            }];
            self.set_mapping(&entry.input, &entry.mode, None, actions);
        }
    }

    /// Remove the mapping for `(input, mode)`. Returns `true` if a mapping
    /// was removed, `false` if no matching mapping existed.
    pub fn remove_mapping(&mut self, input: &InputAddress, mode: &str) -> bool {
        let before = self.mappings.len();
        self.mappings
            .retain(|m| !(m.input == *input && m.mode == mode));
        self.mappings.len() != before
    }

    /// Move the mapping identified by `(input, mode)` to position
    /// `target_index_in_group` within its visual group (Axes / Buttons /
    /// Hats), preserving the relative order of every other mapping.
    ///
    /// The "group" is the bucket the GUI's `mapping_list::group_of`
    /// classifies an input into: `InputId::Axis` -> Axes, `Button` ->
    /// Buttons, `Hat` -> Hats. Reorder is *within-group only*; the
    /// mode partition is also respected (each mode has its own slice
    /// of mappings, and reorder never crosses modes either).
    ///
    /// Returns `true` if the call resulted in a reorder, `false` for any
    /// no-op: source not found, target equals current position, or the
    /// group has fewer than two elements. Out-of-bounds `target_index_in_group`
    /// is clamped to `group_len - 1`. Cross-group attempts are not surfaced
    /// here because the engine command receives only the source key,
    /// the GUI is responsible for rejecting cross-group drops before
    /// dispatching.
    ///
    /// # Panics
    ///
    /// Never panics on user input. Carries an `expect` that asserts a
    /// freshly-located source index appears in its own group's index
    /// list, which is structurally guaranteed by construction; the panic
    /// would only fire on internal logic regression.
    pub fn reorder_mapping_in_group(
        &mut self,
        input: &InputAddress,
        mode: &str,
        target_index_in_group: usize,
    ) -> bool {
        let Some(source_idx) = self
            .mappings
            .iter()
            .position(|m| m.input == *input && m.mode == mode)
        else {
            return false;
        };
        let source_group = group_of_input(&self.mappings[source_idx].input);

        // Collect flat-vec indices in (mode, source_group), preserving order.
        let group_indices: Vec<usize> = self
            .mappings
            .iter()
            .enumerate()
            .filter(|(_, m)| m.mode == mode && group_of_input(&m.input) == source_group)
            .map(|(i, _)| i)
            .collect();
        let group_len = group_indices.len();
        if group_len < 2 {
            return false;
        }
        let source_subpos = group_indices
            .iter()
            .position(|&i| i == source_idx)
            .expect("source must appear in its own group's index list");
        let target_subpos = target_index_in_group.min(group_len - 1);
        if target_subpos == source_subpos {
            return false;
        }

        // After Vec::remove(source_idx), every flat index greater than
        // source_idx shifts down by 1. The target's flat-vec position
        // before the move is group_indices[target_subpos]; the
        // post-removal flat is the same minus 1 if it was past the
        // source. Inserting at that flat puts source at the target's
        // position when moving up; for moving down, source needs to
        // land one slot to the right so it follows the target instead
        // of preceding it.
        let source = self.mappings.remove(source_idx);
        let target_flat_before = group_indices[target_subpos];
        let target_flat_after_removal = if source_idx < target_flat_before {
            target_flat_before - 1
        } else {
            target_flat_before
        };
        let insert_flat = if target_subpos > source_subpos {
            target_flat_after_removal + 1
        } else {
            target_flat_after_removal
        };
        self.mappings.insert(insert_flat, source);
        true
    }

    /// Replace the calibration entries.
    pub fn set_calibrations(&mut self, entries: Vec<CalibrationEntry>) {
        self.calibrations = entries;
    }

    /// Replace the modes wholesale.
    ///
    /// Caller is responsible for ensuring the new list is consistent with
    /// `settings().startup_mode()` and any mode names referenced by mappings
    /// or action graphs. Engine handlers do this validation before calling.
    pub fn set_modes(&mut self, modes: Modes) {
        self.modes = modes;
    }

    /// Set the profile's startup mode.
    ///
    /// Caller must validate that `mode` exists in the profile's modes.
    pub fn set_startup_mode(&mut self, mode: String) {
        self.settings.set_startup_mode(mode);
    }

    /// Drop every mapping whose `mode` field equals `mode`.
    ///
    /// Returns the count of mappings removed; used by tracing events and the
    /// destructive-confirm dialog's affected-mappings count.
    ///
    /// Infallible by contract, `DeleteMode` invokes this in a loop after the
    /// mode-list mutation has already been applied, and a partial cascade would
    /// leave the profile in an inconsistent state. The signature must remain
    /// `usize`, never `Result<usize, _>`.
    pub fn remove_mappings_for_mode(&mut self, mode: &str) -> usize {
        let before = self.mappings.len();
        self.mappings.retain(|m| m.mode != mode);
        before - self.mappings.len()
    }

    /// Rewrite every mode-name reference where `mode == from` to `to` across
    /// all mappings, action graphs, and `ProfileSettings::startup_mode`.
    ///
    /// Returns the count of mappings whose `mode` field or action graph was
    /// touched (a single mapping is counted at most once). The
    /// `startup_mode` rewrite is **not** counted in the return value, it
    /// is a settings-level field, not a mapping. `Mapping.name` (a human
    /// label) is intentionally **not** rewritten; user-authored prose is
    /// preserved across renames.
    ///
    /// Caller (`RenameMode` handler) composes this with
    /// `Modes::with_renamed` and `set_modes` for the full cascade.
    pub fn rename_mode_refs(&mut self, from: &str, to: &str) -> usize {
        if from == to {
            return 0;
        }
        let mut touched = 0usize;
        for mapping in &mut self.mappings {
            let mut mapping_touched = false;
            if mapping.mode == from {
                to.clone_into(&mut mapping.mode);
                mapping_touched = true;
            }
            for action in &mut mapping.actions {
                mapping_touched |= rewrite_mode_in_action(action, from, to);
            }
            if mapping_touched {
                touched += 1;
            }
        }

        if self.settings.startup_mode() == from {
            self.settings.set_startup_mode(to.to_owned());
        }

        touched
    }

    /// Update the profile display name.
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Validate and convert from the raw TOML representation.
    fn from_raw(mut raw: ProfileRaw) -> Result<Self> {
        if raw.controllers.is_none() {
            raw.controllers = raw.linux.take().map(|mut config| {
                config.legacy_axis_settings = true;
                config
            });
        }
        if let Some(linux) = &raw.controllers {
            linux.validate()?;
        }
        // Validate startup_mode exists in the modes.
        if !raw.modes.contains(&raw.profile.startup_mode) {
            return Err(EngineError::InvalidConfig {
                reason: format!(
                    "startup_mode '{}' not found in modes",
                    raw.profile.startup_mode
                ),
            });
        }

        // Validate all mapping mode names exist in the modes.
        for mapping in &raw.mappings {
            if !raw.modes.contains(&mapping.mode) {
                return Err(EngineError::InvalidConfig {
                    reason: format!("mapping references unknown mode '{}'", mapping.mode),
                });
            }
        }

        for mapping in &raw.mappings {
            validate_mapping_actions(mapping)?;
        }

        // Validate all calibration entries.
        for entry in &raw.calibrations {
            entry
                .to_calibration()
                .map_err(|e| EngineError::InvalidConfig {
                    reason: format!(
                        "invalid calibration for device '{}' axis {}: {e}",
                        entry.device.0, entry.axis
                    ),
                })?;
        }

        Ok(Self {
            controllers: raw.controllers,
            id: raw.profile.id,
            name: raw.profile.name,
            devices: raw.devices,
            modes: raw.modes,
            mappings: raw.mappings,
            calibrations: raw.calibrations,
            settings: ProfileSettings {
                startup_mode: raw.profile.startup_mode,
            },
        })
    }

    /// Convert to the raw TOML representation for serialization.
    fn to_raw(&self) -> ProfileRaw {
        ProfileRaw {
            linux: None,
            controllers: self.controllers.clone(),
            modes: self.modes.clone(),
            profile: ProfileMeta {
                id: self.id.clone(),
                name: self.name.clone(),
                startup_mode: self.settings.startup_mode.clone(),
            },
            devices: self.devices.clone(),
            mappings: self.mappings.clone(),
            calibrations: self.calibrations.clone(),
        }
    }
}

fn validate_profile_actions(profile: &Profile) -> Result<()> {
    for mapping in profile.mappings() {
        validate_mapping_actions(mapping)?;
    }
    Ok(())
}

fn validate_mapping_actions(mapping: &Mapping) -> Result<()> {
    validate_actions_for_mapping(&mapping.actions, &mapping.input, 0, false)
}

fn validate_actions_for_mapping(
    actions: &[Action],
    mapping_input: &InputAddress,
    depth: usize,
    inside_gesture: bool,
) -> Result<()> {
    if depth > MAX_ACTION_BRANCH_DEPTH {
        return Err(EngineError::InvalidConfig {
            reason: format!("action branch depth exceeds maximum of {MAX_ACTION_BRANCH_DEPTH}"),
        });
    }

    for action in actions {
        match action {
            Action::Conditional {
                if_true, if_false, ..
            } => {
                validate_actions_for_mapping(if_true, mapping_input, depth + 1, inside_gesture)?;
                validate_actions_for_mapping(if_false, mapping_input, depth + 1, inside_gesture)?;
            }
            Action::TapGesture {
                threshold_ms,
                single_tap,
                double_tap,
                ..
            } => {
                if inside_gesture {
                    return Err(nested_gesture_error());
                }
                validate_gesture_threshold_ms(*threshold_ms)?;
                validate_gesture_mapping_shape(mapping_input)?;
                validate_actions_for_mapping(single_tap, mapping_input, depth + 1, true)?;
                validate_actions_for_mapping(double_tap, mapping_input, depth + 1, true)?;
            }
            Action::PressGesture {
                threshold_ms,
                short_press,
                long_press,
                ..
            } => {
                if inside_gesture {
                    return Err(nested_gesture_error());
                }
                validate_gesture_threshold_ms(*threshold_ms)?;
                validate_gesture_mapping_shape(mapping_input)?;
                validate_actions_for_mapping(short_press, mapping_input, depth + 1, true)?;
                validate_actions_for_mapping(long_press, mapping_input, depth + 1, true)?;
            }
            Action::ResponseCurve { .. }
            | Action::Deadzone { .. }
            | Action::Invert
            | Action::MapToVJoy { .. }
            | Action::MapToKeyboard { .. }
            | Action::MapToMouse { .. }
            | Action::MergeAxis { .. }
            | Action::ChangeMode { .. } => {}
        }
    }
    Ok(())
}

fn nested_gesture_error() -> EngineError {
    EngineError::InvalidConfig {
        reason: "gesture stages cannot be nested inside another gesture branch".to_owned(),
    }
}

/// Validate an action tree against a mapping input without mutating the profile.
///
/// Used by engine command handlers to reject malformed mappings before
/// touching in-memory state. Mirrors the validation performed by
/// [`Profile::to_toml`] on save.
///
/// # Errors
///
/// Returns [`EngineError::InvalidConfig`] when thresholds are out of range,
/// when gesture stages target a non-button mapping, when gestures are nested,
/// or when the action tree exceeds the supported depth.
pub fn validate_mapping_action_tree(
    mapping_input: &InputAddress,
    actions: &[Action],
) -> Result<()> {
    validate_actions_for_mapping(actions, mapping_input, 0, false)
}

fn validate_gesture_mapping_shape(mapping_input: &InputAddress) -> Result<()> {
    if mapping_input.is_button_shaped() {
        return Ok(());
    }

    Err(EngineError::InvalidConfig {
        reason: "gesture stages require a button mapping".to_owned(),
    })
}

/// Walk an action graph in place, rewriting every `from` mode-name reference
/// to `to`. Returns whether any rewrite happened.
fn rewrite_mode_in_action(action: &mut Action, from: &str, to: &str) -> bool {
    use crate::action::ModeChangeStrategy as M;
    match action {
        Action::ChangeMode {
            strategy: M::SwitchTo { mode } | M::Temporary { mode },
        } if mode == from => {
            to.clone_into(mode);
            true
        }
        Action::Conditional {
            if_true, if_false, ..
        } => {
            let mut changed = false;
            for a in if_true {
                changed |= rewrite_mode_in_action(a, from, to);
            }
            for a in if_false {
                changed |= rewrite_mode_in_action(a, from, to);
            }
            changed
        }
        Action::TapGesture {
            single_tap,
            double_tap,
            ..
        } => {
            let mut changed = false;
            for a in single_tap {
                changed |= rewrite_mode_in_action(a, from, to);
            }
            for a in double_tap {
                changed |= rewrite_mode_in_action(a, from, to);
            }
            changed
        }
        Action::PressGesture {
            short_press,
            long_press,
            ..
        } => {
            let mut changed = false;
            for a in short_press {
                changed |= rewrite_mode_in_action(a, from, to);
            }
            for a in long_press {
                changed |= rewrite_mode_in_action(a, from, to);
            }
            changed
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{Condition, ModeChangeStrategy, OutputBehavior};
    use crate::processing::DeadzoneConfig;
    use crate::types::{
        DeviceId, KeyCombo, KeyModifier, MergeOp, OutputAddress, OutputId, PhysicalKey, VJoyAxis,
    };
    fn test_profile_with_one_mode() -> Profile {
        let modes = Modes::new(vec!["Default".to_owned()]).unwrap();
        Profile::new(
            "T".to_owned(),
            vec![],
            modes,
            vec![],
            vec![],
            "Default".to_owned(),
        )
    }

    fn test_profile_with_two_modes() -> Profile {
        let modes = Modes::new(vec!["Default".to_owned(), "Combat".to_owned()]).unwrap();
        Profile::new(
            "T".to_owned(),
            vec![],
            modes,
            vec![],
            vec![],
            "Default".to_owned(),
        )
    }

    fn test_modes() -> Modes {
        Modes::new(vec![
            "Default".to_owned(),
            "Combat".to_owned(),
            "Landing".to_owned(),
            "Missiles".to_owned(),
            "Guns".to_owned(),
        ])
        .unwrap()
    }

    fn test_input() -> InputAddress {
        InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        }
    }

    fn test_output() -> OutputAddress {
        OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::X },
        }
    }

    fn minimal_profile() -> Profile {
        let modes = Modes::new(vec!["Default".to_owned()]).unwrap();

        Profile::new(
            "Test Profile".to_owned(),
            vec![DeviceEntry {
                id: DeviceId("dev-1".to_owned()),
                name: "Test Stick".to_owned(),
            }],
            modes,
            vec![Mapping {
                input: test_input(),
                mode: "Default".to_owned(),
                name: None,
                actions: vec![Action::Invert],
            }],
            vec![],
            "Default".to_owned(),
        )
    }

    // --- Roundtrip ---

    #[test]
    fn profile_toml_roundtrip() {
        let profile = minimal_profile();
        let toml_str = profile.to_toml().unwrap();
        let back = Profile::from_toml(&toml_str).unwrap();
        // IDs match since from_toml preserves the serialized ID.
        assert_eq!(profile.id(), back.id());
        assert_eq!(profile.name(), back.name());
        assert_eq!(profile.devices(), back.devices());
        assert_eq!(profile.modes(), back.modes());
        assert_eq!(profile.mappings(), back.mappings());
        assert_eq!(
            profile.settings().startup_mode(),
            back.settings().startup_mode()
        );
    }

    #[test]
    fn profile_to_toml_rejects_programmatic_tap_gesture_zero_threshold() {
        let mut profile = minimal_profile();
        profile.set_mapping(
            &test_input(),
            "Default",
            Some("invalid tap".to_owned()),
            vec![Action::TapGesture {
                threshold_ms: 0,
                fire_single_immediately: false,
                single_tap: Vec::new(),
                double_tap: Vec::new(),
            }],
        );

        let err = profile
            .to_toml()
            .expect_err("invalid tap threshold should not serialize");

        assert!(err.to_string().contains("outside 1..=10000ms"));
    }

    #[test]
    fn profile_to_toml_rejects_programmatic_press_gesture_above_max_threshold() {
        let mut profile = minimal_profile();
        profile.set_mapping(
            &test_input(),
            "Default",
            Some("invalid press".to_owned()),
            vec![Action::PressGesture {
                threshold_ms: 10001,
                fire_long_when_threshold_crossed: false,
                short_press: Vec::new(),
                long_press: Vec::new(),
            }],
        );

        let err = profile
            .to_toml()
            .expect_err("invalid press threshold should not serialize");

        assert!(err.to_string().contains("outside 1..=10000ms"));
    }

    #[test]
    fn profile_rejects_tap_gesture_on_axis_mapping() {
        let input = r#"
modes = ["Default"]

[profile]
id = "01J00000000000000000000000"
name = "Invalid Gesture"
startup_mode = "Default"

[[mappings]]
mode = "Default"

[mappings.input]
device = "dev-1"

[mappings.input.input]
type = "axis"
index = 0

[[mappings.actions]]
type = "tap_gesture"
threshold_ms = 500
fire_single_immediately = false
single_tap = []
double_tap = []
"#;

        let err = Profile::from_toml(input).expect_err("axis mapping gesture should fail");

        assert!(
            err.to_string()
                .contains("gesture stages require a button mapping"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn profile_reports_unknown_mapping_mode_before_action_validation() {
        let input = r#"
modes = ["Default"]

[profile]
id = "01J00000000000000000000000"
name = "Invalid Precedence"
startup_mode = "Default"

[[mappings]]
mode = "Default"

[mappings.input]
device = "dev-1"

[mappings.input.input]
type = "axis"
index = 0

[[mappings.actions]]
type = "tap_gesture"
threshold_ms = 500
fire_single_immediately = false
single_tap = []
double_tap = []

[[mappings]]
mode = "Unknown"
actions = []

[mappings.input]
device = "dev-1"

[mappings.input.input]
type = "button"
index = 0
"#;

        let err = Profile::from_toml(input).expect_err("profile should fail validation");

        assert!(
            err.to_string().contains("mapping references unknown mode"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn profile_rejects_out_of_range_gesture_threshold() {
        let input = r#"
modes = ["Default"]

[profile]
id = "01J00000000000000000000000"
name = "Invalid Threshold"
startup_mode = "Default"

[[mappings]]
mode = "Default"

[mappings.input]
device = "dev-1"

[mappings.input.input]
type = "button"
index = 0

[[mappings.actions]]
type = "press_gesture"
threshold_ms = 10001
fire_long_when_threshold_crossed = false
short_press = []
long_press = []
"#;

        let err = Profile::from_toml(input).expect_err("large gesture threshold should fail");

        assert!(err.to_string().contains("outside 1..=10000ms"));
    }

    #[test]
    fn profile_rejects_deep_action_branch_tree() {
        fn button_input(index: u8) -> InputAddress {
            InputAddress::Bound {
                device: DeviceId("dev-1".to_owned()),
                input: InputId::Button { index },
            }
        }

        fn nested_conditional(depth: usize) -> Action {
            if depth == 0 {
                return Action::Invert;
            }
            Action::Conditional {
                condition: Condition::ButtonPressed {
                    input: button_input(1),
                },
                if_true: vec![nested_conditional(depth - 1)],
                if_false: Vec::new(),
            }
        }

        let raw = ProfileRaw {
            linux: None,
            controllers: None,
            modes: Modes::new(vec!["Default".to_owned()]).unwrap(),
            profile: ProfileMeta {
                id: ProfileId::new(),
                name: "Too Deep".to_owned(),
                startup_mode: "Default".to_owned(),
            },
            devices: Vec::new(),
            mappings: vec![Mapping {
                input: button_input(0),
                mode: "Default".to_owned(),
                name: None,
                actions: vec![nested_conditional(MAX_ACTION_BRANCH_DEPTH + 1)],
            }],
            calibrations: Vec::new(),
        };

        let err = Profile::from_raw(raw).expect_err("deep action tree should fail");

        assert!(
            err.to_string().contains("action branch depth exceeds"),
            "unexpected error: {err}"
        );
    }

    fn button_mapping_input() -> InputAddress {
        InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 0 },
        }
    }

    fn predicate_button_input() -> InputAddress {
        InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 1 },
        }
    }

    #[test]
    fn validate_rejects_nested_tap_in_tap() {
        let actions = vec![Action::TapGesture {
            threshold_ms: 250,
            fire_single_immediately: false,
            single_tap: vec![Action::TapGesture {
                threshold_ms: 250,
                fire_single_immediately: false,
                single_tap: Vec::new(),
                double_tap: Vec::new(),
            }],
            double_tap: Vec::new(),
        }];

        let err = validate_mapping_action_tree(&button_mapping_input(), &actions)
            .expect_err("nested tap inside tap should be rejected");

        assert!(
            err.to_string().contains("cannot be nested"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn validate_rejects_nested_press_in_tap_via_conditional() {
        let actions = vec![Action::TapGesture {
            threshold_ms: 250,
            fire_single_immediately: false,
            single_tap: vec![Action::Conditional {
                condition: Condition::ButtonPressed {
                    input: predicate_button_input(),
                },
                if_true: vec![Action::PressGesture {
                    threshold_ms: 600,
                    fire_long_when_threshold_crossed: false,
                    short_press: Vec::new(),
                    long_press: Vec::new(),
                }],
                if_false: Vec::new(),
            }],
            double_tap: Vec::new(),
        }];

        let err = validate_mapping_action_tree(&button_mapping_input(), &actions)
            .expect_err("press nested inside tap branch via conditional should be rejected");

        assert!(
            err.to_string().contains("cannot be nested"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn validate_accepts_conditional_inside_gesture_branch() {
        let actions = vec![Action::TapGesture {
            threshold_ms: 250,
            fire_single_immediately: false,
            single_tap: vec![Action::Conditional {
                condition: Condition::ButtonPressed {
                    input: predicate_button_input(),
                },
                if_true: vec![Action::Invert],
                if_false: Vec::new(),
            }],
            double_tap: Vec::new(),
        }];

        validate_mapping_action_tree(&button_mapping_input(), &actions)
            .expect("conditional inside a gesture branch is allowed");
    }

    #[test]
    fn flat_modes_toml_roundtrip() {
        let input = r#"
modes = ["Default", "Combat", "Landing"]

[profile]
id = "01J00000000000000000000000"
name = "Flight"
startup_mode = "Default"
"#;

        let profile = Profile::from_toml(input).unwrap();
        assert_eq!(profile.modes().as_slice(), ["Default", "Combat", "Landing"]);

        let saved = profile.to_toml().unwrap();
        assert!(saved.contains("modes = [\"Default\", \"Combat\", \"Landing\"]"));

        let reparsed = Profile::from_toml(&saved).unwrap();
        assert_eq!(reparsed, profile);
    }

    fn legacy_keyboard_profile_toml(key: &str, modifiers: &str) -> String {
        format!(
            r#"
modes = ["Default"]

[profile]
id = "01J00000000000000000000000"
name = "Legacy Keyboard"
startup_mode = "Default"

[[mappings]]
mode = "Default"

[mappings.input]
device = "dev-1"

[mappings.input.input]
type = "button"
index = 0

[[mappings.actions]]
type = "map_to_keyboard"
behavior = "hold"

[mappings.actions.key]
key = "{key}"
modifiers = [{modifiers}]
"#
        )
    }

    fn assert_keyboard_action(
        action: &Action,
        expected_key: PhysicalKey,
        expected_modifiers: &[KeyModifier],
    ) {
        let Action::MapToKeyboard { key, behavior } = action else {
            panic!("expected map_to_keyboard action, got {action:?}");
        };

        assert_eq!(key.key, expected_key);
        assert_eq!(key.modifiers, expected_modifiers);
        assert_eq!(*behavior, OutputBehavior::Hold);
    }

    #[test]
    fn profile_from_toml_migrates_legacy_keyboard() {
        let toml = legacy_keyboard_profile_toml("F1", r#""Ctrl", "Shift", "Alt", "Win""#);

        let profile = Profile::from_toml(&toml).unwrap();
        let action = &profile.mappings()[0].actions[0];

        assert_keyboard_action(
            action,
            PhysicalKey::F1,
            &[
                KeyModifier::CONTROL_LEFT,
                KeyModifier::SHIFT_LEFT,
                KeyModifier::ALT_LEFT,
                KeyModifier::META_LEFT,
            ],
        );
    }

    #[test]
    fn profile_from_toml_migrates_legacy_keyboard_aliases() {
        for (legacy, expected) in [
            ("Return", PhysicalKey::Enter),
            ("Esc", PhysicalKey::Escape),
            ("Del", PhysicalKey::Delete),
            ("PgUp", PhysicalKey::PageUp),
            ("PgDn", PhysicalKey::PageDown),
        ] {
            let toml = legacy_keyboard_profile_toml(legacy, "");
            let profile = Profile::from_toml(&toml).unwrap();

            assert_keyboard_action(&profile.mappings()[0].actions[0], expected, &[]);
        }
    }

    #[test]
    fn profile_from_toml_rejects_legacy_f13_to_f24() {
        for key in ["F13", "F18", "F24"] {
            let toml = legacy_keyboard_profile_toml(key, "");
            let err = Profile::from_toml(&toml).unwrap_err();

            assert!(
                err.to_string()
                    .contains(&format!("unsupported legacy keyboard key: {key}")),
                "unexpected error for {key}: {err}"
            );
        }
    }

    #[test]
    fn profile_from_toml_rejects_out_of_range_function_keys() {
        for key in ["F0", "F25", "F99", "F100", "F9999999999"] {
            let toml = legacy_keyboard_profile_toml(key, "");
            let err = Profile::from_toml(&toml).unwrap_err();

            assert!(
                err.to_string()
                    .contains(&format!("unsupported legacy keyboard key: {key}")),
                "unexpected error for {key}: {err}"
            );
        }
    }

    #[test]
    fn profile_load_saves_migrated_keyboard_mapping() {
        let dir = std::env::temp_dir().join(format!(
            "inputforge_legacy_keyboard_migration_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("legacy_profile.toml");
        let toml = legacy_keyboard_profile_toml("A", r#""Ctrl""#);
        std::fs::write(&path, toml).unwrap();

        let profile = Profile::load(&path).unwrap();
        let saved = std::fs::read_to_string(&path).unwrap();

        assert_keyboard_action(
            &profile.mappings()[0].actions[0],
            PhysicalKey::KeyA,
            &[KeyModifier::CONTROL_LEFT],
        );
        assert!(saved.contains("KeyA"));
        assert!(saved.contains("ControlLeft"));
        assert!(!saved.contains("\"Ctrl\""));

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn non_list_modes_value_is_rejected_with_neutral_message() {
        let input = r#"
modes = 42

[profile]
id = "01J00000000000000000000000"
name = "Flight"
startup_mode = "Default"
"#;

        let err = Profile::from_toml(input).unwrap_err();

        assert!(
            err.to_string()
                .contains("modes must be a flat list of strings")
        );
    }

    #[test]
    fn non_string_modes_entry_is_rejected() {
        let input = r#"
modes = ["Default", 42]

[profile]
id = "01J00000000000000000000000"
name = "Flight"
startup_mode = "Default"
"#;

        let err = Profile::from_toml(input).unwrap_err();

        assert!(err.to_string().contains("mode names must be strings"));
    }

    #[test]
    fn profile_from_toml_minimal() {
        let toml_str = r#"
modes = ["Default"]

[profile]
id = "test-id-1234"
name = "Minimal"
startup_mode = "Default"

[[mappings]]
mode = "Default"

[mappings.input]
device = "dev-1"

[mappings.input.input]
type = "axis"
index = 0

[[mappings.actions]]
type = "invert"
"#;
        let profile = Profile::from_toml(toml_str).unwrap();
        assert_eq!(profile.name(), "Minimal");
        assert_eq!(profile.id().as_str(), "test-id-1234");
        assert_eq!(profile.settings().startup_mode(), "Default");
        assert!(profile.devices().is_empty());
        assert_eq!(profile.mappings().len(), 1);
    }

    #[test]
    fn profile_from_toml_with_devices() {
        let toml_str = r#"
modes = ["Default"]

[profile]
id = "test-id"
name = "With Devices"
startup_mode = "Default"

[[devices]]
id = "guid-001"
name = "Left Stick"

[[devices]]
id = "guid-002"
name = "Right Stick"
"#;
        let profile = Profile::from_toml(toml_str).unwrap();
        assert_eq!(profile.devices().len(), 2);
        assert_eq!(profile.devices()[0].name, "Left Stick");
        assert_eq!(profile.devices()[1].name, "Right Stick");
    }

    // --- Validation ---

    #[test]
    fn profile_invalid_startup_mode() {
        let toml_str = r#"
modes = ["Default"]

[profile]
id = "test-id"
name = "Bad"
startup_mode = "NonExistent"
"#;
        let err = Profile::from_toml(toml_str).unwrap_err();
        assert!(err.to_string().contains("startup_mode"));
        assert!(err.to_string().contains("NonExistent"));
    }

    #[test]
    fn profile_invalid_mapping_mode() {
        let toml_str = r#"
modes = ["Default"]

[profile]
id = "test-id"
name = "Bad"
startup_mode = "Default"

[[mappings]]
mode = "Unknown"

[mappings.input]
device = "dev-1"

[mappings.input.input]
type = "axis"
index = 0

[[mappings.actions]]
type = "invert"
"#;
        let err = Profile::from_toml(toml_str).unwrap_err();
        assert!(err.to_string().contains("Unknown"));
    }

    #[test]
    fn profile_invalid_toml_syntax() {
        let result = Profile::from_toml("this is not toml [[[");
        result.unwrap_err();
    }

    // --- File I/O ---

    #[test]
    fn profile_load_nonexistent_file() {
        let path = Path::new("nonexistent_profile.toml");
        let err = Profile::load(path).unwrap_err();
        assert!(err.to_string().contains("profile not found"));
    }

    #[test]
    fn profile_save_and_load_roundtrip() {
        let profile = minimal_profile();
        let dir = std::env::temp_dir().join("inputforge_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_profile.toml");

        profile.save(&path).unwrap();
        let loaded = Profile::load(&path).unwrap();

        assert_eq!(profile.id(), loaded.id());
        assert_eq!(profile.name(), loaded.name());
        assert_eq!(profile.modes(), loaded.modes());
        assert_eq!(profile.mappings(), loaded.mappings());

        // Cleanup.
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    // --- Complex profiles ---

    #[test]
    fn profile_with_modes() {
        let modes = test_modes();
        let profile = Profile::new(
            "Complex".to_owned(),
            vec![],
            modes,
            vec![
                Mapping {
                    input: test_input(),
                    mode: "Default".to_owned(),
                    name: None,
                    actions: vec![Action::Invert],
                },
                Mapping {
                    input: test_input(),
                    mode: "Combat".to_owned(),
                    name: None,
                    actions: vec![],
                },
            ],
            vec![],
            "Default".to_owned(),
        );
        let toml_str = profile.to_toml().unwrap();
        let back = Profile::from_toml(&toml_str).unwrap();
        assert_eq!(profile.modes(), back.modes());
        assert_eq!(profile.mappings().len(), 2);
    }

    #[test]
    fn profile_with_deadzone_action() {
        let modes = Modes::new(vec!["Default".to_owned()]).unwrap();

        let profile = Profile::new(
            "Deadzone Test".to_owned(),
            vec![],
            modes,
            vec![Mapping {
                input: test_input(),
                mode: "Default".to_owned(),
                name: None,
                actions: vec![
                    Action::Deadzone {
                        config: DeadzoneConfig::default(),
                    },
                    Action::MapToVJoy {
                        output: test_output(),
                    },
                ],
            }],
            vec![],
            "Default".to_owned(),
        );
        let toml_str = profile.to_toml().unwrap();
        let back = Profile::from_toml(&toml_str).unwrap();
        assert_eq!(profile.mappings(), back.mappings());
    }

    #[test]
    fn profile_with_conditional_actions() {
        let modes = Modes::new(vec!["Default".to_owned()]).unwrap();

        let profile = Profile::new(
            "Conditional Test".to_owned(),
            vec![],
            modes,
            vec![Mapping {
                input: InputAddress::Bound {
                    device: DeviceId("dev-1".to_owned()),
                    input: InputId::Button { index: 0 },
                },
                mode: "Default".to_owned(),
                name: None,
                actions: vec![Action::Conditional {
                    condition: Condition::ButtonPressed {
                        input: InputAddress::Bound {
                            device: DeviceId("dev-1".to_owned()),
                            input: InputId::Button { index: 5 },
                        },
                    },
                    if_true: vec![Action::MapToKeyboard {
                        key: KeyCombo {
                            key: PhysicalKey::F1,
                            modifiers: vec![KeyModifier::CONTROL_LEFT],
                        },
                        behavior: OutputBehavior::Hold,
                    }],
                    if_false: vec![Action::MapToKeyboard {
                        key: KeyCombo {
                            key: PhysicalKey::F1,
                            modifiers: vec![],
                        },
                        behavior: OutputBehavior::Hold,
                    }],
                }],
            }],
            vec![],
            "Default".to_owned(),
        );
        let toml_str = profile.to_toml().unwrap();
        let back = Profile::from_toml(&toml_str).unwrap();
        assert_eq!(profile.mappings(), back.mappings());
    }

    #[test]
    fn profile_with_change_mode_action() {
        let modes = test_modes();
        let profile = Profile::new(
            "Mode Switch".to_owned(),
            vec![],
            modes,
            vec![Mapping {
                input: InputAddress::Bound {
                    device: DeviceId("dev-1".to_owned()),
                    input: InputId::Button { index: 1 },
                },
                mode: "Default".to_owned(),
                name: None,
                actions: vec![Action::ChangeMode {
                    strategy: ModeChangeStrategy::SwitchTo {
                        mode: "Combat".to_owned(),
                    },
                }],
            }],
            vec![],
            "Default".to_owned(),
        );
        let toml_str = profile.to_toml().unwrap();
        let back = Profile::from_toml(&toml_str).unwrap();
        assert_eq!(profile.mappings(), back.mappings());
    }

    #[test]
    fn profile_with_merge_axis_action() {
        let modes = Modes::new(vec!["Default".to_owned()]).unwrap();

        let profile = Profile::new(
            "Merge Test".to_owned(),
            vec![],
            modes,
            vec![Mapping {
                input: test_input(),
                mode: "Default".to_owned(),
                name: None,
                actions: vec![
                    Action::MergeAxis {
                        second_input: InputAddress::Bound {
                            device: DeviceId("dev-1".to_owned()),
                            input: InputId::Axis { index: 1 },
                        },
                        operation: MergeOp::Bidirectional,
                    },
                    Action::MapToVJoy {
                        output: test_output(),
                    },
                ],
            }],
            vec![],
            "Default".to_owned(),
        );
        let toml_str = profile.to_toml().unwrap();
        let back = Profile::from_toml(&toml_str).unwrap();
        assert_eq!(profile.mappings(), back.mappings());
    }

    #[test]
    fn profile_with_calibrations_roundtrip() {
        let modes = Modes::new(vec!["Default".to_owned()]).unwrap();

        let calibrations = vec![
            CalibrationEntry {
                device: DeviceId("dev-1".to_owned()),
                axis: 0,
                physical_min: -32768.0,
                physical_center_low: -100.0,
                physical_center_high: 100.0,
                physical_max: 32767.0,
                enabled: true,
            },
            CalibrationEntry {
                device: DeviceId("dev-1".to_owned()),
                axis: 1,
                physical_min: -500.0,
                physical_center_low: -10.0,
                physical_center_high: 10.0,
                physical_max: 500.0,
                enabled: false,
            },
        ];

        let profile = Profile::new(
            "Calibrated".to_owned(),
            vec![],
            modes,
            vec![],
            calibrations.clone(),
            "Default".to_owned(),
        );
        let toml_str = profile.to_toml().unwrap();
        let back = Profile::from_toml(&toml_str).unwrap();
        assert_eq!(profile.calibrations(), back.calibrations());
        assert_eq!(back.calibrations().len(), 2);
        assert_eq!(back.calibrations()[0].axis, 0);
        assert_eq!(back.calibrations()[1].axis, 1);
        assert!(back.calibrations()[0].enabled);
        assert!(!back.calibrations()[1].enabled);
    }

    #[test]
    fn profile_without_calibrations_loads() {
        let toml_str = r#"
modes = ["Default"]

[profile]
id = "test-id"
name = "No Cals"
startup_mode = "Default"
"#;
        let profile = Profile::from_toml(toml_str).unwrap();
        assert!(profile.calibrations().is_empty());
    }

    #[test]
    fn profile_with_invalid_calibration_rejected() {
        let toml_str = r#"
modes = ["Default"]

[profile]
id = "test-id"
name = "Bad Cal"
startup_mode = "Default"

[[calibrations]]
device = "dev-1"
axis = 0
physical_min = 100.0
physical_center_low = 0.0
physical_center_high = 0.0
physical_max = -100.0
enabled = true
"#;
        let err = Profile::from_toml(toml_str).unwrap_err();
        assert!(err.to_string().contains("calibration"));
    }

    // --- find_mapping / set_mapping ---

    #[test]
    fn find_mapping_returns_existing() {
        let profile = minimal_profile();
        let result = profile.find_mapping(&test_input(), "Default");
        assert!(result.is_some());
        assert_eq!(result.unwrap().actions, vec![Action::Invert]);
    }

    #[test]
    fn find_mapping_returns_none_for_unknown() {
        let profile = minimal_profile();
        assert!(profile.find_mapping(&test_input(), "NonExistent").is_none());

        let other_input = InputAddress::Bound {
            device: DeviceId("other-dev".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        assert!(profile.find_mapping(&other_input, "Default").is_none());
    }

    #[test]
    fn set_mapping_creates_new() {
        let mut profile = minimal_profile();
        let new_input = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 5 },
        };
        let actions = vec![Action::MapToVJoy {
            output: test_output(),
        }];

        profile.set_mapping(
            &new_input,
            "Default",
            Some("My Button".to_owned()),
            actions.clone(),
        );

        let found = profile.find_mapping(&new_input, "Default");
        assert!(found.is_some());
        let m = found.unwrap();
        assert_eq!(m.name, Some("My Button".to_owned()));
        assert_eq!(m.actions, actions);
        assert_eq!(profile.mappings().len(), 2);
    }

    #[test]
    fn set_mapping_updates_existing() {
        let mut profile = minimal_profile();
        let new_actions = vec![Action::MapToVJoy {
            output: test_output(),
        }];

        profile.set_mapping(
            &test_input(),
            "Default",
            Some("Renamed".to_owned()),
            new_actions.clone(),
        );

        assert_eq!(profile.mappings().len(), 1);
        let m = profile.find_mapping(&test_input(), "Default").unwrap();
        assert_eq!(m.name, Some("Renamed".to_owned()));
        assert_eq!(m.actions, new_actions);
    }

    #[test]
    fn set_mapping_with_empty_actions_inserts_placeholder_mapping() {
        // F8 + Add mapping flow dispatches SetMapping with actions: vec![]
        // before F9's action editor ships. The mapping must persist with
        // the empty action vector so it appears in the rail; F9 will fill
        // in actions later. Use `remove_mapping` for explicit deletion.
        let mut profile = minimal_profile();
        let new_input = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 7 },
        };
        let before_len = profile.mappings().len();

        profile.set_mapping(&new_input, "Default", Some("Fresh".to_owned()), vec![]);

        assert_eq!(profile.mappings().len(), before_len + 1);
        let added = profile
            .find_mapping(&new_input, "Default")
            .expect("freshly added mapping must be findable");
        assert_eq!(added.name.as_deref(), Some("Fresh"));
        assert!(
            added.actions.is_empty(),
            "actions vec must round-trip empty"
        );
    }

    #[test]
    fn set_mapping_with_empty_actions_updates_existing_to_empty() {
        // The shortcut "empty actions removes" used to live in set_mapping;
        // it is now gone. Updating an existing mapping to empty actions
        // must overwrite the actions, not delete the mapping.
        let mut profile = minimal_profile();
        assert_eq!(profile.mappings().len(), 1);

        profile.set_mapping(&test_input(), "Default", Some("Cleared".to_owned()), vec![]);

        assert_eq!(
            profile.mappings().len(),
            1,
            "mapping must survive empty-action update"
        );
        let m = profile.find_mapping(&test_input(), "Default").unwrap();
        assert_eq!(m.name, Some("Cleared".to_owned()));
        assert!(m.actions.is_empty());
    }

    #[test]
    fn profile_set_mappings_bulk_with_empty_entries_is_noop() {
        use crate::action::BulkMapEntry;

        let mut profile = test_profile_with_one_mode();
        profile.set_mappings_bulk(&[] as &[BulkMapEntry]);
        assert!(profile.mappings().is_empty());
    }

    #[test]
    fn profile_set_mappings_bulk_creates_single_mapping_with_unnamed_passthrough() {
        use crate::action::{Action, BulkMapEntry};
        use crate::types::{DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis};

        let mut profile = test_profile_with_one_mode();
        let output = OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::X },
        };
        let entries = vec![BulkMapEntry {
            input: InputAddress::Bound {
                device: DeviceId("dev-1".to_owned()),
                input: InputId::Axis { index: 0 },
            },
            mode: "Default".to_owned(),
            output: output.clone(),
        }];
        profile.set_mappings_bulk(&entries);

        assert_eq!(profile.mappings().len(), 1);
        let m = &profile.mappings()[0];
        assert_eq!(m.name, None);
        assert_eq!(m.actions.len(), 1);
        match &m.actions[0] {
            Action::MapToVJoy { output: actual } => assert_eq!(actual, &output),
            other => panic!("expected MapToVJoy action, got {other:?}"),
        }
    }

    #[test]
    fn profile_set_mappings_bulk_creates_one_mapping_per_entry_across_modes() {
        use crate::action::BulkMapEntry;
        use crate::types::{DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis};

        let mut profile = test_profile_with_two_modes();
        let input = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        let output = OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::X },
        };
        let entries = vec![
            BulkMapEntry {
                input: input.clone(),
                mode: "Default".to_owned(),
                output: output.clone(),
            },
            BulkMapEntry {
                input: input.clone(),
                mode: "Combat".to_owned(),
                output: output.clone(),
            },
        ];
        profile.set_mappings_bulk(&entries);
        assert_eq!(profile.mappings().len(), 2);
    }

    #[test]
    fn profile_set_mappings_bulk_replaces_existing_mapping_overwriting_name_and_actions() {
        use crate::action::{Action, BulkMapEntry};
        use crate::types::{DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis};

        let mut profile = test_profile_with_one_mode();
        let input = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        profile.set_mapping(
            &input,
            "Default",
            Some("Throttle".to_owned()),
            vec![Action::Invert],
        );

        let entries = vec![BulkMapEntry {
            input: input.clone(),
            mode: "Default".to_owned(),
            output: OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::Y },
            },
        }];
        profile.set_mappings_bulk(&entries);

        assert_eq!(profile.mappings().len(), 1, "must upsert, not append");
        let m = &profile.mappings()[0];
        assert_eq!(m.name, None, "name must be cleared by bulk replace");
        assert!(matches!(m.actions[0], Action::MapToVJoy { .. }));
    }

    #[test]
    fn profile_set_mappings_bulk_each_generated_mapping_has_action_vec_of_exactly_one_map_to_vjoy()
    {
        use crate::action::{Action, BulkMapEntry};
        use crate::types::{DeviceId, InputAddress, InputId, OutputAddress, OutputId};

        let mut profile = test_profile_with_one_mode();
        let entries = vec![BulkMapEntry {
            input: InputAddress::Bound {
                device: DeviceId("dev-1".to_owned()),
                input: InputId::Button { index: 5 },
            },
            mode: "Default".to_owned(),
            output: OutputAddress {
                device: 1,
                output: OutputId::Button { id: 6 },
            },
        }];
        profile.set_mappings_bulk(&entries);
        let m = &profile.mappings()[0];
        assert_eq!(m.actions.len(), 1);
        assert!(matches!(m.actions[0], Action::MapToVJoy { .. }));
    }

    #[test]
    fn profile_set_mappings_bulk_each_generated_mapping_has_name_none() {
        use crate::action::BulkMapEntry;
        use crate::types::{DeviceId, InputAddress, InputId, OutputAddress, OutputId};

        let mut profile = test_profile_with_one_mode();
        let entries = vec![BulkMapEntry {
            input: InputAddress::Bound {
                device: DeviceId("dev-1".to_owned()),
                input: InputId::Hat { index: 0 },
            },
            mode: "Default".to_owned(),
            output: OutputAddress {
                device: 1,
                output: OutputId::Hat { id: 1 },
            },
        }];
        profile.set_mappings_bulk(&entries);
        assert_eq!(profile.mappings()[0].name, None);
    }

    #[test]
    fn profile_set_mappings_bulk_mixed_create_and_replace_in_one_call() {
        use crate::action::{Action, BulkMapEntry};
        use crate::types::{DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis};

        let mut profile = test_profile_with_one_mode();
        let in_a = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        let in_b = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 1 },
        };
        profile.set_mapping(
            &in_a,
            "Default",
            Some("Pre".to_owned()),
            vec![Action::Invert],
        );

        let out = OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::X },
        };
        let entries = vec![
            BulkMapEntry {
                input: in_a.clone(),
                mode: "Default".to_owned(),
                output: out.clone(),
            },
            BulkMapEntry {
                input: in_b.clone(),
                mode: "Default".to_owned(),
                output: out.clone(),
            },
        ];
        profile.set_mappings_bulk(&entries);

        assert_eq!(profile.mappings().len(), 2);
        assert_eq!(profile.find_mapping(&in_a, "Default").unwrap().name, None);
        assert!(matches!(
            profile.find_mapping(&in_a, "Default").unwrap().actions[0],
            Action::MapToVJoy { .. }
        ));
        assert!(profile.find_mapping(&in_b, "Default").is_some());
    }

    #[test]
    fn profile_set_mappings_bulk_into_unknown_mode_still_upserts_silently() {
        use crate::action::BulkMapEntry;
        use crate::types::{DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis};

        let mut profile = test_profile_with_one_mode();
        let entries = vec![BulkMapEntry {
            input: InputAddress::Bound {
                device: DeviceId("dev-1".to_owned()),
                input: InputId::Axis { index: 0 },
            },
            mode: "Phantom".to_owned(),
            output: OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::X },
            },
        }];
        profile.set_mappings_bulk(&entries);
        assert_eq!(
            profile.mappings().len(),
            1,
            "engine accepts the upsert; reload-time validation will flag the orphan"
        );
    }

    // --- set_modes / remove_mappings_for_mode ---

    #[test]
    fn set_modes_replaces_modes() {
        let mut profile = minimal_profile();
        let new_modes = test_modes();
        profile.set_modes(new_modes.clone());
        assert_eq!(profile.modes(), &new_modes);
    }

    #[test]
    fn remove_mappings_for_mode_drops_matching_and_returns_count() {
        use crate::action::Mapping;

        let modes = test_modes();
        let mut profile = Profile::new(
            "Counted".to_owned(),
            vec![],
            modes,
            vec![
                Mapping {
                    input: test_input(),
                    mode: "Combat".to_owned(),
                    name: None,
                    actions: vec![Action::Invert],
                },
                Mapping {
                    input: InputAddress::Bound {
                        device: DeviceId("dev-1".to_owned()),
                        input: InputId::Button { index: 0 },
                    },
                    mode: "Combat".to_owned(),
                    name: None,
                    actions: vec![Action::Invert],
                },
                Mapping {
                    input: test_input(),
                    mode: "Default".to_owned(),
                    name: None,
                    actions: vec![Action::Invert],
                },
            ],
            vec![],
            "Default".to_owned(),
        );

        let removed = profile.remove_mappings_for_mode("Combat");
        assert_eq!(removed, 2);
        assert_eq!(profile.mappings().len(), 1);
        assert_eq!(profile.mappings()[0].mode, "Default");
    }

    #[test]
    fn remove_mappings_for_mode_returns_zero_for_unmapped_mode() {
        let mut profile = minimal_profile();
        let removed = profile.remove_mappings_for_mode("Combat");
        assert_eq!(removed, 0);
        assert_eq!(profile.mappings().len(), 1);
    }

    // --- rename_mode_refs ---

    #[test]
    fn rename_mode_refs_rewrites_mapping_modes_and_startup() {
        use crate::action::Mapping;

        let modes = test_modes();
        let mut profile = Profile::new(
            "RenameMe".to_owned(),
            vec![],
            modes,
            vec![Mapping {
                input: test_input(),
                mode: "Combat".to_owned(),
                name: None,
                actions: vec![Action::Invert],
            }],
            vec![],
            "Default".to_owned(),
        );

        let touched = profile.rename_mode_refs("Combat", "Fighter");
        assert_eq!(touched, 1);
        assert_eq!(profile.mappings()[0].mode, "Fighter");
        // startup_mode unchanged because it referenced Default, not Combat.
        assert_eq!(profile.settings().startup_mode(), "Default");

        let touched_default = profile.rename_mode_refs("Default", "Root");
        assert_eq!(touched_default, 0, "no mapping referenced Default");
        assert_eq!(profile.settings().startup_mode(), "Root");
    }

    #[test]
    fn rename_mode_refs_rewrites_change_mode_actions() {
        use crate::action::{Mapping, ModeChangeStrategy};

        let modes = test_modes();
        let mut profile = Profile::new(
            "ChangeRename".to_owned(),
            vec![],
            modes,
            vec![Mapping {
                input: test_input(),
                mode: "Default".to_owned(),
                name: None,
                actions: vec![
                    Action::ChangeMode {
                        strategy: ModeChangeStrategy::SwitchTo {
                            mode: "Combat".to_owned(),
                        },
                    },
                    Action::ChangeMode {
                        strategy: ModeChangeStrategy::Temporary {
                            mode: "Combat".to_owned(),
                        },
                    },
                ],
            }],
            vec![],
            "Default".to_owned(),
        );

        let touched = profile.rename_mode_refs("Combat", "Fighter");
        assert_eq!(touched, 1);
        let actions = &profile.mappings()[0].actions;
        match &actions[0] {
            Action::ChangeMode {
                strategy: ModeChangeStrategy::SwitchTo { mode },
            } => assert_eq!(mode, "Fighter"),
            _ => panic!("expected SwitchTo Fighter"),
        }
        match &actions[1] {
            Action::ChangeMode {
                strategy: ModeChangeStrategy::Temporary { mode },
            } => assert_eq!(mode, "Fighter"),
            _ => panic!("expected Temporary Fighter"),
        }
    }

    #[test]
    fn rename_mode_refs_walks_into_conditional_branches() {
        use crate::action::{Condition, Mapping, ModeChangeStrategy};

        let modes = test_modes();
        let mut profile = Profile::new(
            "Conditional".to_owned(),
            vec![],
            modes,
            vec![Mapping {
                input: test_input(),
                mode: "Default".to_owned(),
                name: None,
                actions: vec![Action::Conditional {
                    condition: Condition::ButtonPressed {
                        input: test_input(),
                    },
                    if_true: vec![Action::ChangeMode {
                        strategy: ModeChangeStrategy::SwitchTo {
                            mode: "Combat".to_owned(),
                        },
                    }],
                    if_false: vec![Action::ChangeMode {
                        strategy: ModeChangeStrategy::Temporary {
                            mode: "Combat".to_owned(),
                        },
                    }],
                }],
            }],
            vec![],
            "Default".to_owned(),
        );

        let touched = profile.rename_mode_refs("Combat", "Fighter");
        assert_eq!(touched, 1);
    }

    #[test]
    fn rename_mode_refs_walks_into_gesture_branches() {
        let mut profile = test_profile_with_two_modes();
        profile.set_mapping(
            &test_input(),
            "Default",
            Some("gesture".to_owned()),
            vec![Action::PressGesture {
                threshold_ms: 500,
                fire_long_when_threshold_crossed: false,
                short_press: vec![Action::ChangeMode {
                    strategy: ModeChangeStrategy::SwitchTo {
                        mode: "Combat".to_owned(),
                    },
                }],
                long_press: Vec::new(),
            }],
        );

        let changed = profile.rename_mode_refs("Combat", "Landing");

        assert_eq!(changed, 1);
        let mapping = profile
            .find_mapping(&test_input(), "Default")
            .expect("mapping should remain");
        let Action::PressGesture { short_press, .. } = &mapping.actions[0] else {
            panic!("expected press gesture");
        };
        let Action::ChangeMode {
            strategy: ModeChangeStrategy::SwitchTo { mode },
        } = &short_press[0]
        else {
            panic!("expected change mode");
        };
        assert_eq!(mode, "Landing");
    }

    // --- remove_mapping ---

    #[test]
    fn remove_mapping_drops_existing_returns_true() {
        let mut profile = minimal_profile();
        assert!(!profile.mappings().is_empty(), "fixture invariant");
        let target = profile.mappings()[0].input.clone();
        let target_mode = profile.mappings()[0].mode.clone();

        let before_len = profile.mappings().len();
        let removed = profile.remove_mapping(&target, &target_mode);

        assert!(
            removed,
            "remove_mapping should return true when a mapping was removed"
        );
        assert_eq!(profile.mappings().len(), before_len - 1);
        assert!(profile.find_mapping(&target, &target_mode).is_none());
    }

    #[test]
    fn remove_mapping_unknown_returns_false() {
        let mut profile = minimal_profile();
        assert!(!profile.mappings().is_empty(), "fixture invariant");
        let target = InputAddress::Bound {
            device: DeviceId("nonexistent".to_owned()),
            input: InputId::Button { index: 99 },
        };

        let before_len = profile.mappings().len();
        let removed = profile.remove_mapping(&target, "Default");

        assert!(
            !removed,
            "remove_mapping should return false when nothing matched"
        );
        assert_eq!(profile.mappings().len(), before_len);
    }

    #[test]
    fn remove_mapping_wrong_mode_returns_false() {
        // remove_mapping is mode-scoped: a matching input in a different
        // mode must NOT be removed.
        let mut profile = minimal_profile();
        assert!(!profile.mappings().is_empty(), "fixture invariant");
        let target = profile.mappings()[0].input.clone();

        let removed = profile.remove_mapping(&target, "NonexistentMode");

        assert!(!removed);
        assert!(profile.find_mapping(&target, "Default").is_some());
    }

    // --- reorder_mapping_in_group ---

    fn input_axis(idx: u8) -> InputAddress {
        InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: idx },
        }
    }
    fn input_button(idx: u8) -> InputAddress {
        InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: idx },
        }
    }
    fn input_hat(idx: u8) -> InputAddress {
        InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Hat { index: idx },
        }
    }
    fn no_op_mapping(input: InputAddress, mode: &str) -> Mapping {
        Mapping {
            input,
            mode: mode.to_owned(),
            name: None,
            actions: vec![],
        }
    }
    /// Multi-group, multi-mode fixture.
    /// Default mode flat order: Axis0, Axis1, Button0, Axis2, Button1, Hat0.
    /// Combat mode flat order: Axis5.
    fn reorder_fixture() -> Profile {
        let modes = test_modes();
        Profile::new(
            "Reorder Fixture".to_owned(),
            vec![DeviceEntry {
                id: DeviceId("dev-1".to_owned()),
                name: "Test Stick".to_owned(),
            }],
            modes,
            vec![
                no_op_mapping(input_axis(0), "Default"),
                no_op_mapping(input_axis(1), "Default"),
                no_op_mapping(input_button(0), "Default"),
                no_op_mapping(input_axis(2), "Default"),
                no_op_mapping(input_button(1), "Default"),
                no_op_mapping(input_hat(0), "Default"),
                no_op_mapping(input_axis(5), "Combat"),
            ],
            vec![],
            "Default".to_owned(),
        )
    }

    #[test]
    fn reorder_within_axes_moves_subgroup_down() {
        let mut profile = reorder_fixture();
        // Axes group in Default mode is [Axis0, Axis1, Axis2]. Move Axis0 (subpos 0) to subpos 2.
        let moved = profile.reorder_mapping_in_group(&input_axis(0), "Default", 2);
        assert!(moved);
        let inputs: Vec<&InputAddress> = profile
            .mappings()
            .iter()
            .filter(|m| m.mode == "Default")
            .map(|m| &m.input)
            .collect();
        // Reorder is local to the Axes group within the flat vec: Axis0
        // lands AFTER the previous-last-axis (Axis2), preserving its
        // contiguity with the rest of the axes. Buttons and Hat keep
        // their flat-vec positions relative to one another.
        assert_eq!(
            inputs,
            vec![
                &input_axis(1),
                &input_button(0),
                &input_axis(2),
                &input_axis(0),
                &input_button(1),
                &input_hat(0),
            ]
        );
        // Axes bucket should read [Axis1, Axis2, Axis0] in subpos order.
        let axes_subpos: Vec<&InputAddress> = profile
            .mappings()
            .iter()
            .filter(|m| {
                m.mode == "Default" && matches!(m.input.input_id(), Some(InputId::Axis { .. }))
            })
            .map(|m| &m.input)
            .collect();
        assert_eq!(
            axes_subpos,
            vec![&input_axis(1), &input_axis(2), &input_axis(0)]
        );
    }

    #[test]
    fn reorder_within_axes_moves_subgroup_up() {
        let mut profile = reorder_fixture();
        // Move Axis2 from subpos 2 to subpos 0.
        let moved = profile.reorder_mapping_in_group(&input_axis(2), "Default", 0);
        assert!(moved);
        let axes_in_default: Vec<&InputAddress> = profile
            .mappings()
            .iter()
            .filter(|m| {
                m.mode == "Default" && matches!(m.input.input_id(), Some(InputId::Axis { .. }))
            })
            .map(|m| &m.input)
            .collect();
        assert_eq!(
            axes_in_default,
            vec![&input_axis(2), &input_axis(0), &input_axis(1)]
        );
    }

    #[test]
    fn reorder_within_buttons_does_not_perturb_axes_or_hats() {
        let mut profile = reorder_fixture();
        let axes_before: Vec<InputAddress> = profile
            .mappings()
            .iter()
            .filter(|m| {
                m.mode == "Default" && matches!(m.input.input_id(), Some(InputId::Axis { .. }))
            })
            .map(|m| m.input.clone())
            .collect();
        let hats_before: Vec<InputAddress> = profile
            .mappings()
            .iter()
            .filter(|m| {
                m.mode == "Default" && matches!(m.input.input_id(), Some(InputId::Hat { .. }))
            })
            .map(|m| m.input.clone())
            .collect();

        // Buttons group in Default: [Button0, Button1]. Swap them.
        let moved = profile.reorder_mapping_in_group(&input_button(0), "Default", 1);
        assert!(moved);

        let axes_after: Vec<InputAddress> = profile
            .mappings()
            .iter()
            .filter(|m| {
                m.mode == "Default" && matches!(m.input.input_id(), Some(InputId::Axis { .. }))
            })
            .map(|m| m.input.clone())
            .collect();
        let hats_after: Vec<InputAddress> = profile
            .mappings()
            .iter()
            .filter(|m| {
                m.mode == "Default" && matches!(m.input.input_id(), Some(InputId::Hat { .. }))
            })
            .map(|m| m.input.clone())
            .collect();
        assert_eq!(axes_before, axes_after);
        assert_eq!(hats_before, hats_after);

        let buttons_after: Vec<&InputAddress> = profile
            .mappings()
            .iter()
            .filter(|m| {
                m.mode == "Default" && matches!(m.input.input_id(), Some(InputId::Button { .. }))
            })
            .map(|m| &m.input)
            .collect();
        assert_eq!(buttons_after, vec![&input_button(1), &input_button(0)]);
    }

    #[test]
    fn reorder_clamps_oob_target_to_last_subpos() {
        let mut profile = reorder_fixture();
        // Axes group has 3 elements (subpos 0..2). Target 99 should clamp to 2.
        let moved = profile.reorder_mapping_in_group(&input_axis(0), "Default", 99);
        assert!(moved);
        let axes_after: Vec<&InputAddress> = profile
            .mappings()
            .iter()
            .filter(|m| {
                m.mode == "Default" && matches!(m.input.input_id(), Some(InputId::Axis { .. }))
            })
            .map(|m| &m.input)
            .collect();
        assert_eq!(
            axes_after,
            vec![&input_axis(1), &input_axis(2), &input_axis(0)]
        );
    }

    #[test]
    fn reorder_same_position_is_noop() {
        let mut profile = reorder_fixture();
        let before: Vec<Mapping> = profile.mappings().to_vec();
        let moved = profile.reorder_mapping_in_group(&input_axis(1), "Default", 1);
        assert!(!moved);
        assert_eq!(profile.mappings(), &*before);
    }

    #[test]
    fn reorder_single_element_group_is_noop() {
        let mut profile = reorder_fixture();
        // Hat group in Default has only Hat0; reorder is meaningless.
        let before: Vec<Mapping> = profile.mappings().to_vec();
        let moved = profile.reorder_mapping_in_group(&input_hat(0), "Default", 0);
        assert!(!moved);
        assert_eq!(profile.mappings(), &*before);
    }

    #[test]
    fn reorder_unknown_mapping_is_noop() {
        let mut profile = reorder_fixture();
        let before: Vec<Mapping> = profile.mappings().to_vec();
        let moved = profile.reorder_mapping_in_group(&input_axis(99), "Default", 0);
        assert!(!moved);
        assert_eq!(profile.mappings(), &*before);
    }

    #[test]
    fn reorder_in_one_mode_does_not_touch_other_modes() {
        let mut profile = reorder_fixture();
        let combat_before: Vec<Mapping> = profile
            .mappings()
            .iter()
            .filter(|m| m.mode == "Combat")
            .cloned()
            .collect();
        let moved = profile.reorder_mapping_in_group(&input_axis(0), "Default", 2);
        assert!(moved);
        let combat_after: Vec<Mapping> = profile
            .mappings()
            .iter()
            .filter(|m| m.mode == "Combat")
            .cloned()
            .collect();
        assert_eq!(combat_before, combat_after);
    }

    #[test]
    fn reorder_round_trips_through_toml() {
        let mut profile = reorder_fixture();
        let moved = profile.reorder_mapping_in_group(&input_axis(0), "Default", 2);
        assert!(moved);
        let toml_str = profile.to_toml().unwrap();
        let back = Profile::from_toml(&toml_str).unwrap();
        assert_eq!(profile.mappings(), back.mappings());
    }
}
