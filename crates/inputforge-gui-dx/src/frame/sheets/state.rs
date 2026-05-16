use std::collections::VecDeque;
use std::time::Instant;

use inputforge_core::sheet::{
    AnchorAssignment, AnchorBinding, AnchorId, AnchorPosition, AssetEntry, AssetId, AssetPlacement,
    AssetPlacementId, DeviceTemplate, ExtensionPayload, TemplateAnchor, TemplateId, TemplateRect,
    TokenPreset,
};
use inputforge_core::types::{DeviceId, InputAddress, InputId};

const SHEETS_HISTORY_CAP: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SheetTool {
    #[default]
    Select,
    PlaceAnchor,
    CaptureAssign,
    ManualAssign,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SheetLayoutPreset {
    Single,
    HorizontalPair,
    VerticalStack,
    TwoByTwo,
    Freeform,
}

impl SheetLayoutPreset {
    pub(crate) fn slot_rects(self) -> Vec<TemplateRect> {
        match self {
            Self::Single => vec![TemplateRect {
                x: 0.05,
                y: 0.05,
                w: 0.9,
                h: 0.9,
            }],
            Self::HorizontalPair => vec![
                TemplateRect {
                    x: 0.02,
                    y: 0.10,
                    w: 0.46,
                    h: 0.80,
                },
                TemplateRect {
                    x: 0.52,
                    y: 0.10,
                    w: 0.46,
                    h: 0.80,
                },
            ],
            Self::VerticalStack => vec![
                TemplateRect {
                    x: 0.10,
                    y: 0.02,
                    w: 0.80,
                    h: 0.46,
                },
                TemplateRect {
                    x: 0.10,
                    y: 0.52,
                    w: 0.80,
                    h: 0.46,
                },
            ],
            Self::TwoByTwo => vec![
                TemplateRect {
                    x: 0.02,
                    y: 0.02,
                    w: 0.46,
                    h: 0.46,
                },
                TemplateRect {
                    x: 0.52,
                    y: 0.02,
                    w: 0.46,
                    h: 0.46,
                },
                TemplateRect {
                    x: 0.02,
                    y: 0.52,
                    w: 0.46,
                    h: 0.46,
                },
                TemplateRect {
                    x: 0.52,
                    y: 0.52,
                    w: 0.46,
                    h: 0.46,
                },
            ],
            Self::Freeform => Vec::new(),
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SheetsLibraryTab {
    #[default]
    Templates,
    Assets,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SheetsEventKind {
    CreateTemplate(TemplateId),
    RenameTemplate {
        template_id: TemplateId,
        before: String,
        after: String,
    },
    AddPlacement {
        template_id: TemplateId,
        placement: AssetPlacement,
    },
    RemovePlacement {
        template_id: TemplateId,
        placement: AssetPlacement,
        dropped_anchors: Vec<TemplateAnchor>,
    },
    MovePlacement {
        template_id: TemplateId,
        placement_id: AssetPlacementId,
        before: TemplateRect,
        after: TemplateRect,
    },
    ResizePlacement {
        template_id: TemplateId,
        placement_id: AssetPlacementId,
        before: TemplateRect,
        after: TemplateRect,
    },
    ReorderZ {
        template_id: TemplateId,
        placement_id: AssetPlacementId,
        before: i32,
        after: i32,
    },
    PlaceAnchor {
        template_id: TemplateId,
        anchor: TemplateAnchor,
    },
    RemoveAnchor {
        template_id: TemplateId,
        anchor: TemplateAnchor,
    },
    RenameAnchor {
        template_id: TemplateId,
        anchor_id: AnchorId,
        before: String,
        after: String,
    },
    MoveAnchor {
        template_id: TemplateId,
        anchor_id: AnchorId,
        before: AnchorPosition,
        after: AnchorPosition,
    },
    ToggleAnchorAttach {
        template_id: TemplateId,
        anchor_id: AnchorId,
        before: Option<AssetPlacementId>,
        after: Option<AssetPlacementId>,
    },
    AssignAnchor {
        template_id: TemplateId,
        anchor_id: AnchorId,
        before: Option<AnchorBinding>,
        after: Option<AnchorBinding>,
    },
    ImportAsset {
        asset: AssetEntry,
    },
    RemoveAsset {
        asset: AssetEntry,
        dropped_placements: Vec<(TemplateId, AssetPlacement)>,
        dropped_anchors: Vec<(TemplateId, TemplateAnchor)>,
    },
}

impl SheetsEventKind {
    pub(crate) fn all_kind_names() -> Vec<&'static str> {
        // Compiler-enforced exhaustiveness: when a new variant lands without a name entry below,
        // this match becomes non-exhaustive and the build breaks. Do NOT add a wildcard arm.
        fn _exhaustiveness(k: &SheetsEventKind) -> &'static str {
            match k {
                SheetsEventKind::CreateTemplate(_) => "CreateTemplate",
                SheetsEventKind::RenameTemplate { .. } => "RenameTemplate",
                SheetsEventKind::AddPlacement { .. } => "AddPlacement",
                SheetsEventKind::RemovePlacement { .. } => "RemovePlacement",
                SheetsEventKind::MovePlacement { .. } => "MovePlacement",
                SheetsEventKind::ResizePlacement { .. } => "ResizePlacement",
                SheetsEventKind::ReorderZ { .. } => "ReorderZ",
                SheetsEventKind::PlaceAnchor { .. } => "PlaceAnchor",
                SheetsEventKind::RemoveAnchor { .. } => "RemoveAnchor",
                SheetsEventKind::RenameAnchor { .. } => "RenameAnchor",
                SheetsEventKind::MoveAnchor { .. } => "MoveAnchor",
                SheetsEventKind::ToggleAnchorAttach { .. } => "ToggleAnchorAttach",
                SheetsEventKind::AssignAnchor { .. } => "AssignAnchor",
                SheetsEventKind::ImportAsset { .. } => "ImportAsset",
                SheetsEventKind::RemoveAsset { .. } => "RemoveAsset",
            }
        }
        vec![
            "CreateTemplate",
            "RenameTemplate",
            "AddPlacement",
            "RemovePlacement",
            "MovePlacement",
            "ResizePlacement",
            "ReorderZ",
            "PlaceAnchor",
            "RemoveAnchor",
            "RenameAnchor",
            "MoveAnchor",
            "ToggleAnchorAttach",
            "AssignAnchor",
            "ImportAsset",
            "RemoveAsset",
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SheetsEvent {
    // session-scoped, never serialized; spec line 114
    pub timestamp: Instant,
    pub kind: SheetsEventKind,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SheetsState {
    pub templates: Vec<DeviceTemplate>,
    pub assets: Vec<AssetEntry>,
    pub asset_health: Vec<inputforge_core::sheet::AssetHealth>,
    pub library_tab: SheetsLibraryTab,
    pub selected_template_id: Option<TemplateId>,
    pub selected_asset_id: Option<AssetId>,
    pub selected_anchor_id: Option<AnchorId>,
    pub tool: SheetTool,
    pub autosave: AutosaveStatus,
    pub capture: CaptureStatus,
    pub last_error: Option<String>,
    pub history: VecDeque<SheetsEvent>,
    pub preset_picker_open: bool,
    pub pending_preset_slots: Vec<TemplateRect>,
}

impl Default for SheetsState {
    fn default() -> Self {
        Self {
            templates: Vec::new(),
            assets: Vec::new(),
            asset_health: Vec::new(),
            library_tab: SheetsLibraryTab::Templates,
            selected_template_id: None,
            selected_asset_id: None,
            selected_anchor_id: None,
            tool: SheetTool::Select,
            autosave: AutosaveStatus::Clean,
            capture: CaptureStatus::Unavailable("live input is not available".to_owned()),
            last_error: None,
            history: VecDeque::new(),
            preset_picker_open: false,
            pending_preset_slots: Vec::new(),
        }
    }
}

impl SheetsState {
    pub(crate) fn from_documents(
        documents: &crate::frame::sheets::authoring::SheetsDocuments,
    ) -> Self {
        let templates = documents.templates.templates.clone();
        let selected_template_id = templates
            .first()
            .map(|template| template.template_id.clone());
        let selected_asset_id = templates
            .first()
            .and_then(|template| template.placements.first().map(|p| &p.asset_id))
            .cloned();

        Self {
            templates,
            assets: documents.assets.assets.clone(),
            asset_health: documents.asset_health.clone(),
            library_tab: SheetsLibraryTab::Templates,
            selected_template_id,
            selected_asset_id,
            selected_anchor_id: None,
            tool: SheetTool::Select,
            autosave: AutosaveStatus::Clean,
            capture: CaptureStatus::Unavailable("live input is not available".to_owned()),
            last_error: None,
            history: VecDeque::new(),
            preset_picker_open: false,
            pending_preset_slots: Vec::new(),
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
            placements: vec![AssetPlacement {
                placement_id: AssetPlacementId::new(),
                asset_id: asset_id.clone(),
                position: TemplateRect {
                    x: 0.05,
                    y: 0.05,
                    w: 0.9,
                    h: 0.9,
                },
                z_index: 0,
                extensions: ExtensionPayload::default(),
            }],
            anchors: Vec::new(),
            default_anchor_bindings: Vec::new(),
            grouping_hints: Vec::new(),
            default_token_preset: TokenPreset::Standard,
            extensions: ExtensionPayload::default(),
        });
        self.selected_template_id = Some(template_id.clone());
        self.selected_asset_id = Some(asset_id);
        self.selected_anchor_id = None;
        self.push_event(SheetsEventKind::CreateTemplate(template_id.clone()));
        self.mark_dirty();
        template_id
    }

    pub(crate) fn create_blank_template(&mut self) -> TemplateId {
        let display_name = self.next_untitled_template_name();
        let template_id = TemplateId::new();
        self.templates.push(DeviceTemplate {
            template_id: template_id.clone(),
            display_name,
            matching_hints: Vec::new(),
            placements: Vec::new(),
            anchors: Vec::new(),
            default_anchor_bindings: Vec::new(),
            grouping_hints: Vec::new(),
            default_token_preset: TokenPreset::Standard,
            extensions: ExtensionPayload::default(),
        });
        self.selected_template_id = Some(template_id.clone());
        self.selected_asset_id = None;
        self.selected_anchor_id = None;
        self.push_event(SheetsEventKind::CreateTemplate(template_id.clone()));
        self.mark_dirty();
        template_id
    }

    pub(crate) fn open_template_preset_picker(&mut self) {
        self.preset_picker_open = true;
    }

    pub(crate) fn dismiss_template_preset_picker(&mut self) {
        self.preset_picker_open = false;
    }

    pub(crate) fn apply_preset_after_create(&mut self, preset: SheetLayoutPreset) {
        self.pending_preset_slots = preset.slot_rects();
        self.preset_picker_open = false;
    }

    pub(crate) fn consume_next_preset_slot(&mut self) -> Option<TemplateRect> {
        if self.pending_preset_slots.is_empty() {
            None
        } else {
            Some(self.pending_preset_slots.remove(0))
        }
    }

    pub(crate) fn rename_selected_template(&mut self, display_name: impl AsRef<str>) {
        let display_name = display_name.as_ref().trim();
        if display_name.is_empty() {
            return;
        }

        let Some(template) = self.selected_template_mut() else {
            return;
        };
        if template.display_name == display_name {
            return;
        }

        let after = display_name.to_owned();
        let before = std::mem::replace(&mut template.display_name, after.clone());
        let template_id = template.template_id.clone();
        self.push_event(SheetsEventKind::RenameTemplate {
            template_id,
            before,
            after,
        });
        self.mark_dirty();
    }

    pub(crate) fn place_anchor(&mut self, position: AnchorPosition) -> AnchorId {
        let anchor_id = AnchorId::new();
        let Some(template) = self.selected_template_mut() else {
            return anchor_id;
        };

        let label = format!("Anchor {}", template.anchors.len() + 1);
        let anchor = TemplateAnchor {
            anchor_id: anchor_id.clone(),
            label,
            position,
            attached_to: None,
            input_type_hint: None,
            grouping_hint: None,
            device_matching_hint: None,
            extensions: ExtensionPayload::default(),
        };
        template.anchors.push(anchor.clone());
        let template_id = template.template_id.clone();
        self.selected_anchor_id = Some(anchor_id.clone());
        self.push_event(SheetsEventKind::PlaceAnchor {
            template_id,
            anchor,
        });
        self.mark_dirty();
        anchor_id
    }

    pub(crate) fn add_placement(
        &mut self,
        template_id: TemplateId,
        asset_id: AssetId,
        position: TemplateRect,
    ) -> Result<AssetPlacementId, String> {
        let template = self
            .templates
            .iter_mut()
            .find(|template| template.template_id == template_id)
            .ok_or_else(|| format!("template {} not found", template_id.as_str()))?;
        let auto_z = template
            .placements
            .iter()
            .map(|p| p.z_index)
            .max()
            .unwrap_or(-1)
            + 1;
        let placement_id = AssetPlacementId::new();
        let placement = AssetPlacement {
            placement_id: placement_id.clone(),
            asset_id,
            position,
            z_index: auto_z,
            extensions: ExtensionPayload::default(),
        };
        template.placements.push(placement.clone());
        self.push_event(SheetsEventKind::AddPlacement {
            template_id,
            placement,
        });
        self.mark_dirty();
        Ok(placement_id)
    }

    pub(crate) fn remove_placement(
        &mut self,
        template_id: TemplateId,
        placement_id: AssetPlacementId,
    ) -> Result<(), String> {
        let template = self
            .templates
            .iter_mut()
            .find(|template| template.template_id == template_id)
            .ok_or_else(|| format!("template {} not found", template_id.as_str()))?;
        let index = template
            .placements
            .iter()
            .position(|p| p.placement_id == placement_id)
            .ok_or_else(|| format!("placement {} not found", placement_id.as_str()))?;
        let placement = template.placements.remove(index);
        let mut dropped_anchors = Vec::new();
        template.anchors.retain(|anchor| {
            if anchor.attached_to.as_ref() == Some(&placement_id) {
                dropped_anchors.push(anchor.clone());
                false
            } else {
                true
            }
        });
        self.push_event(SheetsEventKind::RemovePlacement {
            template_id,
            placement,
            dropped_anchors,
        });
        self.mark_dirty();
        Ok(())
    }

    pub(crate) fn move_placement(
        &mut self,
        template_id: TemplateId,
        placement_id: AssetPlacementId,
        x: f32,
        y: f32,
    ) -> Result<(), String> {
        let template = self
            .templates
            .iter_mut()
            .find(|template| template.template_id == template_id)
            .ok_or_else(|| format!("template {} not found", template_id.as_str()))?;
        let placement = template
            .placements
            .iter_mut()
            .find(|p| p.placement_id == placement_id)
            .ok_or_else(|| format!("placement {} not found", placement_id.as_str()))?;
        let before = placement.position;
        let after = TemplateRect {
            x,
            y,
            w: before.w,
            h: before.h,
        };
        placement.position = after;
        self.push_event(SheetsEventKind::MovePlacement {
            template_id,
            placement_id,
            before,
            after,
        });
        self.mark_dirty();
        Ok(())
    }

    pub(crate) fn resize_placement(
        &mut self,
        template_id: TemplateId,
        placement_id: AssetPlacementId,
        after: TemplateRect,
    ) -> Result<(), String> {
        let template = self
            .templates
            .iter_mut()
            .find(|template| template.template_id == template_id)
            .ok_or_else(|| format!("template {} not found", template_id.as_str()))?;
        let placement = template
            .placements
            .iter_mut()
            .find(|p| p.placement_id == placement_id)
            .ok_or_else(|| format!("placement {} not found", placement_id.as_str()))?;
        let before = placement.position;
        placement.position = after;
        self.push_event(SheetsEventKind::ResizePlacement {
            template_id,
            placement_id,
            before,
            after,
        });
        self.mark_dirty();
        Ok(())
    }

    pub(crate) fn set_z_index(
        &mut self,
        template_id: TemplateId,
        placement_id: AssetPlacementId,
        z: i32,
    ) -> Result<(), String> {
        let template = self
            .templates
            .iter_mut()
            .find(|template| template.template_id == template_id)
            .ok_or_else(|| format!("template {} not found", template_id.as_str()))?;
        let placement = template
            .placements
            .iter_mut()
            .find(|p| p.placement_id == placement_id)
            .ok_or_else(|| format!("placement {} not found", placement_id.as_str()))?;
        let before = placement.z_index;
        if before == z {
            return Ok(());
        }
        placement.z_index = z;
        self.push_event(SheetsEventKind::ReorderZ {
            template_id,
            placement_id,
            before,
            after: z,
        });
        self.mark_dirty();
        Ok(())
    }

    pub(crate) fn bring_to_front(
        &mut self,
        template_id: TemplateId,
        placement_id: AssetPlacementId,
    ) -> Result<(), String> {
        let Some(template) = self
            .templates
            .iter()
            .find(|template| template.template_id == template_id)
        else {
            return Ok(());
        };
        let Some(sibling_max) = template
            .placements
            .iter()
            .filter(|p| p.placement_id != placement_id)
            .map(|p| p.z_index)
            .max()
        else {
            return Ok(());
        };
        let current_z = template
            .placements
            .iter()
            .find(|p| p.placement_id == placement_id)
            .map(|p| p.z_index);
        if current_z.is_some_and(|z| z >= sibling_max) {
            return Ok(());
        }
        self.set_z_index(template_id, placement_id, sibling_max.saturating_add(1))
    }

    pub(crate) fn send_to_back(
        &mut self,
        template_id: TemplateId,
        placement_id: AssetPlacementId,
    ) -> Result<(), String> {
        let Some(template) = self
            .templates
            .iter()
            .find(|template| template.template_id == template_id)
        else {
            return Ok(());
        };
        let Some(sibling_min) = template
            .placements
            .iter()
            .filter(|p| p.placement_id != placement_id)
            .map(|p| p.z_index)
            .min()
        else {
            return Ok(());
        };
        let current_z = template
            .placements
            .iter()
            .find(|p| p.placement_id == placement_id)
            .map(|p| p.z_index);
        if current_z.is_some_and(|z| z <= sibling_min) {
            return Ok(());
        }
        self.set_z_index(template_id, placement_id, sibling_min.saturating_sub(1))
    }

    pub(crate) fn selected_template(&self) -> Option<&DeviceTemplate> {
        let selected_template_id = self.selected_template_id.as_ref()?;
        self.templates
            .iter()
            .find(|template| &template.template_id == selected_template_id)
    }

    pub(crate) fn select_template(&mut self, template_id: TemplateId) {
        if let Some(template) = self
            .templates
            .iter()
            .find(|template| template.template_id == template_id)
        {
            self.selected_template_id = Some(template_id);
            self.selected_asset_id = template.placements.first().map(|p| p.asset_id.clone());
            self.selected_anchor_id = None;
        }
    }

    pub(crate) fn select_first_template_for_asset(&mut self, asset_id: AssetId) {
        if let Some(template_id) = self
            .templates
            .iter()
            .find(|template| template.placements.iter().any(|p| p.asset_id == asset_id))
            .map(|template| template.template_id.clone())
        {
            self.selected_template_id = Some(template_id);
            self.selected_asset_id = Some(asset_id);
            self.selected_anchor_id = None;
        }
    }

    pub(crate) fn selected_asset_id(&self) -> Option<&AssetId> {
        let template = self.selected_template()?;
        if let Some(asset_id) = self
            .selected_asset_id
            .as_ref()
            .filter(|asset_id| template.placements.iter().any(|p| &p.asset_id == *asset_id))
        {
            return Some(asset_id);
        }

        template.placements.first().map(|p| &p.asset_id)
    }

    pub(crate) fn selected_asset(&self) -> Option<&AssetEntry> {
        let asset_id = self.selected_asset_id()?;
        self.assets.iter().find(|asset| &asset.asset_id == asset_id)
    }

    pub(crate) fn selected_asset_health(&self) -> Option<&inputforge_core::sheet::AssetHealth> {
        let asset_id = self.selected_asset_id()?;
        self.asset_health
            .iter()
            .find(|health| &health.entry.asset_id == asset_id)
    }

    pub(crate) fn selected_asset_missing(&self) -> bool {
        let Some(_asset_id) = self.selected_asset_id() else {
            return false;
        };

        self.selected_asset().is_none()
            || self
                .selected_asset_health()
                .map_or(true, |health| health.missing)
    }

    pub(crate) fn arm_capture(&mut self, anchor_id: AnchorId) {
        self.capture = CaptureStatus::Armed(anchor_id);
    }

    pub(crate) fn update_selected_anchor_label(&mut self, label: impl Into<String>) {
        let after = label.into();
        let Some(anchor) = self.selected_anchor_mut() else {
            return;
        };
        let anchor_id = anchor.anchor_id.clone();
        let before = std::mem::replace(&mut anchor.label, after.clone());
        let template_id = self
            .selected_template_id
            .clone()
            .expect("selected_anchor_mut implies selected template");
        self.push_event(SheetsEventKind::RenameAnchor {
            template_id,
            anchor_id,
            before,
            after,
        });
        self.mark_dirty();
    }

    pub(crate) fn update_selected_anchor_position(&mut self, position: AnchorPosition) {
        let Some(anchor) = self.selected_anchor_mut() else {
            return;
        };
        let anchor_id = anchor.anchor_id.clone();
        let before = anchor.position;
        anchor.position = position;
        let template_id = self
            .selected_template_id
            .clone()
            .expect("selected_anchor_mut implies selected template");
        self.push_event(SheetsEventKind::MoveAnchor {
            template_id,
            anchor_id,
            before,
            after: position,
        });
        self.mark_dirty();
    }

    pub(crate) fn set_anchor_attached_to(
        &mut self,
        attached_to: Option<AssetPlacementId>,
    ) -> Result<(), String> {
        let template_id = self
            .selected_template_id
            .clone()
            .ok_or_else(|| "no template selected".to_owned())?;
        let anchor_id = self
            .selected_anchor_id
            .clone()
            .ok_or_else(|| "no anchor selected".to_owned())?;
        let anchor = self
            .selected_anchor_mut()
            .ok_or_else(|| "selected anchor missing".to_owned())?;
        let before = anchor.attached_to.clone();
        anchor.attached_to = attached_to.clone();

        self.push_event(SheetsEventKind::ToggleAnchorAttach {
            template_id,
            anchor_id,
            before,
            after: attached_to,
        });
        self.mark_dirty();
        Ok(())
    }

    pub(crate) fn remove_selected_anchor(&mut self) -> Result<(), String> {
        let template_id = self
            .selected_template_id
            .clone()
            .ok_or_else(|| "no template selected".to_owned())?;
        let anchor_id = self
            .selected_anchor_id
            .clone()
            .ok_or_else(|| "no anchor selected".to_owned())?;

        let template = self
            .selected_template_mut()
            .ok_or_else(|| "selected template missing".to_owned())?;
        let idx = template
            .anchors
            .iter()
            .position(|a| a.anchor_id == anchor_id)
            .ok_or_else(|| "anchor not found".to_owned())?;
        let anchor = template.anchors.remove(idx);
        template
            .default_anchor_bindings
            .retain(|b| b.anchor_id != anchor_id);
        self.selected_anchor_id = None;

        self.push_event(SheetsEventKind::RemoveAnchor {
            template_id,
            anchor,
        });
        self.mark_dirty();
        Ok(())
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

        let before = template
            .default_anchor_bindings
            .iter()
            .find(|binding| binding.anchor_id == anchor_id)
            .cloned();
        template
            .default_anchor_bindings
            .retain(|binding| binding.anchor_id != anchor_id);
        let binding = AnchorBinding {
            anchor_id: anchor_id.clone(),
            input,
            captured_device_fingerprint: None,
            assignment,
        };
        template.default_anchor_bindings.push(binding.clone());
        let template_id = template.template_id.clone();
        self.push_event(SheetsEventKind::AssignAnchor {
            template_id,
            anchor_id,
            before,
            after: Some(binding),
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

    pub(crate) fn record_imported_asset(&mut self, asset: AssetEntry) {
        self.push_event(SheetsEventKind::ImportAsset { asset });
    }

    pub(crate) fn remove_asset(&mut self, asset_id: AssetId) -> Result<(), String> {
        let asset_idx = self
            .assets
            .iter()
            .position(|a| a.asset_id == asset_id)
            .ok_or_else(|| format!("asset {asset_id} not found"))?;
        let asset = self.assets.remove(asset_idx);

        let mut dropped_placements = Vec::new();
        let mut dropped_anchors = Vec::new();
        for template in &mut self.templates {
            let template_id = template.template_id.clone();

            // First pass: drop placements that reference the asset, recording their ids for the
            // anchor cascade and the event payload.
            let mut local_placement_ids = Vec::new();
            template.placements.retain(|p| {
                if p.asset_id == asset_id {
                    local_placement_ids.push(p.placement_id.clone());
                    dropped_placements.push((template_id.clone(), p.clone()));
                    false
                } else {
                    true
                }
            });

            if local_placement_ids.is_empty() {
                continue;
            }

            // Second pass on the same template: drop anchors attached to any local doomed
            // placement. Compared against local_placement_ids (per-template scope), not the
            // cross-template dropped_placements vec, so anchors on other templates are never
            // affected by an accidental id collision.
            template.anchors.retain(|anchor| {
                let attached = anchor.attached_to.as_ref();
                if attached.is_some_and(|att| local_placement_ids.iter().any(|id| id == att)) {
                    dropped_anchors.push((template_id.clone(), anchor.clone()));
                    false
                } else {
                    true
                }
            });
        }

        self.push_event(SheetsEventKind::RemoveAsset {
            asset,
            dropped_placements,
            dropped_anchors,
        });
        self.mark_dirty();
        Ok(())
    }

    pub(crate) fn push_event_for_tests(&mut self, kind: SheetsEventKind) {
        self.push_event(kind);
    }

    fn push_event(&mut self, kind: SheetsEventKind) {
        self.history.push_back(SheetsEvent {
            timestamp: Instant::now(),
            kind,
        });
        if self.history.len() > SHEETS_HISTORY_CAP {
            self.history.pop_front();
        }
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

    fn selected_anchor_mut(&mut self) -> Option<&mut TemplateAnchor> {
        let selected_anchor_id = self.selected_anchor_id.clone()?;
        self.selected_template_mut()?
            .anchors
            .iter_mut()
            .find(|anchor| anchor.anchor_id == selected_anchor_id)
    }

    fn next_untitled_template_name(&self) -> String {
        let mut next_index = 1;
        loop {
            let display_name = format!("Untitled template {next_index}");
            if !self
                .templates
                .iter()
                .any(|template| template.display_name == display_name)
            {
                return display_name;
            }
            next_index += 1;
        }
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

    use inputforge_core::sheet::{AssetHealth, InputTypeHint, PixelDimensions};

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
                placements: vec![AssetPlacement {
                    placement_id: AssetPlacementId::new(),
                    asset_id: asset_id.clone(),
                    position: TemplateRect {
                        x: 0.05,
                        y: 0.05,
                        w: 0.9,
                        h: 0.9,
                    },
                    z_index: 0,
                    extensions: ExtensionPayload::default(),
                }],
                anchors: vec![TemplateAnchor {
                    anchor_id: anchor_id.clone(),
                    label: "Trigger".to_owned(),
                    position: AnchorPosition { x: 0.25, y: 0.5 },
                    attached_to: None,
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
                asset_id: asset_id.clone(),
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
            library_tab: SheetsLibraryTab::Templates,
            selected_template_id: Some(template_id),
            selected_asset_id: Some(asset_id),
            selected_anchor_id: Some(anchor_id),
            tool: SheetTool::Select,
            autosave: AutosaveStatus::Clean,
            capture: CaptureStatus::Unavailable("live input is not available".to_owned()),
            last_error: None,
            history: VecDeque::new(),
            preset_picker_open: false,
            pending_preset_slots: Vec::new(),
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
    fn selected_template_asset_without_health_is_missing() {
        let state = state_with_template();

        assert!(state.selected_asset_missing());
    }

    #[test]
    fn selecting_template_clears_anchor_without_marking_dirty() {
        let mut state = state_with_template();
        let template_id = state.templates[0].template_id.clone();
        let asset_id = state.templates[0].placements[0].asset_id.clone();

        state.select_template(template_id.clone());

        assert_eq!(state.selected_template_id, Some(template_id));
        assert_eq!(state.selected_asset_id, Some(asset_id));
        assert_eq!(state.selected_anchor_id, None);
        assert_eq!(state.autosave, AutosaveStatus::Clean);
    }

    #[test]
    fn selecting_multi_asset_template_selects_first_asset_without_marking_dirty() {
        let mut state = state_with_template();
        let template_id = state.templates[0].template_id.clone();
        let first_asset_id = state.templates[0].placements[0].asset_id.clone();
        state.templates[0].placements.push(AssetPlacement {
            placement_id: AssetPlacementId::new(),
            asset_id: AssetId::from_string("asset-2"),
            position: TemplateRect {
                x: 0.05,
                y: 0.05,
                w: 0.9,
                h: 0.9,
            },
            z_index: 1,
            extensions: ExtensionPayload::default(),
        });
        state.selected_asset_id = Some(AssetId::from_string("asset-2"));

        state.select_template(template_id.clone());

        assert_eq!(state.selected_template_id, Some(template_id));
        assert_eq!(state.selected_asset_id, Some(first_asset_id));
        assert_eq!(state.selected_anchor_id, None);
        assert_eq!(state.autosave, AutosaveStatus::Clean);
    }

    #[test]
    fn selecting_by_asset_uses_first_template_without_marking_dirty() {
        let mut state = state_with_template();
        let asset_id = state.assets[0].asset_id.clone();
        let first_template_id = state.templates[0].template_id.clone();
        let second_template_id = TemplateId::from_string("template-2");
        state.templates.push(DeviceTemplate {
            template_id: second_template_id.clone(),
            display_name: "Second template".to_owned(),
            matching_hints: Vec::new(),
            placements: vec![AssetPlacement {
                placement_id: AssetPlacementId::new(),
                asset_id: asset_id.clone(),
                position: TemplateRect {
                    x: 0.05,
                    y: 0.05,
                    w: 0.9,
                    h: 0.9,
                },
                z_index: 0,
                extensions: ExtensionPayload::default(),
            }],
            anchors: Vec::new(),
            default_anchor_bindings: Vec::new(),
            grouping_hints: Vec::new(),
            default_token_preset: TokenPreset::Standard,
            extensions: ExtensionPayload::default(),
        });
        state.selected_template_id = Some(second_template_id);

        state.select_first_template_for_asset(asset_id.clone());

        assert_eq!(state.selected_template_id, Some(first_template_id));
        assert_eq!(state.selected_asset_id, Some(asset_id));
        assert_eq!(state.selected_anchor_id, None);
        assert_eq!(state.autosave, AutosaveStatus::Clean);
    }

    #[test]
    fn selecting_non_first_asset_makes_it_active_asset_without_marking_dirty() {
        let mut state = state_with_template();
        let first_asset = state.assets[0].clone();
        let second_asset_id = AssetId::from_string("asset-2");
        let second_asset = AssetEntry {
            asset_id: second_asset_id.clone(),
            copied_path: PathBuf::from("assets/asset-2.png"),
            content_hash: "hash-2".to_owned(),
            media_type: "image/png".to_owned(),
            pixel_dimensions: PixelDimensions {
                width: 128,
                height: 64,
            },
            original_import_path: None,
            extensions: ExtensionPayload::default(),
        };
        state.templates[0].placements.push(AssetPlacement {
            placement_id: AssetPlacementId::new(),
            asset_id: second_asset_id.clone(),
            position: TemplateRect {
                x: 0.05,
                y: 0.05,
                w: 0.9,
                h: 0.9,
            },
            z_index: 1,
            extensions: ExtensionPayload::default(),
        });
        state.assets.push(second_asset.clone());
        state.asset_health = vec![
            AssetHealth {
                entry: first_asset,
                copied_absolute_path: PathBuf::from("assets/asset-1.png"),
                missing: false,
            },
            AssetHealth {
                entry: second_asset.clone(),
                copied_absolute_path: PathBuf::from("assets/asset-2.png"),
                missing: false,
            },
        ];

        state.select_first_template_for_asset(second_asset_id.clone());

        assert_eq!(state.selected_asset_id(), Some(&second_asset_id));
        assert_eq!(state.selected_asset(), Some(&second_asset));
        assert_eq!(
            state
                .selected_asset_health()
                .map(|health| &health.entry.asset_id),
            Some(&second_asset_id)
        );
        assert_eq!(state.selected_anchor_id, None);
        assert_eq!(state.autosave, AutosaveStatus::Clean);
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
    fn save_failure_keeps_dirty_draft_and_retry_error() {
        let mut state = state_with_template();
        state.update_selected_anchor_label("Fire");

        let mut failed_documents = crate::frame::sheets::authoring::SheetsDocuments {
            templates: inputforge_core::sheet::TemplateStoreDocument::default(),
            assets: inputforge_core::sheet::AssetManifestDocument::default(),
            asset_health: Vec::new(),
        };
        state.apply_to_documents(&mut failed_documents);
        state.mark_saving();
        state.mark_failed("asset manifest write failed");

        assert_eq!(state.autosave, AutosaveStatus::Failed);
        assert_eq!(
            state.last_error,
            Some("asset manifest write failed".to_owned())
        );
        assert_eq!(state.templates[0].anchors[0].label, "Fire");
        assert_eq!(failed_documents.templates.templates, state.templates);
        assert_eq!(failed_documents.assets.assets, state.assets);

        let mut retry_documents = crate::frame::sheets::authoring::SheetsDocuments {
            templates: inputforge_core::sheet::TemplateStoreDocument::default(),
            assets: inputforge_core::sheet::AssetManifestDocument::default(),
            asset_health: Vec::new(),
        };
        state.apply_to_documents(&mut retry_documents);
        state.mark_saving();
        state.mark_failed("template store write failed");

        assert_eq!(state.autosave, AutosaveStatus::Failed);
        assert_eq!(
            state.last_error,
            Some("template store write failed".to_owned())
        );
        assert_eq!(state.templates[0].anchors[0].label, "Fire");
        assert_eq!(retry_documents.templates.templates, state.templates);
        assert_eq!(retry_documents.assets.assets, state.assets);
    }

    #[test]
    fn classify_save_outcome_maps_all_four_arms() {
        use crate::frame::sheets::authoring::classify_save_outcome;
        use inputforge_core::error::EngineError;

        fn asset_err() -> EngineError {
            EngineError::InvalidConfig {
                reason: "asset failed".to_owned(),
            }
        }
        fn template_err() -> EngineError {
            EngineError::InvalidConfig {
                reason: "template failed".to_owned(),
            }
        }

        assert!(classify_save_outcome(Ok(()), Ok(())).is_ok());

        let err = classify_save_outcome(Err(asset_err()), Ok(())).unwrap_err();
        assert!(err.to_string().contains("asset failed"));

        let err = classify_save_outcome(Ok(()), Err(template_err())).unwrap_err();
        assert!(err.to_string().contains("template failed"));

        let err = classify_save_outcome(Err(asset_err()), Err(template_err())).unwrap_err();
        assert!(
            err.to_string().contains("asset failed"),
            "both-failed arm reports the asset error first because the manifest write runs first"
        );
    }

    #[test]
    fn history_starts_empty_and_caps_at_200_entries() {
        let mut state = SheetsState::default();
        assert!(state.history.is_empty());

        for _ in 0..205 {
            state.push_event_for_tests(SheetsEventKind::CreateTemplate(TemplateId::from_string(
                "t",
            )));
        }
        assert_eq!(state.history.len(), 200);
    }

    #[test]
    fn existing_mutators_each_push_one_history_event() {
        let mut state = state_with_template();
        let baseline = state.history.len();

        state.rename_selected_template("Renamed");
        state.update_selected_anchor_label("Fire");
        state.update_selected_anchor_position(AnchorPosition { x: 0.4, y: 0.6 });
        state.assign_selected_anchor(button_input(1), AnchorAssignment::Captured);

        assert_eq!(state.history.len() - baseline, 4);
        assert!(matches!(
            state.history[baseline].kind,
            SheetsEventKind::RenameTemplate { .. }
        ));
        assert!(matches!(
            state.history[baseline + 1].kind,
            SheetsEventKind::RenameAnchor { .. }
        ));
        assert!(matches!(
            state.history[baseline + 2].kind,
            SheetsEventKind::MoveAnchor { .. }
        ));
        assert!(matches!(
            state.history[baseline + 3].kind,
            SheetsEventKind::AssignAnchor { .. }
        ));
    }

    #[test]
    fn create_blank_template_pushes_create_template_event() {
        let mut state = SheetsState::default();
        let template_id = state.create_blank_template();
        assert_eq!(state.history.len(), 1);
        assert!(matches!(
            &state.history[0].kind,
            SheetsEventKind::CreateTemplate(id) if id == &template_id
        ));
    }

    #[test]
    fn place_anchor_pushes_place_anchor_event() {
        let mut state = state_with_template();
        let baseline = state.history.len();
        let anchor_id = state.place_anchor(AnchorPosition { x: 0.1, y: 0.2 });
        assert_eq!(state.history.len() - baseline, 1);
        assert!(matches!(
            &state.history[baseline].kind,
            SheetsEventKind::PlaceAnchor { anchor, .. } if anchor.anchor_id == anchor_id
        ));
    }

    #[test]
    fn add_placement_appends_to_template_and_pushes_event() {
        let mut state = state_with_template();
        let template_id = state.selected_template_id.clone().unwrap();
        let asset_id = state.templates[0].placements.first().map_or_else(
            || AssetId::from_string("asset-extra"),
            |p| p.asset_id.clone(),
        );
        let baseline = state.history.len();

        let placement_id = state
            .add_placement(
                template_id.clone(),
                asset_id.clone(),
                inputforge_core::sheet::TemplateRect {
                    x: 0.1,
                    y: 0.1,
                    w: 0.3,
                    h: 0.3,
                },
            )
            .unwrap();

        let placements = &state.templates[0].placements;
        assert!(placements.iter().any(|p| p.placement_id == placement_id));
        assert_eq!(state.history.len() - baseline, 1);
        assert!(matches!(
            &state.history[baseline].kind,
            SheetsEventKind::AddPlacement { placement, .. } if placement.placement_id == placement_id
        ));
    }

    #[test]
    fn remove_placement_cascades_attached_anchors_and_records_them_in_event() {
        use inputforge_core::sheet::{
            AssetPlacement, AssetPlacementId, TemplateAnchor, TemplateRect,
        };
        let mut state = state_with_template();
        let template_id = state.selected_template_id.clone().unwrap();
        let placement_id = AssetPlacementId::from_string("placement-cascade");
        state.templates[0].placements.push(AssetPlacement {
            placement_id: placement_id.clone(),
            asset_id: AssetId::from_string("asset-1"),
            position: TemplateRect {
                x: 0.0,
                y: 0.0,
                w: 1.0,
                h: 1.0,
            },
            z_index: 0,
            extensions: ExtensionPayload::default(),
        });
        state.templates[0].anchors.push(TemplateAnchor {
            anchor_id: AnchorId::from_string("anchor-attached"),
            label: "Attached".to_owned(),
            position: AnchorPosition { x: 0.5, y: 0.5 },
            attached_to: Some(placement_id.clone()),
            input_type_hint: None,
            grouping_hint: None,
            device_matching_hint: None,
            extensions: ExtensionPayload::default(),
        });
        state.templates[0].anchors.push(TemplateAnchor {
            anchor_id: AnchorId::from_string("anchor-floating"),
            label: "Floating".to_owned(),
            position: AnchorPosition { x: 0.5, y: 0.5 },
            attached_to: None,
            input_type_hint: None,
            grouping_hint: None,
            device_matching_hint: None,
            extensions: ExtensionPayload::default(),
        });

        state
            .remove_placement(template_id, placement_id.clone())
            .unwrap();

        let template = &state.templates[0];
        assert!(
            template
                .placements
                .iter()
                .all(|p| p.placement_id != placement_id)
        );
        assert!(
            template
                .anchors
                .iter()
                .all(|a| a.anchor_id.as_str() != "anchor-attached")
        );
        assert!(
            template
                .anchors
                .iter()
                .any(|a| a.anchor_id.as_str() == "anchor-floating")
        );

        let event = state
            .history
            .back()
            .expect("history should record the removal");
        match &event.kind {
            SheetsEventKind::RemovePlacement {
                placement,
                dropped_anchors,
                ..
            } => {
                assert_eq!(placement.placement_id, placement_id);
                assert_eq!(dropped_anchors.len(), 1);
                assert_eq!(dropped_anchors[0].anchor_id.as_str(), "anchor-attached");
            }
            other => panic!("expected RemovePlacement, got {other:?}"),
        }
    }

    #[test]
    fn move_and_resize_placement_emit_distinct_events() {
        use inputforge_core::sheet::{AssetPlacement, AssetPlacementId, TemplateRect};
        let mut state = state_with_template();
        let template_id = state.selected_template_id.clone().unwrap();
        let placement_id = AssetPlacementId::from_string("placement-move");
        state.templates[0].placements.push(AssetPlacement {
            placement_id: placement_id.clone(),
            asset_id: AssetId::from_string("asset-1"),
            position: TemplateRect {
                x: 0.0,
                y: 0.0,
                w: 0.5,
                h: 0.5,
            },
            z_index: 0,
            extensions: ExtensionPayload::default(),
        });

        state
            .move_placement(template_id.clone(), placement_id.clone(), 0.2, 0.2)
            .unwrap();
        state
            .resize_placement(
                template_id,
                placement_id,
                TemplateRect {
                    x: 0.2,
                    y: 0.2,
                    w: 0.7,
                    h: 0.7,
                },
            )
            .unwrap();

        let kinds: Vec<_> = state
            .history
            .iter()
            .rev()
            .take(2)
            .map(|e| &e.kind)
            .collect();
        assert!(matches!(kinds[0], SheetsEventKind::ResizePlacement { .. }));
        assert!(matches!(kinds[1], SheetsEventKind::MovePlacement { .. }));
    }

    #[test]
    fn bring_to_front_and_send_to_back_set_extreme_z_values_and_push_reorder_z() {
        use inputforge_core::sheet::{AssetPlacement, AssetPlacementId, TemplateRect};
        let mut state = state_with_template();
        let template_id = state.selected_template_id.clone().unwrap();
        let a = AssetPlacementId::from_string("a");
        let b = AssetPlacementId::from_string("b");
        for (id, z) in [(a.clone(), 0), (b.clone(), 5)] {
            state.templates[0].placements.push(AssetPlacement {
                placement_id: id,
                asset_id: AssetId::from_string("asset-1"),
                position: TemplateRect {
                    x: 0.0,
                    y: 0.0,
                    w: 0.5,
                    h: 0.5,
                },
                z_index: z,
                extensions: ExtensionPayload::default(),
            });
        }

        state
            .bring_to_front(template_id.clone(), a.clone())
            .unwrap();
        let z_a = state.templates[0]
            .placements
            .iter()
            .find(|p| p.placement_id == a)
            .unwrap()
            .z_index;
        assert!(z_a > 5);

        state.send_to_back(template_id, b.clone()).unwrap();
        let z_b = state.templates[0]
            .placements
            .iter()
            .find(|p| p.placement_id == b)
            .unwrap()
            .z_index;
        assert!(z_b < z_a);
        assert!(matches!(
            state.history.back().unwrap().kind,
            SheetsEventKind::ReorderZ { .. }
        ));
    }

    #[test]
    fn bring_to_front_on_lone_placement_is_a_silent_noop() {
        use inputforge_core::sheet::{AssetPlacement, AssetPlacementId, TemplateRect};
        let mut state = state_with_template();
        let template_id = state.selected_template_id.clone().unwrap();
        let only = AssetPlacementId::from_string("only");
        state.templates[0].placements.push(AssetPlacement {
            placement_id: only.clone(),
            asset_id: AssetId::from_string("asset-1"),
            position: TemplateRect {
                x: 0.0,
                y: 0.0,
                w: 0.5,
                h: 0.5,
            },
            z_index: 0,
            extensions: ExtensionPayload::default(),
        });
        let baseline = state.history.len();
        state
            .bring_to_front(template_id.clone(), only.clone())
            .unwrap();
        state.send_to_back(template_id, only).unwrap();
        assert_eq!(state.history.len(), baseline);
    }

    #[test]
    fn toggle_anchor_attach_changes_target_and_records_event() {
        use inputforge_core::sheet::AssetPlacementId;
        let mut state = state_with_template();
        let anchor_id = state.selected_anchor_id.clone().unwrap();
        let target = AssetPlacementId::from_string("placement-attach");

        state.set_anchor_attached_to(Some(target.clone())).unwrap();
        let anchor = &state.templates[0].anchors[0];
        assert_eq!(anchor.attached_to, Some(target.clone()));
        let last = state.history.back().unwrap();
        match &last.kind {
            SheetsEventKind::ToggleAnchorAttach {
                anchor_id: id,
                before: None,
                after: Some(placement),
                ..
            } => {
                assert_eq!(id, &anchor_id);
                assert_eq!(placement, &target);
            }
            other => panic!("expected ToggleAnchorAttach, got {other:?}"),
        }

        state.set_anchor_attached_to(None).unwrap();
        assert!(matches!(
            state.history.back().unwrap().kind,
            SheetsEventKind::ToggleAnchorAttach {
                before: Some(_),
                after: None,
                ..
            }
        ));
    }

    #[test]
    fn remove_selected_anchor_records_remove_anchor_event_and_clears_selection() {
        let mut state = state_with_template();
        let anchor_id = state.selected_anchor_id.clone().unwrap();
        state.remove_selected_anchor().unwrap();
        assert!(
            state.templates[0]
                .anchors
                .iter()
                .all(|a| a.anchor_id != anchor_id)
        );
        assert_eq!(state.selected_anchor_id, None);
        assert!(matches!(
            state.history.back().unwrap().kind,
            SheetsEventKind::RemoveAnchor { .. }
        ));
    }

    #[test]
    fn record_imported_asset_pushes_import_asset_event() {
        use inputforge_core::sheet::{AssetEntry, PixelDimensions};
        use std::path::PathBuf;
        let mut state = SheetsState::default();
        let asset = AssetEntry {
            asset_id: AssetId::from_string("asset-1"),
            copied_path: PathBuf::from("assets/asset-1.png"),
            content_hash: "hash".to_owned(),
            media_type: "image/png".to_owned(),
            pixel_dimensions: PixelDimensions {
                width: 64,
                height: 32,
            },
            original_import_path: None,
            extensions: ExtensionPayload::default(),
        };

        state.record_imported_asset(asset.clone());
        assert!(matches!(
            state.history.back().unwrap().kind,
            SheetsEventKind::ImportAsset { asset: ref a } if a.asset_id == asset.asset_id
        ));
    }

    #[test]
    fn remove_asset_cascades_placements_and_attached_anchors_into_event() {
        use inputforge_core::sheet::{AssetPlacement, AssetPlacementId, TemplateRect};
        let mut state = state_with_template();
        let asset_id = AssetId::from_string("asset-cascade");
        let placement_id = AssetPlacementId::from_string("p-a");
        let placement_a = AssetPlacement {
            placement_id: placement_id.clone(),
            asset_id: asset_id.clone(),
            position: TemplateRect {
                x: 0.0,
                y: 0.0,
                w: 0.5,
                h: 0.5,
            },
            z_index: 0,
            extensions: ExtensionPayload::default(),
        };
        state.templates[0].placements.push(placement_a.clone());
        let template_id = state.templates[0].template_id.clone();

        // Two anchors on the same template: one attached to the doomed placement, one floating.
        state.templates[0].anchors.push(TemplateAnchor {
            anchor_id: AnchorId::from_string("anchor-attached"),
            label: "Attached".to_owned(),
            position: AnchorPosition { x: 0.5, y: 0.5 },
            attached_to: Some(placement_id.clone()),
            input_type_hint: None,
            grouping_hint: None,
            device_matching_hint: None,
            extensions: ExtensionPayload::default(),
        });
        state.templates[0].anchors.push(TemplateAnchor {
            anchor_id: AnchorId::from_string("anchor-floating"),
            label: "Floating".to_owned(),
            position: AnchorPosition { x: 0.5, y: 0.5 },
            attached_to: None,
            input_type_hint: None,
            grouping_hint: None,
            device_matching_hint: None,
            extensions: ExtensionPayload::default(),
        });

        state.assets.push(inputforge_core::sheet::AssetEntry {
            asset_id: asset_id.clone(),
            copied_path: std::path::PathBuf::from("assets/cascade.png"),
            content_hash: "hash".to_owned(),
            media_type: "image/png".to_owned(),
            pixel_dimensions: inputforge_core::sheet::PixelDimensions {
                width: 1,
                height: 1,
            },
            original_import_path: None,
            extensions: ExtensionPayload::default(),
        });

        state.remove_asset(asset_id.clone()).unwrap();
        assert!(state.assets.iter().all(|a| a.asset_id != asset_id));
        assert!(
            state.templates[0]
                .anchors
                .iter()
                .all(|a| a.anchor_id.as_str() != "anchor-attached")
        );
        assert!(
            state.templates[0]
                .anchors
                .iter()
                .any(|a| a.anchor_id.as_str() == "anchor-floating")
        );

        let event = state.history.back().unwrap();
        match &event.kind {
            SheetsEventKind::RemoveAsset {
                asset,
                dropped_placements,
                dropped_anchors,
            } => {
                assert_eq!(asset.asset_id, asset_id);
                assert_eq!(dropped_placements.len(), 1);
                assert_eq!(dropped_placements[0].0, template_id);
                assert_eq!(
                    dropped_placements[0].1.placement_id,
                    placement_a.placement_id
                );
                assert_eq!(dropped_anchors.len(), 1);
                assert_eq!(dropped_anchors[0].0, template_id);
                assert_eq!(dropped_anchors[0].1.anchor_id.as_str(), "anchor-attached");
            }
            other => panic!("expected RemoveAsset, got {other:?}"),
        }
    }

    #[test]
    fn every_event_kind_has_a_documented_inverse() {
        let inverses: &[(&str, &str)] = &[
            ("CreateTemplate", "RemovePlacement"),
            ("RenameTemplate", "RenameTemplate"),
            ("AddPlacement", "RemovePlacement"),
            ("RemovePlacement", "AddPlacement"),
            ("MovePlacement", "MovePlacement"),
            ("ResizePlacement", "ResizePlacement"),
            ("ReorderZ", "ReorderZ"),
            ("PlaceAnchor", "RemoveAnchor"),
            ("RemoveAnchor", "PlaceAnchor"),
            ("RenameAnchor", "RenameAnchor"),
            ("MoveAnchor", "MoveAnchor"),
            ("ToggleAnchorAttach", "ToggleAnchorAttach"),
            ("AssignAnchor", "AssignAnchor"),
            ("ImportAsset", "RemoveAsset"),
            ("RemoveAsset", "ImportAsset"),
        ];

        let names = SheetsEventKind::all_kind_names();
        for (kind, _) in inverses {
            assert!(
                names.contains(kind),
                "missing event kind {kind} in inverses table"
            );
        }
        for name in &names {
            assert!(
                inverses.iter().any(|(k, _)| k == name),
                "event kind {name} lacks a reverse pairing"
            );
        }
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

    #[test]
    fn moving_a_placement_leaves_attached_anchor_positions_unchanged() {
        use inputforge_core::sheet::{AssetPlacement, AssetPlacementId, TemplateAnchor};
        let mut state = state_with_template();
        let template_id = state.selected_template_id.clone().unwrap();
        let placement_id = AssetPlacementId::from_string("p-1");
        state.templates[0].placements.push(AssetPlacement {
            placement_id: placement_id.clone(),
            asset_id: AssetId::from_string("asset-1"),
            position: TemplateRect {
                x: 0.1,
                y: 0.1,
                w: 0.5,
                h: 0.5,
            },
            z_index: 0,
            extensions: ExtensionPayload::default(),
        });
        state.templates[0].anchors.push(TemplateAnchor {
            anchor_id: AnchorId::from_string("anchor-att"),
            label: "Attached".to_owned(),
            position: AnchorPosition { x: 0.4, y: 0.6 },
            attached_to: Some(placement_id.clone()),
            input_type_hint: None,
            grouping_hint: None,
            device_matching_hint: None,
            extensions: ExtensionPayload::default(),
        });

        state
            .move_placement(template_id, placement_id, 0.7, 0.2)
            .unwrap();

        let anchor = state.templates[0]
            .anchors
            .iter()
            .find(|a| a.anchor_id.as_str() == "anchor-att")
            .unwrap();
        assert_eq!(anchor.position, AnchorPosition { x: 0.4, y: 0.6 });
    }

    #[test]
    fn layout_preset_slot_rectangles_match_spec_table() {
        use inputforge_core::sheet::TemplateRect;
        assert_eq!(
            SheetLayoutPreset::Single.slot_rects(),
            vec![TemplateRect {
                x: 0.05,
                y: 0.05,
                w: 0.9,
                h: 0.9
            }],
        );
        assert_eq!(
            SheetLayoutPreset::HorizontalPair.slot_rects(),
            vec![
                TemplateRect {
                    x: 0.02,
                    y: 0.10,
                    w: 0.46,
                    h: 0.80
                },
                TemplateRect {
                    x: 0.52,
                    y: 0.10,
                    w: 0.46,
                    h: 0.80
                },
            ],
        );
        assert_eq!(
            SheetLayoutPreset::VerticalStack.slot_rects(),
            vec![
                TemplateRect {
                    x: 0.10,
                    y: 0.02,
                    w: 0.80,
                    h: 0.46
                },
                TemplateRect {
                    x: 0.10,
                    y: 0.52,
                    w: 0.80,
                    h: 0.46
                },
            ],
        );
        assert_eq!(SheetLayoutPreset::TwoByTwo.slot_rects().len(), 4);
        assert!(SheetLayoutPreset::Freeform.slot_rects().is_empty());
    }

    #[test]
    fn open_and_dismiss_template_preset_picker_does_not_mutate_history() {
        let mut state = SheetsState::default();
        state.open_template_preset_picker();
        assert!(state.preset_picker_open);
        let history_before = state.history.len();
        state.dismiss_template_preset_picker();
        assert!(!state.preset_picker_open);
        assert_eq!(state.history.len(), history_before);
    }
}
