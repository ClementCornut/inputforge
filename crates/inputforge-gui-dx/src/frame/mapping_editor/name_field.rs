// Rust guideline compliant 2026-05-01

//! Name field with commit-on-blur dispatch.
//!
//! Renders a text `<input>` that:
//! - Maintains a local working copy while the user types (`oninput`).
//! - Commits via `SetMapping` on blur or Enter key, but only when the
//!   trimmed value differs from the saved name and is non-empty.
//! - Pushes a `Rename` undo entry **only** when the dispatch succeeds.
//!   Phantom undo entries when the engine is offline are prevented.

use dioxus::prelude::*;

use inputforge_core::action::{Action, Mapping};
use inputforge_core::engine::EngineCommand;

use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::undo_log::{LabelArgs, UndoKind, format_undo_label};

/// Editable mapping-name field.
///
/// Keeps a local `Signal<String>` as the working copy; commits to the engine
/// on blur or Enter. Undo entries are pushed only when dispatch succeeds.
#[component]
pub(crate) fn NameField(
    /// Current saved name (read-only mirror; the working copy is local).
    initial: String,
    /// `(mode, InputAddress)` key for the mapping being edited.
    /// Named `mapping_key` to avoid collision with Dioxus's reserved `key` prop.
    mapping_key: MappingKey,
    /// Full action list for the mapping (needed to reconstruct `SetMapping`).
    actions: Vec<Action>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();

    let mut local: Signal<String> = use_signal(|| initial.clone());

    let key_for_blur = mapping_key.clone();
    let actions_for_blur = actions.clone();
    let initial_for_blur = initial.clone();
    let cmd_tx = ctx.commands.clone();
    let mut undo_log = editor.undo_log;

    let onblur = move |_| {
        let new = local.peek().trim().to_owned();
        if new == initial_for_blur || new.is_empty() {
            // No change or user cleared the field: skip dispatch.
            return;
        }
        // Capture a before-snapshot for the undo entry.
        let before = Mapping {
            input: key_for_blur.1.clone(),
            mode: key_for_blur.0.clone(),
            name: Some(initial_for_blur.clone()),
            actions: actions_for_blur.clone(),
        };
        // Dispatch FIRST. When the engine is offline (channel disconnected)
        // do NOT push an undo entry: otherwise the user accumulates phantom
        // entries that Ctrl+Z would later dispatch into a dead channel.
        // The engine-offline banner (Task 13) already informs the user.
        if cmd_tx
            .send(EngineCommand::SetMapping {
                input: key_for_blur.1.clone(),
                mode: key_for_blur.0.clone(),
                name: Some(new.clone()),
                actions: actions_for_blur.clone(),
            })
            .is_err()
        {
            tracing::warn!(
                target: "f9::mapping_editor",
                action = "rename_drop_offline",
                new_name = %new,
                "rename dropped: engine channel disconnected"
            );
            return;
        }
        let label = format_undo_label(
            UndoKind::Rename,
            LabelArgs {
                old_new: Some((&initial_for_blur, &new)),
                ..LabelArgs::default()
            },
        );
        undo_log
            .write()
            .push_edit(key_for_blur.clone(), before, UndoKind::Rename, label);
        tracing::info!(
            target: "f9::mapping_editor",
            action = "rename",
            new_name = %new,
            "mapping renamed"
        );
    };

    let onkeydown = move |evt: KeyboardEvent| {
        if evt.key() == Key::Enter {
            evt.prevent_default();
            // Blur the active input to trigger the canonical commit path.
            // `document::eval` is fire-and-forget; the `let _` suppresses the
            // unused-result lint without blocking.
            let _ = document::eval(
                r"
                const el = document.activeElement;
                if (el && el instanceof HTMLInputElement) { el.blur(); }
                ",
            );
        }
    };

    let oninput = move |evt: FormEvent| {
        local.set(evt.value());
    };

    rsx! {
        div { class: "if-editor__name-field",
            input {
                r#type: "text",
                class: "if-editor__name-input",
                value: "{local}",
                oninput,
                onblur,
                onkeydown,
                // `data-editor-focus` is read by the F8 keyboard nav to focus
                // the editor's first interactive element (mapping_list/mod.rs
                // Intent::FocusEditor handler uses
                // `querySelector('[data-editor-focus]')`).
                "data-editor-focus": "true",
            }
        }
    }
}
