// Application-level settings (persisted outside profiles)
// Rust guideline compliant 2026-04-28

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::snapshot::SnapshotConfig;
use crate::types::{DeviceDiagnostics, DeviceId, DeviceInfo};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceRecord {
    pub info: DeviceInfo,
    #[serde(default)]
    pub diagnostics: DeviceDiagnostics,
    #[serde(default)]
    pub last_seen_unix_ms: Option<u64>,
}

/// Startup preferences (F16): launch at OS sign-in and start minimized to tray.
///
/// Both fields default to `false`. The outer `#[serde(default)]` on
/// `AppSettings.startup` plus the inner `#[serde(default)]` on each field
/// lets pre-F16 `settings.toml` files load with no migration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartupSettings {
    #[serde(default)]
    pub launch_at_startup: bool,
    #[serde(default)]
    pub start_minimized_to_tray: bool,
}

/// Application-wide settings persisted between sessions.
///
/// Stored as TOML at `<config_dir>/inputforge/settings.toml`
/// (on Windows this is typically `%APPDATA%/inputforge/settings.toml`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSettings {
    /// Path to the last loaded profile, if any.
    pub last_profile: Option<PathBuf>,

    /// Snapshot subsystem configuration.
    ///
    /// Persisted as a `[snapshot]` sub-table in `settings.toml`; users can
    /// hand-edit values directly. F15 will ship a typed UI editor on top of
    /// this. Missing `[snapshot]` table (pre-F6 files) loads with defaults
    /// via `#[serde(default)]`.
    #[serde(default)]
    pub snapshot: SnapshotConfig,

    /// Startup preferences (F16). Persisted as a `[startup]` sub-table.
    #[serde(default)]
    pub startup: StartupSettings,

    #[serde(default)]
    pub device_aliases: HashMap<DeviceId, String>,

    #[serde(default)]
    pub device_registry: HashMap<DeviceId, DeviceRecord>,
}

impl AppSettings {
    /// Return the application configuration directory.
    ///
    /// On Windows this is `%APPDATA%/inputforge/`.
    ///
    /// # Panics
    ///
    /// Panics if the OS config directory cannot be determined.
    #[must_use]
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .expect("OS config directory not available; cannot determine settings location")
            .join("inputforge")
    }

    /// Return the directory where user profiles are stored.
    ///
    /// Equivalent to `<config_dir>/profiles/`.
    #[must_use]
    pub fn profiles_dir() -> PathBuf {
        Self::config_dir().join("profiles")
    }

    /// Return the path to the settings file.
    ///
    /// Equivalent to `<config_dir>/settings.toml`.
    #[must_use]
    pub fn settings_path() -> PathBuf {
        Self::config_dir().join("settings.toml")
    }

    /// Load settings from the default settings path.
    ///
    /// Returns [`Default`] settings if the file is missing or cannot be
    /// parsed, logging a warning via [`tracing`].
    #[must_use]
    pub fn load() -> Self {
        Self::load_from(&Self::settings_path())
    }

    /// Load settings from the given path.
    ///
    /// Returns [`Default`] settings if the file is missing or cannot be
    /// parsed, logging a warning via [`tracing`].
    #[must_use]
    pub fn load_from(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(settings) => settings,
                Err(e) => {
                    tracing::warn!("failed to parse settings at {}: {e}", path.display());
                    Self::default()
                }
            },
            Err(e) => {
                tracing::warn!("failed to read settings at {}: {e}", path.display());
                Self::default()
            }
        }
    }

    /// Persist settings to the default settings path.
    ///
    /// Creates the configuration directory if it does not exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created, the settings
    /// cannot be serialized, or the file cannot be written.
    pub fn save(&self) -> Result<()> {
        let dir = Self::config_dir();
        std::fs::create_dir_all(&dir)?;
        self.save_to(&Self::settings_path())
    }

    /// Persist settings to the given path.
    ///
    /// Creates the parent directory of `path` if it does not exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the parent directory cannot be created, the
    /// settings cannot be serialized, or the file cannot be written.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let toml_str = toml::to_string_pretty(self)?;
        std::fs::write(path, toml_str)?;
        Ok(())
    }

    #[must_use]
    pub fn display_name_for(&self, info: &DeviceInfo) -> String {
        display_name_for_device(&self.device_aliases, info)
    }

    pub fn set_device_alias(&mut self, device: DeviceId, alias: Option<String>) {
        match alias
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
        {
            Some(alias) => {
                self.device_aliases.insert(device, alias);
            }
            None => {
                self.device_aliases.remove(&device);
            }
        }
    }
}

