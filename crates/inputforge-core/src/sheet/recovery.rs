// Rust guideline compliant 2026-05-13

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::ids::RecoverySnapshotId;
use super::store::{external_profile_sidecar_dir, global_sheet_store_dir, profile_sidecar_dir};
use crate::error::Result;
use crate::fs::atomic_write;

const RECOVERY_DIR_NAME: &str = "recovery";
const MANIFEST_FILE_NAME: &str = "manifest.toml";
const FALLBACK_SIDECAR_FILE_NAME: &str = "sidecar.toml";

/// Describes one recovery snapshot and the sidecars it captured.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoverySnapshotManifest {
    pub id: RecoverySnapshotId,
    pub label: String,
    pub taken_at: DateTime<Utc>,
    pub files: Vec<RecoverySnapshotFile>,
}

/// Describes one sidecar source and its optional snapshot copy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoverySnapshotFile {
    pub source_path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copied_path: Option<PathBuf>,
}

/// Returns the global recovery root under the mapping sheet store.
#[must_use]
pub fn global_recovery_dir(config_dir: &Path) -> PathBuf {
    global_sheet_store_dir(config_dir).join(RECOVERY_DIR_NAME)
}

/// Returns the profile-owned recovery root next to the profile sidecars.
///
/// # Errors
///
/// Returns an error when [`profile_sidecar_dir`] rejects the profile path.
pub fn profile_recovery_dir(profile_path: &Path) -> Result<PathBuf> {
    Ok(profile_sidecar_dir(profile_path)?.join(RECOVERY_DIR_NAME))
}

/// Returns the config-owned recovery root for an external profile.
#[must_use]
pub fn external_profile_recovery_dir(config_dir: &Path, canonical_profile_path: &Path) -> PathBuf {
    external_profile_sidecar_dir(config_dir, canonical_profile_path).join(RECOVERY_DIR_NAME)
}

/// Creates a recovery snapshot for the provided sidecar files.
///
/// Existing sidecar files are copied under a new snapshot directory. Missing
/// sidecars are recorded in the manifest without failing the snapshot.
///
/// # Errors
///
/// Returns serialization or I/O errors from creating directories, copying
/// existing sidecars, or writing the TOML manifest.
pub fn create_recovery_snapshot(
    recovery_root: &Path,
    label: impl Into<String>,
    sidecar_files: &[PathBuf],
) -> Result<RecoverySnapshotManifest> {
    create_recovery_snapshot_with_copy(
        recovery_root,
        label,
        sidecar_files,
        |source, destination| std::fs::copy(source, destination),
    )
}

fn create_recovery_snapshot_with_copy(
    recovery_root: &Path,
    label: impl Into<String>,
    sidecar_files: &[PathBuf],
    mut copy_file: impl FnMut(&Path, &Path) -> std::io::Result<u64>,
) -> Result<RecoverySnapshotManifest> {
    let id = RecoverySnapshotId::new();
    let snapshot_dir = recovery_root.join(id.to_string());
    std::fs::create_dir_all(&snapshot_dir)?;

    let result = populate_recovery_snapshot(
        &snapshot_dir,
        id,
        label.into(),
        sidecar_files,
        &mut copy_file,
    );
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&snapshot_dir);
    }
    result
}

