# F4 — Toast & Dialog Infrastructure — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `docs/superpowers/specs/2026-04-26-f4-toast-dialog-design.md`

**Goal:** Ship the fourth and final foundation feature in `crates/inputforge-gui-dx`: a global toast queue (level + dedupe + cap + auto-dismiss + pause + keyboard dismissal), a native-`<dialog>`-backed compound dialog primitive, a presentational `DirtyConfirmDialog` reusable, and one production subscriber wiring `MetaSnapshot.warnings` into the toast queue. After F4, all four foundation features (F1–F4) are in place and `--features gui-dioxus` covers warning visibility at parity with the egui crate.

**Architecture:** The toast queue is split into three layers: pure data (`ToastState` — runtime-free, fully unit-testable), a thin `Signal<ToastState>` wrapper (`ToastQueue`), and a single component (`ToastViewport`) that drives the 4 Hz expiration GC, owns the per-level ARIA live regions, and re-renders on each tick. The dialog primitive is a compound API (`DialogRoot/Title/Description/Body/Footer`) over the native `<dialog>` element + `showModal()` (focus trap, ESC, inert background, focus restore — all browser-handled); a tiny JS shim drives `showModal`/`close` and the `cancel` listener via `document::eval`. `DirtyConfirmDialog` is a thin presentational composer of those primitives with default copy and Cancel/Discard/Save buttons (Cancel-first for default focus). `install_warnings_bridge` is a `use_effect` closure that observes `ctx.meta.warnings` and pushes new tail entries as Warning-level toasts, idempotent under spurious re-runs.

**Tech Stack:** Rust 2024 / rustc 1.85, Dioxus 0.7.6 (desktop / wry / WebView2), `tokio::time::interval` (existing dep, used by viewport tick), F2 design-system primitives (Button + tokens), F3 placeholder shell + status-bar view (untouched). No new crate dependencies.

---

## Context

F1 shipped the state-bridge scaffold; F2 shipped the design system + 17 primitives + gallery harness; F3 shipped the placeholder shell, tray bridge, and lifecycle. F4 closes the foundation: toast infrastructure + dialog primitive + dirty-confirm pattern + the warnings bridge that prevents a regression at F14's default-feature flip.

**Why this is worth a plan rather than just "hack it in":**

- **Layering matters.** Putting all toast logic in a `#[component]` ties tests to a Dioxus runtime. Splitting into pure-data (`state.rs`) + Signal wrapper (`queue.rs`) + viewport (`viewport.rs`) lets the unit tests run as plain `#[test]` functions, no harness, no scope. The Signal-wrapper convention from F1 (`use_signal` inside `app_root`) carries forward.
- **The dialog id wiring is delicate.** `aria-labelledby`/`aria-describedby` must point at real ids on the very first render or AT users get a dangling-reference flash. Eagerly computing `dialog_id`/`title_id`/`desc_id` in `DialogRoot`'s `use_hook` initializer (which runs before children render) avoids the parent-child ordering bug.
- **Warnings bridge re-run amplification is real.** Reading `ctx.meta` inside `use_effect` subscribes the entire `MetaSnapshot`. The length-diff guard makes it idempotent, but the wording in code matters — a wrong baseline on first mount would replay accumulated warnings as toasts (annoying, not unsafe). The plan locks the baseline shape.
- **The frontend-design invocation is mid-stream.** Stand the primitives up with stub CSS so the gallery renders, then run `impeccable:frontend-design` against a real screenshot, then commit revised CSS. Doing it before any RSX exists wastes the briefing; doing it at the end means visual rework piles up after manual acceptance.

Outcome at F4: `cargo run --no-default-features --features gui-dioxus` opens the F3 shell; pushing toasts (via gallery or a real engine warning) renders them top-right with per-level accents; ESC + close-button + auto-dismiss all work; opening a dialog traps focus and restores it on close; `DirtyConfirmDialog` exercises the Cancel/Discard/Save flow with Cancel-first default focus. `cargo run` (default features = egui) is byte-identical to today.

---

## Critical files to modify

All paths relative to `E:\Git\Perso\inputforge\` unless otherwise noted.

**Created (in `crates/inputforge-gui-dx/`):**

- `src/toast/mod.rs` — module re-exports (`pub use state::*; pub use queue::ToastQueue; pub(crate) use viewport::ToastViewport; pub(crate) use warnings_bridge::install_warnings_bridge`)
- `src/toast/state.rs` — `ToastLevel`, `Toast`, `ToastState`, `TOAST_DURATION`, `TOAST_MAX_VISIBLE`, `is_expired`; pure data + behavior with `#[cfg(test)] mod tests`
- `src/toast/queue.rs` — `ToastQueue { state: Signal<ToastState> }` Signal wrapper; delegates `push`/`dismiss`/`pause`/`resume`; `visible(now)` snapshot
- `src/toast/viewport.rs` — `ToastViewport` component (drives 4 Hz tick, splits into polite/assertive ARIA regions); `ToastItem` per-toast component with hover/focus/escape handling
- `src/toast/warnings_bridge.rs` — `install_warnings_bridge(ctx, queue, last_seen)` returns the `use_effect` closure
- `src/components/dialog.rs` — `DialogRoot`, `DialogTitle`, `DialogDescription`, `DialogBody`, `DialogFooter` compound; `DIALOG_ID_COUNTER`; `DIALOG_OPEN_JS` / `DIALOG_CLOSE_JS` / `DIALOG_ATTACH_CANCEL_JS`; id-generation `#[cfg(test)]` test
- `src/patterns/mod.rs` — `pub use dirty_confirm::DirtyConfirmDialog;`
- `src/patterns/dirty_confirm.rs` — `DirtyConfirmDialog` presentational component
- `assets/components/dialog.css` — `:modal`, `::backdrop`, panel surface, body scroll, footer alignment
- `assets/toast/toast.css` — viewport position, per-level accents, count badge, hover/focus styles, motion

**Modified (in `crates/inputforge-gui-dx/`):**

- `src/lib.rs` — add `mod toast;` (private) and `pub mod patterns;`; no public surface changes from the toast module — the queue is reached via `use_context::<ToastQueue>()` and ToastQueue itself is `pub use`'d through `toast::mod`. Concretely: add `pub use toast::{ToastLevel, ToastQueue};` so external consumers (gallery, future F8/F11/F13/F14 producers) can name the level enum and the queue type.
- `src/app.rs` — install `ToastQueue` context via `use_context_provider` adjacent to `AppContext`; install warnings bridge via `use_effect`; render `ToastViewport` as a sibling of `PlaceholderShell` inside `ThemeProvider`
- `src/components/mod.rs` — add `pub mod dialog;` + re-export `DialogRoot`, `DialogTitle`, `DialogDescription`, `DialogBody`, `DialogFooter`
- `src/theme/mod.rs` — add `Stylesheet` mounts for `dialog.css` and `toast.css` (toast goes last — overlay layer on top of all component CSS)
- `assets/tokens/elevation.css` — add `--z-toast` token (above the existing `1000` z-index used by Menu — pick `1100` to keep room for future `--z-dialog` if we ever want to swap order)
- `examples/component_gallery.rs` — install `ToastQueue` in `gallery_root`; add three sections: Toasts (5 buttons), Dialog primitives (4 buttons), DirtyConfirmDialog (2 buttons)
- `README.md` — document `ToastQueue`, dialog compound API, dirty-confirm pattern, warnings bridge

**Reused (do not modify):**

- `src/context.rs` — `AppContext`, `MetaSnapshot::warnings` (read-only consumer)
- `src/bridge.rs` — `spawn_polling_task` (existing; gates writes via `PartialEq`, so the warnings-bridge re-run is throttled at the source)
- `src/components/button.rs` — `Button { variant, onclick, ... }` (composed by `DirtyConfirmDialog` and the gallery sections)
- `src/components/icon.rs` + `src/icons/mod.rs` — `Icon { name: IconKind::Info | Warning | Error | Check }` for per-level glyphs (`Check` is the closest existing fit for Success — the icon set is already at parity)
- `src/components/menu.rs` — precedent for `document::eval`-driven JS shims; same shape applies to `dialog.rs`
- F2 tokens: `--color-info-bg`, `--color-success-bg`, `--color-warning-bg`, `--color-error-bg`, `--color-info`, `--color-live`, `--color-warning`, `--color-error`, `--shadow-3`, `--space-{1..6}`, `--radius-md`, `--font-sans`
- F3 `StatusBarView`'s `<span role="status" aria-live="polite">` wrapper — UNCHANGED. Spec confirms.

---

## Existing utilities to reuse

- **F2's `merge_class(base, variant, caller)`** in `src/components/mod.rs:49-62` — every new primitive (Dialog parts, ToastItem) uses this to honor the `class: Option<String>` caller-composition prop.
- **F2's component pattern** — sibling `.rs` + `.css` per primitive (`assets/components/<name>.css`), `asset!()` mounted from `theme/mod.rs`, `.if-<name>` BEM-ish class prefix. `assets/components/badge.css` is the canonical small-primitive template; `assets/components/menu.css` is the canonical eval-driven JS-paired template.
- **F2's `Icon` enum** — already includes `Info`, `Warning`, `Error`, and `Check` (the latter for Success). See `src/icons/mod.rs:30-33` — no new icons needed.
- **F2's `Button { variant: ButtonVariant::{Primary,Secondary,Danger}, onclick, ... }`** — `DirtyConfirmDialog` composes three of these in its footer.
- **F1's `AppContext` + `MetaSnapshot`** — `ctx.meta.read().warnings` is the bridge's input; `ctx.meta.peek().warnings.len()` is the safe (non-subscribing) baseline read.
- **F1's "Signal lives in `app_root`'s body" pattern** — see `src/app.rs:19-30`. `ToastQueue` follows it: `let toast_state = use_signal(ToastState::default); let toast_queue = ToastQueue { state: toast_state };`.
- **Dioxus 0.7's `document::eval`** — already used by `components/menu.rs:105,118` for the focus walker. Same call shape applies to `DIALOG_OPEN_JS`/`DIALOG_CLOSE_JS`/`DIALOG_ATTACH_CANCEL_JS`.
- **Dioxus 0.7's `use_future` + `tokio::time::interval`** — established 60 Hz pattern in `src/bridge.rs:16-19`. The viewport's 4 Hz GC tick reuses the same `tokio::time::interval` machinery (already in `[dependencies]`).
- **`AtomicU64` id counter** — `src/components/menu.rs:1-10` shows the pattern; `dialog.rs` mirrors it.

---

## Dioxus 0.7 / WebView2 / native-`<dialog>` footguns to heed

Surface these in the implementer's mind before they hit them:

