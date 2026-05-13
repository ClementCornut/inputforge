// Rust guideline compliant 2026-05-13

#[cfg(test)]
mod tests;

use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};

use dioxus::prelude::{
    Callback, Code, Location, ReadableExt, Signal, WritableExt, document, spawn, use_callback,
    use_context, use_context_provider, use_drop, use_effect, use_hook, use_memo, use_signal,
};
use inputforge_core::types::{KeyCombo, KeyModifier, PhysicalKey};
use serde::Deserialize;

const UNSUPPORTED_KEY_HINT: &str = "Unsupported key";
const MULTI_MODIFIER_ONLY_HINT: &str = "Modifier-only bindings must use one modifier";
const KEYBOARD_CAPTURE_ARM_JS: &str = "window.__inputforgeKeyboardCaptureArmed = true;";
const KEYBOARD_CAPTURE_DISARM_JS: &str = "window.__inputforgeKeyboardCaptureArmed = false;";
const KEYBOARD_CAPTURE_LISTENER_JS: &str = "\
window.__inputforgeKeyboardCaptureArmed = false;
const sendCapture = (ev, kind) => {
  if (!window.__inputforgeKeyboardCaptureArmed) return;
  ev.preventDefault();
  ev.stopPropagation();
  const payload = {
    kind,
    key: ev.key || '',
    code: ev.code || 'Unidentified',
    location: ev.location || 0,
    ctrl: !!ev.ctrlKey,
    alt: !!ev.altKey,
    shift: !!ev.shiftKey,
    meta: !!ev.metaKey,
  };
  console.debug('[keyboard_capture]', payload);
  dioxus.send(payload);
};
const keydown = (ev) => sendCapture(ev, 'keydown');
const keyup = (ev) => sendCapture(ev, 'keyup');
window.addEventListener('keydown', keydown, true);
window.addEventListener('keyup', keyup, true);
(async () => {
  while (true) {
    const msg = await dioxus.recv();
    if (msg === '__shutdown__') {
      window.removeEventListener('keydown', keydown, true);
      window.removeEventListener('keyup', keyup, true);
      dioxus.send('__ack__');
      return;
    }
  }
})();
";

