use std::collections::VecDeque;
use std::time::Instant;

use inputforge_core::sheet::{
    AnchorAssignment, AnchorBinding, AnchorId, AnchorPosition, AssetEntry, AssetId, AssetPlacement,
    AssetPlacementId, DeviceTemplate, ExtensionPayload, PixelDimensions, TemplateAnchor,
    TemplateId, TemplateRect, TokenPreset,
};
use inputforge_core::types::{DeviceId, InputAddress, InputId};

const SHEETS_HISTORY_CAP: usize = 200;

pub(crate) fn pluralize(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {plural}")
    }
}

/// Blast-radius summary for an asset, used by the Asset inspector's delete
/// confirmation modal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct AssetUsage {
    pub placements: usize,
    pub templates: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SheetTool {
    #[default]
    Select,
    Anchor,
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
pub(crate) enum CaptureAvailabilityReason {
    #[default]
    CaptureAvailable,
    EngineStopped,
    EngineRunningNoDevices,
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
    RenamePlacement {
        template_id: TemplateId,
        placement_id: AssetPlacementId,
        before: Option<String>,
        after: Option<String>,
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
    RenameAsset {
        asset_id: AssetId,
        before: Option<String>,
        after: Option<String>,
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
                SheetsEventKind::RenamePlacement { .. } => "RenamePlacement",
                SheetsEventKind::PlaceAnchor { .. } => "PlaceAnchor",
                SheetsEventKind::RemoveAnchor { .. } => "RemoveAnchor",
                SheetsEventKind::RenameAnchor { .. } => "RenameAnchor",
                SheetsEventKind::MoveAnchor { .. } => "MoveAnchor",
                SheetsEventKind::ToggleAnchorAttach { .. } => "ToggleAnchorAttach",
                SheetsEventKind::AssignAnchor { .. } => "AssignAnchor",
                SheetsEventKind::ImportAsset { .. } => "ImportAsset",
                SheetsEventKind::RemoveAsset { .. } => "RemoveAsset",
                SheetsEventKind::RenameAsset { .. } => "RenameAsset",
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
            "RenamePlacement",
            "PlaceAnchor",
            "RemoveAnchor",
            "RenameAnchor",
            "MoveAnchor",
            "ToggleAnchorAttach",
            "AssignAnchor",
            "ImportAsset",
            "RemoveAsset",
            "RenameAsset",
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
    /// Canvas-internal: which asset's image is rendered behind the placements on
    /// the active template. Set by `select_template` to the first placement's
    /// asset; the template-scoped `selected_asset_id()` method has its own
    /// first-placement fallback when this is `None`. Do NOT use this as the
    /// "user picked an asset in the rail to inspect" signal: that lives in
    /// `inspector_asset_id` so a template click does not pull the inspector
    /// into the Asset section.
    pub selected_asset_id: Option<AssetId>,
    /// User-intent: which asset the rail-click handler marked for inspection.
    /// Drives the Asset inspector branch dispatch and the Assets-tab row
    /// highlight. Cleared by `select_template` so picking a template returns
    /// the inspector to the TEMPLATE section.
    pub inspector_asset_id: Option<AssetId>,
    pub selected_anchor_id: Option<AnchorId>,
    pub selected_placement_id: Option<AssetPlacementId>,
    pub tool: SheetTool,
    pub autosave: AutosaveStatus,
    pub capture: CaptureStatus,
    pub last_error: Option<String>,
    pub history: VecDeque<SheetsEvent>,
    pub preset_picker_open: bool,
    pub pending_preset_slots: Vec<TemplateRect>,
    pub template_display_name_draft: Option<String>,
    pub anchor_label_draft: Option<String>,
    pub dragging_asset: Option<AssetId>,
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
            inspector_asset_id: None,
            selected_anchor_id: None,
            selected_placement_id: None,
            tool: SheetTool::Select,
            autosave: AutosaveStatus::Clean,
            capture: CaptureStatus::Unavailable("live input is not available".to_owned()),
            last_error: None,
            history: VecDeque::new(),
            preset_picker_open: false,
            pending_preset_slots: Vec::new(),
            template_display_name_draft: None,
            anchor_label_draft: None,
            dragging_asset: None,
        }
    }
}

/// Inverse of `crate::frame::sheets::canvas::anchor_canvas_coords`. Converts a canvas-relative
/// anchor position into the storage form expected by the rendered template: placement-local for
/// attached anchors so they keep tracking their parent through move/resize, canvas-relative for
/// floating anchors. The fallback for an orphan parent id mirrors `anchor_canvas_coords` so
/// rendered position stays consistent after the parent is removed without a touchup.
fn anchor_storage_position(
    canvas: AnchorPosition,
    attached_to: Option<&AssetPlacementId>,
    placements: &[AssetPlacement],
) -> AnchorPosition {
    let Some(parent_id) = attached_to else {
        return canvas;
    };
    let Some(parent) = placements.iter().find(|p| &p.placement_id == parent_id) else {
        return canvas;
    };
    let rect = parent.position;
    if rect.w <= 0.0 || rect.h <= 0.0 {
        return canvas;
    }
    AnchorPosition {
        x: (canvas.x - f64::from(rect.x)) / f64::from(rect.w),
        y: (canvas.y - f64::from(rect.y)) / f64::from(rect.h),
    }
}

/// Minimum normalized width / height a placement frame is allowed to have after a resize commit.
/// This keeps the frame visible and grabbable by the eight resize handles.
const MIN_PLACEMENT_DIMENSION: f32 = 0.02;

/// Clamp a placement rectangle so it stays inside the unit canvas and never falls below the
/// minimum dimensions. Width / height are clamped first (so we know how much breathing room is
/// left), then the top-left corner is pulled inside `[0.0, 1.0 - dim]`.
fn clamp_rect_to_canvas(rect: TemplateRect) -> TemplateRect {
    let w = rect.w.clamp(MIN_PLACEMENT_DIMENSION, 1.0);
    let h = rect.h.clamp(MIN_PLACEMENT_DIMENSION, 1.0);
    let x = rect.x.clamp(0.0, 1.0 - w);
    let y = rect.y.clamp(0.0, 1.0 - h);
    TemplateRect { x, y, w, h }
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
            inspector_asset_id: None,
            selected_anchor_id: None,
            selected_placement_id: None,
            tool: SheetTool::Select,
            autosave: AutosaveStatus::Clean,
            capture: CaptureStatus::Unavailable("live input is not available".to_owned()),
            last_error: None,
            history: VecDeque::new(),
            preset_picker_open: false,
            pending_preset_slots: Vec::new(),
            template_display_name_draft: None,
            anchor_label_draft: None,
            dragging_asset: None,
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
                display_name: None,
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

    /// Cancel path: close the picker, do not create a template, do not push any history event.
    pub(crate) fn dismiss_template_preset_picker(&mut self) {
        self.preset_picker_open = false;
    }

    /// Explicit "Skip" path: user wants a blank template without preset slots. Creates the
    /// template, records the `CreateTemplate` event, closes the picker.
    pub(crate) fn skip_template_preset_picker(&mut self) {
        self.create_blank_template();
        self.preset_picker_open = false;
    }

    pub(crate) fn apply_preset_after_create(&mut self, preset: SheetLayoutPreset) {
        self.create_blank_template();
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

    pub(crate) fn update_template_display_name_draft(&mut self, value: String) {
        self.template_display_name_draft = Some(value);
    }

    pub(crate) fn commit_template_display_name_draft(&mut self) {
        let Some(draft) = self.template_display_name_draft.take() else {
            return;
        };
        let trimmed = draft.trim();
        if trimmed.is_empty() {
            return;
        }
        self.rename_selected_template(trimmed);
    }

    pub(crate) fn update_anchor_label_draft(&mut self, value: String) {
        self.anchor_label_draft = Some(value);
    }

    pub(crate) fn commit_anchor_label_draft(&mut self) {
        let Some(draft) = self.anchor_label_draft.take() else {
            return;
        };
        let trimmed = draft.trim();
        if trimmed.is_empty() {
            return;
        }
        self.update_selected_anchor_label(trimmed.to_owned());
    }

    pub(crate) fn rename_selected_placement(&mut self, next: Option<String>) {
        let Some(template_id) = self.selected_template_id.clone() else {
            return;
        };
        let Some(placement_id) = self.selected_placement_id.clone() else {
            return;
        };
        let Some(template) = self.selected_template_mut() else {
            return;
        };
        let Some(placement) = template
            .placements
            .iter_mut()
            .find(|p| p.placement_id == placement_id)
        else {
            return;
        };
        if placement.display_name == next {
            return;
        }
        let before = std::mem::replace(&mut placement.display_name, next.clone());
        self.push_event(SheetsEventKind::RenamePlacement {
            template_id,
            placement_id,
            before,
            after: next,
        });
        self.mark_dirty();
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
        self.place_anchor_on(position, None)
    }

    /// Place a new anchor at a canvas-relative position. When `attached_to` is `Some` and the
    /// parent placement exists, the stored position is converted to placement-local space so the
    /// anchor follows its parent through subsequent move/resize. Floating anchors store the
    /// position verbatim.
    pub(crate) fn place_anchor_on(
        &mut self,
        position: AnchorPosition,
        attached_to: Option<AssetPlacementId>,
    ) -> AnchorId {
        let anchor_id = AnchorId::new();
        let Some(template) = self.selected_template_mut() else {
            return anchor_id;
        };

        let storage = anchor_storage_position(position, attached_to.as_ref(), &template.placements);
        let label = format!("Anchor {}", template.anchors.len() + 1);
        let anchor = TemplateAnchor {
            anchor_id: anchor_id.clone(),
            label,
            position: storage,
            attached_to,
            input_type_hint: None,
            grouping_hint: None,
            device_matching_hint: None,
            extensions: ExtensionPayload::default(),
        };
        let template_id = template.template_id.clone();
        template.anchors.push(anchor.clone());
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
            display_name: None,
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
        placement_id: &AssetPlacementId,
    ) -> Result<(), String> {
        let template = self
            .templates
            .iter_mut()
            .find(|template| template.template_id == template_id)
            .ok_or_else(|| format!("template {} not found", template_id.as_str()))?;
        let index = template
            .placements
            .iter()
            .position(|p| &p.placement_id == placement_id)
            .ok_or_else(|| format!("placement {} not found", placement_id.as_str()))?;
        let placement = template.placements.remove(index);
        let mut dropped_anchors = Vec::new();
        template.anchors.retain(|anchor| {
            if anchor.attached_to.as_ref() == Some(placement_id) {
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

    pub(crate) fn shift_z_up(
        &mut self,
        template_id: TemplateId,
        placement_id: AssetPlacementId,
    ) -> Result<(), String> {
        // Find current z + the placement directly above by z; swap z values via
        // two `set_z_index` calls. Capturing the neighbour id and z by value
        // releases the immutable borrow before the mutating calls below.
        let template = self
            .templates
            .iter()
            .find(|t| t.template_id == template_id)
            .ok_or_else(|| format!("template {} not found", template_id.as_str()))?;
        let current_z = template
            .placements
            .iter()
            .find(|p| p.placement_id == placement_id)
            .map(|p| p.z_index)
            .ok_or_else(|| format!("placement {} not found", placement_id.as_str()))?;
        let next_higher = template
            .placements
            .iter()
            .filter(|p| p.placement_id != placement_id && p.z_index > current_z)
            .min_by_key(|p| p.z_index);
        let Some(neighbour) = next_higher else {
            // Already at the top: silent no-op, no event, no mark_dirty.
            return Ok(());
        };
        let neighbour_id = neighbour.placement_id.clone();
        let neighbour_z = neighbour.z_index;
        // Swap z values: self gets neighbour's z, neighbour gets self's old z.
        self.set_z_index(template_id.clone(), placement_id, neighbour_z)?;
        self.set_z_index(template_id, neighbour_id, current_z)
    }

    pub(crate) fn shift_z_down(
        &mut self,
        template_id: TemplateId,
        placement_id: AssetPlacementId,
    ) -> Result<(), String> {
        let template = self
            .templates
            .iter()
            .find(|t| t.template_id == template_id)
            .ok_or_else(|| format!("template {} not found", template_id.as_str()))?;
        let current_z = template
            .placements
            .iter()
            .find(|p| p.placement_id == placement_id)
            .map(|p| p.z_index)
            .ok_or_else(|| format!("placement {} not found", placement_id.as_str()))?;
        let next_lower = template
            .placements
            .iter()
            .filter(|p| p.placement_id != placement_id && p.z_index < current_z)
            .max_by_key(|p| p.z_index);
        let Some(neighbour) = next_lower else {
            return Ok(());
        };
        let neighbour_id = neighbour.placement_id.clone();
        let neighbour_z = neighbour.z_index;
        self.set_z_index(template_id.clone(), placement_id, neighbour_z)?;
        self.set_z_index(template_id, neighbour_id, current_z)
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
            self.selected_placement_id = None;
            // Picking a template drops any prior rail asset focus so the
            // inspector returns to the TEMPLATE section rather than getting
            // trapped on the previously-inspected asset.
            self.inspector_asset_id = None;
        }
    }

    /// Mark a placement as the selected frame. Selecting a frame clears any selected anchor so the
    /// inspector pivots cleanly to the Frame view, mirroring how `select_template` works.
    pub(crate) fn select_placement(&mut self, placement_id: AssetPlacementId) {
        self.selected_placement_id = Some(placement_id);
        self.selected_anchor_id = None;
        // Selection across the rail / canvas surfaces is mutually exclusive,
        // so picking a placement also drops any prior Asset inspector focus.
        self.inspector_asset_id = None;
    }

    /// Mark an anchor as selected. Selecting an anchor clears the selected placement so the
    /// canvas tears down the placement drag bridge + hides its resize handles, and the inspector
    /// pivots cleanly to the Anchor branch. Mirrors `select_placement` for symmetry.
    pub(crate) fn select_anchor(&mut self, anchor_id: AnchorId) {
        self.selected_anchor_id = Some(anchor_id);
        self.selected_placement_id = None;
        // Mutual exclusion: anchor focus replaces any Asset inspector focus.
        self.inspector_asset_id = None;
    }

    /// Switch the active sheet tool. Entering Anchor mode also clears the selected placement so
    /// its resize handles / drag bridge are released, since the placement is no longer the
    /// primary interaction target. Anchor selection survives the switch because anchors are
    /// still interactable in both Select and Anchor modes.
    pub(crate) fn set_tool(&mut self, tool: SheetTool) {
        self.tool = tool;
        if matches!(tool, SheetTool::Anchor) {
            self.selected_placement_id = None;
        }
    }

    // Side-channel for rail-to-canvas drag-and-drop. Dioxus 0.7.9 desktop's
    // DataTransfer::set_data is a no-op stub, so the dragged asset id can't ride the native
    // dataTransfer between dragstart on the rail and drop on the canvas. Stash it here on
    // dragstart, read it from the drop handler, clear on dragend (or after a successful drop).
    pub(crate) fn begin_asset_drag(&mut self, asset_id: AssetId) {
        self.dragging_asset = Some(asset_id);
    }

    pub(crate) fn end_asset_drag(&mut self) {
        self.dragging_asset = None;
    }

    /// Commit the final position of a drag-to-move gesture. Width and height are kept from the
    /// current placement and the new `(x, y)` is clamped so the frame stays inside the canvas.
    /// One `MovePlacement` event is recorded.
    pub(crate) fn commit_drag_end_move(
        &mut self,
        template_id: TemplateId,
        placement_id: AssetPlacementId,
        x: f32,
        y: f32,
    ) -> Result<(), String> {
        let current = self
            .templates
            .iter()
            .find(|t| t.template_id == template_id)
            .and_then(|t| t.placements.iter().find(|p| p.placement_id == placement_id))
            .map(|p| p.position)
            .ok_or_else(|| format!("placement {placement_id} not found"))?;
        let clamped = clamp_rect_to_canvas(TemplateRect {
            x,
            y,
            w: current.w,
            h: current.h,
        });
        self.move_placement(template_id, placement_id, clamped.x, clamped.y)
    }

    /// Commit the final rect of a drag-to-resize gesture. The incoming rect is clamped so the
    /// frame stays inside the canvas and never collapses below `MIN_PLACEMENT_DIMENSION`.
    /// One `ResizePlacement` event is recorded.
    pub(crate) fn commit_drag_end_resize(
        &mut self,
        template_id: TemplateId,
        placement_id: AssetPlacementId,
        after: TemplateRect,
    ) -> Result<(), String> {
        let clamped = clamp_rect_to_canvas(after);
        self.resize_placement(template_id, placement_id, clamped)
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
            self.selected_placement_id = None;
            self.inspector_asset_id = None;
        }
    }

    /// Clicking an asset in the rail focuses the Asset inspector. Selection
    /// across the four user-intent surfaces (template / asset / placement /
    /// anchor) is mutually exclusive, so this also clears any prior placement
    /// or anchor selection. The canvas-tracking `selected_template_id` is
    /// preserved so the canvas keeps its loaded template (the canvas image
    /// renderer reads through `selected_template()`); only `inspector_asset_id`
    /// drives the Asset inspector branch and the Assets-tab row highlight.
    pub(crate) fn select_asset_for_inspector(&mut self, asset_id: AssetId) {
        self.inspector_asset_id = Some(asset_id);
        self.selected_anchor_id = None;
        self.selected_placement_id = None;
    }

    pub(crate) fn rename_selected_asset(&mut self, next: Option<String>) {
        let Some(asset_id) = self.inspector_asset_id.clone() else {
            return;
        };
        let Some(idx) = self.assets.iter().position(|a| a.asset_id == asset_id) else {
            return;
        };
        if self.assets[idx].display_name == next {
            return;
        }
        let before = std::mem::replace(&mut self.assets[idx].display_name, next.clone());
        self.push_event(SheetsEventKind::RenameAsset {
            asset_id,
            before,
            after: next,
        });
        self.mark_dirty();
    }

    pub(crate) fn asset_usage(&self, asset_id: &AssetId) -> AssetUsage {
        let mut placements = 0usize;
        let mut templates = 0usize;
        for template in &self.templates {
            let hits = template
                .placements
                .iter()
                .filter(|p| &p.asset_id == asset_id)
                .count();
            if hits > 0 {
                placements += hits;
                templates += 1;
            }
        }
        AssetUsage {
            placements,
            templates,
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
                .is_none_or(|health| health.missing)
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

    /// Update the selected anchor to a new canvas-relative position. For attached anchors the
    /// position is converted to placement-local space so the anchor keeps tracking its parent.
    /// The `MoveAnchor` event records storage-form values so undo/redo stays consistent with the
    /// stored representation.
    pub(crate) fn update_selected_anchor_position(&mut self, position: AnchorPosition) {
        let storage = {
            let Some(template) = self.selected_template() else {
                return;
            };
            let Some(anchor_id) = self.selected_anchor_id.as_ref() else {
                return;
            };
            let Some(anchor) = template.anchors.iter().find(|a| &a.anchor_id == anchor_id) else {
                return;
            };
            anchor_storage_position(position, anchor.attached_to.as_ref(), &template.placements)
        };
        let Some(anchor) = self.selected_anchor_mut() else {
            return;
        };
        let anchor_id = anchor.anchor_id.clone();
        let before = anchor.position;
        anchor.position = storage;
        let template_id = self
            .selected_template_id
            .clone()
            .expect("selected_anchor_mut implies selected template");
        self.push_event(SheetsEventKind::MoveAnchor {
            template_id,
            anchor_id,
            before,
            after: storage,
        });
        self.mark_dirty();
    }

    /// Reassign the selected anchor to a different placement, or detach it. The on-screen
    /// position is preserved across the swap: storage flips between canvas-relative
    /// (detached) and placement-local (attached) so the disc stays under the same screen
    /// pixel. Reuses the same canvas <-> storage helpers as `update_selected_anchor_position`.
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
        let (canvas_x, canvas_y) = {
            let template = self
                .selected_template()
                .ok_or_else(|| "selected template missing".to_owned())?;
            let anchor = template
                .anchors
                .iter()
                .find(|a| a.anchor_id == anchor_id)
                .ok_or_else(|| "selected anchor missing".to_owned())?;
            crate::frame::sheets::canvas::anchor_canvas_coords(anchor, &template.placements)
        };
        let new_storage = {
            let template = self
                .selected_template()
                .ok_or_else(|| "selected template missing".to_owned())?;
            anchor_storage_position(
                AnchorPosition {
                    x: canvas_x,
                    y: canvas_y,
                },
                attached_to.as_ref(),
                &template.placements,
            )
        };
        let anchor = self
            .selected_anchor_mut()
            .ok_or_else(|| "selected anchor missing".to_owned())?;
        let before = anchor.attached_to.clone();
        anchor.attached_to.clone_from(&attached_to);
        anchor.position = new_storage;

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

    pub(crate) fn remove_asset(&mut self, asset_id: &AssetId) -> Result<(), String> {
        let asset_idx = self
            .assets
            .iter()
            .position(|a| &a.asset_id == asset_id)
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
                if &p.asset_id == asset_id {
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

        // Drop inspector focus on the removed asset so the Asset branch does
        // not render with an orphan id after the Delete confirmation.
        if self.inspector_asset_id.as_ref() == Some(asset_id) {
            self.inspector_asset_id = None;
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

pub(crate) fn filename_of(asset: &AssetEntry) -> String {
    asset
        .original_import_path
        .as_ref()
        .and_then(|p| p.file_name())
        .or_else(|| asset.copied_path.file_name())
        .map_or_else(
            || asset.copied_path.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        )
}

/// User-facing label for an asset. Prefers a user-supplied `display_name`
/// (trimmed, non-empty) and falls back to the on-disk filename. Drives the
/// rail row label and, transitively, the placement default label for
/// placements without their own `display_name`.
pub(crate) fn asset_label_for(asset: &AssetEntry) -> String {
    if let Some(name) = asset
        .display_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return name.to_owned();
    }
    filename_of(asset)
}

/// Resolve the label a placement would show in the absence of a user-supplied
/// `display_name`: source asset label (asset `display_name` or filename),
/// falling back to "Frame N" using the supplied 0-based render index. Used
/// as the placeholder text for the Name input so the user always sees what
/// label the placement falls back to.
pub(crate) fn placement_default_label_for(
    placement: &AssetPlacement,
    assets: &[AssetEntry],
    idx: usize,
) -> String {
    assets
        .iter()
        .find(|asset| asset.asset_id == placement.asset_id)
        .map_or_else(|| format!("Frame {}", idx + 1), asset_label_for)
}

/// Resolve the user-facing label for a placement. Prefers a user-supplied
/// `display_name` when set (and non-empty after trim), otherwise delegates to
/// `placement_default_label_for`. Used by the inspector "Attached to" picker,
/// the canvas placement chip, and any other surface that needs a stable,
/// disambiguating placement label.
pub(crate) fn placement_label_for(
    placement: &AssetPlacement,
    assets: &[AssetEntry],
    idx: usize,
) -> String {
    if let Some(name) = placement
        .display_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return name.to_owned();
    }
    placement_default_label_for(placement, assets, idx)
}

pub(crate) fn default_rect_at(x: f32, y: f32, pixel_dimensions: &PixelDimensions) -> TemplateRect {
    let aspect = pixel_dimensions.width as f32 / pixel_dimensions.height.max(1) as f32;

    // Target a quarter-canvas width first; if that would produce a height larger than half the
    // canvas (portrait images), clamp on the height axis and recompute the width.
    let target_w = 0.25_f32;
    let target_h = target_w / aspect;
    let (w, h) = if target_h > 0.5 {
        let clamped_h = 0.5;
        (clamped_h * aspect, clamped_h)
    } else {
        (target_w, target_h)
    };

    let x0 = (x - w / 2.0).clamp(0.0, 1.0 - w);
    let y0 = (y - h / 2.0).clamp(0.0, 1.0 - h);
    TemplateRect { x: x0, y: y0, w, h }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use inputforge_core::sheet::{AssetHealth, InputTypeHint};

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
                    display_name: None,
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
                display_name: None,
                extensions: ExtensionPayload::default(),
            }],
            asset_health: Vec::new(),
            library_tab: SheetsLibraryTab::Templates,
            selected_template_id: Some(template_id),
            selected_asset_id: Some(asset_id),
            inspector_asset_id: None,
            selected_anchor_id: Some(anchor_id),
            selected_placement_id: None,
            tool: SheetTool::Select,
            autosave: AutosaveStatus::Clean,
            capture: CaptureStatus::Unavailable("live input is not available".to_owned()),
            last_error: None,
            history: VecDeque::new(),
            preset_picker_open: false,
            pending_preset_slots: Vec::new(),
            template_display_name_draft: None,
            anchor_label_draft: None,
            dragging_asset: None,
        }
    }

    #[test]
    fn typing_into_template_display_name_does_not_commit_until_blur() {
        let mut state = SheetsState::default();
        state.create_blank_template();
        let baseline_history = state.history.len();
        state.update_template_display_name_draft("Th".to_owned());
        state.update_template_display_name_draft("Throttle".to_owned());
        assert_eq!(
            state.history.len(),
            baseline_history,
            "draft edits must not push history events"
        );
        state.commit_template_display_name_draft();
        assert_eq!(state.templates[0].display_name, "Throttle");
        assert_eq!(state.history.len() - baseline_history, 1);
    }

    #[test]
    fn committing_empty_template_name_draft_keeps_persisted_value_and_pushes_no_event() {
        let mut state = SheetsState::default();
        state.create_blank_template();
        let original = state.templates[0].display_name.clone();
        let baseline_history = state.history.len();

        for blank in ["", "   ", "\t\n"] {
            state.update_template_display_name_draft(blank.to_owned());
            state.commit_template_display_name_draft();
        }

        assert_eq!(
            state.templates[0].display_name, original,
            "persisted name must survive empty commits"
        );
        assert_eq!(
            state.history.len(),
            baseline_history,
            "empty commits must not push history events"
        );
    }

    #[test]
    fn committing_empty_anchor_label_draft_keeps_persisted_value() {
        let mut state = state_with_template();
        let original = state.templates[0].anchors[0].label.clone();
        let baseline_history = state.history.len();

        state.update_anchor_label_draft(String::new());
        state.commit_anchor_label_draft();
        state.update_anchor_label_draft("   ".to_owned());
        state.commit_anchor_label_draft();

        assert_eq!(state.templates[0].anchors[0].label, original);
        assert_eq!(state.history.len(), baseline_history);
    }

    #[test]
    fn rename_selected_placement_to_some_sets_display_name_and_pushes_one_event() {
        use inputforge_core::sheet::AssetPlacementId;
        let mut state = state_with_template();
        let placement_id = AssetPlacementId::from_string("p-rename");
        state.templates[0].placements.push(AssetPlacement {
            placement_id: placement_id.clone(),
            asset_id: AssetId::from_string("asset-1"),
            position: TemplateRect {
                x: 0.1,
                y: 0.2,
                w: 0.3,
                h: 0.4,
            },
            z_index: 5,
            display_name: None,
            extensions: ExtensionPayload::default(),
        });
        state.selected_placement_id = Some(placement_id.clone());
        let baseline_history = state.history.len();

        state.rename_selected_placement(Some("Throttle".to_owned()));

        let renamed = state.templates[0]
            .placements
            .iter()
            .find(|p| p.placement_id == placement_id)
            .unwrap();
        assert_eq!(renamed.display_name.as_deref(), Some("Throttle"));
        assert_eq!(state.history.len() - baseline_history, 1);
        assert!(matches!(
            state.history.back().unwrap().kind,
            SheetsEventKind::RenamePlacement {
                before: None,
                after: Some(ref s),
                ..
            } if s == "Throttle"
        ));
    }

    #[test]
    fn rename_selected_placement_to_none_clears_display_name_and_pushes_one_event() {
        use inputforge_core::sheet::AssetPlacementId;
        let mut state = state_with_template();
        let placement_id = AssetPlacementId::from_string("p-clear");
        state.templates[0].placements.push(AssetPlacement {
            placement_id: placement_id.clone(),
            asset_id: AssetId::from_string("asset-1"),
            position: TemplateRect {
                x: 0.0,
                y: 0.0,
                w: 0.5,
                h: 0.5,
            },
            z_index: 3,
            display_name: Some("Custom".to_owned()),
            extensions: ExtensionPayload::default(),
        });
        state.selected_placement_id = Some(placement_id.clone());
        let baseline_history = state.history.len();

        state.rename_selected_placement(None);

        let cleared = state.templates[0]
            .placements
            .iter()
            .find(|p| p.placement_id == placement_id)
            .unwrap();
        assert!(cleared.display_name.is_none());
        assert_eq!(state.history.len() - baseline_history, 1);
        assert!(matches!(
            state.history.back().unwrap().kind,
            SheetsEventKind::RenamePlacement {
                before: Some(ref s),
                after: None,
                ..
            } if s == "Custom"
        ));
    }

    #[test]
    fn rename_selected_placement_with_unchanged_value_pushes_no_event() {
        use inputforge_core::sheet::AssetPlacementId;
        let mut state = state_with_template();
        let placement_id = AssetPlacementId::from_string("p-noop");
        state.templates[0].placements.push(AssetPlacement {
            placement_id: placement_id.clone(),
            asset_id: AssetId::from_string("asset-1"),
            position: TemplateRect {
                x: 0.0,
                y: 0.0,
                w: 0.5,
                h: 0.5,
            },
            z_index: 1,
            display_name: Some("Throttle".to_owned()),
            extensions: ExtensionPayload::default(),
        });
        state.selected_placement_id = Some(placement_id);
        let baseline_history = state.history.len();

        // Same name as currently persisted: no event.
        state.rename_selected_placement(Some("Throttle".to_owned()));
        // Clearing an already-None field: no event.
        state.templates[0]
            .placements
            .last_mut()
            .unwrap()
            .display_name = None;
        state.rename_selected_placement(None);

        assert_eq!(state.history.len(), baseline_history);
    }

    #[test]
    fn renaming_unselected_placement_does_nothing() {
        let mut state = state_with_template();
        state.selected_placement_id = None;
        let baseline_history = state.history.len();

        state.rename_selected_placement(Some("ghost".to_owned()));

        assert_eq!(state.history.len(), baseline_history);
    }

    #[test]
    fn rename_selected_asset_to_some_sets_display_name_and_pushes_one_event() {
        let mut state = state_with_template();
        let asset_id = state.assets[0].asset_id.clone();
        state.inspector_asset_id = Some(asset_id.clone());
        let baseline_history = state.history.len();

        state.rename_selected_asset(Some("Throttle Quadrant".to_owned()));

        assert_eq!(
            state.assets[0].display_name.as_deref(),
            Some("Throttle Quadrant")
        );
        assert_eq!(state.history.len() - baseline_history, 1);
        assert!(matches!(
            state.history.back().unwrap().kind,
            SheetsEventKind::RenameAsset {
                before: None,
                after: Some(ref s),
                ..
            } if s == "Throttle Quadrant"
        ));
        assert_eq!(state.autosave, AutosaveStatus::Dirty);
    }

    #[test]
    fn rename_selected_asset_to_none_clears_display_name_and_pushes_one_event() {
        let mut state = state_with_template();
        let asset_id = state.assets[0].asset_id.clone();
        state.assets[0].display_name = Some("Throttle".to_owned());
        state.inspector_asset_id = Some(asset_id);
        let baseline_history = state.history.len();

        state.rename_selected_asset(None);

        assert!(state.assets[0].display_name.is_none());
        assert_eq!(state.history.len() - baseline_history, 1);
        assert!(matches!(
            state.history.back().unwrap().kind,
            SheetsEventKind::RenameAsset {
                before: Some(ref s),
                after: None,
                ..
            } if s == "Throttle"
        ));
    }

    #[test]
    fn rename_selected_asset_with_unchanged_value_pushes_no_event() {
        let mut state = state_with_template();
        let asset_id = state.assets[0].asset_id.clone();
        state.assets[0].display_name = Some("Same".to_owned());
        state.inspector_asset_id = Some(asset_id);
        let baseline_history = state.history.len();

        state.rename_selected_asset(Some("Same".to_owned()));

        assert_eq!(state.history.len(), baseline_history);
    }

    #[test]
    fn renaming_unselected_asset_does_nothing() {
        let mut state = state_with_template();
        state.inspector_asset_id = None;
        let baseline_history = state.history.len();

        state.rename_selected_asset(Some("ghost".to_owned()));

        assert_eq!(state.history.len(), baseline_history);
    }

    #[test]
    fn select_asset_for_inspector_clears_placement_and_anchor_and_keeps_template_loaded() {
        // Selection across the four user-intent surfaces (template / asset /
        // placement / anchor) is mutually exclusive. Clicking an asset takes
        // focus AWAY from any prior placement / anchor, while leaving the
        // canvas-tracking template id in place so the canvas keeps its image.
        let mut state = state_with_template();
        let saved_template = state.selected_template_id.clone();
        let saved_canvas_asset = state.selected_asset_id.clone();
        let seeded_placement_id = AssetPlacementId::from_string("p-pre-seeded");
        state.templates[0].placements.push(AssetPlacement {
            placement_id: seeded_placement_id.clone(),
            asset_id: state.assets[0].asset_id.clone(),
            position: TemplateRect {
                x: 0.1,
                y: 0.2,
                w: 0.3,
                h: 0.4,
            },
            z_index: 5,
            display_name: None,
            extensions: ExtensionPayload::default(),
        });
        state.selected_placement_id = Some(seeded_placement_id);
        let seeded_anchor_id = state.templates[0].anchors[0].anchor_id.clone();
        state.selected_anchor_id = Some(seeded_anchor_id);

        let other_asset_id = AssetId::from_string("asset-from-rail");
        state.assets.push(AssetEntry {
            asset_id: other_asset_id.clone(),
            copied_path: PathBuf::from("assets/asset-from-rail.png"),
            content_hash: "h".to_owned(),
            media_type: "image/png".to_owned(),
            pixel_dimensions: PixelDimensions {
                width: 16,
                height: 16,
            },
            original_import_path: None,
            display_name: None,
            extensions: ExtensionPayload::default(),
        });
        let baseline_history = state.history.len();

        state.select_asset_for_inspector(other_asset_id.clone());

        assert_eq!(state.inspector_asset_id, Some(other_asset_id));
        assert!(
            state.selected_anchor_id.is_none(),
            "rail asset click must clear any selected anchor"
        );
        assert!(
            state.selected_placement_id.is_none(),
            "rail asset click must clear any selected placement"
        );
        assert_eq!(
            state.selected_template_id, saved_template,
            "rail asset click must keep the canvas template loaded"
        );
        assert_eq!(
            state.selected_asset_id, saved_canvas_asset,
            "rail asset click must not touch the canvas-tracking selected_asset_id"
        );
        assert_eq!(state.history.len(), baseline_history);
    }

    #[test]
    fn selecting_a_placement_clears_inspector_asset_focus() {
        let mut state = state_with_template();
        let asset_id = state.assets[0].asset_id.clone();
        state.inspector_asset_id = Some(asset_id);
        let placement_id = AssetPlacementId::from_string("p-pick");
        state.templates[0].placements.push(AssetPlacement {
            placement_id: placement_id.clone(),
            asset_id: state.assets[0].asset_id.clone(),
            position: TemplateRect {
                x: 0.0,
                y: 0.0,
                w: 0.5,
                h: 0.5,
            },
            z_index: 0,
            display_name: None,
            extensions: ExtensionPayload::default(),
        });

        state.select_placement(placement_id.clone());

        assert_eq!(state.selected_placement_id, Some(placement_id));
        assert!(
            state.inspector_asset_id.is_none(),
            "select_placement must drop inspector asset focus"
        );
    }

    #[test]
    fn selecting_an_anchor_clears_inspector_asset_focus() {
        let mut state = state_with_template();
        let asset_id = state.assets[0].asset_id.clone();
        state.inspector_asset_id = Some(asset_id);
        let anchor_id = state.templates[0].anchors[0].anchor_id.clone();

        state.select_anchor(anchor_id.clone());

        assert_eq!(state.selected_anchor_id, Some(anchor_id));
        assert!(
            state.inspector_asset_id.is_none(),
            "select_anchor must drop inspector asset focus"
        );
    }

    #[test]
    fn selecting_a_template_clears_inspector_asset_focus() {
        let mut state = state_with_template();
        // Simulate a prior rail asset click that focused the Asset inspector.
        let some_asset_id = state.assets[0].asset_id.clone();
        state.inspector_asset_id = Some(some_asset_id);
        let template_id = state.templates[0].template_id.clone();
        let first_placement_asset = state.templates[0]
            .placements
            .first()
            .map(|p| p.asset_id.clone());

        state.select_template(template_id.clone());

        assert!(
            state.inspector_asset_id.is_none(),
            "select_template must drop inspector_asset_id so the inspector returns to TEMPLATE"
        );
        assert_eq!(
            state.selected_asset_id, first_placement_asset,
            "select_template still sets the canvas-tracking selected_asset_id"
        );
        assert_eq!(state.selected_template_id, Some(template_id));
    }

    #[test]
    fn asset_usage_counts_placements_across_templates() {
        let mut state = state_with_template();
        let asset_a = state.assets[0].asset_id.clone();
        let asset_b = AssetId::from_string("asset-b");
        state.assets.push(AssetEntry {
            asset_id: asset_b.clone(),
            copied_path: PathBuf::from("assets/b.png"),
            content_hash: "h".to_owned(),
            media_type: "image/png".to_owned(),
            pixel_dimensions: PixelDimensions {
                width: 16,
                height: 16,
            },
            original_import_path: None,
            display_name: None,
            extensions: ExtensionPayload::default(),
        });
        state.templates[0].placements.push(AssetPlacement {
            placement_id: AssetPlacementId::new(),
            asset_id: asset_a.clone(),
            position: TemplateRect {
                x: 0.0,
                y: 0.0,
                w: 0.5,
                h: 0.5,
            },
            z_index: 1,
            display_name: None,
            extensions: ExtensionPayload::default(),
        });
        state.templates.push(DeviceTemplate {
            template_id: TemplateId::from_string("template-second"),
            display_name: "Second".to_owned(),
            matching_hints: Vec::new(),
            placements: vec![
                AssetPlacement {
                    placement_id: AssetPlacementId::new(),
                    asset_id: asset_a.clone(),
                    position: TemplateRect {
                        x: 0.0,
                        y: 0.0,
                        w: 0.5,
                        h: 0.5,
                    },
                    z_index: 0,
                    display_name: None,
                    extensions: ExtensionPayload::default(),
                },
                AssetPlacement {
                    placement_id: AssetPlacementId::new(),
                    asset_id: asset_b.clone(),
                    position: TemplateRect {
                        x: 0.0,
                        y: 0.0,
                        w: 0.5,
                        h: 0.5,
                    },
                    z_index: 1,
                    display_name: None,
                    extensions: ExtensionPayload::default(),
                },
            ],
            anchors: Vec::new(),
            default_anchor_bindings: Vec::new(),
            grouping_hints: Vec::new(),
            default_token_preset: TokenPreset::Standard,
            extensions: ExtensionPayload::default(),
        });

        let usage_a = state.asset_usage(&asset_a);
        let usage_b = state.asset_usage(&asset_b);
        let unused_id = AssetId::from_string("nowhere");
        let usage_unused = state.asset_usage(&unused_id);

        assert_eq!(
            usage_a,
            AssetUsage {
                placements: 3,
                templates: 2
            }
        );
        assert_eq!(
            usage_b,
            AssetUsage {
                placements: 1,
                templates: 1
            }
        );
        assert_eq!(usage_unused, AssetUsage::default());
    }

    #[test]
    fn asset_label_for_falls_back_to_filename_when_display_name_unset() {
        let asset = AssetEntry {
            asset_id: AssetId::from_string("a"),
            copied_path: PathBuf::from("assets/cockpit.png"),
            content_hash: "h".to_owned(),
            media_type: "image/png".to_owned(),
            pixel_dimensions: PixelDimensions {
                width: 16,
                height: 16,
            },
            original_import_path: Some(PathBuf::from("/imports/cockpit.png")),
            display_name: None,
            extensions: ExtensionPayload::default(),
        };

        assert_eq!(asset_label_for(&asset), "cockpit.png");
    }

    #[test]
    fn asset_label_for_uses_display_name_when_set_and_non_blank() {
        let mut asset = AssetEntry {
            asset_id: AssetId::from_string("a"),
            copied_path: PathBuf::from("assets/cockpit.png"),
            content_hash: "h".to_owned(),
            media_type: "image/png".to_owned(),
            pixel_dimensions: PixelDimensions {
                width: 16,
                height: 16,
            },
            original_import_path: None,
            display_name: Some("  Stick  ".to_owned()),
            extensions: ExtensionPayload::default(),
        };

        assert_eq!(asset_label_for(&asset), "Stick");

        asset.display_name = Some("   ".to_owned());
        assert_eq!(
            asset_label_for(&asset),
            "cockpit.png",
            "blank display_name must fall back to filename"
        );
    }

    #[test]
    fn placement_default_label_for_picks_up_asset_display_name_when_set() {
        let asset_id = AssetId::from_string("renamed");
        let asset = AssetEntry {
            asset_id: asset_id.clone(),
            copied_path: PathBuf::from("assets/ugly-filename.png"),
            content_hash: "h".to_owned(),
            media_type: "image/png".to_owned(),
            pixel_dimensions: PixelDimensions {
                width: 16,
                height: 16,
            },
            original_import_path: None,
            display_name: Some("Throttle".to_owned()),
            extensions: ExtensionPayload::default(),
        };
        let placement = AssetPlacement {
            placement_id: AssetPlacementId::new(),
            asset_id,
            position: TemplateRect {
                x: 0.0,
                y: 0.0,
                w: 1.0,
                h: 1.0,
            },
            z_index: 0,
            display_name: None,
            extensions: ExtensionPayload::default(),
        };

        assert_eq!(
            placement_default_label_for(&placement, &[asset], 0),
            "Throttle",
        );
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
            display_name: None,
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
                display_name: None,
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
            display_name: None,
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
            display_name: None,
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

        classify_save_outcome(Ok(()), Ok(())).unwrap();

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
                TemplateRect {
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
            display_name: None,
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

        state.remove_placement(template_id, &placement_id).unwrap();

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
            display_name: None,
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
                display_name: None,
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
            display_name: None,
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
    fn shift_z_up_on_top_placement_is_silent_no_op() {
        use inputforge_core::sheet::{AssetPlacement, AssetPlacementId, TemplateRect};
        let mut state = state_with_template();
        let template_id = state.selected_template_id.clone().unwrap();
        let top = AssetPlacementId::from_string("top");
        state.templates[0].placements.push(AssetPlacement {
            placement_id: top.clone(),
            asset_id: AssetId::from_string("a"),
            position: TemplateRect {
                x: 0.0,
                y: 0.0,
                w: 0.5,
                h: 0.5,
            },
            z_index: 5,
            display_name: None,
            extensions: ExtensionPayload::default(),
        });
        let baseline = state.history.len();
        state.shift_z_up(template_id, top).unwrap();
        assert_eq!(
            state.history.len(),
            baseline,
            "shift_z_up on top placement must not push any event"
        );
    }

    #[test]
    fn shift_z_up_swaps_z_with_next_higher_sibling() {
        use inputforge_core::sheet::{AssetPlacement, AssetPlacementId, TemplateRect};
        let mut state = state_with_template();
        let template_id = state.selected_template_id.clone().unwrap();
        let lower = AssetPlacementId::from_string("lower");
        let upper = AssetPlacementId::from_string("upper");
        state.templates[0].placements.push(AssetPlacement {
            placement_id: lower.clone(),
            asset_id: AssetId::from_string("a"),
            position: TemplateRect {
                x: 0.0,
                y: 0.0,
                w: 0.5,
                h: 0.5,
            },
            z_index: 1,
            display_name: None,
            extensions: ExtensionPayload::default(),
        });
        state.templates[0].placements.push(AssetPlacement {
            placement_id: upper.clone(),
            asset_id: AssetId::from_string("a"),
            position: TemplateRect {
                x: 0.0,
                y: 0.0,
                w: 0.5,
                h: 0.5,
            },
            z_index: 3,
            display_name: None,
            extensions: ExtensionPayload::default(),
        });
        state.shift_z_up(template_id, lower.clone()).unwrap();
        let placements = &state.templates[0].placements;
        let lower_z = placements
            .iter()
            .find(|p| p.placement_id == lower)
            .unwrap()
            .z_index;
        let upper_z = placements
            .iter()
            .find(|p| p.placement_id == upper)
            .unwrap()
            .z_index;
        assert_eq!(lower_z, 3);
        assert_eq!(upper_z, 1);
    }

    #[test]
    fn shift_z_down_on_bottom_placement_is_silent_no_op() {
        use inputforge_core::sheet::{AssetPlacement, AssetPlacementId, TemplateRect};
        let mut state = state_with_template();
        let template_id = state.selected_template_id.clone().unwrap();
        let bottom = AssetPlacementId::from_string("bottom");
        state.templates[0].placements.push(AssetPlacement {
            placement_id: bottom.clone(),
            asset_id: AssetId::from_string("a"),
            position: TemplateRect {
                x: 0.0,
                y: 0.0,
                w: 0.5,
                h: 0.5,
            },
            z_index: -5,
            display_name: None,
            extensions: ExtensionPayload::default(),
        });
        let baseline = state.history.len();
        state.shift_z_down(template_id, bottom).unwrap();
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
            display_name: None,
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
            display_name: None,
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

        state.assets.push(AssetEntry {
            asset_id: asset_id.clone(),
            copied_path: PathBuf::from("assets/cascade.png"),
            content_hash: "hash".to_owned(),
            media_type: "image/png".to_owned(),
            pixel_dimensions: PixelDimensions {
                width: 1,
                height: 1,
            },
            original_import_path: None,
            display_name: None,
            extensions: ExtensionPayload::default(),
        });

        state.remove_asset(&asset_id).unwrap();
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
            ("RenamePlacement", "RenamePlacement"),
            ("PlaceAnchor", "RemoveAnchor"),
            ("RemoveAnchor", "PlaceAnchor"),
            ("RenameAnchor", "RenameAnchor"),
            ("MoveAnchor", "MoveAnchor"),
            ("ToggleAnchorAttach", "ToggleAnchorAttach"),
            ("AssignAnchor", "AssignAnchor"),
            ("ImportAsset", "RemoveAsset"),
            ("RemoveAsset", "ImportAsset"),
            ("RenameAsset", "RenameAsset"),
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
            display_name: None,
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

    #[test]
    fn pluralize_uses_singular_only_for_count_one() {
        use super::pluralize;
        assert_eq!(pluralize(0, "frame", "frames"), "0 frames");
        assert_eq!(pluralize(1, "frame", "frames"), "1 frame");
        assert_eq!(pluralize(2, "frame", "frames"), "2 frames");
        assert_eq!(pluralize(5, "anchor", "anchors"), "5 anchors");
    }

    #[test]
    fn sheet_tool_default_is_select_and_only_anchor_variant_exists_besides_it() {
        // Compile-time exhaustiveness: this match must remain total. If a third variant lands
        // (or Anchor is renamed), the compiler refuses the build.
        fn _matches(tool: SheetTool) {
            match tool {
                SheetTool::Select | SheetTool::Anchor => {}
            }
        }
        assert_eq!(SheetTool::default(), SheetTool::Select);
    }

    #[test]
    fn placing_anchor_with_target_placement_records_attached_to() {
        use inputforge_core::sheet::{AssetPlacement, AssetPlacementId, TemplateRect};
        let mut state = state_with_template();
        let placement_id = AssetPlacementId::from_string("p-target");
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
            display_name: None,
            extensions: ExtensionPayload::default(),
        });

        let anchor_id = state.place_anchor_on(
            AnchorPosition { x: 0.5, y: 0.5 },
            Some(placement_id.clone()),
        );

        let anchor = state.templates[0]
            .anchors
            .iter()
            .find(|a| a.anchor_id == anchor_id)
            .unwrap();
        assert_eq!(anchor.attached_to, Some(placement_id));
    }

    #[test]
    fn placing_anchor_without_target_records_floating_anchor() {
        let mut state = state_with_template();
        let anchor_id = state.place_anchor_on(AnchorPosition { x: 0.5, y: 0.5 }, None);
        let anchor = state.templates[0]
            .anchors
            .iter()
            .find(|a| a.anchor_id == anchor_id)
            .unwrap();
        assert_eq!(anchor.attached_to, None);
    }

    #[test]
    fn default_rect_at_square_asset_yields_quarter_canvas() {
        use crate::frame::sheets::state::default_rect_at;
        use inputforge_core::sheet::PixelDimensions;
        let rect = default_rect_at(
            0.5,
            0.5,
            &PixelDimensions {
                width: 100,
                height: 100,
            },
        );
        assert!((rect.w - 0.25).abs() < 1e-6);
        assert!((rect.h - 0.25).abs() < 1e-6);
    }

    #[test]
    fn default_rect_at_landscape_asset_keeps_aspect() {
        use crate::frame::sheets::state::default_rect_at;
        use inputforge_core::sheet::PixelDimensions;
        let rect = default_rect_at(
            0.5,
            0.5,
            &PixelDimensions {
                width: 400,
                height: 100,
            },
        );
        assert!((rect.w - 0.25).abs() < 1e-6);
        assert!((rect.h - 0.0625).abs() < 1e-6);
    }

    #[test]
    fn default_rect_at_portrait_asset_clamps_to_half_height() {
        use crate::frame::sheets::state::default_rect_at;
        use inputforge_core::sheet::PixelDimensions;
        let rect = default_rect_at(
            0.5,
            0.5,
            &PixelDimensions {
                width: 100,
                height: 400,
            },
        );
        // aspect = 0.25, target_h = 0.25 / 0.25 = 1.0 > 0.5, clamp to h=0.5, w = 0.5 * 0.25 = 0.125
        assert!((rect.h - 0.5).abs() < 1e-6);
        assert!((rect.w - 0.125).abs() < 1e-6);
    }

    #[test]
    fn default_rect_at_clamps_so_rect_stays_inside_canvas() {
        use crate::frame::sheets::state::default_rect_at;
        use inputforge_core::sheet::PixelDimensions;
        let rect = default_rect_at(
            1.0,
            1.0,
            &PixelDimensions {
                width: 100,
                height: 100,
            },
        );
        assert!(rect.x + rect.w <= 1.0 + f32::EPSILON);
        assert!(rect.y + rect.h <= 1.0 + f32::EPSILON);
    }

    #[test]
    fn drag_end_emits_exactly_one_move_placement_event() {
        use inputforge_core::sheet::{AssetPlacement, AssetPlacementId, TemplateRect};
        let mut state = state_with_template();
        let template_id = state.selected_template_id.clone().unwrap();
        let placement_id = AssetPlacementId::from_string("p-drag");
        state.templates[0].placements.push(AssetPlacement {
            placement_id: placement_id.clone(),
            asset_id: AssetId::from_string("asset-1"),
            position: TemplateRect {
                x: 0.1,
                y: 0.1,
                w: 0.3,
                h: 0.3,
            },
            z_index: 0,
            display_name: None,
            extensions: ExtensionPayload::default(),
        });
        let baseline = state.history.len();

        state
            .commit_drag_end_move(template_id.clone(), placement_id.clone(), 0.5, 0.5)
            .unwrap();

        assert_eq!(state.history.len() - baseline, 1);
        assert!(matches!(
            state.history.back().unwrap().kind,
            SheetsEventKind::MovePlacement { .. }
        ));
    }

    #[test]
    fn commit_drag_end_move_clamps_position_so_the_frame_stays_inside_canvas() {
        use inputforge_core::sheet::{AssetPlacement, AssetPlacementId, TemplateRect};
        let mut state = state_with_template();
        let template_id = state.selected_template_id.clone().unwrap();
        let placement_id = AssetPlacementId::from_string("p-clamp");
        state.templates[0].placements.push(AssetPlacement {
            placement_id: placement_id.clone(),
            asset_id: AssetId::from_string("asset-1"),
            position: TemplateRect {
                x: 0.0,
                y: 0.0,
                w: 0.3,
                h: 0.3,
            },
            z_index: 0,
            display_name: None,
            extensions: ExtensionPayload::default(),
        });

        state
            .commit_drag_end_move(template_id, placement_id.clone(), 5.0, 5.0)
            .unwrap();
        let rect = state.templates[0]
            .placements
            .iter()
            .find(|p| p.placement_id == placement_id)
            .unwrap()
            .position;
        assert!(
            (rect.x + rect.w) <= 1.0 + f32::EPSILON,
            "right edge inside canvas, got {rect:?}"
        );
        assert!(
            (rect.y + rect.h) <= 1.0 + f32::EPSILON,
            "bottom edge inside canvas, got {rect:?}"
        );
    }

    #[test]
    fn commit_drag_end_resize_enforces_minimum_frame_size() {
        use inputforge_core::sheet::{AssetPlacement, AssetPlacementId, TemplateRect};
        let mut state = state_with_template();
        let template_id = state.selected_template_id.clone().unwrap();
        let placement_id = AssetPlacementId::from_string("p-shrink");
        state.templates[0].placements.push(AssetPlacement {
            placement_id: placement_id.clone(),
            asset_id: AssetId::from_string("asset-1"),
            position: TemplateRect {
                x: 0.0,
                y: 0.0,
                w: 0.3,
                h: 0.3,
            },
            z_index: 0,
            display_name: None,
            extensions: ExtensionPayload::default(),
        });

        state
            .commit_drag_end_resize(
                template_id,
                placement_id.clone(),
                TemplateRect {
                    x: 0.5,
                    y: 0.5,
                    w: 0.0,
                    h: 0.0,
                },
            )
            .unwrap();
        let rect = state.templates[0]
            .placements
            .iter()
            .find(|p| p.placement_id == placement_id)
            .unwrap()
            .position;
        assert!(rect.w >= 0.02 - f32::EPSILON);
        assert!(rect.h >= 0.02 - f32::EPSILON);
    }

    #[test]
    fn selecting_a_placement_clears_selected_anchor_id() {
        use inputforge_core::sheet::{AssetPlacement, AssetPlacementId, TemplateRect};
        let mut state = state_with_template();
        state.selected_anchor_id = Some(state.templates[0].anchors[0].anchor_id.clone());
        let placement_id = AssetPlacementId::from_string("p-1");
        state.templates[0].placements.push(AssetPlacement {
            placement_id: placement_id.clone(),
            asset_id: AssetId::from_string("asset-1"),
            position: TemplateRect {
                x: 0.0,
                y: 0.0,
                w: 0.3,
                h: 0.3,
            },
            z_index: 0,
            display_name: None,
            extensions: ExtensionPayload::default(),
        });

        state.select_placement(placement_id.clone());
        assert_eq!(state.selected_placement_id.as_ref(), Some(&placement_id));
        assert!(state.selected_anchor_id.is_none());
    }

    fn push_placement(state: &mut SheetsState, id: &str, rect: TemplateRect) -> AssetPlacementId {
        let placement_id = AssetPlacementId::from_string(id);
        state.templates[0].placements.push(AssetPlacement {
            placement_id: placement_id.clone(),
            asset_id: AssetId::from_string("asset-1"),
            position: rect,
            z_index: 0,
            display_name: None,
            extensions: ExtensionPayload::default(),
        });
        placement_id
    }

    fn anchor_position_approx_eq(actual: AnchorPosition, expected: AnchorPosition) {
        assert!(
            (actual.x - expected.x).abs() < 1e-6,
            "x mismatch: actual={}, expected={}",
            actual.x,
            expected.x,
        );
        assert!(
            (actual.y - expected.y).abs() < 1e-6,
            "y mismatch: actual={}, expected={}",
            actual.y,
            expected.y,
        );
    }

    #[test]
    fn place_anchor_on_attached_converts_canvas_to_local() {
        let mut state = state_with_template();
        let placement_id = push_placement(
            &mut state,
            "p-attach",
            TemplateRect {
                x: 0.2,
                y: 0.3,
                w: 0.4,
                h: 0.5,
            },
        );

        let anchor_id = state.place_anchor_on(
            AnchorPosition { x: 0.4, y: 0.55 },
            Some(placement_id.clone()),
        );

        let anchor = state.templates[0]
            .anchors
            .iter()
            .find(|a| a.anchor_id == anchor_id)
            .unwrap();
        assert_eq!(anchor.attached_to, Some(placement_id));
        anchor_position_approx_eq(anchor.position, AnchorPosition { x: 0.5, y: 0.5 });
    }

    #[test]
    fn place_anchor_on_with_orphan_parent_stores_canvas() {
        let mut state = state_with_template();
        let bogus = AssetPlacementId::from_string("p-does-not-exist");

        let anchor_id =
            state.place_anchor_on(AnchorPosition { x: 0.42, y: 0.61 }, Some(bogus.clone()));

        let anchor = state.templates[0]
            .anchors
            .iter()
            .find(|a| a.anchor_id == anchor_id)
            .unwrap();
        anchor_position_approx_eq(anchor.position, AnchorPosition { x: 0.42, y: 0.61 });
    }

    #[test]
    fn update_selected_anchor_position_attached_converts_canvas_to_local() {
        let mut state = state_with_template();
        let placement_id = push_placement(
            &mut state,
            "p-attach",
            TemplateRect {
                x: 0.2,
                y: 0.3,
                w: 0.4,
                h: 0.5,
            },
        );
        let anchor_id = state.place_anchor_on(
            AnchorPosition { x: 0.2, y: 0.3 },
            Some(placement_id.clone()),
        );
        state.selected_anchor_id = Some(anchor_id.clone());

        state.update_selected_anchor_position(AnchorPosition { x: 0.6, y: 0.8 });

        let anchor = state.templates[0]
            .anchors
            .iter()
            .find(|a| a.anchor_id == anchor_id)
            .unwrap();
        anchor_position_approx_eq(anchor.position, AnchorPosition { x: 1.0, y: 1.0 });
    }

    #[test]
    fn update_selected_anchor_position_floating_preserves_canvas() {
        let mut state = state_with_template();

        state.update_selected_anchor_position(AnchorPosition { x: 0.7, y: 0.4 });

        let anchor = &state.templates[0].anchors[0];
        anchor_position_approx_eq(anchor.position, AnchorPosition { x: 0.7, y: 0.4 });
    }

    #[test]
    fn anchor_canvas_coords_roundtrip_after_place_anchor_on() {
        use crate::frame::sheets::canvas::anchor_canvas_coords;
        let mut state = state_with_template();
        let placement_id = push_placement(
            &mut state,
            "p-attach",
            TemplateRect {
                x: 0.1,
                y: 0.2,
                w: 0.6,
                h: 0.4,
            },
        );

        let anchor_id =
            state.place_anchor_on(AnchorPosition { x: 0.42, y: 0.61 }, Some(placement_id));

        let template = &state.templates[0];
        let anchor = template
            .anchors
            .iter()
            .find(|a| a.anchor_id == anchor_id)
            .unwrap();
        let (rendered_x, rendered_y) = anchor_canvas_coords(anchor, &template.placements);
        assert!((rendered_x - 0.42).abs() < 1e-6, "rendered_x={rendered_x}");
        assert!((rendered_y - 0.61).abs() < 1e-6, "rendered_y={rendered_y}");
    }

    #[test]
    fn selecting_anchor_clears_placement_selection() {
        let mut state = state_with_template();
        let placement_id = push_placement(
            &mut state,
            "p-1",
            TemplateRect {
                x: 0.0,
                y: 0.0,
                w: 0.5,
                h: 0.5,
            },
        );
        state.selected_placement_id = Some(placement_id);
        let anchor_id = state.templates[0].anchors[0].anchor_id.clone();

        state.select_anchor(anchor_id.clone());

        assert_eq!(state.selected_anchor_id, Some(anchor_id));
        assert!(state.selected_placement_id.is_none());
    }

    #[test]
    fn entering_anchor_tool_clears_placement_selection() {
        let mut state = state_with_template();
        let placement_id = push_placement(
            &mut state,
            "p-1",
            TemplateRect {
                x: 0.0,
                y: 0.0,
                w: 0.5,
                h: 0.5,
            },
        );
        state.selected_placement_id = Some(placement_id);

        state.set_tool(SheetTool::Anchor);

        assert_eq!(state.tool, SheetTool::Anchor);
        assert!(state.selected_placement_id.is_none());
    }

    #[test]
    fn entering_select_tool_preserves_anchor_selection() {
        let mut state = state_with_template();
        let anchor_id = state.templates[0].anchors[0].anchor_id.clone();
        state.selected_anchor_id = Some(anchor_id.clone());
        state.tool = SheetTool::Anchor;

        state.set_tool(SheetTool::Select);

        assert_eq!(state.tool, SheetTool::Select);
        assert_eq!(state.selected_anchor_id, Some(anchor_id));
    }
}
