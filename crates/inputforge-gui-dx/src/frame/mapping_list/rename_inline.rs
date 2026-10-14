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

/// Submit prepared row metadata; the app obtains and preserves current actions.
pub(super) fn run_commit(
    raw: &str,
    summary: &MappingSummary,
    mut state: Signal<Option<InputAddress>>,
) {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        // Blur (or Enter) on an empty buffer reverts without dispatch.
        state.set(None);
        return;
    }
    let new_name = trimmed.to_owned();
    if Some(&new_name) == summary.name.as_ref() {
        // No-op rename, no need to round-trip through the engine.
        state.set(None);
        return;
    }
    let ctx = consume_context::<crate::context::AppContext>();
    let delivered = consume_context::<crate::mapping_edits::Bindings>()
        .dispatch
        .call(crate::mapping_edits::Request {
            profile: ctx.meta.peek().profile_id.clone(),
            before: inputforge_core::action::Mapping {
                input: summary.input.clone(),
                mode: summary.mode.clone(),
                name: summary.name.clone(),
                actions: vec![],
            },
            edit: crate::mapping_edits::Edit::Rename(new_name.clone()),
            label: format!(
                "rename: '{}' -> '{new_name}'",
                summary.name.as_deref().unwrap_or("")
            ),
            group: None,
            editor: false,
        });
    if delivered {
        tracing::info!(target: "f8::mapping_list", action = "rename", ?summary.input, mode = %summary.mode, "dispatched mapping rename");
    }
    state.set(None);
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

    let initial = summary.name.clone().unwrap_or_default();
    let mut value: Signal<String> = use_signal(|| initial);

    let summary_for_kb = summary.clone();
    let summary_for_blur = summary.clone();

    rsx! {
        div {
            class: "if-row__rename-wrapper",
            onkeydown: move |evt: KeyboardEvent| {
                match evt.key() {
                    Key::Enter => {
                        evt.prevent_default();
                        let raw = value.read().clone();
                        run_commit(&raw, &summary_for_kb, state);
                    }
                    Key::Escape => {
                        evt.prevent_default();
                        let mut state = state;
                        state.set(None);
                    }
                    _ => {}
                }
            },
            onfocusout: move |_evt: FocusEvent| {
                let raw = value.read().clone();
                run_commit(&raw, &summary_for_blur, state);
            },
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
