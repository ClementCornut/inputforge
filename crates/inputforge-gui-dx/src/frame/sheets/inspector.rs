#![expect(
    unused_qualifications,
    reason = "rsx! macro expansion reports event handler field names as unnecessary qualifications"
)]

use dioxus::prelude::*;

use inputforge_core::sheet::{AnchorAssignment, AnchorId, AssetPlacement, TemplateAnchor};
use inputforge_core::types::{InputAddress, InputId};

use crate::frame::sheets::state::{AutosaveStatus, CaptureStatus, ManualInputKind, SheetsState};

#[component]
pub(crate) fn SheetsInspector(
    sheets: Signal<SheetsState>,
    on_arm_capture: EventHandler<AnchorId>,
    on_cancel_capture: EventHandler<()>,
    on_retry_save: EventHandler<()>,
) -> Element {
    let mut manual_device_id = use_signal(String::new);
    let mut manual_input_kind = use_signal(ManualInputKind::default);
    let mut manual_input_index = use_signal(|| "0".to_owned());

    // Pull every read-only snapshot we need in one borrow so we never hold a `read()` guard
    // across an `rsx!` expression that also has to `write()` to the same signal.
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

    let capture_unavailable_reason = match &capture {
        CaptureStatus::Unavailable(reason) => Some(reason.clone()),
        _ => None,
    };
    let is_capture_armed = matches!(capture, CaptureStatus::Armed(_));
    let capture_disabled =
        capture_unavailable_reason.is_some() || selected_anchor_id.is_none() || is_capture_armed;
    let capture_help = capture_unavailable_reason.clone().or_else(|| {
        selected_anchor_id
            .is_none()
            .then(|| "Select an anchor to capture input.".to_owned())
    });
    let selected_anchor_for_capture = selected_anchor_id.clone();
    let mut sheets_for_template_name = sheets;
    let mut sheets_for_anchor_label = sheets;
    let mut sheets_for_attach = sheets;
    let mut sheets_for_bring_to_front = sheets;
    let mut sheets_for_send_to_back = sheets;
    let mut sheets_for_assign = sheets;
    let manual_address = format_manual_address(
        &manual_device_id.read(),
        *manual_input_kind.read(),
        parse_manual_input_index(&manual_input_index.read()),
    );
    let assign_disabled = selected_anchor_id.is_none() || is_capture_armed;

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
                        label {
                            "Label"
                            input {
                                r#type: "text",
                                value: "{anchor.label}",
                                oninput: move |evt: FormEvent| {
                                    sheets_for_anchor_label.write().update_selected_anchor_label(evt.value());
                                },
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
                        section {
                            h3 { "Capture input" }
                            if let Some((input, assignment)) = selected_binding {
                                p { "Input address" }
                                code { "{format_input_address(&input)}" }
                                p { "{format_assignment(assignment)}" }
                            }
                            button {
                                r#type: "button",
                                disabled: capture_disabled,
                                onclick: move |_| {
                                    if let Some(anchor_id) = selected_anchor_for_capture.clone() {
                                        on_arm_capture.call(anchor_id);
                                    }
                                },
                                "Capture input"
                            }
                            if is_capture_armed {
                                button {
                                    r#type: "button",
                                    onclick: move |_| on_cancel_capture.call(()),
                                    "Cancel capture"
                                }
                            }
                            if let Some(help) = capture_help {
                                p { "{help}" }
                            }
                        }
                        section {
                            h3 { "Manual assignment" }
                            label {
                                "Device id"
                                input {
                                    r#type: "text",
                                    disabled: is_capture_armed,
                                    value: "{manual_device_id}",
                                    oninput: move |evt: FormEvent| manual_device_id.set(evt.value()),
                                }
                            }
                            label {
                                "Input kind"
                                select {
                                    disabled: is_capture_armed,
                                    value: "{manual_input_kind_value(*manual_input_kind.read())}",
                                    onchange: move |evt: FormEvent| {
                                        manual_input_kind.set(manual_input_kind_from_value(&evt.value()));
                                    },
                                    option { value: "button", "Button" }
                                    option { value: "axis", "Axis" }
                                    option { value: "hat", "Hat" }
                                }
                            }
                            label {
                                "Input index"
                                input {
                                    r#type: "number",
                                    min: "0",
                                    max: "255",
                                    disabled: is_capture_armed,
                                    value: "{manual_input_index}",
                                    oninput: move |evt: FormEvent| manual_input_index.set(evt.value()),
                                }
                            }
                            p { "Input address" }
                            code { "{manual_address}" }
                            button {
                                r#type: "button",
                                disabled: assign_disabled,
                                onclick: move |_| {
                                    let device_id = manual_device_id.read().clone();
                                    let input_kind = *manual_input_kind.read();
                                    let input_index = parse_manual_input_index(&manual_input_index.read());
                                    let _ = sheets_for_assign
                                        .write()
                                        .assign_manual_selected_anchor(device_id, input_kind, input_index);
                                },
                                "Assign input"
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
                        label {
                            "Template display name"
                            input {
                                r#type: "text",
                                value: "{template_name}",
                                oninput: move |evt: FormEvent| {
                                    sheets_for_template_name.write().rename_selected_template(evt.value());
                                },
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

fn manual_input_kind_value(kind: ManualInputKind) -> &'static str {
    match kind {
        ManualInputKind::Button => "button",
        ManualInputKind::Axis => "axis",
        ManualInputKind::Hat => "hat",
    }
}

fn manual_input_kind_from_value(value: &str) -> ManualInputKind {
    match value {
        "axis" => ManualInputKind::Axis,
        "hat" => ManualInputKind::Hat,
        _ => ManualInputKind::Button,
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

fn format_manual_address(device_id: &str, input_kind: ManualInputKind, input_index: u8) -> String {
    let device_id = device_id.trim();
    if device_id.is_empty() {
        return "Device id required".to_owned();
    }

    format!(
        "{} / {} {}",
        device_id,
        match input_kind {
            ManualInputKind::Button => "Button",
            ManualInputKind::Axis => "Axis",
            ManualInputKind::Hat => "Hat",
        },
        input_index
    )
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