/// Resolve a device's user-facing display name.
///
/// Single source of truth for the alias / hardware-name / id-fallback
/// rule. Used by `AppSettings::display_name_for` (the engine-side
/// owner of `device_aliases`) and by GUI snapshot code that mirrors
/// the same map onto `AppState.device_aliases`. Keep both call sites
/// delegating here so the rule never drifts.
///
/// Precedence:
///
/// 1. `aliases.get(&info.id)` when present and non-blank after trim.
/// 2. `info.name` when non-blank after trim.
/// 3. `info.id.0` as a last-resort identifier so the returned string
///    is never empty.
#[must_use]
pub fn display_name_for_device<S: std::hash::BuildHasher>(
    aliases: &HashMap<DeviceId, String, S>,
    info: &DeviceInfo,
) -> String {
    aliases
        .get(&info.id)
        .filter(|alias| !alias.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| {
            if info.name.trim().is_empty() {
                info.id.0.clone()
            } else {
                info.name.clone()
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DeviceConnectionState;

    #[test]
    fn config_dir_ends_with_inputforge() {
        let dir = AppSettings::config_dir();
        assert!(
            dir.ends_with("inputforge"),
            "config_dir should end with 'inputforge', got: {dir:?}"
        );
    }

    #[test]
    fn profiles_dir_is_under_config_dir() {
        let profiles = AppSettings::profiles_dir();
        let config = AppSettings::config_dir();
        assert!(
            profiles.starts_with(&config),
            "profiles_dir should be under config_dir"
        );
        assert!(
            profiles.ends_with("profiles"),
            "profiles_dir should end with 'profiles'"
        );
    }

    #[test]
    fn settings_path_is_under_config_dir() {
        let path = AppSettings::settings_path();
        let config = AppSettings::config_dir();
        assert!(
            path.starts_with(&config),
            "settings_path should be under config_dir"
        );
        assert_eq!(
            path.file_name().and_then(|n| n.to_str()),
            Some("settings.toml")
        );
    }

    #[test]
    fn default_settings_has_no_last_profile() {
        let settings = AppSettings::default();
        assert!(settings.last_profile.is_none());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let settings_path = tmp.path().join("settings.toml");

        let settings = AppSettings {
            last_profile: Some(PathBuf::from("C:/profiles/my_profile.toml")),
            ..Default::default()
        };

        settings.save_to(&settings_path).unwrap();

        let loaded = AppSettings::load_from(&settings_path);
        assert_eq!(settings, loaded);
    }

    #[test]
    fn load_returns_default_on_missing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let nonexistent = tmp.path().join("does_not_exist.toml");

        let loaded = AppSettings::load_from(&nonexistent);
        assert_eq!(loaded, AppSettings::default());
    }

    #[test]
    fn serde_roundtrip_with_none() {
        let settings = AppSettings {
            last_profile: None,
            ..Default::default()
        };
        let toml_str = toml::to_string(&settings).unwrap();
        let back: AppSettings = toml::from_str(&toml_str).unwrap();
        assert_eq!(settings, back);
    }

    #[test]
    fn serde_roundtrip_with_path() {
        let settings = AppSettings {
            last_profile: Some(PathBuf::from("/some/path/profile.toml")),
            ..Default::default()
        };
        let toml_str = toml::to_string(&settings).unwrap();
        let back: AppSettings = toml::from_str(&toml_str).unwrap();
        assert_eq!(settings, back);
    }

    /// Test save, load, and invalid-TOML recovery using a temp directory.
    #[test]
    fn save_load_and_invalid_toml_recovery() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.toml");

        // 1. save_to() creates the parent directory and file.
        let settings = AppSettings {
            last_profile: Some(PathBuf::from("test_profile.toml")),
            ..Default::default()
        };
        settings.save_to(&path).unwrap();
        assert!(path.exists(), "settings file should exist after save_to");

        // 2. load_from() roundtrips correctly.
        let loaded = AppSettings::load_from(&path);
        assert_eq!(settings, loaded);

        // 3. load_from() returns Default when the file contains garbage.
        std::fs::write(&path, "this is not valid toml {{{{").unwrap();
        let loaded = AppSettings::load_from(&path);
        assert_eq!(loaded, AppSettings::default());
    }

    #[test]
    fn settings_default_has_default_snapshot_config() {
        let s = AppSettings::default();
        assert_eq!(s.snapshot, SnapshotConfig::default());
    }

    #[test]
    fn pre_f6_settings_loads_with_default_snapshot_table() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.toml");
        // Write a pre-F6 file: no [snapshot] table.
        std::fs::write(&path, "last_profile = \"C:/foo.toml\"\n").unwrap();

        let loaded = AppSettings::load_from(&path);
        assert_eq!(loaded.snapshot, SnapshotConfig::default());
        assert_eq!(loaded.last_profile, Some(PathBuf::from("C:/foo.toml")));
    }

    #[test]
    fn settings_round_trips_device_aliases_and_registry() {
        let device = DeviceId("030000005e0400008e02000000000000".to_owned());
        let mut settings = AppSettings::default();
        settings
            .device_aliases
            .insert(device.clone(), "Wheel Base".to_owned());
        settings.device_registry.insert(
            device.clone(),
            DeviceRecord {
                info: DeviceInfo {
                    id: device.clone(),
                    name: "SDL Wheel".to_owned(),
                    axes: 6,
                    buttons: 32,
                    hats: 1,
                    instance_path: Some(r"\\?\hid#vid_045e&pid_028e".to_owned()),
                    axis_polarities: vec![],
                },
                diagnostics: DeviceDiagnostics {
                    vendor_id: Some(0x045e),
                    product_id: Some(0x028e),
                    connection_state: Some(DeviceConnectionState::Wired),
                    ..DeviceDiagnostics::default()
                },
                last_seen_unix_ms: Some(1_714_200_000_000),
            },
        );

        let toml = toml::to_string_pretty(&settings).expect("settings serialize");
        let loaded: AppSettings = toml::from_str(&toml).expect("settings deserialize");

        assert_eq!(
            loaded.device_aliases.get(&device),
            Some(&"Wheel Base".to_owned())
        );
        assert_eq!(
            loaded
                .device_registry
                .get(&device)
                .map(|record| record.info.name.as_str()),
            Some("SDL Wheel")
        );
    }

    #[test]
    fn settings_round_trips_with_custom_snapshot_table() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.toml");

        let s = AppSettings {
            last_profile: None,
            snapshot: SnapshotConfig {
                max_count: 7,
                skip_if_unchanged: false,
            },
            ..Default::default()
        };
        s.save_to(&path).unwrap();

        let body = std::fs::read_to_string(&path).unwrap();
        assert!(
            body.contains("[snapshot]"),
            "expected [snapshot] table on disk; got: {body}"
        );

        let loaded = AppSettings::load_from(&path);
        assert_eq!(loaded, s);
    }

    fn info_with(id: &str, name: &str) -> DeviceInfo {
        DeviceInfo {
            id: DeviceId(id.to_owned()),
            name: name.to_owned(),
            axes: 0,
            buttons: 0,
            hats: 0,
            instance_path: None,
            axis_polarities: Vec::new(),
        }
    }

    #[test]
    fn display_name_for_device_returns_alias_when_present() {
        let mut aliases = HashMap::new();
        aliases.insert(DeviceId("dev-1".to_owned()), "Throttle Quadrant".to_owned());
        let info = info_with("dev-1", "Generic HID Joystick");
        assert_eq!(
            display_name_for_device(&aliases, &info),
            "Throttle Quadrant"
        );
    }

    #[test]
    fn display_name_for_device_falls_back_to_hardware_name_when_alias_blank() {
        let mut aliases = HashMap::new();
        aliases.insert(DeviceId("dev-1".to_owned()), "   ".to_owned());
        let info = info_with("dev-1", "Generic HID Joystick");
        assert_eq!(
            display_name_for_device(&aliases, &info),
            "Generic HID Joystick"
        );
    }

    #[test]
    fn display_name_for_device_falls_back_to_hardware_name_when_no_alias() {
        let aliases = HashMap::new();
        let info = info_with("dev-1", "Generic HID Joystick");
        assert_eq!(
            display_name_for_device(&aliases, &info),
            "Generic HID Joystick"
        );
    }

    #[test]
    fn display_name_for_device_falls_back_to_id_when_alias_and_name_blank() {
        let aliases = HashMap::new();
        let info = info_with("dev-1", "   ");
        assert_eq!(display_name_for_device(&aliases, &info), "dev-1");
    }

    #[test]
    fn app_settings_display_name_for_delegates_to_free_function() {
        let mut s = AppSettings::default();
        s.device_aliases
            .insert(DeviceId("dev-1".to_owned()), "Pedals".to_owned());
        let info = info_with("dev-1", "VKB Wheel Pedals");
        // The method MUST produce the same value the free function
        // would; this guards against the duplication ever drifting.
        assert_eq!(
            s.display_name_for(&info),
            display_name_for_device(&s.device_aliases, &info)
        );
        assert_eq!(s.display_name_for(&info), "Pedals");
    }

    #[test]
    fn settings_default_has_default_startup() {
        let s = AppSettings::default();
        assert_eq!(s.startup, StartupSettings::default());
        assert!(!s.startup.launch_at_startup);
        assert!(!s.startup.start_minimized_to_tray);
    }

    #[test]
    fn pre_f16_settings_loads_with_default_startup() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.toml");
        // Pre-F16: no [startup] table.
        std::fs::write(&path, "last_profile = \"C:/foo.toml\"\n").unwrap();

        let loaded = AppSettings::load_from(&path);
        assert_eq!(loaded.startup, StartupSettings::default());
    }

    #[test]
    fn pre_f16_settings_loads_with_partial_startup_table() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.toml");
        // [startup] present but missing one field.
        std::fs::write(&path, "[startup]\nlaunch_at_startup = true\n").unwrap();

        let loaded = AppSettings::load_from(&path);
        assert!(loaded.startup.launch_at_startup);
        assert!(!loaded.startup.start_minimized_to_tray);
    }

    #[test]
    fn settings_round_trips_startup_table() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.toml");

        let s = AppSettings {
            startup: StartupSettings {
                launch_at_startup: true,
                start_minimized_to_tray: true,
            },
            ..Default::default()
        };
        s.save_to(&path).unwrap();

        let body = std::fs::read_to_string(&path).unwrap();
        assert!(
            body.contains("[startup]"),
            "expected [startup] table on disk; got: {body}"
        );

        let loaded = AppSettings::load_from(&path);
        assert_eq!(loaded, s);
    }
}
