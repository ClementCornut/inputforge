//! Batch wizard metadata controls and supplied callback wiring.
use crate::{
    batch_mapping::{Intent, View},
    components::{Checkbox, Field, Select, SelectOption},
};
use dioxus::prelude::*;
#[component]
pub(super) fn Metadata(model: View, on_intent: Callback<Intent>) -> Element {
    let source_text = model
        .source
        .as_ref()
        .map(|s| s.0.clone())
        .unwrap_or_default();
    let target_text = model.target.map(|id| id.to_string()).unwrap_or_default();
    let mut source = use_signal(|| source_text.clone());
    let mut target = use_signal(|| target_text.clone());
    let mut mode = use_signal(|| model.mode.clone());
    let mut all = use_signal(|| model.all_modes);
    if *source.peek() != source_text {
        source.set(source_text);
    }
    if *target.peek() != target_text {
        target.set(target_text);
    }
    if *mode.peek() != model.mode {
        mode.set(model.mode.clone());
    }
    if *all.peek() != model.all_modes {
        all.set(model.all_modes);
    }
    let source_options = model
        .sources
        .iter()
        .map(|(id, label)| SelectOption {
            value: id.0.clone(),
            label: label.clone(),
            disabled: false,
            class: None,
        })
        .collect();
    let target_options = model
        .targets
        .iter()
        .map(|t| SelectOption {
            value: t.device_id.to_string(),
            label: super::labels::format_virtual_device_option(t),
            disabled: false,
            class: None,
        })
        .collect();
    let mode_options = model
        .modes
        .iter()
        .map(|m| SelectOption {
            value: m.clone(),
            label: m.clone(),
            disabled: false,
            class: None,
        })
        .collect();
    rsx! {
        div { class: "if-bulk-map__metadata",
            Field { label: "Source".to_owned(), for_id: Some("bulk-map-source".to_owned()),
                Select { id: Some("bulk-map-source".to_owned()), value: ReadSignal::from(source), options: source_options,
                    onchange: move |evt: FormEvent| on_intent.call(Intent::Source(evt.value())) }
            }
            Field { label: "Target".to_owned(), for_id: Some("bulk-map-target".to_owned()),
                Select { id: Some("bulk-map-target".to_owned()), value: ReadSignal::from(target), options: target_options,
                    onchange: move |evt: FormEvent| on_intent.call(Intent::Target(evt.value())) }
            }
            Field { label: "Mode".to_owned(), for_id: Some("bulk-map-mode".to_owned()),
                Select { id: Some("bulk-map-mode".to_owned()), value: ReadSignal::from(mode), options: mode_options, disabled: model.all_modes,
                    onchange: move |evt: FormEvent| on_intent.call(Intent::Mode(evt.value())) }
            }
            Field { class: Some("if-bulk-map__apply-field".to_owned()), label: "Apply to".to_owned(), for_id: Some("bulk-map-all-modes".to_owned()),
                div { class: "if-bulk-map__apply-control",
                    Checkbox { id: Some("bulk-map-all-modes".to_owned()), checked: ReadSignal::from(all),
                        onchange: move |_| on_intent.call(Intent::AllModes(!model.all_modes)) }
                    span { class: "if-bulk-map__apply-label", "All modes ({model.modes.len()})" }
                }
            }
        }
    }
}