- **`Signal::new()` outside a hook leaks.** See `dioxus-signals/src/signal.rs:30-52`. `ToastQueue` exposes `state: Signal<ToastState>` but has NO `::new()` — the Signal must be created via `use_signal(ToastState::default)` inside `app_root` (production wiring) or `gallery_root` (gallery). Same shape F1 uses for `MetaSnapshot`.
- **`use_context_provider` runs once per scope.** Calling it from inside `app_root`'s render body installs the `ToastQueue` for every descendant. Mirror the F1 `AppContext` pattern (`app.rs:31`).
- **`use_effect` subscribes to every signal it reads.** Reading `ctx.meta` inside `install_warnings_bridge` subscribes the entire `MetaSnapshot` (engine_status, current_mode, profile_name, profile_path, warnings). Spurious re-runs (engine status flip, mode rename) are made idempotent by the length-diff guard. Acceptable — the polling task already throttles via `PartialEq` (`bridge.rs:38-46`).
- **`Signal<T>` is `Copy`.** Pass by value into closures (`last_seen` Signal in the bridge); rebind as `mut` inside the closure if you need to call `set`. The F1 polling task does this exact dance — `bridge.rs:35-37`.
- **`use_future` over `use_resource` for a side-effect tick.** A tokio interval that writes a Signal is a side effect, not a value; `use_future` is the standard place for it. The viewport pattern: `let mut now_signal = use_signal(Instant::now); use_future(move || async move { let mut t = tokio::time::interval(Duration::from_millis(250)); loop { t.tick().await; now_signal.set(Instant::now()); } });` — re-reading `now_signal` in the body produces the re-render.
- **`document::eval` is fire-and-forget; the future returned must be discarded with `let _ =`.** Same as `menu.rs:105`. Errors propagate as eval-side JS exceptions, not Rust errors.
- **`use_hook(|| ...)` initializer runs during the parent's render — BEFORE children render.** This is why `DialogRoot` uses `use_hook` (not `use_signal`) to compute `dialog_id`/`title_id`/`desc_id`: children that consume `DialogState` see fully-populated ids on their very first render. `use_signal` would also work but `use_hook` is the right semantic ("compute once, never write") and avoids unnecessary subscription bookkeeping.
- **`use_effect` runs AFTER DOM commit.** That's the post-commit ordering guarantee from `dioxus-core/src/scheduler.rs`. Use `use_effect` (not `use_hook`) for `document::eval` calls that need the `<dialog>` element to exist (`getElementById` lookups).
- **Native `<dialog>` `cancel` event needs `preventDefault()` to suppress ESC dismissal.** Listener attached once on first commit via `DIALOG_ATTACH_CANCEL_JS`; the `dismissible` Rust prop is interpolated into the JS at attach time, so flipping `dismissible` after mount is **not supported**. F4 only ever opens dialogs with a stable `dismissible` value (DirtyConfirmDialog: `true`; gallery non-dismissible: `false`). Document this in the doc-comment on `DialogRoot`.
- **`<dialog>` does NOT close on backdrop click by default.** The Rust-side `onclick` on `<dialog>` fires only on backdrop clicks because the inner `.if-dialog__panel` calls `evt.stop_propagation()` on its own `onclick`. Detection: gate `close_on_backdrop_click` on the existing `evt.stop_propagation()` shape — no `target === currentTarget` comparison needed because the panel swallows inner clicks.
- **`asset!()` paths must start with `/`** and resolve relative to crate root. `/assets/toast/toast.css` and `/assets/components/dialog.css`.
- **`document::Stylesheet` mounts in render order.** `dialog.css` goes alongside other component CSS (after global, before `placeholder-shell.css`). `toast.css` goes LAST so its `position: fixed; z-index: var(--z-toast)` cascades over everything else — toasts always paint on top.
- **`tokio::time::interval` is already in the workspace.** No `Cargo.toml` change. The `time` feature is enabled in `[workspace.dependencies] tokio = { version = "1", default-features = false, features = ["rt", "time", "sync"] }` (`Cargo.toml:28`).
- **`Instant::now()` works inside Dioxus runtime.** Same `std::time::Instant` used elsewhere; no platform-specific gotcha.
- **`onfocusin`/`onfocusout` (NOT `onfocus`/`onblur`) for hover-pause.** Native `focus`/`blur` don't bubble; `focusin`/`focusout` do, which matters because the toast's children (close button) shouldn't trigger separate pause/resume cycles. Dioxus 0.7 supports both; verify the lowercase event-name spelling matches the Dioxus event surface during Task 5 (`cargo check` will catch typos).
- **`Key::Escape` matches the literal ESC key in Dioxus 0.7.** Same idiom `menu.rs:94` uses.
- **`document::eval` happens inside the WebView2 process.** Subprocess startup is automatic but rapid open/close cycles in tests would race; F4 has no such test (only manual gallery interaction).

---

## Phase Overview

- **Phase 0** (Task 1) — Pre-flight: dependency-version verification, baseline warning count, F2 token compatibility check.
- **Phase 1** (Tasks 2–5) — `ToastState` pure-data layer with TDD. Pure `#[test]` functions, no Dioxus runtime.
- **Phase 2** (Tasks 6–8) — `ToastQueue` Signal wrapper, `ToastViewport` rendering with stub CSS, `--z-toast` token, ThemeProvider stylesheet mount.
- **Phase 3** (Task 9) — Gallery wiring for Toasts (queue installation in `gallery_root` + 5-button section).
- **Phase 4** (Tasks 10–11) — `Dialog` compound primitive with stub CSS + ThemeProvider mount; gallery section.
- **Phase 5** (Tasks 12–13) — `DirtyConfirmDialog` patterns module; gallery section.
- **Phase 6** (Tasks 14–15) — `impeccable:frontend-design` invocation against rendered gallery; apply revised `toast.css` + `dialog.css`.
- **Phase 7** (Tasks 16–17) — Production wiring: `warnings_bridge` module, `app_root` installation.
- **Phase 8** (Tasks 18–19) — README updates + final acceptance pass against the gallery and a live engine.

---

## Task 1: Pre-flight verification

Three quick checks before any new file is created. Each is a 1–3 minute confirmation; they exist to lock the baseline so the final acceptance pass has something concrete to compare against.

**Files:**
- Read-only: `Cargo.toml` (workspace), `crates/inputforge-gui-dx/Cargo.toml`, `crates/inputforge-gui-dx/assets/tokens/colors.css`, `crates/inputforge-gui-dx/assets/tokens/spacing.css`, `crates/inputforge-gui-dx/assets/tokens/elevation.css`

- [ ] **Step 1: Confirm Dioxus 0.7.6 is the latest stable**

The `latest-packages` skill MUST be invoked here. Brief: "verify dioxus and dioxus-cli current latest stable on crates.io vs the workspace pin (`dioxus = "0.7.6"` in `Cargo.toml:66`)."

