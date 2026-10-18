//! Render prepared device fields and forward observations to the app.
use super::{DevicePanelRow, UsageBlock, build_device_report};
use crate::{
    components::{Button, ButtonSize, ButtonVariant},
    controllers::inspector::{Context, Intent, View},
};
use dioxus::prelude::*;
#[component]
pub(super) fn DeviceInspector(row: DevicePanelRow, view: View, context: Context) -> Element {
    let draft_value = use_memo(move || context.view.read().draft.clone());
    let report = build_device_report(&row);
    let oninput = use_callback(move |event: FormEvent| {
        let _ = context.send(Intent::Input(event.value()));
    });
    let text = (crate::components::behavior().text)(crate::controls::TextProps {
        value: draft_value.into(),
        disabled: false,
        oninput: Some(oninput),
    });
    let save_click = move |_| {
        let _ = context.send(Intent::Save);
    };
    let remove_click = move |_| {
        let _ = context.send(Intent::Remove);
    };
    let copy_click = move |_| {
        let _ = context.send(Intent::Copy(report.clone()));
    };
    let onkeydown = move |event: KeyboardEvent| {
        if context
            .bindings
            .key
            .call((context.owner, context.workflow, event.key().to_string()))
        {
            event.prevent_default();
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
                        initial_value: "{draft_value}",
                        "data-if-text-owner": "{text.owner}",
                        oninput: text.input,
                        onkeydown,
                    }
                    Button {
                        size: ButtonSize::Sm,
                        class: "if-device-inspector__save",
                        disabled: !view.can_save,
                        onclick: save_click,
                        "Save"
                    }
                }
            }
            if let Some(error) = &view.error {
                div { role: "alert", class: "if-device-inspector__error", "{error}" }
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
            div { class: "if-device-inspector__removal",
                Button {
                    size: ButtonSize::Sm, variant: ButtonVariant::Danger,
                    class: "if-device-inspector__remove",
                    disabled: view.removal_reason.is_some(),
                    onclick: remove_click,
                    "Remove from profile"
                }
                p { class: "if-device-inspector__removal-hint",
                    if let Some(reason) = &view.removal_reason { "{reason}" }
                    else { "Hidden in this profile until reconnected." }
                }
            }
        }
    }
}
