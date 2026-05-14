// Rust guideline compliant 2026-05-14

use std::path::{Path, PathBuf};

use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

use super::model::{
    AssetManifestDocument, MappingMetadataDocument, ProfileSheetsDocument, SidecarDocument,
    TemplateStoreDocument, validate_unique_mapping_metadata_refs,
};
use crate::error::{EngineError, Result};
use crate::fs::atomic_write;

const STORE_DIR_NAME: &str = "mapping-sheet-builder";
const TEMPLATES_FILE_NAME: &str = "templates.toml";
const ASSETS_FILE_NAME: &str = "assets.toml";
const PROFILE_SHEETS_FILE_NAME: &str = "mapping-sheets.toml";
const MAPPING_METADATA_FILE_NAME: &str = "mapping-display-metadata.toml";
const EXTERNAL_PROFILES_DIR_NAME: &str = "external_profiles";

/// Returns the global mapping sheet store directory under the app config root.
#[must_use]
pub fn global_sheet_store_dir(config_dir: &Path) -> PathBuf {
    config_dir.join(STORE_DIR_NAME)
}

/// Returns the global device template store path.
#[must_use]
pub fn global_templates_path(config_dir: &Path) -> PathBuf {
    global_sheet_store_dir(config_dir).join(TEMPLATES_FILE_NAME)
}

/// Returns the global imported asset manifest path.
#[must_use]
pub fn global_asset_manifest_path(config_dir: &Path) -> PathBuf {
    global_sheet_store_dir(config_dir).join(ASSETS_FILE_NAME)
}

/// Returns the profile-owned sidecar directory next to a profile.
///
/// # Errors
///
/// Returns [`EngineError::ProfilePathHasNoParent`] when the profile path has no
/// parent directory, or [`EngineError::InvalidConfig`] when the file stem is
/// missing, empty, or not valid UTF-8.
pub fn profile_sidecar_dir(profile_path: &Path) -> Result<PathBuf> {
    let parent = profile_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or_else(|| EngineError::ProfilePathHasNoParent {
            path: profile_path.to_path_buf(),
        })?;

    let stem = profile_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .ok_or_else(|| EngineError::InvalidConfig {
            reason: format!(
                "profile path has no valid UTF-8 file stem: {}",
                profile_path.display()
            ),
        })?;

    Ok(parent.join(format!("{stem}.inputforge")))
}

/// Returns the profile-owned mapping sheets sidecar path.
///
/// # Errors
///
/// Returns an error when [`profile_sidecar_dir`] rejects the profile path.
pub fn profile_sheets_path(profile_path: &Path) -> Result<PathBuf> {
    Ok(profile_sidecar_dir(profile_path)?.join(PROFILE_SHEETS_FILE_NAME))
}

/// Returns the profile-owned mapping display metadata sidecar path.
///
/// # Errors
///
/// Returns an error when [`profile_sidecar_dir`] rejects the profile path.
pub fn profile_mapping_metadata_path(profile_path: &Path) -> Result<PathBuf> {
    Ok(profile_sidecar_dir(profile_path)?.join(MAPPING_METADATA_FILE_NAME))
}

/// Returns the config-owned sidecar directory for a canonical external profile.
///
/// Callers must pass a canonical profile path; the hash namespace is derived
/// directly from the provided path string.
#[must_use]
pub fn external_profile_sidecar_dir(config_dir: &Path, canonical_profile_path: &Path) -> PathBuf {
    let hash = Sha256::digest(canonical_profile_path.to_string_lossy().as_bytes());

    global_sheet_store_dir(config_dir)
        .join(EXTERNAL_PROFILES_DIR_NAME)
        .join(hex::encode(hash))
}

/// Returns the config-owned mapping sheets sidecar path for an external profile.
#[must_use]
pub fn external_profile_sheets_path(config_dir: &Path, canonical_profile_path: &Path) -> PathBuf {
    external_profile_sidecar_dir(config_dir, canonical_profile_path).join(PROFILE_SHEETS_FILE_NAME)
}

/// Returns the config-owned mapping metadata sidecar path for an external profile.
#[must_use]
pub fn external_profile_mapping_metadata_path(
    config_dir: &Path,
    canonical_profile_path: &Path,
) -> PathBuf {
    external_profile_sidecar_dir(config_dir, canonical_profile_path)
        .join(MAPPING_METADATA_FILE_NAME)
}

/// Saves the global device template store document.
///
/// # Errors
///
/// Returns serialization or I/O errors from writing the TOML document.
pub fn save_template_store(path: &Path, document: &mut TemplateStoreDocument) -> Result<()> {
    save_toml(path, document)
}

