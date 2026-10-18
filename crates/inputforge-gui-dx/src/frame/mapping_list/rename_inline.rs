//! Inline rename for an existing mapping row. Mirrors F7's
//! `mode_tabs::rename_inline::RenameInline`, Enter dispatches
//! `SetMapping` with the same actions + new name; Esc reverts; blur
//! with empty value reverts; blur with non-empty value commits.
//!
//! `TextInput` only forwards `oninput` / `onmounted` / `class` to its
//! inner `<input>` (see `components::TextInput`). To attach `onkeydown`
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

/// Exercise the same app-owned commit callback from isolated unit fixtures.
#[cfg(test)]
pub(super) fn run_commit(
    raw: &str,
    target: Option<&Target>,
    state: Signal<Option<InputAddress>>,
) -> Result<(), String> {
    let publication = crate::mapping_edits::ui::Rename {
        value: Signal::new(raw.to_owned()),
        error: Signal::new(None),
        open: state,
        target: target.cloned(),
    };
    consume_context::<crate::mapping_edits::Bindings>()
        .ui
        .rename
        .call((publication, crate::mapping_edits::ui::RenameEvent::Commit))
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
    let value: Signal<String> = use_signal(|| initial);

    let error = use_signal(|| None::<String>);
    let target = use_hook(|| capture(&summary));
    let publication = crate::mapping_edits::ui::Rename {
        value,
        error,
        open: state,
        target,
    };
    let ui = use_context::<crate::mapping_edits::Bindings>().ui;
    let key_publication = publication.clone();
    let blur_publication = publication.clone();

    rsx! {
        div {
            class: "if-row__rename-wrapper",
            onkeydown:move |evt| {let _=ui.rename.call((key_publication.clone(),crate::mapping_edits::ui::RenameEvent::Key(evt)));},
            onfocusout:move |_| {let _=ui.rename.call((blur_publication.clone(),crate::mapping_edits::ui::RenameEvent::Commit));},
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
                    let _=ui.rename.call((publication.clone(),crate::mapping_edits::ui::RenameEvent::Input(evt.value())));
                },
            }
        }
    }
}
