//! Preview-only values exercise editing and capture presentation without hardware.
use dioxus::prelude::*;
use inputforge_gui_dx::components::{
    Button, ButtonVariant, Card, CardPadding, Field, InlineFeedback, InlineTextEditor,
    KeyCaptureButton, NativeNumberInput, RadioGroup, RadioOption, ReadoutCaption, RebindField,
    SegmentedControl, SegmentedControlOption, Select, SelectOption, SelectableRow, Stack,
    TextInputVariant,
};

#[component]
pub(super) fn EditingGallery() -> Element {
    let mut raw = use_signal(|| "8".to_owned());
    let mut alias = use_signal(|| "Wheel été 🕹".to_owned());
    let mut segment = use_signal(|| "replace".to_owned());
    let mut source = use_signal(|| "blank".to_owned());
    let mut listening = use_signal(|| false);
    let mut key_listening = use_signal(|| false);
    rsx! {
        section {
            h2 { "Native number and inline editor" }
            Card { padding: CardPadding::Md,
                Stack { gap: "--space-3",
                    Field {
                        label: "Raw native change",
                        for_id: "gallery-native-number",
                        helper: "Commits the native change string without parsing.",
                        helper_id: "gallery-native-number-help",
                        NativeNumberInput {
                            id: "gallery-native-number",
                            value: raw(),
                            min: "0",
                            max: "64",
                            step: "1",
                            aria_describedby: "gallery-native-number-help",
                            onchange: move |event: FormEvent| raw.set(event.value()),
                        }
                    }
                    ReadoutCaption { "Last native change: {raw}" }
                    InlineTextEditor {
                        id: "gallery-inline-editor",
                        aria_label: "Inline device alias",
                        value: alias,
                        oninput: move |event: FormEvent| alias.set(event.value()),
                    }
                    InlineTextEditor {
                        id: "gallery-title-editor",
                        aria_label: "Inline title",
                        value: "Profile title".to_owned(),
                        variant: TextInputVariant::Title,
                    }
                }
            }
        }
        section {
            h2 { "Radio source details" }
            RadioGroup { label: "Preview profile source",
                RadioOption {
                    id: "gallery-source-blank",
                    name: "gallery-profile-source",
                    value: "blank",
                    label: "Blank",
                    description: "Start with an empty preview.",
                    checked: source() == "blank",
                    onchange: move |_| source.set("blank".to_owned()),
                }
                RadioOption {
                    id: "gallery-source-copy",
                    name: "gallery-profile-source",
                    value: "copy",
                    label: "Copy",
                    checked: source() == "copy",
                    onchange: move |_| source.set("copy".to_owned()),
                    details: rsx! {
                        Select {
                            id: "gallery-source-profile",
                            aria_label: "Profile to copy",
                            value: "demo",
                            disabled: source() != "copy",
                            options: vec![
                                SelectOption {
                                    value: "demo".into(),
                                    label: "Demo profile".into(),
                                    disabled: false,
                                    class: None,
                                },
                            ],
                        }
                    },
                }
            }
        }
        section {
            h2 { "Compound row and segmented actions" }
            SelectableRow { aria_label: "Preview mapping row", selected: true,
                InlineTextEditor {
                    id: "gallery-row-editor",
                    aria_label: "Preview mapping name",
                    value: alias,
                    oninput: move |event: FormEvent| alias.set(event.value()),
                }
                Button { variant: ButtonVariant::Ghost, "Inspect preview" }
            }
            SegmentedControl { aria_label: "Preview replacement strategy",
                SegmentedControlOption {
                    value: "replace",
                    selected: segment() == "replace",
                    onclick: move |_| segment.set("replace".into()),
                    "Replace"
                }
                SegmentedControlOption {
                    value: "append",
                    selected: segment() == "append",
                    onclick: move |_| segment.set("append".into()),
                    "Append"
                }
            }
        }
        section {
            h2 { "Capture presentation" }
            InlineFeedback { "These actions toggle preview state and do not listen to hardware." }
            RebindField {
                source_label: "Unbound",
                unbound: true,
                listening: listening(),
                action_label: "Preview rebind",
                cancel_label: "Cancel preview rebind",
                onclick: move |_| listening.set(true),
                oncancel: move |_| listening.set(false),
            }
            KeyCaptureButton {
                value: if key_listening() { "Press a key…" } else { "F12" },
                aria_label: "Preview keyboard capture",
                listening: key_listening(),
                hint: "Click to toggle the preview.",
                onclick: move |_| key_listening.toggle(),
            }
        }
    }
}
