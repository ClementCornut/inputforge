#![expect(
    unused_qualifications,
    reason = "rsx! macro expansion triggers false-positive unused_qualifications warnings on onclick:"
)]

use dioxus::prelude::*;

use crate::frame::sheets::state::{
    SheetLayoutPreset, SheetsLibraryTab, SheetsState, asset_label_for, filename_of, pluralize,
};

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
    // Track which row the user clicked in the rail (not the canvas-tracking
    // `selected_asset_id`, which `select_template` writes for the canvas image
    // renderer and would falsely highlight an asset row whenever a template is
    // picked).
    let selected_asset_id = snapshot.inspector_asset_id.clone();
    // The Templates-tab row highlight follows the inspector branch (anchor >
    // placement > asset > template). The state setter invariants keep these
    // four fields mutually exclusive, so the row only highlights when none of
    // the more-specific selections are active.
    let template_branch_active = snapshot.selected_anchor_id.is_none()
        && snapshot.selected_placement_id.is_none()
        && snapshot.inspector_asset_id.is_none();
    let has_selected_template = snapshot.selected_template_id.is_some();
    drop(snapshot);

    let template_count = templates.len();
    let asset_count = assets.len();
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
                                    let selected = template_branch_active
                                        && selected_template_id
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
                                    let label = asset_label_for(&asset);
                                    let filename_label = filename_of(&asset);
                                    let pixel_width = asset.pixel_dimensions.width;
                                    let pixel_height = asset.pixel_dimensions.height;
                                    let asset_id_str = asset.asset_id.as_str().to_owned();
                                    let row_asset_id = asset.asset_id.clone();
                                    let click_asset_id = asset.asset_id.clone();
                                    let dragstart_asset_id = asset.asset_id.clone();
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
                                                // WebView2/Firefox incantation: the source's dataTransfer must carry
                                                // some payload or the drag is silently aborted. dioxus-html 0.7.9
                                                // desktop's set_data is a no-op stub, so the asset id can't ride the
                                                // dataTransfer; we stash it on SheetsState instead and the drop
                                                // handlers read from there. Mirrors components/sortable/handle.rs.
                                                let _ = evt.data_transfer().set_data("text/html", "");
                                                evt.data_transfer().set_effect_allowed("copy");
                                                sheets.write().begin_asset_drag(dragstart_asset_id.clone());
                                            },
                                            ondragend: move |_evt| {
                                                sheets.write().end_asset_drag();
                                            },
                                            button {
                                                "type": "button",
                                                "aria-label": "Inspect asset {filename_label}",
                                                onclick: move |_| {
                                                    sheets.write().select_asset_for_inspector(click_asset_id.clone());
                                                },
                                                span { "{label}" }
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
