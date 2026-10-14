//! Physical key combinations and modifier-only completion.
use super::translation::{
    CaptureKeyEvent, CaptureKeyEventKind, modifier_from_event, physical_key_from_code,
};
use inputforge_core::types::{KeyCombo, KeyModifier};
const UNSUPPORTED_KEY_HINT: &str = "Unsupported key";
const MULTI_MODIFIER_ONLY_HINT: &str = "Modifier-only bindings must use one modifier";
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CaptureOutcome {
    Continue { hint: Option<&'static str> },
    Commit(KeyCombo),
    Cancel { hint: Option<&'static str> },
}

#[derive(Debug, Default)]
pub(crate) struct KeyboardCapture {
    modifiers: Vec<KeyModifier>,
    cancelled: bool,
}

impl KeyboardCapture {
    pub(crate) fn handle_event(&mut self, event: CaptureKeyEvent) -> CaptureOutcome {
        if self.cancelled {
            return CaptureOutcome::Cancel { hint: None };
        }
        if event.key_is_escape {
            self.cancelled = true;
            return CaptureOutcome::Cancel { hint: None };
        }

        match event.kind {
            CaptureKeyEventKind::KeyDown => self.handle_keydown(event),
            CaptureKeyEventKind::KeyUp => self.handle_keyup(event),
        }
    }

    fn handle_keydown(&mut self, event: CaptureKeyEvent) -> CaptureOutcome {
        if let Some(modifier) = modifier_from_event(event) {
            self.push_modifier(modifier);
            return CaptureOutcome::Continue { hint: None };
        }

        let Some(key) = physical_key_from_code(event.code) else {
            tracing::debug!(
                target: "keyboard_capture",
                code = ?event.code,
                location = ?event.location,
                key_is_meta_or_super = event.key_is_meta_or_super,
                modifier_ctrl = event.ctrl,
                modifier_alt = event.alt,
                modifier_shift = event.shift,
                modifier_meta = event.meta,
                "unsupported keyboard capture event"
            );
            self.cancelled = true;
            return CaptureOutcome::Cancel {
                hint: Some(UNSUPPORTED_KEY_HINT),
            };
        };

        let modifiers = self.modifiers_with_fallbacks(event);
        CaptureOutcome::Commit(KeyCombo { key, modifiers })
    }

    fn handle_keyup(&mut self, event: CaptureKeyEvent) -> CaptureOutcome {
        let Some(modifier) = modifier_from_event(event) else {
            return self.handle_non_modifier_keyup(event);
        };

        if self.modifiers.len() > 1 {
            self.cancelled = true;
            return CaptureOutcome::Cancel {
                hint: Some(MULTI_MODIFIER_ONLY_HINT),
            };
        }

        if self.modifiers.first().copied() == Some(modifier) {
            return CaptureOutcome::Commit(KeyCombo {
                key: modifier.physical_key(),
                modifiers: Vec::new(),
            });
        }

        CaptureOutcome::Continue { hint: None }
    }

    fn handle_non_modifier_keyup(&self, event: CaptureKeyEvent) -> CaptureOutcome {
        let Some(key) = physical_key_from_code(event.code) else {
            return CaptureOutcome::Continue { hint: None };
        };
        let modifiers = self.modifiers_with_fallbacks(event);
        if modifiers.is_empty() {
            return CaptureOutcome::Continue { hint: None };
        }
        CaptureOutcome::Commit(KeyCombo { key, modifiers })
    }

    fn push_modifier(&mut self, modifier: KeyModifier) {
        if modifier == KeyModifier::ALT_RIGHT {
            self.modifiers
                .retain(|stored| *stored != KeyModifier::CONTROL_LEFT);
        } else if modifier == KeyModifier::CONTROL_LEFT
            && self.modifiers.contains(&KeyModifier::ALT_RIGHT)
        {
            return;
        }

        if !self.modifiers.contains(&modifier) {
            self.modifiers.push(modifier);
        }
    }

    fn modifiers_with_fallbacks(&self, event: CaptureKeyEvent) -> Vec<KeyModifier> {
        let mut modifiers = self.modifiers.clone();
        if event.ctrl
            && !modifiers.contains(&KeyModifier::ALT_RIGHT)
            && !contains_modifier_family(
                &modifiers,
                KeyModifier::CONTROL_LEFT,
                KeyModifier::CONTROL_RIGHT,
            )
        {
            push_unique(&mut modifiers, KeyModifier::CONTROL_LEFT);
        }
        if event.alt
            && !contains_modifier_family(&modifiers, KeyModifier::ALT_LEFT, KeyModifier::ALT_RIGHT)
        {
            push_unique(&mut modifiers, KeyModifier::ALT_LEFT);
        }
        if event.shift
            && !contains_modifier_family(
                &modifiers,
                KeyModifier::SHIFT_LEFT,
                KeyModifier::SHIFT_RIGHT,
            )
        {
            push_unique(&mut modifiers, KeyModifier::SHIFT_LEFT);
        }
        if event.meta
            && !contains_modifier_family(
                &modifiers,
                KeyModifier::META_LEFT,
                KeyModifier::META_RIGHT,
            )
        {
            push_unique(&mut modifiers, KeyModifier::META_LEFT);
        }
        modifiers
    }
}

fn contains_modifier_family(
    modifiers: &[KeyModifier],
    left: KeyModifier,
    right: KeyModifier,
) -> bool {
    modifiers
        .iter()
        .any(|modifier| *modifier == left || *modifier == right)
}

fn push_unique(modifiers: &mut Vec<KeyModifier>, modifier: KeyModifier) {
    if !modifiers.contains(&modifier) {
        modifiers.push(modifier);
    }
}