static NEXT_CAPTURE_OWNER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CaptureKeyEventKind {
    KeyDown,
    KeyUp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "Browser keyboard events expose modifier state as independent booleans."
)]
pub(crate) struct CaptureKeyEvent {
    pub kind: CaptureKeyEventKind,
    pub code: Code,
    pub location: Location,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    pub key_is_meta_or_super: bool,
    pub key_is_escape: bool,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeyboardCaptureOwner(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeyboardCaptureUpdate {
    pub owner: KeyboardCaptureOwner,
    pub outcome: CaptureOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CaptureDelivery {
    pub sequence: u64,
    pub update: KeyboardCaptureUpdate,
}

impl CaptureDelivery {
    pub(crate) const fn new(sequence: u64, update: KeyboardCaptureUpdate) -> Self {
        Self { sequence, update }
    }

    pub(crate) fn next(&self, update: KeyboardCaptureUpdate) -> Self {
        Self {
            sequence: self.sequence.saturating_add(1),
            update,
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct KeyboardCaptureCore {
    owner: Option<KeyboardCaptureOwner>,
    capture: KeyboardCapture,
}

impl KeyboardCaptureCore {
    pub(crate) fn start(&mut self, owner: KeyboardCaptureOwner) -> Option<KeyboardCaptureOwner> {
        let previous = self
            .owner
            .replace(owner)
            .filter(|previous| *previous != owner);
        self.capture = KeyboardCapture::default();
        previous
    }

    pub(crate) fn cancel(&mut self, owner: KeyboardCaptureOwner) -> bool {
        if self.owner != Some(owner) {
            return false;
        }
        self.owner = None;
        self.capture = KeyboardCapture::default();
        true
    }

    #[cfg(test)]
    pub(crate) fn active_owner(&self) -> Option<KeyboardCaptureOwner> {
        self.owner
    }

    pub(crate) fn handle_payload(
        &mut self,
        payload: &BrowserKeyboardPayload,
    ) -> Option<KeyboardCaptureUpdate> {
        let owner = self.owner?;
        let outcome = self.capture.handle_event(payload.to_capture_key_event());
        log_capture_outcome(owner, &outcome);
        if matches!(
            outcome,
            CaptureOutcome::Commit(_) | CaptureOutcome::Cancel { .. }
        ) {
            self.owner = None;
        }
        Some(KeyboardCaptureUpdate { owner, outcome })
    }
}

#[derive(Clone, Copy)]
pub(crate) struct KeyboardCaptureContext {
    active_owner: Signal<Option<KeyboardCaptureOwner>>,
    hint_owner: Signal<Option<KeyboardCaptureOwner>>,
    hint: Signal<Option<&'static str>>,
    delivery: Signal<Option<CaptureDelivery>>,
    start: Callback<KeyboardCaptureOwner>,
    cancel: Callback<KeyboardCaptureOwner>,
}

#[derive(Clone, Copy)]
pub(crate) struct KeyboardCaptureBinding {
    pub active: dioxus::prelude::Memo<bool>,
    pub hint: dioxus::prelude::Memo<Option<&'static str>>,
    pub start: Callback<()>,
    #[expect(
        dead_code,
        reason = "Reusable capture consumers may expose an explicit cancel control; current caller relies on Escape."
    )]
    pub cancel: Callback<()>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "Browser keyboard events serialize modifier flags as independent booleans."
)]
pub(crate) struct BrowserKeyboardPayload {
    pub kind: String,
    pub key: String,
    pub code: String,
    pub location: u32,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

impl BrowserKeyboardPayload {
    pub(crate) fn to_capture_key_event(&self) -> CaptureKeyEvent {
        let code = Code::from_str(&self.code).unwrap_or(Code::Unidentified);
        CaptureKeyEvent {
            kind: match self.kind.as_str() {
                "keyup" => CaptureKeyEventKind::KeyUp,
                _ => CaptureKeyEventKind::KeyDown,
            },
            code,
            location: location_from_browser(self.location),
            ctrl: self.ctrl,
            alt: self.alt,
            shift: self.shift,
            meta: self.meta,
            key_is_meta_or_super: matches!(self.key.as_str(), "Meta" | "Super"),
            key_is_escape: self.key == "Escape" || matches!(code, Code::Escape),
        }
    }
}

pub(crate) fn use_keyboard_capture_provider() -> KeyboardCaptureContext {
    let mut core: Signal<KeyboardCaptureCore> = use_signal(KeyboardCaptureCore::default);
    let mut active_owner: Signal<Option<KeyboardCaptureOwner>> = use_signal(|| None);
    let mut hint_owner: Signal<Option<KeyboardCaptureOwner>> = use_signal(|| None);
    let mut hint: Signal<Option<&'static str>> = use_signal(|| None);
    let delivery: Signal<Option<CaptureDelivery>> = use_signal(|| None);
    let mut listener_mounted: Signal<bool> = use_signal(|| false);

    let cancel = use_callback(move |owner: KeyboardCaptureOwner| {
        cancel_owner(owner, core, active_owner, hint_owner, hint, delivery, None);
    });

    let start = use_callback(move |owner: KeyboardCaptureOwner| {
        tracing::debug!(
            target: "keyboard_capture",
            owner = owner.0,
            "keyboard capture started"
        );
        if let Some(previous_owner) = core.write().start(owner) {
            tracing::debug!(
                target: "keyboard_capture",
                owner = previous_owner.0,
                replaced_by = owner.0,
                "keyboard capture owner replaced"
            );
            publish_update(
                KeyboardCaptureUpdate {
                    owner: previous_owner,
                    outcome: CaptureOutcome::Cancel { hint: None },
                },
                delivery,
            );
        }
        active_owner.set(Some(owner));
        hint_owner.set(Some(owner));
        hint.set(None);
        let _ = document::eval(KEYBOARD_CAPTURE_ARM_JS);
    });

    use_effect(move || {
        let mut mounted = listener_mounted;
        if *mounted.peek() {
            return;
        }
        mounted.set(true);

        spawn(async move {
            let mut handle = document::eval(KEYBOARD_CAPTURE_LISTENER_JS);
            loop {
                let Ok(payload) = handle.recv::<BrowserKeyboardPayload>().await else {
                    break;
                };
                log_browser_payload(&payload);
                let Some(update) = core.write().handle_payload(&payload) else {
                    continue;
                };
                apply_update(update, active_owner, hint_owner, hint, delivery);
            }
            listener_mounted.set(false);
        });
    });

    let context = KeyboardCaptureContext {
        active_owner,
        hint_owner,
        hint,
        delivery,
        start,
        cancel,
    };
    use_context_provider(|| context);
    context
}

pub(crate) fn use_keyboard_capture(on_commit: Callback<KeyCombo>) -> KeyboardCaptureBinding {
    let context = use_context::<KeyboardCaptureContext>();
    let owner =
        use_hook(|| KeyboardCaptureOwner(NEXT_CAPTURE_OWNER.fetch_add(1, Ordering::Relaxed)));
    let mut processed_sequence: Signal<u64> = use_signal(|| 0);
    let active = use_memo(move || *context.active_owner.read() == Some(owner));
    let hint = use_memo(move || {
        if *context.hint_owner.read() == Some(owner) {
            *context.hint.read()
        } else {
            None
        }
    });
    let start = use_callback(move |()| {
        context.start.call(owner);
    });
    let cancel = use_callback(move |()| {
        context.cancel.call(owner);
    });

    use_effect(move || {
        let Some(delivery) = context.delivery.read().clone() else {
            return;
        };
        if delivery.sequence <= *processed_sequence.peek() || delivery.update.owner != owner {
            return;
        }
        processed_sequence.set(delivery.sequence);
        if let CaptureOutcome::Commit(combo) = delivery.update.outcome {
            on_commit.call(combo);
        }
    });

    use_drop(move || {
        context.cancel.call(owner);
    });

    KeyboardCaptureBinding {
        active,
        hint,
        start,
        cancel,
    }
}

fn apply_update(
    update: KeyboardCaptureUpdate,
    mut active_owner: Signal<Option<KeyboardCaptureOwner>>,
    mut hint_owner: Signal<Option<KeyboardCaptureOwner>>,
    mut hint: Signal<Option<&'static str>>,
    delivery: Signal<Option<CaptureDelivery>>,
) {
    match update.outcome {
        CaptureOutcome::Continue { hint: next_hint } => {
            hint_owner.set(Some(update.owner));
            hint.set(next_hint);
        }
        CaptureOutcome::Cancel { hint: next_hint } => {
            active_owner.set(None);
            hint_owner.set(Some(update.owner));
            hint.set(next_hint);
            tracing::debug!(
                target: "keyboard_capture",
                owner = update.owner.0,
                hint = ?next_hint,
                "keyboard capture cancelled"
            );
            publish_update(
                KeyboardCaptureUpdate {
                    owner: update.owner,
                    outcome: CaptureOutcome::Cancel { hint: next_hint },
                },
                delivery,
            );
            let _ = document::eval(KEYBOARD_CAPTURE_DISARM_JS);
        }
        CaptureOutcome::Commit(combo) => {
            active_owner.set(None);
            hint_owner.set(Some(update.owner));
            hint.set(None);
            tracing::debug!(
                target: "keyboard_capture",
                owner = update.owner.0,
                combo = ?combo,
                "keyboard capture committed"
            );
            publish_update(
                KeyboardCaptureUpdate {
                    owner: update.owner,
                    outcome: CaptureOutcome::Commit(combo),
                },
                delivery,
            );
            let _ = document::eval(KEYBOARD_CAPTURE_DISARM_JS);
        }
    }
}

fn cancel_owner(
    owner: KeyboardCaptureOwner,
    mut core: Signal<KeyboardCaptureCore>,
    mut active_owner: Signal<Option<KeyboardCaptureOwner>>,
    mut hint_owner: Signal<Option<KeyboardCaptureOwner>>,
    mut hint: Signal<Option<&'static str>>,
    delivery: Signal<Option<CaptureDelivery>>,
    next_hint: Option<&'static str>,
) {
    if !core.write().cancel(owner) {
        return;
    }
    active_owner.set(None);
    hint_owner.set(Some(owner));
    hint.set(next_hint);
    tracing::debug!(
        target: "keyboard_capture",
        owner = owner.0,
        hint = ?next_hint,
        "keyboard capture cancelled"
    );
    publish_update(
        KeyboardCaptureUpdate {
            owner,
            outcome: CaptureOutcome::Cancel { hint: next_hint },
        },
        delivery,
    );
    let _ = document::eval(KEYBOARD_CAPTURE_DISARM_JS);
}

fn publish_update(update: KeyboardCaptureUpdate, mut delivery: Signal<Option<CaptureDelivery>>) {
    let next = if let Some(current) = delivery.peek().as_ref() {
        current.next(update)
    } else {
        CaptureDelivery::new(1, update)
    };
    delivery.set(Some(next));
}

fn log_browser_payload(payload: &BrowserKeyboardPayload) {
    tracing::debug!(
        target: "keyboard_capture",
        kind = payload.kind.as_str(),
        key = payload.key.as_str(),
        code = payload.code.as_str(),
        location = payload.location,
        modifier_ctrl = payload.ctrl,
        modifier_alt = payload.alt,
        modifier_shift = payload.shift,
        modifier_meta = payload.meta,
        "keyboard capture browser payload"
    );
}

fn log_capture_outcome(owner: KeyboardCaptureOwner, outcome: &CaptureOutcome) {
    match outcome {
        CaptureOutcome::Continue { hint } => {
            tracing::debug!(
                target: "keyboard_capture",
                owner = owner.0,
                hint = ?hint,
                "keyboard capture outcome: continue"
            );
        }
        CaptureOutcome::Cancel { hint } => {
            tracing::debug!(
                target: "keyboard_capture",
                owner = owner.0,
                hint = ?hint,
                "keyboard capture outcome: cancel"
            );
        }
        CaptureOutcome::Commit(combo) => {
            tracing::debug!(
                target: "keyboard_capture",
                owner = owner.0,
                combo = ?combo,
                "keyboard capture outcome: commit"
            );
        }
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

const fn location_from_browser(location: u32) -> Location {
    match location {
        1 => Location::Left,
        2 => Location::Right,
        3 => Location::Numpad,
        _ => Location::Standard,
    }
}

const fn modifier_from_code(code: Code) -> Option<KeyModifier> {
    match code {
        Code::ControlLeft => Some(KeyModifier::CONTROL_LEFT),
        Code::ControlRight => Some(KeyModifier::CONTROL_RIGHT),
        Code::ShiftLeft => Some(KeyModifier::SHIFT_LEFT),
        Code::ShiftRight => Some(KeyModifier::SHIFT_RIGHT),
        Code::AltLeft => Some(KeyModifier::ALT_LEFT),
        Code::AltRight => Some(KeyModifier::ALT_RIGHT),
        Code::MetaLeft => Some(KeyModifier::META_LEFT),
        Code::MetaRight => Some(KeyModifier::META_RIGHT),
        _ => None,
    }
}

const fn modifier_from_event(event: CaptureKeyEvent) -> Option<KeyModifier> {
    if let Some(modifier) = modifier_from_code(event.code) {
        return Some(modifier);
    }

    if !event.key_is_meta_or_super || !matches!(event.code, Code::Unidentified) {
        return None;
    }

    match event.location {
        Location::Left | Location::Standard => Some(KeyModifier::META_LEFT),
        Location::Right => Some(KeyModifier::META_RIGHT),
        Location::Numpad => None,
    }
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
