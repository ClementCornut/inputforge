// Rust guideline compliant 2026-05-01

//! Editor header: identity row (h2 title with inline rename) plus subtitle
//! row (source label and rebind button).
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
use inputforge_core::types::InputAddress;

use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::undo_log::{LabelArgs, UndoKind, format_undo_label};
use crate::frame::mapping_list::source_label;
use crate::frame::view_state::ViewState;
use crate::patterns::live_capture::{
    CAPTURE_PROMPT, CaptureFilter, LiveCapture, is_current_capture_session, rebind_composite_class,
};

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
    // Disambiguates "we armed this capture" from another consumer's capture.
    // Stores the LiveCapture session this header owns; a newer session means
    // another surface superseded us while `capture.active` can remain true.
    let mut armed_session: Signal<Option<u64>> = use_signal(|| None);

    // Cancel any in-flight rebind capture when the user switches mappings.
    //
    // Subscribe to `view.selected_mapping` (the change source) so the effect
    // re-fires when the user picks a different mapping in the rail. Use
    // `peek()` for `armed_session` so arming in `on_rebind`
    // does NOT trigger this effect to cancel itself.
    let selected_for_cancel = view.selected_mapping;
    use_effect(move || {
        let _selected = selected_for_cancel.read();
        if armed_session.peek().is_some() {
            capture.cancel.call(());
            armed_session.set(None);
        }
    });

    // Rebind capture handler: on a fresh `captured` value while we are the
    // armed consumer, dispatch `SetMapping` then push a `Rebind` undo entry.
    //
    // Subscribe to `capture.captured` (the change source). Read
    // `armed_session` via `peek()` so the dispatch path's cleanup does not
    // retrigger the effect.
    let mapping_key_for_capture = mapping_key.clone();
    let actions_for_capture = actions.clone();
    let name_for_capture = Some(name.clone());
    let cmd_tx_for_capture = ctx.commands.clone();
    let mut undo_log_for_capture = editor.undo_log;
    let cfg_for_capture = ctx.config;

    use_effect(move || {
        let captured_addr = capture.captured.read().clone();
        if !is_current_capture_session(*armed_session.peek(), *capture.session.peek()) {
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
            armed_session.set(None);
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
            armed_session.set(None);
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

        armed_session.set(None);
        let mut cap = capture.captured;
        cap.set(None);
    });

    // Read source label for the subtitle.
    let cfg = ctx.config.read();
    let src = source_label::format(&input, &cfg);
    drop(cfg);

    // Compose the rebind-composite class so the placeholder label renders
    // muted/italic when the primary input is `Unbound`. Mapping primaries
    // are operationally rare to be `Unbound` (only via hand-edited profile
    // or a legacy migration walker), but when it does happen we want the
    // header to read consistently with the predicate / merge-axis call
    // sites of `if-rebind-composite`. Mirrors the same pattern used in
    // `PredicateInputRow`: applied to both the idle and listening branches
    // so the class doesn't flicker if the user opens, then cancels, a
    // rebind on an Unbound row.
    let composite_class = rebind_composite_class(&input, false);
    let listening_class = rebind_composite_class(&input, true);

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

    // External-cancel / supersede watcher. `active=false` handles Esc/cancel;
    // `session` mismatch handles another capture surface starting while the
    // global capture remains active.
    use_effect(move || {
        let active_now = *capture.active.read();
        let current_session = *capture.session.read();
        let owned_session = *armed_session.peek();
        if owned_session.is_none() {
            return;
        }
        if capture.captured.peek().is_some() {
            return;
        }
        if active_now && is_current_capture_session(owned_session, current_session) {
            return;
        }
        armed_session.set(None);
    });

    // Rebind button: arm `LiveCapture::Any`. Set the consumer flag BEFORE
    // calling start so the captured-signal effect cannot fire before the
    // flag is true (effects run synchronously in SSR and on the next
    // microtask tick in the browser).
    let on_rebind = move |_: MouseEvent| {
        capture.start.call(CaptureFilter::Any);
        armed_session.set(Some(*capture.session.peek()));
    };

    // Cancel button (visible only while armed): drop the capture and clear
    // our consumer flag. Esc is handled by F8's own listener; this is the
    // mouse path.
    let on_cancel_rebind = move |_: MouseEvent| {
        capture.cancel.call(());
        armed_session.set(None);
    };

    let is_rebind_listening =
        is_current_capture_session(*armed_session.read(), *capture.session.read());

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
                if is_rebind_listening {
                    div { class: "{listening_class}",
                        span {
                            class: "if-rebind-composite__listening",
                            role: "status",
                            "aria-live": "polite",
                            "{CAPTURE_PROMPT}"
                        }
                        button {
                            class: "if-rebind-composite__action",
                            r#type: "button",
                            onclick: on_cancel_rebind,
                            "Cancel"
                        }
                    }
                } else {
                    div { class: "{composite_class}",
                        span { class: "if-rebind-composite__label", "{src}" }
                        button {
                            class: "if-rebind-composite__action",
                            r#type: "button",
                            onclick: on_rebind,
                            "rebind"
                        }
                    }
                }
            }
        }
    }
}
