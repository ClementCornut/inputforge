// Rust guideline compliant 2026-03-07

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{EngineError, Result};
use crate::mode::ModeTree;
use crate::settings::AppSettings;

use super::Profile;

/// Summary of a profile on disk (name + path, no TOML parsing).
#[derive(Debug, Clone)]
pub struct ProfileSummary {
    /// Display name derived from the filename stem.
    pub name: String,
    /// Absolute path to the `.toml` file.
    pub path: PathBuf,
}

/// Characters that are illegal in filenames on Windows/NTFS.
const ILLEGAL_CHARS: &[char] = &[':', '\\', '/', '*', '?', '"', '<', '>', '|'];

/// Sanitize a profile name into a safe filename (without extension).
///
/// Replaces any character that is not alphanumeric, dash, underscore, or
/// space with `_`, then trims leading and trailing underscores.
#[must_use]
pub fn sanitize_filename(name: &str) -> String {
    let raw: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();
    raw.trim_matches('_').to_owned()
}

/// Validate that a profile name is acceptable.
///
/// # Errors
///
/// Returns [`EngineError::InvalidConfig`] if the name is empty after
/// trimming or contains filesystem-illegal characters
/// (`: \ / * ? " < > |`).
pub fn validate_profile_name(name: &str) -> Result<()> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(EngineError::InvalidConfig {
            reason: "profile name cannot be empty".to_owned(),
        });
    }
    for ch in ILLEGAL_CHARS {
        if trimmed.contains(*ch) {
            return Err(EngineError::InvalidConfig {
                reason: format!("profile name contains illegal character '{ch}'"),
            });
        }
    }
    Ok(())
}

/// List all profiles in the profiles directory.
///
/// Scans [`AppSettings::profiles_dir()`] for `.toml` files and returns a
/// [`ProfileSummary`] for each, sorted alphabetically by name. No TOML
/// parsing is performed, the display name is taken from the filename stem.
///
/// # Errors
///
/// Returns [`EngineError::Io`] if the profiles directory cannot be read.
/// Returns an empty list (not an error) if the directory does not exist.
pub fn list_profiles() -> Result<Vec<ProfileSummary>> {
    list_profiles_in(&AppSettings::profiles_dir())
}

/// List all profiles in a specific directory.
///
/// # Errors
///
/// Returns [`EngineError::Io`] if the directory cannot be read.
/// Returns an empty list (not an error) if the directory does not exist.
pub(crate) fn list_profiles_in(dir: &Path) -> Result<Vec<ProfileSummary>> {
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut summaries = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            let name = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            summaries.push(ProfileSummary { name, path });
        }
    }
    summaries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(summaries)
}

/// Create a minimal profile with the given name and save it.
///
/// The profile contains no devices, no mappings, no calibrations, and a
/// single `"Default"` mode. The file is saved to
/// `profiles_dir()/{sanitized_name}.toml`.
///
/// # Errors
///
/// Returns [`EngineError::InvalidConfig`] if the name fails validation,
/// or [`EngineError::Io`] / [`EngineError::ProfileWrite`] on I/O or
/// serialization failures.
pub fn create_profile(name: &str) -> Result<PathBuf> {
    create_profile_in(name, &AppSettings::profiles_dir())
}

/// Create a minimal profile in a specific directory.
///
/// # Errors
///
/// Returns [`EngineError::InvalidConfig`] if the name fails validation,
/// or [`EngineError::Io`] / [`EngineError::ProfileWrite`] on I/O or
/// serialization failures.
pub(crate) fn create_profile_in(name: &str, dir: &Path) -> Result<PathBuf> {
    validate_profile_name(name)?;

    let sanitized = sanitize_filename(name);
    if sanitized.is_empty() {
        return Err(EngineError::InvalidConfig {
            reason: "profile name is empty after sanitization".to_owned(),
        });
    }

    let filename = format!("{sanitized}.toml");
    let path = dir.join(filename);

    std::fs::create_dir_all(dir)?;

    if path.exists() {
        return Err(EngineError::InvalidConfig {
            reason: format!("a profile named '{name}' already exists"),
        });
    }

    let mut map = HashMap::new();
    map.insert("Default".to_owned(), vec![]);
    #[expect(
        clippy::unwrap_used,
        reason = "single-node adjacency map is always valid"
    )]
    let modes = ModeTree::from_adjacency(&map).unwrap();

    let profile = Profile::new(
        name.to_owned(),
        vec![],
        modes,
        vec![],
        vec![],
        "Default".to_owned(),
    );
    profile.save(&path)?;

    Ok(path)
}

