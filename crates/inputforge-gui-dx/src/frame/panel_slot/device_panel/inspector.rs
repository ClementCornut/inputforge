//! Selected controller details and editable display name.
use super::{
    AppContext, DevicePanelRow, UsageBlock, alias_draft_after_escape, build_device_report,
    build_set_device_alias_command, copy_device_report_to_clipboard,
};
use crate::components::{Button, ButtonSize, ButtonVariant};
use dioxus::prelude::*;

#[component]
pub(super) fn DeviceInspector(
    row: DevicePanelRow,
    mut draft_alias: Signal<String>,
    mut save_error: Signal<Option<String>>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let persisted_alias = row.alias.clone();
    let draft_value = draft_alias.read().clone();
    let dirty = draft_value.trim() != persisted_alias;
    let report = build_device_report(&row);
    let error = save_error.read().clone();
    let save_device = row.device_id.clone();
    let mut draft_alias_for_input = draft_alias;
    let draft_alias_for_save = draft_alias;
    let mut draft_alias_for_keydown = draft_alias;
    let mut save_error_for_save = save_error;
    let mut save_error_for_keydown = save_error;
    let mut save_error_for_copy = save_error;
    let commands = ctx.commands.clone();
    let commands_for_keydown = commands.clone();
    let save_device_for_keydown = save_device.clone();
    let persisted_alias_for_keydown = persisted_alias.clone();
    let report_for_copy = report.clone();
    let oninput = move |event: FormEvent| draft_alias_for_input.set(event.value());
    let save_click = move |_| {
        let command =
            build_set_device_alias_command(save_device.clone(), &draft_alias_for_save.read());
        if let Err(error) = commands.send(command) {
            save_error_for_save.set(Some(error.to_string()));
        } else {
            save_error_for_save.set(None);
        }
    };
    let onkeydown = move |event: KeyboardEvent| match event.key() {
        Key::Enter => {
            event.prevent_default();
            let draft_value = draft_alias_for_keydown.read().clone();
            if draft_value.trim() == persisted_alias_for_keydown {
                return;
            }
            let command =
                build_set_device_alias_command(save_device_for_keydown.clone(), &draft_value);
            if let Err(error) = commands_for_keydown.send(command) {
                save_error_for_keydown.set(Some(error.to_string()));
            } else {
                save_error_for_keydown.set(None);
            }
        }
        Key::Escape => {
            event.prevent_default();
            draft_alias_for_keydown.set(alias_draft_after_escape(&persisted_alias_for_keydown));
            save_error_for_keydown.set(None);
        }
        _ => {}
    };
    let copy_click = move |_| {
        if let Err(error) = copy_device_report_to_clipboard(&report_for_copy) {
            save_error_for_copy.set(Some(format!("Copy failed: {error}")));
        } else {
            save_error_for_copy.set(None);
        }
    };

    rsx! {
        section { class: "if-device-panel__inspector", "aria-label": "Selected device details",
            div { class: "if-device-inspector__field",
                span { "Display name" }
                div { class: "if-device-inspector__edit-row",
                    input {
                        "aria-label": "Display name",
                        class: "if-device-inspector__input if-text-input if-text-input--inset",
                        value: "{draft_value}",
                        oninput,
                        onkeydown,
                    }
                    Button {
                        size: ButtonSize::Sm,
                        class: "if-device-inspector__save",
                        disabled: !dirty,
                        onclick: save_click,
                        "Save"
                    }
                }
            }
            if let Some(error) = error {
                div { class: "if-device-inspector__error", "{error}" }
            }
            div { class: "if-device-inspector__meta",
                span { class: "if-device-inspector__meta-label", "Hardware" }
                span { class: "if-device-inspector__hardware", title: "{row.hardware_name}", "{row.hardware_name}" }
            }
            UsageBlock { row: row.clone() }
            Button {
                size: ButtonSize::Sm, variant: ButtonVariant::Secondary,
                class: "if-device-inspector__copy",
                onclick: copy_click,
                "Copy report"
            }
        }
    }
}