If the latest stable is newer than 0.7.6, record the exact target version and proceed (workspace upgrade is a separate decision; if a patch-level bump is available without API breakage, fold it into Task 16's commit and note the bump in the README change). If no newer version exists, no change.

Expected outcome: a recorded version string and a one-line decision ("staying on 0.7.6" / "bumping to 0.7.X").

- [ ] **Step 2: Capture baseline warning count**

Run from repo root:

```bash
cargo build -p inputforge-gui-dx 2>&1 | tee /tmp/f4-baseline-warnings.txt
grep -c "^warning:" /tmp/f4-baseline-warnings.txt > /tmp/f4-baseline-count.txt
cat /tmp/f4-baseline-count.txt
```

Record the integer. The acceptance pass (Task 19) re-runs `cargo build -p inputforge-gui-dx` and compares the warning count against this recorded baseline. Equal or fewer is acceptable; greater fails the acceptance bullet.

- [ ] **Step 3: Confirm the F2 design tokens this plan references exist**

Spot-check the tokens cited in the spec's CSS sketches:

```bash
grep -E '^\s*--(color-(info|success|warning|error)(-bg)?|color-(info|live|warning|error)|shadow-3|space-[1-6]|radius-md|font-sans)\b' \
  crates/inputforge-gui-dx/assets/tokens/*.css \
  crates/inputforge-gui-dx/assets/global.css
```

Expected: at least one declaration line for each name. The full set is already verified by F2/F3 — this is a 30-second sanity check, not an investigation. If a token is missing under the expected name, RECORD the actual name and update every CSS code block in this plan to match before proceeding.

- [ ] **Step 4: No commit**

All three steps are read-only. No git activity.

---

## Task 2: Toast module scaffold + types

Create the `toast` module skeleton: `state.rs` with type definitions only (enum, structs, constants), `mod.rs` with re-exports, `lib.rs` declaration. Compiles to nothing yet — push/dismiss/pause/resume/is_expired are added in Tasks 3–5 via TDD.

**Files:**
- Create: `crates/inputforge-gui-dx/src/toast/mod.rs`
- Create: `crates/inputforge-gui-dx/src/toast/state.rs`
- Modify: `crates/inputforge-gui-dx/src/lib.rs:9` (add `mod toast;` after `mod tray;`)

- [ ] **Step 1: Create `crates/inputforge-gui-dx/src/toast/state.rs` with type definitions only**

```rust
//! Pure-data layer for the toast queue. No Dioxus runtime dependency —
//! every method on `ToastState` is `&mut self` and the unit tests construct
//! `ToastState::default()` directly. The Signal wrapper lives in `queue.rs`.

use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub id: u64,
    pub level: ToastLevel,
    pub message: String,
    /// Dedupe coalesce count — starts at 1; `push` of an exact duplicate
    /// against a non-dismissed entry increments this.
    pub count: u32,
    pub created: Instant,
    /// Some(start) while the toast is hover/focus-paused.
    pub paused: Option<Instant>,
    /// Accumulated pause time across resume cycles.
    pub paused_total: Duration,
    pub dismissed: bool,
}

#[derive(Debug, Default)]
pub struct ToastState {
    pub toasts: Vec<Toast>,
    pub next_id: u64,
}

/// Auto-dismiss duration excluding paused intervals.
pub const TOAST_DURATION: Duration = Duration::from_secs(8);

/// Cap on simultaneously-visible (non-dismissed) toasts. Push beyond the cap
/// FIFO-drains the oldest non-dismissed entry.
pub const TOAST_MAX_VISIBLE: usize = 5;
```

- [ ] **Step 2: Create `crates/inputforge-gui-dx/src/toast/mod.rs`**

```rust
//! Toast queue: pure-data state + Signal wrapper + viewport component +
//! production warnings bridge.

pub(crate) mod state;
// queue, viewport, warnings_bridge are added in later tasks.

pub use state::{Toast, ToastLevel, ToastState, TOAST_DURATION, TOAST_MAX_VISIBLE};
```

- [ ] **Step 3: Add `mod toast;` to `crates/inputforge-gui-dx/src/lib.rs`**

Open `crates/inputforge-gui-dx/src/lib.rs`, find the private-module block (currently `mod app; mod bridge; mod context; mod lifecycle; mod shell; mod tray;` at lines 3–8). Add `mod toast;` keeping alphabetical order:

```rust
mod app;
mod bridge;
mod context;
mod lifecycle;
mod shell;
mod toast;
mod tray;
```

- [ ] **Step 4: Build and confirm clean**

Run:

```bash
cargo build -p inputforge-gui-dx
```

Expected: PASS, warning count ≤ baseline (Task 1 Step 2). The new types are unused — clippy may flag `dead_code` on `Toast::id`/`Toast::created`/etc. If so, do NOT add `#[allow(dead_code)]` blanket attributes; the next task adds `push`, which uses every field. If the warning count is materially worse, pause and investigate.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/toast/mod.rs \
        crates/inputforge-gui-dx/src/toast/state.rs \
        crates/inputforge-gui-dx/src/lib.rs
git commit -m "feat(gui-dx): scaffold toast pure-data module"
```

Conventional Commits: `feat(gui-dx)`. Invoke the `conventional-commits` skill before running `git commit` to lock the scope/format.

---

## Task 3: TDD — `ToastState::push` (append, coalesce, monotonic ids)

First TDD bundle on `ToastState`. Three tests cover: empty-push appends; same `(level, message)` coalesces by incrementing `count` and resetting `created`; different levels do NOT coalesce; ids are monotonic.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/toast/state.rs` (add `impl ToastState { push }` and `#[cfg(test)]` block)

- [ ] **Step 1: Write the failing tests**

Append to `state.rs` (under the constants):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_appends_when_empty() {
        let mut s = ToastState::default();
        s.push(ToastLevel::Info, "hi");
        assert_eq!(s.toasts.len(), 1);
        assert_eq!(s.toasts[0].message, "hi");
        assert_eq!(s.toasts[0].count, 1);
        assert!(!s.toasts[0].dismissed);
    }

    #[test]
    fn push_coalesces_exact_string_match() {
        let mut s = ToastState::default();
        s.push(ToastLevel::Warning, "HidHide unavailable");
        s.push(ToastLevel::Warning, "HidHide unavailable");
        assert_eq!(s.toasts.len(), 1);
        assert_eq!(s.toasts[0].count, 2);
    }

    #[test]
    fn push_does_not_coalesce_across_levels() {
        let mut s = ToastState::default();
        s.push(ToastLevel::Info, "Saved");
        s.push(ToastLevel::Warning, "Saved");
        assert_eq!(s.toasts.len(), 2);
        assert_eq!(s.toasts[0].count, 1);
        assert_eq!(s.toasts[1].count, 1);
    }

    #[test]
    fn next_id_is_monotonic() {
        let mut s = ToastState::default();
        s.push(ToastLevel::Info, "a");
        s.push(ToastLevel::Info, "b");
        s.push(ToastLevel::Info, "c");
        assert_eq!(s.toasts[0].id, 0);
        assert_eq!(s.toasts[1].id, 1);
        assert_eq!(s.toasts[2].id, 2);
        assert_eq!(s.next_id, 3);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p inputforge-gui-dx toast::state::tests
```

Expected: COMPILE FAILURE (no method `push` on `ToastState`). That's the failing-test signal.

- [ ] **Step 3: Implement `push` (append + coalesce only — cap is Task 4)**

Add `impl ToastState` above the `#[cfg(test)] mod tests` block:

```rust
impl ToastState {
    pub fn push(&mut self, level: ToastLevel, message: impl Into<String>) {
        let msg = message.into();

        // Coalesce — exact (level, message) match against non-dismissed entries.
        if let Some(t) = self
            .toasts
            .iter_mut()
            .find(|t| !t.dismissed && t.level == level && t.message == msg)
        {
            t.count = t.count.saturating_add(1);
            t.created = Instant::now();
            t.paused = None;
            t.paused_total = Duration::ZERO;
            return;
        }

        // Append. wrapping_add on u64 is fine: id collisions only arise after
        // 18 quintillion pushes against this single ToastState — not realistic.
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.toasts.push(Toast {
            id,
            level,
            message: msg,
            count: 1,
            created: Instant::now(),
            paused: None,
            paused_total: Duration::ZERO,
            dismissed: false,
        });
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test -p inputforge-gui-dx toast::state::tests
```

Expected: 4/4 PASS.

- [ ] **Step 5: Add the timer-reset assertion (separate test for clarity)**

Append inside `mod tests`:

```rust
    #[test]
    fn push_resets_timer_on_coalesce() {
        let mut s = ToastState::default();
        s.push(ToastLevel::Info, "tick");
        let first_created = s.toasts[0].created;
        std::thread::sleep(Duration::from_millis(5));
        s.push(ToastLevel::Info, "tick");
        let second_created = s.toasts[0].created;
        assert!(second_created > first_created, "coalesce must reset created");
    }
```

Run again and confirm 5/5 PASS:

```bash
cargo test -p inputforge-gui-dx toast::state::tests
```

(The `sleep(5ms)` keeps the test deterministic without imposing wall-clock cost beyond noise.)

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/src/toast/state.rs
git commit -m "feat(gui-dx): toast push appends and coalesces"
```

---

## Task 4: TDD — `ToastState::push` cap with FIFO drain

Adds the visible-cap behavior: when a non-coalescing push would exceed `TOAST_MAX_VISIBLE` non-dismissed toasts, the oldest non-dismissed entry is auto-dismissed (its `dismissed` field flips to `true`) before the new toast is appended.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/toast/state.rs` (extend `push`, add test)

- [ ] **Step 1: Write the failing test**

Append inside `mod tests`:

```rust
    #[test]
    fn push_drops_oldest_when_cap_exceeded() {
        let mut s = ToastState::default();
        for i in 0..TOAST_MAX_VISIBLE {
            s.push(ToastLevel::Info, format!("msg-{i}"));
        }
        // Fifth push fills the cap exactly. No drain yet.
        let visible_now = s.toasts.iter().filter(|t| !t.dismissed).count();
        assert_eq!(visible_now, TOAST_MAX_VISIBLE);

        // Sixth push triggers the drain — the very first toast ("msg-0")
        // is the oldest non-dismissed entry.
        s.push(ToastLevel::Info, "overflow");

        let visible_after = s.toasts.iter().filter(|t| !t.dismissed).count();
        assert_eq!(visible_after, TOAST_MAX_VISIBLE);

        // The Vec carries 6 entries total (5 originally + 1 new). The "msg-0"
        // entry is now dismissed, every other original entry is still live.
        assert_eq!(s.toasts.len(), TOAST_MAX_VISIBLE + 1);
        assert!(s.toasts[0].dismissed, "oldest must be dismissed");
        assert_eq!(s.toasts[0].message, "msg-0");
        for i in 1..TOAST_MAX_VISIBLE {
            assert!(!s.toasts[i].dismissed, "non-oldest must stay live");
        }
        assert_eq!(s.toasts.last().unwrap().message, "overflow");
    }
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p inputforge-gui-dx toast::state::tests::push_drops_oldest_when_cap_exceeded
```

Expected: FAIL — without the cap, `s.toasts[0].dismissed == false` and the visible count is 6.

- [ ] **Step 3: Add the cap+FIFO drain to `push`**

Inside `impl ToastState::push`, between the coalesce block and the append block, insert the cap drain:

```rust
        // Cap — FIFO drain when exceeded. Counts only non-dismissed entries
        // because dismissed-but-still-in-Vec is the steady state during the
        // CSS fade-out window.
        let visible = self.toasts.iter().filter(|t| !t.dismissed).count();
        if visible >= TOAST_MAX_VISIBLE {
            if let Some(oldest) = self
                .toasts
                .iter_mut()
                .filter(|t| !t.dismissed)
                .min_by_key(|t| t.created)
            {
                oldest.dismissed = true;
            }
        }
```

- [ ] **Step 4: Run all toast::state tests**

```bash
cargo test -p inputforge-gui-dx toast::state::tests
```

Expected: 6/6 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/toast/state.rs
git commit -m "feat(gui-dx): toast cap fifo-drains oldest"
```

---

## Task 5: TDD — `dismiss`, `pause`/`resume`, `is_expired`

Final pure-data behavior: explicit dismiss, hover/focus pause-resume, and the timer predicate that excludes paused intervals.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/toast/state.rs` (add `dismiss`/`pause`/`resume` methods, `is_expired` free function, three tests)

- [ ] **Step 1: Write the failing tests**

Append inside `mod tests`:

```rust
    #[test]
    fn dismiss_marks_entry_dismissed() {
        let mut s = ToastState::default();
        s.push(ToastLevel::Info, "go");
        let id = s.toasts[0].id;
        s.dismiss(id);
        assert!(s.toasts[0].dismissed);
        // Idempotent — second dismiss is a no-op.
        s.dismiss(id);
        assert!(s.toasts[0].dismissed);
    }

    #[test]
    fn pause_resume_accumulates_paused_total() {
        let mut s = ToastState::default();
        s.push(ToastLevel::Info, "p");
        let id = s.toasts[0].id;
        s.pause(id);
        std::thread::sleep(Duration::from_millis(8));
        s.resume(id);
        let after_first = s.toasts[0].paused_total;
        assert!(after_first >= Duration::from_millis(7));
        s.pause(id);
        std::thread::sleep(Duration::from_millis(5));
        s.resume(id);
        let after_second = s.toasts[0].paused_total;
        assert!(
            after_second > after_first,
            "second resume must accumulate"
        );
    }

    #[test]
    fn is_expired_excludes_paused_time() {
        let mut s = ToastState::default();
        s.push(ToastLevel::Info, "x");
        let toast = s.toasts[0].clone();
        let now = toast.created + TOAST_DURATION + Duration::from_millis(1);
        // No pauses → expired right at TOAST_DURATION.
        assert!(is_expired(&toast, now));

        // Paused for the entire elapsed window → effective elapsed is zero.
        let mut t2 = toast.clone();
        t2.paused = Some(t2.created);
        assert!(!is_expired(&t2, now));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p inputforge-gui-dx toast::state::tests
```

Expected: COMPILE FAILURE (no `dismiss`/`pause`/`resume`/`is_expired`).

- [ ] **Step 3: Implement the four methods**

Inside the existing `impl ToastState` block, append:

```rust
    pub fn dismiss(&mut self, id: u64) {
        if let Some(t) = self.toasts.iter_mut().find(|t| t.id == id) {
            t.dismissed = true;
        }
    }

    pub fn pause(&mut self, id: u64) {
        if let Some(t) = self
            .toasts
            .iter_mut()
            .find(|t| t.id == id && !t.dismissed && t.paused.is_none())
        {
            t.paused = Some(Instant::now());
        }
    }

    pub fn resume(&mut self, id: u64) {
        if let Some(t) = self
            .toasts
            .iter_mut()
            .find(|t| t.id == id && !t.dismissed)
        {
            if let Some(start) = t.paused.take() {
                t.paused_total = t.paused_total.saturating_add(start.elapsed());
            }
        }
    }
```

After the `impl ToastState` block (and BEFORE `#[cfg(test)] mod tests`), add the free `is_expired` function:

```rust
/// Compute whether a toast has exceeded `TOAST_DURATION`, excluding paused
/// intervals (both finalized via `paused_total` and any in-progress pause
/// observed via `paused`).
pub fn is_expired(t: &Toast, now: Instant) -> bool {
    if t.dismissed {
        return true;
    }
    let total = now.saturating_duration_since(t.created);
    let current_pause = t
        .paused
        .map_or(Duration::ZERO, |s| now.saturating_duration_since(s));
    let effective = total.saturating_sub(t.paused_total + current_pause);
    effective >= TOAST_DURATION
}
```

- [ ] **Step 4: Re-export `is_expired` from `toast/mod.rs`**

Open `crates/inputforge-gui-dx/src/toast/mod.rs` and add `is_expired` to the existing `pub use state::{...}`:

```rust
pub use state::{is_expired, Toast, ToastLevel, ToastState, TOAST_DURATION, TOAST_MAX_VISIBLE};
```

- [ ] **Step 5: Run all toast::state tests**

```bash
cargo test -p inputforge-gui-dx toast::state::tests
```

Expected: 9/9 PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/src/toast/state.rs \
        crates/inputforge-gui-dx/src/toast/mod.rs
git commit -m "feat(gui-dx): toast dismiss/pause/resume/is_expired"
```

---

## Task 6: `ToastQueue` Signal wrapper

Thin wrapper around `Signal<ToastState>` that delegates every method via `state.write()`. No logic of its own. Constructed in the parent scope (`app_root` or `gallery_root`) — `ToastQueue` itself has NO `::new()`, mirroring the F1 `AppContext` pattern.

**Files:**
- Create: `crates/inputforge-gui-dx/src/toast/queue.rs`
- Modify: `crates/inputforge-gui-dx/src/toast/mod.rs` (add `pub mod queue; pub use queue::ToastQueue;`)
- Modify: `crates/inputforge-gui-dx/src/lib.rs` (add `pub use toast::{ToastLevel, ToastQueue};` so the gallery and external producers can name them)

- [ ] **Step 1: Write the failing compile (no test — pure delegation)**

This module is a one-liner per method, structurally tested by usage downstream. No unit tests warranted — the underlying `ToastState` methods are already covered. Move directly to implementation.

- [ ] **Step 2: Create `crates/inputforge-gui-dx/src/toast/queue.rs`**

```rust
//! Signal wrapper over `ToastState`. Constructed by the parent scope via
//! `use_signal(ToastState::default)`, then placed in context.
//!
//! Producers reach the queue with `use_context::<ToastQueue>().push(level, msg)`.
//! The viewport reads via `queue.visible(now)`.

use std::time::Instant;

use dioxus::prelude::*;

use crate::toast::state::{is_expired, Toast, ToastLevel, ToastState};

/// `Signal<ToastState>` wrapper. `Copy` (Signals are `Copy` in Dioxus 0.7),
/// so `ToastQueue` is freely passed by value into closures and contexts.
///
/// `state` is `pub` (rather than `pub(crate)`) so external example binaries
/// (Cargo `examples/`) and downstream crates can construct a queue. Producers
/// MUST initialize the inner Signal via `use_signal(ToastState::default)` from
/// inside a Dioxus runtime — `Signal::new()` outside a hook leaks per
/// `dioxus-signals/src/signal.rs:30-52`. Production wiring lives in `app_root`.
#[derive(Clone, Copy)]
pub struct ToastQueue {
    pub state: Signal<ToastState>,
}

impl ToastQueue {
    pub fn push(&self, level: ToastLevel, message: impl Into<String>) {
        self.state.write().push(level, message);
    }

    pub fn dismiss(&self, id: u64) {
        self.state.write().dismiss(id);
    }

    pub fn pause(&self, id: u64) {
        self.state.write().pause(id);
    }

    pub fn resume(&self, id: u64) {
        self.state.write().resume(id);
    }

    /// Snapshot of non-expired toasts at `now`. Used by `ToastViewport` on
    /// each tick. Cloning is cheap (toasts are short and bounded by
    /// `TOAST_MAX_VISIBLE` plus a few in-flight dismissed entries fading out).
    pub(crate) fn visible(&self, now: Instant) -> Vec<Toast> {
        self.state
            .read()
            .toasts
            .iter()
            .filter(|t| !is_expired(t, now))
            .cloned()
            .collect()
    }
}
```

- [ ] **Step 3: Update `crates/inputforge-gui-dx/src/toast/mod.rs`**

Replace the body with:

```rust
//! Toast queue: pure-data state + Signal wrapper + viewport component +
//! production warnings bridge.

pub(crate) mod state;
pub(crate) mod queue;
// viewport, warnings_bridge are added in later tasks.

pub use queue::ToastQueue;
pub use state::{is_expired, Toast, ToastLevel, ToastState, TOAST_DURATION, TOAST_MAX_VISIBLE};
```

- [ ] **Step 4: Add public re-exports in `crates/inputforge-gui-dx/src/lib.rs`**

After the existing `pub mod` block (currently lines 10–12 — `pub mod components; pub mod icons; pub mod theme;`), add:

```rust
pub use toast::{ToastLevel, ToastQueue};
```

This is the only public surface F4 adds at the crate root: producers (gallery now, F8/F11/F13/F14 later) import as `use inputforge_gui_dx::{ToastLevel, ToastQueue}; use_context::<ToastQueue>().push(ToastLevel::Warning, "msg");`.

- [ ] **Step 5: Build and confirm clean**

```bash
cargo build -p inputforge-gui-dx
```

Expected: PASS, warnings ≤ baseline. Clippy may flag `dead_code` on `ToastQueue::visible` (it has no caller yet — ToastViewport in Task 7 wires it). Do NOT add `#[allow(dead_code)]` — leave the warning for now; it disappears in Task 7.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/src/toast/queue.rs \
        crates/inputforge-gui-dx/src/toast/mod.rs \
        crates/inputforge-gui-dx/src/lib.rs
git commit -m "feat(gui-dx): add ToastQueue signal wrapper"
```

---

## Task 7: `ToastViewport` component + stub `toast.css` + `--z-toast` token

The single component that consumes `ToastQueue`, drives the 4 Hz expiration tick, splits visible toasts into per-level ARIA live regions, and renders `ToastItem` children with hover/focus/escape handling. CSS is a stub — Task 14 finalizes it after `impeccable:frontend-design`. Mounts the stylesheet in `ThemeProvider`.

**Files:**
- Create: `crates/inputforge-gui-dx/src/toast/viewport.rs`
- Create: `crates/inputforge-gui-dx/assets/toast/toast.css`
- Modify: `crates/inputforge-gui-dx/src/toast/mod.rs` (add `pub(crate) mod viewport; pub(crate) use viewport::ToastViewport;`)
- Modify: `crates/inputforge-gui-dx/assets/tokens/elevation.css` (add `--z-toast: 1100;` token)
- Modify: `crates/inputforge-gui-dx/src/theme/mod.rs` (mount `toast.css`)

- [ ] **Step 1: Add `--z-toast` token**

Open `crates/inputforge-gui-dx/assets/tokens/elevation.css`. After the closing `}` of the existing `:root { … }` block at line 26, append a new `:root { … }` block (cascade-merges into the existing one):

```css
/* Toast viewport sits above all shell content and any open <dialog>.
   Menu's z-index: 1000 (assets/components/menu.css) is the existing top
   stack; toast goes a tier higher so a warning surfacing during an open
   menu or dialog remains visible. */
:root {
    --z-toast: 1100;
}
```

- [ ] **Step 2: Create `crates/inputforge-gui-dx/assets/toast/toast.css` (stub)**

```css
/* Stub — finalized in Task 14 after impeccable:frontend-design.
   Layout-only: the viewport is fixed, top-right, two stacked ARIA
   regions; toasts are tab-reachable; per-level accent classes exist
   but are intentionally bare so frontend-design has clean ground. */

.if-toast-viewport {
    position: fixed;
    right: 12px;
    z-index: var(--z-toast);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    pointer-events: none;
    max-width: 360px;
}
.if-toast-viewport--polite    { top: 12px; }
.if-toast-viewport--assertive { top: 12px; right: 12px; }

.if-toast {
    pointer-events: auto;
    background: var(--color-bg-elevated);
    color: var(--color-text);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-3);
    box-shadow: var(--shadow-3);
    font-family: var(--font-sans);
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    transition: opacity 400ms;
}
.if-toast:focus-visible { outline: 2px solid var(--color-border-focus); outline-offset: 2px; }

