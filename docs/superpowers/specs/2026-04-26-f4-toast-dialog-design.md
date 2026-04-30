# F4, Toast & Dialog Infrastructure: Design Spec

**Status:** Design approved, ready for implementation plan
**Date:** 2026-04-26
**Parent spec:** [`2026-04-24-egui-to-dioxus-rewrite-design.md`](./2026-04-24-egui-to-dioxus-rewrite-design.md), Foundation feature F4
**Predecessors:** [F1](./2026-04-24-f1-dioxus-scaffold-state-bridge-design.md) (state bridge), [F2](./2026-04-25-f2-design-system-design.md) (design system), [F3](./2026-04-26-f3-app-shell-tray-bridge-design.md) (app shell + tray bridge)

---

## Context

F4 is the fourth and final foundation feature. It delivers three concerns that subsequent features (F8 mapping editor, F11 mode editor, F13 calibration, F14 profile) all depend on:

1. **Toast queue**, global, level-aware, dedupe + cap, auto-dismiss with hover/focus pause, keyboard-dismissible. Replaces the egui `ToastManager` (`crates/inputforge-gui/src/widgets/toast.rs`).
2. **Modal dialog primitive**, compound API (`DialogRoot/Title/Description/Body/Footer`) on the native HTML `<dialog>` element with `showModal()`. Focus trap, ESC handling, inert background, focus restore, all native-browser-handled.
3. **Dirty-state confirmation**, a presentational `DirtyConfirmDialog` reusable that composes the dialog primitives with default copy and Cancel/Discard/Save actions. Replaces the egui dirty-confirm flow (`crates/inputforge-gui/src/app.rs:347-394`).

Plus one production wiring:

4. **Warnings bridge**, `MetaSnapshot.warnings` → `ToastQueue.push(Warning, msg)` for new tail entries. Matches today's egui behavior so the F14 default-feature flip to `gui-dioxus` does not regress warning visibility.

The F4 spec in the parent plan is short:

> Global toast queue with level (info/success/warning/error), dedupe, and auto-dismiss. Modal dialog primitive with focus trap and ESC-to-dismiss. Dirty-state confirmation pattern (reusable component used when switching inputs/devices with unsaved changes).
>
> **Acceptance:** test screen exercises all four toast levels, a modal dialog, and a dirty-state confirmation flow.

This spec adds: warnings bridge as a production subscriber, exact-string dedupe with `×N` count badge, max-visible cap of 5 with FIFO drain, per-level ARIA live regions (polite for info/success, assertive for warning/error), keyboard parity (focus pauses timer, ESC-while-focused dismisses), Cancel/Discard/Save button order with default focus on Cancel.

The egui GUI stays the default runtime behavior. F4 changes nothing in `inputforge-gui` or `inputforge-app` except the gallery example in `inputforge-gui-dx`.

---

## Confirmed design choices

Decisions made during brainstorming that shape this spec:

1. **Production wiring scope: infra + warnings bridge.** F4 ships infrastructure (toast queue, dialog primitives, dirty-confirm reusable) plus one production subscriber: `MetaSnapshot.warnings` → toast queue. Other producers (F8 dirty-confirm, F14 profile-load Success toasts) wire themselves later. Pure-infra (no producer) was rejected because it leaves a known regression at F14 cutover; full-shell wiring (with a placeholder dirty-confirm flow) was rejected because it couples F4 to the disposable F3 placeholder shell.

2. **Toast queue: dedicated `ToastQueue` context.** Installed via `use_context_provider` in `app_root` as a sibling of `AppContext`. `AppContext` stays focused on engine state + signals; toasts are UI infrastructure. Producers reach the queue with `use_context::<ToastQueue>().push(level, msg)`. Global static was rejected (Signal must be created inside the runtime; testing the queue in isolation becomes harder); piling onto `AppContext` was rejected to keep that struct's boundary clean.

3. **Dialog primitive: compound API on native `<dialog>`.** Symmetric with the existing `Menu` compound (`MenuRoot/Trigger/Items/Item`). Native `<dialog>` + `showModal()` gives focus trap, ESC handling, inert background, focus restore, and `aria-modal` for free. WebView2 (Chromium) supports `<dialog>` fully; the project's minimum platform (Windows 10 20H1+) is well past the support bar. JS surface is small (`showModal`/`close`/`cancel` event), mirroring `Menu`'s `eval`-driven focus walker. Callbacks use `EventHandler<()>` everywhere (matching the `Button` precedent in `components/button.rs:72`), `Callback<T>` and `EventHandler<T>` are aliases in Dioxus 0.7, but the crate has converged on `EventHandler` and the dialog primitives align.

4. **DirtyConfirmDialog: presentational component.** Takes a controlled `open: Signal<bool>` plus three callbacks (`oncancel`, `ondiscard`, `onsave`) and composes the dialog primitives with default copy. Consumer owns the "pending target" state, same shape as today's egui flow. Hook-based API (`use_dirty_confirm`) was rejected as speculative, until F8 names a real second consumer, it's API for one caller.

