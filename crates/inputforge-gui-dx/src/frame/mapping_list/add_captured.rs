//! Captured input identity, name and commit controls.
use crate::{
    capture::Capture,
    components::{
        Button, ButtonSize, ButtonVariant, Chip, ChipVariant, IconButton, InputSize, TextInput,
    },
    frame::mapping_list::source_label,
    icons::Icon as IconKind,
    mapping_edits::Bindings,
    mapping_rail::{AddIntent, AddView, Phase},
    patterns::live_capture::CAPTURE_PROMPT,
    snapshots::ConfigSnapshot,
};
use dioxus::prelude::*;
use inputforge_core::types::InputId;
pub(super) fn captured(
    phase: Phase,
    config: &ConfigSnapshot,
    state: Signal<AddView>,
    name: Signal<String>,
    bindings: &Bindings,
    capture: Capture,
) -> Element {
    let bindings = *bindings;
    let captured_addr = if let Phase::Captured(addr) = phase {
        Some(addr)
    } else {
        None
    };
    let is_capturing = captured_addr.is_none();
    let (chip_label, device_label, kind_class): (String, String, &'static str) =
        if let Some(addr) = &captured_addr {
            let (device, input) = source_label::split_label(addr, config);
            let kind = match addr
                .input_id()
                .expect("invariant: add_inline addr always bound (built from F8 capture)")
            {
                InputId::Axis { .. } => "axis",
                InputId::Button { .. } => "button",
                InputId::Hat { .. } => "hat",
            };
            (input, device, kind)
        } else {
            (String::new(), CAPTURE_PROMPT.to_owned(), "")
        };
    let onkeydown = move |evt| {
        bindings.ui.add.call((
            crate::mapping_edits::ui::Add {
                state,
                name,
                capture,
            },
            crate::mapping_edits::ui::AddEvent::Key(evt),
        ));
    };
    rsx! {
        div {
            class: "if-add-inline if-add-inline--pad",
            onkeydown,
            div {
                class: "if-add-inline__readout",
                if is_capturing {
                    span {
                        "aria-label": "Listening for input",
                        Chip {
                            variant: ChipVariant::Capture,
                            listening: true,
                        }
                    }
                } else {
                    span {
                        "data-kind": "{kind_class}",
                        Chip {
                            variant: ChipVariant::Capture,
                            "{chip_label}"
                        }
                    }
                }
                span {
                    class: "if-add-inline__device",
                    "{device_label}"
                }
                IconButton {
                    icon: IconKind::Refresh,
                    label: "Capture a different input",
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    disabled: is_capturing,
                    onclick: move |_| {
                        bindings
                            .ui
                            .add
                            .call((
                                crate::mapping_edits::ui::Add {
                                    state,
                                    name,
                                    capture,
                                },
                                crate::mapping_edits::ui::AddEvent::Intent(AddIntent::Refresh),
                            ));
                    },
                }
            }
            {name_and_actions(state, name, &bindings, capture, is_capturing)}
        }
    }
}

fn name_and_actions(
    state: Signal<AddView>,
    name: Signal<String>,
    bindings: &Bindings,
    capture: Capture,
    is_capturing: bool,
) -> Element {
    let bindings = *bindings;
    rsx! {
        TextInput {
            aria_label: "Mapping name",
            id: "new-mapping-name",
            value: ReadSignal::from(name),
            size: InputSize::Sm,
            placeholder: "Mapping name"
                    .to_owned(),
            oninput: move |evt: FormEvent| {
                bindings
                    .ui
                    .add
                    .call((
                        crate::mapping_edits::ui::Add {
                            state,
                            name,
                            capture,
                        },
                        crate::mapping_edits::ui::AddEvent::Input(evt.value()),
                    ));
            },
        }
        div {
            class: "if-add-inline__actions",
            Button {
                variant: ButtonVariant::Ghost,
                size: ButtonSize::Sm,
                onclick: move |_| {
                    bindings
                        .ui
                        .add
                        .call((
                            crate::mapping_edits::ui::Add {
                                state,
                                name,
                                capture,
                            },
                            crate::mapping_edits::ui::AddEvent::Cancel,
                        ));
                },
                "Cancel"
            }
            Button {
                variant: ButtonVariant::Primary,
                size: ButtonSize::Sm,
                disabled: is_capturing,
                onclick: move |_| {
                    bindings
                        .ui
                        .add
                        .call((
                            crate::mapping_edits::ui::Add {
                                state,
                                name,
                                capture,
                            },
                            crate::mapping_edits::ui::AddEvent::Commit,
                        ));
                },
                "Add"
            }
        }
    }
}
