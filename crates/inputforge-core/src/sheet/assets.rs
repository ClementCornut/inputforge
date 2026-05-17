// Rust guideline compliant 2026-05-14

use std::path::{Path, PathBuf};

use image::GenericImageView;
use sha2::{Digest, Sha256};

use super::ids::AssetId;
use super::model::{AssetEntry, AssetManifestDocument, ExtensionPayload, PixelDimensions};
use super::recovery::{create_recovery_snapshot, global_recovery_dir};
use super::store::{global_asset_manifest_path, global_sheet_store_dir};
use crate::error::{EngineError, Result};
use crate::fs::atomic_write;

const ASSET_DIR_NAME: &str = "assets";

/// Describes an image asset imported into app-owned storage.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedAsset {
    pub entry: AssetEntry,
    pub copied_path: PathBuf,
}

/// Describes whether a manifest asset has its copied file on disk.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetHealth {
    pub entry: AssetEntry,
    pub copied_absolute_path: PathBuf,
    pub missing: bool,
}

struct ImageMetadata {
    media_type: &'static str,
    pixel_dimensions: PixelDimensions,
}

/// Returns the app-owned imported asset directory.
#[must_use]
pub fn global_asset_dir(config_dir: &Path) -> PathBuf {
    global_sheet_store_dir(config_dir).join(ASSET_DIR_NAME)
}

/// Imports an image file into app-owned asset storage.
///
/// The source image is decoded before recovery snapshots, directory creation,
/// or manifest mutation so invalid images leave existing state untouched.
/// This mutates the in-memory manifest but does not save it; callers must save
/// the manifest separately.
///
/// # Errors
///
/// Returns I/O errors from reading or copying the image and recovery errors
/// from snapshot creation. Returns [`EngineError::InvalidConfig`] with an
/// `invalid image` reason when the source bytes cannot be decoded as an image.
pub fn import_image_asset(
    config_dir: &Path,
    source_path: &Path,
    manifest: &mut AssetManifestDocument,
) -> Result<ImportedAsset> {
    import_image_asset_with_persist(config_dir, source_path, manifest, persist_asset_bytes)
}

fn import_image_asset_with_persist(
    config_dir: &Path,
    source_path: &Path,
    manifest: &mut AssetManifestDocument,
    persist_bytes: impl FnOnce(&Path, &[u8]) -> Result<()>,
) -> Result<ImportedAsset> {
    let source_bytes = std::fs::read(source_path)?;
    let metadata = read_image_metadata(&source_bytes)?;
    let content_hash = hex::encode(Sha256::digest(&source_bytes));

    create_recovery_snapshot(
        &global_recovery_dir(config_dir),
        "before image import",
        &[global_asset_manifest_path(config_dir)],
    )?;

    let asset_id = AssetId::new();
    let extension = source_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .filter(|extension| !extension.is_empty())
        .unwrap_or_else(|| "img".to_owned());
    let copied_relative_path =
        PathBuf::from(ASSET_DIR_NAME).join(format!("{asset_id}.{extension}"));
    let copied_path = global_sheet_store_dir(config_dir).join(&copied_relative_path);
    persist_bytes(&copied_path, &source_bytes)?;

    let entry = AssetEntry {
        asset_id,
        copied_path: copied_relative_path,
        content_hash,
        media_type: metadata.media_type.to_owned(),
        pixel_dimensions: metadata.pixel_dimensions,
        original_import_path: Some(source_path.to_path_buf()),
        display_name: None,
        extensions: ExtensionPayload::default(),
    };
    manifest.assets.push(entry.clone());

    Ok(ImportedAsset { entry, copied_path })
}

fn persist_asset_bytes(destination: &Path, bytes: &[u8]) -> Result<()> {
    atomic_write(destination, bytes)
}

/// Reports copied-file health for all imported assets in a manifest.
#[must_use]
pub fn asset_health(config_dir: &Path, manifest: &AssetManifestDocument) -> Vec<AssetHealth> {
    let store_dir = global_sheet_store_dir(config_dir);
    manifest
        .assets
        .iter()
        .map(|entry| {
            let copied_absolute_path = store_dir.join(&entry.copied_path);
            let missing = !copied_absolute_path.is_file();

            AssetHealth {
                entry: entry.clone(),
                copied_absolute_path,
                missing,
            }
        })
        .collect()
}

fn read_image_metadata(source_bytes: &[u8]) -> Result<ImageMetadata> {
    let reader = image::ImageReader::new(std::io::Cursor::new(source_bytes))
        .with_guessed_format()
        .map_err(|err| EngineError::InvalidConfig {
            reason: format!("invalid image: {err}"),
        })?;
    let format = reader.format();
    let image = reader.decode().map_err(|err| EngineError::InvalidConfig {
        reason: format!("invalid image: {err}"),
    })?;
    let (width, height) = image.dimensions();

    Ok(ImageMetadata {
        media_type: media_type_for(format),
        pixel_dimensions: PixelDimensions { width, height },
    })
}

