//! `+ Add mapping` inline state machine. See spec section "+ Add mapping
//! state machine" for the full transition table.
//!
//! State graph:
//!
//! ```text
//!   Resting
//!     | click dashed row | force_expanded rising edge
//!     v
//!   Pad { phase: Capturing }
//!     |  ^                                       cap.captured fires
//!     |  |  refresh-icon click (re-arm,            |
//!     |  |  keep typed name)                       |   collision in active mode
//!     |  |                                         v   |
//!     |  └──────────────────────── Pad { phase: Captured(addr) } ──┐
//!     |                                            |               |
//!     | Esc / cap.cancel external                  | Esc / Cancel  | (no collision)
//!     | (closes pad outright)                      | / Add commit  |
//!     v                                            v               v
//!                                              Resting          Collision
//!                                                  ^               |
//!                                                  └── Esc / Cancel / Edit existing
//! ```
//!
//! `Pad` is a single shell with a `PadPhase` discriminator. The chip cell
//! shows a listening animation in `Capturing` and the taxonomy-tinted
//! input identifier in `Captured(addr)`; the refresh icon-button and Add
//! button are disabled during `Capturing` (no captured input yet). Phase
//! flips do not remount the shell, so the typed name and focus carry
//! through.
//!
//! Collision drift: every effect tick, the `Collision` arm re-validates
//! against `cfg.mappings` for the active mode. If the existing mapping is
//! gone, the state transitions to `Pad { phase: Captured(addr) }` so the
//! user can complete the add without re-pressing the input.

use std::sync::mpsc::Sender;

use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::engine::EngineCommand;
use inputforge_core::types::{InputAddress, InputId};

use crate::components::{
    Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, Chip, ChipVariant, IconButton,
    InputSize, TextInput,
};
use crate::context::{AppContext, MappingSummary};
use crate::frame::mapping_list::source_label;
use crate::frame::view_state::ViewState;
use crate::icons::Icon as IconKind;
use crate::patterns::live_capture::{
    CAPTURE_PROMPT, CaptureFilter, LiveCapture, is_current_capture_session,
};

#[derive(Debug, Clone, PartialEq)]
enum AddState {
    /// Pad collapsed; only the dashed `+ Add mapping` row renders.
    Resting,
    /// Pad expanded. The shell (chip, device cell, refresh icon, name
    /// input, actions row) is identical between phases, only the chip
    /// cell and the disabled-state of refresh/Add change. Phase flips do
    /// not remount the shell, so the typed name carries through and
    /// focus stays put.
    Pad { phase: PadPhase },
    /// Capture landed on an input that is already mapped in the active
    /// mode. Distinct from `Pad` because the action row changes
    /// semantics (Edit existing / Cancel) and the chip would otherwise
    /// have to mean two different things ("what you captured" vs "what
    /// was already there").
    Collision {
        existing_name: String,
        existing: InputAddress,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum PadPhase {
    /// `LiveCapture` is armed. Chip shows the listening animation; refresh
    /// icon and Add button are disabled (no captured input yet).
    Capturing,
    /// `LiveCapture` fired. Chip shows the taxonomy-tinted input
    /// identifier; refresh and Add are enabled.
    Captured(InputAddress),
}

/// Free-function commit path. Keeps the per-arm closures `Fn` (no
/// `FnMut` move-out errors when the same dispatch logic is referenced
/// from both the Captured/Enter handler and the Captured/Add button).
///
/// `actions` is the full action vec to commit. For a fresh `+ Add
/// mapping` flow it's `vec![]` (placeholder until F9's action editor
/// lands). For a Duplicate flow it's the source mapping's cloned
/// actions, looked up from `active_profile` at dispatch time.
fn dispatch_add_helper(
    addr: InputAddress,
    name_value: &str,
    actions: Vec<Action>,
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
        actions,
    });
    let mut sel = view.selected_mapping;
    sel.set(Some((mode_now, addr)));
    tracing::info!(
        target: "f8::mapping_list",
        action = "add",
        "dispatch SetMapping (add)",
    );
}

