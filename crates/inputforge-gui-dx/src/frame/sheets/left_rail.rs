#![expect(
    unused_qualifications,
    reason = "rsx! macro expansion triggers false-positive unused_qualifications warnings on onclick:"
)]

use dioxus::prelude::*;
use inputforge_core::sheet::{AssetEntry, AssetId, TemplateId};

use crate::frame::sheets::state::{SheetLayoutPreset, SheetsLibraryTab, SheetsState, pluralize};

fn filename_of(asset: &AssetEntry) -> String {
    asset
        .original_import_path
        .as_ref()
        .and_then(|p| p.file_name())
        .or_else(|| asset.copied_path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| asset.copied_path.display().to_string())
}

const PRESET_OPTIONS: &[(&str, SheetLayoutPreset)] = &[
    ("Single", SheetLayoutPreset::Single),
    ("Horizontal pair", SheetLayoutPreset::HorizontalPair),
    ("Vertical stack", SheetLayoutPreset::VerticalStack),
    ("2x2 grid", SheetLayoutPreset::TwoByTwo),
    ("Freeform", SheetLayoutPreset::Freeform),
];

#[component]
pub(crate) fn SheetsLeftRail(
    sheets: Signal<SheetsState>,
    on_import_image: EventHandler<()>,
    #[allow(
        unused_variables,
        reason = "kept for backward compat; replaced by inline picker"
    )]
    on_create_template: EventHandler<()>,
) -> Element {
    let snapshot = sheets.read();
    let templates = snapshot.templates.clone();
    let assets = snapshot.assets.clone();
    let library_tab = snapshot.library_tab;
    let preset_picker_open = snapshot.preset_picker_open;
    let selected_template_id = snapshot.selected_template_id.clone();
    let selected_asset_id = snapshot.selected_asset_id().cloned();
    let has_selected_template = snapshot.selected_template_id.is_some();
    drop(snapshot);

    let template_count = templates.len();
    let asset_count = assets.len();
    let template_assets: Vec<(TemplateId, Vec<AssetId>)> = templates
        .iter()
        .map(|template| {
            (
                template.template_id.clone(),
                template
                    .placements
                    .iter()
                    .map(|p| p.asset_id.clone())
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    let handle_import_image = move |_| on_import_image.call(());
    let templates_selected = library_tab == SheetsLibraryTab::Templates;
    let assets_selected = library_tab == SheetsLibraryTab::Assets;

    rsx! {
        aside { "data-testid": "sheets-left-rail",
            nav { "aria-label": "Sheet asset library",
                button {
                    "type": "button",
                    "aria-selected": templates_selected,
                    "data-active": templates_selected,
                    onclick: move |_| {
                        sheets.write().library_tab = SheetsLibraryTab::Templates;
                    },
                    "Templates"
                }
                button {
                    "type": "button",
                    "aria-selected": assets_selected,
                    "data-active": assets_selected,
                    onclick: move |_| {
                        sheets.write().library_tab = SheetsLibraryTab::Assets;
                    },
                    "Assets"
                }
            }
            if templates_selected {
                section { "data-testid": "sheets-template-section",
                    h2 { "Templates" }
                    p { "{template_count} templates" }
                    if templates.is_empty() {
                        div { class: "if-sheets__rail-empty",
                            p { "No templates" }
                            button {
                                "type": "button",
                                onclick: move |_| {
                                    sheets.write().open_template_preset_picker();
                                },
                                "New template"
                            }
                        }
                    } else {
                        button {
                            "type": "button",
                            onclick: move |_| {
                                sheets.write().open_template_preset_picker();
                            },
                            "New template"
                        }
                        ul {
                            for template in templates {
                                {
                                    let frame_count = template.placements.len();
                                    let anchor_count = template.anchors.len();
                                    let frame_label = pluralize(frame_count, "frame", "frames");
                                    let anchor_label = pluralize(anchor_count, "anchor", "anchors");
                                    let selected = selected_template_id
                                        .as_ref()
                                        .is_some_and(|template_id| template_id == &template.template_id);
                                    rsx! {
                                        li {
                                            key: "{template.template_id}",
                                            "data-selected": selected,
                                            button {
                                                "type": "button",
                                                "aria-label": "Select template {template.display_name}",
                                                onclick: move |_| {
                                                    sheets.write().select_template(template.template_id.clone());
                                                },
                                                span { "{template.display_name}" }
                                                span { "{frame_label}" }
                                                span { "{anchor_label}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if preset_picker_open {
                        section { "data-testid": "sheets-preset-picker",
                            h3 { "Pick a layout" }
                            ul {
                                for (preset_label , preset) in PRESET_OPTIONS.iter().copied() {
                                    li { key: "{preset_label}",
                                        button {
                                            "type": "button",
                                            onclick: move |_| {
                                                sheets.write().apply_preset_after_create(preset);
                                            },
                                            "{preset_label}"
                                        }
                                    }
                                }
                            }
                            button {
                                "type": "button",
                                "data-action": "skip",
                                onclick: move |_| {
                                    sheets.write().skip_template_preset_picker();
                                },
                                "Skip"
                            }
                            button {
                                "type": "button",
                                "data-action": "dismiss",
                                "aria-label": "Close picker without creating a template",
                                onclick: move |_| {
                                    sheets.write().dismiss_template_preset_picker();
                                },
                                "Cancel"
                            }
                        }
                    }
                }
            }
            if assets_selected {
                section { "data-testid": "sheets-asset-section",
                    h2 { "Assets" }
                    p { "{asset_count} assets" }
                    if assets.is_empty() {
                        div { class: "if-sheets__rail-empty",
                            p { "No background images" }
                            button {
                                "type": "button",
                                onclick: handle_import_image,
                                "Import image"
                            }
                        }
                    } else {
                        button {
                            "type": "button",
                            onclick: handle_import_image,
                            "Import image"
                        }
                        ul {
                            for asset in assets {
                                {
                                    let selected = selected_asset_id
                                        .as_ref()
                                        .is_some_and(|asset_id| asset_id == &asset.asset_id);
                                    let can_select = template_assets
                                        .iter()
                                        .any(|(_, asset_ids)| asset_ids.iter().any(|id| id == &asset.asset_id));
                                    let asset_label = asset.copied_path.display().to_string();
                                    let filename_label = filename_of(&asset);
                                    let pixel_width = asset.pixel_dimensions.width;
                                    let pixel_height = asset.pixel_dimensions.height;
                                    let asset_id_str = asset.asset_id.as_str().to_owned();
                                    let row_asset_id = asset.asset_id.clone();
                                    let click_asset_id = asset.asset_id.clone();
                                    let dragstart_asset_id = asset.asset_id;
                                    let draggable_value = if has_selected_template {
                                        "true"
                                    } else {
                                        "false"
                                    };
                                    rsx! {
                                        li { key: "{row_asset_id}",
                                            "data-selected": selected,
                                            draggable: draggable_value,
                                            "data-asset-id": "{asset_id_str}",
                                            ondragstart: move |evt| {
                                                let _ = evt
                                                    .data_transfer()
                                                    .set_data("text/plain", dragstart_asset_id.as_str());
                                                evt.data_transfer().set_effect_allowed("copy");
                                            },
                                            if can_select {
                                                button {
                                                    "type": "button",
                                                    "aria-label": "Select first template using {asset_label}",
                                                    onclick: move |_| {
                                                        sheets.write().select_first_template_for_asset(click_asset_id.clone());
                                                    },
                                                    span { "{filename_label}" }
                                                    span { "{pixel_width} x {pixel_height}" }
                                                }
                                            } else {
                                                span { "{filename_label}" }
                                                span { "{pixel_width} x {pixel_height}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
