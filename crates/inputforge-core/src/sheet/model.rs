//! Defines persisted sidecar documents for mapping sheet authoring.
// Rust guideline compliant 2026-05-13

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::ids::{
    AnchorId, AssetId, BlockId, LineId, MappingMetadataId, SheetId, TemplateId, TemplateInstanceId,
};
use crate::error::{EngineError, Result};
use crate::profile::ProfileId;
use crate::types::InputAddress;

/// Current schema version for mapping sheet sidecar documents.
pub const MAPPING_SHEET_SCHEMA_VERSION: u32 = 1;

/// Extension key-value payload preserved during sidecar roundtrips.
pub type ExtensionPayload = BTreeMap<String, toml::Value>;

fn extension_payload_is_empty(payload: &ExtensionPayload) -> bool {
    payload.is_empty()
}

/// Common persisted header shared by all sidecar documents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidecarHeader {
    pub schema_version: u32,
    pub app_version_created: String,
    pub app_version_last_saved: String,
}

impl SidecarHeader {
    /// Creates a header for a document authored by the current crate version.
    #[must_use]
    pub fn new() -> Self {
        let current_version = env!("CARGO_PKG_VERSION").to_owned();
        Self {
            schema_version: MAPPING_SHEET_SCHEMA_VERSION,
            app_version_created: current_version.clone(),
            app_version_last_saved: current_version,
        }
    }

    /// Marks the document as saved by the current crate version.
    pub fn mark_saved_by_current_app(&mut self) {
        env!("CARGO_PKG_VERSION").clone_into(&mut self.app_version_last_saved);
    }
}

impl Default for SidecarHeader {
    fn default() -> Self {
        Self::new()
    }
}

/// Provides mutable access to the common sidecar header.
pub trait SidecarDocument {
    fn header_mut(&mut self) -> &mut SidecarHeader;
}

/// Persisted device template collection.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TemplateStoreDocument {
    #[serde(flatten)]
    pub header: SidecarHeader,
    #[serde(default)]
    pub templates: Vec<DeviceTemplate>,
    #[serde(default, flatten, skip_serializing_if = "extension_payload_is_empty")]
    pub extensions: ExtensionPayload,
}

impl SidecarDocument for TemplateStoreDocument {
    fn header_mut(&mut self) -> &mut SidecarHeader {
        &mut self.header
    }
}

/// Persisted imported asset manifest.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AssetManifestDocument {
    #[serde(flatten)]
    pub header: SidecarHeader,
    #[serde(default)]
    pub assets: Vec<AssetEntry>,
    #[serde(default, flatten, skip_serializing_if = "extension_payload_is_empty")]
    pub extensions: ExtensionPayload,
}

impl SidecarDocument for AssetManifestDocument {
    fn header_mut(&mut self) -> &mut SidecarHeader {
        &mut self.header
    }
}

/// Persisted sheets for a single profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileSheetsDocument {
    #[serde(flatten)]
    pub header: SidecarHeader,
    pub profile_id: ProfileId,
    #[serde(default)]
    pub sheets: Vec<MappingSheet>,
    #[serde(default, flatten, skip_serializing_if = "extension_payload_is_empty")]
    pub extensions: ExtensionPayload,
}

impl SidecarDocument for ProfileSheetsDocument {
    fn header_mut(&mut self) -> &mut SidecarHeader {
        &mut self.header
    }
}

/// Persisted mapping display metadata for a single profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MappingMetadataDocument {
    #[serde(flatten)]
    pub header: SidecarHeader,
    pub profile_id: ProfileId,
    #[serde(default)]
    pub records: Vec<MappingDisplayMetadata>,
    #[serde(default, flatten, skip_serializing_if = "extension_payload_is_empty")]
    pub extensions: ExtensionPayload,
}

impl SidecarDocument for MappingMetadataDocument {
    fn header_mut(&mut self) -> &mut SidecarHeader {
        &mut self.header
    }
}

