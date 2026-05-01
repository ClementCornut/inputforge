// Rust guideline compliant 2026-05-01

//! `MapToKeyboard` body: modifier toggles + key input field.
//!
//! # Controls
//!
//! Four modifier checkboxes (Ctrl, Alt, Shift, Win) and a free-text key
//! field allow the user to specify a full [`KeyCombo`]. The modifier
//! checkboxes dispatch on every `onchange` (each toggle is one click, so
//! one dispatch per click). The key field follows Task 15's commit-on-blur
//! pattern: `oninput` only updates the local `Signal`, and the dispatch
//! happens once on `onblur` (or when the user presses Enter, which
//! programmatically blurs the input). This avoids flooding the engine
//! channel and undo log with one entry per keystroke.
//!
//! # Live capture
//!
//! `CaptureFilter::KeysOnly` is not yet defined in the live-capture
//! primitive (it only ships `Any`, `AxesOnly`, and `ButtonsOnly`). Until
//! Task 16's consumer-flag pattern is extended with a `KeysOnly` variant,
//! this body ships the TextInput-only path. When `KeysOnly` lands, a
//! "Capture" button should be added here following the same consumer-flag
//! pattern used by the input-rebind field (Task 16).
//!
//! # Prop naming note
//!
//! Dioxus reserves the prop name `key` for the built-in reconciliation hint.
//! The keyboard combo is therefore exposed as the prop `combo` and
//! destructured that way in the function signature.
//!
//! # Malformed hints (Amendment 3)
//!
//! On every render the component writes to `editor.malformed_hints` when the
//! combo is invalid (empty key with modifiers, or entirely empty). When valid
//! it clears the stale hint for this `stage_id`.
//!
//! # Name preservation (Amendment 4)
//!
//! `EngineCommand::SetMapping` requires a `name` field. On every dispatch we
//! read the current name from `cfg.mapping_names` so that user-set names are
//! never silently cleared.
//!
//! # External-edit subscription (Amendment 6)
//!
//! A `use_effect` subscribes to `editor.external_edit_reset` so Dioxus
//! re-renders the local `Signal`s when Task 33's reconciliation token
//! advances. Local working copies are reset to the incoming prop when the
//! token changes (the prop itself is updated by the reconciler).

use dioxus::prelude::*;

use inputforge_core::action::{Action, Mapping};
use inputforge_core::engine::EngineCommand;
use inputforge_core::types::{KeyCombo, KeyModifier};

use crate::components::Checkbox;
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::replace_at_path;
use crate::frame::mapping_editor::undo_log::{LabelArgs, StageId, UndoKind, format_undo_label};

