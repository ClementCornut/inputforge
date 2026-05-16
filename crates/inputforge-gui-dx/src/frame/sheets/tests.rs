use std::path::PathBuf;

use dioxus::prelude::*;
use inputforge_core::sheet::{
    AnchorId, AnchorPosition, AssetEntry, AssetHealth, AssetId, AssetPlacement, AssetPlacementId,
    DeviceTemplate, ExtensionPayload, PixelDimensions, TemplateAnchor, TemplateId, TemplateRect,
    TokenPreset,
};
use inputforge_core::types::DeviceId;

use super::SheetsWorkbench;
use super::canvas::{
    SheetsCanvas, base64_encode, cleanup_stage_resize_listener_script, file_url_from_path,
    install_stage_resize_listener_script, stage_subscription_key,
};
use super::inspector::SheetsInspector;
use super::left_rail::SheetsLeftRail;
use super::state::{
    AutosaveStatus, CaptureAvailabilityReason, CaptureStatus, SheetLayoutPreset, SheetsEventKind,
    SheetsLibraryTab, SheetsState,
};

#[test]
fn empty_sheets_workbench_exposes_import_create_path() {
    let html = dioxus_ssr::render_element(rsx! {
        SheetsWorkbench {}
    });

    assert!(html.contains("data-testid=\"sheets-workbench\""));
    assert!(html.contains("New template"));
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

fn render_inspector_state_with_availability(
    initial_state: SheetsState,
    capture_availability: CaptureAvailabilityReason,
) -> String {
    #[derive(Clone, PartialEq, Props)]
    struct HarnessProps {
        initial_state: SheetsState,
        capture_availability: CaptureAvailabilityReason,
    }

    #[expect(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    fn Harness(props: HarnessProps) -> Element {
        let sheets = use_signal(|| props.initial_state);
        let capture_availability = props.capture_availability;

        rsx! {
            SheetsInspector {
                sheets,
                capture_availability,
                on_arm_capture: move |_| {},
                on_cancel_capture: move |()| {},
                on_retry_save: move |()| {},
            }
        }
    }

    let mut vdom = VirtualDom::new_with_props(
        Harness,
        HarnessProps {
            initial_state,
            capture_availability,
        },
    );
    vdom.rebuild_in_place();
    dioxus_ssr::render(&vdom)
}

fn render_inspector_state_with_devices(
    initial_state: SheetsState,
    connected_devices: Vec<(DeviceId, String)>,
    capture_availability: CaptureAvailabilityReason,
) -> String {
    #[derive(Clone, PartialEq, Props)]
    struct HarnessProps {
        initial_state: SheetsState,
        connected_devices: Vec<(DeviceId, String)>,
        capture_availability: CaptureAvailabilityReason,
    }

    #[expect(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    fn Harness(props: HarnessProps) -> Element {
        let sheets = use_signal(|| props.initial_state);
        let capture_availability = props.capture_availability;
        let connected_devices = props.connected_devices;

        rsx! {
            SheetsInspector {
                sheets,
                capture_availability,
                connected_devices,
                on_arm_capture: move |_| {},
                on_cancel_capture: move |()| {},
                on_retry_save: move |()| {},
            }
        }
    }

    let mut vdom = VirtualDom::new_with_props(
        Harness,
        HarnessProps {
            initial_state,
            connected_devices,
            capture_availability,
        },
    );
    vdom.rebuild_in_place();
    dioxus_ssr::render(&vdom)
}

