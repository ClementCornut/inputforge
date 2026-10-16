//! Inline rename for an existing mapping row. Mirrors F7's
//! `mode_tabs::rename_inline::RenameInline`, Enter dispatches
//! `SetMapping` with the same actions + new name; Esc reverts; blur
//! with empty value reverts; blur with non-empty value commits.
//!
//! `TextInput` only forwards `oninput` / `onmounted` / `class` to its
//! inner `<input>` (see `components::text_input`). To attach `onkeydown`
//! and `onfocusout`, this component wraps the input in a `<div>` and
//! relies on event bubbling, same shape F7's `mode_tabs::rename_inline`
//! uses.

use dioxus::prelude::*;

use inputforge_core::types::InputAddress;

use crate::components::{InputSize, TextInput};
use crate::context::MappingSummary;
use crate::mapping_rail::{Lookup, Scope, Target};

/// Capture the app-prepared mapping once, alongside the local draft text.
pub(super) fn capture(summary: &MappingSummary) -> Option<Target> {
    let scope = Scope {
        profile:
            crate::frame::mapping_editor::pipeline::stage_body::instruments::stage_dispatch::profile(
            ),
        mode: summary.mode.clone(),
    };
    consume_context::<crate::mapping_edits::Bindings>()
        .menu
        .call(Lookup {
            scope,
            input: summary.input.clone(),
            name: summary.name.clone(),
            filtered: false,
        })
        .map(|menu| menu.target)
}

/// Submit the draft's original target; queued edits are validated by the app.
pub(super) fn run_commit(
    raw: &str,
    target: Option<&Target>,
    state: Signal<Option<InputAddress>>,
) -> Result<(), String> {
    run_inline(raw, target, state, false)
}
fn run_inline(
    raw: &str,
    target: Option<&Target>,
    mut state: Signal<Option<InputAddress>>,
    cancel: bool,
) -> Result<(), String> {
    let result = consume_context::<crate::mapping_edits::Bindings>()
        .inline
        .call(crate::mapping_edits::Inline {
            target: target.cloned(),
            edit: crate::mapping_edits::Edit::Rename(raw.to_owned()),
            label: format!(
                "rename: '{}' -> '{}'",
                target
                    .and_then(|target| target.before.name.as_deref())
                    .unwrap_or(""),
                raw
            ),
            editor: false,
            cancel,
        });
    if result == Ok(true)
        && let Some(target) = target
    {
        tracing::info!(target: "f8::mapping_list", action = "rename", ?target.before.input, mode = %target.before.mode, "dispatched mapping rename");
    }
    if result.is_ok() {
        state.set(None);
    }
    result.map(|_| ())
}

/// Parent-driven inline rename editor for a mapping row. The parent owns
/// `state: Signal<Option<InputAddress>>`, when this matches
/// `summary.input`, the editor is open. The component closes by calling
/// `state.set(None)`.
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures (the macro suggests \
              shorthand-with-no-prop-name as a fix, which would erase the intent). \
              This is a macro-level artifact, not authored qualifications."
)]
pub(crate) fn RenameInline(
    summary: MappingSummary,
    state: Signal<Option<InputAddress>>,
) -> Element {
    tracing::trace!(target: "frame::render", region = "mapping_list::rename_inline");
    crate::frame::mapping_editor::pipeline::stage_body::instruments::stage_dispatch::use_origin();

    let initial = summary.name.clone().unwrap_or_default();
    let mut value: Signal<String> = use_signal(|| initial);

    let mut error = use_signal(|| None::<String>);
    let target = use_hook(|| capture(&summary));
    let target_for_kb = target.clone();
    let target_for_cancel = target.clone();

    rsx! {
        div {
            class: "if-row__rename-wrapper",
            onkeydown: move |evt: KeyboardEvent| {
                match evt.key() {
                    Key::Enter => {
                        evt.prevent_default();
                        let raw = value.read().clone();
                        error.set(run_commit(&raw, target_for_kb.as_ref(), state).err());
                    }
                    Key::Escape => {
                        evt.prevent_default();
                        error.set(run_inline(&value.peek(), target_for_cancel.as_ref(), state, true).err());
                    }
                    _ => {}
                }
            },
            onfocusout: move |_evt: FocusEvent| {
                let raw = value.read().clone();
                error.set(run_commit(&raw, target.as_ref(), state).err());
            },
            if let Some(message) = error.read().as_ref() {
                span { role: "alert", "{message}" }
            }
            TextInput {
                value: ReadSignal::from(value),
                size: InputSize::Sm,
                class: Some("if-row-rename".to_owned()),
                onmounted: move |evt: MountedEvent| {
                    spawn(async move {
                        let _ = evt.data().set_focus(true).await;
                    });
                },
                oninput: move |evt: FormEvent| {
                    value.set(evt.value());
                },
            }
        }
    }
}