/// Ensure at least one profile exists, creating a `"Default"` profile if
/// the profiles directory is empty.
///
/// Returns the path of the default (or first alphabetical) profile.
///
/// # Errors
///
/// Returns [`EngineError::Io`] / [`EngineError::ProfileWrite`] on I/O or
/// serialization failures.
pub fn ensure_default_profile() -> Result<PathBuf> {
    ensure_default_profile_in(&AppSettings::profiles_dir())
}

/// Ensure at least one profile exists in a specific directory.
///
/// # Errors
///
/// Returns [`EngineError::Io`] / [`EngineError::ProfileWrite`] on I/O or
/// serialization failures.
pub(crate) fn ensure_default_profile_in(dir: &Path) -> Result<PathBuf> {
    let profiles = list_profiles_in(dir)?;
    if profiles.is_empty() {
        create_profile_in("Default", dir)
    } else {
        Ok(profiles.into_iter().next().expect("non-empty").path)
    }
}

/// Rename a profile on disk and update its internal name field.
///
/// Computes the new path from `sanitize_filename(new_name)` in the same
/// parent directory, performs an atomic `std::fs::rename`, then loads the
/// profile from the new path, updates the internal name, and saves it
/// back.
///
/// # Errors
///
/// Returns [`EngineError::InvalidConfig`] if `new_name` fails validation,
/// or [`EngineError::Io`] on rename / I/O failures.
pub fn rename_profile(path: &Path, new_name: &str) -> Result<PathBuf> {
    validate_profile_name(new_name)?;

    let sanitized = sanitize_filename(new_name);
    if sanitized.is_empty() {
        return Err(EngineError::InvalidConfig {
            reason: "profile name is empty after sanitization".to_owned(),
        });
    }

    let parent = path.parent().ok_or_else(|| EngineError::InvalidConfig {
        reason: "profile path has no parent directory".to_owned(),
    })?;

    let new_filename = format!("{sanitized}.toml");
    let new_path = parent.join(new_filename);

    // Allow case-change renames (same file on case-insensitive filesystems)
    // but reject if a different file already occupies the destination.
    if new_path.exists() && new_path != path {
        return Err(EngineError::InvalidConfig {
            reason: format!("a profile named '{new_name}' already exists"),
        });
    }

    std::fs::rename(path, &new_path)?;

    let mut profile = Profile::load(&new_path)?;
    profile.set_name(new_name.to_owned());
    profile.save(&new_path)?;

    Ok(new_path)
}