.if-toast--info    { border-left: 4px solid var(--color-info); }
.if-toast--success { border-left: 4px solid var(--color-live); }
.if-toast--warning { border-left: 4px solid var(--color-warning); }
.if-toast--error   { border-left: 4px solid var(--color-error); }

.if-toast__message    { flex: 1; }
.if-toast__count      { color: var(--color-text-muted); margin-left: var(--space-1); }
.if-toast__close {
    background: transparent;
    border: none;
    color: var(--color-text-muted);
    cursor: pointer;
    padding: 0 var(--space-1);
    font-size: var(--text-md);
    line-height: 1;
}
.if-toast__close:hover         { color: var(--color-text); }
.if-toast__close:focus-visible { outline: 2px solid var(--color-border-focus); outline-offset: 2px; }
```

The polite/assertive `top` values are deliberately identical for now — they stack via flex inside one viewport per region. Task 14 (frontend-design) may revisit (e.g., assertive overlays polite at the same anchor; or two columns; or a single region with z-index ordering). The Rust side stays stable across that decision.

- [ ] **Step 3: Create `crates/inputforge-gui-dx/src/toast/viewport.rs`**

```rust
use std::time::{Duration, Instant};

use dioxus::prelude::*;

use crate::components::Icon;
use crate::icons::{Icon as IconKind, IconSize};
use crate::toast::queue::ToastQueue;
use crate::toast::state::{Toast, ToastLevel};

/// Renders the toast queue. Two stacked ARIA regions split visible toasts
/// by level so AT picks the correct delivery verb without per-item tagging.
///
/// Tick mechanism: a `use_signal(Instant::now)` Signal is updated every 250 ms
/// by a tokio interval; reading it in the body produces the per-tick re-render
/// that drives expiration GC. 250 ms is far coarser than per-frame and an
/// order of magnitude finer than the CSS fade-out duration, so cost is
/// negligible.
#[component]
pub(crate) fn ToastViewport() -> Element {
    let queue = use_context::<ToastQueue>();

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

    let polite: Vec<Toast> = toasts
        .iter()
        .filter(|t| matches!(t.level, ToastLevel::Info | ToastLevel::Success))
        .cloned()
        .collect();
    let assertive: Vec<Toast> = toasts
        .iter()
        .filter(|t| matches!(t.level, ToastLevel::Warning | ToastLevel::Error))
        .cloned()
        .collect();

    rsx! {
        div {
            class: "if-toast-viewport if-toast-viewport--polite",
            role: "status",
            "aria-live": "polite",
            for t in polite {
                ToastItem { key: "{t.id}", toast: t }
            }
        }
        div {
            class: "if-toast-viewport if-toast-viewport--assertive",
            role: "alert",
            "aria-live": "assertive",
            for t in assertive {
                ToastItem { key: "{t.id}", toast: t }
            }
        }
    }
}

#[component]
fn ToastItem(toast: Toast) -> Element {
    let queue = use_context::<ToastQueue>();
    let id = toast.id;
    let (level_class, icon_kind) = match toast.level {
        ToastLevel::Info    => ("if-toast--info",    IconKind::Info),
        ToastLevel::Success => ("if-toast--success", IconKind::Check),
        ToastLevel::Warning => ("if-toast--warning", IconKind::Warning),
        ToastLevel::Error   => ("if-toast--error",   IconKind::Error),
    };
    let count = toast.count;
    let message = toast.message.clone();

    rsx! {
        div {
            class: "if-toast {level_class}",
            tabindex: "0",
            onmouseenter: move |_| queue.pause(id),
            onmouseleave: move |_| queue.resume(id),
            onfocusin:    move |_| queue.pause(id),
            onfocusout:   move |_| queue.resume(id),
            onkeydown:    move |e: KeyboardEvent| {
                if e.key() == Key::Escape {
                    queue.dismiss(id);
                }
            },
            Icon { name: icon_kind, size: IconSize::Sm }
            span { class: "if-toast__message", "{message}" }
            if count > 1 {
                span { class: "if-toast__count", "×{count}" }
            }
            button {
                class: "if-toast__close",
                "aria-label": "Dismiss",
                onclick: move |_| queue.dismiss(id),
                "×"
            }
        }
    }
}
```

- [ ] **Step 4: Update `crates/inputforge-gui-dx/src/toast/mod.rs`**

```rust
//! Toast queue: pure-data state + Signal wrapper + viewport component +
//! production warnings bridge.

pub(crate) mod state;
pub(crate) mod queue;
pub(crate) mod viewport;
// warnings_bridge is added in Task 16.

pub use queue::ToastQueue;
pub use state::{is_expired, Toast, ToastLevel, ToastState, TOAST_DURATION, TOAST_MAX_VISIBLE};
pub(crate) use viewport::ToastViewport;
```

- [ ] **Step 5: Mount `toast.css` in `ThemeProvider`**

Open `crates/inputforge-gui-dx/src/theme/mod.rs`. Add the asset constant (alphabetized with the existing block at lines 10–36):

```rust
const TOAST_CSS: Asset = asset!("/assets/toast/toast.css");
```

Then, inside the `rsx!` (currently lines 40–73), append the `Stylesheet` mount AFTER the last component CSS line and BEFORE `{children}`:

```rust
        // Toast overlay — last so its z-index cascade wins.
        Stylesheet { href: TOAST_CSS }

        {children}
```

- [ ] **Step 6: Build and confirm clean**

```bash
cargo build -p inputforge-gui-dx
```

Expected: PASS. The `dead_code` warning on `ToastQueue::visible` from Task 6 should be GONE. The `dead_code` warning on `ToastViewport` will appear instead — gallery wiring (Task 9) consumes it. Acceptable.

- [ ] **Step 7: Commit**

```bash
git add crates/inputforge-gui-dx/src/toast/viewport.rs \
        crates/inputforge-gui-dx/src/toast/mod.rs \
        crates/inputforge-gui-dx/assets/toast/toast.css \
        crates/inputforge-gui-dx/assets/tokens/elevation.css \
        crates/inputforge-gui-dx/src/theme/mod.rs
git commit -m "feat(gui-dx): add ToastViewport component"
```

---

## Task 8: Make `ToastViewport` reachable for gallery / app installation

Tiny export-only task: bump `ToastViewport`'s visibility so the gallery and `app_root` can render it without going through a re-export shim. Already `pub(crate) use` — it's reachable from sibling `app.rs`. The gallery is a separate binary, so we need a public-by-feature-flag re-export OR the gallery installs its OWN `ToastViewport` via the public API.

The simplest solution: keep `ToastViewport` `pub(crate)` for the production app installation, and have the gallery example install a tiny inline `ToastViewport` proxy that uses the public `ToastQueue`. But that duplicates rendering — unwanted.

Better: re-export `ToastViewport` from `lib.rs` under `#[doc(hidden)]` so external consumers can render it but it doesn't show up in the public docs. Future external consumers (other examples, downstream crates) can wire it into their own roots without F4 needing to expose `ToastQueue::visible` or component internals.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/toast/mod.rs` — change `pub(crate) use viewport::ToastViewport;` to `pub use viewport::ToastViewport;`
- Modify: `crates/inputforge-gui-dx/src/lib.rs` — extend the `pub use toast::...` line to `pub use toast::{ToastLevel, ToastQueue, ToastViewport};` and add `#[doc(hidden)]` above it

- [ ] **Step 1: Update `crates/inputforge-gui-dx/src/toast/mod.rs`**

Change:

```rust
pub(crate) use viewport::ToastViewport;
```

To:

```rust
pub use viewport::ToastViewport;
```

- [ ] **Step 2: Update `crates/inputforge-gui-dx/src/lib.rs`**

Replace the existing `pub use toast::{ToastLevel, ToastQueue};` line (added in Task 6) with:

```rust
#[doc(hidden)]
pub use toast::ToastViewport;
pub use toast::{ToastLevel, ToastQueue};
```