fn state_with_selected_anchor() -> SheetsState {
    let mut state = SheetsState::default();
    state.create_blank_template();
    let anchor_id = AnchorId::from_string("anchor-1");
    state.templates[0].anchors.push(TemplateAnchor {
        anchor_id: anchor_id.clone(),
        label: "Trigger".to_owned(),
        position: AnchorPosition { x: 0.25, y: 0.5 },
        attached_to: None,
        input_type_hint: None,
        grouping_hint: None,
        device_matching_hint: None,
        extensions: ExtensionPayload::default(),
    });
    state.selected_anchor_id = Some(anchor_id);
    state
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
fn inspector_with_no_selection_shows_template_summary_eyebrow() {
    let mut state = SheetsState::default();
    state.create_blank_template();
    let html = render_inspector_state(state);
    assert!(
        html.contains(">TEMPLATE<"),
        "expected TEMPLATE eyebrow in inspector: {html}"
    );
    assert!(html.contains("Frame count"), "expected frame count: {html}");
    assert!(
        html.contains("Anchor count"),
        "expected anchor count: {html}"
    );
}

#[test]
fn inspector_with_frame_selected_shows_position_inputs_and_z_index_buttons() {
    let mut state = SheetsState::default();
    state.create_blank_template();
    let placement_id = AssetPlacementId::from_string("p-sel");
    state.templates[0].placements.push(AssetPlacement {
        placement_id: placement_id.clone(),
        asset_id: AssetId::from_string("asset-1"),
        position: TemplateRect {
            x: 0.1,
            y: 0.2,
            w: 0.3,
            h: 0.4,
        },
        z_index: 7,
        extensions: ExtensionPayload::default(),
    });
    state.selected_placement_id = Some(placement_id);
    let html = render_inspector_state(state);
    assert!(
        html.contains(">FRAME<"),
        "expected FRAME eyebrow in inspector: {html}"
    );
    for axis in ["x", "y", "w", "h"] {
        assert!(
            html.contains(&format!("data-axis=\"{axis}\"")),
            "missing axis input {axis}: {html}"
        );
    }
    assert!(html.contains("Bring to front"), "missing z+ button: {html}");
    assert!(html.contains("Send to back"), "missing z- button: {html}");
}

#[test]
fn inspector_with_anchor_selected_shows_label_attach_toggle_and_assignment_composer() {
    let state = state_with_selected_anchor();
    assert!(
        state.selected_anchor_id.is_some(),
        "test helper must select an anchor",
    );
    let html = render_inspector_state(state);
    assert!(
        html.contains(">ANCHOR<"),
        "expected ANCHOR eyebrow in inspector: {html}"
    );
    assert!(
        html.contains("Attach to frame") || html.contains("Detach"),
        "expected attach toggle: {html}"
    );
    assert!(
        html.contains(">ASSIGNMENT<"),
        "expected ASSIGNMENT eyebrow in inspector: {html}"
    );
}

#[test]
fn inspector_keeps_manual_assignment_visible_when_capture_unavailable() {
    let html = render_inspector_state(state_with_selected_anchor());

    assert!(html.contains("Press input to assign"));
    assert!(html.contains(">Choose device manually<"));
}

#[test]
fn composer_idle_with_capture_available_renders_press_to_assign_button() {
    let mut state = state_with_selected_anchor();
    state.capture = CaptureStatus::Idle;
    let html = render_inspector_state(state);
    assert!(html.contains(">Press input to assign<"));
    assert!(html.contains("Press a control on any connected device to assign."));
}

#[test]
fn composer_armed_renders_if_rebind_composite_listening_visual() {
    let mut state = state_with_selected_anchor();
    let anchor_id = state.selected_anchor_id.clone().unwrap();
    state.capture = CaptureStatus::Armed(anchor_id);
    let html = render_inspector_state(state);
    assert!(html.contains("if-rebind-composite__listening"));
    assert!(html.contains("Hold Esc to cancel"));
}

#[test]
fn composer_timed_out_and_canceled_render_retry_help_branch() {
    let mut state = state_with_selected_anchor();
    let anchor_id = state.selected_anchor_id.clone().unwrap();
    state.capture = CaptureStatus::TimedOut(anchor_id.clone());
    assert!(render_inspector_state(state.clone()).contains("Capture timed out. Press to retry."));
    state.capture = CaptureStatus::Canceled(anchor_id);
    assert!(render_inspector_state(state).contains("Capture canceled. Press to retry."));
}

#[test]
fn composer_help_branches_when_engine_stopped() {
    let mut state = state_with_selected_anchor();
    state.capture = CaptureStatus::Unavailable("live input is not available".to_owned());
    let html =
        render_inspector_state_with_availability(state, CaptureAvailabilityReason::EngineStopped);
    assert!(html.contains("Start the engine to capture, or choose device manually."));
}

#[test]
fn manual_disclosure_is_closed_by_default() {
    let mut state = state_with_selected_anchor();
    state.capture = CaptureStatus::Idle;
    let html = render_inspector_state_with_devices(
        state,
        vec![(
            DeviceId("stick-alpha".to_owned()),
            "Throttle Quadrant".to_owned(),
        )],
        CaptureAvailabilityReason::CaptureAvailable,
    );
    assert!(html.contains("<details"), "disclosure must render: {html}");
    assert!(
        !html.contains("<details open"),
        "disclosure must NOT carry the `open` attribute by default: {html}"
    );
    assert!(html.contains(">Choose device manually<"));
}

#[test]
fn manual_disclosure_lists_connected_devices_with_display_names() {
    let mut state = state_with_selected_anchor();
    state.capture = CaptureStatus::Idle;
    let html = render_inspector_state_with_devices(
        state,
        vec![
            (
                DeviceId("stick-alpha".to_owned()),
                "Throttle Quadrant".to_owned(),
            ),
            (
                DeviceId("stick-beta".to_owned()),
                "Rudder Pedals".to_owned(),
            ),
        ],
        CaptureAvailabilityReason::CaptureAvailable,
    );
    assert!(html.contains("Throttle Quadrant"));
    assert!(html.contains("Rudder Pedals"));
    assert!(html.contains(">Button<"));
    assert!(html.contains(">Axis<"));
    assert!(html.contains(">Hat<"));
    assert!(html.contains("<fieldset"));
    assert!(html.contains(">Type<"));
}

#[test]
fn manual_disclosure_empty_state_branches_on_engine_state() {
    let state = state_with_selected_anchor();
    let html = render_inspector_state_with_devices(
        state.clone(),
        Vec::new(),
        CaptureAvailabilityReason::EngineStopped,
    );
    assert!(html.contains("Start the engine to assign."));

    let html = render_inspector_state_with_devices(
        state,
        Vec::new(),
        CaptureAvailabilityReason::EngineRunningNoDevices,
    );
    assert!(html.contains("No devices seen. Connect a device to assign manually."));
}

#[test]
fn composer_help_branches_when_engine_running_with_no_devices() {
    let mut state = state_with_selected_anchor();
    state.capture = CaptureStatus::Unavailable("no connected devices".to_owned());
    let html = render_inspector_state_with_availability(
        state,
        CaptureAvailabilityReason::EngineRunningNoDevices,
    );
    assert!(html.contains(
        "Connect a device to capture, or use the manual disclosure when a device appears."
    ));
}

#[test]
fn inspector_disables_manual_assignment_while_capture_is_armed() {
    let anchor_id = AnchorId::from_string("anchor-1");
    let html = render_inspector_state_with_devices(
        SheetsState {
            selected_anchor_id: Some(anchor_id.clone()),
            capture: CaptureStatus::Armed(anchor_id),
            ..SheetsState::default()
        },
        vec![(
            DeviceId("stick-alpha".to_owned()),
            "Throttle Quadrant".to_owned(),
        )],
        CaptureAvailabilityReason::CaptureAvailable,
    );
    let assign_button_start = html
        .find(">Assign<")
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
#[allow(
    clippy::too_many_lines,
    reason = "test scenario builds a multi-asset workbench inline for clarity"
)]
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

    // The stage carries the selected asset's id; the multi-image canvas renders every
    // placement's own image, so the selected asset is identified via the stage attribute
    // rather than being the only rendered image.
    assert!(html.contains("data-asset-id=\"asset-2\""));
    assert!(html.contains(&format!("data-source-path=\"{expected_second_path}\"")));
    assert!(html.contains(&format!("data-source-path=\"{expected_first_path}\"")));
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "test scenario seeds multi-asset missing-state recovery surface inline"
)]
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
            anchor_id: AnchorId::from_string("anchor-1"),
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
            anchor_id: AnchorId::from_string("anchor-1"),
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
                    z_index: i32::try_from(i).expect("test placement count fits in i32"),
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

