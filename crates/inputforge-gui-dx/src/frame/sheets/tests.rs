use std::path::PathBuf;

use dioxus::prelude::*;
use inputforge_core::sheet::{
    AnchorId, AnchorPosition, AssetEntry, AssetHealth, AssetId, AssetPlacement, AssetPlacementId,
    DeviceTemplate, ExtensionPayload, PixelDimensions, TemplateAnchor, TemplateId, TemplateRect,
    TokenPreset,
};

use super::SheetsWorkbench;
use super::canvas::{
    SheetsCanvas, base64_encode, cleanup_stage_resize_listener_script, file_url_from_path,
    install_stage_resize_listener_script, stage_subscription_key,
};
use super::inspector::SheetsInspector;
use super::left_rail::SheetsLeftRail;
use super::state::{AutosaveStatus, CaptureStatus, SheetsEventKind, SheetsLibraryTab, SheetsState};

#[test]
fn empty_sheets_workbench_exposes_import_create_path() {
    let html = dioxus_ssr::render_element(rsx! {
        SheetsWorkbench {}
    });

    assert!(html.contains("data-testid=\"sheets-workbench\""));
    assert!(html.contains("New template"));
    assert!(!html.contains("Import image"));
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
fn sheets_left_rail_templates_tab_uses_template_create_path() {
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
                on_create_template: move |()| {},
            }
        }
    }

    let mut vdom = VirtualDom::new(Harness);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert!(html.contains("Arcade panel"));
    assert!(html.contains("New template"));
    assert!(!html.contains("Import image"));
}

#[test]
fn sheets_left_rail_defaults_to_templates_tab_only() {
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
                on_create_template: move |()| {},
            }
        }
    }

    let mut vdom = VirtualDom::new(Harness);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert!(html.contains("aria-selected=\"true\"") || html.contains("aria-selected=true"));
    assert!(html.contains("Arcade panel"));
    assert!(html.contains("New template"));
    assert!(!html.contains("assets/asset-1.png"));
    assert!(!html.contains("1 assets"));
    assert!(!html.contains("Import image"));
}

#[test]
fn sheets_left_rail_assets_tab_renders_assets_only() {
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
            library_tab: SheetsLibraryTab::Assets,
            ..SheetsState::default()
        };
        initial_state.create_template_from_asset(asset_id, "Arcade panel");
        initial_state.library_tab = SheetsLibraryTab::Assets;
        let sheets = use_signal(|| initial_state);

        rsx! {
            SheetsLeftRail {
                sheets,
                on_import_image: move |()| {},
                on_create_template: move |()| {},
            }
        }
    }

    let mut vdom = VirtualDom::new(Harness);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert!(html.contains("assets/asset-1.png"));
    assert!(html.contains("1 assets"));
    assert!(html.contains("Import image"));
    assert!(!html.contains("Arcade panel"));
    assert!(!html.contains("1 templates"));
    assert!(!html.contains("New template"));
}

#[test]
fn empty_templates_tab_uses_template_focused_empty_state() {
    #[expect(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    fn Harness() -> Element {
        let sheets = use_signal(SheetsState::default);

        rsx! {
            SheetsLeftRail {
                sheets,
                on_import_image: move |()| {},
                on_create_template: move |()| {},
            }
        }
    }

    let mut vdom = VirtualDom::new(Harness);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert!(html.contains("if-sheets__rail-empty"));
    assert!(html.contains("No templates"));
    assert!(html.contains("New template"));
    assert!(!html.contains("Import image"));
}

#[test]
fn empty_assets_tab_uses_background_image_empty_state() {
    #[expect(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    fn Harness() -> Element {
        let sheets = use_signal(|| SheetsState {
            library_tab: SheetsLibraryTab::Assets,
            ..SheetsState::default()
        });

        rsx! {
            SheetsLeftRail {
                sheets,
                on_import_image: move |()| {},
                on_create_template: move |()| {},
            }
        }
    }

    let mut vdom = VirtualDom::new(Harness);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert!(html.contains("if-sheets__rail-empty"));
    assert!(html.contains("No background images"));
    assert!(html.contains("Import image"));
    assert!(!html.contains("New template"));
}

