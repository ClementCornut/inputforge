#![expect(
    unused_qualifications,
    reason = "rsx! macro expansion reports event handler field names as unnecessary qualifications"
)]

use dioxus::prelude::*;

use inputforge_core::sheet::{AnchorAssignment, AnchorId, AssetPlacement, TemplateAnchor};
use inputforge_core::types::{DeviceId, InputAddress, InputId};

use crate::frame::sheets::state::{
    AutosaveStatus, CaptureAvailabilityReason, CaptureStatus, ManualInputKind, SheetsState,
};

#[component]
pub(crate) fn SheetsInspector(
    sheets: Signal<SheetsState>,
    #[props(default)] capture_availability: CaptureAvailabilityReason,
    #[props(default)] connected_devices: Vec<(DeviceId, String)>,
    on_arm_capture: EventHandler<AnchorId>,
    on_cancel_capture: EventHandler<()>,
    on_retry_save: EventHandler<()>,
) -> Element {
    let mut manual_device_id = use_signal(String::new);
    let mut manual_input_kind = use_signal(ManualInputKind::default);
    let mut manual_input_index = use_signal(|| "0".to_owned());

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
        selected_anchor,
        selected_placement,
        selected_binding,
        capture,
        last_error,
        frame_count,
        anchor_count,
        first_placement_id,
    ) = {
        let state = sheets.read();
        let selected_anchor_id = state.selected_anchor_id.clone();
        let selected_placement_id = state.selected_placement_id.clone();
        let selected_template_id = state.selected_template_id.clone();
        let selected_template = state.selected_template();
        let selected_template_name = selected_template.map(|t| t.display_name.clone());
        let frame_count = selected_template.map_or(0, |t| t.placements.len());
        let anchor_count = selected_template.map_or(0, |t| t.anchors.len());
        let first_placement_id =
            selected_template.and_then(|t| t.placements.first().map(|p| p.placement_id.clone()));

        let selected_anchor: Option<TemplateAnchor> = selected_template.and_then(|template| {
            let anchor_id = selected_anchor_id.as_ref()?;
            template
                .anchors
                .iter()
                .find(|anchor| &anchor.anchor_id == anchor_id)
                .cloned()
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
            selected_anchor,
            selected_placement,
            selected_binding,
            state.capture.clone(),
            state.last_error.clone(),
            frame_count,
            anchor_count,
            first_placement_id,
        )
    };

    let is_capture_armed = matches!(capture, CaptureStatus::Armed(_));
    let composer_button_disabled = selected_anchor_id.is_none() || is_capture_armed;
    let composer_help = composer_help_for(&capture, capture_availability);
    let selected_anchor_for_capture = selected_anchor_id.clone();
    let selected_anchor_for_reassign = selected_anchor_id.clone();
    let mut sheets_for_template_name = sheets;
    let mut sheets_for_anchor_label = sheets;
    let mut sheets_for_attach = sheets;
    let mut sheets_for_bring_to_front = sheets;
    let mut sheets_for_send_to_back = sheets;
    let mut sheets_for_assign = sheets;
    let assign_disabled =
        manual_device_id.read().is_empty() || is_capture_armed || selected_anchor_id.is_none();

    // Clones for onclick closures that fire multiple times.
    let bring_to_front_template_id = selected_template_id.clone();
    let bring_to_front_placement_id = selected_placement_id.clone();
    let send_to_back_template_id = selected_template_id.clone();
    let send_to_back_placement_id = selected_placement_id.clone();
    let attach_first_placement_id = first_placement_id.clone();
    let anchor_for_attach = selected_anchor.clone();

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
                            let mut sheets_for_anchor_label_blur = sheets_for_anchor_label;
                            rsx! {
                                label {
                                    "Label"
                                    input {
                                        r#type: "text",
                                        value: "{displayed_label}",
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
                            let is_attached = anchor_for_attach
                                .as_ref()
                                .is_some_and(|a| a.attached_to.is_some());
                            let attach_default = attach_first_placement_id.clone();
                            rsx! {
                                button {
                                    r#type: "button",
                                    onclick: move |_| {
                                        let target = if is_attached {
                                            None
                                        } else {
                                            attach_default.clone()
                                        };
                                        let _ = sheets_for_attach.write().set_anchor_attached_to(target);
                                    },
                                    if is_attached { "Detach" } else { "Attach to frame" }
                                }
                            }
                        }
                        p { "Position: ({anchor.position.x:.3}, {anchor.position.y:.3})" }
                    }
                    section {
                        span { class: "if-sheets__eyebrow", "ASSIGNMENT" }
                        section { class: "if-sheets__composer",
                            match capture.clone() {
                                CaptureStatus::Armed(_) => rsx! {
                                    div { class: "if-rebind-composite if-rebind-composite--listening",
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
                                },
                                CaptureStatus::Assigned(_) => {
                                    let selected_anchor_for_reassign = selected_anchor_for_reassign.clone();
                                    rsx! {
                                        if let Some((input, assignment)) = selected_binding.clone() {
                                            p { "Input address" }
                                            code { "{format_input_address(&input)}" }
                                            p { "{format_assignment(assignment)}" }
                                        }
                                        button {
                                            class: "if-sheets__composer-trigger",
                                            r#type: "button",
                                            disabled: composer_button_disabled,
                                            onclick: move |_| {
                                                if let Some(anchor_id) = selected_anchor_for_reassign.clone() {
                                                    on_arm_capture.call(anchor_id);
                                                }
                                            },
                                            "Re-assign"
                                        }
                                    }
                                }
                                _ => {
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
                        }
                        details { class: "if-sheets__manual-disclosure",
                            // No `open` attribute: progressive disclosure stays
                            // collapsed until the user expands it.
                            summary { "Choose device manually" }
                            if connected_devices.is_empty() {
                                p {
                                    match capture_availability {
                                        CaptureAvailabilityReason::EngineStopped => {
                                            "Start the engine to assign."
                                        }
                                        _ => "No devices seen. Connect a device to assign manually.",
                                    }
                                }
                            } else {
                                label {
                                    "Device"
                                    select {
                                        value: "{manual_device_id}",
                                        onchange: move |evt: FormEvent| manual_device_id.set(evt.value()),
                                        option { value: "", "(select)" }
                                        for (device_id , display_name) in connected_devices.iter().cloned() {
                                            option {
                                                key: "{device_id.0}",
                                                value: "{device_id.0}",
                                                "{display_name}"
                                            }
                                        }
                                    }
                                }
                                fieldset { class: "if-sheets__manual-kind-segment",
                                    legend { class: "if-sheets__inspector-eyebrow", "Type" }
                                    for (kind_label , kind) in [
                                        ("Button", ManualInputKind::Button),
                                        ("Axis", ManualInputKind::Axis),
                                        ("Hat", ManualInputKind::Hat),
                                    ]
                                    {
                                        button {
                                            r#type: "button",
                                            "data-active": *manual_input_kind.read() == kind,
                                            onclick: move |_| manual_input_kind.set(kind),
                                            "{kind_label}"
                                        }
                                    }
                                }
                                label {
                                    "Index"
                                    input {
                                        r#type: "number",
                                        min: "0",
                                        max: "255",
                                        class: "if-sheets__manual-index",
                                        value: "{manual_input_index}",
                                        oninput: move |evt: FormEvent| manual_input_index.set(evt.value()),
                                    }
                                }
                                button {
                                    r#type: "button",
                                    disabled: assign_disabled,
                                    onclick: move |_| {
                                        let device_id = manual_device_id.read().clone();
                                        let kind = *manual_input_kind.read();
                                        let index = parse_manual_input_index(&manual_input_index.read());
                                        let _ = sheets_for_assign
                                            .write()
                                            .assign_manual_selected_anchor(device_id, kind, index);
                                    },
                                    "Assign"
                                }
                            }
                        }
                    }
                }
            } else if let Some(placement) = selected_placement {
                // Frame branch
                section { "data-testid": "sheets-inspector-frame",
                    span { class: "if-sheets__eyebrow", "FRAME" }
                    label {
                        "X"
                        input {
                            r#type: "number",
                            "data-axis": "x",
                            value: "{placement.position.x}",
                            step: "0.01",
                        }
                    }
                    label {
                        "Y"
                        input {
                            r#type: "number",
                            "data-axis": "y",
                            value: "{placement.position.y}",
                            step: "0.01",
                        }
                    }
                    label {
                        "Width"
                        input {
                            r#type: "number",
                            "data-axis": "w",
                            value: "{placement.position.w}",
                            step: "0.01",
                        }
                    }
                    label {
                        "Height"
                        input {
                            r#type: "number",
                            "data-axis": "h",
                            value: "{placement.position.h}",
                            step: "0.01",
                        }
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
                }
            } else {
                // Template branch (no placement, no anchor selected)
                section { "data-testid": "sheets-inspector-template",
                    span { class: "if-sheets__eyebrow", "TEMPLATE" }
                    if let Some(template_name) = selected_template_name {
                        {
                            let displayed_name = template_name_draft
                                .clone()
                                .unwrap_or(template_name);
                            let mut sheets_for_template_name_blur = sheets_for_template_name;
                            rsx! {
                                label {
                                    "Template display name"
                                    input {
                                        r#type: "text",
                                        value: "{displayed_name}",
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

fn parse_manual_input_index(value: &str) -> u8 {
    let trimmed = value.trim();
    if trimmed.starts_with('-') {
        return 0;
    }

    let digits = trimmed.strip_prefix('+').unwrap_or(trimmed);
    let mut index = 0_u8;
    for character in digits.chars() {
        let Some(digit) = character
            .to_digit(10)
            .and_then(|digit| u8::try_from(digit).ok())
        else {
            return 0;
        };
        let Some(next_index) = index
            .checked_mul(10)
            .and_then(|next_index| next_index.checked_add(digit))
        else {
            return u8::MAX;
        };
        index = next_index;
    }

    index
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
                "Start the engine to capture, or choose device manually.".to_owned()
            }
            CaptureAvailabilityReason::EngineRunningNoDevices => {
                "Connect a device to capture, or use the manual disclosure when a device appears."
                    .to_owned()
            }
        }),
    }
}

fn format_assignment(assignment: AnchorAssignment) -> &'static str {
    match assignment {
        AnchorAssignment::Captured => "Captured assignment",
        AnchorAssignment::Manual => "Manual assignment",
        AnchorAssignment::Unavailable => "Unavailable assignment",
    }
}

fn format_input_address(input: &InputAddress) -> String {
    match input {
        InputAddress::Unbound => "Unbound".to_owned(),
        InputAddress::Bound { device, input } => format!(
            "{} / {}",
            device.0,
            match input {
                InputId::Axis { index } => format!("Axis {index}"),
                InputId::Button { index } => format!("Button {index}"),
                InputId::Hat { index } => format!("Hat {index}"),
            }
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_input_index_parser_clamps_out_of_range_values() {
        assert_eq!(parse_manual_input_index("-1"), 0);
        assert_eq!(parse_manual_input_index("256"), 255);
        assert_eq!(parse_manual_input_index("999999"), 255);
        assert_eq!(parse_manual_input_index("999999999999999999999999"), 255);
    }
}