/// `MapToKeyboard` body: four modifier toggles and a free-text key field.
///
/// The prop is named `combo` rather than `key` because Dioxus reserves the
/// identifier `key` for its built-in reconciliation-hint attribute.
#[component]
pub(crate) fn MapToKeyboardBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    /// The keyboard combo to edit. Named `combo` (not `key`) because Dioxus
    /// reserves `key` as a built-in prop for its reconciliation hint.
    combo: KeyCombo,
    /// Full root-level action list for the mapping. Needed so that
    /// `replace_at_path` can build the new action tree on every edit.
    /// Named `root_actions` per Amendment 5 (the dispatcher uses this name).
    root_actions: Vec<Action>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();

    // Amendment 6: subscribe to external_edit_reset so that when Task 33's
    // reconciliation token advances, Dioxus re-renders this component and the
    // incoming `combo` prop (updated by the reconciler) refreshes local state.
    let reset_token = editor.external_edit_reset;
    use_effect(move || {
        let _ = *reset_token.read();
    });

    // Local working copies of each field so widgets are fully controlled.
    let mut local_key: Signal<String> = use_signal(|| combo.key.clone());
    let mut local_ctrl: Signal<bool> = use_signal(|| combo.modifiers.contains(&KeyModifier::Ctrl));
    let mut local_alt: Signal<bool> = use_signal(|| combo.modifiers.contains(&KeyModifier::Alt));
    let mut local_shift: Signal<bool> =
        use_signal(|| combo.modifiers.contains(&KeyModifier::Shift));
    let mut local_win: Signal<bool> = use_signal(|| combo.modifiers.contains(&KeyModifier::Win));

    // Amendment 3: malformed-hint write / clear on every render.
    {
        let k = local_key.read();
        let has_modifiers =
            *local_ctrl.read() || *local_alt.read() || *local_shift.read() || *local_win.read();
        let key_empty = k.trim().is_empty();
        // Both an empty field with modifiers (modifier-only) and an entirely
        // empty combo are invalid per spec lines 587-589.
        let mut malformed = editor.malformed_hints;
        if key_empty {
            let msg = if has_modifiers {
                "Key combo is modifier-only: add a base key".to_owned()
            } else {
                "Key combo is empty: enter a key".to_owned()
            };
            malformed.write().insert(stage_id.clone(), msg);
        } else {
            malformed.write().remove(&stage_id);
        }
    }

    // Snapshot of the mapping before any edit, used for undo entries.
    let cfg = ctx.config.read();
    // Amendment 4: read current name from cfg so we never clear user names.
    let current_name = cfg.mapping_names.get(&mapping_key.1).cloned();
    let before_mapping = Mapping {
        input: mapping_key.1.clone(),
        mode: mapping_key.0.clone(),
        name: current_name.clone(),
        actions: root_actions.clone(),
    };
    drop(cfg);

    // --- Ctrl toggle handler ---
    let mapping_key_ctrl = mapping_key.clone();
    let stage_id_ctrl = stage_id.clone();
    let root_actions_ctrl = root_actions.clone();
    let before_ctrl = before_mapping.clone();
    let current_name_ctrl = current_name.clone();
    let cmd_tx_ctrl = ctx.commands.clone();
    let mut undo_log_ctrl = editor.undo_log;

    let on_ctrl = move |_evt: FormEvent| {
        let new_ctrl = !*local_ctrl.peek();
        local_ctrl.set(new_ctrl);
        let new_combo = build_combo_from(
            local_key,
            new_ctrl,
            *local_alt.peek(),
            *local_shift.peek(),
            *local_win.peek(),
        );
        dispatch_keyboard(
            new_combo,
            "Ctrl modifier",
            &mapping_key_ctrl,
            &stage_id_ctrl,
            &root_actions_ctrl,
            &before_ctrl,
            current_name_ctrl.clone(),
            &cmd_tx_ctrl,
            &mut undo_log_ctrl,
        );
    };

    // --- Alt toggle handler ---
    let mapping_key_alt = mapping_key.clone();
    let stage_id_alt = stage_id.clone();
    let root_actions_alt = root_actions.clone();
    let before_alt = before_mapping.clone();
    let current_name_alt = current_name.clone();
    let cmd_tx_alt = ctx.commands.clone();
    let mut undo_log_alt = editor.undo_log;

    let on_alt = move |_evt: FormEvent| {
        let new_alt = !*local_alt.peek();
        local_alt.set(new_alt);
        let new_combo = build_combo_from(
            local_key,
            *local_ctrl.peek(),
            new_alt,
            *local_shift.peek(),
            *local_win.peek(),
        );
        dispatch_keyboard(
            new_combo,
            "Alt modifier",
            &mapping_key_alt,
            &stage_id_alt,
            &root_actions_alt,
            &before_alt,
            current_name_alt.clone(),
            &cmd_tx_alt,
            &mut undo_log_alt,
        );
    };

    // --- Shift toggle handler ---
    let mapping_key_shift = mapping_key.clone();
    let stage_id_shift = stage_id.clone();
    let root_actions_shift = root_actions.clone();
    let before_shift = before_mapping.clone();
    let current_name_shift = current_name.clone();
    let cmd_tx_shift = ctx.commands.clone();
    let mut undo_log_shift = editor.undo_log;

    let on_shift = move |_evt: FormEvent| {
        let new_shift = !*local_shift.peek();
        local_shift.set(new_shift);
        let new_combo = build_combo_from(
            local_key,
            *local_ctrl.peek(),
            *local_alt.peek(),
            new_shift,
            *local_win.peek(),
        );
        dispatch_keyboard(
            new_combo,
            "Shift modifier",
            &mapping_key_shift,
            &stage_id_shift,
            &root_actions_shift,
            &before_shift,
            current_name_shift.clone(),
            &cmd_tx_shift,
            &mut undo_log_shift,
        );
    };

    // --- Win toggle handler ---
    let mapping_key_win = mapping_key.clone();
    let stage_id_win = stage_id.clone();
    let root_actions_win = root_actions.clone();
    let before_win = before_mapping.clone();
    let current_name_win = current_name.clone();
    let cmd_tx_win = ctx.commands.clone();
    let mut undo_log_win = editor.undo_log;

    let on_win = move |_evt: FormEvent| {
        let new_win = !*local_win.peek();
        local_win.set(new_win);
        let new_combo = build_combo_from(
            local_key,
            *local_ctrl.peek(),
            *local_alt.peek(),
            *local_shift.peek(),
            new_win,
        );
        dispatch_keyboard(
            new_combo,
            "Win modifier",
            &mapping_key_win,
            &stage_id_win,
            &root_actions_win,
            &before_win,
            current_name_win.clone(),
            &cmd_tx_win,
            &mut undo_log_win,
        );
    };

    // --- Key text field handlers ---
    //
    // Per the F9 plan (lines 5340-5352) and Task 15's NameField pattern, the
    // key field commits on `onblur`, NOT on every `oninput` keystroke. The
    // `oninput` handler only updates the local working copy so the textbox
    // stays controlled; the actual `dispatch_keyboard` runs once when the
    // user moves focus away (or presses Enter, which programmatically blurs
    // the input via the same path NameField uses).
    let oninput = move |evt: FormEvent| {
        local_key.set(evt.value());
    };

    let mapping_key_blur = mapping_key.clone();
    let stage_id_blur = stage_id.clone();
    let root_actions_blur = root_actions.clone();
    let before_blur = before_mapping.clone();
    let current_name_blur = current_name.clone();
    let cmd_tx_blur = ctx.commands.clone();
    let mut undo_log_blur = editor.undo_log;
    // Remember the key value at mount so blur with no actual change is a no-op.
    let initial_key_blur = combo.key.clone();

    let onblur = move |_evt: FocusEvent| {
        let new_key_str = local_key.peek().trim().to_owned();
        // Skip the dispatch when the field is empty or unchanged. The
        // malformed-hint write at the top of the function still flags the
        // empty-key state on the next render so the user gets a visual cue.
        if new_key_str.is_empty() || new_key_str == initial_key_blur {
            return;
        }
        let new_combo = build_combo_from_key(
            new_key_str,
            *local_ctrl.peek(),
            *local_alt.peek(),
            *local_shift.peek(),
            *local_win.peek(),
        );
        dispatch_keyboard(
            new_combo,
            "key",
            &mapping_key_blur,
            &stage_id_blur,
            &root_actions_blur,
            &before_blur,
            current_name_blur.clone(),
            &cmd_tx_blur,
            &mut undo_log_blur,
        );
    };

    // Enter key behaves like blur: programmatically blur the active input so
    // the canonical commit path (`onblur`) runs exactly once. Mirrors
    // NameField's onkeydown handler (Task 15).
    let onkeydown = move |evt: KeyboardEvent| {
        if evt.key() == Key::Enter {
            evt.prevent_default();
            let _ = document::eval(
                r"
                const el = document.activeElement;
                if (el && el instanceof HTMLInputElement) { el.blur(); }
                ",
            );
        }
    };

    // `ReadSignal` conversions for the Checkbox `checked` prop.
    let ctrl_ro: ReadSignal<bool> = local_ctrl.into();
    let alt_ro: ReadSignal<bool> = local_alt.into();
    let shift_ro: ReadSignal<bool> = local_shift.into();
    let win_ro: ReadSignal<bool> = local_win.into();

    let invalid_field = local_key.read().trim().is_empty();
    let key_class = if invalid_field {
        "if-text-input if-text-input--md if-text-input--invalid"
    } else {
        "if-text-input if-text-input--md"
    };

    rsx! {
        div { class: "if-stage__body-keyboard",
            // Modifier toggles row
            div { class: "if-stage__body-field if-stage__body-field--modifiers",
                label { class: "if-stage__body-label", "Modifiers" }
                div { class: "if-stage__body-modifier-row",
                    label { class: "if-stage__body-modifier-item",
                        Checkbox {
                            checked: ctrl_ro,
                            onchange: on_ctrl,
                        }
                        span { class: "if-stage__body-modifier-label", "Ctrl" }
                    }
                    label { class: "if-stage__body-modifier-item",
                        Checkbox {
                            checked: alt_ro,
                            onchange: on_alt,
                        }
                        span { class: "if-stage__body-modifier-label", "Alt" }
                    }
                    label { class: "if-stage__body-modifier-item",
                        Checkbox {
                            checked: shift_ro,
                            onchange: on_shift,
                        }
                        span { class: "if-stage__body-modifier-label", "Shift" }
                    }
                    label { class: "if-stage__body-modifier-item",
                        Checkbox {
                            checked: win_ro,
                            onchange: on_win,
                        }
                        span { class: "if-stage__body-modifier-label", "Win" }
                    }
                }
            }
            // Key text field. Uses a raw `<input>` (not `TextInput`) because
            // the F2 `TextInput` component does not currently expose `onblur`
            // or `onkeydown` props, both of which are required for the
            // commit-on-blur dispatch pattern (see Task 15's NameField).
            // Class strings mirror what `TextInput` would emit so the styling
            // stays consistent across forms.
            div { class: "if-stage__body-field",
                label { class: "if-stage__body-label", "Key" }
                input {
                    r#type: "text",
                    class: "{key_class}",
                    value: "{local_key}",
                    placeholder: "e.g. Q, F1, Space",
                    // Use binding shorthand here. The compiler's
                    // `unused_qualifications` lint flags `oninput: oninput`
                    // style (because the macro expands to a redundant
                    // path), so we name the closures to match the prop
                    // names: `oninput`, `onblur`, `onkeydown`.
                    oninput,
                    onblur,
                    onkeydown,
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Build a [`KeyCombo`] by peeking the current `local_key` signal for the
/// base key and using the four caller-supplied bools for modifiers.
///
/// Each handler passes the *new* value for its own modifier and
/// `*signal.peek()` for all others, so the combo reflects the post-toggle
/// state without triggering a reactive subscription.
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "The four bools map 1-to-1 to the four KeyModifier variants; \
              introducing a wrapper struct would add ceremony at every call site \
              without improving clarity."
)]
fn build_combo_from(
    local_key: Signal<String>,
    ctrl: bool,
    alt: bool,
    shift: bool,
    win: bool,
) -> KeyCombo {
    build_combo_from_key(local_key.peek().clone(), ctrl, alt, shift, win)
}

/// Build a [`KeyCombo`] from an explicit key string and four modifier bools.
///
/// Separated from `build_combo_from` so the key-field handler can supply the
/// new key string it just received rather than re-reading the signal (which
/// was written in the same handler call, so the signal value may not yet have
/// propagated).
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "The four bools map 1-to-1 to the four KeyModifier variants; \
              introducing a wrapper struct would add ceremony at every call site \
              without improving clarity."
)]
fn build_combo_from_key(key: String, ctrl: bool, alt: bool, shift: bool, win: bool) -> KeyCombo {
    let mut modifiers = Vec::new();
    if ctrl {
        modifiers.push(KeyModifier::Ctrl);
    }
    if alt {
        modifiers.push(KeyModifier::Alt);
    }
    if shift {
        modifiers.push(KeyModifier::Shift);
    }
    if win {
        modifiers.push(KeyModifier::Win);
    }
    KeyCombo { key, modifiers }
}

