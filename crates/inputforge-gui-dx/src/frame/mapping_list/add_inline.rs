//! `+ Add mapping` inline state machine. See spec section "+ Add mapping
//! state machine" for the full transition table.
//!
//! State graph:
//!
//! ```text
//!   Resting
//!     | click dashed row | force_expanded rising edge
//!     v
//!   CapturingArmed  ----- cap.captured fires ------> Captured | Collision
//!     |                                                      |       |
//!     | cap.active flips false externally                    |       |
//!     v                                                      |       |
//!   CapturingDisarmed                                         |       |
//!     | click pad to re-arm -> CapturingArmed                |       |
//!     | Esc                                                   |       |
//!     v                                                      v       v
//!                                                          Resting <- Esc/Cancel/Add
//! ```
//!
//! Collision drift: every effect tick, the `Collision` arm re-validates
//! against `cfg.mappings` for the active mode. If the existing mapping is
//! gone, the state transitions to `Captured` so the user can complete the
//! add without re-pressing the input.

use std::sync::mpsc::Sender;

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;
use inputforge_core::types::{InputAddress, InputId};

use crate::components::{Button, ButtonSize, ButtonVariant, IconButton, InputSize, TextInput};
use crate::context::AppContext;
use crate::frame::mapping_list::source_label;
use crate::frame::view_state::ViewState;
use crate::icons::Icon as IconKind;
use crate::patterns::live_capture::{CaptureFilter, LiveCapture};

#[derive(Debug, Clone, PartialEq)]
enum AddState {
    Resting,
    CapturingArmed,
    CapturingDisarmed,
    Captured {
        addr: InputAddress,
    },
    Collision {
        existing_name: String,
        existing: InputAddress,
    },
}

/// Free-function commit path. Keeps the per-arm closures `Fn` (no
/// `FnMut` move-out errors when the same dispatch logic is referenced
/// from both the Captured/Enter handler and the Captured/Add button).
fn dispatch_add_helper(
    addr: InputAddress,
    name_value: &str,
    view: ViewState,
    commands: &Sender<EngineCommand>,
) {
    let mode_now = view.editing_mode.read().clone();
    let trimmed = name_value.trim();
    let _ = commands.send(EngineCommand::SetMapping {
        input: addr.clone(),
        mode: mode_now.clone(),
        name: if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        },
        actions: vec![],
    });
    let mut sel = view.selected_mapping;
    sel.set(Some((mode_now, addr)));
    tracing::info!(
        target: "f8::mapping_list",
        action = "add",
        "dispatch SetMapping (add)",
    );
}