#[test]
fn asset_row_when_a_template_is_selected_is_draggable_and_carries_asset_id() {
    #[expect(non_snake_case, reason = "Dioxus component")]
    fn Harness() -> Element {
        let asset_id = AssetId::from_string("asset-drag");
        let mut initial_state = SheetsState {
            assets: vec![AssetEntry {
                asset_id: asset_id.clone(),
                copied_path: PathBuf::from("imports/cockpit.png"),
                content_hash: "hash".to_owned(),
                media_type: "image/png".to_owned(),
                pixel_dimensions: PixelDimensions {
                    width: 320,
                    height: 240,
                },
                original_import_path: Some(PathBuf::from("D:/imports/cockpit.png")),
                extensions: ExtensionPayload::default(),
            }],
            library_tab: SheetsLibraryTab::Assets,
            ..SheetsState::default()
        };
        initial_state.create_blank_template();
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
    assert!(
        html.contains("draggable=\"true\""),
        "expected draggable=\"true\" attribute: {html}"
    );
    assert!(
        html.contains("data-asset-id=\"asset-drag\""),
        "expected data-asset-id=\"asset-drag\": {html}"
    );
    assert!(
        html.contains("cockpit.png"),
        "expected filename label: {html}"
    );
}

#[test]
fn canvas_renders_toolbar_with_segmented_select_anchor_plus_import_and_status_chip() {
    #[expect(non_snake_case, reason = "Dioxus component")]
    fn Harness() -> Element {
        let mut initial_state = SheetsState::default();
        initial_state.create_blank_template();
        initial_state.autosave = AutosaveStatus::Clean;
        let sheets = use_signal(|| initial_state);
        rsx! {
            SheetsCanvas { sheets, on_import_image: move |()| {}, on_arm_capture: move |_| {} }
        }
    }
    let mut vdom = VirtualDom::new(Harness);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert!(html.contains("data-testid=\"sheets-toolbar\""));
    assert!(html.contains(">Select<"));
    assert!(html.contains(">Anchor<"));
    assert!(html.contains("aria-label=\"Import image\""));
    assert!(html.contains("data-testid=\"sheets-autosave-chip\""));
    // The + button is the Phosphor plus icon, not a literal `+` character.
    // The Icon component injects the SVG via dangerous_inner_html; the rendered HTML will
    // contain the SVG path data. The robust check is for the icon's containing class.
    assert!(
        html.contains("if-icon"),
        "import button should render an icon (if-icon class): {html}"
    );
    // Default (no mutation, no failure) lands in the Clean/neutral tone.
    assert!(html.contains("data-status=\"clean\""));
    assert!(html.contains("data-tone=\"neutral\""));
    assert!(html.contains("Saved"));
}

#[test]
fn autosave_chip_status_and_tone_follow_the_state_machine() {
    #[expect(non_snake_case, reason = "Dioxus component")]
    fn Harness(status: AutosaveStatus) -> Element {
        let mut initial = SheetsState::default();
        initial.create_blank_template();
        initial.autosave = status;
        let sheets = use_signal(|| initial);
        rsx! { SheetsCanvas { sheets, on_import_image: move |()| {}, on_arm_capture: move |_| {} } }
    }

    for (status, expected_status, expected_tone, expected_label) in [
        (AutosaveStatus::Clean, "clean", "neutral", "Saved"),
        (AutosaveStatus::Saving, "saving", "neutral", "Saving"),
        (AutosaveStatus::Dirty, "dirty", "amber", "Unsaved"),
        (AutosaveStatus::Failed, "failed", "red", "Save failed"),
    ] {
        let mut vdom = VirtualDom::new_with_props(Harness, status);
        vdom.rebuild_in_place();
        let html = dioxus_ssr::render(&vdom);
        assert!(
            html.contains(&format!("data-status=\"{expected_status}\"")),
            "expected status `{expected_status}` for {status:?}: {html}"
        );
        assert!(
            html.contains(&format!("data-tone=\"{expected_tone}\"")),
            "expected tone `{expected_tone}` for {status:?}: {html}"
        );
        assert!(
            html.contains(expected_label),
            "expected label `{expected_label}` for {status:?}: {html}"
        );
    }
}

#[test]
fn canvas_with_template_and_no_placements_renders_full_canvas_drop_zone_copy() {
    #[expect(non_snake_case, reason = "Dioxus component")]
    fn Harness() -> Element {
        let mut state = SheetsState::default();
        state.create_blank_template();
        let sheets = use_signal(|| state);
        rsx! { SheetsCanvas { sheets, on_import_image: move |()| {}, on_arm_capture: move |_| {} } }
    }
    let mut vdom = VirtualDom::new(Harness);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    assert!(html.contains("No images yet. Drop an asset from the rail or click + to import."));
    assert!(html.contains("data-testid=\"sheets-empty-dropzone\""));
}

#[test]
fn canvas_after_preset_pick_renders_preset_slot_drop_zones() {
    #[expect(non_snake_case, reason = "Dioxus component")]
    fn Harness() -> Element {
        let mut state = SheetsState::default();
        state.apply_preset_after_create(SheetLayoutPreset::HorizontalPair);
        let sheets = use_signal(|| state);
        rsx! { SheetsCanvas { sheets, on_import_image: move |()| {}, on_arm_capture: move |_| {} } }
    }
    let mut vdom = VirtualDom::new(Harness);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    let slot_count = html.matches("data-testid=\"sheets-preset-slot\"").count();
    assert_eq!(
        slot_count, 2,
        "H-pair should render 2 slot drop zones: {html}"
    );
    assert!(html.contains("+ Add image"));
}

#[test]
fn canvas_renders_each_placement_clipped_to_its_rect_and_sorted_by_z_index() {
    #[expect(non_snake_case, reason = "Dioxus component")]
    fn Harness() -> Element {
        let asset_a = AssetEntry {
            asset_id: AssetId::from_string("asset-a"),
            copied_path: PathBuf::from("assets/asset-a.png"),
            content_hash: "ha".to_owned(),
            media_type: "image/png".to_owned(),
            pixel_dimensions: PixelDimensions {
                width: 64,
                height: 32,
            },
            original_import_path: None,
            extensions: ExtensionPayload::default(),
        };
        let asset_b = AssetEntry {
            asset_id: AssetId::from_string("asset-b"),
            ..asset_a.clone()
        };
        let mut state = SheetsState {
            assets: vec![asset_a.clone(), asset_b.clone()],
            asset_health: vec![
                AssetHealth {
                    entry: asset_a.clone(),
                    copied_absolute_path: PathBuf::from("/tmp/a"),
                    missing: false,
                },
                AssetHealth {
                    entry: asset_b.clone(),
                    copied_absolute_path: PathBuf::from("/tmp/b"),
                    missing: false,
                },
            ],
            ..SheetsState::default()
        };
        state.create_blank_template();
        let template_id = state.templates[0].template_id.clone();
        state.templates[0].placements.push(AssetPlacement {
            placement_id: AssetPlacementId::from_string("p-bottom"),
            asset_id: asset_a.asset_id,
            position: TemplateRect {
                x: 0.1,
                y: 0.1,
                w: 0.4,
                h: 0.4,
            },
            z_index: 0,
            extensions: ExtensionPayload::default(),
        });
        state.templates[0].placements.push(AssetPlacement {
            placement_id: AssetPlacementId::from_string("p-top"),
            asset_id: asset_b.asset_id,
            position: TemplateRect {
                x: 0.2,
                y: 0.2,
                w: 0.4,
                h: 0.4,
            },
            z_index: 5,
            extensions: ExtensionPayload::default(),
        });
        state.selected_template_id = Some(template_id);
        let sheets = use_signal(|| state);
        rsx! { SheetsCanvas { sheets, on_import_image: move |()| {}, on_arm_capture: move |_| {} } }
    }
    let mut vdom = VirtualDom::new(Harness);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    let bottom = html.find("data-placement-id=\"p-bottom\"").unwrap();
    let top = html.find("data-placement-id=\"p-top\"").unwrap();
    assert!(
        bottom < top,
        "lower-z placement should render before higher-z so DOM order matches paint order: {html}"
    );
}

#[test]
fn selecting_a_frame_renders_eight_resize_handles_and_primary_border() {
    use inputforge_core::sheet::{AssetPlacement, AssetPlacementId, TemplateRect};
    #[expect(non_snake_case, reason = "Dioxus component")]
    fn Harness() -> Element {
        let asset_id = AssetId::from_string("asset-x");
        let asset = AssetEntry {
            asset_id: asset_id.clone(),
            copied_path: PathBuf::from("assets/asset-x.png"),
            content_hash: "hx".to_owned(),
            media_type: "image/png".to_owned(),
            pixel_dimensions: PixelDimensions {
                width: 64,
                height: 32,
            },
            original_import_path: None,
            extensions: ExtensionPayload::default(),
        };
        let mut state = SheetsState {
            assets: vec![asset.clone()],
            asset_health: vec![AssetHealth {
                entry: asset,
                copied_absolute_path: PathBuf::from("/tmp/x"),
                missing: false,
            }],
            ..SheetsState::default()
        };
        state.create_blank_template();
        let placement_id = AssetPlacementId::from_string("p-sel");
        state.templates[0].placements.push(AssetPlacement {
            placement_id: placement_id.clone(),
            asset_id,
            position: TemplateRect {
                x: 0.1,
                y: 0.1,
                w: 0.4,
                h: 0.4,
            },
            z_index: 0,
            extensions: ExtensionPayload::default(),
        });
        state.selected_placement_id = Some(placement_id);
        let sheets = use_signal(|| state);
        rsx! { SheetsCanvas { sheets, on_import_image: move |()| {}, on_arm_capture: move |_| {} } }
    }
    let mut vdom = VirtualDom::new(Harness);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);

    let handle_count = html.matches("class=\"if-sheets__resize-handle").count();
    assert_eq!(
        handle_count, 8,
        "selected frame should render 8 resize handles: {html}"
    );
    assert!(html.contains("data-selected=\"true\""));
}

