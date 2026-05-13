//! Mapping Sheet Builder sidecar contracts.

pub mod ids;
pub mod model;
pub mod recovery;
pub mod store;

#[doc(inline)]
pub use ids::{
    AnchorId, AssetId, BlockId, LineId, MappingMetadataId, RecoverySnapshotId, SheetId, TemplateId,
    TemplateInstanceId,
};
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