/// Device template used to instantiate sheet anchors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceTemplate {
    pub template_id: TemplateId,
    pub display_name: String,
    #[serde(default)]
    pub matching_hints: Vec<DeviceMatchingHint>,
    #[serde(default)]
    pub asset_ids: Vec<AssetId>,
    #[serde(default)]
    pub anchors: Vec<TemplateAnchor>,
    #[serde(default)]
    pub default_anchor_bindings: Vec<AnchorBinding>,
    #[serde(default)]
    pub grouping_hints: Vec<GroupingHint>,
    pub default_token_preset: TokenPreset,
    #[serde(default, flatten, skip_serializing_if = "extension_payload_is_empty")]
    pub extensions: ExtensionPayload,
}

/// Optional hardware matching information for a device template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceMatchingHint {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vendor_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name_contains: Option<String>,
}

/// Imported asset entry used by sheet templates and boards.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetEntry {
    pub asset_id: AssetId,
    pub copied_path: PathBuf,
    pub content_hash: String,
    pub media_type: String,
    pub pixel_dimensions: PixelDimensions,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_import_path: Option<PathBuf>,
    #[serde(default, flatten, skip_serializing_if = "extension_payload_is_empty")]
    pub extensions: ExtensionPayload,
}

/// Pixel dimensions for raster assets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PixelDimensions {
    pub width: u32,
    pub height: u32,
}

/// Template anchor that can be bound to a concrete input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateAnchor {
    pub anchor_id: AnchorId,
    pub label: String,
    pub position: AnchorPosition,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_type_hint: Option<InputTypeHint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grouping_hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_matching_hint: Option<DeviceMatchingHint>,
    #[serde(default, flatten, skip_serializing_if = "extension_payload_is_empty")]
    pub extensions: ExtensionPayload,
}

/// Board-space anchor position.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AnchorPosition {
    pub x: f64,
    pub y: f64,
}

/// Coarse physical input type for template authoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputTypeHint {
    Axis,
    Button,
    Hat,
}

/// Template grouping kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupingKind {
    Directional,
    AxisPair,
    LogicalCluster,
}

/// Display token preset for authored mappings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenPreset {
    Standard,
    Emphasis,
    Warning,
}

/// Named template grouping hint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupingHint {
    pub name: String,
    #[serde(default)]
    pub anchor_ids: Vec<AnchorId>,
    pub kind: GroupingKind,
}

/// Mapping sheet board for one profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MappingSheet {
    pub id: SheetId,
    pub display_name: String,
    pub profile_id: ProfileId,
    pub layout_preset: LayoutPreset,
    pub board_size: BoardSize,
    pub zoom_default: f64,
    pub export: ExportSettings,
    #[serde(default)]
    pub template_instances: Vec<TemplateInstance>,
    #[serde(default)]
    pub mode_slots: Vec<ModeMappingSlot>,
    #[serde(default)]
    pub blocks: Vec<SheetBlock>,
    #[serde(default)]
    pub lines: Vec<SheetLine>,
    #[serde(default)]
    pub anchor_overrides: Vec<SheetAnchorOverride>,
    #[serde(default)]
    pub sheet_text: Vec<SheetText>,
    #[serde(default, flatten, skip_serializing_if = "extension_payload_is_empty")]
    pub extensions: ExtensionPayload,
}

/// Layout preset used by a sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutPreset {
    FreeBoard,
}

/// Sheet board dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoardSize {
    pub width: f64,
    pub height: f64,
}

/// Export preferences for a mapping sheet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportSettings {
    pub filename_prefix: String,
    pub include_background: bool,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            filename_prefix: "inputforge-sheet".to_owned(),
            include_background: true,
        }
    }
}

/// Instance of a device template placed on a sheet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateInstance {
    pub id: TemplateInstanceId,
    pub template_id: TemplateId,
    pub rect: BoardRect,
    #[serde(default)]
    pub anchor_bindings: Vec<AnchorBinding>,
    #[serde(default, flatten, skip_serializing_if = "extension_payload_is_empty")]
    pub extensions: ExtensionPayload,
}

/// Binding between a template anchor and an input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorBinding {
    pub anchor_id: AnchorId,
    pub input: InputAddress,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub captured_device_fingerprint: Option<String>,
    pub assignment: AnchorAssignment,
}

/// Source of an anchor assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnchorAssignment {
    Captured,
    Manual,
    Unavailable,
}

/// Mapping references grouped by a mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModeMappingSlot {
    pub mode_id: String,
    #[serde(default)]
    pub mapping_refs: Vec<MappingRef>,
}

