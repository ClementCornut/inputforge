//! Inline `+` add-mode flow.
//!
//! Renders an F2 `TextInput` where the `+` tail-tab normally sits.
//! Enter dispatches `AddMode` (on Valid) and optimistically points
//! `editing_mode` at the new name; the parent's
//! `use_effect`-on-`modes` then focuses the new tab once it appears
//! in the snapshot. Esc and blur-with-invalid revert and close. Blur
//! with valid commits.
//!
//! Open-state is parent-driven: the parent owns `Signal<bool>` (for
//! add) or `Signal<Option<String>>` (for rename) and the inline
//! component sets it back to closed on commit/cancel. This avoids
//! `Signal::new_in_scope` (not a supported public API) and keeps
//! reactivity centralized in the parent.

use std::sync::mpsc::Sender;

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;

use crate::components::{InputSize, TextInput};
use crate::context::AppContext;
use crate::frame::view_state::ViewState;

use super::logic::{NameValidation, validate_mode_name};

/// Shared commit path. Each event handler clones its `Sender` and call this
/// directly; that keeps the per-handler closures `Fn` (mut-free) so they may
/// be referenced from both `onkeydown` and `onfocusout` without `FnOnce`
/// move-out errors.
#[allow(
    clippy::too_many_arguments,
    reason = "Two callers (Enter + blur) share this commit path; threading the \
              state explicitly keeps the call sites Fn-clean (no FnMut move-out)."
)]
fn run_commit(
    raw: &str,
    modes: &[String],
    commands: &Sender<EngineCommand>,
    mut editing: Signal<String>,
    mut value: Signal<String>,
    mut error_msg: Signal<Option<String>>,
    mut open: Signal<bool>,
    mut pending_focus: Signal<Option<String>>,
) {
    match validate_mode_name(raw, modes, None) {
        NameValidation::Valid(name) => {
            let _ = commands.send(EngineCommand::AddMode {
                name: name.clone(),
                parent: None,
            });
            editing.set(name.clone());
            // Hand the new name to ModeTabs's pending_focus effect so it
            // focuses the new tab once the engine snapshot makes it
            // mountable. Decoupled from `editing` so an Escape-cancel
            // mid round-trip doesn't desync the focus target.
            pending_focus.set(Some(name));
            value.set(String::new());
            error_msg.set(None);
            open.set(false);
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
    mut value: Signal<String>,
    mut error_msg: Signal<Option<String>>,
    mut open: Signal<bool>,
) {
    value.set(String::new());
    error_msg.set(None);
    open.set(false);
}

#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures (the macro suggests \
              shorthand-with-no-prop-name as a fix, which would erase the intent). \
              This is a macro-level artifact, not authored qualifications."
)]
pub(crate) fn AddInline(
    open: Signal<bool>,
    /// Owned by the parent. Set on a successful commit so the parent's
    /// focus effect can target the new tab once it appears in modes.
    pending_focus: Signal<Option<String>>,
) -> Element {
    tracing::trace!(target: "frame::render", region = "mode_tabs::add_inline");
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let modes = use_memo(move || ctx.meta.read().modes.clone());

    let mut value: Signal<String> = use_signal(String::new);
    let mut error_msg: Signal<Option<String>> = use_signal(|| None);
    let editing = view.editing_mode;

    // Per-handler clones. `Sender` is `!Copy` but trivially clonable; cloning
    // upfront lets each event closure own its sender and stay `Fn`.
    let commands_for_keydown = ctx.commands.clone();
    let commands_for_blur = ctx.commands.clone();
    let modes_for_keydown = modes.read().clone();
    let modes_for_blur = modes.read().clone();

    // Single global id is safe because at most one AddInline mounts at
    // a time — the `+` button toggles into one inline editor, never two.
    // RenameInline derives a per-`from`-name id (different invariant:
    // up to N tabs, but only one rename can be active at once).
    let error_id = "mode-name-error-add";

    rsx! {
        div {
            class: "if-mode-tab if-mode-tab--add-inline",
            onkeydown: move |evt: KeyboardEvent| {
                match evt.key() {
                    Key::Enter => {
                        evt.prevent_default();
                        let raw = value.read().clone();
                        run_commit(
                            &raw,
                            &modes_for_keydown,
                            &commands_for_keydown,
                            editing,
                            value,
                            error_msg,
                            open,
                            pending_focus,
                        );
                    }
                    Key::Escape => {
                        evt.prevent_default();
                        run_revert(value, error_msg, open);
                    }
                    _ => {}
                }
            },
            onfocusout: move |evt: FocusEvent| {
                let raw = value.read().clone();
                if raw.trim().is_empty() {
                    run_revert(value, error_msg, open);
                } else {
                    run_commit(
                        &raw,
                        &modes_for_blur,
                        &commands_for_blur,
                        editing,
                        value,
                        error_msg,
                        open,
                        pending_focus,
                    );
                }
                let _ = evt;
            },
            TextInput {
                value: ReadSignal::from(value),
                size: InputSize::Sm,
                placeholder: "New mode name".to_owned(),
                invalid: error_msg.read().is_some(),
                // Only point at the error span when it's actually mounted —
                // otherwise the IDREF dangles. Tracks the same gate as the
                // <span id="{error_id}"> branch below.
                aria_describedby: error_msg.read().as_ref().map(|_| error_id.to_owned()),
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
