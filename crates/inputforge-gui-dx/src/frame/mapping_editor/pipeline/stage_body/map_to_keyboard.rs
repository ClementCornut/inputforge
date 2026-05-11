// Rust guideline compliant 2026-05-01

//! `MapToKeyboard` body: physical key-combo capture + behavior selector.
//!
//! # Controls
//!
//! A single stable button captures the physical base key and any held
//! modifiers from the next keyboard event. Modifier-only events remain in
//! capture mode with an inline hint; unsupported DOM codes are ignored with
//! an inline hint.
//!
//! # Prop naming note
//!
//! Dioxus reserves the prop name `key` for the built-in reconciliation hint.
//! The keyboard combo is therefore exposed as the prop `combo` and
//! destructured that way in the function signature.
//!
//! # Name preservation (Amendment 4)
//!
//! `EngineCommand::SetMapping` requires a `name` field. On every dispatch we
//! read the current name from `cfg.mapping_names` so that user-set names are
//! never silently cleared.
//!

use dioxus::prelude::*;

use inputforge_core::action::{Action, Mapping, OutputBehavior};
use inputforge_core::engine::EngineCommand;
use inputforge_core::types::{KeyCombo, KeyModifier, PhysicalKey};

use crate::components::{SegmentedControl, SegmentedControlOption};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::replace_at_path;
use crate::frame::mapping_editor::undo_log::{LabelArgs, StageId, UndoKind, format_undo_label};

