#![expect(
    unused_qualifications,
    reason = "rsx! macro expansion triggers false-positive unused_qualifications warnings on onclick:"
)]

use dioxus::prelude::*;
use inputforge_core::sheet::{AssetId, TemplateId};

use crate::frame::sheets::state::SheetsState;

#[component]
pub(crate) fn SheetsLeftRail(
    sheets: Signal<SheetsState>,
    on_import_image: EventHandler<()>,
) -> Element {
    let snapshot = sheets.read();
    let templates = snapshot.templates.clone();
    let assets = snapshot.assets.clone();
    let selected_template_id = snapshot.selected_template_id.clone();
    drop(snapshot);

    let template_count = templates.len();
    let asset_count = assets.len();
    let template_assets: Vec<(TemplateId, Vec<AssetId>)> = templates
        .iter()
        .map(|template| (template.template_id.clone(), template.asset_ids.clone()))
        .collect();
    let handle_import_image = move |_| on_import_image.call(());

    rsx! {
        aside { "data-testid": "sheets-left-rail",
            nav { "aria-label": "Sheet asset library",
                button { "type": "button", "Templates" }
                button { "type": "button", "Assets" }
            }
            section { "data-testid": "sheets-template-section",
                h2 { "Templates" }
                p { "{template_count} templates" }
                button {
                    "type": "button",
                    onclick: handle_import_image,
                    "Import image"
                }
                if !templates.is_empty() {
                    ul {
                        for template in templates {
                            {
                                let anchor_count = template.anchors.len();
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
                                            span { "{anchor_count} anchors" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            section { "data-testid": "sheets-asset-section",
                h2 { "Assets" }
                p { "{asset_count} assets" }
                ul {
                    for asset in assets {
                        {
                            let selected = selected_template_id.as_ref().is_some_and(|template_id| {
                                template_assets.iter().any(|(candidate_template_id, asset_ids)| {
                                    candidate_template_id == template_id
                                        && asset_ids.iter().any(|id| id == &asset.asset_id)
                                })
                            });
                            let can_select = template_assets
                                .iter()
                                .any(|(_, asset_ids)| asset_ids.iter().any(|id| id == &asset.asset_id));
                            let asset_label = asset.copied_path.display().to_string();
                            rsx! {
                        li { key: "{asset.asset_id}",
                            "data-selected": selected,
                            if can_select {
                                button {
                                    "type": "button",
                                    "aria-label": "Select first template using {asset_label}",
                                    onclick: move |_| {
                                        sheets.write().select_first_template_for_asset(asset.asset_id.clone());
                                    },
                                    span { "{asset_label}" }
                                    span {
                                        "{asset.pixel_dimensions.width} x {asset.pixel_dimensions.height}"
                                    }
                                }
                            } else {
                                span { "{asset_label}" }
                                span {
                                    "{asset.pixel_dimensions.width} x {asset.pixel_dimensions.height}"
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