/// Loads the global device template store document, defaulting when missing.
///
/// # Errors
///
/// Returns parse or I/O errors for existing files.
pub fn load_template_store(path: &Path) -> Result<TemplateStoreDocument> {
    load_toml_or_default(path)
}

/// Saves the global imported asset manifest document.
///
/// # Errors
///
/// Returns serialization or I/O errors from writing the TOML document.
pub fn save_asset_manifest(path: &Path, document: &mut AssetManifestDocument) -> Result<()> {
    save_toml(path, document)
}

/// Loads the global imported asset manifest document, defaulting when missing.
///
/// # Errors
///
/// Returns parse or I/O errors for existing files.
pub fn load_asset_manifest(path: &Path) -> Result<AssetManifestDocument> {
    load_toml_or_default(path)
}

/// Saves a profile mapping sheets document.
///
/// # Errors
///
/// Returns serialization or I/O errors from writing the TOML document.
pub fn save_profile_sheets(path: &Path, document: &mut ProfileSheetsDocument) -> Result<()> {
    save_toml(path, document)
}

/// Loads a profile mapping sheets document, returning `None` when missing.
///
/// # Errors
///
/// Returns parse or I/O errors for existing files.
pub fn load_profile_sheets(path: &Path) -> Result<Option<ProfileSheetsDocument>> {
    load_optional_toml(path)
}

/// Saves profile mapping display metadata after validating mapping refs.
///
/// # Errors
///
/// Returns validation, serialization, or I/O errors.
pub fn save_mapping_metadata(path: &Path, document: &mut MappingMetadataDocument) -> Result<()> {
    validate_unique_mapping_metadata_refs(document)?;
    save_toml(path, document)
}

/// Loads profile mapping display metadata, validating mapping refs when present.
///
/// # Errors
///
/// Returns parse, validation, or I/O errors for existing files.
pub fn load_mapping_metadata(path: &Path) -> Result<Option<MappingMetadataDocument>> {
    let document = load_optional_toml(path)?;
    if let Some(document) = &document {
        validate_unique_mapping_metadata_refs(document)?;
    }

    Ok(document)
}

fn save_toml<T>(path: &Path, document: &mut T) -> Result<()>
where
    T: Serialize + SidecarDocument,
{
    let previous_last_saved = document.header_mut().app_version_last_saved.clone();
    document.header_mut().mark_saved_by_current_app();

    let result = toml::to_string_pretty(document)
        .map_err(EngineError::from)
        .and_then(|toml| atomic_write(path, toml.as_bytes()));

    if result.is_err() {
        document.header_mut().app_version_last_saved = previous_last_saved;
    }

    result
}

fn load_toml_or_default<T>(path: &Path) -> Result<T>
where
    T: DeserializeOwned + Default,
{
    match std::fs::read_to_string(path) {
        Ok(toml) => Ok(toml::from_str(&toml)?),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(err) => Err(err.into()),
    }
}