#[test]
fn anchor_shape_classes_match_assignment_state_and_selection_is_additive() {
    use inputforge_core::sheet::{AnchorAssignment, AnchorBinding, TemplateAnchor};
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    #[expect(non_snake_case, reason = "Dioxus component")]
    fn Harness(props: SheetsState) -> Element {
        let sheets = use_signal(|| props);
        rsx! { SheetsCanvas { sheets, on_import_image: move |()| {}, on_arm_capture: move |_| {} } }
    }

    let mut placed_unassigned = SheetsState::default();
    placed_unassigned.create_blank_template();
    placed_unassigned.templates[0].anchors.push(TemplateAnchor {
        anchor_id: AnchorId::from_string("a-1"),
        label: "L".to_owned(),
        position: AnchorPosition { x: 0.5, y: 0.5 },
        attached_to: None,
        input_type_hint: None,
        grouping_hint: None,
        device_matching_hint: None,
        extensions: ExtensionPayload::default(),
    });
    let mut vdom = VirtualDom::new_with_props(Harness, placed_unassigned);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);
    assert!(html.contains("if-sheets__anchor--unassigned"));
    assert!(!html.contains("if-sheets__anchor--captured"));

    let mut captured = SheetsState::default();
    captured.create_blank_template();
    captured.templates[0].anchors.push(TemplateAnchor {
        anchor_id: AnchorId::from_string("a-2"),
        label: "L".to_owned(),
        position: AnchorPosition { x: 0.5, y: 0.5 },
        attached_to: None,
        input_type_hint: None,
        grouping_hint: None,
        device_matching_hint: None,
        extensions: ExtensionPayload::default(),
    });
    captured.templates[0]
        .default_anchor_bindings
        .push(AnchorBinding {
            anchor_id: AnchorId::from_string("a-2"),
            input: InputAddress::Bound {
                device: DeviceId("d".to_owned()),
                input: InputId::Button { index: 1 },
            },
            captured_device_fingerprint: None,
            assignment: AnchorAssignment::Captured,
        });
    captured.selected_anchor_id = Some(AnchorId::from_string("a-2"));
    let mut vdom = VirtualDom::new_with_props(Harness, captured);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);
    assert!(html.contains("if-sheets__anchor--captured"));
    assert!(html.contains("if-sheets__anchor--selected"));
    assert!(html.contains("if-sheets__anchor-label"));
}