#[test]
fn canvas_without_template_uses_centered_empty_state() {
    #[expect(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    fn Harness() -> Element {
        let sheets = use_signal(SheetsState::default);

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

    assert!(html.contains("if-sheets__canvas-empty"));
    assert!(html.contains("No template selected"));
    assert!(!html.contains("Import image"));
    assert!(!html.contains("New template"));
}

#[test]
fn template_without_background_image_renders_blank_canvas_stage() {
    #[expect(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    fn Harness() -> Element {
        let mut initial_state = SheetsState::default();
        initial_state.create_blank_template();
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

    assert!(html.contains("data-testid=\"sheets-image-stage\""));
    assert!(html.contains("data-asset-id=\"\""));
    assert!(!html.contains("<img"));
    assert!(!html.contains("Import image"));
}

#[test]
fn creating_blank_template_selects_template_without_asset_and_marks_dirty() {
    let mut state = SheetsState {
        autosave: AutosaveStatus::Clean,
        ..SheetsState::default()
    };

    let template_id = state.create_blank_template();

    assert_eq!(state.templates.len(), 1);
    assert_eq!(state.selected_template_id, Some(template_id.clone()));
    assert_eq!(state.selected_asset_id, None);
    assert_eq!(state.selected_anchor_id, None);
    assert_eq!(state.autosave, AutosaveStatus::Dirty);

    let template = &state.templates[0];
    assert_eq!(template.template_id, template_id);
    assert_eq!(template.display_name, "Untitled template 1");
    assert!(template.placements.is_empty());
    assert!(template.anchors.is_empty());
    assert!(template.default_anchor_bindings.is_empty());
    assert_eq!(template.default_token_preset, TokenPreset::Standard);
}

#[test]
fn creating_blank_template_uses_next_available_untitled_name() {
    let mut state = SheetsState::default();
    state.create_blank_template();

    let template_id = state.create_blank_template();

    assert_eq!(state.selected_template_id, Some(template_id));
    assert_eq!(state.templates[1].display_name, "Untitled template 2");
}

#[test]
fn renaming_selected_template_marks_dirty_and_preserves_non_blank_names() {
    let mut state = SheetsState {
        autosave: AutosaveStatus::Clean,
        ..SheetsState::default()
    };
    state.create_blank_template();
    state.mark_saved();

    state.rename_selected_template("Throttle quadrant");

    assert_eq!(state.templates[0].display_name, "Throttle quadrant");
    assert_eq!(state.autosave, AutosaveStatus::Dirty);

    state.mark_saved();
    state.rename_selected_template("   ");

    assert_eq!(state.templates[0].display_name, "Throttle quadrant");
    assert_eq!(state.autosave, AutosaveStatus::Clean);
}

#[test]
fn inspector_template_display_name_is_editable() {
    let mut state = SheetsState::default();
    state.create_blank_template();

    let html = render_inspector_state(state);
    let input_tag = control_tag_after_label(&html, "Template display name", "input");

    assert!(!input_tag.contains("readonly"));
    assert!(input_tag.contains("value=\"Untitled template 1\""));
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
    assert_eq!(
        template
            .placements
            .iter()
            .map(|p| p.asset_id.clone())
            .collect::<Vec<_>>(),
        vec![asset_id]
    );
    assert!(template.anchors.is_empty());
    assert!(template.default_anchor_bindings.is_empty());
    assert_eq!(template.default_token_preset, TokenPreset::Standard);
}

#[test]
fn base64_encoder_pads_binary_image_sources() {
    assert_eq!(base64_encode(b"hello"), "aGVsbG8=");
}

