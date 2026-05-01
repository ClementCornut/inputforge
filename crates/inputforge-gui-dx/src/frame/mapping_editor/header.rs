// Rust guideline compliant 2026-05-01

//! Editor header: identity row (h2 title with inline rename) plus subtitle
//! row (source label, rebind button, optional output label).
//!
//! Per DESIGN.md §6 "Inline-First Rule": the property is edited where it
//! is displayed. The h2 is both the display surface and (via F2 / right
//! click) the entry point to the inline rename editor. The subtitle row
//! anchors the rebind affordance next to the source label so that source
//! and its action live as one unit.
//!
//! Display state: bare `<h2>` with `tabindex="0"` and `data-editor-focus`
//! so the F8 keyboard nav lands here. `Key::F2` and right-click both arm
//! the rename editor. Long names truncate via CSS `text-overflow: ellipsis`;
//! the user reads the full name by entering rename mode (the inline editor
//! scrolls horizontally).
//!
//! Edit state: `<input type="text">` styled to inherit h2 typography
//! (20px / 600 / Inter, line-height 28px) so the swap is visually
//! invisible. Enter / blur commit, Esc reverts. Empty / whitespace-only
//! inputs revert without dispatch. The undo entry is pushed only when
//! dispatch succeeds, mirroring the F2 commit-on-blur model.

use dioxus::prelude::*;

use inputforge_core::action::{Action, Mapping};
use inputforge_core::engine::EngineCommand;
use inputforge_core::types::{InputAddress, OutputAddress, OutputId, VJoyAxis};

use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::undo_log::{LabelArgs, UndoKind, format_undo_label};
use crate::frame::mapping_list::source_label;
use crate::frame::view_state::ViewState;
use crate::patterns::live_capture::{CaptureFilter, LiveCapture};

