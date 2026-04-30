// Rust guideline compliant 2026-03-06

pub mod manager;
mod types;

pub use types::{CalibrationEntry, DeviceEntry, ProfileId, ProfileSettings};

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::action::{Action, Mapping};
use crate::error::{EngineError, Result};
use crate::mode::ModeTree;
use crate::types::{InputAddress, InputId};

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
    match addr.input {
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
/// [profile]
/// id = "550e8400-e29b-41d4-a716-446655440000"
/// name = "My Profile"
/// startup_mode = "Default"
///
/// [[devices]]
/// id = "030000005e040000ea02000000007801"
/// name = "VKB Gladiator NXT Left"
///
/// [modes]
/// Default = ["Combat", "Landing"]
/// Combat = ["Missiles", "Guns"]
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
    modes: ModeTree,
    mappings: Vec<Mapping>,
    calibrations: Vec<CalibrationEntry>,
    settings: ProfileSettings,
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
    profile: ProfileMeta,
    #[serde(default)]
    devices: Vec<DeviceEntry>,
    modes: ModeTree,
    #[serde(default)]
    mappings: Vec<Mapping>,
    #[serde(default)]
    calibrations: Vec<CalibrationEntry>,
}

impl Profile {
    /// Create a new profile with a generated ID.
    #[must_use]
    pub fn new(
        name: String,
        devices: Vec<DeviceEntry>,
        modes: ModeTree,
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
        }
    }

    /// Parse a profile from a TOML string.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::ProfileParse`] on invalid TOML, or
    /// [`EngineError::InvalidConfig`] if validation fails (e.g., startup
    /// mode not in tree, mapping references unknown mode).
    pub fn from_toml(s: &str) -> Result<Self> {
        let raw: ProfileRaw = toml::from_str(s)?;
        Self::from_raw(raw)
    }

    /// Serialize the profile to a pretty-printed TOML string.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::ProfileWrite`] on serialization failure.
    pub fn to_toml(&self) -> Result<String> {
        let raw = self.to_raw();
        Ok(toml::to_string_pretty(&raw)?)
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
        Self::from_toml(&contents)
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

    /// Return the mode tree.
    #[must_use]
    pub fn modes(&self) -> &ModeTree {
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

    /// Replace the mode tree wholesale.
    ///
    /// Caller is responsible for ensuring the new tree is consistent with
    /// `settings().startup_mode()` and any mode names referenced by mappings
    /// or action graphs. Engine handlers do this validation before calling.
    pub fn set_modes(&mut self, modes: ModeTree) {
        self.modes = modes;
    }

    /// Set the profile's startup mode.
    ///
    /// Caller must validate that `mode` exists in the profile's mode tree.
    pub fn set_startup_mode(&mut self, mode: String) {
        self.settings.set_startup_mode(mode);
    }

    /// Drop every mapping whose `mode` field equals `mode`.
    ///
    /// Returns the count of mappings removed; used by tracing events and the
    /// destructive-confirm dialog's affected-mappings count.
    ///
    /// Infallible by contract, `DeleteMode` invokes this in a loop after the
    /// tree mutation has already been applied, and a partial cascade would
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
    /// Pre-validates `CycleModes` for the rename. If applying the rename
    /// would produce a `CycleModes` duplicate, returns the constructor error
    /// without mutating the profile. Caller (`RenameMode` handler) composes
    /// this with `ModeTree::with_renamed` and `set_modes` for the full
    /// cascade.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::InvalidConfig`] if any contained `CycleModes`
    /// would collapse to duplicates after the rename.
    pub fn rename_mode_refs(&mut self, from: &str, to: &str) -> Result<usize> {
        if from == to {
            return Ok(0);
        }
        // Pre-validate cycle-rename safety on a clone of the action graphs.
        // Returns Err early without touching self.
        for mapping in &self.mappings {
            for action in &mapping.actions {
                check_cycle_rename(action, from, to)?;
            }
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

        Ok(touched)
    }

    /// Update the profile display name.
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Validate and convert from the raw TOML representation.
    fn from_raw(raw: ProfileRaw) -> Result<Self> {
        // Validate startup_mode exists in the mode tree.
        if !raw.modes.contains(&raw.profile.startup_mode) {
            return Err(EngineError::InvalidConfig {
                reason: format!(
                    "startup_mode '{}' not found in mode tree",
                    raw.profile.startup_mode
                ),
            });
        }

        // Validate all mapping mode names exist in the tree.
        for mapping in &raw.mappings {
            if !raw.modes.contains(&mapping.mode) {
                return Err(EngineError::InvalidConfig {
                    reason: format!("mapping references unknown mode '{}'", mapping.mode),
                });
            }
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
            profile: ProfileMeta {
                id: self.id.clone(),
                name: self.name.clone(),
                startup_mode: self.settings.startup_mode.clone(),
            },
            devices: self.devices.clone(),
            modes: self.modes.clone(),
            mappings: self.mappings.clone(),
            calibrations: self.calibrations.clone(),
        }
    }
}

/// Walk an action graph; if any cycle action would collapse to duplicates
/// after applying the rename, return the constructor error.
fn check_cycle_rename(action: &Action, from: &str, to: &str) -> Result<()> {
    match action {
        Action::ChangeMode {
            strategy: crate::action::ModeChangeStrategy::Cycle { modes },
        } => {
            modes.with_renamed(from, to)?;
            Ok(())
        }
        Action::Conditional {
            if_true, if_false, ..
        } => {
            for a in if_true {
                check_cycle_rename(a, from, to)?;
            }
            if let Some(branch) = if_false {
                for a in branch {
                    check_cycle_rename(a, from, to)?;
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Walk an action graph in place, rewriting every `from` mode-name reference
/// to `to`. Returns whether any rewrite happened. Cycle pre-validation lives
/// in `check_cycle_rename`, callers must run that first.
fn rewrite_mode_in_action(action: &mut Action, from: &str, to: &str) -> bool {
    use crate::action::ModeChangeStrategy as M;
    match action {
        Action::ChangeMode {
            strategy: M::SwitchTo { mode } | M::Temporary { mode },
        } => {
            if mode == from {
                to.clone_into(mode);
                true
            } else {
                false
            }
        }
        Action::ChangeMode {
            strategy: M::Cycle { modes },
        } => {
            // check_cycle_rename pre-validates this rename; the unwrap
            // below cannot fail provided the caller ran the pre-validation
            // pass.
            let updated = modes
                .with_renamed(from, to)
                .expect("cycle rename pre-validated");
            // Only swap the field when the rename actually touched a
            // member, for profiles with many cycle-bearing mappings that
            // don't reference `from`, this avoids an unnecessary clone
            // round-trip per mapping.
            if updated.modes() == modes.modes() {
                false
            } else {
                *modes = updated;
                true
            }
        }
        Action::Conditional {
            if_true, if_false, ..
        } => {
            let mut changed = false;
            for a in if_true {
                changed |= rewrite_mode_in_action(a, from, to);
            }
            if let Some(branch) = if_false {
                for a in branch {
                    changed |= rewrite_mode_in_action(a, from, to);
                }
            }
            changed
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{Condition, ModeChangeStrategy};
    use crate::processing::DeadzoneConfig;
    use crate::types::{
        DeviceId, InputId, KeyCombo, KeyModifier, MergeOp, OutputAddress, OutputId, VJoyAxis,
    };
    use std::collections::HashMap;

    fn test_modes() -> ModeTree {
        let mut map = HashMap::new();
        map.insert(
            "Default".to_owned(),
            vec!["Combat".to_owned(), "Landing".to_owned()],
        );
        map.insert(
            "Combat".to_owned(),
            vec!["Missiles".to_owned(), "Guns".to_owned()],
        );
        ModeTree::from_adjacency(&map).unwrap()
    }

    fn test_input() -> InputAddress {
        InputAddress {
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
        let mut map = HashMap::new();
        map.insert("Default".to_owned(), vec![]);
        let modes = ModeTree::from_adjacency(&map).unwrap();

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
    fn profile_from_toml_minimal() {
        let toml_str = r#"
[profile]
id = "test-id-1234"
name = "Minimal"
startup_mode = "Default"

[modes]
Default = []

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

[modes]
Default = []
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
[profile]
id = "test-id"
name = "Bad"
startup_mode = "NonExistent"

[modes]
Default = []
"#;
        let err = Profile::from_toml(toml_str).unwrap_err();
        assert!(err.to_string().contains("startup_mode"));
        assert!(err.to_string().contains("NonExistent"));
    }

    #[test]
    fn profile_invalid_mapping_mode() {
        let toml_str = r#"
[profile]
id = "test-id"
name = "Bad"
startup_mode = "Default"

[modes]
Default = []

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
    fn profile_with_mode_tree() {
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
        let mut map = HashMap::new();
        map.insert("Default".to_owned(), vec![]);
        let modes = ModeTree::from_adjacency(&map).unwrap();

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
        let mut map = HashMap::new();
        map.insert("Default".to_owned(), vec![]);
        let modes = ModeTree::from_adjacency(&map).unwrap();

        let profile = Profile::new(
            "Conditional Test".to_owned(),
            vec![],
            modes,
            vec![Mapping {
                input: InputAddress {
                    device: DeviceId("dev-1".to_owned()),
                    input: InputId::Button { index: 0 },
                },
                mode: "Default".to_owned(),
                name: None,
                actions: vec![Action::Conditional {
                    condition: Condition::ButtonPressed {
                        input: InputAddress {
                            device: DeviceId("dev-1".to_owned()),
                            input: InputId::Button { index: 5 },
                        },
                    },
                    if_true: vec![Action::MapToKeyboard {
                        key: KeyCombo {
                            key: "F1".to_owned(),
                            modifiers: vec![KeyModifier::Ctrl],
                        },
                    }],
                    if_false: Some(vec![Action::MapToKeyboard {
                        key: KeyCombo {
                            key: "F1".to_owned(),
                            modifiers: vec![],
                        },
                    }]),
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
                input: InputAddress {
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
        let mut map = HashMap::new();
        map.insert("Default".to_owned(), vec![]);
        let modes = ModeTree::from_adjacency(&map).unwrap();

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
                        second_input: InputAddress {
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
        let mut map = HashMap::new();
        map.insert("Default".to_owned(), vec![]);
        let modes = ModeTree::from_adjacency(&map).unwrap();

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
[profile]
id = "test-id"
name = "No Cals"
startup_mode = "Default"

[modes]
Default = []
"#;
        let profile = Profile::from_toml(toml_str).unwrap();
        assert!(profile.calibrations().is_empty());
    }

    #[test]
    fn profile_with_invalid_calibration_rejected() {
        let toml_str = r#"
[profile]
id = "test-id"
name = "Bad Cal"
startup_mode = "Default"

[modes]
Default = []

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

        let other_input = InputAddress {
            device: DeviceId("other-dev".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        assert!(profile.find_mapping(&other_input, "Default").is_none());
    }

    #[test]
    fn set_mapping_creates_new() {
        let mut profile = minimal_profile();
        let new_input = InputAddress {
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
        let new_input = InputAddress {
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

    // --- set_modes / remove_mappings_for_mode ---

    #[test]
    fn set_modes_replaces_tree() {
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
                    input: InputAddress {
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

        let touched = profile.rename_mode_refs("Combat", "Fighter").unwrap();
        assert_eq!(touched, 1);
        assert_eq!(profile.mappings()[0].mode, "Fighter");
        // startup_mode unchanged because it referenced Default, not Combat.
        assert_eq!(profile.settings().startup_mode(), "Default");

        let touched_default = profile.rename_mode_refs("Default", "Root").unwrap();
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

        let touched = profile.rename_mode_refs("Combat", "Fighter").unwrap();
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
    fn rename_mode_refs_rewrites_cycle_entries() {
        use crate::action::{CycleModes, Mapping, ModeChangeStrategy};

        let modes = test_modes();
        let mut profile = Profile::new(
            "Cycler".to_owned(),
            vec![],
            modes,
            vec![Mapping {
                input: test_input(),
                mode: "Default".to_owned(),
                name: None,
                actions: vec![Action::ChangeMode {
                    strategy: ModeChangeStrategy::Cycle {
                        modes: CycleModes::new(vec!["Combat".to_owned(), "Landing".to_owned()])
                            .unwrap(),
                    },
                }],
            }],
            vec![],
            "Default".to_owned(),
        );

        let touched = profile.rename_mode_refs("Combat", "Fighter").unwrap();
        assert_eq!(touched, 1);
        match &profile.mappings()[0].actions[0] {
            Action::ChangeMode {
                strategy: ModeChangeStrategy::Cycle { modes },
            } => assert_eq!(modes.modes(), &["Fighter".to_owned(), "Landing".to_owned()]),
            _ => panic!("expected Cycle"),
        }
    }

    #[test]
    fn rename_mode_refs_rejects_cycle_collision() {
        use crate::action::{CycleModes, Mapping, ModeChangeStrategy};

        let modes = test_modes();
        let mut profile = Profile::new(
            "BadCycle".to_owned(),
            vec![],
            modes,
            vec![Mapping {
                input: test_input(),
                mode: "Default".to_owned(),
                name: None,
                actions: vec![Action::ChangeMode {
                    strategy: ModeChangeStrategy::Cycle {
                        modes: CycleModes::new(vec!["Combat".to_owned(), "Landing".to_owned()])
                            .unwrap(),
                    },
                }],
            }],
            vec![],
            "Default".to_owned(),
        );

        // Renaming Combat → Landing would collapse the cycle into a duplicate.
        let err = profile.rename_mode_refs("Combat", "Landing").unwrap_err();
        assert!(err.to_string().contains("duplicate"));

        // Profile must be unchanged on error.
        match &profile.mappings()[0].actions[0] {
            Action::ChangeMode {
                strategy: ModeChangeStrategy::Cycle { modes },
            } => assert_eq!(modes.modes(), &["Combat".to_owned(), "Landing".to_owned()]),
            _ => panic!("expected unchanged Cycle"),
        }
        assert_eq!(profile.mappings()[0].mode, "Default");
    }

    #[test]
    fn rename_mode_refs_atomic_when_cycle_collides() {
        // Locks the contract that pre-validation precedes any mutation. If a
        // later cycle-mapping would collapse, every prior mapping's `mode`
        // field AND every prior SwitchTo action must remain byte-identical to
        // the pre-call clone.
        use crate::action::{CycleModes, Mapping, ModeChangeStrategy};

        let modes = test_modes();
        let mappings = vec![
            // Earlier mappings reference `Combat` so they would be rewritten
            // if the cascade weren't atomic.
            Mapping {
                input: test_input(),
                mode: "Combat".to_owned(),
                name: None,
                actions: vec![Action::ChangeMode {
                    strategy: ModeChangeStrategy::SwitchTo {
                        mode: "Combat".to_owned(),
                    },
                }],
            },
            Mapping {
                input: test_input(),
                mode: "Combat".to_owned(),
                name: None,
                actions: vec![Action::Invert],
            },
            // Last mapping holds the colliding cycle.
            Mapping {
                input: test_input(),
                mode: "Default".to_owned(),
                name: None,
                actions: vec![Action::ChangeMode {
                    strategy: ModeChangeStrategy::Cycle {
                        modes: CycleModes::new(vec!["Combat".to_owned(), "Landing".to_owned()])
                            .unwrap(),
                    },
                }],
            },
        ];
        let mut profile = Profile::new(
            "Atomic".to_owned(),
            vec![],
            modes,
            mappings.clone(),
            vec![],
            "Default".to_owned(),
        );

        let err = profile.rename_mode_refs("Combat", "Landing").unwrap_err();
        assert!(err.to_string().contains("duplicate"));

        // Every prior mapping must be byte-identical to its pre-call clone.
        assert_eq!(
            profile.mappings(),
            &*mappings,
            "no mapping may be rewritten when pre-validation rejects the rename"
        );
        assert_eq!(
            profile.settings().startup_mode(),
            "Default",
            "startup_mode must be unchanged"
        );
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
                    if_false: Some(vec![Action::ChangeMode {
                        strategy: ModeChangeStrategy::Temporary {
                            mode: "Combat".to_owned(),
                        },
                    }]),
                }],
            }],
            vec![],
            "Default".to_owned(),
        );

        let touched = profile.rename_mode_refs("Combat", "Fighter").unwrap();
        assert_eq!(touched, 1);
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
        let target = InputAddress {
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
}
