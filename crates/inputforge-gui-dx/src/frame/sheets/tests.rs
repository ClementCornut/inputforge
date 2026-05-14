use std::path::PathBuf;

use dioxus::prelude::*;
use inputforge_core::sheet::{AssetEntry, AssetId, ExtensionPayload, PixelDimensions, TokenPreset};

use super::left_rail::SheetsLeftRail;
use super::SheetsWorkbench;
use super::state::{AutosaveStatus, SheetsState};

#[test]
fn empty_sheets_workbench_exposes_import_create_path() {
    let html = dioxus_ssr::render_element(rsx! {
        SheetsWorkbench {}
    });

    assert!(html.contains("data-testid=\"sheets-workbench\""));
    assert!(html.contains("Import image"));
    assert!(html.contains("Templates"));
    assert!(html.contains("Assets"));
}

#[test]
fn sheets_left_rail_keeps_import_create_path_with_existing_template() {
    #[expect(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    fn Harness() -> Element {
        let mut initial_state = SheetsState::default();
        initial_state.create_template_from_asset(
            AssetId::from_string("asset-1"),
            "Arcade panel",
        );
        let sheets = use_signal(|| initial_state);

        rsx! {
            SheetsLeftRail {
                sheets,
                on_import_image: move |()| {},
            }
        }
    }

    let mut vdom = VirtualDom::new(Harness);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert!(html.contains("Arcade panel"));
    assert!(
        html.contains("Import image"),
        "template rail should keep import available when templates exist: {html}"
    );
}

#[test]
fn creating_template_from_asset_selects_template_and_marks_dirty() {
    let asset_id = AssetId::from_string("asset-1");
    let asset = AssetEntry {
        asset_id: asset_id.clone(),
        copied_path: PathBuf::from("assets/asset-1.png"),
        content_hash: "hash".to_owned(),
        media_type: "image/png".to_owned(),
        pixel_dimensions: PixelDimensions {
            width: 320,
            height: 240,
        },
        original_import_path: None,
        extensions: ExtensionPayload::default(),
    };
    let mut state = SheetsState {
        assets: vec![asset],
        autosave: AutosaveStatus::Clean,
        ..SheetsState::default()
    };

    let template_id = state.create_template_from_asset(asset_id.clone(), "Arcade panel");

    assert_eq!(state.templates.len(), 1);
    assert_eq!(state.selected_template_id, Some(template_id.clone()));
    assert_eq!(state.selected_anchor_id, None);
    assert_eq!(state.autosave, AutosaveStatus::Dirty);

    let template = &state.templates[0];
    assert_eq!(template.template_id, template_id);
    assert_eq!(template.display_name, "Arcade panel");
    assert_eq!(template.asset_ids, vec![asset_id]);
    assert!(template.anchors.is_empty());
    assert!(template.default_anchor_bindings.is_empty());
    assert_eq!(template.default_token_preset, TokenPreset::Standard);
}
