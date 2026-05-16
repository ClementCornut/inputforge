use std::path::Path;

use inputforge_core::error::Result;
use inputforge_core::settings::AppSettings;
use inputforge_core::sheet::{
    AssetManifestDocument, DeviceTemplate, ImportedAsset, TemplateStoreDocument, asset_health,
    global_asset_manifest_path, global_templates_path, import_image_asset, load_asset_manifest,
    load_template_store, save_asset_manifest, save_template_store,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SheetsDocuments {
    pub templates: TemplateStoreDocument,
    pub assets: AssetManifestDocument,
    pub asset_health: Vec<inputforge_core::sheet::AssetHealth>,
}

pub(crate) fn load_sheets_documents() -> Result<SheetsDocuments> {
    let config_dir = AppSettings::config_dir();
    let templates = load_template_store(&global_templates_path(&config_dir))?;
    let assets = load_asset_manifest(&global_asset_manifest_path(&config_dir))?;
    let asset_health = asset_health(&config_dir, &assets);

    Ok(SheetsDocuments {
        templates,
        assets,
        asset_health,
    })
}

pub(crate) fn save_sheets_documents(documents: &mut SheetsDocuments) -> Result<()> {
    let config_dir = AppSettings::config_dir();
    save_sheets_documents_at(&config_dir, documents)
}

pub(crate) fn save_sheets_documents_at(
    config_dir: &Path,
    documents: &mut SheetsDocuments,
) -> Result<()> {
    let asset_result = save_asset_manifest(
        &global_asset_manifest_path(config_dir),
        &mut documents.assets,
    );
    let template_result =
        save_template_store(&global_templates_path(config_dir), &mut documents.templates);

    documents.asset_health = asset_health(config_dir, &documents.assets);

    classify_save_outcome(asset_result, template_result)
}

pub(crate) fn classify_save_outcome(
    asset_result: Result<()>,
    template_result: Result<()>,
) -> Result<()> {
    match (asset_result, template_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(asset_err), Ok(())) => Err(asset_err),
        (Ok(()), Err(template_err)) => Err(template_err),
        (Err(asset_err), Err(_template_err)) => Err(asset_err),
    }
}

pub(crate) fn import_template_asset(
    source_path: &Path,
) -> Result<(ImportedAsset, SheetsDocuments)> {
    let mut documents = load_sheets_documents()?;
    let imported = import_template_asset_into(source_path, &mut documents)?;

    Ok((imported, documents))
}

pub(crate) fn import_template_asset_into(
    source_path: &Path,
    documents: &mut SheetsDocuments,
) -> Result<ImportedAsset> {
    let config_dir = AppSettings::config_dir();
    let imported = import_image_asset(&config_dir, source_path, &mut documents.assets)?;

    save_asset_manifest(
        &global_asset_manifest_path(&config_dir),
        &mut documents.assets,
    )?;
    documents.asset_health = asset_health(&config_dir, &documents.assets);

    Ok(imported)
}

pub(crate) fn save_template_after_asset(
    documents: &mut SheetsDocuments,
    template: DeviceTemplate,
) -> Result<()> {
    documents.templates.templates.push(template);
    save_sheets_documents(documents)
}

#[cfg(test)]
#[cfg(unix)]
mod failure_injection_tests {
    use super::*;
    use inputforge_core::sheet::{
        AssetEntry, AssetManifestDocument, ExtensionPayload, PixelDimensions, TemplateStoreDocument,
    };

    #[test]
    fn save_attempts_both_writes_and_recovers_after_directory_becomes_writable() {
        use std::os::unix::fs::PermissionsExt as _;

        let parent = tempfile::tempdir().unwrap();
        let config_dir = parent.path().to_path_buf();
        let store_dir = config_dir.join("mapping-sheet-builder");
        std::fs::create_dir_all(&store_dir).unwrap();
        std::fs::set_permissions(&store_dir, std::fs::Permissions::from_mode(0o555)).unwrap();

        let mut documents = SheetsDocuments {
            templates: TemplateStoreDocument::default(),
            assets: AssetManifestDocument {
                assets: vec![AssetEntry {
                    asset_id: inputforge_core::sheet::AssetId::from_string("asset-x"),
                    copied_path: std::path::PathBuf::from("assets/asset-x.png"),
                    content_hash: "hash".to_owned(),
                    media_type: "image/png".to_owned(),
                    pixel_dimensions: PixelDimensions {
                        width: 1,
                        height: 1,
                    },
                    original_import_path: None,
                    extensions: ExtensionPayload::default(),
                }],
                ..AssetManifestDocument::default()
            },
            asset_health: Vec::new(),
        };

        let err = save_sheets_documents_at(&config_dir, &mut documents).unwrap_err();
        let message = err.to_string().to_lowercase();
        assert!(
            message.contains("denied") || message.contains("permission"),
            "expected permission failure, got: {err}"
        );

        // Both writes were attempted under the failing condition: nothing landed on disk because the
        // directory was read-only when each write opened its target. After lifting the permission
        // bit, recovery succeeds and both files now exist with the expected content.
        assert!(!store_dir.join("assets.toml").exists());
        assert!(!store_dir.join("templates.toml").exists());

        std::fs::set_permissions(&store_dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        save_sheets_documents_at(&config_dir, &mut documents).unwrap();

        let on_disk_assets = std::fs::read_to_string(store_dir.join("assets.toml")).unwrap();
        let on_disk_templates = std::fs::read_to_string(store_dir.join("templates.toml")).unwrap();
        assert!(on_disk_assets.contains("asset-x"));
        assert!(on_disk_templates.contains("schema_version"));
    }

    // Windows lacks an equivalent POSIX permission bit for this directory denial test.
    #[test]
    fn save_reports_template_branch_when_only_template_write_fails() {
        use std::os::unix::fs::PermissionsExt as _;

        let parent = tempfile::tempdir().unwrap();
        let config_dir = parent.path().to_path_buf();
        let store_dir = config_dir.join("mapping-sheet-builder");
        std::fs::create_dir_all(&store_dir).unwrap();

        // Pre-create templates.toml as a read-only file so the templates write fails while the
        // assets write succeeds. This exercises the (Ok, Err) arm of classify_save_outcome.
        let templates_path = store_dir.join("templates.toml");
        std::fs::write(&templates_path, "").unwrap();
        std::fs::set_permissions(&templates_path, std::fs::Permissions::from_mode(0o444)).unwrap();

        let mut documents = SheetsDocuments {
            templates: TemplateStoreDocument::default(),
            assets: AssetManifestDocument::default(),
            asset_health: Vec::new(),
        };

        let err = save_sheets_documents_at(&config_dir, &mut documents).unwrap_err();
        let message = err.to_string().to_lowercase();
        assert!(
            message.contains("denied") || message.contains("permission"),
            "expected permission failure on templates branch, got: {err}"
        );

        // Assets branch was attempted and succeeded; templates branch failed: classifier surfaces the
        // template error per the (Ok, Err) arm.
        assert!(store_dir.join("assets.toml").exists());
    }
}
