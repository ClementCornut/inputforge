use std::path::PathBuf;

use dioxus::prelude::*;
use inputforge_core::sheet::{
    AnchorPosition, AssetEntry, AssetHealth, AssetId, DeviceTemplate, ExtensionPayload,
    PixelDimensions, TemplateAnchor, TemplateId, TokenPreset,
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
    assert_eq!(state.selected_asset_id, Some(asset_id.clone()));
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
fn selecting_non_first_asset_makes_it_canvas_asset() {
    #[expect(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    fn Harness() -> Element {
        let first_asset_id = AssetId::from_string("asset-1");
        let second_asset_id = AssetId::from_string("asset-2");
        let first_asset = AssetEntry {
            asset_id: first_asset_id.clone(),
            copied_path: PathBuf::from("assets/asset-1.png"),
            content_hash: "hash-1".to_owned(),
            media_type: "image/png".to_owned(),
            pixel_dimensions: PixelDimensions {
                width: 320,
                height: 240,
            },
            original_import_path: None,
            extensions: ExtensionPayload::default(),
        };
        let second_asset = AssetEntry {
            asset_id: second_asset_id.clone(),
            copied_path: PathBuf::from("assets/asset-2.png"),
            content_hash: "hash-2".to_owned(),
            media_type: "image/png".to_owned(),
            pixel_dimensions: PixelDimensions {
                width: 640,
                height: 480,
            },
            original_import_path: Some(PathBuf::from("D:/imports/asset-2.png")),
            extensions: ExtensionPayload::default(),
        };
        let template_id = TemplateId::from_string("template-1");
        let mut initial_state = SheetsState {
            templates: vec![DeviceTemplate {
                template_id: template_id.clone(),
                display_name: "Arcade panel".to_owned(),
                matching_hints: Vec::new(),
                asset_ids: vec![first_asset_id, second_asset_id.clone()],
                anchors: Vec::new(),
                default_anchor_bindings: Vec::new(),
                grouping_hints: Vec::new(),
                default_token_preset: TokenPreset::Standard,
                extensions: ExtensionPayload::default(),
            }],
            assets: vec![first_asset.clone(), second_asset.clone()],
            asset_health: vec![
                AssetHealth {
                    entry: first_asset,
                    copied_absolute_path: PathBuf::from(r"C:\InputForge\assets\asset-1.png"),
                    missing: false,
                },
                AssetHealth {
                    entry: second_asset,
                    copied_absolute_path: PathBuf::from(r"C:\InputForge\assets\asset-2.png"),
                    missing: false,
                },
            ],
            selected_template_id: Some(template_id),
            ..SheetsState::default()
        };
        initial_state.select_first_template_for_asset(second_asset_id);
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

    assert!(html.contains("data-asset-id=\"asset-2\""));
    assert!(html.contains("src=\"file:///C:/InputForge/assets/asset-2.png\""));
    assert!(!html.contains("src=\"file:///C:/InputForge/assets/asset-1.png\""));
}

#[test]
fn selecting_non_first_missing_asset_renders_its_recovery_paths() {
    #[expect(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    fn Harness() -> Element {
        let first_asset_id = AssetId::from_string("asset-1");
        let second_asset_id = AssetId::from_string("asset-2");
        let first_asset = AssetEntry {
            asset_id: first_asset_id.clone(),
            copied_path: PathBuf::from("assets/asset-1.png"),
            content_hash: "hash-1".to_owned(),
            media_type: "image/png".to_owned(),
            pixel_dimensions: PixelDimensions {
                width: 320,
                height: 240,
            },
            original_import_path: Some(PathBuf::from("D:/imports/asset-1.png")),
            extensions: ExtensionPayload::default(),
        };
        let second_asset = AssetEntry {
            asset_id: second_asset_id.clone(),
            copied_path: PathBuf::from("assets/asset-2.png"),
            content_hash: "hash-2".to_owned(),
            media_type: "image/png".to_owned(),
            pixel_dimensions: PixelDimensions {
                width: 640,
                height: 480,
            },
            original_import_path: Some(PathBuf::from("D:/imports/asset-2.png")),
            extensions: ExtensionPayload::default(),
        };
        let mut initial_state = SheetsState {
            templates: vec![DeviceTemplate {
                template_id: TemplateId::from_string("template-1"),
                display_name: "Arcade panel".to_owned(),
                matching_hints: Vec::new(),
                asset_ids: vec![first_asset_id, second_asset_id.clone()],
                anchors: Vec::new(),
                default_anchor_bindings: Vec::new(),
                grouping_hints: Vec::new(),
                default_token_preset: TokenPreset::Standard,
                extensions: ExtensionPayload::default(),
            }],
            assets: vec![first_asset.clone(), second_asset.clone()],
            asset_health: vec![
                AssetHealth {
                    entry: first_asset,
                    copied_absolute_path: PathBuf::from("C:/InputForge/assets/asset-1.png"),
                    missing: false,
                },
                AssetHealth {
                    entry: second_asset,
                    copied_absolute_path: PathBuf::from("C:/InputForge/assets/asset-2.png"),
                    missing: true,
                },
            ],
            ..SheetsState::default()
        };
        initial_state.select_first_template_for_asset(second_asset_id);
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
    assert!(html.contains("C:/InputForge/assets/asset-2.png"));
    assert!(html.contains("D:/imports/asset-2.png"));
    assert!(!html.contains("C:/InputForge/assets/asset-1.png"));
    assert!(!html.contains("D:/imports/asset-1.png"));
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

#[test]
fn missing_asset_canvas_renders_recovery_paths_with_anchor_count() {
    #[expect(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    fn Harness() -> Element {
        let asset_id = AssetId::from_string("asset-1");
        let mut initial_state = SheetsState {
            assets: vec![AssetEntry {
                asset_id: asset_id.clone(),
                copied_path: PathBuf::from("assets/copied-panel.png"),
                content_hash: "hash".to_owned(),
                media_type: "image/png".to_owned(),
                pixel_dimensions: PixelDimensions {
                    width: 320,
                    height: 240,
                },
                original_import_path: Some(PathBuf::from("D:/imports/original-panel.png")),
                extensions: ExtensionPayload::default(),
            }],
            ..SheetsState::default()
        };
        initial_state.create_template_from_asset(asset_id, "Arcade panel");
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
            copied_absolute_path: PathBuf::from("C:/InputForge/assets/copied-panel.png"),
            missing: true,
        }];
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
    assert!(html.contains("Copied path"));
    assert!(html.contains("C:/InputForge/assets/copied-panel.png"));
    assert!(html.contains("Original import path"));
    assert!(html.contains("D:/imports/original-panel.png"));
    assert!(html.contains("1 anchor"));
}

