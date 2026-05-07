//! Contains profile library lifecycle operations.
//!
//! These helpers keep persisted profile files and adjacent snapshot
//! directories consistent while reusing the existing profile manager APIs.

use std::path::{Path, PathBuf};

use crate::error::{EngineError, Result};
use crate::profile::Profile;
use crate::profile::manager::{rename_profile, sanitize_filename, validate_profile_name};
use crate::snapshot::fs::snapshots_dir_for;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryProfile {
    /// Profile display name persisted inside the profile file.
    pub name: String,
    /// Absolute path to the profile file.
    pub path: PathBuf,
}

/// Rename a library profile and move its adjacent snapshot directory.
///
/// # Errors
///
/// Returns [`EngineError::InvalidConfig`] when the new name is invalid,
/// the profile path has no parent, or the destination profile/snapshot path
/// already exists. Returns I/O or profile serialization errors from the
/// underlying rename and save operations.
pub fn rename_library_profile(path: &Path, new_name: &str) -> Result<LibraryProfile> {
    validate_profile_name(new_name)?;
    let old_snapshot_dir = snapshots_dir_for(path)?;
    let old_snapshot_exists = old_snapshot_dir.exists();
    let new_path = destination_path_for_name(path, new_name)?;
    let new_snapshot_dir = snapshots_dir_for(&new_path)?;
    if old_snapshot_exists && new_snapshot_dir.exists() && new_snapshot_dir != old_snapshot_dir {
        return Err(EngineError::InvalidConfig {
            reason: format!("snapshot directory already exists for profile '{new_name}'"),
        });
    }

    let renamed_path = rename_profile(path, new_name)?;
    if old_snapshot_exists {
        std::fs::rename(&old_snapshot_dir, &new_snapshot_dir)?;
    }

    Ok(LibraryProfile {
        name: new_name.to_owned(),
        path: renamed_path,
    })
}

/// Duplicate a profile file into the library with a new internal name.
///
/// Snapshot directories are intentionally not copied.
///
/// # Errors
///
/// Returns [`EngineError::InvalidConfig`] when `new_name` is invalid or the
/// destination already exists. Returns I/O, profile load, or profile save
/// errors from the underlying filesystem and serialization operations.
pub fn duplicate_library_profile(
    source_path: &Path,
    new_name: &str,
    library_dir: &Path,
) -> Result<LibraryProfile> {
    save_profile_copy_with_name(source_path, new_name, library_dir)
}

/// Import an external profile file into the library with a new name.
///
/// Snapshot directories beside the external profile are intentionally not
/// copied into the library.
///
/// # Errors
///
/// Returns [`EngineError::InvalidConfig`] when `name` is invalid or the
/// destination already exists. Returns I/O, profile load, or profile save
/// errors from the underlying filesystem and serialization operations.
pub fn add_external_profile_to_library(
    external_path: &Path,
    name: &str,
    library_dir: &Path,
) -> Result<LibraryProfile> {
    save_profile_copy_with_name(external_path, name, library_dir)
}

fn save_profile_copy_with_name(
    source_path: &Path,
    new_name: &str,
    library_dir: &Path,
) -> Result<LibraryProfile> {
    let destination = destination_path_in_dir(library_dir, new_name)?;
    std::fs::create_dir_all(library_dir)?;
    if destination.exists() {
        return Err(EngineError::InvalidConfig {
            reason: format!("a profile named '{new_name}' already exists"),
        });
    }

    let mut profile = Profile::load(source_path)?;
    profile.set_name(new_name.to_owned());
    profile.save(&destination)?;

    Ok(LibraryProfile {
        name: new_name.to_owned(),
        path: destination,
    })
}

/// Reveal `path` in the platform OS file manager.
///
/// Best-effort UX, the caller decides how to surface a failure. On Windows
/// this selects the file in Explorer; on macOS Finder selects the file;
/// on Linux the parent directory is opened by the desktop's default file
/// manager.
///
/// # Errors
///
/// Returns [`EngineError::Io`] when the underlying [`opener`] call fails
/// (e.g., no file manager configured, invalid path, OS error).
pub fn reveal_profile_in_explorer(path: &Path) -> Result<()> {
    opener::reveal(path).map_err(|e| EngineError::Io(std::io::Error::other(e)))
}