/// Stable reference to a profile mapping.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MappingRef {
    pub mode_id: String,
    pub input: InputAddress,
    pub fallback_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_details: Option<String>,
}

/// Visual block placed on a sheet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SheetBlock {
    pub id: BlockId,
    pub preset: BlockPreset,
    pub rect: BoardRect,
    #[serde(default)]
    pub contents: Vec<BlockContent>,
    #[serde(default, flatten, skip_serializing_if = "extension_payload_is_empty")]
    pub extensions: ExtensionPayload,
}

/// Visual preset for a sheet block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockPreset {
    Mapping,
    Group,
    Axis,
    Legend,
    Note,
    Warning,
}

/// Content row or item stored inside a sheet block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockContent {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mapping_ref: Option<MappingRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<BlockContent>,
    #[serde(default, flatten, skip_serializing_if = "extension_payload_is_empty")]
    pub extensions: ExtensionPayload,
}

impl BlockContent {
    /// Creates text label content.
    #[must_use]
    pub fn text_label(text: impl Into<String>) -> Self {
        Self {
            kind: "text_label".to_owned(),
            mapping_ref: None,
            text: Some(text.into()),
            icon: None,
            children: Vec::new(),
            extensions: ExtensionPayload::default(),
        }
    }

    /// Creates mapping reference content.
    #[must_use]
    pub fn mapping_ref(mapping_ref: MappingRef) -> Self {
        Self {
            kind: "mapping_ref".to_owned(),
            mapping_ref: Some(mapping_ref),
            text: None,
            icon: None,
            children: Vec::new(),
            extensions: ExtensionPayload::default(),
        }
    }

    /// Returns whether this content kind is unknown to the current model.
    #[must_use]
    pub fn is_unknown(&self) -> bool {
        !matches!(
            self.kind.as_str(),
            "mapping_ref"
                | "text_label"
                | "icon"
                | "separator"
                | "directional_row"
                | "axis_row"
                | "caption"
                | "legend_item"
        )
    }
}

/// Board-space rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoardRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Visual connector between sheet elements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SheetLine {
    pub id: LineId,
    pub from: LineEndpoint,
    pub to: LineEndpoint,
    pub style: LineStyle,
    #[serde(default, flatten, skip_serializing_if = "extension_payload_is_empty")]
    pub extensions: ExtensionPayload,
}

/// Endpoint of a sheet line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineEndpoint {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_id: Option<BlockId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_id: Option<AnchorId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<AnchorPosition>,
}

/// Visual style for a sheet line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineStyle {
    Straight,
    Elbow,
}

/// Sheet-level override for a template anchor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SheetAnchorOverride {
    pub template_instance_id: TemplateInstanceId,
    pub anchor_id: AnchorId,
    pub position: AnchorPosition,
    #[serde(default, flatten, skip_serializing_if = "extension_payload_is_empty")]
    pub extensions: ExtensionPayload,
}

/// Free text rendered on a sheet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SheetText {
    pub kind: SheetTextKind,
    pub text: String,
}

impl SheetText {
    /// Creates caption text.
    #[must_use]
    pub fn caption(text: impl Into<String>) -> Self {
        Self {
            kind: SheetTextKind::Caption,
            text: text.into(),
        }
    }
}

/// Kind of free text rendered on a sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SheetTextKind {
    Caption,
    Legend,
    Note,
}

/// Display metadata for a mapping reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MappingDisplayMetadata {
    pub id: MappingMetadataId,
    pub mapping_ref: MappingRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classification_token: Option<String>,
    #[serde(default, flatten, skip_serializing_if = "extension_payload_is_empty")]
    pub extensions: ExtensionPayload,
}

