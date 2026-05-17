#![expect(
    unused_qualifications,
    reason = "rsx! macro expansion triggers false-positive unused_qualifications warnings on onclick:"
)]

use std::fmt::Write as _;
use std::path::Path;

use dioxus::prelude::*;
use inputforge_core::sheet::{
    AnchorAssignment, AnchorId, AnchorPosition, AssetHealth, TemplateRect,
};
use serde::Deserialize;

use crate::components::Icon;
use crate::frame::sheets::state::{
    AutosaveStatus, SheetTool, SheetsState, default_rect_at, normalized_image_point,
};
use crate::icons::{Icon as IconKind, IconSize};
use inputforge_core::sheet::AssetId;

#[derive(Debug, Clone, Copy, Deserialize)]
struct StageRectPayload {
    left: f64,
    top: f64,
    width: f64,
    height: f64,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
enum PlacementDragPayload {
    Move { x: f32, y: f32 },
    Resize { x: f32, y: f32, w: f32, h: f32 },
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct AnchorDragPayload {
    x: f64,
    y: f64,
}

const STAGE_RESIZE_LISTENER_PREFIX: &str = "__inputforgeSheetsStageResize_";
const PLACEMENT_DRAG_LISTENER_PREFIX: &str = "__inputforgeSheetsPlacementDrag_";
const ANCHOR_DRAG_LISTENER_PREFIX: &str = "__inputforgeSheetsAnchorDrag_";

#[component]
pub(crate) fn SheetsCanvas(
    sheets: Signal<SheetsState>,
    on_import_image: EventHandler<()>,
    on_arm_capture: EventHandler<AnchorId>,
) -> Element {
    let _ = on_arm_capture;
    let stage_rect = use_signal(|| StageRectPayload {
        left: 0.0,
        top: 0.0,
        width: 1.0,
        height: 1.0,
    });
    let mut image_load_failed = use_signal(|| false);
    let stage_id = "if-sheets-image-stage";
    let state = sheets.read();
    let selected_template = state.selected_template().cloned();
    let selected_asset = state.selected_asset().cloned();
    let selected_asset_health = state.selected_asset_health().cloned();
    let selected_asset_missing = state.selected_asset_missing();
    let selected_anchor_id = state.selected_anchor_id.clone();
    let selected_placement_id = state.selected_placement_id.clone();
    let tool = state.tool;
    let asset_health_all = state.asset_health.clone();
    let assets_all = state.assets.clone();
    drop(state);

    let mut placements_sorted = selected_template
        .as_ref()
        .map(|t| t.placements.clone())
        .unwrap_or_default();
    placements_sorted.sort_by_key(|p| p.z_index);

    let has_template = selected_template.is_some();
    let toolbar_name = selected_template
        .as_ref()
        .map_or_else(|| "No template".to_owned(), |t| t.display_name.clone());
    let tool_is_select = matches!(tool, SheetTool::Select);
    let tool_is_anchor = matches!(tool, SheetTool::Anchor);
    let (autosave_status_token, autosave_tone, autosave_label) = match sheets.read().autosave {
        AutosaveStatus::Clean => ("clean", "neutral", "Saved"),
        AutosaveStatus::Saving => ("saving", "neutral", "Saving"),
        AutosaveStatus::Dirty => ("dirty", "amber", "Unsaved"),
        AutosaveStatus::Failed => ("failed", "red", "Save failed"),
    };
    let anchor_count = selected_template
        .as_ref()
        .map_or(0, |template| template.anchors.len());
    let anchor_count_label = if anchor_count == 1 {
        "1 anchor".to_owned()
    } else {
        format!("{anchor_count} anchors")
    };
    let asset_id = selected_asset
        .as_ref()
        .map(|asset| asset.asset_id.as_str().to_owned())
        .unwrap_or_default();
    let stage_identity = selected_asset
        .as_ref()
        .map(|asset| asset.asset_id.as_str().to_owned())
        .or_else(|| {
            selected_template
                .as_ref()
                .map(|template| template.template_id.as_str().to_owned())
        })
        .unwrap_or_default();
    let asset_source_path = selected_asset_health
        .as_ref()
        .filter(|health| !health.missing)
        .map(|health| file_url_from_path(&health.copied_absolute_path))
        .unwrap_or_default();
    let (asset_src, asset_read_error) = selected_asset_health
        .as_ref()
        .filter(|health| !health.missing)
        .map(asset_data_url)
        .map_or_else(
            || (String::new(), None),
            |result| match result {
                Ok(asset_src) => (asset_src, None),
                Err(err) => (String::new(), Some(err.to_string())),
            },
        );
    let image_load_key = format!("{asset_source_path}|{selected_asset_missing}");
    let image_load_failed_now = *image_load_failed.read() || asset_read_error.is_some();
    let missing_asset_entry = selected_asset_health
        .as_ref()
        .map(|health| &health.entry)
        .or(selected_asset.as_ref());
    let missing_copied_path = selected_asset_health
        .as_ref()
        .map(|health| health.copied_absolute_path.display().to_string())
        .or_else(|| missing_asset_entry.map(|asset| asset.copied_path.display().to_string()));
    let missing_original_path = missing_asset_entry
        .and_then(|asset| asset.original_import_path.as_ref())
        .map(|path| path.display().to_string());
    let current_stage_listener_key =
        stage_subscription_key(has_template, selected_asset_missing, &stage_identity)
            .map(|stage_key| stage_resize_listener_key(&stage_key));
    let mut active_stage_listener_key = use_signal(|| Option::<String>::None);
    use_effect(use_reactive!(|current_stage_listener_key| {
        let previous_key = active_stage_listener_key.peek().clone();
        if previous_key == current_stage_listener_key {
            return;
        }

        if let Some(listener_key) = previous_key {
            spawn(async move {
                let _ = document::eval(&cleanup_stage_resize_listener_script(&listener_key));
            });
        }

        active_stage_listener_key.set(current_stage_listener_key.clone());

        let Some(listener_key) = current_stage_listener_key.clone() else {
            return;
        };

        let stage_id = stage_id.to_owned();
        let mut stage_rect = stage_rect;
        spawn(async move {
            let js = install_stage_resize_listener_script(&stage_id, &listener_key);
            let mut handle = document::eval(&js);
            loop {
                let Ok(rect) = handle.recv::<StageRectPayload>().await else {
                    break;
                };
                stage_rect.set(rect);
            }
        });
    }));
    use_effect(use_reactive!(|image_load_key| {
        let _ = image_load_key;
        image_load_failed.set(false);
    }));
    // Placement pointer-drag bridge: install when a placement is selected.
    let mut sheets_for_placement_drag = sheets;
    let placement_drag_selection = selected_placement_id.clone();
    let mut active_placement_drag_key = use_signal(|| Option::<String>::None);
    use_effect(use_reactive!(|placement_drag_selection| {
        let previous_key = active_placement_drag_key.peek().clone();
        let new_key = placement_drag_selection
            .as_ref()
            .map(|id| format!("{PLACEMENT_DRAG_LISTENER_PREFIX}{}", id.as_str()));
        if previous_key == new_key {
            return;
        }

        if let Some(listener_key) = previous_key {
            spawn(async move {
                let _ = document::eval(&cleanup_drag_bridge_script(&listener_key));
            });
        }

        active_placement_drag_key.set(new_key.clone());

        let Some(placement_id) = placement_drag_selection.clone() else {
            return;
        };
        let Some(listener_key) = new_key else {
            return;
        };
        let stage_id = stage_id.to_owned();
        let placement_id_str = placement_id.as_str().to_owned();

        spawn(async move {
            let js = install_placement_drag_bridge(&stage_id, &placement_id_str, &listener_key);
            let mut handle = document::eval(&js);
            loop {
                let Ok(payload) = handle.recv::<PlacementDragPayload>().await else {
                    break;
                };
                let template_id = sheets_for_placement_drag
                    .peek()
                    .selected_template_id
                    .clone();
                let Some(template_id) = template_id else {
                    continue;
                };
                match payload {
                    PlacementDragPayload::Move { x, y } => {
                        let _ = sheets_for_placement_drag.write().commit_drag_end_move(
                            template_id,
                            placement_id.clone(),
                            x,
                            y,
                        );
                    }
                    PlacementDragPayload::Resize { x, y, w, h } => {
                        let _ = sheets_for_placement_drag.write().commit_drag_end_resize(
                            template_id,
                            placement_id.clone(),
                            TemplateRect { x, y, w, h },
                        );
                    }
                }
            }
        });
    }));
    // Anchor pointer-drag bridge: install when an anchor is selected.
    let mut sheets_for_anchor_drag = sheets;
    let anchor_drag_selection = selected_anchor_id.clone();
    let mut active_anchor_drag_key = use_signal(|| Option::<String>::None);
    use_effect(use_reactive!(|anchor_drag_selection| {
        let previous_key = active_anchor_drag_key.peek().clone();
        let new_key = anchor_drag_selection
            .as_ref()
            .map(|id| format!("{ANCHOR_DRAG_LISTENER_PREFIX}{}", id.as_str()));
        if previous_key == new_key {
            return;
        }

        if let Some(listener_key) = previous_key {
            spawn(async move {
                let _ = document::eval(&cleanup_drag_bridge_script(&listener_key));
            });
        }

        active_anchor_drag_key.set(new_key.clone());

        let Some(anchor_id) = anchor_drag_selection.clone() else {
            return;
        };
        let Some(listener_key) = new_key else {
            return;
        };
        let stage_id = stage_id.to_owned();
        let anchor_id_str = anchor_id.as_str().to_owned();

        spawn(async move {
            let js = install_anchor_drag_bridge(&stage_id, &anchor_id_str, &listener_key);
            let mut handle = document::eval(&js);
            loop {
                let Ok(payload) = handle.recv::<AnchorDragPayload>().await else {
                    break;
                };
                sheets_for_anchor_drag
                    .write()
                    .update_selected_anchor_position(AnchorPosition {
                        x: payload.x.clamp(0.0, 1.0),
                        y: payload.y.clamp(0.0, 1.0),
                    });
            }
        });
    }));
    use_drop(move || {
        let listener_key = active_stage_listener_key.peek().clone();
        let placement_drag_key = active_placement_drag_key.peek().clone();
        let anchor_drag_key = active_anchor_drag_key.peek().clone();
        spawn(async move {
            if let Some(listener_key) = listener_key {
                let _ = document::eval(&cleanup_stage_resize_listener_script(&listener_key));
            }
            if let Some(listener_key) = placement_drag_key {
                let _ = document::eval(&cleanup_drag_bridge_script(&listener_key));
            }
            if let Some(listener_key) = anchor_drag_key {
                let _ = document::eval(&cleanup_drag_bridge_script(&listener_key));
            }
        });
    });
    let handle_stage_click = move |evt: MouseEvent| {
        let coordinates = evt.client_coordinates();
        let rect = *stage_rect.read();
        let Some(position) = normalized_image_point(
            coordinates.x,
            coordinates.y,
            rect.left,
            rect.top,
            rect.width,
            rect.height,
        ) else {
            return;
        };

        match tool {
            SheetTool::Anchor => {
                let target = sheets.read().selected_template().and_then(|template| {
                    placement_at_canvas_point(template, position.x, position.y)
                });
                sheets.write().place_anchor_on(position, target);
            }
            SheetTool::Select => {}
        }
    };

    let handle_stage_dragover = move |evt: Event<DragData>| {
        // preventDefault on dragover is required for the subsequent drop event to fire.
        evt.prevent_default();
        evt.data_transfer().set_drop_effect("copy");
    };

    let drop_template_id = selected_template
        .as_ref()
        .map(|template| template.template_id.clone());
    let handle_stage_drop = move |evt: Event<DragData>| {
        evt.prevent_default();
        let Some(template_id) = drop_template_id.clone() else {
            return;
        };
        let Some(raw_asset_id) = evt.data_transfer().get_data("text/plain") else {
            return;
        };
        if raw_asset_id.is_empty() {
            return;
        }
        let asset_id = AssetId::from_string(raw_asset_id);
        let coordinates = evt.client_coordinates();
        let rect = *stage_rect.read();
        let Some(position) = normalized_image_point(
            coordinates.x,
            coordinates.y,
            rect.left,
            rect.top,
            rect.width,
            rect.height,
        ) else {
            return;
        };
        let pixel_dimensions = sheets
            .read()
            .assets
            .iter()
            .find(|asset| asset.asset_id == asset_id)
            .map(|asset| asset.pixel_dimensions.clone());
        let Some(pixel_dimensions) = pixel_dimensions else {
            return;
        };
        #[allow(
            clippy::cast_possible_truncation,
            reason = "DOM event coordinates arrive as f64 but downstream layout uses f32"
        )]
        let drop_rect = default_rect_at(position.x as f32, position.y as f32, &pixel_dimensions);
        let _ = sheets
            .write()
            .add_placement(template_id, asset_id, drop_rect);
    };