#[test]
fn valid_asset_canvas_uses_data_url_for_image_source() {
    #[expect(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    fn Harness(copied_path: PathBuf) -> Element {
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
            copied_absolute_path: copied_path,
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

    let copied_path =
        std::env::temp_dir().join(format!("inputforge-sheets-{}.png", ulid::Ulid::new()));
    std::fs::write(&copied_path, [0x89, b'P', b'N', b'G']).unwrap();
    let mut vdom = VirtualDom::new_with_props(Harness, copied_path.clone());
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);
    let _ = std::fs::remove_file(copied_path);

    assert!(
        html.contains("src=\"data:image/png;base64,iVBORw==\""),
        "valid copied asset should render as a WebView-safe data URL: {html}"
    );
    assert!(
        html.contains("data-source-path=\"file://") || html.contains("data-source-path=file://"),
        "valid copied asset should preserve its absolute source path for diagnostics: {html}"
    );
}

#[test]
fn selecting_non_first_asset_makes_it_canvas_asset() {
    #[expect(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    fn Harness(copied_paths: (PathBuf, PathBuf)) -> Element {
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
                placements: vec![
                    AssetPlacement {
                        placement_id: AssetPlacementId::new(),
                        asset_id: first_asset_id,
                        position: TemplateRect {
                            x: 0.05,
                            y: 0.05,
                            w: 0.9,
                            h: 0.9,
                        },
                        z_index: 0,
                        extensions: ExtensionPayload::default(),
                    },
                    AssetPlacement {
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
                    },
                ],
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
                    copied_absolute_path: copied_paths.0,
                    missing: false,
                },
                AssetHealth {
                    entry: second_asset,
                    copied_absolute_path: copied_paths.1,
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

    let first_path =
        std::env::temp_dir().join(format!("inputforge-sheets-{}.png", ulid::Ulid::new()));
    let second_path =
        std::env::temp_dir().join(format!("inputforge-sheets-{}.png", ulid::Ulid::new()));
    std::fs::write(&first_path, [0x89, b'P', b'N', b'G']).unwrap();
    std::fs::write(&second_path, [0x89, b'P', b'N', b'G']).unwrap();
    let expected_second_path = file_url_from_path(&second_path);
    let expected_first_path = file_url_from_path(&first_path);

    let mut vdom = VirtualDom::new_with_props(Harness, (first_path.clone(), second_path.clone()));
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);
    let _ = std::fs::remove_file(first_path);
    let _ = std::fs::remove_file(second_path);

    assert!(html.contains("data-asset-id=\"asset-2\""));
    assert!(html.contains(&format!("data-source-path=\"{expected_second_path}\"")));
    assert!(!html.contains(&format!("data-source-path=\"{expected_first_path}\"")));
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
                placements: vec![
                    AssetPlacement {
                        placement_id: AssetPlacementId::new(),
                        asset_id: first_asset_id,
                        position: TemplateRect {
                            x: 0.05,
                            y: 0.05,
                            w: 0.9,
                            h: 0.9,
                        },
                        z_index: 0,
                        extensions: ExtensionPayload::default(),
                    },
                    AssetPlacement {
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
                    },
                ],
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
            attached_to: None,
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
            attached_to: None,
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
        initial_state.library_tab = SheetsLibraryTab::Assets;
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
        initial_state.library_tab = SheetsLibraryTab::Assets;
        let sheets = use_signal(|| initial_state);

        rsx! {
            SheetsLeftRail {
                sheets,
                on_import_image: move |()| {},
                on_create_template: move |()| {},
            }
        }
    }

    let mut vdom = VirtualDom::new(Harness);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert!(html.contains("aria-label=\"Select first template using assets/asset-1.png\""));
    assert!(!html.contains("aria-label=\"Select template Arcade panel\""));
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
                placements: vec![
                    AssetPlacement {
                        placement_id: AssetPlacementId::new(),
                        asset_id: first_asset_id.clone(),
                        position: TemplateRect {
                            x: 0.05,
                            y: 0.05,
                            w: 0.9,
                            h: 0.9,
                        },
                        z_index: 0,
                        extensions: ExtensionPayload::default(),
                    },
                    AssetPlacement {
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
                    },
                ],
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
        initial_state.library_tab = SheetsLibraryTab::Assets;
        let sheets = use_signal(|| initial_state);

        rsx! {
            SheetsLeftRail {
                sheets,
                on_import_image: move |()| {},
                on_create_template: move |()| {},
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

#[test]
fn new_template_action_opens_inline_preset_picker_with_five_presets_and_skip() {
    #[expect(non_snake_case, reason = "Dioxus component")]
    fn Harness() -> Element {
        let sheets = use_signal(|| {
            let mut s = SheetsState::default();
            s.open_template_preset_picker();
            s
        });
        rsx! {
            SheetsLeftRail {
                sheets,
                on_import_image: move |()| {},
                on_create_template: move |()| {},
            }
        }
    }
    let mut vdom = VirtualDom::new(Harness);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    for label in [
        "Single",
        "Horizontal pair",
        "Vertical stack",
        "2x2 grid",
        "Freeform",
        "Skip",
        "Cancel",
    ] {
        assert!(
            html.contains(label),
            "preset picker missing {label}: {html}"
        );
    }
    assert!(html.contains("data-testid=\"sheets-preset-picker\""));
}

#[test]
fn skip_button_creates_a_blank_template_and_pushes_create_template() {
    let mut state = SheetsState::default();
    state.open_template_preset_picker();
    let baseline = state.history.len();

    state.skip_template_preset_picker();

    assert_eq!(state.templates.len(), 1);
    assert!(matches!(
        state.history.back().unwrap().kind,
        SheetsEventKind::CreateTemplate(_)
    ));
    assert_eq!(state.history.len(), baseline + 1);
    assert!(!state.preset_picker_open);
}

#[test]
fn cancel_button_closes_picker_without_creating_a_template() {
    let mut state = SheetsState::default();
    state.open_template_preset_picker();
    let baseline_templates = state.templates.len();
    let baseline_history = state.history.len();

    state.dismiss_template_preset_picker();

    assert_eq!(state.templates.len(), baseline_templates);
    assert_eq!(state.history.len(), baseline_history);
    assert!(!state.preset_picker_open);
}

#[test]
fn template_card_pluralizes_frame_and_anchor_counts() {
    #[expect(non_snake_case, reason = "Dioxus component")]
    fn Harness(counts: (usize, usize)) -> Element {
        let (placements, anchors) = counts;
        let sheets = use_signal(|| {
            let mut s = SheetsState::default();
            s.create_blank_template();
            for i in 0..placements {
                s.templates[0].placements.push(AssetPlacement {
                    placement_id: AssetPlacementId::from_string(format!("p-{i}")),
                    asset_id: AssetId::from_string("asset-1"),
                    position: TemplateRect {
                        x: 0.0,
                        y: 0.0,
                        w: 0.1,
                        h: 0.1,
                    },
                    z_index: i as i32,
                    extensions: ExtensionPayload::default(),
                });
            }
            for i in 0..anchors {
                s.templates[0].anchors.push(TemplateAnchor {
                    anchor_id: AnchorId::from_string(format!("a-{i}")),
                    label: format!("Anchor {i}"),
                    position: AnchorPosition { x: 0.5, y: 0.5 },
                    attached_to: None,
                    input_type_hint: None,
                    grouping_hint: None,
                    device_matching_hint: None,
                    extensions: ExtensionPayload::default(),
                });
            }
            s
        });
        rsx! {
            SheetsLeftRail {
                sheets,
                on_import_image: move |()| {},
                on_create_template: move |()| {},
            }
        }
    }

    for (placements, anchors, frame_word, anchor_word) in [
        (1, 1, "1 frame", "1 anchor"),
        (2, 2, "2 frames", "2 anchors"),
        (0, 0, "0 frames", "0 anchors"),
    ] {
        let mut vdom = VirtualDom::new_with_props(Harness, (placements, anchors));
        vdom.rebuild_in_place();
        let html = dioxus_ssr::render(&vdom);
        assert!(
            html.contains(frame_word),
            "expected `{frame_word}` in card: {html}"
        );
        assert!(
            html.contains(anchor_word),
            "expected `{anchor_word}` in card: {html}"
        );
    }
}