fn destination_path_for_name(path: &Path, name: &str) -> Result<PathBuf> {
    let parent = path.parent().ok_or_else(|| EngineError::InvalidConfig {
        reason: "profile path has no parent directory".to_owned(),
    })?;
    destination_path_in_dir(parent, name)
}

fn destination_path_in_dir(dir: &Path, name: &str) -> Result<PathBuf> {
    validate_profile_name(name)?;
    let sanitized = sanitize_filename(name);
    if sanitized.is_empty() {
        return Err(EngineError::InvalidConfig {
            reason: "profile name is empty after sanitization".to_owned(),
        });
    }
    Ok(dir.join(format!("{sanitized}.toml")))
}

#[cfg(test)]
mod tests {
    use super::{
        add_external_profile_to_library, duplicate_library_profile, rename_library_profile,
    };
    use crate::profile::Profile;
    use crate::profile::manager::{create_profile_in, list_profiles_in};
    use crate::snapshot::fs::snapshots_dir_for;

    #[test]
    fn rename_profile_updates_internal_name_and_moves_sibling_snapshot_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_dir = tmp.path().join("profiles");
        let original_path = create_profile_in("Alpha", &profiles_dir).unwrap();
        let old_snapshots = snapshots_dir_for(&original_path).unwrap();
        std::fs::create_dir_all(&old_snapshots).unwrap();

        let renamed = rename_library_profile(&original_path, "Bravo").unwrap();

        let new_snapshots = snapshots_dir_for(&renamed.path).unwrap();
        assert_eq!(renamed.name, "Bravo");
        assert!(!original_path.exists());
        assert!(renamed.path.exists());
        assert_eq!(Profile::load(&renamed.path).unwrap().name(), "Bravo");
        assert!(!old_snapshots.exists());
        assert!(new_snapshots.exists());
    }

    #[test]
    fn duplicate_profile_rewrites_internal_name_without_copying_snapshots() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_dir = tmp.path().join("profiles");
        let original_path = create_profile_in("Alpha", &profiles_dir).unwrap();
        std::fs::create_dir_all(snapshots_dir_for(&original_path).unwrap()).unwrap();

        let duplicated =
            duplicate_library_profile(&original_path, "Alpha Copy", &profiles_dir).unwrap();

        assert_eq!(duplicated.name, "Alpha Copy");
        assert_eq!(
            Profile::load(&duplicated.path).unwrap().name(),
            "Alpha Copy"
        );
        assert!(!snapshots_dir_for(&duplicated.path).unwrap().exists());
    }

    #[test]
    fn add_external_to_library_rewrites_internal_name_without_copying_external_snapshots() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_dir = tmp.path().join("profiles");
        let source_path = create_profile_in("Source", &profiles_dir).unwrap();
        let external_path = tmp.path().join("external.toml");
        std::fs::copy(&source_path, &external_path).unwrap();
        std::fs::create_dir_all(snapshots_dir_for(&external_path).unwrap()).unwrap();

        let imported =
            add_external_profile_to_library(&external_path, "Imported", &profiles_dir).unwrap();

        assert_eq!(imported.name, "Imported");
        assert_eq!(Profile::load(&imported.path).unwrap().name(), "Imported");
        assert!(!snapshots_dir_for(&imported.path).unwrap().exists());
    }

    #[test]
    fn duplicate_name_uses_invalid_config_error() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_dir = tmp.path().join("profiles");
        let original_path = create_profile_in("Alpha", &profiles_dir).unwrap();
        create_profile_in("Alpha Copy", &profiles_dir).unwrap();

        let err =
            duplicate_library_profile(&original_path, "Alpha Copy", &profiles_dir).unwrap_err();
        assert!(err.to_string().contains("invalid config"));
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn list_library_rows_sorts_alphabetically() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_dir = tmp.path().join("profiles");
        create_profile_in("Zulu", &profiles_dir).unwrap();
        create_profile_in("Alpha", &profiles_dir).unwrap();

        let profiles = list_profiles_in(&profiles_dir).unwrap();
        assert_eq!(profiles[0].name, "Alpha");
        assert_eq!(profiles[1].name, "Zulu");
    }
}