#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures. Mirrors the suppression \
              used in mapping_list/row.rs and mode_tabs/rename_inline.rs."
)]
pub(crate) fn Header(
    /// Current saved mapping name (read-only mirror; edit working copy is local).
    name: String,
    /// Mapping's input address; drives the source-label readout.
    input: InputAddress,
    /// `(mode, InputAddress)` key for rename + rebind dispatch.
    /// Named `mapping_key` to avoid Dioxus's reserved `key` prop.
    mapping_key: MappingKey,
    /// Full action list for the mapping; needed to reconstruct `SetMapping`.
    actions: Vec<Action>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();
    let capture = use_context::<LiveCapture>();
    let view = use_context::<ViewState>();

    // Local UI state.
    let mut armed: Signal<bool> = use_signal(|| false);
    let mut local_name: Signal<String> = use_signal(|| name.clone());
    // Disambiguates "we armed this capture" from another consumer's capture
    // (e.g. MergeAxis secondary picker). Without it the captured-signal effect
    // would fire for every consumer's capture.
    let mut is_armed_consumer: Signal<bool> = use_signal(|| false);

    // Cancel any in-flight rebind capture when the user switches mappings.
    //
    // Subscribe to `view.selected_mapping` (the change source) so the effect
    // re-fires when the user picks a different mapping in the rail. Use
    // `peek()` for `is_armed_consumer` so arming the flag in `on_rebind`
    // does NOT trigger this effect to cancel itself.
    let selected_for_cancel = view.selected_mapping;
    use_effect(move || {
        let _selected = selected_for_cancel.read();
        if *is_armed_consumer.peek() {
            capture.cancel.call(());
            is_armed_consumer.set(false);
        }
    });

    // Rebind capture handler: on a fresh `captured` value while we are the
    // armed consumer, dispatch `SetMapping` then push a `Rebind` undo entry.
    //
    // Subscribe to `capture.captured` (the change source). Read
    // `is_armed_consumer` via `peek()` so the dispatch path's
    // `is_armed_consumer.set(false)` cleanup does not retrigger the effect.
    let mapping_key_for_capture = mapping_key.clone();
    let actions_for_capture = actions.clone();
    let name_for_capture = Some(name.clone());
    let cmd_tx_for_capture = ctx.commands.clone();
    let mut undo_log_for_capture = editor.undo_log;
    let cfg_for_capture = ctx.config;

    use_effect(move || {
        let captured_addr = capture.captured.read().clone();
        if !*is_armed_consumer.peek() {
            return;
        }
        let Some(new_addr) = captured_addr else {
            return;
        };

        let (mode, old_addr) = mapping_key_for_capture.clone();

        // No-op rebind: the user pressed the input that is already mapped
        // here. Skip dispatch + undo, mirroring the rename path's
        // same-name skip. Clearing `captured` and the consumer flag keeps
        // the listening UI from sticking around.
        if new_addr == old_addr {
            is_armed_consumer.set(false);
            let mut cap = capture.captured;
            cap.set(None);
            return;
        }

        // Resolve labels before dispatch so the undo entry survives the
        // config snapshot rebuild that follows.
        let old_label = source_label::format(&old_addr, &cfg_for_capture.read());
        let new_label = source_label::format(&new_addr, &cfg_for_capture.read());

        let before = Mapping {
            input: old_addr.clone(),
            mode: mode.clone(),
            name: name_for_capture.clone(),
            actions: actions_for_capture.clone(),
        };

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
        // Undo entry keys on the OLD address: that is the mapping that
        // existed before this rebind; restoring it writes the old snapshot
        // back under the old key.
        undo_log_for_capture
            .write()
            .push_edit((mode, old_addr), before, UndoKind::Rebind, label);

        tracing::info!(
            target: "f9::mapping_editor",
            action = "rebind",
            "mapping rebound"
        );

        is_armed_consumer.set(false);
        let mut cap = capture.captured;
        cap.set(None);
    });

    // Read source label and (optional) output label for the subtitle.
    let cfg = ctx.config.read();
    let src = source_label::format(&input, &cfg);
    let output_label = cfg
        .selected_mapping_actions
        .as_ref()
        .and_then(|a| first_map_to_vjoy_label(a));
    drop(cfg);

    // ----- Display-mode handlers (h2) -----

    let name_for_arm_kb = name.clone();
    let on_h2_keydown = move |evt: KeyboardEvent| {
        if matches!(evt.key(), Key::F2) {
            evt.prevent_default();
            local_name.set(name_for_arm_kb.clone());
            armed.set(true);
        }
    };

    let name_for_arm_ctx = name.clone();
    let on_h2_contextmenu = move |evt: MouseEvent| {
        evt.prevent_default();
        local_name.set(name_for_arm_ctx.clone());
        armed.set(true);
    };

    // ----- Edit-mode handlers (input) -----

    let mapping_key_for_blur = mapping_key.clone();
    let actions_for_blur = actions.clone();
    let initial_for_blur = name.clone();
    let cmd_tx_for_blur = ctx.commands.clone();
    let mut undo_log_for_blur = editor.undo_log;

    let on_input_blur = move |_| {
        let new = local_name.peek().trim().to_owned();
        if new.is_empty() || new == initial_for_blur {
            // Empty (or whitespace-only) revert; same-name no-op skip.
            local_name.set(initial_for_blur.clone());
            armed.set(false);
            return;
        }
        let before = Mapping {
            input: mapping_key_for_blur.1.clone(),
            mode: mapping_key_for_blur.0.clone(),
            name: Some(initial_for_blur.clone()),
            actions: actions_for_blur.clone(),
        };
        // Dispatch FIRST. When the engine is offline (channel disconnected)
        // do NOT push an undo entry: otherwise the user accumulates phantom
        // entries that Ctrl+Z would later dispatch into a dead channel.
        if cmd_tx_for_blur
            .send(EngineCommand::SetMapping {
                input: mapping_key_for_blur.1.clone(),
                mode: mapping_key_for_blur.0.clone(),
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
            armed.set(false);
            return;
        }
        let label = format_undo_label(
            UndoKind::Rename,
            LabelArgs {
                old_new: Some((&initial_for_blur, &new)),
                ..LabelArgs::default()
            },
        );
        undo_log_for_blur.write().push_edit(
            mapping_key_for_blur.clone(),
            before,
            UndoKind::Rename,
            label,
        );
        tracing::info!(
            target: "f9::mapping_editor",
            action = "rename",
            new_name = %new,
            "mapping renamed"
        );
        armed.set(false);
    };

    let initial_for_esc = name.clone();
    let on_input_keydown = move |evt: KeyboardEvent| match evt.key() {
        Key::Enter => {
            evt.prevent_default();
            // Trigger blur on the active input to commit via the canonical
            // path (`onblur` handler). `document::eval` is fire and forget;
            // `let _` suppresses the unused-result lint without blocking.
            let _ = document::eval(
                r"
                const el = document.activeElement;
                if (el && el instanceof HTMLInputElement) { el.blur(); }
                ",
            );
        }
        Key::Escape => {
            evt.prevent_default();
            local_name.set(initial_for_esc.clone());
            armed.set(false);
        }
        _ => {}
    };

    let on_input_input = move |evt: FormEvent| {
        local_name.set(evt.value());
    };

    // External-cancel watcher: when `cap.active` flips false while we were
    // armed and nothing was captured, reset `is_armed_consumer` so the
    // listening UI clears. Triggered by F8's document-level Esc listener
    // or by another consumer claiming capture. Mirrors the equivalent
    // watcher in `mapping_list/add_inline.rs`.
    use_effect(move || {
        if *capture.active.read() {
            return;
        }
        if !*is_armed_consumer.peek() {
            return;
        }
        if capture.captured.peek().is_some() {
            return;
        }
        is_armed_consumer.set(false);
    });

    // Rebind button: arm `LiveCapture::Any`. Set the consumer flag BEFORE
    // calling start so the captured-signal effect cannot fire before the
    // flag is true (effects run synchronously in SSR and on the next
    // microtask tick in the browser).
    let on_rebind = move |_: MouseEvent| {
        is_armed_consumer.set(true);
        capture.start.call(CaptureFilter::Any);
    };

    // Cancel button (visible only while armed): drop the capture and clear
    // our consumer flag. Esc is handled by F8's own listener; this is the
    // mouse path.
    let on_cancel_rebind = move |_: MouseEvent| {
        capture.cancel.call(());
        is_armed_consumer.set(false);
    };

    rsx! {
        div { class: "if-editor__header",
            if *armed.read() {
                input {
                    r#type: "text",
                    class: "if-editor__title-input",
                    "aria-label": "Edit mapping name",
                    "data-editor-focus": "true",
                    value: "{local_name}",
                    oninput: on_input_input,
                    onkeydown: on_input_keydown,
                    onblur: on_input_blur,
                    onmounted: move |evt: MountedEvent| {
                        spawn(async move {
                            let _ = evt.data().set_focus(true).await;
                            // Select all so the user can type-to-replace.
                            // Fire and forget; tests run in SSR where eval is a no-op.
                            let _ = document::eval(
                                r"
                                const el = document.activeElement;
                                if (el && typeof el.select === 'function') { el.select(); }
                                ",
                            );
                        });
                    },
                }
            } else {
                h2 {
                    class: "if-editor__title",
                    tabindex: "0",
                    "data-editor-focus": "true",
                    "aria-label": "Mapping name. Press F2 or right-click to rename.",
                    onkeydown: on_h2_keydown,
                    oncontextmenu: on_h2_contextmenu,
                    "{name}"
                }
            }
            div { class: "if-editor__subtitle",
                if *is_armed_consumer.read() {
                    div { class: "if-rebind-composite if-rebind-composite--listening",
                        span {
                            class: "if-rebind-composite__listening",
                            role: "status",
                            "aria-live": "polite",
                            "Press an input\u{2026}"
                        }
                        button {
                            class: "if-rebind-composite__action",
                            r#type: "button",
                            onclick: on_cancel_rebind,
                            "Cancel"
                        }
                    }
                } else {
                    div { class: "if-rebind-composite",
                        span { class: "if-rebind-composite__label", "{src}" }
                        button {
                            class: "if-rebind-composite__action",
                            r#type: "button",
                            onclick: on_rebind,
                            "rebind"
                        }
                    }
                }
                if let Some(out) = output_label {
                    span { class: "if-editor__subtitle-arrow",
                        "\u{00a0}\u{00a0}\u{2192}\u{00a0}\u{00a0}"
                    }
                    span { class: "if-editor__subtitle-output", "{out}" }
                }
            }
        }
    }
}

/// Walk the action tree (DFS pre-order, including `Conditional` branches)
/// and return the formatted label for the first `MapToVJoy` found.
fn first_map_to_vjoy_label(actions: &[Action]) -> Option<String> {
    fn walk(actions: &[Action]) -> Option<&OutputAddress> {
        for action in actions {
            match action {
                Action::MapToVJoy { output } => return Some(output),
                Action::Conditional {
                    if_true, if_false, ..
                } => {
                    if let Some(found) = walk(if_true) {
                        return Some(found);
                    }
                    if let Some(found) = walk(if_false) {
                        return Some(found);
                    }
                }
                _ => {}
            }
        }
        None
    }
    walk(actions).map(format_output_label)
}

fn format_output_label(output: &OutputAddress) -> String {
    let suffix = match output.output {
        OutputId::Axis { id } => match id {
            VJoyAxis::X => "X axis",
            VJoyAxis::Y => "Y axis",
            VJoyAxis::Z => "Z axis",
            VJoyAxis::Rx => "Rx axis",
            VJoyAxis::Ry => "Ry axis",
            VJoyAxis::Rz => "Rz axis",
            VJoyAxis::Slider0 => "Slider 0",
            VJoyAxis::Slider1 => "Slider 1",
        }
        .to_owned(),
        OutputId::Button { id } => format!("Button {id}"),
        OutputId::Hat { id } => format!("Hat {id}"),
    };
    format!("vJoy {} \u{00b7} {}", output.device, suffix)
}