#[test]
fn unavailable_assignment_renders_unassigned_shape() {
    use inputforge_core::sheet::{AnchorAssignment, AnchorBinding, TemplateAnchor};
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    #[expect(non_snake_case, reason = "Dioxus component")]
    fn Harness(props: SheetsState) -> Element {
        let sheets = use_signal(|| props);
        rsx! { SheetsCanvas { sheets, on_import_image: move |()| {}, on_arm_capture: move |_| {} } }
    }

    let mut state = SheetsState::default();
    state.create_blank_template();
    state.templates[0].anchors.push(TemplateAnchor {
        anchor_id: AnchorId::from_string("a-u"),
        label: "U".to_owned(),
        position: AnchorPosition { x: 0.5, y: 0.5 },
        attached_to: None,
        input_type_hint: None,
        grouping_hint: None,
        device_matching_hint: None,
        extensions: ExtensionPayload::default(),
    });
    state.templates[0]
        .default_anchor_bindings
        .push(AnchorBinding {
            anchor_id: AnchorId::from_string("a-u"),
            input: InputAddress::Bound {
                device: DeviceId("d".to_owned()),
                input: InputId::Button { index: 1 },
            },
            captured_device_fingerprint: None,
            assignment: AnchorAssignment::Unavailable,
        });

    let mut vdom = VirtualDom::new_with_props(Harness, state);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);
    assert!(html.contains("if-sheets__anchor--unassigned"));
    assert!(!html.contains("if-sheets__anchor--captured"));
    assert!(!html.contains("if-sheets__anchor--manual"));
}

#[test]
fn autosave_chip_block_does_not_reference_color_live() {
    let css = include_str!("../../../assets/frame/sheets.css");
    let mut autosave_block = String::new();
    let mut in_block = false;
    for line in css.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with(".if-sheets__autosave-") {
            in_block = true;
        }
        if in_block {
            autosave_block.push_str(line);
            autosave_block.push('\n');
            if line.contains('}') && !line.contains('{') {
                in_block = false;
            }
        }
    }
    assert!(
        !autosave_block.contains("var(--color-live)"),
        "autosave chip must not use the engine-truth colour: {autosave_block}"
    );
}

#[test]
fn sheets_css_defines_autosave_status_classes_for_clean_saving_dirty_failed() {
    let css = include_str!("../../../assets/frame/sheets.css");
    for token in [
        "data-status=\"clean\"",
        "data-status=\"saving\"",
        "data-status=\"dirty\"",
        "data-status=\"failed\"",
    ] {
        assert!(css.contains(token), "css missing autosave token {token}");
    }
}

#[test]
fn every_non_flex_display_value_in_sheets_css_has_inline_justification() {
    let css = include_str!("../../../assets/frame/sheets.css");
    for (line_no, line) in css.lines().enumerate() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("display:") {
            continue;
        }
        if trimmed.starts_with("display: flex") || trimmed.starts_with("display:flex") {
            continue;
        }
        let after_value = trimmed.split(';').next().unwrap_or("");
        assert!(
            line.contains("/*"),
            "line {} has non-flex display without inline /* ... */ justification: {}",
            line_no + 1,
            after_value
        );
    }
}

#[test]
fn sheets_css_defines_all_four_anchor_shape_classes_and_halo() {
    let css = include_str!("../../../assets/frame/sheets.css");
    for token in [
        ".if-sheets__anchor--unassigned",
        ".if-sheets__anchor--captured",
        ".if-sheets__anchor--manual",
        ".if-sheets__anchor--selected",
        ".if-sheets__anchor-label",
        ".if-sheets__empty-dropzone",
        ".if-sheets__preset-slot",
        ".if-sheets__resize-handle",
        ".if-sheets__toolbar-segment",
    ] {
        assert!(css.contains(token), "css missing class {token}");
    }
    assert!(css.contains("var(--color-bg-elevated)"));
    assert!(css.contains("var(--color-border-strong)"));
}

#[test]
fn manual_anchor_donut_uses_a_4px_inset_for_the_inner_cutout() {
    let css = include_str!("../../../assets/frame/sheets.css");
    assert!(
        css.contains("inset 0 0 0 4px var(--color-bg-elevated)"),
        "manual anchor box-shadow must include the 4px-inset cutout"
    );
}

#[test]
fn color_border_focus_is_only_used_inside_focus_visible_selectors() {
    let css = include_str!("../../../assets/frame/sheets.css");
    for rule in css.split('}') {
        if !rule.contains("var(--color-border-focus)") {
            continue;
        }
        assert!(
            rule.contains(":focus-visible"),
            "--color-border-focus is reserved for :focus-visible only, found in rule: {rule}"
        );
    }
}