#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures."
)]
pub(crate) fn AddInline(
    /// When set to `true` from outside, expand directly into `CapturingArmed`,
    /// skipping the Resting -> click step. The component clears this prop
    /// back to `false` once the rising edge has been observed, so the
    /// parent only needs to set it once per open request.
    force_expanded: Signal<bool>,
) -> Element {
    tracing::trace!(target: "frame::render", region = "mapping_list::add_inline");
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let cap = use_context::<LiveCapture>();

    let mut state: Signal<AddState> = use_signal(|| AddState::Resting);
    let mut name: Signal<String> = use_signal(String::new);

    // Honor `force_expanded` from the parent — used by EmptyZeroMappings'
    // primary button to skip the dashed-row click. The component clears
    // the prop after observing the rising edge so the parent only needs
    // to set it once per open request.
    //
    // `use_hook` fires synchronously on first render so the initial mount
    // case (parent passes `Signal::new(true)`) lands in CapturingArmed
    // without waiting for an effect tick — this is how the SSR test
    // `add_inline_force_expanded_arms_capture` observes ARMED status.
    // The `use_effect` below handles subsequent rising edges (parent
    // re-flips false → true after a previous capture cycle closed).
    use_hook(|| {
        let mut force = force_expanded;
        if *force.peek() {
            let mut state = state;
            state.set(AddState::CapturingArmed);
            cap.start.call(CaptureFilter::Any);
            force.set(false);
        }
    });
    let mut force_for_effect = force_expanded;
    use_effect(move || {
        if *force_for_effect.read() {
            state.set(AddState::CapturingArmed);
            cap.start.call(CaptureFilter::Any);
            force_for_effect.set(false);
        }
    });

    // Watch `cap.captured` — when capture lands, transition to Captured
    // or Collision based on whether the address is already mapped in the
    // active editing mode.
    let editing_for_cap = view.editing_mode;
    let ctx_for_cap = ctx.clone();
    use_effect(move || {
        let captured_now = cap.captured.read().clone();
        if *state.peek() != AddState::CapturingArmed {
            return;
        }
        let Some(addr) = captured_now else {
            return;
        };
        let mode_now = editing_for_cap.read().clone();
        let cfg = ctx_for_cap.config.read();
        let collision = cfg
            .mappings
            .iter()
            .find(|m| m.input == addr && m.mode == mode_now);
        let next_state = match collision {
            Some(existing) => AddState::Collision {
                existing_name: existing
                    .name
                    .clone()
                    .unwrap_or_else(|| "(unnamed)".to_owned()),
                existing: existing.input.clone(),
            },
            None => AddState::Captured { addr: addr.clone() },
        };
        drop(cfg);
        cap.cancel.call(());
        state.set(next_state);
    });

    // Watch `cap.active` flipping false externally (Esc handled by the
    // primitive's document-level listener, or `cancel` invoked elsewhere)
    // — transition Armed -> Disarmed.
    use_effect(move || {
        if *cap.active.read() {
            return;
        }
        if *state.peek() == AddState::CapturingArmed {
            state.set(AddState::CapturingDisarmed);
        }
    });

    // Collision drift: re-validate once per polling tick. If `existing` is
    // no longer in cfg.mappings for the active mode, transition to
    // Captured so the user can complete the add without re-pressing.
    let editing_for_drift = view.editing_mode;
    let ctx_for_drift = ctx.clone();
    use_effect(move || {
        let s = state.read().clone();
        if let AddState::Collision { existing, .. } = s {
            let mode_now = editing_for_drift.read().clone();
            let cfg = ctx_for_drift.config.read();
            let still_present = cfg
                .mappings
                .iter()
                .any(|m| m.input == existing && m.mode == mode_now);
            drop(cfg);
            if !still_present {
                state.set(AddState::Captured { addr: existing });
            }
        }
    });

    // Document-level Esc listener that closes the pad whenever LiveCapture
    // is NOT active. While `cap.active == true`, LiveCapture's own Esc
    // listener (Task 8) owns the key and routes Armed → Disarmed; this
    // listener stays out of the way. While `cap.active == false`
    // (Captured / Disarmed / Collision), Esc closes the pad to Resting
    // and clears the typed name.
    //
    // The JS handler short-circuits if the keystroke originated inside the
    // rail's filter input so Task 22's filter-Esc-clears-query routing
    // keeps working without contention.
    //
    // Pattern mirrors Task 8's LiveCapture Esc listener — capture-phase
    // window listener, parked recv loop, no shutdown signal because the
    // listener lives for the lifetime of the AddInline component (which
    // is the lifetime of the rail).
    let esc_listener_mounted: Signal<bool> = use_signal(|| false);
    use_effect(move || {
        let mut mounted = esc_listener_mounted;
        if *mounted.peek() {
            return;
        }
        mounted.set(true);

        spawn(async move {
            let mut handle = document::eval(
                "const h = (ev) => {\n\
                   if (ev.key !== 'Escape') return;\n\
                   // Defer to MappingList's filter-Esc handling when the\n\
                   // filter input is the event target.\n\
                   if (ev.target && ev.target.closest && ev.target.closest('.if-rail__filter')) return;\n\
                   dioxus.send('esc');\n\
                 };\n\
                 window.addEventListener('keydown', h, true);\n\
                 (async () => { while (true) { await dioxus.recv(); } })();\n\
                 ",
            );

            loop {
                match handle.recv::<String>().await {
                    Ok(s) if s == "esc" => {
                        // Gate: only close when LiveCapture is off and the
                        // pad is currently expanded.
                        if *cap.active.read() {
                            continue;
                        }
                        if *state.peek() == AddState::Resting {
                            continue;
                        }
                        state.set(AddState::Resting);
                        name.set(String::new());
                    }
                    _ => break,
                }
            }
        });
    });

    match state.read().clone() {
        AddState::Resting => rsx! {
            div { class: "if-add-inline if-add-inline--resting",
                button {
                    r#type: "button",
                    class: "if-add-inline__dashed-row",
                    onclick: move |_| {
                        state.set(AddState::CapturingArmed);
                        cap.start.call(CaptureFilter::Any);
                    },
                    "aria-label": "Add mapping",
                    "+ Add mapping"
                }
            }
        },
        AddState::CapturingArmed => rsx! {
            div { class: "if-add-inline if-add-inline--armed",
                div { class: "if-add-inline__pad",
                    "Press an input on any device..."
                }
                TextInput {
                    value: ReadSignal::from(name),
                    size: InputSize::Sm,
                    placeholder: "Mapping name (optional)".to_owned(),
                    oninput: move |evt: FormEvent| name.set(evt.value()),
                }
            }
        },
        AddState::CapturingDisarmed => rsx! {
            div {
                class: "if-add-inline if-add-inline--disarmed",
                // Esc handled by AddInline's document-level listener.
                button {
                    r#type: "button",
                    class: "if-add-inline__pad if-add-inline__pad--disarmed",
                    onclick: move |_| {
                        state.set(AddState::CapturingArmed);
                        cap.start.call(CaptureFilter::Any);
                    },
                    "Cancelled - click to capture again"
                }
                TextInput {
                    value: ReadSignal::from(name),
                    size: InputSize::Sm,
                    placeholder: "Mapping name (optional)".to_owned(),
                    oninput: move |evt: FormEvent| name.set(evt.value()),
                }
            }
        },
        AddState::Captured { addr } => {
            let addr_for_enter = addr.clone();
            let addr_for_btn = addr.clone();
            let view_for_enter = view;
            let view_for_btn = view;
            let cmd_for_enter = ctx.commands.clone();
            let cmd_for_btn = ctx.commands.clone();
            let cfg = ctx.config.read();
            let (device_label, input_label) = source_label::split_label(&addr, &cfg);
            drop(cfg);
            // The chip's hue classifies the captured input by kind, sharing
            // the gold/violet/teal vocabulary already used for F8 row glyphs
            // (`glyph-merge`, `glyph-cond`). Selector key is `data-kind`.
            let kind_class = match addr.input {
                InputId::Axis { .. } => "axis",
                InputId::Button { .. } => "button",
                InputId::Hat { .. } => "hat",
            };
            rsx! {
                div {
                    class: "if-add-inline if-add-inline--captured",
                    onkeydown: move |evt: KeyboardEvent| {
                        // Esc is handled by AddInline's document-level
                        // listener above; only Enter (commit) lives here.
                        if evt.key() == Key::Enter {
                            evt.prevent_default();
                            let n = name.read().clone();
                            dispatch_add_helper(
                                addr_for_enter.clone(),
                                &n,
                                view_for_enter,
                                &cmd_for_enter,
                            );
                            state.set(AddState::Resting);
                            name.set(String::new());
                        }
                    },
                    div { class: "if-add-inline__readout",
                        span {
                            class: "if-add-inline__chip",
                            "data-kind": kind_class,
                            "{input_label}"
                        }
                        span { class: "if-add-inline__device", "{device_label}" }
                        IconButton {
                            icon: IconKind::Refresh,
                            label: "Capture a different input",
                            variant: ButtonVariant::Ghost,
                            size: ButtonSize::Sm,
                            onclick: move |_| {
                                // Re-arm capture; keep the typed name so the user
                                // does not have to retype after correcting the input.
                                state.set(AddState::CapturingArmed);
                                cap.start.call(CaptureFilter::Any);
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
                                state.set(AddState::Resting);
                                name.set(String::new());
                            },
                            "Cancel"
                        }
                        Button {
                            variant: ButtonVariant::Primary,
                            size: ButtonSize::Sm,
                            onclick: move |_| {
                                let n = name.read().clone();
                                dispatch_add_helper(
                                    addr_for_btn.clone(),
                                    &n,
                                    view_for_btn,
                                    &cmd_for_btn,
                                );
                                state.set(AddState::Resting);
                                name.set(String::new());
                            },
                            "Add"
                        }
                    }
                }
            }
        }
        AddState::Collision {
            existing_name,
            existing,
        } => {
            let existing_for_btn = existing.clone();
            let cfg = ctx.config.read();
            let captured_label = source_label::format(&existing, &cfg);
            drop(cfg);
            rsx! {
                div { class: "if-add-inline if-add-inline--collision",
                    div { class: "if-add-inline__collision-text",
                        em { "{captured_label} already mapped to " }
                        strong { "{existing_name}" }
                        "."
                    }
                    Button {
                        variant: ButtonVariant::Secondary,
                        onclick: move |_| {
                            let mode_now = view.editing_mode.read().clone();
                            let mut sel = view.selected_mapping;
                            sel.set(Some((mode_now, existing_for_btn.clone())));
                            state.set(AddState::Resting);
                            name.set(String::new());
                        },
                        "Edit existing"
                    }
                    Button {
                        variant: ButtonVariant::Ghost,
                        onclick: move |_| {
                            state.set(AddState::Resting);
                            name.set(String::new());
                        },
                        "Cancel"
                    }
                }
            }
        }
    }
}