/// Dispatch a `SetMapping` command and push an undo entry if it succeeds.
///
/// This is the shared commit path for all five field handlers. Amendment 7:
/// `push_edit` is skipped when `cmd_tx.send` returns `Err` so that a
/// disconnected engine channel never generates phantom undo entries.
#[allow(
    clippy::too_many_arguments,
    reason = "All arguments are logically distinct; grouping them into a struct \
              would introduce indirection without simplifying call sites, which \
              each already own the captured values independently."
)]
fn dispatch_keyboard(
    new_combo: KeyCombo,
    field_label: &'static str,
    mapping_key: &MappingKey,
    stage_id: &StageId,
    root_actions: &[Action],
    before: &Mapping,
    current_name: Option<String>,
    cmd_tx: &std::sync::mpsc::Sender<EngineCommand>,
    undo_log: &mut Signal<crate::frame::mapping_editor::undo_log::UndoLog>,
) {
    let new_action = Action::MapToKeyboard { key: new_combo };
    let Some(new_actions) = replace_at_path(root_actions, stage_id, new_action) else {
        return;
    };
    // Amendment 7: dispatch first; skip push_edit if the channel is closed.
    if cmd_tx
        .send(EngineCommand::SetMapping {
            input: mapping_key.1.clone(),
            mode: mapping_key.0.clone(),
            name: current_name,
            actions: new_actions,
        })
        .is_err()
    {
        tracing::warn!(
            target: "f9::mapping_editor",
            action = "map_to_keyboard_drop_offline",
            field = field_label,
            "keyboard change dropped: engine channel disconnected"
        );
        return;
    }
    let label = format_undo_label(
        UndoKind::StageEdit,
        LabelArgs {
            stage_name: Some("Map to keyboard"),
            field: Some(field_label),
            ..LabelArgs::default()
        },
    );
    undo_log.write().push_edit(
        mapping_key.clone(),
        before.clone(),
        UndoKind::StageEdit,
        label,
    );
}