`#[doc(hidden)]` keeps `ToastViewport` out of the public API surface of the docs (it's an implementation detail of the rendering layer), while still letting external example binaries import it.

- [ ] **Step 3: Build and confirm clean**

```bash
cargo build -p inputforge-gui-dx
```

Expected: PASS, warnings ≤ baseline. The `dead_code` warning from Task 7 stays — fixed by Task 9.

- [ ] **Step 4: Commit**

```bash
git add crates/inputforge-gui-dx/src/toast/mod.rs \
        crates/inputforge-gui-dx/src/lib.rs
git commit -m "feat(gui-dx): expose ToastViewport via doc(hidden)"
```

---

## Task 9: Gallery — Toasts section

Wires `ToastQueue` into `gallery_root` via `use_context_provider`, mounts `ToastViewport` at the top of the gallery, and adds the five-button section described in the spec. This is the first end-to-end render path for the toast infrastructure.

**Files:**
- Modify: `crates/inputforge-gui-dx/examples/component_gallery.rs`

- [ ] **Step 1: Add imports**

At the top of `examples/component_gallery.rs`, extend the existing `use inputforge_gui_dx::components::{...}` import to also bring in nothing extra (we use the public re-export instead). Add a sibling import line:

```rust
use inputforge_gui_dx::{ToastLevel, ToastQueue, ToastViewport};
use inputforge_gui_dx::toast::ToastState;
```

The second import accesses `ToastState` for `use_signal(ToastState::default)`. `crate::toast` is currently `mod toast` (private) at the crate root — we need to make `state::ToastState` reachable. The cleanest fix: re-export `ToastState` at the crate root alongside `ToastLevel` / `ToastQueue`.

Update `crates/inputforge-gui-dx/src/lib.rs` to:

```rust
#[doc(hidden)]
pub use toast::ToastViewport;
pub use toast::{ToastLevel, ToastQueue, ToastState};
```

And update the import in the gallery to:

```rust
use inputforge_gui_dx::{ToastLevel, ToastQueue, ToastState, ToastViewport};
```

- [ ] **Step 2: Install the queue context in `gallery_root`**

At the top of `gallery_root` (currently `examples/component_gallery.rs:35-42`), add the queue installation alongside the existing `use_signal` calls:

```rust
fn gallery_root() -> Element {
    let toast_state = use_signal(ToastState::default);
    let toast_queue = ToastQueue { state: toast_state };
    use_context_provider(|| toast_queue);

    // Existing F2 demo signals…
    let mut number_demo = use_signal(|| 50.0_f64);
    // …unchanged…
```

`ToastQueue { state: toast_state }` is a struct expression — `state` is `pub` (Task 6 made it pub specifically so example binaries can construct the queue). Cargo examples consume the crate as an external dependency, so only `pub` items are reachable. Verify with `cargo build --examples -p inputforge-gui-dx` after Step 4 if there's any doubt.

- [ ] **Step 3: Mount `ToastViewport` and add the Toasts section**

Inside the existing `rsx! { ThemeProvider { main { Stack { ... } } } }` block — currently the heading-and-sections wall in the gallery — replace the outermost `ThemeProvider {` to also render `ToastViewport`:

```rust
    rsx! {
        ThemeProvider {
            ToastViewport {}
            main {
                Stack { gap: "--space-8".to_owned(), padding: "--space-6".to_owned(),
                    h1 { "InputForge — Component Gallery (F4)" }

                    section {
                        h2 { "Toasts" }
                        Card { padding: CardPadding::Md,
                            Stack { gap: "--space-3".to_owned(),
                                Cluster { gap: "--space-3".to_owned(),
                                    Button { onclick: move |_| toast_queue.push(ToastLevel::Info,    "Info toast"),    "Push Info" }
                                    Button { onclick: move |_| toast_queue.push(ToastLevel::Success, "Saved successfully"), "Push Success" }
                                    Button { onclick: move |_| toast_queue.push(ToastLevel::Warning, "HidHide unavailable"), "Push Warning" }
                                    Button { onclick: move |_| toast_queue.push(ToastLevel::Error,   "Engine failed to start"), "Push Error" }
                                }
                                Cluster { gap: "--space-3".to_owned(),
                                    Button {
                                        onclick: move |_| {
                                            for _ in 0..10 {
                                                toast_queue.push(ToastLevel::Warning, "Spammy");
                                            }
                                        },
                                        "Push spam (×10)"
                                    }
                                    Button {
                                        onclick: move |_| {
                                            for i in 0..7 {
                                                toast_queue.push(ToastLevel::Info, format!("Distinct {i}"));
                                            }
                                        },
                                        "Push 7 distinct"
                                    }
                                }
                                p {
                                    style: "color: var(--color-text-muted); font-size: var(--text-sm);",
                                    "Hover or focus a toast to pause its timer; press ESC while focused to dismiss; click × to dismiss."
                                }
                            }
                        }
                    }

                    // Existing F2 sections (h1 already moved above): Icon, Button, IconButton, …
                    // Keep the existing wall as-is — don't reshuffle.
```

The replacement is structural: the existing `h1 { "InputForge — Component Gallery (F2)" }` line becomes the `(F4)` line, and the new `section` for Toasts is the FIRST section. The gallery h1 update is intentional — F3 had its own incremental update (it's `(F2)` today; F4 owns the bump).

- [ ] **Step 4: Run the gallery and verify each interaction**

```bash
dx serve --example component_gallery --platform desktop
```

Open the window. Verify:
- "Push Info" → one info toast (top-right, blue accent in stub CSS).
- "Push Success" → one success toast (green accent).
- "Push Warning" → one warning toast (amber accent, in the assertive region).
- "Push Error" → one error toast (red accent, assertive region).
- Click "Push Warning" three times → ONE toast with `×3` count.
- "Push spam (×10)" → ONE toast with `×10` count.
- "Push 7 distinct" → exactly 5 toasts visible; oldest two ("Distinct 0" / "Distinct 1") are not in view.
- Hover any toast → it does not fade after 8 s (timer paused).
- Tab to a toast → focus ring visible (stub CSS).
- ESC while focused → toast dismisses immediately.
- Click × → dismisses.
- After 8 s with no hover/focus → toast fades and unmounts.
- Click in the empty area near the top-right while no toasts are visible → click reaches the underlying gallery section underneath (verifies `pointer-events: none` on the empty viewport divs).

Two of these checks (no toasts visible → underlying click works, fade duration) require a state where the viewport divs cover area but contain nothing. Confirm by clicking through where the viewport sits.

- [ ] **Step 5: Build the example as a smoke check**

```bash
cargo build --example component_gallery -p inputforge-gui-dx
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/examples/component_gallery.rs \
        crates/inputforge-gui-dx/src/lib.rs
git commit -m "feat(gui-dx): add Toasts section to gallery"
```

---

## Task 10: `Dialog` compound primitive — Rust file with JS shims + stub CSS

Native `<dialog>` + `showModal()`-driven compound. Eagerly computes `dialog_id`/`title_id`/`desc_id` in `DialogRoot`'s `use_hook` initializer so `aria-labelledby`/`aria-describedby` resolve correctly on the very first render. JS shims drive `showModal`/`close` and the `cancel` listener via `document::eval`.

**Files:**
- Create: `crates/inputforge-gui-dx/src/components/dialog.rs`
- Create: `crates/inputforge-gui-dx/assets/components/dialog.css`
- Modify: `crates/inputforge-gui-dx/src/components/mod.rs` (add `pub mod dialog;` + re-exports)
- Modify: `crates/inputforge-gui-dx/src/theme/mod.rs` (mount `dialog.css`)

- [ ] **Step 1: Write the failing id-generation test**

Create `crates/inputforge-gui-dx/src/components/dialog.rs` with the test block first:

```rust
//! Compound dialog primitive over the native HTML `<dialog>` element.
//! See spec for boundary contract; implementation locked in this module.

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: id derivation must produce stable, monotonic, well-formed
    /// names. Children read `dialog_id`/`title_id`/`desc_id` from `DialogState`
    /// during their first render, so a typo in the format string would produce
    /// a dangling `aria-labelledby`/`aria-describedby` on first paint.
    #[test]
    fn dialog_id_derivation_is_well_formed() {
        let n = next_dialog_seq_for_test();
        let dialog_id = format!("if-dialog-{n}");
        let title_id  = format!("{dialog_id}-title");
        let desc_id   = format!("{dialog_id}-desc");
        assert!(dialog_id.starts_with("if-dialog-"));
        assert_eq!(title_id, format!("if-dialog-{n}-title"));
        assert_eq!(desc_id,  format!("if-dialog-{n}-desc"));
    }

    #[test]
    fn dialog_seq_is_monotonic() {
        let a = next_dialog_seq_for_test();
        let b = next_dialog_seq_for_test();
        let c = next_dialog_seq_for_test();
        assert!(b > a);
        assert!(c > b);
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cargo test -p inputforge-gui-dx components::dialog
```

Expected: COMPILE FAILURE (`next_dialog_seq_for_test`, `DialogState`, etc. don't exist).

- [ ] **Step 3: Write the implementation**

At the top of `dialog.rs`, ABOVE the `#[cfg(test)] mod tests { … }` block, add the full implementation:

```rust
use std::sync::atomic::{AtomicU64, Ordering};

use dioxus::prelude::*;

use super::merge_class;

static DIALOG_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Test seam — bumps the counter and returns the new value. Production code
/// goes through `DialogRoot`'s `use_hook` and never calls this directly.
#[cfg(test)]
fn next_dialog_seq_for_test() -> u64 {
    DIALOG_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

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

/// Shared per-dialog context. Children read; only `DialogRoot` writes.
/// All ids are eagerly computed by `DialogRoot`'s `use_hook` initializer
/// (which runs during render — BEFORE children render), so children see
/// fully-populated ids on their very first render and `aria-labelledby` /
/// `aria-describedby` resolve correctly on the initial `showModal()` call.
#[derive(Clone)]
struct DialogState {
    open: Signal<bool>,
    dialog_id: String,
    title_id: String,
    desc_id: String,
    close_on_backdrop_click: bool,
}

/// Root of the dialog compound. Drives `showModal()` / `close()` on `open`
/// changes; attaches a one-shot `cancel` listener on first commit.
///
/// `dismissible` is **read once** at attach time. Flipping it after mount
/// has no effect on subsequent ESC events. F4's only consumers (gallery
/// demos and `DirtyConfirmDialog`) pass stable values.
#[component]
pub fn DialogRoot(
    open: Signal<bool>,
    onclose: EventHandler<()>,
    #[props(default = true)]  dismissible: bool,
    #[props(default = false)] close_on_backdrop_click: bool,
    #[props(default)]         class: Option<String>,
    children: Element,
) -> Element {
    // Eager id derivation — runs once during the parent's render, BEFORE
    // children render.
    let state = use_hook(|| {
        let n = DIALOG_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dialog_id = format!("if-dialog-{n}");
        let title_id = format!("{dialog_id}-title");
        let desc_id = format!("{dialog_id}-desc");
        DialogState {
            open,
            dialog_id,
            title_id,
            desc_id,
            close_on_backdrop_click,
        }
    });
    use_context_provider(|| state.clone());

    // Drive showModal()/close() on `open` changes. use_effect runs AFTER
    // DOM commit so getElementById is guaranteed to find the <dialog>.
    let id_for_open = state.dialog_id.clone();
    use_effect(move || {
        let action = if *open.read() { DIALOG_OPEN_JS } else { DIALOG_CLOSE_JS };
        let _ = document::eval(&format!("{action}({id_for_open:?})"));
    });

    // Attach `cancel` listener once after first DOM commit. The `dismissible`
    // value is interpolated into the JS at attach time — see doc-comment.
    let id_for_cancel = state.dialog_id.clone();
    let dismissible_now = dismissible;
    let mut attached = use_signal(|| false);
    use_effect(move || {
        if *attached.peek() {
            return;
        }
        let _ = document::eval(&format!(
            "{DIALOG_ATTACH_CANCEL_JS}({id_for_cancel:?}, {dismissible_now})"
        ));
        attached.set(true);
    });

    let combined = merge_class("if-dialog", "", class.as_deref());
    let close_on_backdrop = state.close_on_backdrop_click;
    let mut open_signal = state.open;

    rsx! {
        dialog {
            id: "{state.dialog_id}",
            class: "{combined}",
            "aria-labelledby":  "{state.title_id}",
            "aria-describedby": "{state.desc_id}",
            onclose: move |_| {
                open_signal.set(false);
                onclose.call(());
            },
            onclick: move |_evt| {
                if !close_on_backdrop {
                    return;
                }
                // Reaches here only on backdrop clicks because the inner
                // .if-dialog__panel calls evt.stop_propagation() on its onclick.
                open_signal.set(false);
                onclose.call(());
            },
            div {
                class: "if-dialog__panel",
                onclick: move |evt| evt.stop_propagation(),
                {children}
            }
        }
    }
}

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

#[component]
pub fn DialogBody(children: Element) -> Element {
    rsx! { div { class: "if-dialog__body", {children} } }
}

#[component]
pub fn DialogFooter(children: Element) -> Element {
    rsx! { div { class: "if-dialog__footer", {children} } }
}
```

- [ ] **Step 4: Run the dialog tests**

```bash
cargo test -p inputforge-gui-dx components::dialog
```

Expected: 2/2 PASS.

- [ ] **Step 5: Create `crates/inputforge-gui-dx/assets/components/dialog.css` (stub)**

```css
/* Stub — finalized in Task 14 after impeccable:frontend-design.
   Layout-only: native <dialog> resets, panel surface, scrollable body,
   right-aligned footer. No motion, no fancy backdrop yet. */

.if-dialog {
    background: transparent;
    border: none;
    padding: 0;
    /* Native :modal already centers; we just constrain width. */
    max-width: min(560px, calc(100vw - 32px));
    color: var(--color-text);
}
.if-dialog::backdrop {
    background: var(--color-bg-overlay);
}
.if-dialog__panel {
    background: var(--color-bg-elevated);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-5);
    box-shadow: var(--shadow-3);
    font-family: var(--font-sans);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    max-height: 80vh;
}
.if-dialog__title {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: var(--weight-semibold);
}
.if-dialog__desc {
    margin: 0;
    color: var(--color-text-muted);
}
.if-dialog__body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
}
.if-dialog__footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
}
```

- [ ] **Step 6: Update `crates/inputforge-gui-dx/src/components/mod.rs`**

Add the `pub mod dialog;` line in alphabetical order (between `checkbox` and `field`):

```rust
pub mod checkbox;
pub mod dialog;
pub mod field;
```

And add the re-exports (alphabetical with the existing `pub use` block, around the current `pub use checkbox::Checkbox;` line):

```rust
pub use dialog::{DialogBody, DialogDescription, DialogFooter, DialogRoot, DialogTitle};
```

- [ ] **Step 7: Mount `dialog.css` in `ThemeProvider`**

Open `crates/inputforge-gui-dx/src/theme/mod.rs`. Add the asset constant alphabetized with the existing block:

```rust
const DIALOG_CSS: Asset = asset!("/assets/components/dialog.css");
```

Then inside `rsx!`, add the `Stylesheet` mount alongside other component CSS, BEFORE the `TOAST_CSS` mount (toast must come last):

```rust
        Stylesheet { href: DIALOG_CSS }
        // …existing component CSS mounts unchanged…
```

The exact ordering inside the component-CSS block isn't critical — pick a sensible alphabetical slot.

- [ ] **Step 8: Build and confirm clean**

```bash
cargo build -p inputforge-gui-dx
```

Expected: PASS, warnings ≤ baseline. The Dialog parts are unused — gallery (Task 11) consumes them.

- [ ] **Step 9: Commit**

```bash
git add crates/inputforge-gui-dx/src/components/dialog.rs \
        crates/inputforge-gui-dx/src/components/mod.rs \
        crates/inputforge-gui-dx/assets/components/dialog.css \
        crates/inputforge-gui-dx/src/theme/mod.rs
git commit -m "feat(gui-dx): add Dialog compound primitive"
```

---

## Task 11: Gallery — Dialog primitives section

Four buttons opening four dialogs that cover the contract: basic dismissible, non-dismissible, close-on-backdrop, scrollable body.

**Files:**
- Modify: `crates/inputforge-gui-dx/examples/component_gallery.rs`

- [ ] **Step 1: Add imports**

In the existing `use inputforge_gui_dx::components::{...}` block, append:

```rust
DialogBody, DialogDescription, DialogFooter, DialogRoot, DialogTitle,
```

- [ ] **Step 2: Add four signals in `gallery_root`**

After the existing `let mut tabs_demo = use_signal(...)` line (and before `rsx! { ... }`):

```rust
    let mut basic_open       = use_signal(|| false);
    let mut non_dismiss_open = use_signal(|| false);
    let mut backdrop_open    = use_signal(|| false);
    let mut scroll_open      = use_signal(|| false);
```

- [ ] **Step 3: Add the Dialog primitives section**

Insert this `section { … }` block after the Toasts section (introduced in Task 9):

```rust
                    section {
                        h2 { "Dialog primitives" }
                        Card { padding: CardPadding::Md,
                            Cluster { gap: "--space-3".to_owned(),
                                Button { onclick: move |_| basic_open.set(true),       "Basic" }
                                Button { onclick: move |_| non_dismiss_open.set(true), "Non-dismissible" }
                                Button { onclick: move |_| backdrop_open.set(true),    "Close-on-backdrop" }
                                Button { onclick: move |_| scroll_open.set(true),      "Scrollable body" }
                            }
                        }

                        DialogRoot {
                            open: basic_open,
                            onclose: move |_| {},
                            DialogTitle { "Basic dialog" }
                            DialogDescription { "ESC dismisses; backdrop click does not." }
                            DialogBody {}
                            DialogFooter {
                                Button {
                                    variant: ButtonVariant::Primary,
                                    onclick: move |_| basic_open.set(false),
                                    "Close"
                                }
                            }
                        }

                        DialogRoot {
                            open: non_dismiss_open,
                            onclose: move |_| {},
                            dismissible: false,
                            DialogTitle { "Non-dismissible dialog" }
                            DialogDescription {
                                "ESC and backdrop click do nothing. Only the explicit Close button resolves this."
                            }
                            DialogBody {}
                            DialogFooter {
                                Button {
                                    variant: ButtonVariant::Primary,
                                    onclick: move |_| non_dismiss_open.set(false),
                                    "Close"
                                }
                            }
                        }

                        DialogRoot {
                            open: backdrop_open,
                            onclose: move |_| {},
                            close_on_backdrop_click: true,
                            DialogTitle { "Close-on-backdrop dialog" }
                            DialogDescription { "Click outside the panel to close." }
                            DialogBody {}
                            DialogFooter {
                                Button {
                                    variant: ButtonVariant::Primary,
                                    onclick: move |_| backdrop_open.set(false),
                                    "Close"
                                }
                            }
                        }

                        DialogRoot {
                            open: scroll_open,
                            onclose: move |_| {},
                            DialogTitle { "Scrollable body" }
                            DialogDescription { "Long content scrolls inside the body region." }
                            DialogBody {
                                {
                                    let lines = (1..=80).map(|i| rsx! { p { "Line {i}" } });
                                    rsx! { for el in lines { {el} } }
                                }
                            }
                            DialogFooter {
                                Button {
                                    variant: ButtonVariant::Primary,
                                    onclick: move |_| scroll_open.set(false),
                                    "Close"
                                }
                            }
                        }
                    }
```

- [ ] **Step 4: Verify the gallery interactively**

```bash
dx serve --example component_gallery --platform desktop
```

For each button:
- **Basic** → opens. Tab cycles between Close button and the dialog itself. ESC closes. Click on the backdrop (gray area) does NOT close. Focus returns to the trigger button on close.
- **Non-dismissible** → opens. ESC does nothing. Backdrop click does nothing. Only the Close button closes.
- **Close-on-backdrop** → opens. Click outside the panel closes. ESC also closes (default `dismissible: true`).
- **Scrollable body** → opens. Body scrolls vertically; the footer (Close button) stays anchored at the bottom of the panel.

Open DevTools (right-click → Inspect Element if WebView2 allows) and verify:
- The `<dialog>` element has `aria-modal="true"` while open (browser-set).
- `aria-labelledby` resolves to the visible `<h2>` title.
- `aria-describedby` resolves to the description `<p>`.

- [ ] **Step 5: Build the example**

```bash
cargo build --example component_gallery -p inputforge-gui-dx
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/examples/component_gallery.rs
git commit -m "feat(gui-dx): add Dialog primitives gallery section"
```

---

## Task 12: `DirtyConfirmDialog` patterns module

Presentational composer of the dialog compound: title, description, and three buttons (Cancel/Discard/Save) in fixed order. Cancel-first for default focus; ESC routes to `oncancel`; `close_on_backdrop_click: false`.

**Files:**
- Create: `crates/inputforge-gui-dx/src/patterns/mod.rs`
- Create: `crates/inputforge-gui-dx/src/patterns/dirty_confirm.rs`
- Modify: `crates/inputforge-gui-dx/src/lib.rs` (add `pub mod patterns;`)

- [ ] **Step 1: Create `crates/inputforge-gui-dx/src/patterns/mod.rs`**

```rust
//! Reusable composed-component patterns. F4 ships only `DirtyConfirmDialog`;
//! later features may add `SaveBeforeLeave`, `ConfirmDestructive`, etc.

pub mod dirty_confirm;

pub use dirty_confirm::DirtyConfirmDialog;
```

- [ ] **Step 2: Create `crates/inputforge-gui-dx/src/patterns/dirty_confirm.rs`**

```rust
//! Presentational dirty-state confirmation dialog. Cancel/Discard/Save in
//! fixed document order so `showModal()`'s default-focus rule lands on
//! Cancel (the safe default — destructive-confirmation a11y guidance).
//!
//! ESC routes to `oncancel` (matches Cancel button). `close_on_backdrop_click`
//! is hard-coded to `false` — destructive dialogs should not close on a stray
//! click outside the panel.

use dioxus::prelude::*;

use crate::components::{
    Button, ButtonVariant, DialogBody, DialogDescription, DialogFooter, DialogRoot, DialogTitle,
};

#[derive(Clone, PartialEq, Props)]
pub struct DirtyConfirmDialogProps {
    /// Controlled open state. The component flips this to `false` on every
    /// resolution path (Cancel/Discard/Save) and fires the matching callback.
    pub open: Signal<bool>,

    /// Title — defaults to "Unsaved Changes".
    #[props(default)]
    pub title: Option<String>,
    /// Description — defaults to
    /// "You have unsaved changes. What would you like to do?".
    #[props(default)]
    pub message: Option<String>,
    /// Save button label — defaults to "Save". Future consumers may pass
    /// "Save & Switch", "Save & Close", etc.
    #[props(default)]
    pub save_label: Option<String>,

    pub oncancel: EventHandler<()>,
    pub ondiscard: EventHandler<()>,
    pub onsave: EventHandler<()>,

    #[props(default)]
    pub class: Option<String>,
}

#[component]
pub fn DirtyConfirmDialog(props: DirtyConfirmDialogProps) -> Element {
    let title = props.title.as_deref().unwrap_or("Unsaved Changes");
    let message = props
        .message
        .as_deref()
        .unwrap_or("You have unsaved changes. What would you like to do?");
    let save_label = props.save_label.as_deref().unwrap_or("Save");

    let mut open = props.open;
    let cancel = props.oncancel;
    let discard = props.ondiscard;
    let save = props.onsave;

    rsx! {
        DialogRoot {
            open: open,
            // ESC routes to Cancel — matches default-focus and safe-default
            // semantics. The dialog's own onclose handler fires after the
            // browser closes the <dialog>.
            onclose: move |_| {
                open.set(false);
                cancel.call(());
            },
            dismissible: true,
            close_on_backdrop_click: false,
            class: props.class,

            DialogTitle { "{title}" }
            DialogDescription { "{message}" }
            DialogBody {} // empty — Description carries the body content
            DialogFooter {
                // Cancel first → receives showModal()'s default focus.
                Button {
                    variant: ButtonVariant::Secondary,
                    onclick: move |_| {
                        open.set(false);
                        cancel.call(());
                    },
                    "Cancel"
                }
                Button {
                    variant: ButtonVariant::Danger,
                    onclick: move |_| {
                        open.set(false);
                        discard.call(());
                    },
                    "Discard"
                }
                Button {
                    variant: ButtonVariant::Primary,
                    onclick: move |_| {
                        open.set(false);
                        save.call(());
                    },
                    "{save_label}"
                }
            }
        }
    }
}
```

- [ ] **Step 3: Add `pub mod patterns;` to `crates/inputforge-gui-dx/src/lib.rs`**

Add after the existing `pub mod theme;` line (around line 12):

```rust
pub mod patterns;
```

- [ ] **Step 4: Build and confirm clean**

```bash
cargo build -p inputforge-gui-dx
```

Expected: PASS, warnings ≤ baseline. `DirtyConfirmDialog` is unused — gallery (Task 13) consumes it.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/patterns/mod.rs \
        crates/inputforge-gui-dx/src/patterns/dirty_confirm.rs \
        crates/inputforge-gui-dx/src/lib.rs
git commit -m "feat(gui-dx): add DirtyConfirmDialog pattern"
```

---

## Task 13: Gallery — `DirtyConfirmDialog` section

Two buttons: default-copy variant and custom-copy variant. Three captions next to each button light up to confirm each callback fires exactly once per resolution.

**Files:**
- Modify: `crates/inputforge-gui-dx/examples/component_gallery.rs`

- [ ] **Step 1: Add the import**

Append to the existing `use inputforge_gui_dx::components::{...}` block (it doesn't expose `patterns`), and add a separate import line:

```rust
use inputforge_gui_dx::patterns::DirtyConfirmDialog;
```

- [ ] **Step 2: Add signals in `gallery_root`**

After the four dialog signals from Task 11:

```rust
    let mut dirty_a_open = use_signal(|| false);
    let mut dirty_b_open = use_signal(|| false);
    let mut dirty_a_outcome = use_signal(|| String::new());
    let mut dirty_b_outcome = use_signal(|| String::new());
```

- [ ] **Step 3: Add the section**

Insert this block after the Dialog primitives section:

```rust
                    section {
                        h2 { "DirtyConfirmDialog" }
                        Card { padding: CardPadding::Md,
                            Stack { gap: "--space-3".to_owned(),
                                Cluster { gap: "--space-3".to_owned(),
                                    Button {
                                        onclick: move |_| {
                                            dirty_a_outcome.set(String::new());
                                            dirty_a_open.set(true);
                                        },
                                        "Switch input (dirty)"
                                    }
                                    span {
                                        style: "color: var(--color-text-muted);",
                                        "Last outcome: "
                                        code { "{dirty_a_outcome.read()}" }
                                    }
                                }
                                Cluster { gap: "--space-3".to_owned(),
                                    Button {
                                        onclick: move |_| {
                                            dirty_b_outcome.set(String::new());
                                            dirty_b_open.set(true);
                                        },
                                        "With custom copy"
                                    }
                                    span {
                                        style: "color: var(--color-text-muted);",
                                        "Last outcome: "
                                        code { "{dirty_b_outcome.read()}" }
                                    }
                                }
                            }
                        }

                        DirtyConfirmDialog {
                            open: dirty_a_open,
                            oncancel:  move |_| dirty_a_outcome.set("cancel".to_owned()),
                            ondiscard: move |_| dirty_a_outcome.set("discard".to_owned()),
                            onsave:    move |_| dirty_a_outcome.set("save".to_owned()),
                        }

                        DirtyConfirmDialog {
                            open: dirty_b_open,
                            title: Some("Switch profile?".to_owned()),
                            message: Some(
                                "Profile X has unsaved mappings. Saving applies them now.".to_owned()
                            ),
                            save_label: Some("Save & Switch".to_owned()),
                            oncancel:  move |_| dirty_b_outcome.set("cancel".to_owned()),
                            ondiscard: move |_| dirty_b_outcome.set("discard".to_owned()),
                            onsave:    move |_| dirty_b_outcome.set("save".to_owned()),
                        }
                    }
```

- [ ] **Step 4: Verify interactively**

```bash
dx serve --example component_gallery --platform desktop
```

- Click "Switch input (dirty)" → dialog opens. Initial focus is on Cancel (visible focus ring; Tab moves to Discard, then Save, then back).
- Press ESC → caption shows "cancel"; dialog closes.
- Re-open. Click Discard → caption shows "discard"; closed.
- Re-open. Click Save → caption shows "save"; closed.
- Click "With custom copy" → title is "Switch profile?", body is the custom message, primary button reads "Save & Switch".
- Repeat the three resolutions; outcomes match.
- Click outside the panel while a dirty-confirm is open → nothing happens (`close_on_backdrop_click: false`).

- [ ] **Step 5: Build the example**

```bash
cargo build --example component_gallery -p inputforge-gui-dx
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/examples/component_gallery.rs
git commit -m "feat(gui-dx): add DirtyConfirmDialog gallery section"
```

---

## Task 14: `impeccable:frontend-design` — toast & dialog visual treatment

The infrastructure renders. Now invoke `impeccable:frontend-design` against rendered screenshots, scoped narrowly to visual presentation of the new primitives. Apply the resulting CSS revisions to `toast.css` and `dialog.css`.

**Files:**
- Modify: `crates/inputforge-gui-dx/assets/toast/toast.css`
- Modify: `crates/inputforge-gui-dx/assets/components/dialog.css`

- [ ] **Step 1: Capture screenshots of the gallery in its current (stub) state**

Run `dx serve --example component_gallery --platform desktop`. Screenshot:
- Gallery scrolled to the Toasts section with one of each level (info, success, warning, error) visible. Click each push button in turn so all four are stacked, then capture before any auto-dismiss.
- Toast spam state: 5 visible toasts after "Push 7 distinct".
- Dialog Basic — visible.
- Dialog Non-dismissible — visible.
- Dialog Scrollable body (mid-scroll inside).
- DirtyConfirmDialog — visible.

Save screenshots somewhere accessible (project-relative `tmp/` or attach when invoking the skill).

- [ ] **Step 2: Invoke `impeccable:frontend-design`**

Brief (paste verbatim into the skill input):

> **Scope:** Visual presentation of the F4 toast and dialog primitives in the InputForge Dioxus gallery. Crate: `crates/inputforge-gui-dx`. Files in scope: `assets/toast/toast.css`, `assets/components/dialog.css`. Out of scope: any `.rs` file (Rust component contracts are fixed); information architecture; F2 design tokens (those are stable).
>
> **Inputs:**
> - Screenshots of: four toast levels stacked; spam state with 5 visible toasts; four dialog variants (basic, non-dismissible, scrollable body, close-on-backdrop); DirtyConfirmDialog with three buttons.
> - Tokens to use: `--color-info-bg/-bg`, `--color-success-bg`, `--color-warning-bg`, `--color-error-bg`, `--color-info`, `--color-live`, `--color-warning`, `--color-error`, `--shadow-3`, `--space-1..6`, `--radius-md`, `--font-sans`, `--color-bg-elevated`, `--color-bg-overlay`, `--color-border`, `--color-text`, `--color-text-muted`. New token allowed: `--z-toast` (already added).
> - Elevation philosophy (project-wide): borders-over-shadows; real shadow blur reserved for genuine overlays. Both toasts and the dialog panel are genuine overlays — `--shadow-3` is appropriate. Backdrop is allowed (already `--color-bg-overlay`).
> - Existing F2 conventions: `.if-<name>` BEM-ish prefix; `--<level>` modifiers; per-level accent surfaces and borders mirror the Badge primitive (see `assets/components/badge.css`).
>
> **What I want:**
> - Toast chrome: surface treatment (per-level accent), accent bar (left edge or top edge — your call), icon size/position relative to the message text, count badge styling (`×N`), close button affordance, hover/focus visual states.
> - Toast motion: fade-in on mount; fade-out on expire/dismiss (the Rust side stops including dismissed toasts in the snapshot — CSS handles the fade). `transition-behavior: allow-discrete` + `@starting-style` are fine for Dioxus 0.7 / WebView2.
> - Dialog: panel surface chrome, backdrop treatment (subtle blur or stay solid?), title/description/body/footer rhythm, footer button alignment, focused-button affordance, motion (fade-in on open; if any motion on close).
>
> **Constraints:**
> - Cascade order in `theme/mod.rs` is fixed; do not assume rules can re-order CSS files.
> - `toast.css` is mounted LAST (overlay layer); `dialog.css` is mounted with the rest of component CSS.
> - Two stacked viewport divs (`.if-toast-viewport--polite` and `.if-toast-viewport--assertive`) currently render at the same `top: 12px`. If you need different anchors, adjust the CSS rules but keep the two-region structure (Rust contract).
> - `pointer-events: none` on `.if-toast-viewport` and `pointer-events: auto` on `.if-toast` are load-bearing — do not remove them.
> - The `<dialog>` element's `:modal` and `::backdrop` pseudo-classes are the canonical hooks; native browser CSS reset is patchy across versions, so keep the `border: none; padding: 0; background: transparent` resets on `.if-dialog`.
> - WebView2 (Chromium) supports `@starting-style` and `transition-behavior: allow-discrete`; both are fine to use.
>
> **Deliverable:** revised `toast.css` and `dialog.css` content blocks (full file contents, not diffs), suitable for direct paste into the existing files.

- [ ] **Step 3: Apply the revised CSS**

Paste the skill output into `crates/inputforge-gui-dx/assets/toast/toast.css` and `crates/inputforge-gui-dx/assets/components/dialog.css` as full file replacements. Do NOT edit the Rust side — class names are contract.

If the output uses class names not in the Rust component (e.g., a new `.if-toast__icon` wrapper), add a doc-note in the CSS about which Rust element it targets, but do not change Rust. If a class is structurally needed and CSS-only is insufficient, raise it as a follow-up note (not a blocker for this task).

- [ ] **Step 4: Verify visually**

Re-run `dx serve --example component_gallery --platform desktop`. Walk through:
- All four toast levels look distinctly typed (clear accent per level).
- Toast spam state — 5 visible toasts read as a stack, not a wall.
- Dialog Basic — the panel and backdrop read as a "pause the world" overlay.
- Scrollable body — body region scrolls; footer stays anchored.
- DirtyConfirmDialog — Cancel button reads visually as the safe choice (Secondary), Discard as destructive (Danger styling), Save as the primary action.

If anything looks broken (overlap, clipped content, focus ring missing), restore the previous file from git and re-invoke the skill with a tighter brief mentioning the specific issue.

- [ ] **Step 5: Build the gallery one more time**

```bash
cargo build --example component_gallery -p inputforge-gui-dx
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/assets/toast/toast.css \
        crates/inputforge-gui-dx/assets/components/dialog.css
git commit -m "style(gui-dx): apply frontend-design pass for toast and dialog"
```

The commit type is `style` per Conventional Commits — visual revisions only, no logic change. (Invoke `conventional-commits` skill before commit.)

---

## Task 15: `warnings_bridge` module

Production subscriber: `use_effect` closure that observes `ctx.meta.warnings` and pushes new tail entries as Warning-level toasts. Idempotent under spurious re-runs via length-diff guard. Re-baselines on length-decrease (engine resets warnings).

**Files:**
- Create: `crates/inputforge-gui-dx/src/toast/warnings_bridge.rs`
- Modify: `crates/inputforge-gui-dx/src/toast/mod.rs` (add `pub(crate) mod warnings_bridge; pub(crate) use warnings_bridge::install_warnings_bridge;`)

- [ ] **Step 1: Create `crates/inputforge-gui-dx/src/toast/warnings_bridge.rs`**

```rust
//! Bridges `MetaSnapshot.warnings` (engine-side, polled every 16 ms by
//! `bridge::spawn_polling_task`) into the toast queue. The polling task
//! gates writes via `PartialEq` so this `use_effect` only re-runs on actual
//! snapshot changes; the length-diff guard makes spurious re-runs (e.g.
//! `engine_status` flip without a new warning) idempotent.

use dioxus::prelude::*;

use crate::context::AppContext;
use crate::toast::{ToastLevel, ToastQueue};

/// Returns a closure suitable for `use_effect`. Watches `ctx.meta` for new
/// tail entries on `warnings` and pushes them as Warning-level toasts.
///
/// `last_seen` is initialized by the caller to `ctx.meta.peek().warnings.len()`
/// so the first run is a no-op even if warnings accumulated before mount.
///
/// `Signal<T>` is `Copy`; the closure rebinds `last_seen` as `mut` so it can
/// call `.set(...)`. The same shape is used by the F1 polling task in
/// `bridge.rs`.
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
            // Engine cleared/reset warnings — re-baseline.
            seen.set(len);
        }
    }
}
```

- [ ] **Step 2: Update `crates/inputforge-gui-dx/src/toast/mod.rs`**

Add the module and re-export. Final state of the file:

```rust
//! Toast queue: pure-data state + Signal wrapper + viewport component +
//! production warnings bridge.

