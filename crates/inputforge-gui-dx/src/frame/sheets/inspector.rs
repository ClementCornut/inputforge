#![expect(
    unused_qualifications,
    reason = "rsx! macro expansion reports event handler field names as unnecessary qualifications"
)]

use dioxus::prelude::*;

use inputforge_core::sheet::{AnchorAssignment, AnchorId};
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

    let (
        save_status,
        selected_template_name,
        selected_anchor_id,
        selected_binding,
        capture,
        last_error,
    ) = {
        let state = sheets.read();
        let selected_anchor_id = state.selected_anchor_id.clone();
        let selected_binding = state
            .selected_template()
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
            state
                .selected_template()
                .map(|template| template.display_name.clone()),
            selected_anchor_id,
            selected_binding,
            state.capture.clone(),
            state.last_error.clone(),
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
    let mut sheets_for_assign = sheets;
    let manual_address = format_manual_address(
        &manual_device_id.read(),
        *manual_input_kind.read(),
        parse_manual_input_index(&manual_input_index.read()),
    );
    let assign_disabled = selected_anchor_id.is_none() || is_capture_armed;

    rsx! {
        aside { "data-testid": "sheets-inspector",
            h2 { "Inspector" }
            p { "{save_status}" }
            if save_status == "Save failed" {
                button {
                    r#type: "button",
                    onclick: move |_| on_retry_save.call(()),
                    "Retry save"
                }
            }
            if let Some(error) = last_error {
                p { role: "alert", "{error}" }
            }

            if let Some(template_name) = selected_template_name {
                section {
                    h3 { "Template" }
                    label {
                        "Template display name"
                        input {
                            r#type: "text",
                            readonly: true,
                            value: "{template_name}",
                        }
                    }
                }
            }

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
