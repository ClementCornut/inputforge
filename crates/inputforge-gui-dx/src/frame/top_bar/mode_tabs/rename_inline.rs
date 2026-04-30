//! Inline rename flow. Same shape as add, but the `from` name is preserved
//! and the engine command is `RenameMode`.

use std::sync::mpsc::Sender;

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;

use crate::components::{InputSize, TextInput};
use crate::context::AppContext;
use crate::frame::view_state::ViewState;

use super::logic::{NameValidation, validate_mode_name};

/// Shared commit path. Each event handler clones its `Sender` and call this
/// directly; that keeps the per-handler closures `Fn` so they may be
/// referenced from both `onkeydown` and `onfocusout` without `FnOnce`
/// move-out errors.
#[allow(
    clippy::too_many_arguments,
    reason = "Two callers (Enter + blur) share this commit path; threading the \
              state explicitly keeps the call sites Fn-clean (no FnMut move-out)."
)]
fn run_commit(
    raw: &str,
    from: &str,
    modes: &[String],
    commands: &Sender<EngineCommand>,
    mut editing: Signal<String>,
    mut error_msg: Signal<Option<String>>,
    mut state: Signal<Option<String>>,
) {
    match validate_mode_name(raw, modes, Some(from)) {
        NameValidation::Valid(name) => {
            if name != from {
                let _ = commands.send(EngineCommand::RenameMode {
                    from: from.to_owned(),
                    to: name.clone(),
                });
                if *editing.peek() == from {
                    editing.set(name);
                }
            }
            error_msg.set(None);
            state.set(None);
        }
        NameValidation::Empty => {
            error_msg.set(Some("Name cannot be empty".to_owned()));
        }
        NameValidation::Duplicate { name } => {
            error_msg.set(Some(format!("'{name}' already exists")));
        }
        NameValidation::TooLong { len, max } => {
            error_msg.set(Some(format!("Name is too long ({len}/{max} characters)")));
        }
    }
}

fn run_revert(
    from: &str,
    mut value: Signal<String>,
    mut error_msg: Signal<Option<String>>,
    mut state: Signal<Option<String>>,
) {
    value.set(from.to_owned());
    error_msg.set(None);
    state.set(None);
}

/// Parent-driven rename inline editor. The parent owns
/// `state: Signal<Option<String>>` — when this matches `from`, the
/// editor is open. The component closes by calling `state.set(None)`.
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures (the macro suggests \
              shorthand-with-no-prop-name as a fix, which would erase the intent). \
              This is a macro-level artifact, not authored qualifications."
)]
pub(crate) fn RenameInline(from: String, state: Signal<Option<String>>) -> Element {
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let modes = use_memo(move || ctx.meta.read().modes.clone());

    let mut value: Signal<String> = use_signal(|| from.clone());
    let mut error_msg: Signal<Option<String>> = use_signal(|| None);
    let editing = view.editing_mode;

    // Per-handler clones. `Sender` is `!Copy` but trivially clonable; cloning
    // upfront lets each event closure own its sender and stay `Fn`.
    let commands_for_keydown = ctx.commands.clone();
    let commands_for_blur = ctx.commands.clone();
    let modes_for_keydown = modes.read().clone();
    let modes_for_blur = modes.read().clone();
    let from_for_keydown = from.clone();
    let from_for_blur = from.clone();
    let from_for_revert_kb = from.clone();
    let from_for_revert_blur = from.clone();

    let error_id = format!("mode-name-error-{from}");
    let error_id_for_input = error_id.clone();

    rsx! {
        div {
            class: "if-mode-tab if-mode-tab--rename-inline",
            onkeydown: move |evt: KeyboardEvent| {
                match evt.key() {
                    Key::Enter => {
                        evt.prevent_default();
                        let raw = value.read().clone();
                        run_commit(
                            &raw,
                            &from_for_keydown,
                            &modes_for_keydown,
                            &commands_for_keydown,
                            editing,
                            error_msg,
                            state,
                        );
                    }
                    Key::Escape => {
                        evt.prevent_default();
                        run_revert(&from_for_revert_kb, value, error_msg, state);
                    }
                    _ => {}
                }
            },
            onfocusout: move |evt: FocusEvent| {
                let raw = value.read().clone();
                if raw.trim().is_empty() {
                    run_revert(&from_for_revert_blur, value, error_msg, state);
                } else {
                    run_commit(
                        &raw,
                        &from_for_blur,
                        &modes_for_blur,
                        &commands_for_blur,
                        editing,
                        error_msg,
                        state,
                    );
                }
                let _ = evt;
            },
            TextInput {
                value: ReadSignal::from(value),
                size: InputSize::Sm,
                invalid: error_msg.read().is_some(),
                aria_describedby: error_id_for_input,
                onmounted: move |evt: MountedEvent| {
                    spawn(async move {
                        let _ = evt.data().set_focus(true).await;
                    });
                },
                oninput: move |evt: FormEvent| {
                    value.set(evt.value());
                    error_msg.set(None);
                },
            }
            if let Some(msg) = error_msg.read().as_ref() {
                span {
                    id: "{error_id}",
                    role: "alert",
                    "aria-live": "assertive",
                    class: "if-mode-tab__error",
                    "{msg}"
                }
            }
        }
    }
}