#[test]
fn bring_to_front_updates_render_order_in_dom() {
    #[expect(non_snake_case, reason = "Dioxus component")]
    fn Harness() -> Element {
        let mut state = SheetsState::default();
        state.create_blank_template();
        // Seed asset_health so the placement render path executes (not the missing-asset recovery panel).
        let asset_id = AssetId::from_string("a");
        state.assets.push(AssetEntry {
            asset_id: asset_id.clone(),
            copied_path: PathBuf::from("a.png"),
            content_hash: "h".to_owned(),
            media_type: "image/png".to_owned(),
            pixel_dimensions: PixelDimensions {
                width: 1,
                height: 1,
            },
            original_import_path: None,
            extensions: ExtensionPayload::default(),
        });
        state.asset_health.push(AssetHealth {
            entry: state.assets[0].clone(),
            copied_absolute_path: PathBuf::from("/tmp/a"),
            missing: false,
        });
        let template_id = state.templates[0].template_id.clone();
        let bottom = AssetPlacementId::from_string("p-bottom");
        let top = AssetPlacementId::from_string("p-top");
        for (id, z) in [(bottom.clone(), 0), (top.clone(), 1)] {
            state.templates[0].placements.push(AssetPlacement {
                placement_id: id,
                asset_id: asset_id.clone(),
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
        state.bring_to_front(template_id, bottom).unwrap();
        let sheets = use_signal(|| state);
        rsx! { SheetsCanvas { sheets, on_import_image: move |()| {}, on_arm_capture: move |_| {} } }
    }
    let mut vdom = VirtualDom::new(Harness);
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);
    let bottom = html.find("data-placement-id=\"p-bottom\"").unwrap();
    let top = html.find("data-placement-id=\"p-top\"").unwrap();
    assert!(
        top < bottom,
        "after bring_to_front, the previously-bottom placement renders last"
    );
}

#[test]
fn z_index_input_commits_one_reorder_z_event_on_blur() {
    let mut state = SheetsState::default();
    state.create_blank_template();
    let template_id = state.templates[0].template_id.clone();
    let placement_id = AssetPlacementId::from_string("p");
    state.templates[0].placements.push(AssetPlacement {
        placement_id: placement_id.clone(),
        asset_id: AssetId::from_string("a"),
        position: TemplateRect {
            x: 0.0,
            y: 0.0,
            w: 0.5,
            h: 0.5,
        },
        z_index: 1,
        extensions: ExtensionPayload::default(),
    });
    state.selected_placement_id = Some(placement_id.clone());

    let baseline = state.history.len();
    state.set_z_index(template_id, placement_id, 5).unwrap();
    assert_eq!(state.history.len() - baseline, 1);
    assert!(matches!(
        state.history.back().unwrap().kind,
        SheetsEventKind::ReorderZ { .. }
    ));
}

#[test]
fn template_display_name_input_ellipsizes_at_seventy_plus_characters() {
    let css = include_str!("../../../assets/frame/sheets.css");
    assert!(css.contains("text-overflow: ellipsis"));
}

#[test]
fn selection_state_cycle_template_to_frame_to_anchor_to_template() {
    let mut state = SheetsState::default();
    state.create_blank_template();
    assert!(state.selected_placement_id.is_none() && state.selected_anchor_id.is_none());

    let placement_id = AssetPlacementId::from_string("p");
    state.templates[0].placements.push(AssetPlacement {
        placement_id: placement_id.clone(),
        asset_id: AssetId::from_string("a"),
        position: TemplateRect {
            x: 0.0,
            y: 0.0,
            w: 0.5,
            h: 0.5,
        },
        z_index: 0,
        extensions: ExtensionPayload::default(),
    });
    state.selected_placement_id = Some(placement_id);
    assert!(state.selected_placement_id.is_some());

    let _anchor_id = state.place_anchor(AnchorPosition { x: 0.5, y: 0.5 });
    assert!(state.selected_anchor_id.is_some());

    state.select_template(state.templates[0].template_id.clone());
    assert!(state.selected_placement_id.is_none());
    assert!(state.selected_anchor_id.is_none());
}

#[test]
fn pressing_assign_arms_capture_and_renders_listening_visual() {
    let mut state = state_with_selected_anchor();
    let anchor_id = state.selected_anchor_id.clone().unwrap();
    state.arm_capture(anchor_id.clone());
    assert!(matches!(state.capture, CaptureStatus::Armed(ref id) if id == &anchor_id));

    let html = render_inspector_state(state);
    assert!(html.contains("if-rebind-composite__listening"));
}

#[test]
fn captured_input_commits_anchor_binding_with_assignment_captured() {
    use inputforge_core::sheet::AnchorAssignment;
    use inputforge_core::types::{InputAddress, InputId};
    let mut state = state_with_selected_anchor();
    let anchor_id = state.selected_anchor_id.clone().unwrap();
    let template_id = state.selected_template_id.clone().unwrap();

    let baseline = state.history.len();
    state.assign_selected_anchor(
        InputAddress::Bound {
            device: DeviceId("d-1".to_owned()),
            input: InputId::Button { index: 7 },
        },
        AnchorAssignment::Captured,
    );

    let binding = state
        .templates
        .iter()
        .find(|t| t.template_id == template_id)
        .unwrap()
        .default_anchor_bindings
        .iter()
        .find(|b| b.anchor_id == anchor_id)
        .expect("anchor binding recorded after capture");
    assert!(matches!(binding.assignment, AnchorAssignment::Captured));
    assert_eq!(state.history.len() - baseline, 1);
    assert!(matches!(
        state.history.back().unwrap().kind,
        SheetsEventKind::AssignAnchor { .. }
    ));
}

#[test]
fn manual_assign_commits_anchor_binding_with_assignment_manual() {
    use super::state::ManualInputKind;
    use inputforge_core::sheet::AnchorAssignment;
    let mut state = state_with_selected_anchor();
    let anchor_id = state.selected_anchor_id.clone().unwrap();
    let template_id = state.selected_template_id.clone().unwrap();

    let baseline = state.history.len();
    state
        .assign_manual_selected_anchor("d-2", ManualInputKind::Button, 3)
        .unwrap();

    let binding = state
        .templates
        .iter()
        .find(|t| t.template_id == template_id)
        .unwrap()
        .default_anchor_bindings
        .iter()
        .find(|b| b.anchor_id == anchor_id)
        .expect("manual binding recorded");
    assert!(matches!(binding.assignment, AnchorAssignment::Manual));
    assert_eq!(state.history.len() - baseline, 1);
    assert!(matches!(
        state.history.back().unwrap().kind,
        SheetsEventKind::AssignAnchor { .. }
    ));
}

#[test]
fn placement_wrapper_is_positioned_absolutely_and_blocks_user_select() {
    let css = include_str!("../../../assets/frame/sheets.css");
    let mut block = String::new();
    let mut in_block = false;
    let mut depth = 0_i32;
    for line in css.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with(".if-sheets__placement {") {
            in_block = true;
        }
        if in_block {
            block.push_str(line);
            block.push('\n');
            depth += line.matches('{').count() as i32;
            depth -= line.matches('}').count() as i32;
            if depth <= 0 {
                in_block = false;
            }
        }
    }
    assert!(
        block.contains("position: absolute"),
        "placement wrapper missing position: absolute: {block}"
    );
    assert!(
        block.contains("user-select: none"),
        "placement wrapper missing user-select: none: {block}"
    );
}

#[test]
fn placement_image_uses_object_fit_contain_and_fills_wrapper() {
    let css = include_str!("../../../assets/frame/sheets.css");
    let mut block = String::new();
    let mut in_block = false;
    let mut depth = 0_i32;
    for line in css.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with(".if-sheets__placement-image {") {
            in_block = true;
        }
        if in_block {
            block.push_str(line);
            block.push('\n');
            depth += line.matches('{').count() as i32;
            depth -= line.matches('}').count() as i32;
            if depth <= 0 {
                in_block = false;
            }
        }
    }
    assert!(
        block.contains("object-fit: contain"),
        "image rule missing object-fit: contain: {block}"
    );
    assert!(
        block.contains("width: 100%"),
        "image rule missing width: 100%: {block}"
    );
    assert!(
        block.contains("height: 100%"),
        "image rule missing height: 100%: {block}"
    );
    assert!(
        block.contains("pointer-events: none"),
        "image rule missing pointer-events: none: {block}"
    );
}

