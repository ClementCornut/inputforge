use std::path::PathBuf;

use dioxus::prelude::*;
use inputforge_core::sheet::{
    AnchorPosition, AssetEntry, AssetHealth, AssetId, ExtensionPayload, PixelDimensions,
    TemplateAnchor, TokenPreset,
};

use super::SheetsWorkbench;
use super::canvas::{
    SheetsCanvas, cleanup_stage_resize_listener_script, file_url_from_path,
    install_stage_resize_listener_script, stage_subscription_key,
};
use super::inspector::SheetsInspector;
use super::left_rail::SheetsLeftRail;
use super::state::{AutosaveStatus, CaptureStatus, SheetsState};

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

fn render_inspector_state(initial_state: SheetsState) -> String {
    #[expect(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    fn Harness(initial_state: SheetsState) -> Element {
        let sheets = use_signal(|| initial_state);

        rsx! {
            SheetsInspector {
                sheets,
                on_arm_capture: move |_| {},
                on_cancel_capture: move |()| {},
                on_retry_save: move |()| {},
            }
        }
    }

    let mut vdom = VirtualDom::new_with_props(Harness, initial_state);
    vdom.rebuild_in_place();
    dioxus_ssr::render(&vdom)
}

#[test]
fn autosave_failure_renders_retry_action() {
    let html = render_inspector_state(SheetsState {
        autosave: AutosaveStatus::Failed,
        last_error: Some("disk is full".to_owned()),
        ..SheetsState::default()
    });

    assert!(html.contains("Save failed"));
    assert!(html.contains("disk is full"));
    assert!(html.contains("Retry save"));
}

#[test]
fn inspector_disables_capture_when_unavailable_but_keeps_manual_assignment_visible() {
    let html = render_inspector_state(SheetsState::default());

    assert!(html.contains("Capture input"));
    assert!(html.contains("disabled"));
    assert!(html.contains("live input is not available"));
    assert!(html.contains("Manual assignment"));
    assert!(html.contains("Input address"));
}

#[test]
fn inspector_disables_manual_assignment_while_capture_is_armed() {
    let anchor_id = inputforge_core::sheet::AnchorId::from_string("anchor-1");
    let html = render_inspector_state(SheetsState {
        selected_anchor_id: Some(anchor_id.clone()),
        capture: CaptureStatus::Armed(anchor_id),
        ..SheetsState::default()
    });
    let assign_button_start = html
        .find("Assign input")
        .expect("inspector should render manual assign button");
    let assign_button_tag = &html[..assign_button_start];
    let assign_button_tag = assign_button_tag
        .rsplit_once("<button")
        .map(|(_, tag)| tag)
        .expect("assign label should be inside a button");

    assert!(
        assign_button_tag.contains("disabled"),
        "manual assign button should be disabled while capture is armed: {html}"
    );
}

fn control_tag_after_label<'a>(html: &'a str, label: &str, tag_name: &str) -> &'a str {
    let label_start = html
        .find(label)
        .unwrap_or_else(|| panic!("inspector should render {label} label: {html}"));
    let html_after_label = &html[label_start..];
    let tag_start = html_after_label
        .find(&format!("<{tag_name}"))
        .unwrap_or_else(|| panic!("{label} should include a {tag_name} control: {html}"));
    let html_after_tag = &html_after_label[tag_start..];
    let tag_end = html_after_tag
        .find('>')
        .unwrap_or_else(|| panic!("{label} {tag_name} control should have a closing tag: {html}"));

    &html_after_tag[..=tag_end]
}

#[test]
fn inspector_disables_manual_form_controls_while_capture_is_armed() {
    let anchor_id = inputforge_core::sheet::AnchorId::from_string("anchor-1");
    let html = render_inspector_state(SheetsState {
        selected_anchor_id: Some(anchor_id.clone()),
        capture: CaptureStatus::Armed(anchor_id),
        ..SheetsState::default()
    });

    for (label, tag_name) in [
        ("Device id", "input"),
        ("Input kind", "select"),
        ("Input index", "input"),
    ] {
        let control_tag = control_tag_after_label(&html, label, tag_name);
        assert!(
            control_tag.contains("disabled"),
            "{label} control should be disabled while capture is armed: {html}"
        );
    }
}