fn load_optional_toml<T>(path: &Path) -> Result<Option<T>>
where
    T: DeserializeOwned,
{
    match std::fs::read_to_string(path) {
        Ok(toml) => Ok(Some(toml::from_str(&toml)?)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err.into()),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    use super::{
        external_profile_mapping_metadata_path, external_profile_sheets_path,
        external_profile_sidecar_dir, global_asset_manifest_path, global_sheet_store_dir,
        global_templates_path, load_asset_manifest, load_mapping_metadata, load_profile_sheets,
        load_template_store, profile_mapping_metadata_path, profile_sheets_path,
        profile_sidecar_dir, save_asset_manifest, save_mapping_metadata, save_profile_sheets,
        save_template_store,
    };
    use crate::error::EngineError;
    use crate::profile::ProfileId;
    use crate::sheet::{
        AssetManifestDocument, DeviceTemplate, MappingDisplayMetadata, MappingMetadataDocument,
        MappingMetadataId, MappingRef, ProfileSheetsDocument, SidecarHeader, TemplateId,
        TemplateStoreDocument, TokenPreset,
    };
    use crate::types::{DeviceId, InputAddress, InputId};
    use sha2::{Digest, Sha256};

    fn profile_sheets_document() -> ProfileSheetsDocument {
        ProfileSheetsDocument {
            header: SidecarHeader::default(),
            profile_id: ProfileId::default(),
            sheets: Vec::new(),
            extensions: BTreeMap::default(),
        }
    }

    fn mapping_metadata_document() -> MappingMetadataDocument {
        MappingMetadataDocument {
            header: SidecarHeader::default(),
            profile_id: ProfileId::default(),
            records: Vec::new(),
            extensions: BTreeMap::default(),
        }
    }

    fn mapping_ref() -> MappingRef {
        MappingRef {
            mode_id: "combat".to_owned(),
            input: InputAddress::Bound {
                device: DeviceId("stick-alpha".to_owned()),
                input: InputId::Button { index: 1 },
            },
            fallback_label: "Fire".to_owned(),
            fallback_details: None,
        }
    }

    fn duplicate_mapping_metadata_document() -> MappingMetadataDocument {
        let mapping_ref = mapping_ref();

        MappingMetadataDocument {
            records: vec![
                MappingDisplayMetadata {
                    id: MappingMetadataId::from_string("metadata-a"),
                    mapping_ref: mapping_ref.clone(),
                    display_name: Some("Fire primary".to_owned()),
                    category: None,
                    classification_token: None,
                    extensions: BTreeMap::default(),
                },
                MappingDisplayMetadata {
                    id: MappingMetadataId::from_string("metadata-b"),
                    mapping_ref,
                    display_name: Some("Fire duplicate".to_owned()),
                    category: None,
                    classification_token: None,
                    extensions: BTreeMap::default(),
                },
            ],
            ..mapping_metadata_document()
        }
    }

    fn template_store_document() -> TemplateStoreDocument {
        TemplateStoreDocument {
            templates: vec![DeviceTemplate {
                template_id: TemplateId::from_string("template-stick-left"),
                display_name: "Left Stick".to_owned(),
                matching_hints: Vec::new(),
                asset_ids: Vec::new(),
                anchors: Vec::new(),
                grouping_hints: Vec::new(),
                default_token_preset: TokenPreset::Standard,
                extensions: BTreeMap::default(),
            }],
            ..TemplateStoreDocument::default()
        }
    }

    #[test]
    fn global_paths_live_under_mapping_sheet_builder_dir() {
        let config_dir = Path::new("config");
        let store_dir = config_dir.join("mapping-sheet-builder");

        assert_eq!(global_sheet_store_dir(config_dir), store_dir);
        assert_eq!(
            global_templates_path(config_dir),
            store_dir.join("templates.toml")
        );
        assert_eq!(
            global_asset_manifest_path(config_dir),
            store_dir.join("assets.toml")
        );
    }

    #[test]
    fn profile_sidecars_live_next_to_profile_in_named_dir() {
        let profile_path = Path::new("profiles").join("flight.toml");
        let sidecar_dir = Path::new("profiles").join("flight.inputforge");

        assert_eq!(profile_sidecar_dir(&profile_path).unwrap(), sidecar_dir);
        assert_eq!(
            profile_sheets_path(&profile_path).unwrap(),
            sidecar_dir.join("mapping-sheets.toml")
        );
        assert_eq!(
            profile_mapping_metadata_path(&profile_path).unwrap(),
            sidecar_dir.join("mapping-display-metadata.toml")
        );
    }

    #[test]
    fn profile_path_without_parent_is_rejected() {
        let err = profile_sidecar_dir(Path::new("profile.toml")).unwrap_err();

        assert!(matches!(err, EngineError::ProfilePathHasNoParent { .. }));
    }

    #[test]
    fn save_and_load_template_store_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("templates.toml");
        let mut document = template_store_document();

        save_template_store(&path, &mut document).unwrap();
        let loaded = load_template_store(&path).unwrap();

        assert_eq!(loaded, document);
    }

    #[test]
    fn missing_sidecar_loads_as_default_document() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(
            load_template_store(&dir.path().join("missing.toml")).unwrap(),
            TemplateStoreDocument::default()
        );
    }

    #[test]
    fn external_profile_sidecars_live_under_config_hash_namespace() {
        let config_dir = Path::new("app-config");
        let external_profile = PathBuf::from(r"C:\external\profile.toml");
        let expected_dir = global_sheet_store_dir(config_dir)
            .join("external_profiles")
            .join(hex::encode(Sha256::digest(
                external_profile.to_string_lossy().as_bytes(),
            )));

        assert_eq!(
            external_profile_sidecar_dir(config_dir, &external_profile),
            expected_dir
        );
        assert_eq!(
            external_profile_sheets_path(config_dir, &external_profile),
            expected_dir.join("mapping-sheets.toml")
        );
        assert_eq!(
            external_profile_mapping_metadata_path(config_dir, &external_profile),
            expected_dir.join("mapping-display-metadata.toml")
        );
        assert!(
            !external_profile_sheets_path(config_dir, &external_profile)
                .starts_with(external_profile.parent().unwrap())
        );
    }

    #[test]
    fn external_profile_sidecar_uses_canonical_path_namespace() {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().join("config");
        let profile_path = dir.path().join("external").join("profile.toml");
        std::fs::create_dir_all(profile_path.parent().unwrap()).unwrap();
        std::fs::write(&profile_path, "").unwrap();
        let canonical_profile = std::fs::canonicalize(&profile_path).unwrap();

        let sidecar_dir = external_profile_sidecar_dir(&config_dir, &canonical_profile);

        assert!(
            sidecar_dir.starts_with(global_sheet_store_dir(&config_dir).join("external_profiles"))
        );
        assert_eq!(
            sidecar_dir,
            global_sheet_store_dir(&config_dir)
                .join("external_profiles")
                .join(hex::encode(Sha256::digest(
                    canonical_profile.to_string_lossy().as_bytes(),
                )))
        );
    }

    #[test]
    fn failed_save_restores_previous_last_saved_version() {
        let mut document = template_store_document();
        document.header.app_version_last_saved = "0.0.0".to_owned();

        let err = save_template_store(Path::new("templates.toml"), &mut document).unwrap_err();

        assert!(matches!(err, EngineError::ProfilePathHasNoParent { .. }));
        assert_eq!(document.header.app_version_last_saved, "0.0.0");
    }

    #[test]
    fn save_mapping_metadata_rejects_duplicate_mapping_refs() {
        let dir = tempfile::tempdir().unwrap();
        let mut document = duplicate_mapping_metadata_document();

        let err = save_mapping_metadata(
            &dir.path().join("mapping-display-metadata.toml"),
            &mut document,
        )
        .unwrap_err();

        assert!(matches!(err, EngineError::InvalidConfig { .. }));
    }

    #[test]
    fn load_mapping_metadata_rejects_duplicate_mapping_refs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mapping-display-metadata.toml");
        let document = duplicate_mapping_metadata_document();
        std::fs::write(&path, toml::to_string_pretty(&document).unwrap()).unwrap();

        let err = load_mapping_metadata(&path).unwrap_err();

        assert!(matches!(err, EngineError::InvalidConfig { .. }));
    }

    #[test]
    fn all_sidecar_save_load_helpers_roundtrip_and_update_last_saved() {
        let dir = tempfile::tempdir().unwrap();

        let mut templates = template_store_document();
        templates.header.app_version_last_saved = "older".to_owned();
        let templates_path = dir.path().join("templates.toml");
        save_template_store(&templates_path, &mut templates).unwrap();
        assert_eq!(
            templates.header.app_version_last_saved,
            env!("CARGO_PKG_VERSION")
        );
        assert_eq!(load_template_store(&templates_path).unwrap(), templates);

        let mut assets = AssetManifestDocument::default();
        assets.header.app_version_last_saved = "older".to_owned();
        let assets_path = dir.path().join("assets.toml");
        save_asset_manifest(&assets_path, &mut assets).unwrap();
        assert_eq!(
            assets.header.app_version_last_saved,
            env!("CARGO_PKG_VERSION")
        );
        assert_eq!(load_asset_manifest(&assets_path).unwrap(), assets);

        let mut sheets = profile_sheets_document();
        sheets.header.app_version_last_saved = "older".to_owned();
        let sheets_path = dir.path().join("mapping-sheets.toml");
        save_profile_sheets(&sheets_path, &mut sheets).unwrap();
        assert_eq!(
            sheets.header.app_version_last_saved,
            env!("CARGO_PKG_VERSION")
        );
        assert_eq!(load_profile_sheets(&sheets_path).unwrap(), Some(sheets));

        let mut metadata = mapping_metadata_document();
        metadata.header.app_version_last_saved = "older".to_owned();
        let metadata_path = dir.path().join("mapping-display-metadata.toml");
        save_mapping_metadata(&metadata_path, &mut metadata).unwrap();
        assert_eq!(
            metadata.header.app_version_last_saved,
            env!("CARGO_PKG_VERSION")
        );
        assert_eq!(
            load_mapping_metadata(&metadata_path).unwrap(),
            Some(metadata)
        );
    }

    #[test]
    fn missing_global_documents_load_defaults_and_missing_profile_documents_load_none() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(
            load_template_store(&dir.path().join("missing-templates.toml")).unwrap(),
            TemplateStoreDocument::default()
        );
        assert_eq!(
            load_asset_manifest(&dir.path().join("missing-assets.toml")).unwrap(),
            AssetManifestDocument::default()
        );
        assert_eq!(
            load_profile_sheets(&dir.path().join("missing-sheets.toml")).unwrap(),
            None
        );
        assert_eq!(
            load_mapping_metadata(&dir.path().join("missing-metadata.toml")).unwrap(),
            None
        );
    }
}