#[test]
fn canvas_treats_template_asset_without_health_as_missing() {
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
        !html.contains("data-testid=\"sheets-image-stage\""),
        "asset without health should not render an empty image stage: {html}"
    );
}

#[test]
fn left_rail_renders_template_and_asset_rows_as_selectable_controls() {
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

    assert!(html.contains("aria-label=\"Select template Arcade panel\""));
    assert!(html.contains("aria-label=\"Select first template using assets/asset-1.png\""));
}

#[test]
fn left_rail_marks_only_selected_non_first_asset_row() {
    #[expect(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    fn Harness() -> Element {
        let first_asset_id = AssetId::from_string("asset-1");
        let second_asset_id = AssetId::from_string("asset-2");
        let mut initial_state = SheetsState {
            templates: vec![DeviceTemplate {
                template_id: TemplateId::from_string("template-1"),
                display_name: "Arcade panel".to_owned(),
                matching_hints: Vec::new(),
                asset_ids: vec![first_asset_id.clone(), second_asset_id.clone()],
                anchors: Vec::new(),
                default_anchor_bindings: Vec::new(),
                grouping_hints: Vec::new(),
                default_token_preset: TokenPreset::Standard,
                extensions: ExtensionPayload::default(),
            }],
            assets: vec![
                AssetEntry {
                    asset_id: first_asset_id,
                    copied_path: PathBuf::from("assets/asset-1.png"),
                    content_hash: "hash-1".to_owned(),
                    media_type: "image/png".to_owned(),
                    pixel_dimensions: PixelDimensions {
                        width: 320,
                        height: 240,
                    },
                    original_import_path: None,
                    extensions: ExtensionPayload::default(),
                },
                AssetEntry {
                    asset_id: second_asset_id.clone(),
                    copied_path: PathBuf::from("assets/asset-2.png"),
                    content_hash: "hash-2".to_owned(),
                    media_type: "image/png".to_owned(),
                    pixel_dimensions: PixelDimensions {
                        width: 640,
                        height: 480,
                    },
                    original_import_path: None,
                    extensions: ExtensionPayload::default(),
                },
            ],
            ..SheetsState::default()
        };
        initial_state.select_first_template_for_asset(second_asset_id);
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
    let asset_1_start = html.find("assets/asset-1.png").unwrap();
    let asset_2_start = html.find("assets/asset-2.png").unwrap();
    let asset_1_row = &html[..asset_1_start]
        .rsplit_once("<li")
        .map(|(_, row)| row)
        .unwrap();
    let asset_2_row = &html[..asset_2_start]
        .rsplit_once("<li")
        .map(|(_, row)| row)
        .unwrap();

    let row_is_selected =
        |row: &str| row.contains("data-selected=\"true\"") || row.contains("data-selected=true");

    assert!(!row_is_selected(asset_1_row));
    assert!(row_is_selected(asset_2_row));
}