fn media_type_for(format: Option<image::ImageFormat>) -> &'static str {
    match format {
        Some(image::ImageFormat::Png) => "image/png",
        Some(image::ImageFormat::Jpeg) => "image/jpeg",
        Some(image::ImageFormat::Gif) => "image/gif",
        Some(image::ImageFormat::Bmp) => "image/bmp",
        Some(image::ImageFormat::WebP) => "image/webp",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE_BY_ONE_PNG: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0a, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];
    const ONE_BY_ONE_PNG_SHA256: &str =
        "ebf4f635a17d10d6eb46ba680b70142419aa3220f228001a036d311a22ee9d2a";

    #[test]
    fn import_image_copies_into_app_asset_dir_and_records_manifest_entry() {
        let dir = tempfile::tempdir().unwrap();
        let source_path = dir.path().join("source.png");
        std::fs::write(&source_path, ONE_BY_ONE_PNG).unwrap();
        let mut manifest = AssetManifestDocument::default();

        let imported = import_image_asset(dir.path(), &source_path, &mut manifest).unwrap();

        let store_dir = global_sheet_store_dir(dir.path());
        let asset_dir = store_dir.join("assets");
        assert_eq!(manifest.assets, vec![imported.entry.clone()]);
        assert!(imported.copied_path.exists());
        assert!(imported.copied_path.starts_with(&asset_dir));
        assert_eq!(imported.entry.media_type, "image/png");
        assert_eq!(
            imported.entry.pixel_dimensions,
            PixelDimensions {
                width: 1,
                height: 1
            }
        );
        assert_eq!(imported.entry.original_import_path, Some(source_path));
        assert_eq!(
            imported.entry.copied_path,
            imported.copied_path.strip_prefix(&store_dir).unwrap()
        );
        assert_eq!(imported.entry.content_hash, ONE_BY_ONE_PNG_SHA256);
    }

    #[test]
    fn invalid_image_import_leaves_manifest_and_assets_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let source_path = dir.path().join("source.txt");
        std::fs::write(&source_path, "not an image").unwrap();
        let mut manifest = AssetManifestDocument::default();

        let err = import_image_asset(dir.path(), &source_path, &mut manifest).unwrap_err();

        assert!(err.to_string().contains("invalid image"));
        assert!(manifest.assets.is_empty());
        assert!(!global_asset_dir(dir.path()).exists());
    }

    #[test]
    fn failed_asset_persistence_leaves_manifest_and_existing_asset_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let source_path = dir.path().join("source.png");
        std::fs::write(&source_path, ONE_BY_ONE_PNG).unwrap();
        let mut manifest = AssetManifestDocument::default();
        let mut observed_destination = None;

        let err = import_image_asset_with_persist(
            dir.path(),
            &source_path,
            &mut manifest,
            |destination, bytes| {
                assert_eq!(bytes, ONE_BY_ONE_PNG);
                observed_destination = Some(destination.to_path_buf());
                std::fs::create_dir_all(destination.parent().unwrap())?;
                std::fs::write(destination, b"stable existing asset")?;
                Err(EngineError::Io(std::io::Error::other("persist failed")))
            },
        )
        .unwrap_err();

        let destination = observed_destination.unwrap();
        assert!(err.to_string().contains("persist failed"));
        assert!(manifest.assets.is_empty());
        assert_eq!(
            std::fs::read_to_string(destination).unwrap(),
            "stable existing asset"
        );
    }

    #[test]
    fn import_image_creates_recovery_snapshot_before_manifest_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let source_path = dir.path().join("source.png");
        std::fs::write(&source_path, ONE_BY_ONE_PNG).unwrap();
        let manifest_path = global_asset_manifest_path(dir.path());
        std::fs::create_dir_all(manifest_path.parent().unwrap()).unwrap();
        std::fs::write(&manifest_path, "assets = []\n").unwrap();
        let mut manifest = AssetManifestDocument::default();

        import_image_asset(dir.path(), &source_path, &mut manifest).unwrap();

        let snapshot_count = std::fs::read_dir(global_recovery_dir(dir.path()))
            .unwrap()
            .count();
        assert_eq!(snapshot_count, 1);
    }

    #[test]
    fn missing_asset_health_keeps_manifest_data() {
        let dir = tempfile::tempdir().unwrap();
        let entry = AssetEntry {
            asset_id: AssetId::from_string("asset-missing"),
            copied_path: PathBuf::from("assets").join("missing.png"),
            content_hash: "hash".to_owned(),
            media_type: "image/png".to_owned(),
            pixel_dimensions: PixelDimensions {
                width: 1,
                height: 1,
            },
            original_import_path: Some(PathBuf::from("original.png")),
            display_name: None,
            extensions: ExtensionPayload::default(),
        };
        let manifest = AssetManifestDocument {
            assets: vec![entry.clone()],
            ..AssetManifestDocument::default()
        };

        let health = asset_health(dir.path(), &manifest);

        assert_eq!(health.len(), 1);
        assert_eq!(health[0].entry, entry);
        assert!(health[0].missing);
    }
}