    let pending_slots = sheets.read().pending_preset_slots.clone();
    let placements_empty = selected_template
        .as_ref()
        .is_none_or(|template| template.placements.is_empty());
    let show_empty_dropzone = placements_empty && pending_slots.is_empty() && has_template;
    let empty_dropzone_template_id = selected_template
        .as_ref()
        .map(|template| template.template_id.clone());
    let handle_empty_dropzone_dragover = move |evt: Event<DragData>| {
        evt.prevent_default();
        evt.data_transfer().set_drop_effect("copy");
    };
    let empty_dropzone_template_id_for_drop = empty_dropzone_template_id.clone();
    let handle_empty_dropzone_drop = move |evt: Event<DragData>| {
        evt.stop_propagation();
        evt.prevent_default();
        let Some(template_id) = empty_dropzone_template_id_for_drop.clone() else {
            return;
        };
        let Some(raw_asset_id) = evt.data_transfer().get_data("text/plain") else {
            return;
        };
        if raw_asset_id.is_empty() {
            return;
        }
        let asset_id = AssetId::from_string(raw_asset_id);
        let coordinates = evt.client_coordinates();
        let rect = *stage_rect.read();
        let Some(position) = normalized_image_point(
            coordinates.x,
            coordinates.y,
            rect.left,
            rect.top,
            rect.width,
            rect.height,
        ) else {
            return;
        };
        let pixel_dimensions = sheets
            .read()
            .assets
            .iter()
            .find(|asset| asset.asset_id == asset_id)
            .map(|asset| asset.pixel_dimensions.clone());
        let Some(pixel_dimensions) = pixel_dimensions else {
            return;
        };
        #[allow(
            clippy::cast_possible_truncation,
            reason = "DOM event coordinates arrive as f64 but downstream layout uses f32"
        )]
        let drop_rect = default_rect_at(position.x as f32, position.y as f32, &pixel_dimensions);
        let _ = sheets
            .write()
            .add_placement(template_id, asset_id, drop_rect);
    };

    rsx! {
        main { "data-testid": "sheets-canvas",
            section { "data-testid": "sheets-toolbar",
                div { class: "if-sheets__toolbar-tools",
                    h2 { class: "if-sheets__toolbar-name", "{toolbar_name}" }
                    div { class: "if-sheets__toolbar-segment", role: "tablist",
                        button {
                            "type": "button",
                            "data-active": tool_is_select,
                            onclick: move |_| {
                                sheets.write().set_tool(SheetTool::Select);
                            },
                            "Select"
                        }
                        button {
                            "type": "button",
                            "data-active": tool_is_anchor,
                            onclick: move |_| {
                                sheets.write().set_tool(SheetTool::Anchor);
                            },
                            "Anchor"
                        }
                    }
                    button {
                        "type": "button",
                        class: "if-sheets__toolbar-import",
                        "aria-label": "Import image",
                        disabled: !has_template,
                        title: if has_template { "Import image" } else { "Select a template first" },
                        onclick: move |_| {
                            on_import_image.call(());
                        },
                        Icon { name: IconKind::Plus, size: IconSize::Sm }
                    }
                }
                div {
                    class: "if-sheets__autosave-chip",
                    "data-testid": "sheets-autosave-chip",
                    "data-status": autosave_status_token,
                    "data-tone": autosave_tone,
                    span { class: "if-sheets__autosave-dot" }
                    span { "{autosave_label}" }
                }
            }
            if !has_template {
                div { class: "if-sheets__canvas-empty",
                    h2 { "No template selected" }
                    p { "Create or select a template from the Templates tab." }
                }
            } else if let Some(template) = selected_template {
                section {
                    class: "if-sheets__canvas-panel",
                    "aria-label": "Template canvas",
                    if selected_asset_missing {
                        div { class: "if-sheets__image-status",
                            p { "Missing image asset" }
                            if let Some(copied_path) = missing_copied_path {
                                p {
                                    span { "Copied path" }
                                    code { "{copied_path}" }
                                }
                            }
                            if let Some(original_path) = missing_original_path {
                                p {
                                    span { "Original import path" }
                                    code { "{original_path}" }
                                }
                            }
                            p { "{anchor_count_label}" }
                        }
                    } else {
                        div {
                            id: "{stage_id}",
                            class: "if-sheets__image-stage",
                            "data-testid": "sheets-image-stage",
                            "data-asset-id": "{asset_id}",
                            onclick: handle_stage_click,
                            ondragover: handle_stage_dragover,
                            ondrop: handle_stage_drop,
                            for placement in placements_sorted.iter().cloned() {
                                {
                                    let health = asset_health_all
                                        .iter()
                                        .find(|h| h.entry.asset_id == placement.asset_id);
                                    let placement_src = health
                                        .filter(|h| !h.missing)
                                        .and_then(|h| asset_data_url(h).ok())
                                        .unwrap_or_default();
                                    let placement_source_path = health
                                        .filter(|h| !h.missing)
                                        .map(|h| file_url_from_path(&h.copied_absolute_path))
                                        .unwrap_or_default();
                                    let left = placement.position.x * 100.0;
                                    let top = placement.position.y * 100.0;
                                    let width = placement.position.w * 100.0;
                                    let height = placement.position.h * 100.0;
                                    let z = placement.z_index;
                                    let is_selected = selected_placement_id.as_ref()
                                        == Some(&placement.placement_id);
                                    let placement_filename = is_selected
                                        .then(|| {
                                            assets_all
                                                .iter()
                                                .find(|a| a.asset_id == placement.asset_id)
                                                .map(crate::frame::sheets::state::filename_of)
                                        })
                                        .flatten();
                                    let placement_id_for_click = placement.placement_id.clone();
                                    rsx! {
                                        div {
                                            class: "if-sheets__placement",
                                            "data-placement-id": "{placement.placement_id}",
                                            "data-selected": is_selected.then_some("true"),
                                            "data-drag-bridge": is_selected.then_some("ready"),
                                            style: "left:{left}%;top:{top}%;width:{width}%;height:{height}%;z-index:{z};",
                                            onclick: move |evt| {
                                                if !matches!(tool, SheetTool::Select) {
                                                    return;
                                                }
                                                evt.stop_propagation();
                                                sheets.write().select_placement(placement_id_for_click.clone());
                                            },
                                            if !placement_src.is_empty() {
                                                img {
                                                    class: "if-sheets__placement-image",
                                                    src: "{placement_src}",
                                                    "data-source-path": "{placement_source_path}",
                                                    draggable: "false",
                                                    onload: move |_| {
                                                        image_load_failed.set(false);
                                                    },
                                                    onerror: move |_| {
                                                        image_load_failed.set(true);
                                                    },
                                                }
                                            }
                                            if let Some(name) = placement_filename {
                                                span { class: "if-sheets__placement-name", "{name}" }
                                            }
                                            if is_selected {
                                                for token in ["nw", "n", "ne", "e", "se", "s", "sw", "w"] {
                                                    div {
                                                        class: "if-sheets__resize-handle if-sheets__resize-handle--{token}",
                                                        "data-handle": "{token}",
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if show_empty_dropzone {
                                div {
                                    class: "if-sheets__empty-dropzone",
                                    "data-testid": "sheets-empty-dropzone",
                                    ondragover: handle_empty_dropzone_dragover,
                                    ondrop: handle_empty_dropzone_drop,
                                    p { "No images yet. Drop an asset from the rail or click + to import." }
                                }
                            }
                            for slot in pending_slots.iter().copied() {
                                {
                                    let slot_rect = slot;
                                    let template_id_for_drop = empty_dropzone_template_id.clone();
                                    let left = slot.x * 100.0;
                                    let top = slot.y * 100.0;
                                    let width = slot.w * 100.0;
                                    let height = slot.h * 100.0;
                                    rsx! {
                                        div {
                                            class: "if-sheets__preset-slot",
                                            "data-testid": "sheets-preset-slot",
                                            style: "left:{left}%;top:{top}%;width:{width}%;height:{height}%;",
                                            ondragover: move |evt| evt.prevent_default(),
                                            ondrop: move |evt| {
                                                evt.stop_propagation();
                                                evt.prevent_default();
                                                let Some(template_id) = template_id_for_drop.clone() else {
                                                    return;
                                                };
                                                let Some(raw_asset_id) = evt.data_transfer().get_data("text/plain") else {
                                                    return;
                                                };
                                                if raw_asset_id.is_empty() {
                                                    return;
                                                }
                                                let asset_id = AssetId::from_string(raw_asset_id);
                                                let mut state = sheets.write();
                                                if state
                                                    .add_placement(template_id, asset_id, slot_rect)
                                                    .is_ok()
                                                {
                                                    state.pending_preset_slots.retain(|rect| rect != &slot_rect);
                                                }
                                            },
                                            span { "+ Add image" }
                                        }
                                    }
                                }
                            }
                            for anchor in template.anchors.iter().cloned() {
                                {
                                    let binding = template
                                        .default_anchor_bindings
                                        .iter()
                                        .find(|b| b.anchor_id == anchor.anchor_id);
                                    #[allow(
                                        clippy::match_same_arms,
                                        reason = "explicit arms keep each AnchorAssignment variant visible"
                                    )]
                                    let shape_class = match binding.map(|b| b.assignment) {
                                        None => "if-sheets__anchor--unassigned",
                                        Some(AnchorAssignment::Captured) => {
                                            "if-sheets__anchor--captured"
                                        }
                                        Some(AnchorAssignment::Manual) => {
                                            "if-sheets__anchor--manual"
                                        }
                                        Some(AnchorAssignment::Unavailable) => {
                                            "if-sheets__anchor--unassigned"
                                        }
                                    };
                                    let selected =
                                        selected_anchor_id.as_ref() == Some(&anchor.anchor_id);
                                    let selected_class = if selected {
                                        " if-sheets__anchor--selected"
                                    } else {
                                        ""
                                    };
                                    let (wrapper_x, wrapper_y) =
                                        anchor_canvas_coords(&anchor, &template.placements);
                                    let left = wrapper_x * 100.0;
                                    let top = wrapper_y * 100.0;
                                    let anchor_id_for_click = anchor.anchor_id.clone();
                                    rsx! {
                                        div {
                                            class: "if-sheets__anchor-mount",
                                            "data-anchor-id": "{anchor.anchor_id}",
                                            style: "left:{left}%;top:{top}%;",
                                            button {
                                                "type": "button",
                                                class: "if-sheets__anchor {shape_class}{selected_class}",
                                                "data-anchor-id": "{anchor.anchor_id}",
                                                "aria-pressed": if selected { "true" } else { "false" },
                                                onclick: move |evt| {
                                                    evt.stop_propagation();
                                                    sheets.write().select_anchor(anchor_id_for_click.clone());
                                                },
                                                "Anchor"
                                            }
                                            span { class: "if-sheets__anchor-label", "{anchor.label}" }
                                        }
                                    }
                                }
                            }
                        }
                        if image_load_failed_now {
                            div { class: "if-sheets__image-status",
                                p { "Image failed to load" }
                                if let Some(error) = asset_read_error {
                                    p {
                                        span { "Load error" }
                                        code { "{error}" }
                                    }
                                }
                                if !asset_src.is_empty() {
                                    p {
                                        span { "Image source" }
                                        code { "{asset_src}" }
                                    }
                                }
                                p { "{anchor_count_label}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub(super) fn file_url_from_path(path: &Path) -> String {
    let path = encode_file_url_path(&path.to_string_lossy().replace('\\', "/"));

    if path.starts_with("//") {
        format!("file:{path}")
    } else if path.starts_with('/') {
        format!("file://{path}")
    } else {
        format!("file:///{path}")
    }
}

pub(crate) fn anchor_canvas_coords(
    anchor: &inputforge_core::sheet::TemplateAnchor,
    placements: &[inputforge_core::sheet::AssetPlacement],
) -> (f64, f64) {
    let Some(parent_id) = anchor.attached_to.as_ref() else {
        return (anchor.position.x, anchor.position.y);
    };
    let Some(parent) = placements.iter().find(|p| &p.placement_id == parent_id) else {
        return (anchor.position.x, anchor.position.y);
    };
    let rect = parent.position;
    (
        f64::from(rect.x) + anchor.position.x * f64::from(rect.w),
        f64::from(rect.y) + anchor.position.y * f64::from(rect.h),
    )
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "canvas coordinates arrive as f64 but template rects are stored as f32"
)]
pub(crate) fn placement_at_canvas_point(
    template: &inputforge_core::sheet::DeviceTemplate,
    canvas_x: f64,
    canvas_y: f64,
) -> Option<inputforge_core::sheet::AssetPlacementId> {
    template
        .placements
        .iter()
        .filter(|p| {
            let rect = p.position;
            let x = canvas_x as f32;
            let y = canvas_y as f32;
            x >= rect.x && x <= rect.x + rect.w && y >= rect.y && y <= rect.y + rect.h
        })
        .max_by_key(|p| p.z_index)
        .map(|p| p.placement_id.clone())
}

fn asset_data_url(health: &AssetHealth) -> std::io::Result<String> {
    let bytes = std::fs::read(&health.copied_absolute_path)?;
    Ok(format!(
        "{}{}",
        image_data_url_prefix(&health.entry.media_type),
        base64_encode(&bytes)
    ))
}

fn image_data_url_prefix(media_type: &str) -> String {
    format!("data:{media_type};base64,")
}

pub(super) fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);

        encoded.push(TABLE[(first >> 2) as usize] as char);
        encoded.push(TABLE[(((first & 0b0000_0011) << 4) | (second >> 4)) as usize] as char);
        encoded.push(if chunk.len() > 1 {
            TABLE[(((second & 0b0000_1111) << 2) | (third >> 6)) as usize] as char
        } else {
            '='
        });
        encoded.push(if chunk.len() > 2 {
            TABLE[(third & 0b0011_1111) as usize] as char
        } else {
            '='
        });
    }

    encoded
}

pub(super) fn stage_subscription_key(
    has_template: bool,
    selected_asset_missing: bool,
    stage_identity: &str,
) -> Option<String> {
    (has_template && !selected_asset_missing && !stage_identity.is_empty())
        .then(|| stage_identity.to_owned())
}

fn stage_resize_listener_key(stage_key: &str) -> String {
    format!("{STAGE_RESIZE_LISTENER_PREFIX}{stage_key}")
}

pub(super) fn install_stage_resize_listener_script(stage_id: &str, listener_key: &str) -> String {
    format!(
        r"
        var stage = document.getElementById({stage_id:?});
        if (!stage) return;
        var listenerKey = {listener_key:?};
        var existing = window[listenerKey];
        if (existing) {{
            window.removeEventListener('resize', existing.sendRect || existing);
            if (existing.observer) {{
                existing.observer.disconnect();
            }}
            delete window[listenerKey];
        }}
        var sendRect = function() {{
            var r = stage.getBoundingClientRect();
            dioxus.send({{
                left: r.left,
                top: r.top,
                width: r.width,
                height: r.height
            }});
        }};
        var observer = new ResizeObserver(sendRect);
        observer.observe(stage);
        window[listenerKey] = {{
            sendRect: sendRect,
            observer: observer
        }};
        sendRect();
        window.addEventListener('resize', sendRect, {{ passive: true }});
        "
    )
}

pub(super) fn cleanup_stage_resize_listener_script(listener_key: &str) -> String {
    format!(
        "const h = window[{listener_key:?}];\
         if (h) {{\
           window.removeEventListener('resize', h.sendRect || h);\
           if (h.observer) {{\
             h.observer.disconnect();\
           }}\
           delete window[{listener_key:?}];\
         }}"
    )
}

#[allow(
    clippy::too_many_lines,
    reason = "the function body is a single large JS literal; splitting it would obscure the script"
)]
pub(super) fn install_placement_drag_bridge(
    stage_id: &str,
    placement_id: &str,
    listener_key: &str,
) -> String {
    format!(
        r"
        var stage = document.getElementById({stage_id:?});
        if (!stage) return;
        var placementId = {placement_id:?};
        var placement = stage.querySelector('[data-placement-id=' + JSON.stringify(placementId) + ']');
        if (!placement) return;
        var listenerKey = {listener_key:?};
        var existing = window[listenerKey];
        if (existing && typeof existing.detach === 'function') {{
            existing.detach();
            delete window[listenerKey];
        }}

        var DRAG_THRESHOLD_PX = 3;
        var state = null;
        var handles = placement.querySelectorAll('.if-sheets__resize-handle');

        function stageRect() {{
            return stage.getBoundingClientRect();
        }}

        function placementRectNormalized() {{
            var rect = stageRect();
            var box = placement.getBoundingClientRect();
            if (rect.width === 0 || rect.height === 0) {{
                return {{ x: 0, y: 0, w: 0, h: 0 }};
            }}
            return {{
                x: (box.left - rect.left) / rect.width,
                y: (box.top - rect.top) / rect.height,
                w: box.width / rect.width,
                h: box.height / rect.height
            }};
        }}

        function onPointerDownBody(evt) {{
            if (evt.button !== 0) return;
            if (state) return;
            if (evt.target && evt.target.classList && evt.target.classList.contains('if-sheets__resize-handle')) return;
            evt.preventDefault();
            if (placement.setPointerCapture) {{
                try {{ placement.setPointerCapture(evt.pointerId); }} catch (e) {{}}
            }}
            state = {{
                kind: 'move',
                target: placement,
                pointerId: evt.pointerId,
                startX: evt.clientX,
                startY: evt.clientY,
                origin: placementRectNormalized(),
                moved: false
            }};
        }}

        function onPointerDownHandle(handle) {{
            return function(evt) {{
                if (evt.button !== 0) return;
                if (state) return;
                evt.preventDefault();
                evt.stopPropagation();
                if (handle.setPointerCapture) {{
                    try {{ handle.setPointerCapture(evt.pointerId); }} catch (e) {{}}
                }}
                state = {{
                    kind: 'resize',
                    handle: handle.dataset.handle || '',
                    target: handle,
                    pointerId: evt.pointerId,
                    startX: evt.clientX,
                    startY: evt.clientY,
                    origin: placementRectNormalized(),
                    moved: false
                }};
            }};
        }}

        function applyLive(rect, dxNorm, dyNorm) {{
            // Mutate inline style during the drag so the user sees the frame track the cursor.
            // The pointerup commit re-renders through Dioxus, which writes the same style attribute
            // with the canonical values and naturally replaces these temporary strings.
            if (state.kind === 'move') {{
                placement.style.left = ((state.origin.x + dxNorm) * 100) + '%';
                placement.style.top = ((state.origin.y + dyNorm) * 100) + '%';
            }} else {{
                var nx = state.origin.x;
                var ny = state.origin.y;
                var nw = state.origin.w;
                var nh = state.origin.h;
                var h = state.handle;
                if (h.indexOf('e') !== -1) {{ nw = state.origin.w + dxNorm; }}
                if (h.indexOf('w') !== -1) {{ nx = state.origin.x + dxNorm; nw = state.origin.w - dxNorm; }}
                if (h.indexOf('s') !== -1) {{ nh = state.origin.h + dyNorm; }}
                if (h.indexOf('n') !== -1) {{ ny = state.origin.y + dyNorm; nh = state.origin.h - dyNorm; }}
                placement.style.left = (nx * 100) + '%';
                placement.style.top = (ny * 100) + '%';
                placement.style.width = (nw * 100) + '%';
                placement.style.height = (nh * 100) + '%';
            }}
        }}

        function onPointerMove(evt) {{
            if (!state || evt.pointerId !== state.pointerId) return;
            var dx = evt.clientX - state.startX;
            var dy = evt.clientY - state.startY;
            if (Math.abs(dx) >= DRAG_THRESHOLD_PX || Math.abs(dy) >= DRAG_THRESHOLD_PX) {{
                state.moved = true;
            }}
            if (!state.moved) return;
            var rect = stageRect();
            if (rect.width === 0 || rect.height === 0) return;
            applyLive(rect, dx / rect.width, dy / rect.height);
        }}

        function commit(evt) {{
            if (!state || evt.pointerId !== state.pointerId) return;
            if (!state.moved) {{
                state = null;
                return;
            }}
            var rect = stageRect();
            if (rect.width === 0 || rect.height === 0) {{
                state = null;
                return;
            }}
            var dxNorm = (evt.clientX - state.startX) / rect.width;
            var dyNorm = (evt.clientY - state.startY) / rect.height;
            try {{
                if (state.kind === 'move') {{
                    var nx = state.origin.x + dxNorm;
                    var ny = state.origin.y + dyNorm;
                    dioxus.send({{ mode: 'move', x: nx, y: ny }});
                }} else {{
                    var newX = state.origin.x;
                    var newY = state.origin.y;
                    var newW = state.origin.w;
                    var newH = state.origin.h;
                    var h = state.handle;
                    if (h.indexOf('e') !== -1) {{ newW = state.origin.w + dxNorm; }}
                    if (h.indexOf('w') !== -1) {{ newX = state.origin.x + dxNorm; newW = state.origin.w - dxNorm; }}
                    if (h.indexOf('s') !== -1) {{ newH = state.origin.h + dyNorm; }}
                    if (h.indexOf('n') !== -1) {{ newY = state.origin.y + dyNorm; newH = state.origin.h - dyNorm; }}
                    dioxus.send({{ mode: 'resize', x: newX, y: newY, w: newW, h: newH }});
                }}
            }} catch (e) {{
                // Bridge channel closed or unavailable; abort silently.
            }}
            state = null;
        }}

        function cancel(evt) {{
            if (!state) return;
            if (evt && evt.pointerId !== undefined && evt.pointerId !== state.pointerId) return;
            state = null;
        }}

        placement.addEventListener('pointerdown', onPointerDownBody);
        window.addEventListener('pointermove', onPointerMove);
        window.addEventListener('pointerup', commit);
        window.addEventListener('pointercancel', cancel);
        window.addEventListener('blur', cancel);

        var handleListeners = [];
        handles.forEach(function(h) {{
            var fn = onPointerDownHandle(h);
            h.addEventListener('pointerdown', fn);
            handleListeners.push({{ el: h, fn: fn }});
        }});

        window[listenerKey] = {{
            detach: function() {{
                placement.removeEventListener('pointerdown', onPointerDownBody);
                window.removeEventListener('pointermove', onPointerMove);
                window.removeEventListener('pointerup', commit);
                window.removeEventListener('pointercancel', cancel);
                window.removeEventListener('blur', cancel);
                handleListeners.forEach(function(rec) {{
                    rec.el.removeEventListener('pointerdown', rec.fn);
                }});
                state = null;
            }}
        }};
        "
    )
}

pub(super) fn install_anchor_drag_bridge(
    stage_id: &str,
    anchor_id: &str,
    listener_key: &str,
) -> String {
    format!(
        r"
        var stage = document.getElementById({stage_id:?});
        if (!stage) return;
        var anchorId = {anchor_id:?};
        var anchor = stage.querySelector('button[data-anchor-id=' + JSON.stringify(anchorId) + ']');
        if (!anchor) return;
        var mount = stage.querySelector('.if-sheets__anchor-mount[data-anchor-id=' + JSON.stringify(anchorId) + ']');
        if (!mount) return;
        var listenerKey = {listener_key:?};
        var existing = window[listenerKey];
        if (existing && typeof existing.detach === 'function') {{
            existing.detach();
            delete window[listenerKey];
        }}

        var DRAG_THRESHOLD_PX = 3;
        var state = null;

        function stageRect() {{
            return stage.getBoundingClientRect();
        }}

        function onPointerDown(evt) {{
            if (evt.button !== 0) return;
            if (state) return;
            evt.preventDefault();
            evt.stopPropagation();
            if (anchor.setPointerCapture) {{
                try {{ anchor.setPointerCapture(evt.pointerId); }} catch (e) {{}}
            }}
            state = {{ pointerId: evt.pointerId, startX: evt.clientX, startY: evt.clientY, moved: false }};
        }}

        function onPointerMove(evt) {{
            if (!state || evt.pointerId !== state.pointerId) return;
            var dx = evt.clientX - state.startX;
            var dy = evt.clientY - state.startY;
            if (Math.abs(dx) >= DRAG_THRESHOLD_PX || Math.abs(dy) >= DRAG_THRESHOLD_PX) {{
                state.moved = true;
            }}
            if (!state.moved) return;
            var rect = stageRect();
            if (rect.width === 0 || rect.height === 0) return;
            // Live-update the anchor mount so the dot tracks the cursor during the drag.
            // The pointerup commit re-renders through Dioxus, which writes the canonical style.
            var nx = (evt.clientX - rect.left) / rect.width;
            var ny = (evt.clientY - rect.top) / rect.height;
            mount.style.left = (nx * 100) + '%';
            mount.style.top = (ny * 100) + '%';
        }}

        function commit(evt) {{
            if (!state || evt.pointerId !== state.pointerId) return;
            if (!state.moved) {{
                state = null;
                return;
            }}
            var rect = stageRect();
            if (rect.width === 0 || rect.height === 0) {{
                state = null;
                return;
            }}
            var nx = (evt.clientX - rect.left) / rect.width;
            var ny = (evt.clientY - rect.top) / rect.height;
            try {{
                dioxus.send({{ x: nx, y: ny }});
            }} catch (e) {{
                // Bridge channel closed or unavailable; abort silently.
            }}
            state = null;
        }}

        function cancel(evt) {{
            if (!state) return;
            if (evt && evt.pointerId !== undefined && evt.pointerId !== state.pointerId) return;
            state = null;
        }}

        anchor.addEventListener('pointerdown', onPointerDown);
        window.addEventListener('pointermove', onPointerMove);
        window.addEventListener('pointerup', commit);
        window.addEventListener('pointercancel', cancel);
        window.addEventListener('blur', cancel);

        window[listenerKey] = {{
            detach: function() {{
                anchor.removeEventListener('pointerdown', onPointerDown);
                window.removeEventListener('pointermove', onPointerMove);
                window.removeEventListener('pointerup', commit);
                window.removeEventListener('pointercancel', cancel);
                window.removeEventListener('blur', cancel);
                state = null;
            }}
        }};
        "
    )
}

pub(super) fn cleanup_drag_bridge_script(listener_key: &str) -> String {
    format!(
        "const h = window[{listener_key:?}];\
         if (h && typeof h.detach === 'function') {{\
           h.detach();\
           delete window[{listener_key:?}];\
         }}"
    )
}

fn encode_file_url_path(path: &str) -> String {
    let mut encoded = String::with_capacity(path.len());

    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' | b':' => {
                encoded.push(byte as char);
            }
            _ => {
                let _ = write!(encoded, "%{byte:02X}");
            }
        }
    }

    encoded
}