pub(crate) mod state;
pub(crate) mod queue;
pub(crate) mod viewport;
pub(crate) mod warnings_bridge;

pub use queue::ToastQueue;
pub use state::{is_expired, Toast, ToastLevel, ToastState, TOAST_DURATION, TOAST_MAX_VISIBLE};
pub use viewport::ToastViewport;
pub(crate) use warnings_bridge::install_warnings_bridge;
```

(Note: `ToastViewport` was already `pub use` from Task 8 with `#[doc(hidden)]` at the crate root — keep it `pub use` here to allow the example crate to import it.)

- [ ] **Step 3: Build and confirm clean**

```bash
cargo build -p inputforge-gui-dx
```

Expected: PASS, warnings ≤ baseline. `install_warnings_bridge` is unused — Task 16 wires it.

- [ ] **Step 4: Commit**

```bash
git add crates/inputforge-gui-dx/src/toast/warnings_bridge.rs \
        crates/inputforge-gui-dx/src/toast/mod.rs
git commit -m "feat(gui-dx): add warnings bridge module"
```

---

## Task 16: Wire `ToastQueue` and `warnings_bridge` into `app_root`

Production wiring. Install `ToastQueue` context as a sibling of `AppContext`; install the warnings bridge via `use_effect`; render `ToastViewport` as a sibling of `PlaceholderShell` inside `ThemeProvider`. After this task, `cargo run --no-default-features --features gui-dioxus` against an engine that emits warnings produces real Warning-level toasts.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/app.rs`

- [ ] **Step 1: Update imports**

Add to the imports block at the top of `app.rs`:

```rust
use crate::toast::{install_warnings_bridge, ToastQueue, ToastState, ToastViewport};
```

- [ ] **Step 2: Insert the queue installation**

After the `use_context_provider(|| ctx.clone());` line (currently `app.rs:31`) and BEFORE `use_hook(|| spawn_polling_task(ctx.clone()));`, insert:

```rust
    // F4: ToastQueue context — Signal lives in app_root's scope, mirroring the
    // F1 AppContext pattern. Calling Signal::new() outside a hook leaks per
    // dioxus-signals/src/signal.rs:30-52, so use_signal is mandatory here.
    let toast_state = use_signal(ToastState::default);
    let toast_queue = ToastQueue { state: toast_state };
    use_context_provider(|| toast_queue);

    // F4: warnings bridge — reads ctx.meta, pushes new tail entries as
    // Warning toasts. last_seen initializes from peek() so first run is a
    // no-op even if warnings accumulated before mount.
    let last_seen = use_signal(|| ctx.meta.peek().warnings.len());
    use_effect(install_warnings_bridge(ctx.clone(), toast_queue, last_seen));