#[test]
fn sheets_left_rail_keeps_import_create_path_with_existing_template() {
    #[expect(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    fn Harness() -> Element {
        let mut initial_state = SheetsState::default();
        initial_state.create_template_from_asset(AssetId::from_string("asset-1"), "Arcade panel");
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

#[test]
fn valid_asset_canvas_uses_file_url_for_image_source() {
    #[expect(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    fn Harness() -> Element {
        let asset_id = AssetId::from_string("asset-1");
        let mut initial_state = SheetsState {
            assets: vec![AssetEntry {
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
            }],
            ..SheetsState::default()
        };
        initial_state.create_template_from_asset(asset_id, "Arcade panel");
        let selected_template = initial_state.templates[0].template_id.clone();
        let selected_asset = initial_state.assets[0].clone();
        initial_state.asset_health = vec![AssetHealth {
            entry: selected_asset,
            copied_absolute_path: PathBuf::from(r"C:\InputForge\assets\asset-1.png"),
            missing: false,
        }];
        initial_state.selected_template_id = Some(selected_template);
        let sheets = use_signal(|| initial_state);

        rsx! {
            SheetsCanvas {
                sheets,
                on_import_image: move |()| {},
                on_arm_capture: move |_| {},
            }
        }
    }

    let mut vdom = VirtualDom::new(Harness);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert!(
        html.contains("src=\"file:///C:/InputForge/assets/asset-1.png\""),
        "valid copied asset should render as a WebView-safe file URL: {html}"
    );
    assert!(
        !html.contains("src=\"assets/asset-1.png\""),
        "valid copied asset must not render the manifest-relative copied_path: {html}"
    );
}

#[test]
fn file_url_from_path_formats_unc_paths_with_authority() {
    let path = PathBuf::from(r"\\server\share\My Stick é.png");

    let url = file_url_from_path(&path);

    assert_eq!(url, "file://server/share/My%20Stick%20%C3%A9.png");
}

#[test]
fn stage_subscription_key_tracks_rendered_asset_identity() {
    assert_eq!(
        stage_subscription_key(true, false, "asset-1"),
        Some("asset-1".to_owned())
    );
    assert_eq!(stage_subscription_key(true, false, ""), None);
    assert_eq!(stage_subscription_key(true, true, "asset-1"), None);
    assert_eq!(stage_subscription_key(false, false, "asset-1"), None);
}

#[test]
fn stage_resize_bridge_observes_stage_layout_changes_and_cleans_up() {
    let install_script = install_stage_resize_listener_script(
        "if-sheets-image-stage",
        "__inputforgeSheetsStageResize_asset-1",
    );
    let cleanup_script =
        cleanup_stage_resize_listener_script("__inputforgeSheetsStageResize_asset-1");

    assert!(
        install_script.contains("new ResizeObserver"),
        "stage bridge should remeasure after intrinsic image/layout size changes: {install_script}"
    );
    assert!(
        install_script.contains(".observe(stage)"),
        "stage bridge should observe the stage element, not just window resize: {install_script}"
    );
    assert!(
        cleanup_script.contains(".disconnect()"),
        "stage bridge cleanup should disconnect the stage observer: {cleanup_script}"
    );
}

#[test]
fn missing_asset_canvas_preserves_anchor_count_message() {
    #[expect(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    fn Harness() -> Element {
        let asset_id = AssetId::from_string("asset-1");
        let mut initial_state = SheetsState {
            assets: vec![AssetEntry {
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
            }],
            ..SheetsState::default()
        };
        initial_state.create_template_from_asset(asset_id, "Arcade panel");
        let selected_template = initial_state.templates[0].template_id.clone();
        let selected_asset = initial_state.assets[0].clone();
        initial_state.templates[0].anchors.push(TemplateAnchor {
            anchor_id: inputforge_core::sheet::AnchorId::from_string("anchor-1"),
            label: "Trigger".to_owned(),
            position: AnchorPosition { x: 0.25, y: 0.5 },
            input_type_hint: None,
            grouping_hint: None,
            device_matching_hint: None,
            extensions: ExtensionPayload::default(),
        });
        initial_state.asset_health = vec![AssetHealth {
            entry: selected_asset,
            copied_absolute_path: PathBuf::from("missing/asset-1.png"),
            missing: true,
        }];
        initial_state.selected_template_id = Some(selected_template);
        let sheets = use_signal(|| initial_state);

        rsx! {
            SheetsCanvas {
                sheets,
                on_import_image: move |()| {},
                on_arm_capture: move |_| {},
            }
        }
    }

    let mut vdom = VirtualDom::new(Harness);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert!(html.contains("Missing image asset"));
    assert!(
        html.contains("1 anchor"),
        "missing asset message should keep anchor count visible: {html}"
    );
}
