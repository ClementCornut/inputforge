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
    let asset_result = save_asset_manifest(
        &global_asset_manifest_path(&config_dir),
        &mut documents.assets,
    );
    let template_result = save_template_store(
        &global_templates_path(&config_dir),
        &mut documents.templates,
    );

    documents.asset_health = asset_health(&config_dir, &documents.assets);

    asset_result?;
    template_result?;
    Ok(())
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