```

`ToastQueue::state` is `pub` (Task 6) so this construction compiles in any consumer.

- [ ] **Step 3: Render `ToastViewport` as a sibling of `PlaceholderShell`**

Replace the existing `rsx!` block (currently `app.rs:55-57`):

```rust
    rsx! {
        ThemeProvider { PlaceholderShell {} }
    }
```

With:

```rust
    rsx! {
        ThemeProvider {
            ToastViewport {}
            PlaceholderShell {}
        }
    }
```

- [ ] **Step 4: Build the production binary path**

```bash
cargo build -p inputforge-gui-dx
cargo build -p inputforge-app --no-default-features --features gui-dioxus
cargo build -p inputforge-app
```

All three: PASS, warnings ≤ baseline (`inputforge-gui-dx` only — `inputforge-app` is checked for compile, not warning count). Default-feature build (last command) verifies the egui crate is byte-identical (no F4 changes leak into it).

- [ ] **Step 5: Run full test suite**

```bash
cargo test -p inputforge-gui-dx
```

Expected: all tests pass — pure-function `ToastState` tests (9), dialog id-generation tests (2), all existing F1/F2/F3 tests.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/src/app.rs
git commit -m "feat(gui-dx): wire ToastQueue and warnings bridge into app_root"
```

---

## Task 17: README updates

