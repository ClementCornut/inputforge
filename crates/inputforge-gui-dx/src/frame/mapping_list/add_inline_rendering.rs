//! Add-pad markup, labels and existing workflow callback wiring.
use super::add_inline::submit;
use crate::{
    capture::Capture,
    components::{Badge, BadgeVariant, Button, ButtonSize, ButtonVariant},
    frame::mapping_list::source_label,
    mapping_edits::Bindings,
    mapping_rail::{AddIntent, AddView, Phase, Target as RailTarget},
    snapshots::ConfigSnapshot,
};
use dioxus::prelude::*;
#[path = "add_captured.rs"]
mod captured;
use captured::captured;
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
        Phase::Resting => {
            rsx! {
                div {
                    class: "if-add-inline if-add-inline--resting",
                    Button {
                        presentation: crate::components::ButtonPresentation::Create,
                        size: ButtonSize::Sm,
                        onclick: move |_| begin.call(None),
                        aria_label: "Add mapping",
                        "+ Add mapping"
                    }
                }
            }
        }
        Phase::Capturing | Phase::Captured(_) => {
            captured(phase, config, state, name, &bindings, capture)
        }
        Phase::Collision(existing) => {
            let existing_name = existing.name.as_deref().unwrap_or("(unnamed)");
            let captured_label = source_label::format(&existing.input, config);
            rsx! {
                div {
                    class: "if-add-inline if-add-inline--collision",
                    div {
                        class: "if-add-inline__collision-text",
                        Badge {
                            variant: BadgeVariant::Warning,
                            "Collision"
                        }
                        span {
                            class: "if-add-inline__collision-text-body",
                            " {captured_label} already mapped to "
                            strong {

                                "{existing_name}"
                            }
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