#[test]
fn placement_image_carries_draggable_false_attribute() {
    #[expect(non_snake_case, reason = "Dioxus component")]
    fn Harness(copied_path: PathBuf) -> Element {
        let mut state = SheetsState::default();
        state.create_blank_template();
        let asset_id = AssetId::from_string("a");
        state.assets.push(AssetEntry {
            asset_id: asset_id.clone(),
            copied_path: PathBuf::from("a.png"),
            content_hash: "h".to_owned(),
            media_type: "image/png".to_owned(),
            pixel_dimensions: PixelDimensions {
                width: 1,
                height: 1,
            },
            original_import_path: None,
            extensions: ExtensionPayload::default(),
        });
        state.asset_health.push(AssetHealth {
            entry: state.assets[0].clone(),
            copied_absolute_path: copied_path,
            missing: false,
        });
        state.templates[0].placements.push(AssetPlacement {
            placement_id: AssetPlacementId::from_string("p"),
            asset_id,
            position: TemplateRect {
                x: 0.1,
                y: 0.1,
                w: 0.4,
                h: 0.4,
            },
            z_index: 0,
            extensions: ExtensionPayload::default(),
        });
        let sheets = use_signal(|| state);
        rsx! { SheetsCanvas { sheets, on_import_image: move |()| {}, on_arm_capture: move |_| {} } }
    }
    let copied_path =
        std::env::temp_dir().join(format!("inputforge-sheets-{}.png", ulid::Ulid::new()));
    std::fs::write(&copied_path, [0x89, b'P', b'N', b'G']).unwrap();
    let mut vdom = VirtualDom::new_with_props(Harness, copied_path.clone());
    vdom.rebuild_in_place();
    let html = dioxus_ssr::render(&vdom);
    let _ = std::fs::remove_file(copied_path);
    assert!(
        html.contains("draggable=\"false\""),
        "placement image must render draggable=\"false\": {html}"
    );
}

#[test]
fn toolbar_plus_import_emits_one_import_asset_event() {
    let mut state = SheetsState::default();
    let baseline = state.history.len();

    state.record_imported_asset(AssetEntry {
        asset_id: AssetId::from_string("asset-toolbar"),
        copied_path: PathBuf::from("assets/asset-toolbar.png"),
        content_hash: "hash".to_owned(),
        media_type: "image/png".to_owned(),
        pixel_dimensions: PixelDimensions {
            width: 1,
            height: 1,
        },
        original_import_path: None,
        extensions: ExtensionPayload::default(),
    });

    assert_eq!(state.history.len() - baseline, 1);
    assert!(matches!(
        state.history.back().unwrap().kind,
        SheetsEventKind::ImportAsset { .. }
    ));
}

#[test]
fn frame_inspector_shows_asset_filename() {
    let mut state = SheetsState::default();
    state.create_blank_template();
    let asset_id = AssetId::from_string("asset-frame-1");
    state.assets.push(AssetEntry {
        asset_id: asset_id.clone(),
        copied_path: PathBuf::from("assets/throttle.png"),
        content_hash: "h".to_owned(),
        media_type: "image/png".to_owned(),
        pixel_dimensions: PixelDimensions {
            width: 1,
            height: 1,
        },
        original_import_path: Some(PathBuf::from("D:/imports/throttle.png")),
        extensions: ExtensionPayload::default(),
    });
    let placement_id = AssetPlacementId::from_string("p-1");
    state.templates[0].placements.push(AssetPlacement {
        placement_id: placement_id.clone(),
        asset_id,
        position: TemplateRect {
            x: 0.1,
            y: 0.1,
            w: 0.4,
            h: 0.4,
        },
        z_index: 0,
        extensions: ExtensionPayload::default(),
    });
    state.selected_placement_id = Some(placement_id);
    let html = render_inspector_state(state);
    assert!(
        html.contains("throttle.png"),
        "Frame inspector must show asset filename: {html}"
    );
}