/// Delete a profile file from disk.
///
/// # Errors
///
/// Returns [`EngineError::Io`] if the file cannot be removed.
pub fn delete_profile(path: &Path) -> Result<()> {
    std::fs::remove_file(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- sanitize_filename ---

    #[test]
    fn sanitize_basic_name() {
        assert_eq!(sanitize_filename("My Profile"), "My Profile");
    }

    #[test]
    fn sanitize_with_illegal_chars() {
        assert_eq!(sanitize_filename("a:b/c\\d"), "a_b_c_d");
    }

    #[test]
    fn sanitize_preserves_dashes_and_underscores() {
        assert_eq!(sanitize_filename("my-profile_v2"), "my-profile_v2");
    }

    #[test]
    fn sanitize_trims_leading_trailing_underscores() {
        assert_eq!(sanitize_filename("***name***"), "name");
    }

    #[test]
    fn sanitize_all_special_chars() {
        assert_eq!(sanitize_filename("***"), "");
    }

    #[test]
    fn sanitize_empty_input() {
        assert_eq!(sanitize_filename(""), "");
    }

    // --- validate_profile_name ---

    #[test]
    fn validate_accepts_normal_name() {
        validate_profile_name("My Profile").unwrap();
    }

    #[test]
    fn validate_accepts_name_with_dashes() {
        validate_profile_name("flight-sim-v2").unwrap();
    }

    #[test]
    fn validate_rejects_empty_name() {
        let err = validate_profile_name("").unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn validate_rejects_whitespace_only() {
        let err = validate_profile_name("   ").unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn validate_rejects_colon() {
        let err = validate_profile_name("a:b").unwrap_err();
        assert!(err.to_string().contains(':'));
    }

    #[test]
    fn validate_rejects_backslash() {
        let err = validate_profile_name("a\\b").unwrap_err();
        assert!(err.to_string().contains('\\'));
    }

    #[test]
    fn validate_rejects_forward_slash() {
        let err = validate_profile_name("a/b").unwrap_err();
        assert!(err.to_string().contains('/'));
    }

    #[test]
    fn validate_rejects_star() {
        let err = validate_profile_name("a*b").unwrap_err();
        assert!(err.to_string().contains('*'));
    }

    #[test]
    fn validate_rejects_question_mark() {
        let err = validate_profile_name("a?b").unwrap_err();
        assert!(err.to_string().contains('?'));
    }

    #[test]
    fn validate_rejects_double_quote() {
        let err = validate_profile_name("a\"b").unwrap_err();
        assert!(err.to_string().contains('"'));
    }

    #[test]
    fn validate_rejects_angle_brackets() {
        assert!(validate_profile_name("a<b").is_err());
        assert!(validate_profile_name("a>b").is_err());
    }

    #[test]
    fn validate_rejects_pipe() {
        let err = validate_profile_name("a|b").unwrap_err();
        assert!(err.to_string().contains('|'));
    }

    // --- create_profile ---

    #[test]
    fn create_profile_writes_toml_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = create_profile_in("Test Profile", tmp.path()).unwrap();

        assert!(path.exists());
        assert_eq!(
            path.file_name().unwrap().to_str().unwrap(),
            "Test Profile.toml"
        );

        let profile = Profile::load(&path).unwrap();
        assert_eq!(profile.name(), "Test Profile");
        assert!(profile.devices().is_empty());
        assert!(profile.mappings().is_empty());
        assert!(profile.calibrations().is_empty());
        assert_eq!(profile.settings().startup_mode(), "Default");
    }

    #[test]
    fn create_profile_rejects_invalid_name() {
        let tmp = tempfile::tempdir().unwrap();
        let err = create_profile_in("bad:name", tmp.path()).unwrap_err();
        assert!(err.to_string().contains("illegal character"));
    }

    #[test]
    fn create_profile_creates_directory_if_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let subdir = tmp.path().join("nested").join("profiles");
        let path = create_profile_in("Nested", &subdir).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn create_profile_rejects_dot_name() {
        let tmp = tempfile::tempdir().unwrap();
        let err = create_profile_in(".", tmp.path()).unwrap_err();
        assert!(err.to_string().contains("empty after sanitization"));
    }

    #[test]
    fn create_profile_rejects_dot_dot_name() {
        let tmp = tempfile::tempdir().unwrap();
        let err = create_profile_in("..", tmp.path()).unwrap_err();
        assert!(err.to_string().contains("empty after sanitization"));
    }

    #[test]
    fn create_profile_rejects_duplicate_name() {
        let tmp = tempfile::tempdir().unwrap();
        create_profile_in("Duplicate", tmp.path()).unwrap();
        let err = create_profile_in("Duplicate", tmp.path()).unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    // --- list_profiles ---

    #[test]
    fn list_profiles_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles = list_profiles_in(tmp.path()).unwrap();
        assert!(profiles.is_empty());
    }

    #[test]
    fn list_profiles_nonexistent_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles = list_profiles_in(&tmp.path().join("nonexistent")).unwrap();
        assert!(profiles.is_empty());
    }

    #[test]
    fn list_profiles_finds_toml_files() {
        let tmp = tempfile::tempdir().unwrap();
        create_profile_in("Bravo", tmp.path()).unwrap();
        create_profile_in("Alpha", tmp.path()).unwrap();
        create_profile_in("Charlie", tmp.path()).unwrap();

        let profiles = list_profiles_in(tmp.path()).unwrap();
        assert_eq!(profiles.len(), 3);
        assert_eq!(profiles[0].name, "Alpha");
        assert_eq!(profiles[1].name, "Bravo");
        assert_eq!(profiles[2].name, "Charlie");
    }

    #[test]
    fn list_profiles_ignores_non_toml_files() {
        let tmp = tempfile::tempdir().unwrap();
        create_profile_in("Real", tmp.path()).unwrap();
        std::fs::write(tmp.path().join("notes.txt"), "not a profile").unwrap();
        std::fs::write(tmp.path().join("data.json"), "{}").unwrap();

        let profiles = list_profiles_in(tmp.path()).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].name, "Real");
    }

    // --- ensure_default_profile ---

    #[test]
    fn ensure_default_creates_when_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let path = ensure_default_profile_in(tmp.path()).unwrap();
        assert!(path.exists());

        let profile = Profile::load(&path).unwrap();
        assert_eq!(profile.name(), "Default");
    }

    #[test]
    fn ensure_default_returns_existing_when_present() {
        let tmp = tempfile::tempdir().unwrap();
        let created = create_profile_in("Existing", tmp.path()).unwrap();

        let returned = ensure_default_profile_in(tmp.path()).unwrap();
        assert_eq!(created, returned);
    }

    #[test]
    fn ensure_default_returns_first_alphabetically() {
        let tmp = tempfile::tempdir().unwrap();
        create_profile_in("Zebra", tmp.path()).unwrap();
        let alpha_path = create_profile_in("Alpha", tmp.path()).unwrap();

        let returned = ensure_default_profile_in(tmp.path()).unwrap();
        assert_eq!(returned, alpha_path);
    }

    // --- rename_profile ---

    #[test]
    fn rename_profile_updates_file_and_name() {
        let tmp = tempfile::tempdir().unwrap();
        let old_path = create_profile_in("Old Name", tmp.path()).unwrap();

        let new_path = rename_profile(&old_path, "New Name").unwrap();

        assert!(!old_path.exists());
        assert!(new_path.exists());
        assert_eq!(
            new_path.file_name().unwrap().to_str().unwrap(),
            "New Name.toml"
        );

        let profile = Profile::load(&new_path).unwrap();
        assert_eq!(profile.name(), "New Name");
    }

    #[test]
    fn rename_profile_rejects_invalid_name() {
        let tmp = tempfile::tempdir().unwrap();
        let path = create_profile_in("Valid", tmp.path()).unwrap();

        let err = rename_profile(&path, "bad/name").unwrap_err();
        assert!(err.to_string().contains("illegal character"));
        // Original file should still exist.
        assert!(path.exists());
    }

    #[test]
    fn rename_profile_rejects_dot_name() {
        let tmp = tempfile::tempdir().unwrap();
        let path = create_profile_in("Valid", tmp.path()).unwrap();
        let err = rename_profile(&path, ".").unwrap_err();
        assert!(err.to_string().contains("empty after sanitization"));
        assert!(path.exists());
    }

    #[test]
    fn rename_profile_rejects_collision() {
        let tmp = tempfile::tempdir().unwrap();
        let path_a = create_profile_in("Alpha", tmp.path()).unwrap();
        create_profile_in("Bravo", tmp.path()).unwrap();

        let err = rename_profile(&path_a, "Bravo").unwrap_err();
        assert!(err.to_string().contains("already exists"));
        // Original file should still exist.
        assert!(path_a.exists());
    }

    #[test]
    fn rename_profile_preserves_id() {
        let tmp = tempfile::tempdir().unwrap();
        let old_path = create_profile_in("Original", tmp.path()).unwrap();
        let original_id = Profile::load(&old_path).unwrap().id().clone();

        let new_path = rename_profile(&old_path, "Renamed").unwrap();
        let renamed_id = Profile::load(&new_path).unwrap().id().clone();

        assert_eq!(original_id, renamed_id);
    }

    // --- delete_profile ---

    #[test]
    fn delete_profile_removes_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = create_profile_in("ToDelete", tmp.path()).unwrap();
        assert!(path.exists());

        delete_profile(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn delete_profile_nonexistent_returns_error() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nonexistent.toml");
        assert!(delete_profile(&path).is_err());
    }

    // --- Integration: create, list, rename, delete lifecycle ---

    #[test]
    fn full_lifecycle() {
        let tmp = tempfile::tempdir().unwrap();

        // Create two profiles.
        let p1 = create_profile_in("First", tmp.path()).unwrap();
        let p2 = create_profile_in("Second", tmp.path()).unwrap();

        // List should show both, sorted.
        let list = list_profiles_in(tmp.path()).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "First");
        assert_eq!(list[1].name, "Second");

        // Rename First -> Alpha.
        let p1_new = rename_profile(&p1, "Alpha").unwrap();
        let list = list_profiles_in(tmp.path()).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "Alpha");
        assert_eq!(list[1].name, "Second");

        // Delete Alpha.
        delete_profile(&p1_new).unwrap();
        let list = list_profiles_in(tmp.path()).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Second");

        // Delete Second.
        delete_profile(&p2).unwrap();
        let list = list_profiles_in(tmp.path()).unwrap();
        assert!(list.is_empty());

        // Ensure default creates a profile when none exist.
        let default_path = ensure_default_profile_in(tmp.path()).unwrap();
        assert!(default_path.exists());
        let list = list_profiles_in(tmp.path()).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Default");
    }
}
