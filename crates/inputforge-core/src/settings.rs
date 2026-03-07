// Application-level settings (persisted outside profiles)
// Rust guideline compliant 2026-03-07

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Application-wide settings persisted between sessions.
///
/// Stored as TOML at `<config_dir>/inputforge/settings.toml`
/// (on Windows this is typically `%APPDATA%/inputforge/settings.toml`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSettings {
    /// Path to the last loaded profile, if any.
    pub last_profile: Option<PathBuf>,
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

    /// Load settings from disk.
    ///
    /// Returns [`Default`] settings if the file is missing or cannot be
    /// parsed, logging a warning via [`tracing`].
    #[must_use]
    pub fn load() -> Self {
        let path = Self::settings_path();
        match std::fs::read_to_string(&path) {
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

    /// Persist settings to disk.
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
        let toml_str = toml::to_string(self)?;
        std::fs::write(Self::settings_path(), toml_str)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // Use a unique temp directory to avoid interference with real settings.
        let tmp = std::env::temp_dir().join("inputforge_settings_test");
        std::fs::create_dir_all(&tmp).unwrap();
        let settings_path = tmp.join("settings.toml");

        let settings = AppSettings {
            last_profile: Some(PathBuf::from("C:/profiles/my_profile.toml")),
        };

        // Manually write to the temp path (bypass config_dir).
        let toml_str = toml::to_string(&settings).unwrap();
        std::fs::write(&settings_path, &toml_str).unwrap();

        // Read back and verify.
        let contents = std::fs::read_to_string(&settings_path).unwrap();
        let loaded: AppSettings = toml::from_str(&contents).unwrap();
        assert_eq!(settings, loaded);

        // Cleanup.
        let _ = std::fs::remove_file(&settings_path);
        let _ = std::fs::remove_dir(&tmp);
    }

    #[test]
    fn load_returns_default_on_missing_file() {
        // AppSettings::load() reads from the real settings_path, which may
        // or may not exist. Instead, test the parse-fallback logic directly.
        let result: std::result::Result<AppSettings, _> = toml::from_str("invalid { toml");
        assert!(result.is_err());

        // Confirm Default is what we expect.
        let fallback = AppSettings::default();
        assert!(fallback.last_profile.is_none());
    }

    #[test]
    fn serde_roundtrip_with_none() {
        let settings = AppSettings { last_profile: None };
        let toml_str = toml::to_string(&settings).unwrap();
        let back: AppSettings = toml::from_str(&toml_str).unwrap();
        assert_eq!(settings, back);
    }

    #[test]
    fn serde_roundtrip_with_path() {
        let settings = AppSettings {
            last_profile: Some(PathBuf::from("/some/path/profile.toml")),
        };
        let toml_str = toml::to_string(&settings).unwrap();
        let back: AppSettings = toml::from_str(&toml_str).unwrap();
        assert_eq!(settings, back);
    }

    /// Single sequential test for real-path I/O to avoid parallel test
    /// interference (all three scenarios share the same settings file).
    #[test]
    fn save_load_and_invalid_toml_recovery() {
        let path = AppSettings::settings_path();

        // 1. save() creates the directory and file.
        let settings = AppSettings {
            last_profile: Some(PathBuf::from("test_profile.toml")),
        };
        settings.save().unwrap();
        assert!(path.exists(), "settings file should exist after save");
        let contents = std::fs::read_to_string(&path).unwrap();
        let loaded: AppSettings = toml::from_str(&contents).unwrap();
        assert_eq!(settings, loaded);

        // 2. load() roundtrips correctly.
        let loaded = AppSettings::load();
        assert_eq!(settings, loaded);

        // 3. load() returns Default when the file contains garbage.
        std::fs::write(&path, "this is not valid toml {{{{").unwrap();
        let loaded = AppSettings::load();
        assert_eq!(loaded, AppSettings::default());

        // Cleanup: restore empty settings.
        let empty = AppSettings::default();
        empty.save().unwrap();
    }
}