Document the F4 surfaces that future feature plans will consume: ToastQueue API, dialog compound API, dirty-confirm pattern, warnings bridge.

**Files:**
- Modify: `crates/inputforge-gui-dx/README.md`

- [ ] **Step 1: Add the F4 sections**

Open `crates/inputforge-gui-dx/README.md`. Append these sections after the existing F3 sections (before any "Next steps"-style closing block, if present). The exact heading style follows what F2/F3 used in this README.

```markdown
## F4 — Toast & Dialog Infrastructure

### `ToastQueue`

Global toast queue installed via `use_context_provider` in `app_root`. Producers
reach it with:

```rust
use inputforge_gui_dx::{ToastLevel, ToastQueue};

let toasts = use_context::<ToastQueue>();
toasts.push(ToastLevel::Warning, "HidHide unavailable");
```

- **Levels:** `Info`, `Success`, `Warning`, `Error`. Info/Success render in
  `role="status" aria-live="polite"`; Warning/Error in
  `role="alert" aria-live="assertive"`.
- **Dedupe:** identical `(level, message)` against any non-dismissed toast
  increments its count (`×N` badge) and resets the auto-dismiss timer.
- **Cap:** at most 5 visible toasts; FIFO drain of the oldest non-dismissed
  entry on overflow.
- **Auto-dismiss:** 8 s. Hover or focus on a toast pauses the timer; ESC while
  focused dismisses; click × dismisses.

### Dialog primitive

Compound API on the native `<dialog>` element:

```rust
use dioxus::prelude::*;
use inputforge_gui_dx::components::{
    DialogBody, DialogDescription, DialogFooter, DialogRoot, DialogTitle,
};

let mut open = use_signal(|| false);

rsx! {
    DialogRoot {
        open: open,
        onclose: move |_| {},
        DialogTitle { "Title" }
        DialogDescription { "Body description (rendered in a <p>)." }
        DialogBody { /* optional scrollable region */ }
        DialogFooter { /* action buttons */ }
    }
}
```

- **`dismissible: bool` (default `true`)** — when `false`, ESC is suppressed.
  Read once at mount; flipping after mount has no effect.
- **`close_on_backdrop_click: bool` (default `false`)** — backdrop click
  resolves the dialog when `true`.
- Native `<dialog>` provides focus trap, inert background, `aria-modal`, and
  focus restore on close.

### `DirtyConfirmDialog`

Presentational reusable composing the dialog primitives with default copy and
Cancel/Discard/Save buttons. Cancel-first for default focus; ESC routes to
`oncancel`; `close_on_backdrop_click: false`.

```rust
use inputforge_gui_dx::patterns::DirtyConfirmDialog;

let mut open = use_signal(|| false);

rsx! {
    DirtyConfirmDialog {
        open: open,
        oncancel:  move |_| { /* abort the action — distinct from discard */ },
        ondiscard: move |_| { /* drop unsaved changes, proceed */ },
        onsave:    move |_| { /* persist, then proceed */ },
    }
}
```

Override `title`, `message`, and `save_label` for context-specific phrasing
(e.g., `save_label: Some("Save & Switch".to_owned())`).

### Warnings bridge

`MetaSnapshot.warnings` (populated by the engine via the F1 polling task)
flows into the toast queue as Warning-level toasts. Producers do not need to
opt in — append to `AppState.warnings` and the bridge handles delivery.
```

- [ ] **Step 2: Verify the README renders sensibly**

```bash
cat crates/inputforge-gui-dx/README.md | head -200
```

Eyeball the new sections — fenced code blocks closed properly, headings consistent with the rest of the file.

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/README.md
git commit -m "docs(gui-dx): document F4 toast and dialog primitives"
```

---

## Task 18: Final acceptance pass — gallery + live engine

Walk every acceptance bullet from the spec. This is the gate before declaring F4 complete.

**Files:**
- Read-only: nothing modified in this task; this is a verification-only sweep with possible follow-up commits if fixes are needed.

- [ ] **Step 1: Build sanity (all three targets)**

```bash
cargo build -p inputforge-gui-dx 2>&1 | tee /tmp/f4-final-warnings.txt
cargo build -p inputforge-app --no-default-features --features gui-dioxus
cargo build -p inputforge-app
```

All three: PASS.

Compare the gui-dx warning count to the baseline from Task 1 Step 2:

```bash
WARN_NOW=$(grep -c "^warning:" /tmp/f4-final-warnings.txt)
WARN_BASE=$(cat /tmp/f4-baseline-count.txt)
echo "baseline=$WARN_BASE now=$WARN_NOW"
test "$WARN_NOW" -le "$WARN_BASE" || echo "FAIL: warnings regressed"
```

If `FAIL`, investigate. Common causes: a `dead_code` warning from leftover unused export. Fix and recommit.

- [ ] **Step 2: Test sanity**

```bash
cargo test -p inputforge-gui-dx
```

All pure-function tests pass: `toast::state::tests::*` (9 tests) and `components::dialog::tests::*` (2 tests), plus everything from F1/F2/F3.

- [ ] **Step 3: Gallery — Toasts acceptance pass**

```bash
dx serve --example component_gallery --platform desktop
```

Walk through:
- Push Info / Success / Warning / Error → each renders with the correct accent and level icon.
- Push spam (×10) → ONE toast with `×10`.
- Push 7 distinct → exactly 5 visible; oldest two missing.
- Click ×, ESC-while-focused both dismiss.
- Hover / focus pauses the timer (toast stays past 8 s).
- Tab key reaches each toast; focus visible.
- Click in the empty viewport area → click reaches the underlying gallery section (verify by clicking through onto a Button in the Toasts section).

- [ ] **Step 4: Gallery — Dialogs acceptance pass**

- All four dialog demos open via `showModal()`.
- Focus moves into the dialog on open (default focus on the first focusable element — the Close button for basic/scrollable, the Save button… wait — the basic dialog only has Close, so focus lands there).
- Tab cycles inside; ESC closes the dismissible variants; ESC is no-op on the non-dismissible.
- Backdrop click does nothing on basic / non-dismissible / scrollable; closes the close-on-backdrop variant.
- Focus returns to the trigger button on close.

Open WebView2 DevTools (right-click → Inspect, if available; otherwise rely on the visible behavior). Verify the structural ARIA contracts:
- `<dialog>` carries `aria-modal="true"` while open (browser-set).
- `aria-labelledby` resolves to the visible `<h2>` title.
- `aria-describedby` resolves to the description `<p>` (or is dangling for dialogs that omit `DialogDescription` — F4's gallery dialogs all include it).

- [ ] **Step 5: Gallery — DirtyConfirmDialog acceptance pass**

- Open the default-copy variant. Initial focus on Cancel.
- ESC fires `oncancel` (caption "cancel"); dialog closes.
- Re-open, click each of Cancel / Discard / Save in turn → caption matches the resolution; `open` Signal returns to `false` after each.
- Custom-copy variant displays the overridden title, message, and save-label.

- [ ] **Step 6: ARIA contracts in DevTools accessibility inspector**

Where DevTools is available (WebView2 in dev mode supports it via right-click → Inspect):
- Toast viewport: ONE `role="status" aria-live="polite"` region (info+success), ONE `role="alert" aria-live="assertive"` region (warning+error).
- Each toast item: `tabindex="0"`. Close button: `aria-label="Dismiss"`.
- Dialog: `aria-modal="true"` while open; `aria-labelledby` and `aria-describedby` reference the title/description ids.

- [ ] **Step 7: Live engine — warnings bridge end-to-end**

Run the production app under `gui-dioxus`:

```bash
cargo run -p inputforge-app --no-default-features --features gui-dioxus
```

Trigger a real warning. The simplest reproducible path: launch with HidHide unavailable on the system (or invoke whatever code path the existing egui crate already surfaces as a warning toast — `inputforge-core/src/state.rs::AppState::warnings` is the source of truth). The expected behavior:
- Warning appears as a Warning-level toast within ~1 polling tick (16 ms cadence).
- Identical warning emitted again coalesces into `×2`.
- The F3 status-bar engine-status badge UNCHANGED — F4 does not push toasts on engine-status flips.

If the live engine is unavailable today (e.g., HidHide is fine on this machine and no warnings are emitted), document the constraint in the acceptance log and fall back to a manual injection: temporarily push a dummy warning into `AppState.warnings` from `inputforge-app/src/main.rs` startup (and revert before commit). This is a verification-only step; no production code change ships from this task.

- [ ] **Step 8: F3 ARIA wrapper unchanged**

Open `crates/inputforge-gui-dx/src/shell/status_bar_view.rs:42-46`. Confirm the `<span role="status" aria-live="polite">` wrapper around the engine-status `Badge` is INTACT. F4 must not have touched this.

```bash
grep -A2 'role: "status"' crates/inputforge-gui-dx/src/shell/status_bar_view.rs
```

Expected: the lines exist and read exactly as F3 left them.

- [ ] **Step 9: Z-stack and pointer-events smoke**

Still in the running gallery:
- Open a dialog. Push a toast (e.g., via the Toasts section's Push Warning button — they'd both be visible if the dialog doesn't fully cover the viewport region). The toast paints ABOVE the dialog (intentional; spec confirms).
- Close the dialog. Click somewhere in the upper-right of the gallery where the (now-empty) viewport sits. The click reaches the underlying gallery content (verifies `pointer-events: none` on `.if-toast-viewport`).

- [ ] **Step 10: Egui parity — default features unchanged**

```bash
cargo run -p inputforge-app
```

Visually identical to before F4. The egui crate is in `crates/inputforge-gui` — F4 should not have touched it. Verify:

```bash
git log --since="<F4 start commit>" -- crates/inputforge-gui/
```

Expected: no commits.

- [ ] **Step 11: Acceptance log**

If everything above passed, F4 is complete. Mark all spec acceptance bullets as ☑ in a final commit (or in a brief post-merge note). Otherwise, fix the failing bullets and re-run from Step 1.

```bash
# Optional: a single empty commit signaling F4 acceptance is closed.
git commit --allow-empty -m "chore(gui-dx): close F4 acceptance pass"
```

(Skip the empty commit if the convention in this repo is to gate via PR description instead.)

---

## Task 19: Hand off

After Task 18 passes, F4 is shippable. The successor features (F5+ shell composer, F8 mapping editor, F11 mode editor, F13 calibration, F14 profile manager) consume:
- `use_context::<ToastQueue>().push(level, msg)` — every producer except the warnings bridge.
- `DialogRoot { ... }` and `DirtyConfirmDialog { ... }` — every modal flow.
- The `MetaSnapshot.warnings` → toast bridge — already wired; producers append to `AppState.warnings` and toasts surface automatically.

There is no F4 follow-up work. The next plan is for F5 (shell composer) once that spec lands.

- [ ] **Step 1: Update the parent rewrite spec's status table**

Open `docs/superpowers/specs/2026-04-24-egui-to-dioxus-rewrite-design.md`. Find the F4 row in the foundation-features table; flip its status from "ready for plan" / "in progress" to "complete" (or whatever wording the spec uses). One-line edit.

- [ ] **Step 2: Commit and close**

```bash
git add docs/superpowers/specs/2026-04-24-egui-to-dioxus-rewrite-design.md
git commit -m "docs(rewrite): mark F4 complete"
```

---

## Self-review — items checked against spec

(Reviewer notes — recorded here, not as a task; written at plan-completion time.)

- **Spec coverage:** Each numbered design choice (1–13) maps to at least one task. Production wiring scope (1) → Tasks 15–16. ToastQueue context (2) → Tasks 6, 16. Compound dialog (3) → Tasks 10–11. DirtyConfirmDialog (4) → Tasks 12–13. Cancel/Discard/Save order (5) → Task 12. Four levels (6) → Task 2. Exact-string dedupe (7) → Task 3. Cap + FIFO (8) → Task 4. Per-level ARIA (9) → Task 7. Engine-status coexistence (10) → Task 18 Step 8. Hover/focus + ESC (11) → Tasks 5, 7. Top-right position (12) → Tasks 7, 14. Frontend-design pass (13) → Task 14.
- **Acceptance bullets:** every bullet from the spec has a corresponding step in Task 18.
- **Type consistency:** `ToastQueue { state: Signal<ToastState> }` consistent across `queue.rs`, `app.rs`, `gallery_root`. `EventHandler<()>` on `oncancel`/`ondiscard`/`onsave` matches the `Button` precedent. JS shim names (`DIALOG_OPEN_JS` / `DIALOG_CLOSE_JS` / `DIALOG_ATTACH_CANCEL_JS`) consistent across `dialog.rs` snippets.
- **No placeholders:** every step contains the actual content — code blocks where code is needed, exact commands with expected outcomes, file paths verified against the F1/F2/F3 layout.