/// `MapToKeyboard` body: key-combo capture and behavior selector.
///
/// The prop is named `combo` rather than `key` because Dioxus reserves the
/// identifier `key` for its built-in reconciliation-hint attribute.
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX reports bound event handlers as redundant qualifications."
)]
pub(crate) fn MapToKeyboardBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    /// The keyboard combo to edit. Named `combo` (not `key`) because Dioxus
    /// reserves `key` as a built-in prop for its reconciliation hint.
    combo: KeyCombo,
    behavior: OutputBehavior,
    /// Full root-level action list for the mapping. Needed so that
    /// `replace_at_path` can build the new action tree on every edit.
    /// Named `root_actions` per Amendment 5 (the dispatcher uses this name).
    root_actions: Vec<Action>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let mut editor = use_context::<EditorState>();

    let mut local_combo: Signal<KeyCombo> = use_signal(|| combo.clone());
    let mut capture_active: Signal<bool> = use_signal(|| false);
    let mut capture_hint: Signal<Option<String>> = use_signal(|| None);
    if !*capture_active.peek() && *local_combo.peek() != combo {
        local_combo.set(combo.clone());
    }

    editor.malformed_hints.write().remove(&stage_id);

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

    let on_capture_start = move |_| {
        capture_active.set(true);
        capture_hint.set(None);
    };

    let mapping_key_capture = mapping_key.clone();
    let stage_id_capture = stage_id.clone();
    let root_actions_capture = root_actions.clone();
    let before_capture = before_mapping.clone();
    let current_name_capture = current_name.clone();
    let cmd_tx_capture = ctx.commands.clone();
    let mut undo_log_capture = editor.undo_log;
    let on_capture_keydown = move |evt: KeyboardEvent| {
        if !*capture_active.peek() {
            return;
        }
        evt.prevent_default();
        evt.stop_propagation();

        let code = evt.code();
        if is_capture_cancel_event(&evt, code) {
            capture_active.set(false);
            capture_hint.set(None);
            return;
        }

        if is_modifier_code(code) {
            capture_hint.set(Some("Press a base key with any modifiers held".to_owned()));
            return;
        }

        let Some(key) = physical_key_from_code(code) else {
            capture_hint.set(Some("Unsupported key".to_owned()));
            return;
        };

        let new_combo = build_combo_from_key(key, modifier_state_from_event(&evt));
        let old_combo = local_combo.peek().clone();

        local_combo.set(new_combo.clone());
        capture_active.set(false);
        capture_hint.set(None);

        if new_combo == old_combo {
            return;
        }
        dispatch_keyboard(
            new_combo,
            behavior,
            "key",
            &mapping_key_capture,
            &stage_id_capture,
            &root_actions_capture,
            &before_capture,
            current_name_capture.clone(),
            &cmd_tx_capture,
            &mut undo_log_capture,
        );
    };

    let current_combo = local_combo.read().clone();
    let key_label = format_key_combo(&current_combo);
    let capture_message = capture_hint.read().clone();
    let is_listening = *capture_active.read();
    let capture_class = if is_listening {
        "if-key-capture__surface is-listening"
    } else {
        "if-key-capture__surface"
    };
    let capture_aria_label = if is_listening {
        "Press keyboard shortcut"
    } else {
        "Capture keyboard shortcut"
    };
    let capture_text = if is_listening {
        "Press keys"
    } else {
        key_label.as_str()
    };

    let mapping_key_hold = mapping_key.clone();
    let stage_id_hold = stage_id.clone();
    let root_actions_hold = root_actions.clone();
    let before_hold = before_mapping.clone();
    let current_name_hold = current_name.clone();
    let cmd_tx_hold = ctx.commands.clone();
    let mut undo_log_hold = editor.undo_log;
    let on_hold = move |_| {
        if is_output_behavior_click_noop(behavior, OutputBehavior::Hold) {
            return;
        }
        let new_combo = local_combo.peek().clone();
        dispatch_keyboard(
            new_combo,
            OutputBehavior::Hold,
            "behavior",
            &mapping_key_hold,
            &stage_id_hold,
            &root_actions_hold,
            &before_hold,
            current_name_hold.clone(),
            &cmd_tx_hold,
            &mut undo_log_hold,
        );
    };

    let mapping_key_pulse = mapping_key.clone();
    let stage_id_pulse = stage_id.clone();
    let root_actions_pulse = root_actions.clone();
    let before_pulse = before_mapping.clone();
    let current_name_pulse = current_name.clone();
    let cmd_tx_pulse = ctx.commands.clone();
    let mut undo_log_pulse = editor.undo_log;
    let on_pulse = move |_| {
        if is_output_behavior_click_noop(behavior, OutputBehavior::Pulse) {
            return;
        }
        let new_combo = local_combo.peek().clone();
        dispatch_keyboard(
            new_combo,
            OutputBehavior::Pulse,
            "behavior",
            &mapping_key_pulse,
            &stage_id_pulse,
            &root_actions_pulse,
            &before_pulse,
            current_name_pulse.clone(),
            &cmd_tx_pulse,
            &mut undo_log_pulse,
        );
    };

    rsx! {
        div { class: "if-stage__body-keyboard",
            div { class: "if-stage__body-field",
                label { class: "if-stage__body-label", "Key" }
                div { class: "if-key-capture",
                    button {
                        r#type: "button",
                        class: "{capture_class}",
                        "aria-label": "{capture_aria_label}",
                        "data-key-capture": if is_listening { "active" } else { "idle" },
                        onclick: on_capture_start,
                        onkeydown: on_capture_keydown,
                        span { class: "if-key-capture__value", "{capture_text}" }
                    }
                    if let Some(message) = capture_message {
                        span { class: "if-key-capture__hint", "{message}" }
                    }
                }
            }
            div { class: "if-stage__body-field",
                label { class: "if-stage__body-label", "Behavior" }
                SegmentedControl { aria_label: "Keyboard output behavior".to_owned(),
                    SegmentedControlOption {
                        value: "hold".to_owned(),
                        selected: behavior == OutputBehavior::Hold,
                        onclick: on_hold,
                        "Hold"
                    }
                    SegmentedControlOption {
                        value: "pulse".to_owned(),
                        selected: behavior == OutputBehavior::Pulse,
                        onclick: on_pulse,
                        "Pulse"
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn build_combo_from_key(key: PhysicalKey, (ctrl, alt, shift, win): ModifierState) -> KeyCombo {
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

fn format_key_combo(combo: &KeyCombo) -> String {
    let mut parts: Vec<String> = combo
        .modifiers
        .iter()
        .map(|modifier| match modifier {
            KeyModifier::Ctrl => "Ctrl",
            KeyModifier::Shift => "Shift",
            KeyModifier::Alt => "Alt",
            KeyModifier::Win => "Win",
        })
        .map(str::to_owned)
        .collect();
    parts.push(combo.key.display_label().into_owned());
    parts.join(" + ")
}

type ModifierState = (bool, bool, bool, bool);

fn modifier_state_from_event(evt: &KeyboardEvent) -> ModifierState {
    let modifiers = evt.modifiers();
    (
        modifiers.ctrl(),
        modifiers.alt(),
        modifiers.shift(),
        modifiers.meta(),
    )
}

const fn is_modifier_code(code: Code) -> bool {
    matches!(
        code,
        Code::ControlLeft
            | Code::ControlRight
            | Code::ShiftLeft
            | Code::ShiftRight
            | Code::AltLeft
            | Code::AltRight
            | Code::MetaLeft
            | Code::MetaRight
    )
}

const fn is_capture_cancel_code(code: Code) -> bool {
    matches!(code, Code::Escape)
}

fn is_capture_cancel_event(evt: &KeyboardEvent, code: Code) -> bool {
    evt.key() == Key::Escape || is_capture_cancel_code(code)
}

const fn physical_key_from_code(code: Code) -> Option<PhysicalKey> {
    match code {
        Code::KeyA => Some(PhysicalKey::KeyA),
        Code::KeyB => Some(PhysicalKey::KeyB),
        Code::KeyC => Some(PhysicalKey::KeyC),
        Code::KeyD => Some(PhysicalKey::KeyD),
        Code::KeyE => Some(PhysicalKey::KeyE),
        Code::KeyF => Some(PhysicalKey::KeyF),
        Code::KeyG => Some(PhysicalKey::KeyG),
        Code::KeyH => Some(PhysicalKey::KeyH),
        Code::KeyI => Some(PhysicalKey::KeyI),
        Code::KeyJ => Some(PhysicalKey::KeyJ),
        Code::KeyK => Some(PhysicalKey::KeyK),
        Code::KeyL => Some(PhysicalKey::KeyL),
        Code::KeyM => Some(PhysicalKey::KeyM),
        Code::KeyN => Some(PhysicalKey::KeyN),
        Code::KeyO => Some(PhysicalKey::KeyO),
        Code::KeyP => Some(PhysicalKey::KeyP),
        Code::KeyQ => Some(PhysicalKey::KeyQ),
        Code::KeyR => Some(PhysicalKey::KeyR),
        Code::KeyS => Some(PhysicalKey::KeyS),
        Code::KeyT => Some(PhysicalKey::KeyT),
        Code::KeyU => Some(PhysicalKey::KeyU),
        Code::KeyV => Some(PhysicalKey::KeyV),
        Code::KeyW => Some(PhysicalKey::KeyW),
        Code::KeyX => Some(PhysicalKey::KeyX),
        Code::KeyY => Some(PhysicalKey::KeyY),
        Code::KeyZ => Some(PhysicalKey::KeyZ),
        Code::Digit0 => Some(PhysicalKey::Digit0),
        Code::Digit1 => Some(PhysicalKey::Digit1),
        Code::Digit2 => Some(PhysicalKey::Digit2),
        Code::Digit3 => Some(PhysicalKey::Digit3),
        Code::Digit4 => Some(PhysicalKey::Digit4),
        Code::Digit5 => Some(PhysicalKey::Digit5),
        Code::Digit6 => Some(PhysicalKey::Digit6),
        Code::Digit7 => Some(PhysicalKey::Digit7),
        Code::Digit8 => Some(PhysicalKey::Digit8),
        Code::Digit9 => Some(PhysicalKey::Digit9),
        Code::F1 => Some(PhysicalKey::F1),
        Code::F2 => Some(PhysicalKey::F2),
        Code::F3 => Some(PhysicalKey::F3),
        Code::F4 => Some(PhysicalKey::F4),
        Code::F5 => Some(PhysicalKey::F5),
        Code::F6 => Some(PhysicalKey::F6),
        Code::F7 => Some(PhysicalKey::F7),
        Code::F8 => Some(PhysicalKey::F8),
        Code::F9 => Some(PhysicalKey::F9),
        Code::F10 => Some(PhysicalKey::F10),
        Code::F11 => Some(PhysicalKey::F11),
        Code::F12 => Some(PhysicalKey::F12),
        Code::Space => Some(PhysicalKey::Space),
        Code::Enter => Some(PhysicalKey::Enter),
        Code::Tab => Some(PhysicalKey::Tab),
        Code::Escape => Some(PhysicalKey::Escape),
        Code::Backspace => Some(PhysicalKey::Backspace),
        Code::Delete => Some(PhysicalKey::Delete),
        Code::Insert => Some(PhysicalKey::Insert),
        Code::ArrowUp => Some(PhysicalKey::ArrowUp),
        Code::ArrowDown => Some(PhysicalKey::ArrowDown),
        Code::ArrowLeft => Some(PhysicalKey::ArrowLeft),
        Code::ArrowRight => Some(PhysicalKey::ArrowRight),
        Code::Home => Some(PhysicalKey::Home),
        Code::End => Some(PhysicalKey::End),
        Code::PageUp => Some(PhysicalKey::PageUp),
        Code::PageDown => Some(PhysicalKey::PageDown),
        Code::Minus => Some(PhysicalKey::Minus),
        Code::Equal => Some(PhysicalKey::Equal),
        Code::BracketLeft => Some(PhysicalKey::BracketLeft),
        Code::BracketRight => Some(PhysicalKey::BracketRight),
        Code::Backslash => Some(PhysicalKey::Backslash),
        Code::IntlBackslash => Some(PhysicalKey::IntlBackslash),
        Code::Semicolon => Some(PhysicalKey::Semicolon),
        Code::Quote => Some(PhysicalKey::Quote),
        Code::Backquote => Some(PhysicalKey::Backquote),
        Code::Comma => Some(PhysicalKey::Comma),
        Code::Period => Some(PhysicalKey::Period),
        Code::Slash => Some(PhysicalKey::Slash),
        Code::Numpad0 => Some(PhysicalKey::Numpad0),
        Code::Numpad1 => Some(PhysicalKey::Numpad1),
        Code::Numpad2 => Some(PhysicalKey::Numpad2),
        Code::Numpad3 => Some(PhysicalKey::Numpad3),
        Code::Numpad4 => Some(PhysicalKey::Numpad4),
        Code::Numpad5 => Some(PhysicalKey::Numpad5),
        Code::Numpad6 => Some(PhysicalKey::Numpad6),
        Code::Numpad7 => Some(PhysicalKey::Numpad7),
        Code::Numpad8 => Some(PhysicalKey::Numpad8),
        Code::Numpad9 => Some(PhysicalKey::Numpad9),
        Code::NumpadAdd => Some(PhysicalKey::NumpadAdd),
        Code::NumpadSubtract => Some(PhysicalKey::NumpadSubtract),
        Code::NumpadMultiply => Some(PhysicalKey::NumpadMultiply),
        Code::NumpadDivide => Some(PhysicalKey::NumpadDivide),
        Code::NumpadDecimal => Some(PhysicalKey::NumpadDecimal),
        Code::NumpadEnter => Some(PhysicalKey::NumpadEnter),
        _ => None,
    }
}

fn is_output_behavior_click_noop(
    current_behavior: OutputBehavior,
    requested_behavior: OutputBehavior,
) -> bool {
    current_behavior == requested_behavior
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
    new_behavior: OutputBehavior,
    field_label: &'static str,
    mapping_key: &MappingKey,
    stage_id: &StageId,
    root_actions: &[Action],
    before: &Mapping,
    current_name: Option<String>,
    cmd_tx: &std::sync::mpsc::Sender<EngineCommand>,
    undo_log: &mut Signal<crate::frame::mapping_editor::undo_log::UndoLog>,
) {
    let new_action = Action::MapToKeyboard {
        key: new_combo,
        behavior: new_behavior,
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_to_keyboard_behavior_click_noop_only_when_behavior_is_unchanged() {
        assert!(is_output_behavior_click_noop(
            OutputBehavior::Hold,
            OutputBehavior::Hold
        ));
        assert!(!is_output_behavior_click_noop(
            OutputBehavior::Hold,
            OutputBehavior::Pulse
        ));
    }

    #[test]
    fn physical_key_capture_maps_dom_codes_to_physical_keys() {
        assert_eq!(physical_key_from_code(Code::KeyA), Some(PhysicalKey::KeyA));
        assert_eq!(
            physical_key_from_code(Code::Digit7),
            Some(PhysicalKey::Digit7)
        );
        assert_eq!(physical_key_from_code(Code::F12), Some(PhysicalKey::F12));
        assert_eq!(
            physical_key_from_code(Code::Slash),
            Some(PhysicalKey::Slash)
        );
        assert_eq!(
            physical_key_from_code(Code::BracketLeft),
            Some(PhysicalKey::BracketLeft)
        );
        assert_eq!(
            physical_key_from_code(Code::IntlBackslash),
            Some(PhysicalKey::IntlBackslash)
        );
        assert_eq!(
            physical_key_from_code(Code::NumpadDivide),
            Some(PhysicalKey::NumpadDivide)
        );
        assert_eq!(
            physical_key_from_code(Code::NumpadEnter),
            Some(PhysicalKey::NumpadEnter)
        );
        assert_eq!(physical_key_from_code(Code::AudioVolumeUp), None);
    }

    #[test]
    fn modifier_dom_codes_are_not_base_keys() {
        assert!(is_modifier_code(Code::ControlLeft));
        assert!(is_modifier_code(Code::MetaRight));
        assert_eq!(physical_key_from_code(Code::ControlLeft), None);
    }

    #[test]
    fn escape_code_aborts_capture_before_key_mapping() {
        assert!(is_capture_cancel_code(Code::Escape));
        assert_eq!(
            physical_key_from_code(Code::Escape),
            Some(PhysicalKey::Escape)
        );
    }
}
