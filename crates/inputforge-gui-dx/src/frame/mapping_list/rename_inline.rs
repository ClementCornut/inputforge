//! Inline rename for an existing mapping row. Mirrors F7's
//! `mode_tabs::rename_inline::RenameInline` — Enter dispatches
//! `SetMapping` with the same actions + new name; Esc reverts; blur
//! with empty value reverts; blur with non-empty value commits.
//!
//! `TextInput` only forwards `oninput` / `onmounted` / `class` to its
//! inner `<input>` (see `components::text_input`). To attach `onkeydown`
//! and `onfocusout`, this component wraps the input in a `<div>` and
//! relies on event bubbling — same shape F7's `mode_tabs::rename_inline`
//! uses.

use std::sync::mpsc::Sender;

use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::engine::EngineCommand;
use inputforge_core::types::InputAddress;

use crate::components::{InputSize, TextInput};
use crate::context::{AppContext, MappingSummary};

/// Shared commit path. Each event handler clones its `Sender` and calls
/// this directly; that keeps the per-handler closures `Fn` (mirroring
/// F7's `mode_tabs::rename_inline::run_commit`).
fn run_commit(
    raw: &str,
    summary: &MappingSummary,
    commands: &Sender<EngineCommand>,
    actions: Vec<Action>,
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
        // No-op rename — no need to round-trip through the engine.
        state.set(None);
        return;
    }
    let _ = commands.send(EngineCommand::SetMapping {
        input: summary.input.clone(),
        mode: summary.mode.clone(),
        name: Some(new_name),
        actions,
    });
    tracing::info!(
        target: "f8::mapping_list",
        action = "rename",
        ?summary.input,
        mode = %summary.mode,
        "dispatch SetMapping (rename)",
    );
    state.set(None);
}

/// Read the current actions for `(input, mode)` from the active profile,
/// returning `Vec::new()` if the mapping (or profile) is not present.
/// `SetMapping` on an empty actions list removes the mapping, so callers
/// must only invoke this when a mapping is known to exist.
fn read_actions(ctx: &AppContext, summary: &MappingSummary) -> Vec<Action> {
    ctx.state
        .read()
        .active_profile
        .as_ref()
        .and_then(|p| {
            p.find_mapping(&summary.input, &summary.mode)
                .map(|m| m.actions.clone())
        })
        .unwrap_or_default()
}

/// Parent-driven inline rename editor for a mapping row. The parent owns
/// `state: Signal<Option<InputAddress>>` — when this matches
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
    let ctx = use_context::<AppContext>();

    let initial = summary.name.clone().unwrap_or_default();
    let mut value: Signal<String> = use_signal(|| initial);

    // Per-handler clones — `Sender` is `!Copy` but trivially clonable;
    // cloning upfront lets each event closure own its sender and stay
    // `Fn` (no `FnMut` move-out errors when the same pair of closures
    // is referenced from both `onkeydown` and `onfocusout`).
    let summary_for_kb = summary.clone();
    let summary_for_blur = summary.clone();
    let cmd_for_kb = ctx.commands.clone();
    let cmd_for_blur = ctx.commands.clone();
    let ctx_for_kb = ctx.clone();
    let ctx_for_blur = ctx.clone();

    rsx! {
        div {
            class: "if-row__rename-wrapper",
            onkeydown: move |evt: KeyboardEvent| {
                match evt.key() {
                    Key::Enter => {
                        evt.prevent_default();
                        let raw = value.read().clone();
                        let actions = read_actions(&ctx_for_kb, &summary_for_kb);
                        run_commit(&raw, &summary_for_kb, &cmd_for_kb, actions, state);
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
                let actions = read_actions(&ctx_for_blur, &summary_for_blur);
                run_commit(&raw, &summary_for_blur, &cmd_for_blur, actions, state);
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
