#![expect(
    unused_qualifications,
    reason = "rsx! macro expansion reports event handler field names as unnecessary qualifications"
)]

use dioxus::prelude::*;

use inputforge_core::sheet::{
    AnchorAssignment, AnchorId, AnchorPosition, AssetEntry, AssetId, AssetPlacement,
    AssetPlacementId, TemplateAnchor, TemplateRect,
};
use inputforge_core::types::{DeviceId, DeviceInfo, InputAddress, InputId};

use crate::components::{Button, ButtonVariant, Select, SelectOption, TextInput};
use crate::context::ConfigSnapshot;
use crate::frame::mapping_list::source_label;
use crate::frame::sheets::canvas::asset_data_url;
use crate::frame::sheets::state::{
    AutosaveStatus, CaptureAvailabilityReason, CaptureStatus, ManualInputKind, SheetsState,
    asset_label_for, filename_of, placement_default_label_for, placement_label_for, pluralize,
};
use crate::patterns::DestructiveConfirmDialog;
use crate::patterns::live_capture::rebind_composite_class;

#[component]
pub(crate) fn SheetsInspector(
    sheets: Signal<SheetsState>,
    #[props(default)] capture_availability: CaptureAvailabilityReason,
    #[props(default)] connected_devices: Vec<(DeviceId, String)>,
    #[props(default)] config: Option<ConfigSnapshot>,
    on_arm_capture: EventHandler<AnchorId>,
    on_cancel_capture: EventHandler<()>,
    on_retry_save: EventHandler<()>,
) -> Element {
    let mut manual_device_id = use_signal(String::new);
    // Encoded input selection: "axis:N" / "button:N" / "hat:N", or "" for none.
    let mut manual_input_sel = use_signal(String::new);
    // Tracks the (anchor, binding) the manual selects were last seeded from, so the
    // seed re-runs only when the binding actually changes (see seeding block below).
    let mut last_seeded_binding_key =
        use_signal(|| None::<(Option<AnchorId>, Option<InputAddress>)>);

    // Local drafts for the four x/y/w/h frame inputs. They hold the in-progress
    // typed value until the user blurs the field or presses Enter, at which
    // point the value is parsed, clamped, and committed via the state helper.
    // Drafts reset whenever the selected placement changes, so a freshly
    // selected frame shows its persisted values rather than stale text from a
    // previously focused field.
    let mut frame_x_draft = use_signal::<Option<String>>(|| None);
    let mut frame_y_draft = use_signal::<Option<String>>(|| None);
    let mut frame_w_draft = use_signal::<Option<String>>(|| None);
    let mut frame_h_draft = use_signal::<Option<String>>(|| None);

    // Local drafts for the anchor x/y inputs. Same draft/commit-on-blur
    // pattern as the frame rect drafts above. Reset whenever the selected
    // anchor changes so a fresh anchor renders its persisted position.
    let mut anchor_x_draft = use_signal::<Option<String>>(|| None);
    let mut anchor_y_draft = use_signal::<Option<String>>(|| None);

    // Local signal that holds the in-flight Name input text. Mirrors the mapping
    // editor's rename pattern (see `frame::mapping_list::rename_inline`): keystrokes
    // only mutate this LOCAL signal so they do not trigger a SheetsInspector-wide
    // re-render per keystroke, which is what made the previous state-backed-draft
    // implementation feel laggy on fast typing. Commits land via blur / Enter, which
    // read this signal and call `rename_selected_placement` directly.
    let mut frame_name_signal: Signal<String> = use_signal(String::new);
    // Tracks the placement_id the local Name signal was last seeded for, so we can
    // reseed during render (before TextInput reads the signal) whenever the user
    // switches placements. A `use_effect` would run too late for the initial render
    // and for dioxus_ssr's single-pass renderer used in tests.
    let mut last_seeded_name_placement_id: Signal<Option<AssetPlacementId>> = use_signal(|| None);

    // Same pattern as the frame Name input above, but for the Asset inspector's
    // Name field. Local signal so each keystroke does not re-render the whole
    // inspector; tracker signal so we re-seed when the user picks a different
    // asset in the rail.
    let mut asset_name_signal: Signal<String> = use_signal(String::new);
    let mut last_seeded_name_asset_id: Signal<Option<AssetId>> = use_signal(|| None);
    // Controls the destructive-confirm delete modal opened by the Asset
    // inspector's Delete button.
    let delete_asset_modal_open: Signal<bool> = use_signal(|| false);

    // Pull every read-only snapshot we need in one borrow so we never hold a `read()` guard
    // across an `rsx!` expression that also has to `write()` to the same signal.
    let template_name_draft = sheets.read().template_display_name_draft.clone();
    let anchor_label_draft = sheets.read().anchor_label_draft.clone();
    let (
        save_status,
        selected_template_name,
        selected_template_id,
        selected_anchor_id,
        selected_placement_id,
        selected_asset_id,
        selected_asset_entry,
        selected_asset_health,
        asset_usage,
        selected_anchor,
        selected_anchor_canvas,
        selected_placement,
        selected_binding,
        capture,
        last_error,
        frame_count,
        anchor_count,
        placement_options,
    ) = {
        let state = sheets.read();
        let selected_anchor_id = state.selected_anchor_id.clone();
        let selected_placement_id = state.selected_placement_id.clone();
        let selected_template_id = state.selected_template_id.clone();
        // Resolve the asset to inspect by `inspector_asset_id`, the rail-click
        // intent field. The canvas-tracking `selected_asset_id` is set by
        // `select_template` to the active template's first placement asset and
        // is read by the canvas image renderer; reading IT here would trap the
        // Asset branch every time the user clicks a template. The two fields
        // are deliberately kept separate so template clicks land in TEMPLATE
        // and asset clicks land in ASSET.
        let selected_asset_id = state.inspector_asset_id.clone();
        let selected_asset_entry: Option<AssetEntry> = selected_asset_id
            .as_ref()
            .and_then(|id| state.assets.iter().find(|a| &a.asset_id == id).cloned());
        let selected_asset_health = selected_asset_id
            .as_ref()
            .and_then(|id| state.asset_health.iter().find(|h| &h.entry.asset_id == id))
            .cloned();
        let asset_usage = selected_asset_id
            .as_ref()
            .map(|id| state.asset_usage(id))
            .unwrap_or_default();
        let selected_template = state.selected_template();
        let selected_template_name = selected_template.map(|t| t.display_name.clone());
        let frame_count = selected_template.map_or(0, |t| t.placements.len());
        let anchor_count = selected_template.map_or(0, |t| t.anchors.len());
        // (placement_id, dropdown label) tuples for the anchor "Attached to" picker.
        // Routed through placement_label_for so the user-supplied display_name wins,
        // then the asset filename, then "Frame N".
        let placement_options: Vec<(AssetPlacementId, String)> = selected_template
            .map(|template| {
                template
                    .placements
                    .iter()
                    .enumerate()
                    .map(|(idx, placement)| {
                        let label = placement_label_for(placement, &state.assets, idx);
                        (placement.placement_id.clone(), label)
                    })
                    .collect()
            })
            .unwrap_or_default();

        let selected_anchor: Option<TemplateAnchor> = selected_template.and_then(|template| {
            let anchor_id = selected_anchor_id.as_ref()?;
            template
                .anchors
                .iter()
                .find(|anchor| &anchor.anchor_id == anchor_id)
                .cloned()
        });
        // Canvas-relative position of the selected anchor: the inspector shows what the user
        // sees on the canvas, not the placement-local storage value used for attached anchors.
        let selected_anchor_canvas: Option<(f64, f64)> = selected_template
            .zip(selected_anchor.as_ref())
            .map(|(template, anchor)| {
                crate::frame::sheets::canvas::anchor_canvas_coords(anchor, &template.placements)
            });
        let selected_placement: Option<AssetPlacement> = selected_template.and_then(|template| {
            let placement_id = selected_placement_id.as_ref()?;
            template
                .placements
                .iter()
                .find(|placement| &placement.placement_id == placement_id)
                .cloned()
        });
        let selected_binding = selected_template
            .and_then(|template| {
                let anchor_id = selected_anchor_id.as_ref()?;
                template
                    .default_anchor_bindings
                    .iter()
                    .find(|binding| &binding.anchor_id == anchor_id)
            })
            .map(|binding| (binding.input.clone(), binding.assignment));

        (
            match state.autosave {
                AutosaveStatus::Clean => "Saved",
                AutosaveStatus::Dirty => "Unsaved",
                AutosaveStatus::Saving => "Saving",
                AutosaveStatus::Failed => "Save failed",
            },
            selected_template_name,
            selected_template_id,
            selected_anchor_id,
            selected_placement_id,
            selected_asset_id,
            selected_asset_entry,
            selected_asset_health,
            asset_usage,
            selected_anchor,
            selected_anchor_canvas,
            selected_placement,
            selected_binding,
            state.capture.clone(),
            state.last_error.clone(),
            frame_count,
            anchor_count,
            placement_options,
        )
    };

    // Seed value for the local frame Name signal. Derived from the committed
    // display_name of the currently selected placement. We re-apply this seed to
    // `frame_name_signal` during render (not via use_effect) whenever the user
    // switches placements, using `last_seeded_name_placement_id` to avoid clobbering
    // the in-flight typed text while the same placement remains selected.
    let frame_name_seed = selected_placement
        .as_ref()
        .and_then(|placement| placement.display_name.clone())
        .unwrap_or_default();
    if last_seeded_name_placement_id.peek().as_ref() != selected_placement_id.as_ref() {
        last_seeded_name_placement_id.set(selected_placement_id.clone());
        frame_name_signal.set(frame_name_seed.clone());
    }

    // Same seeding rule for the Asset inspector's Name input. Seeded from the
    // committed asset display_name and refreshed during render whenever the user
    // picks a different asset in the rail.
    let asset_name_seed = selected_asset_entry
        .as_ref()
        .and_then(|asset| asset.display_name.clone())
        .unwrap_or_default();
    if last_seeded_name_asset_id.peek().as_ref() != selected_asset_id.as_ref() {
        last_seeded_name_asset_id.set(selected_asset_id.clone());
        asset_name_signal.set(asset_name_seed.clone());
    }

    // Reset the per-axis drafts whenever the selected placement changes so the
    // inputs always render the persisted rect for the active frame instead of
    // stale text from a previous selection.
    let placement_id_for_reset = selected_placement_id.clone();
    use_effect(use_reactive!(|placement_id_for_reset| {
        let _ = placement_id_for_reset;
        frame_x_draft.set(None);
        frame_y_draft.set(None);
        frame_w_draft.set(None);
        frame_h_draft.set(None);
    }));

    // Reset anchor x/y drafts whenever the selected anchor changes so the
    // inputs always render the persisted position for the active anchor.
    let selected_anchor_id_for_reset = selected_anchor_id.clone();
    use_effect(use_reactive!(|selected_anchor_id_for_reset| {
        let _ = selected_anchor_id_for_reset;
        anchor_x_draft.set(None);
        anchor_y_draft.set(None);
    }));

    let is_capture_armed = matches!(capture, CaptureStatus::Armed(_));
    let composer_button_disabled = selected_anchor_id.is_none() || is_capture_armed;
    let composer_help = composer_help_for(&capture, capture_availability);
    let selected_anchor_for_capture = selected_anchor_id.clone();
    let mut sheets_for_template_name = sheets;
    let mut sheets_for_anchor_label = sheets;
    let mut sheets_for_attach = sheets;
    let mut sheets_for_bring_to_front = sheets;
    let mut sheets_for_send_to_back = sheets;
    let mut sheets_for_shift_z_up = sheets;
    let mut sheets_for_shift_z_down = sheets;
    let mut sheets_for_delete_placement = sheets;
    let mut sheets_for_assign = sheets;

    // Seed the manual selects from the selected anchor's binding so the Device/Input
    // dropdowns mirror the live binding (captured or manual). Re-seed only when the
    // anchor or its binding changes, so a mid-edit device pick is not clobbered.
    let binding_address = selected_binding.as_ref().map(|(addr, _)| addr.clone());
    let binding_seed_key = (selected_anchor_id.clone(), binding_address.clone());
    if last_seeded_binding_key.peek().as_ref() != Some(&binding_seed_key) {
        last_seeded_binding_key.set(Some(binding_seed_key));
        let (device_seed, input_seed) = match binding_address {
            Some(InputAddress::Bound { device, input }) => {
                (device.0, encode_input_selection(&input))
            }
            _ => (String::new(), String::new()),
        };
        manual_device_id.set(device_seed);
        manual_input_sel.set(input_seed);
    }

    // Clones for onclick closures that fire multiple times.
    let bring_to_front_template_id = selected_template_id.clone();
    let bring_to_front_placement_id = selected_placement_id.clone();
    let send_to_back_template_id = selected_template_id.clone();
    let send_to_back_placement_id = selected_placement_id.clone();
    let shift_z_up_template_id = selected_template_id.clone();
    let shift_z_up_placement_id = selected_placement_id.clone();
    let shift_z_down_template_id = selected_template_id.clone();
    let shift_z_down_placement_id = selected_placement_id.clone();
    let delete_placement_template_id = selected_template_id.clone();
    let delete_placement_placement_id = selected_placement_id.clone();

    rsx! {
        aside { "data-testid": "sheets-inspector",
            h2 { class: "if-sheets__inspector-title", "Inspector" }
            p { "{save_status}" }
            if let Some(error) = last_error {
                div { role: "alert",
                    p { "{error}" }
                    button {
                        r#type: "button",
                        onclick: move |_| on_retry_save.call(()),
                        "Retry save"
                    }
                }
            }

            if selected_anchor_id.is_some() {
                // Anchor branch
                section { "data-testid": "sheets-inspector-anchor",
                    span { class: "if-sheets__eyebrow", "ANCHOR" }
                    if let Some(anchor) = selected_anchor {
                        {
                            let displayed_label = anchor_label_draft
                                .clone()
                                .unwrap_or_else(|| anchor.label.clone());
                            let anchor_label_signal: ReadSignal<String> =
                                ReadSignal::new(Signal::new(displayed_label));
                            let mut sheets_for_anchor_label_blur = sheets_for_anchor_label;
                            rsx! {
                                label {
                                    "Label"
                                    TextInput {
                                        value: anchor_label_signal,
                                        oninput: move |evt: FormEvent| {
                                            sheets_for_anchor_label.write().update_anchor_label_draft(evt.value());
                                        },
                                        onblur: move |_| {
                                            sheets_for_anchor_label_blur.write().commit_anchor_label_draft();
                                        },
                                    }
                                }
                            }
                        }
                        {
                            let current_attached: Option<AssetPlacementId> =
                                anchor.attached_to.clone();
                            let placement_options_for_picker = placement_options.clone();
                            rsx! {
                                label {
                                    "Attached to"
                                    select {
                                        onchange: move |evt| {
                                            let raw = evt.value();
                                            let target = if raw.is_empty() {
                                                None
                                            } else {
                                                Some(AssetPlacementId::from_string(raw))
                                            };
                                            let _ = sheets_for_attach
                                                .write()
                                                .set_anchor_attached_to(target);
                                        },
                                        option {
                                            value: "",
                                            selected: current_attached.is_none(),
                                            "Detached"
                                        }
                                        for (placement_id , label) in placement_options_for_picker.iter().cloned() {
                                            {
                                                let is_selected = current_attached.as_ref()
                                                    == Some(&placement_id);
                                                rsx! {
                                                    option {
                                                        value: "{placement_id}",
                                                        selected: is_selected,
                                                        "{label}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        {
                            // The inspector x/y inputs show the canvas-relative position the user
                            // sees on the stage. For attached anchors the storage value differs
                            // (placement-local), so we expose anchor_canvas_coords here and rely on
                            // update_selected_anchor_position to convert canvas -> storage internally.
                            let (canvas_x, canvas_y) = selected_anchor_canvas
                                .unwrap_or((anchor.position.x, anchor.position.y));
                            rsx! {
                                label {
                                    "X"
                                    input {
                                        r#type: "number",
                                        "data-axis": "x",
                                        step: "0.01",
                                        min: "0",
                                        max: "1",
                                        value: "{anchor_x_draft.read().clone().unwrap_or_else(|| format_coord(canvas_x))}",
                                        oninput: move |evt: FormEvent| { anchor_x_draft.set(Some(evt.value())); },
                                        onblur: {
                                            let mut sheets_for_x_blur = sheets;
                                            move |_| {
                                                if let Some(draft) = anchor_x_draft.write().take()
                                                    && let Ok(parsed) = draft.trim().parse::<f64>() {
                                                    let clamped_x = parsed.clamp(0.0, 1.0);
                                                    sheets_for_x_blur.write().update_selected_anchor_position(AnchorPosition { x: clamped_x, y: canvas_y });
                                                }
                                            }
                                        },
                                        onkeydown: {
                                            let mut sheets_for_x_enter = sheets;
                                            move |evt: KeyboardEvent| {
                                                if evt.key() == Key::Enter
                                                    && let Some(draft) = anchor_x_draft.write().take()
                                                    && let Ok(parsed) = draft.trim().parse::<f64>() {
                                                    let clamped_x = parsed.clamp(0.0, 1.0);
                                                    sheets_for_x_enter.write().update_selected_anchor_position(AnchorPosition { x: clamped_x, y: canvas_y });
                                                }
                                            }
                                        },
                                    }
                                }
                                label {
                                    "Y"
                                    input {
                                        r#type: "number",
                                        "data-axis": "y",
                                        step: "0.01",
                                        min: "0",
                                        max: "1",
                                        value: "{anchor_y_draft.read().clone().unwrap_or_else(|| format_coord(canvas_y))}",
                                        oninput: move |evt: FormEvent| { anchor_y_draft.set(Some(evt.value())); },
                                        onblur: {
                                            let mut sheets_for_y_blur = sheets;
                                            move |_| {
                                                if let Some(draft) = anchor_y_draft.write().take()
                                                    && let Ok(parsed) = draft.trim().parse::<f64>() {
                                                    let clamped_y = parsed.clamp(0.0, 1.0);
                                                    sheets_for_y_blur.write().update_selected_anchor_position(AnchorPosition { x: canvas_x, y: clamped_y });
                                                }
                                            }
                                        },
                                        onkeydown: {
                                            let mut sheets_for_y_enter = sheets;
                                            move |evt: KeyboardEvent| {
                                                if evt.key() == Key::Enter
                                                    && let Some(draft) = anchor_y_draft.write().take()
                                                    && let Ok(parsed) = draft.trim().parse::<f64>() {
                                                    let clamped_y = parsed.clamp(0.0, 1.0);
                                                    sheets_for_y_enter.write().update_selected_anchor_position(AnchorPosition { x: canvas_x, y: clamped_y });
                                                }
                                            }
                                        },
                                    }
                                }
                            }
                        }
                    }
                    section {
                        span { class: "if-sheets__eyebrow", "ASSIGNMENT" }
                        // Read-only summary of the current binding, driven by the persisted
                        // `selected_binding` (not transient capture state), so it survives a
                        // reload and shows even with the engine stopped / no device connected.
                        if let Some((input_address, assignment)) = selected_binding.as_ref() {
                            {
                                let cfg_default = ConfigSnapshot::default();
                                let cfg_ref = config.as_ref().unwrap_or(&cfg_default);
                                let (device_label, input_label) =
                                    source_label::split_label(input_address, cfg_ref);
                                rsx! {
                                    div { class: "if-sheets__binding-summary",
                                        p { class: "if-sheets__binding-line",
                                            if !device_label.is_empty() {
                                                span { class: "if-sheets__binding-device", "{device_label}" }
                                                span { class: "if-sheets__binding-sep", " \u{00b7} " }
                                            }
                                            span { class: "if-sheets__binding-input", "{input_label}" }
                                        }
                                        p {
                                            class: "if-sheets__binding-assignment",
                                            "{format_assignment(*assignment)}"
                                        }
                                    }
                                }
                            }
                        }
                        // Capture trigger: live-capture arm + re-arm. While armed, the
                        // listening visual replaces the button and the manual selects below
                        // are disabled.
                        if is_capture_armed {
                            {
                                let armed_input = selected_binding
                                    .as_ref()
                                    .map_or(InputAddress::Unbound, |(input, _)| input.clone());
                                let listening_class = rebind_composite_class(&armed_input, true);
                                rsx! {
                                    div { class: "{listening_class}",
                                        span {
                                            class: "if-rebind-composite__listening",
                                            role: "status",
                                            "aria-live": "polite",
                                            "Press an input..."
                                        }
                                        button {
                                            class: "if-rebind-composite__action",
                                            r#type: "button",
                                            onclick: move |_| on_cancel_capture.call(()),
                                            "Cancel"
                                        }
                                    }
                                    p { class: "if-sheets__composer-hint", "Hold Esc to cancel" }
                                }
                            }
                        } else {
                            {
                                let selected_anchor_for_capture = selected_anchor_for_capture.clone();
                                rsx! {
                                    button {
                                        class: "if-sheets__composer-trigger",
                                        r#type: "button",
                                        disabled: composer_button_disabled,
                                        onclick: move |_| {
                                            if let Some(anchor_id) = selected_anchor_for_capture.clone() {
                                                on_arm_capture.call(anchor_id);
                                            }
                                        },
                                        "Press input to assign"
                                    }
                                    if let Some(help) = composer_help.clone() {
                                        p { class: "if-sheets__composer-hint", "{help}" }
                                    }
                                }
                            }
                        }
                        // Device + Input selects: always visible, mirror the current binding
                        // (seeded above), and editing them re-assigns. Two-way synced with
                        // the live capture above through the anchor binding.
                        if connected_devices.is_empty() {
                            p {
                                class: "if-sheets__composer-hint",
                                match capture_availability {
                                    CaptureAvailabilityReason::EngineStopped => {
                                        "Start the engine to assign."
                                    }
                                    _ => "No devices seen. Connect a device to assign.",
                                }
                            }
                        } else {
                            {
                                let mut device_options =
                                    Vec::with_capacity(connected_devices.len() + 1);
                                device_options.push(SelectOption {
                                    value: String::new(),
                                    label: "(select)".to_owned(),
                                    disabled: false,
                                    class: None,
                                });
                                for (device_id, display_name) in connected_devices.iter().cloned() {
                                    device_options.push(SelectOption {
                                        value: device_id.0,
                                        label: display_name,
                                        disabled: false,
                                        class: None,
                                    });
                                }

                                // Real inputs for the selected device, grouped by type. Counts
                                // come from the live config snapshot; labels reuse the
                                // mapping-list helper so they read identically (X, Btn 1, Hat 0).
                                let selected_device_id = manual_device_id.read().clone();
                                let device_info = config.as_ref().and_then(|cfg| {
                                    cfg.devices
                                        .iter()
                                        .find(|d| d.info.id.0 == selected_device_id)
                                        .map(|d| &d.info)
                                });
                                let input_options = build_input_options(device_info);

                                rsx! {
                                    div { class: "if-sheets__manual-fields",
                                        label {
                                            "Device"
                                            Select {
                                                value: ReadSignal::from(manual_device_id),
                                                disabled: is_capture_armed,
                                                onchange: move |evt: FormEvent| {
                                                    manual_device_id.set(evt.value());
                                                    // The prior input may not exist on the new device.
                                                    manual_input_sel.set(String::new());
                                                },
                                                options: device_options,
                                            }
                                        }
                                        label {
                                            "Input"
                                            Select {
                                                value: ReadSignal::from(manual_input_sel),
                                                disabled: is_capture_armed,
                                                onchange: move |evt: FormEvent| {
                                                    let value = evt.value();
                                                    manual_input_sel.set(value.clone());
                                                    let device_id = manual_device_id.read().clone();
                                                    if !device_id.is_empty()
                                                        && let Some((kind, index)) =
                                                            parse_input_selection(&value)
                                                    {
                                                        let _ = sheets_for_assign
                                                            .write()
                                                            .assign_manual_selected_anchor(device_id, kind, index);
                                                    }
                                                },
                                                options: input_options,
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    button {
                        r#type: "button",
                        class: "if-sheets__inspector-destructive",
                        onclick: {
                            let mut sheets_for_anchor_delete = sheets;
                            move |_| {
                                let _ = sheets_for_anchor_delete.write().remove_selected_anchor();
                            }
                        },
                        "Delete"
                    }
                }
            } else if let Some(placement) = selected_placement {
                // Frame branch
                {
                    // Compute the placement's render index (for the "Frame N" fallback) and
                    // the default label that the Name input shows as placeholder. Both
                    // derive from the same state snapshot, so we fold the read into one
                    // block to release the borrow before the rsx body runs.
                    let frame_default_label = {
                        let state = sheets.read();
                        let idx = state
                            .selected_template()
                            .and_then(|template| {
                                template
                                    .placements
                                    .iter()
                                    .position(|p| p.placement_id == placement.placement_id)
                            })
                            .unwrap_or(0);
                        placement_default_label_for(&placement, &state.assets, idx)
                    };
                    // The Name input is a shared TextInput in default (controlled) mode bound
                    // to a LOCAL signal that lives on SheetsInspector. Keystrokes only mutate
                    // `frame_name_signal` so they do not trigger a SheetsInspector-wide
                    // re-render per keystroke; commit-on-blur / Enter reads the signal and
                    // calls `rename_selected_placement` once. Mirrors the typing-feel of the
                    // mapping editor's inline rename.
                    let frame_name_for_blur = frame_name_signal;
                    let frame_name_for_enter = frame_name_signal;
                    let mut sheets_for_frame_name_blur = sheets;
                    let mut sheets_for_frame_name_enter = sheets;
                    // The Frame branch needs an owned `TemplateId` to feed the
                    // commit_drag_end_* helpers. Each axis closure clones its
                    // own copy below; this base value is the source of those
                    // clones.
                    let frame_template_id = selected_template_id.clone();
                    let frame_placement_id = placement.placement_id.clone();
                    let current_x = placement.position.x;
                    let current_y = placement.position.y;
                    let current_w = placement.position.w;
                    let current_h = placement.position.h;
                    let displayed_x = frame_x_draft
                        .read()
                        .clone()
                        .unwrap_or_else(|| format_coord(f64::from(current_x)));
                    let displayed_y = frame_y_draft
                        .read()
                        .clone()
                        .unwrap_or_else(|| format_coord(f64::from(current_y)));
                    let displayed_w = frame_w_draft
                        .read()
                        .clone()
                        .unwrap_or_else(|| format_coord(f64::from(current_w)));
                    let displayed_h = frame_h_draft
                        .read()
                        .clone()
                        .unwrap_or_else(|| format_coord(f64::from(current_h)));

                    // One clone per oninput/onblur/onkeydown closure per axis.
                    let mut sheets_for_x_blur = sheets;
                    let mut sheets_for_x_keydown = sheets;
                    let mut sheets_for_y_blur = sheets;
                    let mut sheets_for_y_keydown = sheets;
                    let mut sheets_for_w_blur = sheets;
                    let mut sheets_for_w_keydown = sheets;
                    let mut sheets_for_h_blur = sheets;
                    let mut sheets_for_h_keydown = sheets;
                    let x_template_id_blur = frame_template_id.clone();
                    let x_template_id_keydown = frame_template_id.clone();
                    let y_template_id_blur = frame_template_id.clone();
                    let y_template_id_keydown = frame_template_id.clone();
                    let w_template_id_blur = frame_template_id.clone();
                    let w_template_id_keydown = frame_template_id.clone();
                    let h_template_id_blur = frame_template_id.clone();
                    let h_template_id_keydown = frame_template_id.clone();
                    let x_placement_id_blur = frame_placement_id.clone();
                    let x_placement_id_keydown = frame_placement_id.clone();
                    let y_placement_id_blur = frame_placement_id.clone();
                    let y_placement_id_keydown = frame_placement_id.clone();
                    let w_placement_id_blur = frame_placement_id.clone();
                    let w_placement_id_keydown = frame_placement_id.clone();
                    let h_placement_id_blur = frame_placement_id.clone();
                    let h_placement_id_keydown = frame_placement_id.clone();

                    rsx! {
                        section { "data-testid": "sheets-inspector-frame",
                            span { class: "if-sheets__eyebrow", "FRAME" }
                            label {
                                "Name"
                                TextInput {
                                    value: ReadSignal::from(frame_name_signal),
                                    placeholder: frame_default_label.clone(),
                                    class: Some(
                                        "if-sheets__inspector-frame-name-input".to_owned(),
                                    ),
                                    oninput: move |evt: FormEvent| {
                                        frame_name_signal.set(evt.value());
                                    },
                                    onblur: move |_| {
                                        let raw = frame_name_for_blur.read().clone();
                                        let trimmed = raw.trim();
                                        let next = if trimmed.is_empty() {
                                            None
                                        } else {
                                            Some(trimmed.to_owned())
                                        };
                                        sheets_for_frame_name_blur
                                            .write()
                                            .rename_selected_placement(next);
                                    },
                                    onkeydown: move |evt: KeyboardEvent| {
                                        if evt.key() == Key::Enter {
                                            let raw = frame_name_for_enter.read().clone();
                                            let trimmed = raw.trim();
                                            let next = if trimmed.is_empty() {
                                                None
                                            } else {
                                                Some(trimmed.to_owned())
                                            };
                                            sheets_for_frame_name_enter
                                                .write()
                                                .rename_selected_placement(next);
                                        }
                                    },
                                }
                            }
                            label {
                                "X"
                                input {
                                    r#type: "number",
                                    "data-axis": "x",
                                    step: "0.01",
                                    min: "0",
                                    max: "1",
                                    value: "{displayed_x}",
                                    oninput: move |evt: FormEvent| {
                                        frame_x_draft.set(Some(evt.value()));
                                    },
                                    onblur: move |_| {
                                        if let Some(draft) = frame_x_draft.write().take()
                                            && let Ok(parsed) = draft.trim().parse::<f32>()
                                            && let Some(tid) = x_template_id_blur.clone()
                                        {
                                            let clamped = parsed.clamp(0.0, 1.0);
                                            let _ = sheets_for_x_blur
                                                .write()
                                                .commit_drag_end_move(
                                                    tid,
                                                    x_placement_id_blur.clone(),
                                                    clamped,
                                                    current_y,
                                                );
                                        }
                                    },
                                    onkeydown: move |evt: KeyboardEvent| {
                                        if evt.key() == Key::Enter
                                            && let Some(draft) = frame_x_draft.write().take()
                                            && let Ok(parsed) = draft.trim().parse::<f32>()
                                            && let Some(tid) = x_template_id_keydown.clone()
                                        {
                                            let clamped = parsed.clamp(0.0, 1.0);
                                            let _ = sheets_for_x_keydown
                                                .write()
                                                .commit_drag_end_move(
                                                    tid,
                                                    x_placement_id_keydown.clone(),
                                                    clamped,
                                                    current_y,
                                                );
                                        }
                                    },
                                }
                            }
                            label {
                                "Y"
                                input {
                                    r#type: "number",
                                    "data-axis": "y",
                                    step: "0.01",
                                    min: "0",
                                    max: "1",
                                    value: "{displayed_y}",
                                    oninput: move |evt: FormEvent| {
                                        frame_y_draft.set(Some(evt.value()));
                                    },
                                    onblur: move |_| {
                                        if let Some(draft) = frame_y_draft.write().take()
                                            && let Ok(parsed) = draft.trim().parse::<f32>()
                                            && let Some(tid) = y_template_id_blur.clone()
                                        {
                                            let clamped = parsed.clamp(0.0, 1.0);
                                            let _ = sheets_for_y_blur
                                                .write()
                                                .commit_drag_end_move(
                                                    tid,
                                                    y_placement_id_blur.clone(),
                                                    current_x,
                                                    clamped,
                                                );
                                        }
                                    },
                                    onkeydown: move |evt: KeyboardEvent| {
                                        if evt.key() == Key::Enter
                                            && let Some(draft) = frame_y_draft.write().take()
                                            && let Ok(parsed) = draft.trim().parse::<f32>()
                                            && let Some(tid) = y_template_id_keydown.clone()
                                        {
                                            let clamped = parsed.clamp(0.0, 1.0);
                                            let _ = sheets_for_y_keydown
                                                .write()
                                                .commit_drag_end_move(
                                                    tid,
                                                    y_placement_id_keydown.clone(),
                                                    current_x,
                                                    clamped,
                                                );
                                        }
                                    },
                                }
                            }
                            label {
                                "Width"
                                input {
                                    r#type: "number",
                                    "data-axis": "w",
                                    step: "0.01",
                                    min: "0",
                                    max: "1",
                                    value: "{displayed_w}",
                                    oninput: move |evt: FormEvent| {
                                        frame_w_draft.set(Some(evt.value()));
                                    },
                                    onblur: move |_| {
                                        if let Some(draft) = frame_w_draft.write().take()
                                            && let Ok(parsed) = draft.trim().parse::<f32>()
                                            && let Some(tid) = w_template_id_blur.clone()
                                        {
                                            let clamped = parsed.clamp(0.0, 1.0);
                                            let _ = sheets_for_w_blur
                                                .write()
                                                .commit_drag_end_resize(
                                                    tid,
                                                    w_placement_id_blur.clone(),
                                                    TemplateRect {
                                                        x: current_x,
                                                        y: current_y,
                                                        w: clamped,
                                                        h: current_h,
                                                    },
                                                );
                                        }
                                    },
                                    onkeydown: move |evt: KeyboardEvent| {
                                        if evt.key() == Key::Enter
                                            && let Some(draft) = frame_w_draft.write().take()
                                            && let Ok(parsed) = draft.trim().parse::<f32>()
                                            && let Some(tid) = w_template_id_keydown.clone()
                                        {
                                            let clamped = parsed.clamp(0.0, 1.0);
                                            let _ = sheets_for_w_keydown
                                                .write()
                                                .commit_drag_end_resize(
                                                    tid,
                                                    w_placement_id_keydown.clone(),
                                                    TemplateRect {
                                                        x: current_x,
                                                        y: current_y,
                                                        w: clamped,
                                                        h: current_h,
                                                    },
                                                );
                                        }
                                    },
                                }
                            }
                            label {
                                "Height"
                                input {
                                    r#type: "number",
                                    "data-axis": "h",
                                    step: "0.01",
                                    min: "0",
                                    max: "1",
                                    value: "{displayed_h}",
                                    oninput: move |evt: FormEvent| {
                                        frame_h_draft.set(Some(evt.value()));
                                    },
                                    onblur: move |_| {
                                        if let Some(draft) = frame_h_draft.write().take()
                                            && let Ok(parsed) = draft.trim().parse::<f32>()
                                            && let Some(tid) = h_template_id_blur.clone()
                                        {
                                            let clamped = parsed.clamp(0.0, 1.0);
                                            let _ = sheets_for_h_blur
                                                .write()
                                                .commit_drag_end_resize(
                                                    tid,
                                                    h_placement_id_blur.clone(),
                                                    TemplateRect {
                                                        x: current_x,
                                                        y: current_y,
                                                        w: current_w,
                                                        h: clamped,
                                                    },
                                                );
                                        }
                                    },
                                    onkeydown: move |evt: KeyboardEvent| {
                                        if evt.key() == Key::Enter
                                            && let Some(draft) = frame_h_draft.write().take()
                                            && let Ok(parsed) = draft.trim().parse::<f32>()
                                            && let Some(tid) = h_template_id_keydown.clone()
                                        {
                                            let clamped = parsed.clamp(0.0, 1.0);
                                            let _ = sheets_for_h_keydown
                                                .write()
                                                .commit_drag_end_resize(
                                                    tid,
                                                    h_placement_id_keydown.clone(),
                                                    TemplateRect {
                                                        x: current_x,
                                                        y: current_y,
                                                        w: current_w,
                                                        h: clamped,
                                                    },
                                                );
                                        }
                                    },
                                }
                            }
                            button {
                                r#type: "button",
                                onclick: move |_| {
                                    if let (Some(tid), Some(pid)) = (
                                        shift_z_up_template_id.clone(),
                                        shift_z_up_placement_id.clone(),
                                    ) {
                                        let _ = sheets_for_shift_z_up.write().shift_z_up(tid, pid);
                                    }
                                },
                                "Move up"
                            }
                            button {
                                r#type: "button",
                                onclick: move |_| {
                                    if let (Some(tid), Some(pid)) = (
                                        bring_to_front_template_id.clone(),
                                        bring_to_front_placement_id.clone(),
                                    ) {
                                        let _ = sheets_for_bring_to_front.write().bring_to_front(tid, pid);
                                    }
                                },
                                "Bring to front"
                            }
                            button {
                                r#type: "button",
                                onclick: move |_| {
                                    if let (Some(tid), Some(pid)) = (
                                        send_to_back_template_id.clone(),
                                        send_to_back_placement_id.clone(),
                                    ) {
                                        let _ = sheets_for_send_to_back.write().send_to_back(tid, pid);
                                    }
                                },
                                "Send to back"
                            }
                            button {
                                r#type: "button",
                                onclick: move |_| {
                                    if let (Some(tid), Some(pid)) = (
                                        shift_z_down_template_id.clone(),
                                        shift_z_down_placement_id.clone(),
                                    ) {
                                        let _ = sheets_for_shift_z_down.write().shift_z_down(tid, pid);
                                    }
                                },
                                "Move down"
                            }
                            button {
                                r#type: "button",
                                class: "if-sheets__inspector-destructive",
                                onclick: move |_| {
                                    if let (Some(tid), Some(pid)) = (
                                        delete_placement_template_id.clone(),
                                        delete_placement_placement_id.clone(),
                                    ) {
                                        let _ = sheets_for_delete_placement.write().remove_placement(tid, &pid);
                                    }
                                },
                                "Delete"
                            }
                        }
                    }
                }
            } else if let Some(asset) = selected_asset_entry.as_ref() {
                // Asset branch (asset selected from the rail; no placement / anchor)
                {
                    let asset_filename = filename_of(asset);
                    let asset_label = asset_label_for(asset);
                    let pixel_w = asset.pixel_dimensions.width;
                    let pixel_h = asset.pixel_dimensions.height;
                    let media_type = asset.media_type.clone();
                    let asset_missing = selected_asset_health
                        .as_ref()
                        .is_none_or(|health| health.missing);
                    let thumb_src = selected_asset_health
                        .as_ref()
                        .filter(|health| !health.missing)
                        .and_then(|health| asset_data_url(health).ok());
                    let asset_id_for_delete = asset.asset_id.clone();
                    let asset_label_for_modal = asset_label.clone();
                    let usage_for_modal = asset_usage;
                    let asset_name_for_blur = asset_name_signal;
                    let asset_name_for_enter = asset_name_signal;
                    let mut sheets_for_asset_name_blur = sheets;
                    let mut sheets_for_asset_name_enter = sheets;
                    let mut sheets_for_asset_delete = sheets;
                    let mut delete_modal_for_button = delete_asset_modal_open;
                    let description_text = if usage_for_modal.placements == 0 {
                        format!(
                            "{asset_label_for_modal} is not used by any placement on any template. Deleting it will only remove it from the asset library."
                        )
                    } else {
                        let placements_label =
                            pluralize(usage_for_modal.placements, "placement", "placements");
                        let templates_label =
                            pluralize(usage_for_modal.templates, "template", "templates");
                        format!(
                            "{asset_label_for_modal} is used by {placements_label} across {templates_label}. Deleting it will remove every placement of this asset and any anchors attached to those placements."
                        )
                    };
                    rsx! {
                        section { "data-testid": "sheets-inspector-asset",
                            span { class: "if-sheets__eyebrow", "ASSET" }
                            if let Some(src) = thumb_src.as_ref() {
                                img {
                                    class: "if-sheets__inspector-asset-thumb",
                                    src: "{src}",
                                    alt: "{asset_filename}",
                                }
                            } else if asset_missing {
                                div {
                                    class: "if-sheets__inspector-asset-thumb if-sheets__inspector-asset-thumb--missing",
                                    "data-testid": "sheets-inspector-asset-thumb-missing",
                                }
                            }
                            label {
                                "Name"
                                TextInput {
                                    value: ReadSignal::from(asset_name_signal),
                                    placeholder: asset_filename.clone(),
                                    class: Some(
                                        "if-sheets__inspector-asset-name-input".to_owned(),
                                    ),
                                    oninput: move |evt: FormEvent| {
                                        asset_name_signal.set(evt.value());
                                    },
                                    onblur: move |_| {
                                        let raw = asset_name_for_blur.read().clone();
                                        let trimmed = raw.trim();
                                        let next = if trimmed.is_empty() {
                                            None
                                        } else {
                                            Some(trimmed.to_owned())
                                        };
                                        sheets_for_asset_name_blur.write().rename_selected_asset(next);
                                    },
                                    onkeydown: move |evt: KeyboardEvent| {
                                        if evt.key() == Key::Enter {
                                            let raw = asset_name_for_enter.read().clone();
                                            let trimmed = raw.trim();
                                            let next = if trimmed.is_empty() {
                                                None
                                            } else {
                                                Some(trimmed.to_owned())
                                            };
                                            sheets_for_asset_name_enter.write().rename_selected_asset(next);
                                        }
                                    },
                                }
                            }
                            dl { class: "if-sheets__inspector-asset-summary",
                                dt { "Filename" }
                                dd { "{asset_filename}" }
                                dt { "Dimensions" }
                                dd { "{pixel_w}x{pixel_h}" }
                                dt { "Type" }
                                dd { "{media_type}" }
                            }
                            Button {
                                variant: ButtonVariant::Danger,
                                class: Some("if-sheets__inspector-asset-delete".to_owned()),
                                onclick: move |_| {
                                    delete_modal_for_button.set(true);
                                },
                                "Delete asset"
                            }
                            DestructiveConfirmDialog {
                                open: delete_asset_modal_open,
                                title: Some("Delete asset?".to_owned()),
                                description: rsx! { "{description_text}" },
                                confirm_label: "Delete".to_owned(),
                                oncancel: move |()| {},
                                onconfirm: move |()| {
                                    let _ = sheets_for_asset_delete
                                        .write()
                                        .remove_asset(&asset_id_for_delete);
                                },
                            }
                        }
                    }
                }
            } else {
                // Template branch (no placement, no anchor, no asset selected)
                section { "data-testid": "sheets-inspector-template",
                    span { class: "if-sheets__eyebrow", "TEMPLATE" }
                    if let Some(template_name) = selected_template_name {
                        {
                            let displayed_name = template_name_draft
                                .clone()
                                .unwrap_or(template_name);
                            let template_name_signal: ReadSignal<String> =
                                ReadSignal::new(Signal::new(displayed_name));
                            let mut sheets_for_template_name_blur = sheets_for_template_name;
                            rsx! {
                                label {
                                    "Template display name"
                                    TextInput {
                                        value: template_name_signal,
                                        oninput: move |evt: FormEvent| {
                                            sheets_for_template_name.write().update_template_display_name_draft(evt.value());
                                        },
                                        onblur: move |_| {
                                            sheets_for_template_name_blur.write().commit_template_display_name_draft();
                                        },
                                    }
                                }
                            }
                        }
                    }
                    p { "Frame count: {frame_count}" }
                    p { "Anchor count: {anchor_count}" }
                }
            }
        }
    }
}

fn composer_help_for(
    capture: &CaptureStatus,
    availability: CaptureAvailabilityReason,
) -> Option<String> {
    match capture {
        CaptureStatus::Armed(_) | CaptureStatus::Assigned(_) => None,
        CaptureStatus::TimedOut(_) => Some("Capture timed out. Press to retry.".to_owned()),
        CaptureStatus::Canceled(_) => Some("Capture canceled. Press to retry.".to_owned()),
        CaptureStatus::Idle | CaptureStatus::Unavailable(_) => Some(match availability {
            CaptureAvailabilityReason::CaptureAvailable => {
                "Press a control on any connected device to assign.".to_owned()
            }
            CaptureAvailabilityReason::EngineStopped => {
                "Start the engine to capture, or pick a device below.".to_owned()
            }
            CaptureAvailabilityReason::EngineRunningNoDevices => {
                "Connect a device to capture or assign.".to_owned()
            }
        }),
    }
}

/// Sub-text under the binding summary describing how the input was assigned.
fn format_assignment(assignment: AnchorAssignment) -> &'static str {
    match assignment {
        AnchorAssignment::Captured => "Captured assignment",
        AnchorAssignment::Manual => "Manual assignment",
        AnchorAssignment::Unavailable => "Unavailable assignment",
    }
}

/// A non-selectable group header inside the manual "Input" dropdown. Disabled
/// so it cannot be chosen; the distinct `group:` value keeps it from matching
/// the empty placeholder selection.
fn manual_input_group_option(label: &str) -> SelectOption {
    SelectOption {
        value: format!("group:{label}"),
        label: label.to_owned(),
        disabled: true,
        class: Some("if-sheets__manual-input-group".to_owned()),
    }
}

/// Encode an `InputId` as a manual "Input" dropdown value (`axis:0`,
/// `button:3`, `hat:0`). Inverse of [`parse_input_selection`]; also used to
/// seed the Input select from the current binding.
fn encode_input_selection(input: &InputId) -> String {
    match input {
        InputId::Axis { index } => format!("axis:{index}"),
        InputId::Button { index } => format!("button:{index}"),
        InputId::Hat { index } => format!("hat:{index}"),
    }
}

/// One real input in the manual "Input" dropdown. The value encodes the kind
/// and index; the label reuses the mapping-list helper.
fn manual_input_option(input: &InputId) -> SelectOption {
    SelectOption {
        value: encode_input_selection(input),
        label: source_label::input_label(input),
        disabled: false,
        class: None,
    }
}

/// Build the manual "Input" dropdown options for the selected device: a
/// placeholder, then the device's real axes/buttons/hats grouped by type with
/// non-selectable headers. `None` (no device selected, or no capability data)
/// yields just the placeholder.
fn build_input_options(info: Option<&DeviceInfo>) -> Vec<SelectOption> {
    let mut options = vec![SelectOption {
        value: String::new(),
        label: "Select input\u{2026}".to_owned(),
        disabled: false,
        class: None,
    }];
    let Some(info) = info else {
        return options;
    };
    if info.axes > 0 {
        options.push(manual_input_group_option("Axes"));
        for i in 0..info.axes {
            options.push(manual_input_option(&InputId::Axis { index: i }));
        }
    }
    if info.buttons > 0 {
        options.push(manual_input_group_option("Buttons"));
        for i in 0..info.buttons {
            options.push(manual_input_option(&InputId::Button { index: i }));
        }
    }
    if info.hats > 0 {
        options.push(manual_input_group_option("Hats"));
        for i in 0..info.hats {
            options.push(manual_input_option(&InputId::Hat { index: i }));
        }
    }
    options
}

/// Decode a manual "Input" dropdown value (`axis:N` / `button:N` / `hat:N`)
/// into the `(kind, index)` pair `assign_manual_selected_anchor` expects.
/// Returns `None` for the placeholder, group headers, or malformed values.
fn parse_input_selection(value: &str) -> Option<(ManualInputKind, u8)> {
    let (tag, index) = value.split_once(':')?;
    let index: u8 = index.parse().ok()?;
    let kind = match tag {
        "axis" => ManualInputKind::Axis,
        "button" => ManualInputKind::Button,
        "hat" => ManualInputKind::Hat,
        _ => return None,
    };
    Some((kind, index))
}

/// Format a normalized 0..1 position for display in a number input. Caps the
/// raw `f32`/`f64` precision that would otherwise leak through `to_string`
/// (e.g. `0.1905256062746048`); the stored value is unchanged, so dragging and
/// editing keep full precision.
fn format_coord(value: f64) -> String {
    format!("{value:.4}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_coord_caps_decimals_at_four() {
        assert_eq!(format_coord(0.190_525_606_274_604_8), "0.1905");
        assert_eq!(format_coord(0.5), "0.5000");
        assert_eq!(format_coord(0.0), "0.0000");
        assert_eq!(format_coord(1.0), "1.0000");
    }

    #[test]
    fn parse_input_selection_decodes_kind_and_index() {
        assert_eq!(
            parse_input_selection("axis:2"),
            Some((ManualInputKind::Axis, 2))
        );
        assert_eq!(
            parse_input_selection("button:0"),
            Some((ManualInputKind::Button, 0))
        );
        assert_eq!(
            parse_input_selection("hat:1"),
            Some((ManualInputKind::Hat, 1))
        );
    }

    #[test]
    fn parse_input_selection_rejects_placeholder_groups_and_garbage() {
        assert_eq!(parse_input_selection(""), None);
        assert_eq!(parse_input_selection("group:Axes"), None);
        assert_eq!(parse_input_selection("axis:"), None);
        assert_eq!(parse_input_selection("axis:300"), None); // overflows u8
        assert_eq!(parse_input_selection("wat:1"), None);
    }

    #[test]
    fn build_input_options_lists_real_inputs_grouped_by_type() {
        let info = DeviceInfo {
            id: DeviceId("dev".to_owned()),
            name: "Stick".to_owned(),
            axes: 3,
            buttons: 2,
            hats: 1,
            instance_path: None,
            axis_polarities: Vec::new(),
        };
        let options = build_input_options(Some(&info));
        let labels: Vec<&str> = options.iter().map(|o| o.label.as_str()).collect();
        assert_eq!(
            labels,
            vec![
                "Select input\u{2026}",
                "Axes",
                "X",
                "Y",
                "Z",
                "Buttons",
                "Btn 1",
                "Btn 2",
                "Hats",
                "Hat 0",
            ]
        );
        let x = options.iter().find(|o| o.label == "X").unwrap();
        assert_eq!(x.value, "axis:0");
        assert_eq!(
            parse_input_selection(&x.value),
            Some((ManualInputKind::Axis, 0))
        );
        let axes_header = options.iter().find(|o| o.label == "Axes").unwrap();
        assert!(axes_header.disabled);
        assert_eq!(parse_input_selection(&axes_header.value), None);
    }

    #[test]
    fn build_input_options_without_device_is_placeholder_only() {
        let options = build_input_options(None);
        assert_eq!(options.len(), 1);
        assert!(options[0].value.is_empty());
    }
}