/// Resolve the actions vec to commit. `Some(source)` means we're in a
/// Duplicate flow, clone the source mapping's actions from the active
/// profile (or fall back to an empty vec if the source disappeared
/// mid-flow). `None` means a fresh add, empty actions placeholder.
fn resolve_actions(source: Option<&MappingSummary>, ctx: &AppContext) -> Vec<Action> {
    match source {
        None => Vec::new(),
        Some(src) => ctx
            .state
            .read()
            .active_profile
            .as_ref()
            .and_then(|p| {
                p.find_mapping(&src.input, &src.mode)
                    .map(|m| m.actions.clone())
            })
            .unwrap_or_default(),
    }
}

#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures."
)]
pub(crate) fn AddInline(
    /// When set to `true` from outside, expand directly into
    /// `Pad { Capturing }`, skipping the Resting -> click step. The
    /// component clears this prop back to `false` once the rising edge
    /// has been observed, so the parent only needs to set it once per
    /// open request.
    force_expanded: Signal<bool>,
    /// When set to `Some(source)` from outside (right-click "Duplicate"
    /// in the context menu), open the pad in `Pad { Capturing }` with
    /// the name pre-filled to `<source.name> (copy)` and stash the
    /// source for the commit path so the dispatched `SetMapping`
    /// carries the source's cloned actions instead of an empty vec.
    /// The component clears this prop back to `None` once the rising
    /// edge has been observed (one-shot, like `force_expanded`).
    pending_duplicate: Signal<Option<MappingSummary>>,
) -> Element {
    tracing::trace!(target: "frame::render", region = "mapping_list::add_inline");
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let cap = use_context::<LiveCapture>();

    let mut state: Signal<AddState> = use_signal(|| AddState::Resting);
    let mut name: Signal<String> = use_signal(String::new);
    // Local stash of the duplicate source. Held throughout a single
    // duplicate flow so Refresh-icon recapture and Add-commit can both
    // see it. Cleared on Cancel / Esc / commit / Resting transitions.
    let mut local_source: Signal<Option<MappingSummary>> = use_signal(|| None);
    let mut capture_session: Signal<Option<u64>> = use_signal(|| None);

    // Honor `force_expanded` from the parent, used by EmptyZeroMappings'
    // primary button to skip the dashed-row click. The component clears
    // the prop after observing the rising edge so the parent only needs
    // to set it once per open request.
    //
    // `use_hook` fires synchronously on first render so the initial mount
    // case (parent passes `Signal::new(true)`) lands in `Pad { Capturing }`
    // without waiting for an effect tick, this is how the SSR test
    // `add_inline_force_expanded_arms_capture` observes the listening pad.
    // The `use_effect` below handles subsequent rising edges (parent
    // re-flips false → true after a previous capture cycle closed).
    use_hook(|| {
        let mut force = force_expanded;
        if *force.peek() {
            let mut state = state;
            state.set(AddState::Pad {
                phase: PadPhase::Capturing,
            });
            cap.start.call(CaptureFilter::Any);
            capture_session.set(Some(*cap.session.peek()));
            force.set(false);
        }
    });
    let mut force_for_effect = force_expanded;
    use_effect(move || {
        if *force_for_effect.read() {
            state.set(AddState::Pad {
                phase: PadPhase::Capturing,
            });
            cap.start.call(CaptureFilter::Any);
            capture_session.set(Some(*cap.session.peek()));
            force_for_effect.set(false);
        }
    });

    // Honor `pending_duplicate` from the parent, set by the right-click
    // menu's Duplicate item. Pre-fill name with `<source.name> (copy)`,
    // stash the source for the commit path, arm capture, and clear the
    // parent's signal (one-shot rising edge, same pattern as
    // `force_expanded`).
    use_hook(|| {
        let mut pending = pending_duplicate;
        // Snapshot then drop the borrow before mutating `pending`.
        let snapshot = pending.peek().clone();
        if let Some(source) = snapshot {
            let prefilled = format!("{} (copy)", source.name.as_deref().unwrap_or("(unnamed)"),);
            let mut name = name;
            name.set(prefilled);
            let mut local = local_source;
            local.set(Some(source));
            let mut state = state;
            state.set(AddState::Pad {
                phase: PadPhase::Capturing,
            });
            cap.start.call(CaptureFilter::Any);
            capture_session.set(Some(*cap.session.peek()));
            pending.set(None);
        }
    });
    let mut pending_for_effect = pending_duplicate;
    use_effect(move || {
        let snapshot = pending_for_effect.read().clone();
        if let Some(source) = snapshot {
            let prefilled = format!("{} (copy)", source.name.as_deref().unwrap_or("(unnamed)"),);
            name.set(prefilled);
            local_source.set(Some(source));
            state.set(AddState::Pad {
                phase: PadPhase::Capturing,
            });
            cap.start.call(CaptureFilter::Any);
            capture_session.set(Some(*cap.session.peek()));
            pending_for_effect.set(None);
        }
    });

    // Watch `cap.captured`, when capture lands, transition the pad's
    // phase to `Captured(addr)`, or transition the whole state to
    // `Collision` if the address is already mapped in the active mode.
    let editing_for_cap = view.editing_mode;
    let ctx_for_cap = ctx.clone();
    use_effect(move || {
        let captured_now = cap.captured.read().clone();
        if !matches!(
            *state.peek(),
            AddState::Pad {
                phase: PadPhase::Capturing,
            }
        ) {
            return;
        }
        if !is_current_capture_session(*capture_session.peek(), *cap.session.peek()) {
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
            None => AddState::Pad {
                phase: PadPhase::Captured(addr.clone()),
            },
        };
        drop(cfg);
        cap.cancel.call(());
        capture_session.set(None);
        state.set(next_state);
    });

    // External-cancel / supersede watcher: `cap.active=false` handles Esc
    // and explicit cancel; session mismatch handles another capture surface
    // starting while the global capture remains active. Either case closes
    // the pad outright (per design choice 2.a, first Esc closes, no
    // Disarmed intermediate).
    //
    // Distinguishing case: when `cap.captured` is `Some`, the captured-
    // watcher above is doing the work; this watcher must not race it.
    use_effect(move || {
        let active_now = *cap.active.read();
        let current_session = *cap.session.read();
        if !matches!(
            *state.peek(),
            AddState::Pad {
                phase: PadPhase::Capturing,
            }
        ) {
            return;
        }
        if cap.captured.peek().is_some() {
            return;
        }
        let owned_session = *capture_session.peek();
        if active_now && is_current_capture_session(owned_session, current_session) {
            return;
        }
        state.set(AddState::Resting);
        name.set(String::new());
        local_source.set(None);
        capture_session.set(None);
    });

    // Collision drift: re-validate once per polling tick. If `existing` is
    // no longer in cfg.mappings for the active mode, transition to
    // `Pad { Captured(addr) }` so the user can complete the add without
    // re-pressing.
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
                state.set(AddState::Pad {
                    phase: PadPhase::Captured(existing),
                });
            }
        }
    });

    // Document-level Esc listener that closes the pad whenever LiveCapture
    // is NOT active. While `cap.active == true` (Pad { Capturing }),
    // LiveCapture's own Esc listener (Task 8) owns the key and fires
    // `cap.cancel`; the external-cancel watcher above then closes the
    // pad. This listener handles the cap.active==false cases (Pad with
    // Captured phase / Collision), where Esc closes the pad and clears
    // the typed name.
    //
    // The JS handler short-circuits if the keystroke originated inside the
    // rail's filter input so Task 22's filter-Esc-clears-query routing
    // keeps working without contention.
    //
    // Pattern mirrors Task 8's LiveCapture Esc listener, capture-phase
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
                        local_source.set(None);
                        capture_session.set(None);
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
                        state.set(AddState::Pad {
                            phase: PadPhase::Capturing,
                        });
                        cap.start.call(CaptureFilter::Any);
                        capture_session.set(Some(*cap.session.peek()));
                    },
                    "aria-label": "Add mapping",
                    "+ Add mapping"
                }
            }
        },
        AddState::Pad { phase } => {
            // Compute phase-dependent content up-front so the rsx body is
            // a single shell (no nested conditional that would reshape the
            // DOM and reset focus / animation timing).
            let captured_addr: Option<InputAddress> = match &phase {
                PadPhase::Capturing => None,
                PadPhase::Captured(addr) => Some(addr.clone()),
            };
            let is_capturing = captured_addr.is_none();

            let cfg = ctx.config.read();
            let (chip_label, device_label, kind_class): (String, String, &'static str) =
                if let Some(addr) = &captured_addr {
                    let (device, input) = source_label::split_label(addr, &cfg);
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
            drop(cfg);

            // Closures need owned captures; clone once per arm (Captured
            // phase only, Capturing's clones are unused but cheap).
            let addr_for_enter = captured_addr.clone();
            let addr_for_btn = captured_addr.clone();
            let view_for_enter = view;
            let view_for_btn = view;
            let cmd_for_enter = ctx.commands.clone();
            let cmd_for_btn = ctx.commands.clone();
            // For the commit path: clone the AppContext so closures can
            // call `resolve_actions` to look up the duplicate source's
            // actions in the active profile (or use vec![] for fresh add).
            let ctx_for_enter = ctx.clone();
            let ctx_for_btn = ctx.clone();

            rsx! {
                div {
                    class: "if-add-inline if-add-inline--pad",
                    onkeydown: move |evt: KeyboardEvent| {
                        // Enter commits when an input has been captured.
                        // Esc is handled by the document-level listener.
                        if evt.key() == Key::Enter
                            && let Some(addr) = &addr_for_enter
                        {
                            evt.prevent_default();
                            let n = name.read().clone();
                            let source_snap = local_source.peek().clone();
                            let actions = resolve_actions(source_snap.as_ref(), &ctx_for_enter);
                            dispatch_add_helper(
                                addr.clone(),
                                &n,
                                actions,
                                view_for_enter,
                                &cmd_for_enter,
                            );
                            state.set(AddState::Resting);
                            name.set(String::new());
                            local_source.set(None);
                            capture_session.set(None);
                        }
                    },
                    div { class: "if-add-inline__readout",
                        if is_capturing {
                            span { "aria-label": "Listening for input",
                                Chip {
                                    variant: ChipVariant::Capture,
                                    class: "if-add-inline__chip if-add-inline__chip--listening"
                                        .to_owned(),
                                }
                            }
                        } else {
                            span { "data-kind": "{kind_class}",
                                Chip {
                                    variant: ChipVariant::Capture,
                                    class: "if-add-inline__chip".to_owned(),
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
                                state.set(AddState::Pad {
                                    phase: PadPhase::Capturing,
                                });
                                cap.start.call(CaptureFilter::Any);
                                capture_session.set(Some(*cap.session.peek()));
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
                                cap.cancel.call(());
                                state.set(AddState::Resting);
                                name.set(String::new());
                                local_source.set(None);
                                capture_session.set(None);
                            },
                            "Cancel"
                        }
                        Button {
                            variant: ButtonVariant::Primary,
                            size: ButtonSize::Sm,
                            disabled: is_capturing,
                            onclick: move |_| {
                                if let Some(addr) = &addr_for_btn {
                                    let n = name.read().clone();
                                    let source_snap = local_source.peek().clone();
                                    let actions = resolve_actions(source_snap.as_ref(), &ctx_for_btn);
                                    dispatch_add_helper(
                                        addr.clone(),
                                        &n,
                                        actions,
                                        view_for_btn,
                                        &cmd_for_btn,
                                    );
                                    state.set(AddState::Resting);
                                    name.set(String::new());
                                    local_source.set(None);
                                    capture_session.set(None);
                                }
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
                            let mode_now = view.editing_mode.read().clone();
                            let mut sel = view.selected_mapping;
                            sel.set(Some((mode_now, existing_for_btn.clone())));
                            state.set(AddState::Resting);
                            name.set(String::new());
                            local_source.set(None);
                            capture_session.set(None);
                        },
                        "Edit existing"
                    }
                    Button {
                        variant: ButtonVariant::Ghost,
                        onclick: move |_| {
                            state.set(AddState::Resting);
                            name.set(String::new());
                            local_source.set(None);
                            capture_session.set(None);
                        },
                        "Cancel"
                    }
                }
            }
        }
    }
}
