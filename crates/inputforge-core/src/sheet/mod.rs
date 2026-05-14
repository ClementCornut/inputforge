//! Mapping Sheet Builder sidecar contracts.

pub mod assets;
pub mod ids;
pub mod model;
pub mod recovery;
pub mod store;

#[doc(inline)]
pub use assets::{AssetHealth, ImportedAsset, asset_health, global_asset_dir, import_image_asset};
#[doc(inline)]
pub use ids::{
    AnchorId, AssetId, BlockId, LineId, MappingMetadataId, RecoverySnapshotId, SheetId, TemplateId,
    TemplateInstanceId,
};
#[doc(inline)]
pub use model::{
    AnchorAssignment, AnchorBinding, AnchorPosition, AssetEntry, AssetManifestDocument,
    BlockContent, BlockPreset, BoardRect, BoardSize, DeviceMatchingHint, DeviceTemplate,
    ExportSettings, ExtensionPayload, GroupingHint, GroupingKind, InputTypeHint, LayoutPreset,
    LineEndpoint, LineStyle, MAPPING_SHEET_SCHEMA_VERSION, MappingDisplayMetadata,
    MappingMetadataDocument, MappingRef, MappingSheet, ModeMappingSlot, PixelDimensions,
    ProfileSheetsDocument, SheetAnchorOverride, SheetBlock, SheetLine, SheetText, SheetTextKind,
    SidecarDocument, SidecarHeader, TemplateAnchor, TemplateInstance, TemplateStoreDocument,
    TokenPreset, validate_unique_mapping_metadata_refs,
};
#[doc(inline)]
pub use recovery::{
    RecoverySnapshotFile, RecoverySnapshotManifest, create_recovery_snapshot,
    external_profile_recovery_dir, global_recovery_dir, profile_recovery_dir,
};
#[doc(inline)]
pub use store::{
    external_profile_mapping_metadata_path, external_profile_sheets_path,
    external_profile_sidecar_dir, global_asset_manifest_path, global_sheet_store_dir,
    global_templates_path, load_asset_manifest, load_mapping_metadata, load_profile_sheets,
    load_template_store, profile_mapping_metadata_path, profile_sheets_path, profile_sidecar_dir,
    save_asset_manifest, save_mapping_metadata, save_profile_sheets, save_template_store,
};