fn populate_recovery_snapshot(
    snapshot_dir: &Path,
    id: RecoverySnapshotId,
    label: String,
    sidecar_files: &[PathBuf],
    copy_file: &mut dyn FnMut(&Path, &Path) -> std::io::Result<u64>,
) -> Result<RecoverySnapshotManifest> {
    let mut files = Vec::with_capacity(sidecar_files.len());
    for (index, source_path) in sidecar_files.iter().enumerate() {
        let copied_path = if source_path.try_exists()? {
            let file_name = source_path
                .file_name()
                .and_then(|file_name| file_name.to_str())
                .unwrap_or(FALLBACK_SIDECAR_FILE_NAME);
            let copied_path = snapshot_dir.join(format!("{index:02}-{file_name}"));

            match copy_file(source_path, &copied_path) {
                Ok(_) => Some(copied_path),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
                Err(err) => return Err(err.into()),
            }
        } else {
            None
        };

        files.push(RecoverySnapshotFile {
            source_path: source_path.clone(),
            copied_path,
        });
    }

    let manifest = RecoverySnapshotManifest {
        id,
        label,
        taken_at: Utc::now(),
        files,
    };
    let manifest_toml = toml::to_string_pretty(&manifest)?;
    atomic_write(
        &snapshot_dir.join(MANIFEST_FILE_NAME),
        manifest_toml.as_bytes(),
    )?;

    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        create_recovery_snapshot, create_recovery_snapshot_with_copy,
        external_profile_recovery_dir, global_recovery_dir, profile_recovery_dir,
    };
    use crate::sheet::{external_profile_sidecar_dir, global_sheet_store_dir, profile_sidecar_dir};

    #[test]
    fn recovery_roots_live_beside_their_owner_sidecar_namespace() {
        let config_dir = Path::new("config");
        let profile_path = Path::new("profiles").join("flight.toml");
        let external_profile_path = Path::new("external").join("profile.toml");

        assert_eq!(
            global_recovery_dir(config_dir),
            global_sheet_store_dir(config_dir).join("recovery")
        );
        assert_eq!(
            profile_recovery_dir(&profile_path).unwrap(),
            profile_sidecar_dir(&profile_path).unwrap().join("recovery")
        );
        assert_eq!(
            external_profile_recovery_dir(config_dir, &external_profile_path),
            external_profile_sidecar_dir(config_dir, &external_profile_path).join("recovery")
        );
    }

    #[test]
    fn recovery_snapshot_copies_existing_sidecars_and_writes_manifest_under_root() {
        let dir = tempfile::tempdir().unwrap();
        let recovery_root = dir.path().join("recovery");
        let first_sidecar = dir.path().join("mapping-sheets.toml");
        let second_sidecar = dir.path().join("mapping-display-metadata.toml");
        std::fs::write(&first_sidecar, "sheets = []\n").unwrap();
        std::fs::write(&second_sidecar, "records = []\n").unwrap();

        let manifest = create_recovery_snapshot(
            &recovery_root,
            "before import",
            &[first_sidecar.clone(), second_sidecar.clone()],
        )
        .unwrap();

        let snapshot_dir = recovery_root.join(manifest.id.to_string());
        let manifest_path = snapshot_dir.join("manifest.toml");
        assert_eq!(manifest.label, "before import");
        assert_eq!(manifest.files.len(), 2);
        assert_eq!(manifest.files[0].source_path, first_sidecar);
        assert_eq!(manifest.files[1].source_path, second_sidecar);

        let first_copy = snapshot_dir.join("00-mapping-sheets.toml");
        let second_copy = snapshot_dir.join("01-mapping-display-metadata.toml");
        assert_eq!(manifest.files[0].copied_path, Some(first_copy.clone()));
        assert_eq!(manifest.files[1].copied_path, Some(second_copy.clone()));
        assert_eq!(
            std::fs::read_to_string(first_copy).unwrap(),
            "sheets = []\n"
        );
        assert_eq!(
            std::fs::read_to_string(second_copy).unwrap(),
            "records = []\n"
        );

        let persisted: super::RecoverySnapshotManifest =
            toml::from_str(&std::fs::read_to_string(manifest_path).unwrap()).unwrap();
        assert_eq!(persisted, manifest);
    }

    #[test]
    fn recovery_snapshot_records_missing_sidecars_without_failing() {
        let dir = tempfile::tempdir().unwrap();
        let recovery_root = dir.path().join("recovery");
        let missing_sidecar = dir.path().join("missing.toml");

        let manifest = create_recovery_snapshot(
            &recovery_root,
            "before save",
            std::slice::from_ref(&missing_sidecar),
        )
        .unwrap();

        assert_eq!(manifest.files.len(), 1);
        assert_eq!(manifest.files[0].source_path, missing_sidecar);
        assert_eq!(manifest.files[0].copied_path, None);
        assert!(
            recovery_root
                .join(manifest.id.to_string())
                .join("manifest.toml")
                .is_file()
        );
    }

    #[test]
    fn recovery_snapshot_cleans_up_snapshot_dir_on_copy_failure() {
        let dir = tempfile::tempdir().unwrap();
        let recovery_root = dir.path().join("recovery");
        let sidecar = dir.path().join("mapping-sheets.toml");
        std::fs::write(&sidecar, "sheets = []\n").unwrap();

        let err = create_recovery_snapshot_with_copy(
            &recovery_root,
            "before save",
            std::slice::from_ref(&sidecar),
            |_, _| {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "denied",
                ))
            },
        )
        .unwrap_err();

        assert!(matches!(err, crate::error::EngineError::Io(_)));
        let leftover_snapshots: Vec<_> = std::fs::read_dir(&recovery_root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert!(
            leftover_snapshots.is_empty(),
            "expected no orphan snapshot dirs, found {leftover_snapshots:?}"
        );
    }

    #[test]
    fn recovery_snapshot_records_sidecar_missing_during_copy_without_failing() {
        let dir = tempfile::tempdir().unwrap();
        let recovery_root = dir.path().join("recovery");
        let sidecar = dir.path().join("mapping-sheets.toml");
        std::fs::write(&sidecar, "sheets = []\n").unwrap();

        let manifest = create_recovery_snapshot_with_copy(
            &recovery_root,
            "before save",
            std::slice::from_ref(&sidecar),
            |_, _| {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "source disappeared",
                ))
            },
        )
        .unwrap();

        assert_eq!(manifest.files.len(), 1);
        assert_eq!(manifest.files[0].source_path, sidecar);
        assert_eq!(manifest.files[0].copied_path, None);
        assert!(
            recovery_root
                .join(manifest.id.to_string())
                .join("manifest.toml")
                .is_file()
        );
    }
}