/// Validates that metadata records do not duplicate mapping references.
///
/// # Errors
///
/// Returns [`EngineError::InvalidConfig`] when two records share the same
/// [`MappingRef`].
pub fn validate_unique_mapping_metadata_refs(document: &MappingMetadataDocument) -> Result<()> {
    let mut seen = HashSet::new();
    for record in &document.records {
        if !seen.insert(&record.mapping_ref) {
            return Err(EngineError::InvalidConfig {
                reason: "mapping metadata contains duplicate mapping_ref entries".to_owned(),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[expect(
        redundant_imports,
        reason = "Task requires explicit error import alongside super::*."
    )]
    use crate::error::EngineError;
    #[expect(
        redundant_imports,
        reason = "Task requires explicit profile ID import alongside super::*."
    )]
    use crate::profile::ProfileId;
    #[expect(
        redundant_imports,
        reason = "Task requires explicit input address import alongside super::*."
    )]
    use crate::types::InputAddress;
    use crate::types::{DeviceId, InputId};

    fn mapping_ref(index: u8) -> MappingRef {
        MappingRef {
            mode_id: "combat".to_owned(),
            input: InputAddress::Bound {
                device: DeviceId("stick-alpha".to_owned()),
                input: InputId::Button { index },
            },
            fallback_label: format!("Button {index}"),
            fallback_details: Some("Primary stick".to_owned()),
        }
    }

    #[test]
    fn legacy_template_without_default_anchor_bindings_loads_empty_bindings() {
        let toml = r#"
schema_version = 1
app_version_created = "0.2.0"
app_version_last_saved = "0.2.0"

[[templates]]
template_id = "template-stick-left"
display_name = "Left Stick"
default_token_preset = "standard"
"#;

        let document: TemplateStoreDocument = toml::from_str(toml).unwrap();

        assert_eq!(document.templates.len(), 1);
        assert!(document.templates[0].default_anchor_bindings.is_empty());
    }

    #[test]
    fn template_default_anchor_bindings_roundtrip() {
        let anchor_id = AnchorId::from_string("anchor-trigger");
        let document = TemplateStoreDocument {
            templates: vec![DeviceTemplate {
                template_id: TemplateId::from_string("template-stick-left"),
                display_name: "Left Stick".to_owned(),
                matching_hints: Vec::new(),
                asset_ids: Vec::new(),
                anchors: vec![TemplateAnchor {
                    anchor_id: anchor_id.clone(),
                    label: "Trigger".to_owned(),
                    position: AnchorPosition { x: 0.42, y: 0.18 },
                    input_type_hint: Some(InputTypeHint::Button),
                    grouping_hint: None,
                    device_matching_hint: None,
                    extensions: BTreeMap::default(),
                }],
                default_anchor_bindings: vec![AnchorBinding {
                    anchor_id: anchor_id.clone(),
                    input: InputAddress::Bound {
                        device: DeviceId("stick-alpha".to_owned()),
                        input: InputId::Button { index: 1 },
                    },
                    captured_device_fingerprint: Some("vid:044f pid:b10a".to_owned()),
                    assignment: AnchorAssignment::Captured,
                }],
                grouping_hints: Vec::new(),
                default_token_preset: TokenPreset::Standard,
                extensions: BTreeMap::default(),
            }],
            ..TemplateStoreDocument::default()
        };

        let toml = toml::to_string_pretty(&document).unwrap();
        let roundtripped: TemplateStoreDocument = toml::from_str(&toml).unwrap();

        assert_eq!(roundtripped, document);
        assert_eq!(
            roundtripped.templates[0].default_anchor_bindings[0].anchor_id,
            anchor_id
        );
        assert_eq!(
            roundtripped.templates[0].default_anchor_bindings[0].assignment,
            AnchorAssignment::Captured
        );
    }

    #[test]
    fn template_default_binding_edits_do_not_mutate_sheet_instance_bindings_or_overrides() {
        let anchor_id = AnchorId::from_string("anchor-trigger");
        let template_id = TemplateId::from_string("template-stick-left");
        let sheet_binding = AnchorBinding {
            anchor_id: anchor_id.clone(),
            input: InputAddress::Bound {
                device: DeviceId("stick-bravo".to_owned()),
                input: InputId::Button { index: 7 },
            },
            captured_device_fingerprint: None,
            assignment: AnchorAssignment::Manual,
        };
        let override_position = AnchorPosition { x: 128.0, y: 64.0 };
        let template_document = TemplateStoreDocument {
            templates: vec![DeviceTemplate {
                template_id: template_id.clone(),
                display_name: "Left Stick".to_owned(),
                matching_hints: Vec::new(),
                asset_ids: Vec::new(),
                anchors: vec![TemplateAnchor {
                    anchor_id: anchor_id.clone(),
                    label: "Trigger".to_owned(),
                    position: AnchorPosition { x: 0.2, y: 0.3 },
                    input_type_hint: Some(InputTypeHint::Button),
                    grouping_hint: None,
                    device_matching_hint: None,
                    extensions: BTreeMap::default(),
                }],
                default_anchor_bindings: vec![AnchorBinding {
                    anchor_id: anchor_id.clone(),
                    input: InputAddress::Bound {
                        device: DeviceId("stick-alpha".to_owned()),
                        input: InputId::Button { index: 1 },
                    },
                    captured_device_fingerprint: None,
                    assignment: AnchorAssignment::Manual,
                }],
                grouping_hints: Vec::new(),
                default_token_preset: TokenPreset::Standard,
                extensions: BTreeMap::default(),
            }],
            ..TemplateStoreDocument::default()
        };
        let profile_id = ProfileId::new();
        let sheet_document = ProfileSheetsDocument {
            header: SidecarHeader::new(),
            profile_id: profile_id.clone(),
            sheets: vec![MappingSheet {
                id: SheetId::from_string("sheet-primary"),
                display_name: "Flight".to_owned(),
                profile_id,
                layout_preset: LayoutPreset::FreeBoard,
                board_size: BoardSize {
                    width: 1920.0,
                    height: 1080.0,
                },
                zoom_default: 1.0,
                export: ExportSettings::default(),
                template_instances: vec![TemplateInstance {
                    id: TemplateInstanceId::from_string("instance-left"),
                    template_id,
                    rect: BoardRect {
                        x: 10.0,
                        y: 20.0,
                        width: 400.0,
                        height: 300.0,
                    },
                    anchor_bindings: vec![sheet_binding.clone()],
                    extensions: BTreeMap::default(),
                }],
                mode_slots: Vec::new(),
                blocks: Vec::new(),
                lines: Vec::new(),
                anchor_overrides: vec![SheetAnchorOverride {
                    template_instance_id: TemplateInstanceId::from_string("instance-left"),
                    anchor_id: anchor_id.clone(),
                    position: override_position,
                    extensions: BTreeMap::default(),
                }],
                sheet_text: Vec::new(),
                extensions: BTreeMap::default(),
            }],
            extensions: BTreeMap::default(),
        };

        let template_toml = toml::to_string_pretty(&template_document).unwrap();
        let sheet_toml = toml::to_string_pretty(&sheet_document).unwrap();
        let mut loaded_template_document: TemplateStoreDocument =
            toml::from_str(&template_toml).unwrap();
        let loaded_sheet_document: ProfileSheetsDocument = toml::from_str(&sheet_toml).unwrap();
        let expected_sheet_document = loaded_sheet_document.clone();

        loaded_template_document.templates[0].default_anchor_bindings[0].input =
            InputAddress::Bound {
                device: DeviceId("stick-alpha".to_owned()),
                input: InputId::Button { index: 2 },
            };
        loaded_template_document.templates[0].anchors[0].position =
            AnchorPosition { x: 0.4, y: 0.5 };

        let updated_template_toml = toml::to_string_pretty(&loaded_template_document).unwrap();
        let reloaded_template_document: TemplateStoreDocument =
            toml::from_str(&updated_template_toml).unwrap();
        let reloaded_sheet_toml = toml::to_string_pretty(&loaded_sheet_document).unwrap();
        let reloaded_sheet_document: ProfileSheetsDocument =
            toml::from_str(&reloaded_sheet_toml).unwrap();

        assert_eq!(
            reloaded_template_document.templates[0].default_anchor_bindings[0].input,
            InputAddress::Bound {
                device: DeviceId("stick-alpha".to_owned()),
                input: InputId::Button { index: 2 },
            }
        );
        assert_eq!(
            reloaded_template_document.templates[0].anchors[0].position,
            AnchorPosition { x: 0.4, y: 0.5 }
        );
        assert_eq!(reloaded_sheet_document, expected_sheet_document);
        assert_eq!(
            reloaded_sheet_document.sheets[0].template_instances[0].anchor_bindings,
            vec![sheet_binding]
        );
        assert_eq!(
            reloaded_sheet_document.sheets[0].anchor_overrides[0].position,
            override_position
        );
        assert_ne!(
            reloaded_sheet_document.sheets[0].template_instances[0].anchor_bindings[0].input,
            InputAddress::Bound {
                device: DeviceId("stick-alpha".to_owned()),
                input: InputId::Button { index: 2 },
            }
        );
    }

    #[test]
    fn documents_roundtrip_with_versions_and_ids() {
        let profile_id = ProfileId::new();
        let mapping_ref = mapping_ref(1);
        let sheet = MappingSheet {
            id: SheetId::from_string("sheet-main"),
            display_name: "Main sheet".to_owned(),
            profile_id: profile_id.clone(),
            layout_preset: LayoutPreset::FreeBoard,
            board_size: BoardSize {
                width: 1920.0,
                height: 1080.0,
            },
            zoom_default: 0.75,
            export: ExportSettings::default(),
            template_instances: Vec::new(),
            mode_slots: vec![ModeMappingSlot {
                mode_id: "combat".to_owned(),
                mapping_refs: vec![mapping_ref.clone()],
            }],
            blocks: vec![SheetBlock {
                id: BlockId::from_string("block-primary"),
                preset: BlockPreset::Mapping,
                rect: BoardRect {
                    x: 10.0,
                    y: 20.0,
                    width: 220.0,
                    height: 120.0,
                },
                contents: vec![
                    BlockContent::mapping_ref(mapping_ref),
                    BlockContent::text_label("Fire primary"),
                ],
                extensions: ExtensionPayload::default(),
            }],
            lines: Vec::new(),
            anchor_overrides: Vec::new(),
            sheet_text: vec![SheetText::caption("Combat bindings")],
            extensions: ExtensionPayload::default(),
        };
        let document = ProfileSheetsDocument {
            header: SidecarHeader::new(),
            profile_id,
            sheets: vec![sheet],
            extensions: ExtensionPayload::default(),
        };

        let encoded = toml::to_string(&document).unwrap();
        assert!(encoded.contains("schema_version = 1"));
        assert!(encoded.contains("app_version_created"));
        assert!(encoded.contains("app_version_last_saved"));

        let decoded: ProfileSheetsDocument = toml::from_str(&encoded).unwrap();
        assert_eq!(decoded, document);
    }

    #[test]
    fn unknown_block_content_roundtrips_without_payload_loss() {
        let source = r#"
kind = "radial_meter"
radius = 42

[custom]
theme = "amber"
"#;

        let content: BlockContent = toml::from_str(source).unwrap();
        assert!(content.is_unknown());
        assert_eq!(content.extensions["radius"].as_integer(), Some(42));
        assert_eq!(
            content.extensions["custom"]
                .as_table()
                .unwrap()
                .get("theme")
                .unwrap()
                .as_str(),
            Some("amber")
        );

        let encoded = toml::to_string(&content).unwrap();
        let decoded: BlockContent = toml::from_str(&encoded).unwrap();
        assert_eq!(decoded.kind, "radial_meter");
        assert_eq!(decoded.extensions["radius"].as_integer(), Some(42));
        assert_eq!(
            decoded.extensions["custom"]
                .as_table()
                .unwrap()
                .get("theme")
                .unwrap()
                .as_str(),
            Some("amber")
        );
    }

    #[test]
    fn document_extensions_roundtrip_without_payload_loss() {
        let source = r#"
schema_version = 1
app_version_created = "0.1.0"
app_version_last_saved = "0.1.0"
generator = "fixture"

[[assets]]
asset_id = "asset-stick"
copied_path = "assets/stick.png"
content_hash = "sha256:abc"
media_type = "image/png"
original_import_path = "C:/imports/stick.png"
tint = "blue"

[assets.pixel_dimensions]
width = 640
height = 480
"#;

        let document: AssetManifestDocument = toml::from_str(source).unwrap();
        assert_eq!(document.extensions["generator"].as_str(), Some("fixture"));
        assert_eq!(
            document.assets[0].copied_path,
            PathBuf::from("assets/stick.png")
        );
        assert_eq!(document.assets[0].pixel_dimensions.width, 640);
        assert_eq!(document.assets[0].extensions["tint"].as_str(), Some("blue"));

        let encoded = toml::to_string(&document).unwrap();
        let decoded: AssetManifestDocument = toml::from_str(&encoded).unwrap();
        assert_eq!(decoded.extensions["generator"].as_str(), Some("fixture"));
        assert_eq!(decoded.assets[0].extensions["tint"].as_str(), Some("blue"));
        assert_eq!(decoded.assets[0].pixel_dimensions.height, 480);
    }

    #[test]
    fn mapping_metadata_document_roundtrips_with_stable_record_ids() {
        let profile_id = ProfileId::new();
        let document = MappingMetadataDocument {
            header: SidecarHeader::new(),
            profile_id,
            records: vec![MappingDisplayMetadata {
                id: MappingMetadataId::from_string("metadata-primary-fire"),
                mapping_ref: mapping_ref(2),
                display_name: Some("Primary fire".to_owned()),
                category: Some("Weapons".to_owned()),
                classification_token: Some("weapon".to_owned()),
                extensions: ExtensionPayload::default(),
            }],
            extensions: ExtensionPayload::default(),
        };

        let encoded = toml::to_string(&document).unwrap();
        let decoded: MappingMetadataDocument = toml::from_str(&encoded).unwrap();

        assert_eq!(decoded.profile_id, document.profile_id);
        assert_eq!(
            decoded.records[0].id,
            MappingMetadataId::from_string("metadata-primary-fire")
        );
        assert_eq!(decoded.records[0].mapping_ref, mapping_ref(2));
        assert_eq!(
            decoded.records[0].classification_token.as_deref(),
            Some("weapon")
        );
    }

    #[test]
    fn mapping_metadata_extensions_roundtrip_without_payload_loss() {
        let source = r#"
schema_version = 1
app_version_created = "0.1.0"
app_version_last_saved = "0.1.0"
profile_id = "7266c542-02e5-4c76-bdc8-81135a72a746"
source = "designer"

[[records]]
id = "metadata-trigger"
display_name = "Trigger"
category = "Weapons"
color = "red"

[records.mapping_ref]
mode_id = "combat"
fallback_label = "Trigger"

[records.mapping_ref.input]
device = "stick-alpha"

[records.mapping_ref.input.input]
type = "button"
index = 3
"#;

        let document: MappingMetadataDocument = toml::from_str(source).unwrap();
        assert_eq!(document.extensions["source"].as_str(), Some("designer"));
        assert_eq!(
            document.records[0].extensions["color"].as_str(),
            Some("red")
        );

        let encoded = toml::to_string(&document).unwrap();
        let decoded: MappingMetadataDocument = toml::from_str(&encoded).unwrap();
        assert_eq!(decoded.extensions["source"].as_str(), Some("designer"));
        assert_eq!(decoded.records[0].extensions["color"].as_str(), Some("red"));
        assert_eq!(decoded.records[0].mapping_ref.input, mapping_ref(3).input);
        assert_eq!(decoded.records[0].mapping_ref.mode_id, "combat");
        assert_eq!(decoded.records[0].mapping_ref.fallback_label, "Trigger");
    }

    #[test]
    fn duplicate_metadata_mapping_refs_are_rejected() {
        let duplicate = mapping_ref(4);
        let document = MappingMetadataDocument {
            header: SidecarHeader::new(),
            profile_id: ProfileId::new(),
            records: vec![
                MappingDisplayMetadata {
                    id: MappingMetadataId::from_string("metadata-a"),
                    mapping_ref: duplicate.clone(),
                    display_name: Some("First".to_owned()),
                    category: None,
                    classification_token: None,
                    extensions: ExtensionPayload::default(),
                },
                MappingDisplayMetadata {
                    id: MappingMetadataId::from_string("metadata-b"),
                    mapping_ref: duplicate,
                    display_name: Some("Second".to_owned()),
                    category: None,
                    classification_token: None,
                    extensions: ExtensionPayload::default(),
                },
            ],
            extensions: ExtensionPayload::default(),
        };

        let err = validate_unique_mapping_metadata_refs(&document).unwrap_err();

        match err {
            EngineError::InvalidConfig { reason } => {
                assert_eq!(
                    reason,
                    "mapping metadata contains duplicate mapping_ref entries"
                );
            }
            other => panic!("expected invalid config error, got {other:?}"),
        }
    }
}
