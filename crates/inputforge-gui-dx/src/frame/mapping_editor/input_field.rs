// Rust guideline compliant 2026-05-01

//! Read-only source-address label plus a ghost "rebind" button that arms
//! `LiveCapture`. When the capture fires the component dispatches
//! `SetMapping` with the new `InputAddress` and pushes a `Rebind` undo entry.

use dioxus::prelude::*;

use inputforge_core::action::{Action, Mapping};
use inputforge_core::engine::EngineCommand;

use crate::components::{Button, ButtonSize, ButtonVariant};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::undo_log::{LabelArgs, UndoKind, format_undo_label};
use crate::frame::mapping_list::source_label;
use crate::patterns::live_capture::{CaptureFilter, LiveCapture};

/// Read-only source label with a ghost "rebind" button.
///
/// The rebind flow:
/// 1. User clicks "rebind": arms `LiveCapture` and sets the local
///    `is_armed_consumer` flag so only *this* component's effect reacts.
/// 2. `LiveCapture.captured` fires: component dispatches `SetMapping` with
///    the new `InputAddress`, then pushes a `Rebind` undo entry.
/// 3. If the user switches the selected mapping while armed, the capture is
///    cancelled via a `use_effect` that watches `mapping_key`.
#[component]
pub(crate) fn InputField(
    /// `(mode, InputAddress)` key for the mapping being edited.
    /// Named `mapping_key` to avoid collision with Dioxus's reserved `key` prop.
    mapping_key: MappingKey,
    /// Full action list for the mapping (needed to reconstruct `SetMapping`).
    actions: Vec<Action>,
    /// Current saved display name (forwarded into `SetMapping` and undo).
    name: Option<String>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();
    let capture = use_context::<LiveCapture>();

    // Local flag that disambiguates "we armed this capture" from "another
    // consumer (e.g. MergeAxis secondary picker) armed it". Without this flag
    // the captured-signal effect would fire for any consumer's capture, and
    // would self-fire again after the component clears `captured`.
    let mut is_armed_consumer: Signal<bool> = use_signal(|| false);

    let mapping_key_for_cancel = mapping_key.clone();
    // Cancel any in-flight capture when the user switches to a different mapping.
    use_effect(move || {
        // Track the key as a reactive dependency so the effect re-fires on change.
        let _key = mapping_key_for_cancel.clone();
        if *is_armed_consumer.read() {
            capture.cancel.call(());
            is_armed_consumer.set(false);
        }
    });

    // Watch `capture.captured`: when *we* armed it and a new address arrives,
    // dispatch `SetMapping` first; push undo only on successful dispatch.
    let mapping_key_for_capture = mapping_key.clone();
    let actions_for_capture = actions.clone();
    let name_for_capture = name.clone();
    let cmd_tx_for_capture = ctx.commands.clone();
    let mut undo_log = editor.undo_log;
    let cfg_for_capture = ctx.config;

    use_effect(move || {
        let captured_addr = capture.captured.read().clone();

        // Only act when we armed this specific capture.
        if !*is_armed_consumer.read() {
            return;
        }
        let Some(new_addr) = captured_addr else {
            return;
        };

        let (mode, old_addr) = mapping_key_for_capture.clone();

        // Build labels for the undo entry (before dispatch so we can read the
        // old snapshot; the config signal may update after dispatch arrives).
        let old_label = source_label::format(&old_addr, &cfg_for_capture.read());
        let new_label = source_label::format(&new_addr, &cfg_for_capture.read());

        // Snapshot the mapping-before state for the undo entry.
        let before = Mapping {
            input: old_addr.clone(),
            mode: mode.clone(),
            name: name_for_capture.clone(),
            actions: actions_for_capture.clone(),
        };

        // Dispatch FIRST. Undo entries are only pushed on success so phantom
        // entries do not accumulate while the engine is offline.
        if cmd_tx_for_capture
            .send(EngineCommand::SetMapping {
                input: new_addr.clone(),
                mode: mode.clone(),
                name: name_for_capture.clone(),
                actions: actions_for_capture.clone(),
            })
            .is_err()
        {
            tracing::warn!(
                target: "f9::mapping_editor",
                action = "rebind_drop_offline",
                "rebind dropped: engine channel disconnected"
            );
            // Disarm and clear so we don't re-trigger.
            is_armed_consumer.set(false);
            capture.cancel.call(());
            return;
        }

        let label = format_undo_label(
            UndoKind::Rebind,
            LabelArgs {
                old_new: Some((&old_label, &new_label)),
                ..LabelArgs::default()
            },
        );
        // The undo entry key uses the OLD address because that is the mapping
        // that existed before this rebind; restoring it means writing the old
        // snapshot back under the old key.
        undo_log
            .write()
            .push_edit((mode, old_addr), before, UndoKind::Rebind, label);

        tracing::info!(
            target: "f9::mapping_editor",
            action = "rebind",
            "mapping rebound"
        );

        // Disarm and clear the captured signal so stale effects don't re-fire.
        is_armed_consumer.set(false);
        let mut cap = capture.captured;
        cap.set(None);
    });

    // Read the current source label from the config snapshot.
    let (_, input_addr) = mapping_key.clone();
    let source = source_label::format(&input_addr, &ctx.config.read());

    let on_rebind = move |_: MouseEvent| {
        // Set the consumer flag *before* calling start so the captured effect
        // cannot fire before the flag is true (effects run synchronously in SSR
        // and on the next microtask tick in the browser).
        is_armed_consumer.set(true);
        capture.start.call(CaptureFilter::Any);
    };

    rsx! {
        div { class: "if-editor__input-field",
            div { class: "if-editor__input-label", "{source}" }
            Button {
                variant: ButtonVariant::Ghost,
                size: ButtonSize::Sm,
                onclick: on_rebind,
                "rebind"
            }
        }
    }
}
