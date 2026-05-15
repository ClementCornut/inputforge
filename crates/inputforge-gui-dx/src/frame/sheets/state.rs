use inputforge_core::sheet::{
    AnchorAssignment, AnchorBinding, AnchorId, AnchorPosition, AssetEntry, AssetId, DeviceTemplate,
    ExtensionPayload, TemplateId, TokenPreset,
};
use inputforge_core::types::{DeviceId, InputAddress, InputId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SheetTool {
    #[default]
    Select,
    PlaceAnchor,
    CaptureAssign,
    ManualAssign,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum AutosaveStatus {
    #[default]
    Clean,
    Dirty,
    Saving,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CaptureStatus {
    Unavailable(String),
    Idle,
    Armed(AnchorId),
    Assigned(AnchorId),
    TimedOut(AnchorId),
    Canceled(AnchorId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ManualInputKind {
    #[default]
    Button,
    Axis,
    Hat,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SheetsState {
    pub templates: Vec<DeviceTemplate>,
    pub assets: Vec<AssetEntry>,
    pub asset_health: Vec<inputforge_core::sheet::AssetHealth>,
    pub selected_template_id: Option<TemplateId>,
    pub selected_anchor_id: Option<AnchorId>,
    pub tool: SheetTool,
    pub autosave: AutosaveStatus,
    pub capture: CaptureStatus,
    pub last_error: Option<String>,
}

impl Default for SheetsState {
    fn default() -> Self {
        Self {
            templates: Vec::new(),
            assets: Vec::new(),
            asset_health: Vec::new(),
            selected_template_id: None,
            selected_anchor_id: None,
            tool: SheetTool::Select,
            autosave: AutosaveStatus::Clean,
            capture: CaptureStatus::Unavailable("live input is not available".to_owned()),
            last_error: None,
        }
    }
}

impl SheetsState {
    pub(crate) fn from_documents(
        documents: &crate::frame::sheets::authoring::SheetsDocuments,
    ) -> Self {
        Self {
            templates: documents.templates.templates.clone(),
            assets: documents.assets.assets.clone(),
            asset_health: documents.asset_health.clone(),
            selected_template_id: documents
                .templates
                .templates
                .first()
                .map(|template| template.template_id.clone()),
            selected_anchor_id: None,
            tool: SheetTool::Select,
            autosave: AutosaveStatus::Clean,
            capture: CaptureStatus::Unavailable("live input is not available".to_owned()),
            last_error: None,
        }
    }

    pub(crate) fn apply_to_documents(
        &self,
        documents: &mut crate::frame::sheets::authoring::SheetsDocuments,
    ) {
        documents.templates.templates.clone_from(&self.templates);
        documents.assets.assets.clone_from(&self.assets);
    }

    pub(crate) fn create_template_from_asset(
        &mut self,
        asset_id: AssetId,
        display_name: impl Into<String>,
    ) -> TemplateId {
        let template_id = TemplateId::new();
        self.templates.push(DeviceTemplate {
            template_id: template_id.clone(),
            display_name: display_name.into(),
            matching_hints: Vec::new(),
            asset_ids: vec![asset_id],
            anchors: Vec::new(),
            default_anchor_bindings: Vec::new(),
            grouping_hints: Vec::new(),
            default_token_preset: TokenPreset::Standard,
            extensions: ExtensionPayload::default(),
        });
        self.selected_template_id = Some(template_id.clone());
        self.selected_anchor_id = None;
        self.mark_dirty();
        template_id
    }

    pub(crate) fn place_anchor(&mut self, position: AnchorPosition) -> AnchorId {
        let anchor_id = AnchorId::new();
        let Some(template) = self.selected_template_mut() else {
            return anchor_id;
        };

        let label = format!("Anchor {}", template.anchors.len() + 1);
        template
            .anchors
            .push(inputforge_core::sheet::TemplateAnchor {
                anchor_id: anchor_id.clone(),
                label,
                position,
                input_type_hint: None,
                grouping_hint: None,
                device_matching_hint: None,
                extensions: ExtensionPayload::default(),
            });
        self.selected_anchor_id = Some(anchor_id.clone());
        self.mark_dirty();
        anchor_id
    }

    pub(crate) fn selected_template(&self) -> Option<&DeviceTemplate> {
        let selected_template_id = self.selected_template_id.as_ref()?;
        self.templates
            .iter()
            .find(|template| &template.template_id == selected_template_id)
    }

    pub(crate) fn selected_asset(&self) -> Option<&AssetEntry> {
        let asset_id = self.selected_template()?.asset_ids.first()?;
        self.assets.iter().find(|asset| &asset.asset_id == asset_id)
    }

    pub(crate) fn selected_asset_health(&self) -> Option<&inputforge_core::sheet::AssetHealth> {
        let asset_id = &self.selected_asset()?.asset_id;
        self.asset_health
            .iter()
            .find(|health| &health.entry.asset_id == asset_id)
    }

    pub(crate) fn selected_asset_missing(&self) -> bool {
        self.selected_asset_health()
            .is_some_and(|health| health.missing)
    }

    pub(crate) fn arm_capture(&mut self, anchor_id: AnchorId) {
        self.capture = CaptureStatus::Armed(anchor_id);
    }

    pub(crate) fn update_selected_anchor_label(&mut self, label: impl Into<String>) {
        if let Some(anchor) = self.selected_anchor_mut() {
            anchor.label = label.into();
            self.mark_dirty();
        }
    }

    pub(crate) fn update_selected_anchor_position(&mut self, position: AnchorPosition) {
        if let Some(anchor) = self.selected_anchor_mut() {
            anchor.position = position;
            self.mark_dirty();
        }
    }

    pub(crate) fn assign_selected_anchor(
        &mut self,
        input: InputAddress,
        assignment: AnchorAssignment,
    ) {
        let Some(anchor_id) = self.selected_anchor_id.clone() else {
            return;
        };

        let Some(template) = self.selected_template_mut() else {
            return;
        };

        template
            .default_anchor_bindings
            .retain(|binding| binding.anchor_id != anchor_id);
        template.default_anchor_bindings.push(AnchorBinding {
            anchor_id,
            input,
            captured_device_fingerprint: None,
            assignment,
        });
        self.mark_dirty();
    }

    pub(crate) fn assign_manual_selected_anchor(
        &mut self,
        device_id: impl AsRef<str>,
        input_kind: ManualInputKind,
        input_index: u8,
    ) -> Result<(), String> {
        let trimmed_device_id = device_id.as_ref().trim();
        if trimmed_device_id.is_empty() {
            let message = "Device id is required for manual assignment.".to_owned();
            self.last_error = Some(message.clone());
            return Err(message);
        }

        let input = match input_kind {
            ManualInputKind::Button => InputId::Button { index: input_index },
            ManualInputKind::Axis => InputId::Axis { index: input_index },
            ManualInputKind::Hat => InputId::Hat { index: input_index },
        };

        self.assign_selected_anchor(
            InputAddress::Bound {
                device: DeviceId(trimmed_device_id.to_owned()),
                input,
            },
            AnchorAssignment::Manual,
        );
        Ok(())
    }

    pub(crate) fn cancel_capture(&mut self) {
        if let CaptureStatus::Armed(anchor_id) = &self.capture {
            self.capture = CaptureStatus::Canceled(anchor_id.clone());
        }
    }

    pub(crate) fn capture_timed_out(&mut self) {
        if let CaptureStatus::Armed(anchor_id) = &self.capture {
            self.capture = CaptureStatus::TimedOut(anchor_id.clone());
        }
    }

    pub(crate) fn mark_saving(&mut self) {
        self.autosave = AutosaveStatus::Saving;
        self.last_error = None;
    }

    pub(crate) fn mark_saved(&mut self) {
        self.autosave = AutosaveStatus::Clean;
        self.last_error = None;
    }

    pub(crate) fn mark_failed(&mut self, error: impl Into<String>) {
        self.autosave = AutosaveStatus::Failed;
        self.last_error = Some(error.into());
    }

    fn mark_dirty(&mut self) {
        self.autosave = AutosaveStatus::Dirty;
        self.last_error = None;
    }

    fn selected_template_mut(&mut self) -> Option<&mut DeviceTemplate> {
        let selected_template_id = self.selected_template_id.as_ref()?;
        self.templates
            .iter_mut()
            .find(|template| &template.template_id == selected_template_id)
    }

    fn selected_anchor_mut(&mut self) -> Option<&mut inputforge_core::sheet::TemplateAnchor> {
        let selected_anchor_id = self.selected_anchor_id.clone()?;
        self.selected_template_mut()?
            .anchors
            .iter_mut()
            .find(|anchor| anchor.anchor_id == selected_anchor_id)
    }
}

pub(crate) fn normalized_image_point(
    client_x: f64,
    client_y: f64,
    stage_left: f64,
    stage_top: f64,
    stage_width: f64,
    stage_height: f64,
) -> Option<AnchorPosition> {
    if stage_width <= 0.0 || stage_height <= 0.0 {
        return None;
    }

    Some(AnchorPosition {
        x: ((client_x - stage_left) / stage_width).clamp(0.0, 1.0),
        y: ((client_y - stage_top) / stage_height).clamp(0.0, 1.0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use inputforge_core::sheet::{AssetHealth, InputTypeHint, PixelDimensions, TemplateAnchor};

    fn button_input(index: u8) -> InputAddress {
        InputAddress::Bound {
            device: DeviceId("device-1".to_owned()),
            input: InputId::Button { index },
        }
    }

    fn state_with_template() -> SheetsState {
        let asset_id = AssetId::from_string("asset-1");
        let template_id = TemplateId::from_string("template-1");
        let anchor_id = AnchorId::from_string("anchor-1");

        SheetsState {
            templates: vec![DeviceTemplate {
                template_id: template_id.clone(),
                display_name: "Test template".to_owned(),
                matching_hints: Vec::new(),
                asset_ids: vec![asset_id.clone()],
                anchors: vec![TemplateAnchor {
                    anchor_id: anchor_id.clone(),
                    label: "Trigger".to_owned(),
                    position: AnchorPosition { x: 0.25, y: 0.5 },
                    input_type_hint: Some(InputTypeHint::Button),
                    grouping_hint: None,
                    device_matching_hint: None,
                    extensions: ExtensionPayload::default(),
                }],
                default_anchor_bindings: Vec::new(),
                grouping_hints: Vec::new(),
                default_token_preset: TokenPreset::Standard,
                extensions: ExtensionPayload::default(),
            }],
            assets: vec![AssetEntry {
                asset_id,
                copied_path: PathBuf::from("assets/asset-1.png"),
                content_hash: "hash".to_owned(),
                media_type: "image/png".to_owned(),
                pixel_dimensions: PixelDimensions {
                    width: 64,
                    height: 32,
                },
                original_import_path: None,
                extensions: ExtensionPayload::default(),
            }],
            asset_health: Vec::new(),
            selected_template_id: Some(template_id),
            selected_anchor_id: Some(anchor_id),
            tool: SheetTool::Select,
            autosave: AutosaveStatus::Clean,
            capture: CaptureStatus::Unavailable("live input is not available".to_owned()),
            last_error: None,
        }
    }

    #[test]
    fn editing_anchor_label_and_position_keeps_anchor_id_stable() {
        let mut state = state_with_template();
        let anchor_id = state.selected_anchor_id.clone().unwrap();

        state.update_selected_anchor_label("Fire");
        state.update_selected_anchor_position(AnchorPosition { x: 0.5, y: 0.75 });

        let anchor = &state.templates[0].anchors[0];
        assert_eq!(anchor.anchor_id, anchor_id);
        assert_eq!(anchor.label, "Fire");
        assert_eq!(anchor.position, AnchorPosition { x: 0.5, y: 0.75 });
        assert_eq!(state.autosave, AutosaveStatus::Dirty);
    }

    #[test]
    fn placing_anchor_adds_selected_anchor_in_image_space() {
        let mut state = state_with_template();

        let anchor_id = state.place_anchor(AnchorPosition { x: 0.75, y: 0.125 });

        let template = &state.templates[0];
        let anchor = template
            .anchors
            .iter()
            .find(|anchor| anchor.anchor_id == anchor_id)
            .unwrap();
        assert_eq!(template.anchors.len(), 2);
        assert_eq!(anchor.label, "Anchor 2");
        assert_eq!(anchor.position, AnchorPosition { x: 0.75, y: 0.125 });
        assert_eq!(state.selected_anchor_id, Some(anchor_id));
        assert_eq!(state.autosave, AutosaveStatus::Dirty);
    }

    #[test]
    fn missing_selected_asset_is_reported_without_dropping_template_data() {
        let mut state = state_with_template();
        let asset = state.assets[0].clone();
        state.asset_health = vec![AssetHealth {
            entry: asset.clone(),
            copied_absolute_path: PathBuf::from("missing/asset-1.png"),
            missing: true,
        }];

        assert_eq!(state.selected_asset(), Some(&asset));
        assert!(state.selected_asset_missing());
        assert_eq!(state.templates[0].anchors.len(), 1);
        assert_eq!(state.templates[0].anchors[0].label, "Trigger");
    }

    #[test]
    fn assigning_input_replaces_template_default_binding_for_anchor_only() {
        let mut state = state_with_template();

        state.assign_selected_anchor(button_input(1), AnchorAssignment::Captured);
        state.assign_selected_anchor(button_input(2), AnchorAssignment::Manual);

        let anchor_id = state.selected_anchor_id.clone().unwrap();
        let bindings: Vec<&AnchorBinding> = state.templates[0]
            .default_anchor_bindings
            .iter()
            .filter(|binding| binding.anchor_id == anchor_id)
            .collect();

        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].input, button_input(2));
        assert_eq!(bindings[0].assignment, AnchorAssignment::Manual);
        assert_eq!(state.autosave, AutosaveStatus::Dirty);
    }

    #[test]
    fn manual_assignment_builds_structured_input_address_and_marks_dirty() {
        let mut state = state_with_template();

        let result = state.assign_manual_selected_anchor(
            " device:stick-1/button-7 ",
            ManualInputKind::Axis,
            7,
        );

        assert_eq!(result, Ok(()));
        assert_eq!(state.templates[0].default_anchor_bindings.len(), 1);
        assert_eq!(
            state.templates[0].default_anchor_bindings[0].input,
            InputAddress::Bound {
                device: DeviceId("device:stick-1/button-7".to_owned()),
                input: InputId::Axis { index: 7 },
            }
        );
        assert_eq!(
            state.templates[0].default_anchor_bindings[0].assignment,
            AnchorAssignment::Manual
        );
        assert_eq!(state.autosave, AutosaveStatus::Dirty);
        assert_eq!(state.last_error, None);
    }

    #[test]
    fn manual_assignment_rejects_empty_device_id_without_mutating_bindings() {
        let mut state = state_with_template();

        let result = state.assign_manual_selected_anchor("   ", ManualInputKind::Button, 0);

        assert_eq!(
            result,
            Err("Device id is required for manual assignment.".to_owned())
        );
        assert!(state.templates[0].default_anchor_bindings.is_empty());
        assert_eq!(state.autosave, AutosaveStatus::Clean);
        assert_eq!(
            state.last_error,
            Some("Device id is required for manual assignment.".to_owned())
        );
    }

    #[test]
    fn capture_cancel_leaves_unassigned_selected_anchor() {
        let mut state = state_with_template();
        let anchor_id = state.selected_anchor_id.clone().unwrap();
        state.capture = CaptureStatus::Armed(anchor_id.clone());

        state.cancel_capture();

        assert_eq!(state.capture, CaptureStatus::Canceled(anchor_id));
        assert!(state.templates[0].default_anchor_bindings.is_empty());
        assert_eq!(state.autosave, AutosaveStatus::Clean);
    }

    #[test]
    fn applying_state_to_documents_preserves_document_metadata() {
        let mut state = state_with_template();
        state.update_selected_anchor_label("Fire");

        let mut documents = crate::frame::sheets::authoring::SheetsDocuments {
            templates: inputforge_core::sheet::TemplateStoreDocument::default(),
            assets: inputforge_core::sheet::AssetManifestDocument::default(),
            asset_health: Vec::new(),
        };
        documents.templates.header.app_version_created = "template-created".to_owned();
        documents.assets.header.app_version_created = "asset-created".to_owned();

        state.apply_to_documents(&mut documents);

        assert_eq!(documents.templates.templates, state.templates);
        assert_eq!(documents.assets.assets, state.assets);
        assert_eq!(
            documents.templates.header.app_version_created,
            "template-created"
        );
        assert_eq!(documents.assets.header.app_version_created, "asset-created");
    }
}
