//! Add-pad markup, labels and existing workflow callback wiring.
use super::add_inline::{commit, submit};
use crate::{
    capture::Capture,
    components::{
        Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, Chip, ChipVariant, IconButton,
        InputSize, TextInput,
    },
    frame::mapping_list::source_label,
    icons::Icon as IconKind,
    mapping_edits::Bindings,
    mapping_rail::{AddIntent, AddView, Phase, Target as RailTarget},
    patterns::live_capture::CAPTURE_PROMPT,
    snapshots::ConfigSnapshot,
};
use dioxus::prelude::*;
use inputforge_core::types::InputId;
#[expect(
    unused_qualifications,
    reason = "Dioxus RSX emits redundant qualifications"
)]
pub(super) fn body(
    phase: Phase,
    config: &ConfigSnapshot,
    begin: Callback<Option<RailTarget>>,
    state: Signal<AddView>,
    name: Signal<String>,
    bindings: &Bindings,
    capture: Capture,
) -> Element {
    let bindings = *bindings;
    match phase {
        Phase::Resting => rsx! {
            div { class: "if-add-inline if-add-inline--resting",
                button { r#type: "button", class: "if-add-inline__dashed-row",
                    onclick: move |_| begin.call(None), "aria-label": "Add mapping", "+ Add mapping"
                }
            }
        },
        Phase::Capturing | Phase::Captured(_) => {
            captured(phase, config, state, name, &bindings, capture)
        }
        Phase::Collision(existing) => {
            let existing_name = existing.name.as_deref().unwrap_or("(unnamed)");
            let captured_label = source_label::format(&existing.input, config);
            rsx! {
                div { class: "if-add-inline if-add-inline--collision",
                    div { class: "if-add-inline__collision-text",
                        Badge { variant: BadgeVariant::Warning, "Collision" }
                        span { class: "if-add-inline__collision-text-body",
                            " {captured_label} already mapped to "
                            strong { "{existing_name}" }
                            "."
                        }
                    }
                    Button {
                        variant: ButtonVariant::Secondary,
                        onclick: move |_| {
                            submit(state, name, &bindings, AddIntent::Open);
                        },
                        "Edit existing"
                    }
                    Button {
                        variant: ButtonVariant::Ghost,
                        onclick: move |_| {
                            submit(state, name, &bindings, AddIntent::Cancel);
                        },
                        "Cancel"
                    }
                }
            }
        }
    }
}

#[expect(
    unused_qualifications,
    reason = "Dioxus RSX emits redundant qualifications"
)]
fn captured(
    phase: Phase,
    config: &ConfigSnapshot,
    state: Signal<AddView>,
    mut name: Signal<String>,
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

    let addr_for_enter = captured_addr.clone();
    let addr_for_btn = captured_addr;
    rsx! {
        div {
            class: "if-add-inline if-add-inline--pad",
            onkeydown: move |evt: KeyboardEvent| {
                // Enter commits when an input has been captured.
                // Esc is handled by the document-level listener.
                if evt.key() == Key::Enter
                    && addr_for_enter.is_some()
                {
                    evt.prevent_default();
                    commit(state, name, &bindings);
                }
            },
            div { class: "if-add-inline__readout",
                if is_capturing {
                    span { "aria-label": "Listening for input",
                        Chip {
                            variant: ChipVariant::Capture,
                            class: "if-add-inline__chip--listening".to_owned(),
                        }
                    }
                } else {
                    span { "data-kind": "{kind_class}",
                        Chip {
                            variant: ChipVariant::Capture,
                            "{chip_label}"
                        }
                    }
                }
                span { class: "if-add-inline__device", "{device_label}" }
                IconButton {
                    icon: IconKind::Refresh,
                    label: "Capture a different input",
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    disabled: is_capturing,
                    onclick: move |_| {
                        if submit(state, name, &bindings, AddIntent::Refresh) { capture.start.call(()); }
                    },
                }
            }
            TextInput {
                value: ReadSignal::from(name),
                size: InputSize::Sm,
                placeholder: "Mapping name".to_owned(),
                oninput: move |evt: FormEvent| name.set(evt.value()),
            }
            div { class: "if-add-inline__actions",
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    onclick: move |_| {
                        capture.cancel.call(());
                        submit(state, name, &bindings, AddIntent::Cancel);
                    },
                    "Cancel"
                }
                Button {
                    variant: ButtonVariant::Primary,
                    size: ButtonSize::Sm,
                    disabled: is_capturing,
                    onclick: move |_| {
                        if addr_for_btn.is_some() {
                            commit(state, name, &bindings);
                        }
                    },
                    "Add"
                }
            }
        }
    }
}