#[test]
fn frame_x_input_commits_on_blur_via_commit_drag_end_move() {
    // State-level mirror of what the inspector's onblur handler does: parse
    // the draft, clamp into the canvas, and commit via commit_drag_end_move.
    let mut state = SheetsState::default();
    state.create_blank_template();
    let template_id = state.templates[0].template_id.clone();
    let placement_id = AssetPlacementId::from_string("p-x");
    state.templates[0].placements.push(AssetPlacement {
        placement_id: placement_id.clone(),
        asset_id: AssetId::from_string("a"),
        position: TemplateRect {
            x: 0.1,
            y: 0.2,
            w: 0.3,
            h: 0.4,
        },
        z_index: 0,
        extensions: ExtensionPayload::default(),
    });
    let baseline = state.history.len();
    state
        .commit_drag_end_move(template_id, placement_id.clone(), 0.5_f32, 0.2_f32)
        .unwrap();
    let rect = state.templates[0]
        .placements
        .iter()
        .find(|p| p.placement_id == placement_id)
        .unwrap()
        .position;
    assert!((rect.x - 0.5).abs() < 1e-6);
    assert!((rect.y - 0.2).abs() < 1e-6);
    assert_eq!(state.history.len() - baseline, 1);
}

#[test]
fn frame_inspector_renders_move_up_move_down_and_delete_buttons() {
    let mut state = SheetsState::default();
    state.create_blank_template();
    let placement_id = AssetPlacementId::from_string("p");
    state.templates[0].placements.push(AssetPlacement {
        placement_id: placement_id.clone(),
        asset_id: AssetId::from_string("a"),
        position: TemplateRect {
            x: 0.1,
            y: 0.1,
            w: 0.4,
            h: 0.4,
        },
        z_index: 0,
        extensions: ExtensionPayload::default(),
    });
    state.selected_placement_id = Some(placement_id);
    let html = render_inspector_state(state);
    assert!(
        html.contains("Move up"),
        "Frame branch must render Move up button: {html}"
    );
    assert!(
        html.contains("Move down"),
        "Frame branch must render Move down button: {html}"
    );
    assert!(
        html.contains(">Delete<"),
        "Frame branch must render a Delete button: {html}"
    );
}

#[test]
fn frame_delete_invokes_remove_placement() {
    // State-level test: calling remove_placement removes the placement and
    // pushes RemovePlacement. This mirrors what the inspector's Delete button
    // onclick does.
    let mut state = SheetsState::default();
    state.create_blank_template();
    let template_id = state.templates[0].template_id.clone();
    let placement_id = AssetPlacementId::from_string("p-delete");
    state.templates[0].placements.push(AssetPlacement {
        placement_id: placement_id.clone(),
        asset_id: AssetId::from_string("a"),
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
    state.remove_placement(template_id, &placement_id).unwrap();
    assert!(
        state.templates[0]
            .placements
            .iter()
            .all(|p| p.placement_id != placement_id)
    );
    assert_eq!(state.history.len() - baseline, 1);
}

#[test]
fn frame_inspector_inputs_render_with_event_handlers() {
    // SSR test: each axis input renders with the expected `data-axis` and
    // none of them is read-only. Direct handler-presence assertions are
    // impossible in SSR, but the absence of `readonly` plus the presence of
    // every data-axis attribute is sufficient to detect the bug where the
    // inputs were rendered with only `value:` and no event hooks.
    let mut state = SheetsState::default();
    state.create_blank_template();
    let placement_id = AssetPlacementId::from_string("p");
    state.templates[0].placements.push(AssetPlacement {
        placement_id: placement_id.clone(),
        asset_id: AssetId::from_string("a"),
        position: TemplateRect {
            x: 0.1,
            y: 0.2,
            w: 0.3,
            h: 0.4,
        },
        z_index: 0,
        extensions: ExtensionPayload::default(),
    });
    state.selected_placement_id = Some(placement_id);
    let html = render_inspector_state(state);
    for axis in ["x", "y", "w", "h"] {
        assert!(
            html.contains(&format!("data-axis=\"{axis}\"")),
            "missing data-axis={axis}: {html}"
        );
    }
    assert!(
        !html.contains("readonly"),
        "frame inputs must not be readonly: {html}"
    );
}

#[test]
fn anchor_inspector_renders_x_y_inputs_with_correct_values() {
    let mut state = state_with_selected_anchor();
    state.templates[0].anchors[0].position = AnchorPosition { x: 0.42, y: 0.73 };
    let html = render_inspector_state(state);
    for axis in ["x", "y"] {
        assert!(
            html.contains(&format!("data-axis=\"{axis}\"")),
            "anchor branch missing data-axis={axis}: {html}"
        );
    }
    assert!(
        html.contains("0.42"),
        "anchor x input value should reflect position: {html}"
    );
    assert!(
        html.contains("0.73"),
        "anchor y input value should reflect position: {html}"
    );
}

#[test]
fn anchor_inspector_renders_delete_button() {
    let state = state_with_selected_anchor();
    let html = render_inspector_state(state);
    // Multiple ">Delete<" matches are possible if frame Delete is also rendered, but the anchor
    // branch render path doesn't render the frame branch concurrently. Still, scope to the anchor
    // branch by also asserting the ANCHOR eyebrow is present.
    assert!(html.contains(">ANCHOR<"));
    assert!(
        html.contains(">Delete<"),
        "anchor branch must render a Delete button: {html}"
    );
}

#[test]
fn anchor_delete_invokes_remove_selected_anchor() {
    let mut state = state_with_selected_anchor();
    let anchor_id = state.selected_anchor_id.clone().unwrap();
    let baseline = state.history.len();
    state.remove_selected_anchor().unwrap();
    assert!(
        state.templates[0]
            .anchors
            .iter()
            .all(|a| a.anchor_id != anchor_id)
    );
    assert!(state.selected_anchor_id.is_none());
    assert_eq!(state.history.len() - baseline, 1);
}