5. **Buttons: Cancel / Discard / Save.** Cancel first in document order (so `showModal()`'s default-focus rule lands on it); ESC fires Cancel. Egui's two-button (Discard / Save, ESC = Discard) was rejected because discarding work on accidental ESC is a footgun on a HOTAS-config tool, Cancel-by-default is the WAI-ARIA recommendation for destructive-confirmation dialogs. **Migration note for F8:** the egui flow has only two outcomes (discard-and-switch / save-and-switch); the Dioxus flow adds a third outcome (Cancel = stay on current input, no switch). F8's switch-input handler must absorb this, Cancel is not a no-op equivalent of Discard but a distinct "abort the switch entirely" path.

6. **Toast levels: info / success / warning / error.** Matches F2's published primitive vocabulary. Egui parity (info/warning/error) was rejected because adding `Success` later means F14 circles back to F4; the cost now is one extra CSS rule and one variant.

7. **Dedupe: exact-string coalesce with count badge.** Identical (`level`, `message`) pushed against any non-dismissed toast increments its count and resets the auto-dismiss timer. Renders as `HidHide unavailable ×3`. Time-windowed suppression was rejected (loses the "happened multiple times" signal). Caller-provided dedupe keys were rejected as premature API.

8. **Max-visible cap: 5; FIFO drain.** When a non-coalescing push would exceed 5 visible toasts, the oldest non-dismissed toast is auto-dismissed. Hard cap with drop-on-full was rejected (silent loss). No cap was rejected (a flood of distinct messages escapes dedupe and floods the screen).

9. **ARIA: per-level live regions.** Two viewport divs: `role="status" aria-live="polite"` for Info/Success, `role="alert" aria-live="assertive"` for Warning/Error. AT picks the correct delivery verb without us tagging each toast individually.

10. **Engine-status announcement coexistence.** F3's `<span role="status" aria-live="polite">` wrapper around the engine-status badge stays. F4 does **not** push toasts on engine-status flips, only on warnings. Two channels, two purposes. F3's wrapper does NOT drop to ARIA-neutral. (F15 audit may revisit.)

11. **Hover/focus pause + keyboard dismissal.** Hover or focus on a toast pauses the auto-dismiss timer; ESC-while-focused dismisses; toasts are tab-reachable. Egui-parity-only (hover-pause + click-x) was rejected because keyboard-only users would be locked out of dismissal.

12. **Toast position: top-right.** Matches today's egui. `position: fixed; top: 12px; right: 12px;` with `pointer-events: none` on the viewport and `pointer-events: auto` on individual toasts so empty space doesn't block underlying clicks. `impeccable:frontend-design` may revise during implementation; the change is a few CSS rules with tokens, not a structural change.

13. **`impeccable:frontend-design` invoked early in F4 implementation.** Brief scoped to: toast visual treatment (chrome, accent bar, icon, count badge, motion), dialog visual treatment (panel surface, backdrop, footer alignment, motion). NOT IA, this is presentation polish on already-defined primitives.

## Non-goals (deferred to named later features)

- **Toast producers other than warnings bridge**, F8 (dirty-state on switch), F11 (mode rename collisions), F13 (calibration apply confirmation), F14 (profile load success). Each feature wires its own.
- **Light-theme values** for toast accents and dialog backdrop → out of scope for the whole rewrite until needed.
- **Custom dialog stacking / multi-dialog support**, native `<dialog>` handles one-at-a-time; F4 only ever opens one at a time.
- **Toast → engine-status announcement supersession** → F15 audit may decide later.
- **Animated/blurred dialog backdrop**, F2's elevation philosophy is borders-over-shadows; if frontend-design wants more, the change is CSS-only.
- **Hook-based dirty-confirm API (`use_dirty_confirm`)** → consider in F8 if a second real consumer materializes.
- **Per-toast action buttons (e.g., "Undo")** → F14 may want this; out of scope for F4.
- **Persisting toast history / "view recent" panel** → F15 polish at the earliest.

---

## Architecture

### Crate layout (additions on top of F3)

```
crates/inputforge-gui-dx/
├── src/
│   ├── lib.rs                       # MODIFIED, pub mod toast; pub mod patterns; expose Dialog* and DirtyConfirmDialog
│   ├── app.rs                       # MODIFIED, install ToastQueue context, render ToastViewport, install warnings bridge
│   ├── toast/
│   │   ├── mod.rs                   # NEW, pub use of state + queue + viewport + types
│   │   ├── state.rs                 # NEW, ToastState (pure data: Vec<Toast> + next_id) + push/coalesce/cap/dismiss/pause/resume/is_expired methods; runtime-free, fully unit-testable
│   │   ├── queue.rs                 # NEW, ToastQueue Signal wrapper around ToastState, delegates every method
│   │   ├── viewport.rs              # NEW, ToastViewport component (renders the queue, drives timers, ARIA live regions)
│   │   └── warnings_bridge.rs       # NEW, install_warnings_bridge: subscribe to MetaSnapshot.warnings, push Warning toasts on new tail
│   ├── components/
│   │   ├── dialog.rs                # NEW, DialogRoot/Title/Description/Body/Footer compound; showModal eval
│   │   ├── mod.rs                   # MODIFIED, re-export Dialog primitives
│   │   └── ... (existing F2/F3, unchanged)
│   ├── patterns/                    # NEW, future home for additional reusables (e.g. SaveBeforeLeave, ConfirmDestructive); F4 ships only DirtyConfirmDialog
│   │   ├── mod.rs                   # NEW, pub use DirtyConfirmDialog
│   │   └── dirty_confirm.rs         # NEW, DirtyConfirmDialog component
│   ├── theme/                       # F2, unchanged
│   ├── shell/                       # F3, unchanged
│   ├── tray/                        # F3, unchanged
│   ├── lifecycle/                   # F3, unchanged
│   ├── bridge.rs                    # F1, unchanged
│   └── context.rs                   # F1/F3, unchanged (ToastQueue lives in its own context)
├── assets/
│   ├── components/
│   │   ├── dialog.css               # NEW, :modal, ::backdrop, panel, body scroll, footer alignment
│   │   └── ... (existing F2/F3, unchanged)
│   ├── toast/
│   │   └── toast.css                # NEW, viewport position, per-level accents, count badge, hover/focus styles, motion
│   └── ... (existing F2/F3, unchanged)
└── examples/
    └── component_gallery.rs         # MODIFIED, add Toasts, Dialogs, DirtyConfirmDialog sections
```

**Key boundaries:**

- `toast::state` is pure data, `ToastState { toasts: Vec<Toast>, next_id: u64 }` with all push/coalesce/cap/dismiss/pause/resume/is_expired methods on `&mut self`. Unit-tested directly; constructs without a Dioxus runtime.
- `toast::queue` is a thin Signal wrapper (`ToastQueue { state: Signal<ToastState> }`) that delegates every method via `self.state.write()`. No logic here.
- `toast::viewport` is the one component that subscribes to the queue, drives timers, and owns the ARIA live regions.
- `components::dialog` knows nothing about toasts, pure presentation primitive.
- `patterns::dirty_confirm` knows nothing about engine state, pure presentation; consumers wire action outcomes.
- `toast::warnings_bridge` is the only place that couples engine state (`MetaSnapshot.warnings`) to UI infrastructure (`ToastQueue`). It reads `ctx.meta` directly, which only works because `AppContext` and `MetaSnapshot` are `pub(crate)` in `context.rs:31,42`; moving the bridge out of this crate would require promoting both to `pub`.

**Design tokens added:** F4 introduces `--z-toast` in `theme/tokens.css`, layered above existing F2 z-index tokens so toasts surface above shell content and any open `<dialog>` (intentional, see Risks).

### Toast queue

The queue is split into two layers: `state.rs` is pure data (no Dioxus runtime, fully unit-testable), and `queue.rs` is a thin Signal wrapper that delegates every method.

#### `toast/state.rs`, pure data + behavior

```rust
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel { Info, Success, Warning, Error }

#[derive(Debug, Clone)]
pub struct Toast {
    pub id: u64,
    pub level: ToastLevel,
    pub message: String,
    pub count: u32,                // dedupe coalesce, starts at 1
    pub created: Instant,
    pub paused: Option<Instant>,   // hover/focus pause start
    pub paused_total: Duration,    // accumulated pause time
    pub dismissed: bool,
}

#[derive(Debug, Default)]
pub struct ToastState {
    pub toasts: Vec<Toast>,
    pub next_id: u64,
}

pub const TOAST_DURATION: Duration = Duration::from_secs(8);
pub const TOAST_MAX_VISIBLE: usize = 5;
// Fade-out duration is a CSS concern; see toast.css `transition: opacity 400ms`.
// No corresponding Rust constant, keeps the layer free of timing it doesn't enforce.

impl ToastState {
    pub fn push(&mut self, level: ToastLevel, message: impl Into<String>) {
        let msg = message.into();

        // 1. Coalesce, exact (level, message) match against non-dismissed entries.
        if let Some(t) = self.toasts.iter_mut()
            .find(|t| !t.dismissed && t.level == level && t.message == msg)
        {
            t.count = t.count.saturating_add(1);
            t.created = Instant::now();
            t.paused = None;
            t.paused_total = Duration::ZERO;
            return;
        }

        // 2. Cap, FIFO drain when exceeded.
        let visible = self.toasts.iter().filter(|t| !t.dismissed).count();
        if visible >= TOAST_MAX_VISIBLE {
            if let Some(oldest) = self.toasts.iter_mut()
                .filter(|t| !t.dismissed)
                .min_by_key(|t| t.created)
            {
                oldest.dismissed = true;
            }
        }

        // 3. Append. wrapping_add on u64 is fine: id collisions only arise after
        // 18 quintillion pushes against this single ToastState, not realistic.
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.toasts.push(Toast {
            id, level, message: msg, count: 1,
            created: Instant::now(),
            paused: None, paused_total: Duration::ZERO,
            dismissed: false,
        });
    }

    pub fn dismiss(&mut self, id: u64) {
        if let Some(t) = self.toasts.iter_mut().find(|t| t.id == id) {
            t.dismissed = true;
        }
    }

    pub fn pause(&mut self, id: u64) {
        if let Some(t) = self.toasts.iter_mut()
            .find(|t| t.id == id && !t.dismissed && t.paused.is_none())
        {
            t.paused = Some(Instant::now());
        }
    }

    pub fn resume(&mut self, id: u64) {
        if let Some(t) = self.toasts.iter_mut()
            .find(|t| t.id == id && !t.dismissed)
        {
            if let Some(start) = t.paused.take() {
                t.paused_total = t.paused_total.saturating_add(start.elapsed());
            }
        }
    }
}

/// Compute whether a toast has exceeded TOAST_DURATION, excluding paused intervals.
pub fn is_expired(t: &Toast, now: Instant) -> bool {
    if t.dismissed { return true; }
    let total = now.saturating_duration_since(t.created);
    let current_pause = t.paused.map_or(Duration::ZERO, |s| now.saturating_duration_since(s));
    let effective = total.saturating_sub(t.paused_total + current_pause);
    effective >= TOAST_DURATION
}

#[cfg(test)]
mod tests {
    // All tests construct ToastState::default() directly, no Dioxus runtime.
    // Covers: push_appends_when_empty, push_coalesces_exact_string_match,
    // push_resets_timer_on_coalesce, push_does_not_coalesce_across_levels,
    // push_drops_oldest_when_cap_exceeded, dismiss_marks_entry_dismissed,
    // pause_resume_accumulates_paused_total, is_expired_excludes_paused_time,
    // next_id_is_monotonic.
}
```

**Why exact-string match for dedupe:** the realistic producer is the warnings bridge, which pushes literal strings out of `AppState.warnings`. Collision-free identity is the message itself. A future producer that needs richer dedupe can add a key field, out of scope here.

**Why FIFO drain on cap exceed:** the latest problem is most relevant for the user; oldest visible toasts auto-dismiss to make room.

**Saturating arithmetic on `count`:** a long-running session can't panic on overflow. `next_id` uses `wrapping_add` for the same reason, the id space is uniqueness-over-the-current-Vec, not lifetime-of-process; collisions after wraparound are mathematically possible but require ~10^19 pushes.

#### `toast/queue.rs`, Signal wrapper

```rust
use std::time::Instant;
use dioxus::prelude::*;
use crate::toast::state::{Toast, ToastLevel, ToastState, is_expired};

#[derive(Clone, Copy)]
pub struct ToastQueue {
    pub(crate) state: Signal<ToastState>,
}

impl ToastQueue {
    pub fn push(&self, level: ToastLevel, message: impl Into<String>) {
        self.state.write().push(level, message)
    }
    pub fn dismiss(&self, id: u64)  { self.state.write().dismiss(id) }
    pub fn pause(&self, id: u64)    { self.state.write().pause(id) }
    pub fn resume(&self, id: u64)   { self.state.write().resume(id) }

    /// Snapshot of non-expired toasts at `now`; used by the viewport on each tick.
    pub(crate) fn visible(&self, now: Instant) -> Vec<Toast> {
        self.state.read().toasts.iter()
            .filter(|t| !is_expired(t, now))
            .cloned()
            .collect()
    }
}
```

`ToastQueue::new()` is intentionally absent, `Signal::new()` outside a hook leaks per Dioxus 0.7 docs (see `dioxus-signals/src/signal.rs:30-52`). Construction lives in `app_root`'s body via `use_signal(ToastState::default)` (see "Wiring in `app.rs`" below).

#### `toast/viewport.rs`, rendering and timers

```rust
use std::time::{Duration, Instant};
use dioxus::prelude::*;
use crate::toast::state::{Toast, ToastLevel};
use crate::toast::queue::ToastQueue;

#[component]
pub(crate) fn ToastViewport() -> Element {
    let queue = use_context::<ToastQueue>();

    // 4Hz "now" Signal drives expiration GC. Reading `now_signal` in the body
    // makes each tick produce a re-render; 250 ms is far coarser than per-frame
    // and an order of magnitude finer than the CSS fade-out duration, so cost
    // is negligible.
    let mut now_signal = use_signal(Instant::now);
    use_future(move || async move {
        let mut t = tokio::time::interval(Duration::from_millis(250));
        loop {
            t.tick().await;
            now_signal.set(Instant::now());
        }
    });
    let now = *now_signal.read();
    let toasts = queue.visible(now);

    let polite: Vec<&Toast> = toasts.iter()
        .filter(|t| matches!(t.level, ToastLevel::Info | ToastLevel::Success))
        .collect();
    let assertive: Vec<&Toast> = toasts.iter()
        .filter(|t| matches!(t.level, ToastLevel::Warning | ToastLevel::Error))
        .collect();

    rsx! {
        div { class: "if-toast-viewport if-toast-viewport--polite",
            role: "status", aria_live: "polite",
            for t in polite { ToastItem { key: "{t.id}", toast: t.clone() } }
        }
        div { class: "if-toast-viewport if-toast-viewport--assertive",
            role: "alert", aria_live: "assertive",
            for t in assertive { ToastItem { key: "{t.id}", toast: t.clone() } }
        }
    }
}

#[component]
fn ToastItem(toast: Toast) -> Element {
    let queue = use_context::<ToastQueue>();
    let id = toast.id;
    let level_class = match toast.level {
        ToastLevel::Info    => "if-toast--info",
        ToastLevel::Success => "if-toast--success",
        ToastLevel::Warning => "if-toast--warning",
        ToastLevel::Error   => "if-toast--error",
    };

    rsx! {
        div {
            class: "if-toast {level_class}",
            tabindex: "0",
            onmouseenter: move |_| queue.pause(id),
            onmouseleave: move |_| queue.resume(id),
            onfocusin:    move |_| queue.pause(id),
            onfocusout:   move |_| queue.resume(id),
            onkeydown:    move |e| if e.key() == Key::Escape { queue.dismiss(id); },

            // accent bar, pure CSS via .if-toast--{level}::before
            // icon, Icon enum lookup per level
            // message, toast.message
            // count badge, rendered when toast.count > 1: " ×{count}"
            // close button, aria-label="Dismiss", onclick → queue.dismiss(id)
        }
    }
}
```

**Why two viewport divs:** single-region ARIA can only declare one live verb. Splitting into `polite` (info/success) and `assertive` (warning/error) lets the AT pick correct delivery per level without tagging each toast item individually (which would announce the chrome, icon, count badge, alongside the message). `aria-atomic` is omitted because `false` is the default for `aria-live`.

**Tick mechanism:** `use_signal(Instant::now)` + `use_future` over `use_resource` is deliberate, the Signal makes the "what causes re-render" cause visible at the read site (`*now_signal.read()`), and `use_future` is the standard place for a tokio interval driving a side-effect Signal write.

**Fade-out:** handled entirely in `toast.css` via `transition: opacity 400ms` on `.if-toast`. The viewport stops including dismissed/expired toasts in its snapshot; CSS animates the unmount via `transitionend` semantics. No Rust-side fade timer.

**`pointer-events`:** the `.if-toast-viewport` divs are `pointer-events: none` so they don't block clicks on the shell when no toasts are visible; individual `.if-toast` items are `pointer-events: auto`.

**Z-stack:** viewport is rendered as a sibling of `PlaceholderShell` (and any future shell), not nested inside. `position: fixed` with a high `z-index` token (`--z-toast`, added in F4's CSS) keeps toasts above all shell content. Toasts intentionally also render above an open `<dialog>` (a toast surfacing during a dialog must remain visible).

#### `toast/warnings_bridge.rs`, production subscriber

```rust
use dioxus::prelude::*;
use crate::context::AppContext;
use crate::toast::{ToastLevel, ToastQueue};

/// Returns a closure suitable for `use_effect`. Watches `ctx.meta` for new
/// tail entries on `warnings` and pushes them as Warning-level toasts.
///
/// Re-runs whenever `ctx.meta` changes (any field, not just warnings), the
/// length-diff guard makes this idempotent.
///
/// `Signal<T>` is `Copy`; the closure rebinds `last_seen` as `mut` to match
/// the same shape used by the F1 polling task in `bridge.rs`.
pub(crate) fn install_warnings_bridge(
    ctx: AppContext,
    toasts: ToastQueue,
    last_seen: Signal<usize>,
) -> impl FnMut() + 'static {
    move || {
        let meta = ctx.meta.read();
        let len = meta.warnings.len();
        let mut seen = last_seen;
        let last = *seen.peek();
        if len > last {
            for msg in &meta.warnings[last..] {
                toasts.push(ToastLevel::Warning, msg.clone());
            }
            seen.set(len);
        } else if len < last {
            // Engine cleared/reset warnings, re-baseline.
            seen.set(len);
        }
    }
}
```

**Why `use_effect` instead of a polling task:** the F1 polling task already produces `ctx.meta.set(new_meta)` only when `PartialEq` differs (see `bridge.rs`). A `use_effect` reading `ctx.meta` re-runs strictly on those changes, fewer wake-ups than a yield-loop, no manual `tokio::interval`. Auto-cancels on scope teardown like all hooks.

**Re-run amplification:** reading `ctx.meta` subscribes to the entire `MetaSnapshot`; any field change re-runs the effect (engine_status flip, current_mode rename, profile_path change, warnings append). The length-diff guard makes spurious re-runs idempotent. Acceptable given the polling task's `PartialEq` gate already throttles writes.

#### Wiring in `app.rs`

```rust
pub(crate) fn app_root() -> Element {
    // … F1/F2/F3 setup unchanged: AppContext, polling, tray bridge, start-minimized …

    // F4: ToastQueue context, Signal lives in app_root's scope, mirroring the
    // F1 AppContext pattern (use_signal in body, struct holds the Signal handle,
    // context provider closure runs once via the underlying use_hook). Calling
    // Signal::new() outside a hook leaks per dioxus-signals/src/signal.rs:30-52,
    // so use_signal is mandatory here.
    let toast_state = use_signal(ToastState::default);
    let toast_queue = ToastQueue { state: toast_state };
    use_context_provider(|| toast_queue);

    // F4: warnings bridge, reads ctx.meta, pushes new tail entries as Warning
    // toasts. last_seen initializes from peek() so first run is a no-op even if
    // warnings accumulated before mount.
    let last_seen = use_signal(|| ctx.meta.peek().warnings.len());
    use_effect(toast::install_warnings_bridge(ctx.clone(), toast_queue, last_seen));

    rsx! {
        ThemeProvider {
            ToastViewport {}      // overlay layer, position: fixed, sibling to shell
            PlaceholderShell {}
        }
    }
}
```

### Dialog primitives

#### `components/dialog.rs`, compound API

```rust
use dioxus::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};

static DIALOG_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Shared state passed via context. All ids are eagerly computed by DialogRoot
/// at mount; children read them but never write, avoids the "child-writes-Signal-
/// during-render-after-parent-already-read-it" race that produces a stale
/// aria-labelledby on first paint.
#[derive(Clone)]
struct DialogState {
    open: Signal<bool>,
    dialog_id: String,
    title_id: String,
    desc_id: String,
    onclose: EventHandler<()>,
    dismissible: bool,
    close_on_backdrop_click: bool,
}

#[component]
pub fn DialogRoot(
    open: Signal<bool>,
    onclose: EventHandler<()>,
    #[props(default = true)]  dismissible: bool,
    #[props(default = false)] close_on_backdrop_click: bool,
    #[props(default)]         class: Option<String>,
    children: Element,
) -> Element { /* see below */ }

#[component]
pub fn DialogTitle(children: Element) -> Element       // <h2 id={state.title_id}>
#[component]
pub fn DialogDescription(children: Element) -> Element // <p id={state.desc_id}>
#[component]
pub fn DialogBody(children: Element) -> Element        // scrollable region
#[component]
pub fn DialogFooter(children: Element) -> Element      // action-row, right-aligned
```

**`DialogRoot` body sketch:**

```rust
// Eagerly compute all ids once, before first render. use_hook initializer runs
// during the parent's render, children that consume DialogState see fully-
// populated ids on their very first render, so aria-labelledby/aria-describedby
// resolve correctly on the initial showModal() call.
let state = use_hook(|| {
    let n = DIALOG_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dialog_id = format!("if-dialog-{n}");
    let title_id  = format!("{dialog_id}-title");
    let desc_id   = format!("{dialog_id}-desc");
    DialogState {
        open, dialog_id, title_id, desc_id,
        onclose, dismissible, close_on_backdrop_click,
    }
});
use_context_provider(|| state.clone());

// Drive showModal()/close() on `open` change. use_effect runs after DOM commit
// so getElementById is guaranteed to find the <dialog>.
let id_for_open = state.dialog_id.clone();
use_effect(move || {
    let action = if *open.read() { DIALOG_OPEN_JS } else { DIALOG_CLOSE_JS };
    let _ = document::eval(&format!("{action}({id_for_open:?})"));
});

// Attach `cancel` listener once after first DOM commit. Must NOT be in use_hook
// (which runs before commit and would race the getElementById lookup), see
// dioxus-core/src/scheduler.rs:75-77 for the post-commit ordering guarantee.
let id_for_cancel = state.dialog_id.clone();
let dismissible_now = state.dismissible;
let mut attached = use_signal(|| false);
use_effect(move || {
    if *attached.peek() { return; }
    let _ = document::eval(
        &format!("{DIALOG_ATTACH_CANCEL_JS}({id_for_cancel:?}, {dismissible_now})")
    );
    attached.set(true);
});

let combined = merge_class("if-dialog", "", class.as_deref());

rsx! {
    dialog {
        id: "{state.dialog_id}",
        class: "{combined}",
        aria_labelledby:  "{state.title_id}",
        aria_describedby: "{state.desc_id}",
        onclose: move |_| { open.set(false); onclose.call(()); },
        onclick: move |evt| {
            if !state.close_on_backdrop_click { return; }
            // Backdrop click only, gate on target == current_target so an
            // inner click that bubbled up (despite stop_propagation) cannot
            // accidentally close. Implementer: pick the right Dioxus 0.7 event
            // API for this comparison; menu.rs's onkeydown delegation pattern
            // is the local precedent for working with target/currentTarget.
            if !is_backdrop_click(&evt) { return; }
            open.set(false);
            onclose.call(());
        },
        div {
            class: "if-dialog__panel",
            onclick: move |evt| evt.stop_propagation(),
            {children}
        }
    }
}
```

**JS shims:**

```rust
const DIALOG_OPEN_JS: &str = r#"
(function(id) {
    var d = document.getElementById(id);
    if (d && !d.open) d.showModal();
})
"#;

const DIALOG_CLOSE_JS: &str = r#"
(function(id) {
    var d = document.getElementById(id);
    if (d && d.open) d.close();
})
"#;

const DIALOG_ATTACH_CANCEL_JS: &str = r#"
(function(id, dismissible) {
    var d = document.getElementById(id);
    if (!d) return;
    d.addEventListener('cancel', function(e) {
        if (!dismissible) e.preventDefault();
    });
})
"#;
```

**`DialogTitle` / `DialogDescription`:** each reads `dialog_id`-derived ids out of the parent-owned `DialogState` and renders with that id. No writes during render, no parent-child ordering bug.

```rust
#[component]
pub fn DialogTitle(children: Element) -> Element {
    let state = use_context::<DialogState>();
    rsx! { h2 { id: "{state.title_id}", class: "if-dialog__title", {children} } }
}

#[component]
pub fn DialogDescription(children: Element) -> Element {
    let state = use_context::<DialogState>();
    rsx! { p { id: "{state.desc_id}", class: "if-dialog__desc", {children} } }
}
```

`DialogBody` and `DialogFooter` are pure layout wrappers with no ARIA wiring.

**Dangling-reference contract:** `DialogRoot` always emits `aria-labelledby="if-dialog-N-title"` and `aria-describedby="if-dialog-N-desc"`. If a consumer omits `DialogTitle` or `DialogDescription`, the attribute points at no element. Browsers and assistive tech ignore dangling references silently. The contract is: omit a child if and only if the dialog genuinely has no title or no description, otherwise an a11y audit will rightly flag the dangling reference. F4's only consumer (`DirtyConfirmDialog`) always renders both, so the contract is upheld at F4 boundary.

**Native behavior we rely on:**

- **Modal trap:** `showModal()` traps Tab inside the dialog.
- **Inert background:** the rest of the document becomes inert (cannot be focused or interacted with).
- **`aria-modal`:** browser sets it to `true` automatically when in modal mode.
- **Initial focus:** browser focuses the first focusable element inside the dialog (or the dialog itself if none). Consumers control this by ordering, for `DirtyConfirmDialog`, Cancel is first in document order.
- **Focus restore:** on `close()`, focus returns to the element that was focused before `showModal()` was called.

**ESC handling:**

- Native `<dialog>` fires a `cancel` event on ESC. Our shim either lets it propagate (`dismissible: true`, default) or `preventDefault`s it (`dismissible: false`).
- If `cancel` is not prevented, the browser fires `close` → our `onclose` Rust handler fires.

**Backdrop click:**

- Native `<dialog>` does NOT close on backdrop click by default. We add it via `close_on_backdrop_click: bool` (default `false`).
- Detection: a click on the `<dialog>` element with `target === currentTarget` means the user hit the backdrop (the inner `.if-dialog__panel` stops propagation). Our `onclick` on `<dialog>` fires only on backdrop clicks because the panel swallows inner clicks.

#### What we don't ship

- **No close-button slot built into `DialogRoot`.** Consumers add their own button inside `DialogFooter`/`DialogBody` and call `open.set(false)` (or `state.open` via context if a `DialogClose` becomes useful, for F4 we don't need it).
- **No multi-dialog stacking management.** Native `<dialog>` handles one-at-a-time; opening a second `showModal()` while one is open is browser-defined behavior. F4 only ever opens one at a time.
- **No animation primitives.** `impeccable:frontend-design` may add fade/scale-in transitions during implementation via CSS only (`@starting-style` / `transition-behavior: allow-discrete`). Not a F4 design decision.

### `DirtyConfirmDialog` (patterns/dirty_confirm.rs)

```rust
use dioxus::prelude::*;

use crate::components::{
    Button, ButtonVariant,
    DialogBody, DialogDescription, DialogFooter, DialogRoot, DialogTitle,
};

#[derive(Clone, PartialEq, Props)]
pub struct DirtyConfirmDialogProps {
    /// Controlled open state. The component flips this to `false` on every
    /// resolution path (Cancel/Discard/Save) and fires the matching callback.
    pub open: Signal<bool>,

    /// Title, defaults to "Unsaved Changes".
    #[props(default)] pub title: Option<String>,
    /// Description, defaults to "You have unsaved changes. What would you like to do?"
    #[props(default)] pub message: Option<String>,
    /// Save button label, defaults to "Save". Future consumers may pass
    /// "Save & Switch", "Save & Close", etc.
    #[props(default)] pub save_label: Option<String>,

    pub oncancel:  EventHandler<()>,
    pub ondiscard: EventHandler<()>,
    pub onsave:    EventHandler<()>,

    #[props(default)] pub class: Option<String>,
}

#[component]
pub fn DirtyConfirmDialog(props: DirtyConfirmDialogProps) -> Element {
    let title      = props.title.as_deref().unwrap_or("Unsaved Changes");
    let message    = props.message.as_deref()
        .unwrap_or("You have unsaved changes. What would you like to do?");
    let save_label = props.save_label.as_deref().unwrap_or("Save");

    let mut open = props.open;
    let cancel  = props.oncancel;
    let discard = props.ondiscard;
    let save    = props.onsave;

    rsx! {
        DialogRoot {
            open: open,
            // ESC routes to Cancel, matches default-focus and safe-default semantics.
            onclose: move |_| { open.set(false); cancel.call(()); },
            dismissible: true,
            close_on_backdrop_click: false,
            class: props.class,

            DialogTitle { "{title}" }
            DialogDescription { "{message}" }
            DialogBody {} // empty, Description carries the body content
            DialogFooter {
                // Cancel first in document order → receives showModal()'s default focus.
                Button {
                    variant: ButtonVariant::Secondary,
                    onclick: move |_| { open.set(false); cancel.call(()); },
                    "Cancel"
                }
                Button {
                    variant: ButtonVariant::Danger,
                    onclick: move |_| { open.set(false); discard.call(()); },
                    "Discard"
                }
                Button {
                    variant: ButtonVariant::Primary,
                    onclick: move |_| { open.set(false); save.call(()); },
                    "{save_label}"
                }
            }
        }
    }
}
```

**Behavior contract:**

- Controlled `open` Signal, caller flips to `true` to show; component flips to `false` on every resolution (Cancel/Discard/Save).
- ESC routes to `oncancel` (matches Cancel button).
- `close_on_backdrop_click: false`, destructive-confirmation dialogs should not close on a stray click outside the panel.
- Default copy is generic; consumers override `title` / `message` / `save_label` for context-specific phrasing (e.g., F8 might pass `save_label: "Save & Switch"`).

**Why a thin wrapper is the right shape:**

1. **Standardizes action layout.** Cancel/Discard/Save in fixed order, default focus on Cancel, Discard styled as `Danger`. Without it, every consumer invents their own button order, that's how UX drift happens.
2. **Test surface.** Gallery exercises the full flow once, not three times.

It deliberately doesn't try to manage the "pending target" state, that lives in the consumer (`Option<InputAddress>` for F8, `Option<ProfilePath>` for F14, etc.). Egui's pattern was right; we're keeping it.

---

## Gallery additions (`examples/component_gallery.rs`)

Three new sections:

**Section: Toasts.** Five buttons:
- `Push Info`, `Push Success`, `Push Warning`, `Push Error`, one push each; demonstrates per-level styling and ARIA delivery.
- `Push spam (×10)`, calls `push(Warning, "Spammy")` ten times; renders one toast with `×10` count badge.
- `Push 7 distinct`, pushes 7 unique messages quickly; verifies max-visible cap of 5 with FIFO drain.

Toast viewport is mounted at the top of the gallery so toasts overlay example content as they would in production.

**Section: Dialog primitives.** Four buttons opening four dialogs:
- **Basic**, `DialogRoot { Title; Description; Footer { Close button } }`. ESC dismisses.
- **Non-dismissible**, `dismissible: false`. ESC and click-outside do nothing; only the explicit Close button resolves it.
- **Close-on-backdrop**, `close_on_backdrop_click: true`. Clicking outside the panel closes the dialog; verifies the backdrop-click detection.
- **Scrollable body**, long content inside `DialogBody` to verify scroll region works.

**Section: DirtyConfirmDialog.** Two buttons:
- **Switch input (dirty)**, opens with default copy. Three captions next to it light up when Cancel/Discard/Save fires (proving each callback is reachable).
- **With custom copy**, opens the same dialog with overridden `title` / `message` / `save_label`.

The gallery does NOT exercise the warnings bridge, that requires a real engine. Manual end-to-end verification (`cargo run --no-default-features --features gui-dioxus` against an engine that emits a warning) covers it.

---

## Acceptance criteria

- [ ] `cargo build -p inputforge-gui-dx` introduces no new warnings vs the baseline measured at the start of F4 implementation. The implementation plan must record the baseline count from `cargo build -p inputforge-gui-dx` on `main` HEAD before any F4 file is created; the final acceptance pass compares against that recorded number.
- [ ] `cargo build -p inputforge-app --no-default-features --features gui-dioxus` succeeds.
- [ ] `cargo build -p inputforge-app` (default egui) unchanged.
- [ ] `cargo test -p inputforge-gui-dx` passes, pure-function tests on `ToastState` (push, coalesce, cap, dismiss, pause/resume, expiration). Tests construct `ToastState::default()` directly; no Dioxus runtime required.
- [ ] **Gallery, Toasts:** `dx serve --example component_gallery` opens; each level button pushes a toast with the correct accent and level icon; `Push spam` produces a single toast with `×10`; `Push 7 distinct` shows exactly 5 visible toasts (oldest two auto-dismissed); close button (`x`) and ESC-while-focused both dismiss; hover and focus pause the timer; tab key moves focus into and through the toast.
- [ ] **Gallery, Dialogs:** all four primitive demos open via `showModal()`; focus moves into the dialog on open; Tab cycles inside; ESC closes the dismissible ones and is no-op on the non-dismissible one; backdrop click does nothing on the basic/non-dismissible/scrollable variants and closes the close-on-backdrop variant; focus returns to the trigger button on close.
- [ ] **Gallery, DirtyConfirmDialog:** initial focus lands on Cancel; ESC fires `oncancel`; each button fires the right callback exactly once; the open Signal goes back to `false` after every resolution.
- [ ] **ARIA contracts verified via DevTools accessibility inspector:**
  - Toast viewport: two regions, one `role="status" aria-live="polite"` (info+success), one `role="alert" aria-live="assertive"` (warning+error).
  - Each toast item has `tabindex="0"` and the close button has `aria-label="Dismiss"`.
  - Dialog: `aria-modal` is `true` while open (browser-set), `aria-labelledby` points at the `DialogTitle` id, `aria-describedby` points at `DialogDescription` when present and is absent otherwise.
- [ ] **Live engine warnings produce toasts.** With `gui-dioxus` running against a real engine, an emitted warning (e.g., HidHide unavailable) appears as a Warning-level toast within ~1 polling tick; identical repeats coalesce into a `×N` count.
- [ ] **F3 ARIA wrapper unchanged.** F3's `<span role="status" aria-live="polite">` around the engine-status badge in `StatusBarView` stays intact. F4 does NOT push toasts on engine-status flips.
- [ ] **Z-stack.** With `PlaceholderShell` rendered, an open dialog appears above the shell; toasts appear above both the shell and the dialog.
- [ ] **Pointer-events.** When no toasts are visible, clicks on the area where the viewport sits pass through to the underlying shell (verify by clicking through to a Tab in the placeholder).

---

## Test strategy

- **Unit tests** in `crates/inputforge-gui-dx/src/toast/state.rs` under `#[cfg(test)] mod tests`. The pure-data layer means tests construct `ToastState::default()` directly, no runtime, no Signals, no helpers.
  - `push_appends_when_empty`
  - `push_coalesces_exact_string_match`, count goes to 2, no new entry
  - `push_resets_timer_on_coalesce`
  - `push_does_not_coalesce_across_levels`, same string, different level → two entries
  - `push_drops_oldest_when_cap_exceeded`
  - `dismiss_marks_entry_dismissed`
  - `pause_resume_accumulates_paused_total`
  - `is_expired_excludes_paused_time`
  - `next_id_is_monotonic`
- **Pure-function test** on dialog id generation, `DIALOG_ID_COUNTER` produces monotonic unique values formatted as `if-dialog-{n}`; `DialogState` derives `if-dialog-{n}-title` and `if-dialog-{n}-desc` from the dialog id.
- **Manual gallery interaction pass** documented in the acceptance bullets above.
- **No CI matrix change.** F1's `--no-default-features --features gui-dioxus build` covers F4 build verification.
- **Process check (not a code-verifiable bullet):** `impeccable:frontend-design` should be invoked early in F4 implementation with brief scoped to "toast visual treatment + dialog visual treatment, scoped to the new primitives." Its output must be committed before component CSS finalization.

---

## Risks

- **Native `<dialog>` quirks across WebView2 versions.** `<dialog>` and `showModal()` have shipped in Edge/Chromium since 2022; the inputforge minimum platform (Windows 10 20H1+) bundles a WebView2 well past that bar. Risk: a future WebView2 minor that changes `cancel`/`close` event timing or focus-restore semantics. Mitigation: behavior covered by manual gallery checks; if it ever breaks, the JS shim is the single point of churn.
- **`use_effect` re-run amplification on warnings bridge.** Reading `ctx.meta` inside `use_effect` subscribes to the entire `MetaSnapshot`; any field change re-runs the effect. The length-diff guard makes spurious re-runs idempotent, no double-pushes, but the effect runs more often than strictly necessary. Acceptable given the polling task already gates on `PartialEq` so re-runs only happen on actual snapshot changes (a few per second at most, often zero).
- **`document::eval` race on rapid `open` toggle.** Setting `open` true → false in a single render frame fires two evals back-to-back. Native `<dialog>` is robust against this (idempotent `showModal` / `close`), and the JS guards (`if (d && !d.open)` / `if (d && d.open)`) make it a no-op if state is already correct.
- **Toast viewport rendering above dialog content.** Intentional, but worth flagging because it differs from some toast libraries. If `impeccable:frontend-design` or F15 audit decides toasts should defer to an open dialog, the change is a single CSS `z-index` token swap.
- **Gallery doesn't exercise the warnings bridge.** The bridge requires a real engine. We accept manual end-to-end verification (`cargo run --no-default-features --features gui-dioxus` against an engine that emits a warning) as the bridge test.
- **Hover-pause vs pointer-events on the viewport.** The two stacked viewport divs cover the top-right corner with `position: fixed`; an empty viewport must not block clicks on the underlying shell. Mitigation: `pointer-events: none` on `.if-toast-viewport`, `pointer-events: auto` on `.if-toast`.
- **Dialog focus restore depends on browser behavior.** If the trigger element is removed from the DOM between `showModal()` and `close()`, focus restore fails silently (focus lands on `<body>`). For F4's call sites this is not a concern (triggers are stable). F8/F14 should keep dialog triggers stable across the open lifetime.
- **`use_effect` warnings-bridge fires on first mount.** `last_seen` is initialized to `ctx.meta.peek().warnings.len()` so the first run is a no-op even if warnings were already accumulated before mount. Verified by the length-diff guard.
- **Dangling `aria-labelledby` / `aria-describedby` if a consumer omits `DialogTitle`/`DialogDescription`.** `DialogRoot` always emits the attribute, so omitting the child leaves the reference pointing at no element. Browsers and assistive tech ignore dangling references silently, but a future a11y audit will flag them. Mitigation: F4's only consumer (`DirtyConfirmDialog`) always renders both children; F8/F11/F13/F14 should keep the same discipline unless the dialog genuinely has no title or description.

---

## Open questions (inherited, not decided here)

- **Testing story for the Dioxus GUI**, parent-plan open question; F4 adds ToastQueue unit tests but commits to nothing on rendering automation.
- **Exact Dioxus and `dioxus-cli` versions**, pinned at implementation start via `latest-packages` against <https://crates.io>.
- **Toast → engine-status announcement supersession**, F15 audit may decide to drop F3's `aria-live` wrapper and route engine-status flips through toasts. Out of scope for F4.
- **Light-theme values** for toast accents and dialog backdrop, out of scope for the whole rewrite until needed.

---

## Files

**Created:**

```
crates/inputforge-gui-dx/src/toast/mod.rs
crates/inputforge-gui-dx/src/toast/state.rs                      # ToastState pure data + ToastLevel + Toast; push/coalesce/cap/dismiss/pause/resume/is_expired (+ unit tests, no runtime)
crates/inputforge-gui-dx/src/toast/queue.rs                      # ToastQueue { state: Signal<ToastState> }, Signal wrapper, delegates every method
crates/inputforge-gui-dx/src/toast/viewport.rs                   # ToastViewport, ToastItem
crates/inputforge-gui-dx/src/toast/warnings_bridge.rs            # install_warnings_bridge
crates/inputforge-gui-dx/src/components/dialog.rs                # DialogRoot/Title/Description/Body/Footer (+ id-generation unit test)
crates/inputforge-gui-dx/src/patterns/mod.rs
crates/inputforge-gui-dx/src/patterns/dirty_confirm.rs           # DirtyConfirmDialog
crates/inputforge-gui-dx/assets/components/dialog.css
crates/inputforge-gui-dx/assets/toast/toast.css
```

**Modified:**

```
crates/inputforge-gui-dx/src/lib.rs                              # pub mod toast; pub mod patterns; expose Dialog* and DirtyConfirmDialog
crates/inputforge-gui-dx/src/app.rs                              # install ToastQueue context, render ToastViewport, install warnings bridge
crates/inputforge-gui-dx/src/components/mod.rs                   # re-export Dialog primitives
crates/inputforge-gui-dx/src/theme/mod.rs                        # mount dialog.css and toast.css stylesheets
crates/inputforge-gui-dx/assets/theme/tokens.css                 # add --z-toast token (above existing F2 z-index tokens)
crates/inputforge-gui-dx/examples/component_gallery.rs           # add Toasts, Dialogs, DirtyConfirmDialog sections
crates/inputforge-gui-dx/README.md                               # document ToastQueue, dialog primitives, dirty-confirm pattern
```

**Reused (not modified) from F1/F2/F3:**

- `crates/inputforge-gui-dx/src/context.rs`, `AppContext`, snapshots
- `crates/inputforge-gui-dx/src/bridge.rs`, polling task
- `crates/inputforge-gui-dx/src/shell/`, `PlaceholderShell`, `StatusBarView`
- `crates/inputforge-gui-dx/src/tray/`, `src/lifecycle/`, tray bridge
- All F2 components except `mod.rs` (re-exports updated)

---

## Next steps

1. Commit this spec to git.
2. Invoke `superpowers:writing-plans` to produce a step-by-step implementation plan with TDD-friendly checkpoints. The plan should sequence:
   - `latest-packages` verification of Dioxus / `dioxus-cli` versions.
   - `impeccable:frontend-design` invocation early (after structure scaffold, before component CSS finalization), brief scoped to toast + dialog visual treatment.
   - `ToastQueue` pure-data module + unit tests (TDD).
   - `ToastViewport` component + gallery section (verification continuity with F2/F3).
   - Dialog compound primitives + gallery section.
   - `DirtyConfirmDialog` + gallery section.
   - Warnings bridge + manual end-to-end verification against a real engine.
   - End-to-end manual acceptance pass on the gallery and the running app.
