# F3 — Application Shell + Tray Bridge — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `docs/superpowers/specs/2026-04-26-f3-app-shell-tray-bridge-design.md`

**Goal:** Replace F1's polling tray workaround with a Dioxus-native event-driven flow, restore the full hide-to-tray window lifecycle, and stand up an intentionally-disposable placeholder shell with two new design-system primitives (Tabs, StatusBar). After F3, `--features gui-dioxus` is production-viable as a future default at F14.

**Architecture:** A `Config::with_custom_event_handler` closure observes `tao::event::Event::UserEvent(UserWindowEvent::MudaMenuEvent(_))` (which `dioxus-desktop` 0.7.6 already forwards from its own muda handler), routes through a bounded `tokio::sync::mpsc` channel, and is drained by a Dioxus task spawned from `app_root`. Window lifecycle uses `WindowCloseBehaviour::WindowHides` for X-click hide-to-tray and a per-window `set_close_behavior(WindowCloses) + close()` flip for tray Quit, leaving `exit_on_last_window_close` at its default `true`. The placeholder shell is a four-region grid (top / left / center / status) explicitly disposable at F5; the StatusBar primitive composes signal-bound badges driven by `MetaSnapshot` / `ConfigSnapshot`.

**Tech Stack:** Rust 2024 / rustc 1.85, Dioxus 0.7.6 (desktop / wry / WebView2 / tao), `tokio::sync::mpsc` (bounded channel), `muda` 0.17 (`MenuEvent`, `MenuId`), `parking_lot` (already present), F2 design-system primitives.

---

## Context

F1 shipped a state-bridge scaffold for `crates/inputforge-gui-dx` and punted on tray and lifecycle: today the egui crate still polls `MenuEvent::receiver().try_recv()` per frame, and under `--features gui-dioxus` the X-click closes the window for good (no hide-to-tray, no second `launch_gui` because `tao::EventLoop::run` is one-shot).

F2 shipped the design-system foundation: tokens, `ThemeProvider`, 17 primitives, gallery harness. F3 builds on F2 — Tabs and StatusBar are added as F2-style primitives, the placeholder shell composes them, and the gallery grows two more sections.

**Why this is worth a plan rather than just "hack it in":**
- **The tray bridge is structural.** Dioxus 0.7.6 unconditionally registers `muda::MenuEvent::set_event_handler` itself and forwards every event as `UserWindowEvent::MudaMenuEvent`. F3 must observe via `Config::with_custom_event_handler` (not `set_event_handler` — Dioxus would clobber it on next launch tick).
- **Lifecycle is delicate.** Close-requested handling is owned by Dioxus; F3 cannot pre-cancel it. The Quit pathway therefore inverts the question: flip per-window `WindowCloseBehaviour::WindowCloses` then call `close()`. Get this wrong and the process never exits, or `HidHide` unhide / `vJoy` release never fires.
- **The placeholder shell sets F5's mutability budget.** No `AppShell` abstraction. The grid and its CSS file (`assets/shell/placeholder-shell.css`) are explicitly disposable at F5. F3 commits to NOT building a layout primitive that F5 will then need to undo.

Outcome at F3: `cargo run --no-default-features --features gui-dioxus` opens a 1280×800 window with a four-region placeholder shell. X-click hides it; tray Show re-opens; tray Toggle flips engine status (status-bar badge updates within ~16ms); tray Quit cleanly exits the process and runs `shutdown()` so `HidHide` unhide and `vJoy` release fire via `Drop`. `cargo run` (default features = egui) is byte-identical to today.

---

## Critical files to modify

All paths relative to `E:\Git\Perso\inputforge\` unless otherwise noted.

**Created (in `crates/inputforge-gui-dx/`):**

- `src/tray/mod.rs` — `make_event_handler`, `spawn_listener_task`, `dispatch_toggle`, `CHANNEL_CAPACITY`
- `src/tray/action.rs` — `TrayAction`, `TrayMenuIds`, `from_id`, `from_event`, unit tests
- `src/lifecycle/mod.rs` — `show_window`, `request_quit`, `apply_start_minimized` (no unit tests; behavior covered by lifecycle scenarios)
- `src/shell/mod.rs` — `pub(crate) PlaceholderShell` re-export
- `src/shell/placeholder.rs` — `PlaceholderShell` (the four-region grid; explicitly disposable at F5)
- `src/shell/status_bar_view.rs` — `StatusBarView` (the only F3 surface bound to AppContext signals)
- `src/components/tabs.rs` — Tabs F2-style primitive (full ARIA, keyboard nav)
- `src/components/status_bar.rs` — StatusBar F2-style primitive (presentation only; ARIA-neutral wrapper)
- `assets/components/tabs.css` — Tabs CSS
- `assets/components/status-bar.css` — StatusBar CSS
- `assets/shell/placeholder-shell.css` — disposable shell-scoped grid CSS

**Modified (in `crates/inputforge-gui-dx/`):**

- `src/lib.rs` — extend `launch_gui` signature with `start_minimized: bool`; thread `tokio::sync::mpsc::channel` through `Config::with_custom_event_handler` + `LaunchParams::listener_rx`; add `mod tray;`, `mod lifecycle;`, `mod shell;`; set `with_close_behaviour(WindowHides)`
- `src/app.rs` — `app_root` consumes `LaunchParams` via `use_context`; spawns listener task with take-once Mutex; calls `apply_start_minimized`; renders `ThemeProvider { PlaceholderShell {} }`
- `src/context.rs` — remove the F1 `#[expect(dead_code, reason = "used in later tasks (engine command dispatch)")]` attribute on `AppContext.commands` (and the sibling `#[expect(dead_code)]` on `settings` if it becomes false too)
- `src/components/mod.rs` — add `pub mod tabs;` + `pub mod status_bar;` modules and re-exports
- `src/theme/mod.rs` — add two `Stylesheet` mounts for `tabs.css` and `status-bar.css`
- `examples/bridge_demo.rs` — adapt to new `launch_gui` signature (add `start_minimized: false`); the spec also asks the dev loop to render the real shell, but `bridge_demo.rs` already calls `launch_gui` directly (which now mounts `PlaceholderShell`), so this is automatic — no `PlaceholderShell` wrap is needed in `bridge_demo.rs`
- `examples/component_gallery.rs` — add Tabs section (variants, states, keyboard demo) and StatusBar section (slots demo + empty-slots demo)
- `README.md` — document `with_custom_event_handler` tray bridge, hide-to-tray lifecycle, new primitives

**Modified (in `crates/inputforge-gui/`):**

- `src/lib.rs` — add `start_minimized: bool` parameter to `launch_gui` (ignored — parity-only, deletes at F16). Extend the existing `#[expect(clippy::needless_pass_by_value, reason = "...")]` annotation if a fresh lint hits.

**Modified (in `crates/inputforge-app/`):**

- `src/main.rs` — delete `IS_GUI_DIOXUS` const + the two F1 cfg-guards in `main()` and `run_tray_loop`; cfg-split startup branch into Shape A; add `#[cfg(feature = "gui-egui")]` on `launch_gui_blocking`, `run_tray_loop`, `drain_stale_gui_events`; pass `cli.start_minimized` to the `launch_gui` call site under both feature flags

**Reused (do not modify):**

- `src/context.rs` types — `RawHandles`, `AppContext`, `MetaSnapshot`, `ConfigSnapshot`, `LiveSnapshot` shapes (only the `#[expect]` attrs are touched)
- `src/bridge.rs` — `spawn_polling_task` (unchanged)
- F2 components: `Badge`, `Separator` (consumed by `StatusBarView`), all gallery-section helpers
- F2 tokens: `--space-3`, `--color-border`, etc. (Task 1 verifies)

---

## Existing utilities to reuse

- **F2's `merge_class(base, variant, caller)`** in `src/components/mod.rs:45-58` — every new primitive uses this for the `class: Option<String>` caller-composition prop. (Confirmed via `Read crates/inputforge-gui-dx/src/components/mod.rs`.)
- **F2's component pattern** — sibling `.rs` + `.css` per primitive, `asset!()` mounted from `theme/mod.rs`, `.if-<name>` BEM-ish class prefix. Cross-reference `src/components/badge.rs` + `assets/components/badge.css` as the canonical small-primitive template.
- **F2's `BadgeVariant` enum** — `Neutral`, `Info`, `Success`, `Warning`, `Error`. `StatusBarView` consumes via `status_to_variant(EngineStatus) -> BadgeVariant`.
- **F2's `SeparatorOrientation::Vertical`** — `StatusBarView` uses for the badge separator.
- **F1's `MetaSnapshot::engine_status` / `current_mode` / `profile_name`** and `ConfigSnapshot::devices` for `StatusBarView`'s memo subscriptions.
- **F1's `bridge_demo.rs` shape** for any new desktop example (no engine, no I/O, hot-reload safe).
- **Tray-icon menu id triple from `inputforge-app/src/tray.rs::AppTray::menu_item_ids()`** — already returns `(MenuId, MenuId, MenuId)` from `tray_icon::menu`, which re-exports `muda::MenuId`. Type-identical to what F3 needs in `TrayMenuIds`.

---

## Dioxus 0.7 / `tao` / `muda` footguns to heed

Surface these in the implementer's mind before they hit them:

- **`AppContext.commands` is `std::sync::mpsc::Sender<EngineCommand>`** (existing F1 type; see `src/context.rs:34`). The spec snippet `ctx.commands.try_send(cmd)` would only compile against `tokio::sync::mpsc::Sender`. Use `let _ = ctx.commands.send(cmd);` instead — std mpsc unbounded `send` returns immediately unless the receiver is dropped. The plan code blocks below reflect this correction.
- **`tokio::sync::mpsc::Sender::try_send` is correct for the tray channel** (Task 9's `make_event_handler`) because that channel uses `tokio::sync::mpsc::channel(CHANNEL_CAPACITY)` — bounded, with a real `try_send`.
- **`dioxus-desktop` re-export paths.** F1 used `dioxus::desktop::{Config, LogicalSize, WindowBuilder}`. Dioxus 0.7 also re-exports the lower layers — verify `dioxus::desktop::tao::event::Event`, `dioxus::desktop::tao::event_loop::EventLoopWindowTarget`, `dioxus::desktop::ipc::UserWindowEvent`, `dioxus::desktop::WindowCloseBehaviour`, `dioxus::desktop::window` work via `cargo check`. If any path doesn't resolve, fall back to `dioxus_desktop::tao::...` etc. (and add `dioxus-desktop = { workspace = true }` to `Cargo.toml`'s `[dependencies]` only if necessary). Prefer `dioxus::desktop::*` to avoid an extra direct dep.
- **`window()` panics outside a Dioxus scope.** `dioxus_desktop::window()` consumes context via `dioxus_core::consume_context()`. The listener task is spawned via `dioxus::prelude::spawn` from inside `app_root`'s `use_hook`, inheriting `ScopeId::ROOT`'s context — every `window()` call from `show_window` / `request_quit` / `apply_start_minimized` resolves correctly because it runs while the runtime is alive.
- **`muda::MenuEvent` field constructor.** In muda 0.17, `MenuEvent` is `pub struct MenuEvent { pub id: MenuId }`, so `MenuEvent { id: MenuId::new("show") }` should compile in `#[cfg(test)] mod tests`. If a future muda revision makes the field private, the unit test should call `TrayAction::from_id(&menu_id, &ids)` directly (`from_event` is a one-line wrapper around `from_id`). The plan structures the routing this way for testability.
- **`Config::with_custom_event_handler` runs AFTER Dioxus's own app-level event handling.** The closure is observe-only — never mutate `ControlFlow`. Cannot pre-cancel close-requested. (See `dioxus-desktop-0.7.6/src/app.rs:201-205` and `:449`.)
- **`WindowCloseBehaviour::WindowHides` is set at launch and consumed by Dioxus's own close-handling.** F3 has no close-handler code path for the X-click case — Dioxus calls `set_visible(false)` natively.
- **`set_close_behavior` is mutable post-launch** via `DesktopService::set_close_behavior` (`desktop_context.rs:177-179`). This is the entire Quit pathway: flip to `WindowCloses`, then call `close()`.
- **`with_context` requires `T: Any + Clone + Send + Sync + 'static`** (`dioxus-0.7.6/src/launch.rs:256`). `Rc<Cell<Option<T>>>` is `!Send + !Sync` and won't compile. Use `std::sync::Arc<std::sync::Mutex<Option<T>>>` for `LaunchParams::listener_rx`.
- **`tokio::sync::mpsc::Receiver` is `!Sync` but `Send`.** Wrapping in `Arc<Mutex<Option<Receiver>>>` is necessary (Sync requirement) and makes the take-once shape explicit: `lock().unwrap().take()` empties the slot so subsequent mounts (e.g. `dx serve` hot-reload of `app_root`) become no-ops rather than double-spawning the listener.
- **`tray_icon::menu::MenuId` IS `muda::MenuId`** — re-exported. So `tray.menu_item_ids()` returns IDs that compare-equal to those carried by `muda::MenuEvent`. F3 needs no conversion.
- **`asset!()` paths must start with `/`** and resolve relative to crate root. `/assets/components/tabs.css` and `/assets/shell/placeholder-shell.css`.
- **`document::Stylesheet` mounts in render order** — Tabs CSS and StatusBar CSS go alongside other component CSS, AFTER tokens and global, BEFORE `placeholder-shell.css` (which is shell-scoped and may override component layout in the shell context). The plan locks the order in Task 5.
- **Egui crate uses `std::sync::mpsc`** for `EngineCommand` — and so does `RawHandles.commands`. Don't conflate with `tokio::sync::mpsc`.

---

## Phase Overview

- **Phase 0** (Task 1) — F2 token name verification.
- **Phase 1** (Tasks 2–6) — Tabs + StatusBar primitives (Rust + minimal CSS) + gallery + ThemeProvider mount. Lets frontend-design see them rendered.
- **Phase 2** (Tasks 7–8) — Capture screenshots and invoke `impeccable:frontend-design` with a single brief covering tab-bar / status-bar / shell-level visual treatment. Apply revised CSS for the two primitives + draft `placeholder-shell.css`.
- **Phase 3** (Tasks 9–11) — Tray bridge: `tray/action.rs` (TDD), `tray/mod.rs`, `lifecycle/mod.rs`.
- **Phase 4** (Tasks 12–13) — `launch_gui` signature changes: parity update on egui side, Dioxus side wires channel + Config builder + `LaunchParams`.
- **Phase 5** (Tasks 14–17) — Shell: `placeholder.rs`, `status_bar_view.rs`, `placeholder-shell.css` finalization, module declarations.
- **Phase 6** (Tasks 18–19) — `app_root` rewrite for `LaunchParams` consumption + `bridge_demo.rs` adaptation.
- **Phase 7** (Task 20) — `main.rs` Shape A divergence + F1 cleanup.
- **Phase 8** (Tasks 21–22) — README updates + manual lifecycle pass S1–S10.

---

## Task 1: Verify F2 token name compatibility

The placeholder-shell CSS sketch in the spec references `var(--color-border)` and `var(--space-3)`. Confirm these resolve to declared tokens before any CSS is written that references them. Tokens are already verified to exist (`assets/tokens/colors.css` line 25, `assets/tokens/spacing.css` line 5), so this task is a 30-second sanity check, not an investigation.

**Files:**
- Read-only: `crates/inputforge-gui-dx/assets/tokens/colors.css`
- Read-only: `crates/inputforge-gui-dx/assets/tokens/spacing.css`
- Read-only: `crates/inputforge-gui-dx/assets/tokens/typography.css`

- [ ] **Step 1: Confirm the four tokens this plan will reference exist**

The shell and primitive CSS will reference: `--color-border`, `--space-3`, `--color-bg`, `--font-sans`, plus standard Badge/Separator tokens already used by F2 components. Confirm the names declared in F2 are exactly:

```bash
grep -E '^\s*--(color-border|space-3|color-bg|font-sans)\b' crates/inputforge-gui-dx/assets/tokens/*.css
```

Expected: at least one declaration line for each name. If F2 used different names (e.g., `--space-md` instead of `--space-3`), record the actual names and update every CSS code block in this plan to match before proceeding. The spec calls this out under "Token compatibility" — do not skip this check.

- [ ] **Step 2: No commit**

Sanity check only.

---

## Task 2: Tabs primitive — Rust file with ARIA and keyboard nav

Pure component logic. CSS is a stub in this task (Task 5 wires it; Task 8 finalizes after frontend-design). The keyboard contract is the load-bearing part — it activates on arrow-key / Home / End and emits `onchange(id)` synchronously. F11 (Modes) reuses this primitive.

**Files:**
- Create: `crates/inputforge-gui-dx/src/components/tabs.rs`

- [ ] **Step 1: Write the Tabs component**

Create `crates/inputforge-gui-dx/src/components/tabs.rs`:

```rust
use dioxus::prelude::*;

use super::merge_class;

#[derive(Clone, PartialEq, Props)]
pub struct TabsProps {
    /// Stable id of the active tab.
    pub value: String,
    pub onchange: EventHandler<String>,
    /// (id, label) pairs in display order.
    pub items: Vec<(String, String)>,
    #[props(default)]
    pub class: Option<String>,
    #[props(default)]
    pub disabled: bool,
}

/// WAI-ARIA Tabs primitive with focus-roving and automatic activation.
///
/// - `role="tablist"` on the wrapper, `role="tab"` per item.
/// - Arrow Left / Right cycles focus AND activates (automatic activation —
///   panel swaps are synchronous and cheap, see spec rationale).
/// - Home / End jumps to first / last (and activates).
/// - `tabindex` is `0` for the active tab and `-1` for the rest (focus-roving).
/// - `disabled` short-circuits keyboard and click; visible state via
///   `.if-tabs--disabled`.
///
/// The component is stateless: the caller owns `value` and renders panel
/// content based on it. F11 (Modes) reuses this — keeping it stateless avoids
/// over-coupling.
#[component]
pub fn Tabs(props: TabsProps) -> Element {
    let combined = merge_class(
        "if-tabs",
        if props.disabled { "if-tabs--disabled" } else { "" },
        props.class.as_deref(),
    );

    let items = props.items.clone();
    let active = props.value.clone();
    let onchange = props.onchange;
    let disabled = props.disabled;

    rsx! {
        div {
            class: "{combined}",
            role: "tablist",
            "aria-orientation": "horizontal",
            for (idx, (id, label)) in items.iter().cloned().enumerate() {
                {
                    let is_active = id == active;
                    let id_for_click = id.clone();
                    let items_for_key = items.clone();
                    rsx! {
                        button {
                            key: "{id}",
                            r#type: "button",
                            class: if is_active { "if-tab if-tab--active" } else { "if-tab" },
                            role: "tab",
                            "aria-selected": "{is_active}",
                            tabindex: if is_active { "0" } else { "-1" },
                            disabled,
                            onclick: move |_| {
                                if !disabled {
                                    onchange.call(id_for_click.clone());
                                }
                            },
                            onkeydown: move |evt| {
                                if disabled { return; }
                                let key = evt.key();
                                let len = items_for_key.len();
                                if len == 0 { return; }
                                let next_idx: Option<usize> = match key {
                                    Key::ArrowRight => Some((idx + 1) % len),
                                    Key::ArrowLeft  => Some((idx + len - 1) % len),
                                    Key::Home       => Some(0),
                                    Key::End        => Some(len - 1),
                                    // Space/Enter on already-active tab: absorb to
                                    // prevent default scroll behavior; no re-emit.
                                    Key::Character(ref s) if s == " " => {
                                        evt.prevent_default();
                                        None
                                    }
                                    Key::Enter => {
                                        evt.prevent_default();
                                        None
                                    }
                                    _ => None,
                                };
                                if let Some(i) = next_idx {
                                    evt.prevent_default();
                                    if let Some((next_id, _)) = items_for_key.get(i) {
                                        onchange.call(next_id.clone());
                                    }
                                }
                            },
                            "{label}"
                        }
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p inputforge-gui-dx`
Expected: builds cleanly. (Tabs is not yet wired into `mod.rs`, so this step proves the file itself is syntactically valid.)

If `Key::Character(ref s)` doesn't compile against the Dioxus 0.7 `Key` enum (variant naming may differ), simplify the Space/Enter absorption to:

```rust
Key::Character(ref s) if s.as_str() == " " => { evt.prevent_default(); None }
```

or fall back to checking `evt.key().to_string()` — adjust to the actual API surface. The point is to absorb Space/Enter, not the exact match form.

- [ ] **Step 3: No commit yet**

The file is orphaned until Task 4 wires it. Defer the commit to bundle Tabs + StatusBar + mod.rs + ThemeProvider together at Task 6.

---

## Task 3: StatusBar primitive — Rust file with three slots

Presentation-only primitive. ARIA-neutral wrapper (no `role="status"` at the primitive level — `role` lives on consumer-controlled inner elements). F11+ may reuse via composition.

**Files:**
- Create: `crates/inputforge-gui-dx/src/components/status_bar.rs`

- [ ] **Step 1: Write the StatusBar component**

Create `crates/inputforge-gui-dx/src/components/status_bar.rs`:

```rust
use dioxus::prelude::*;

use super::merge_class;

/// Three-slot horizontal bar used as a window-level status surface.
///
/// `start` flows left, `end` is right-anchored, `middle` fills the gap.
/// Fixed 28px height (matches today's egui status bar; reviewable by
/// frontend-design).
///
/// **ARIA shape.** The wrapper is intentionally neutral — no `role`, no
/// `aria-label`. `role="status"` is a live region; applying it at the
/// primitive level would make every badge change announce. Consumers add
/// `role="status"` (or `aria-live`) on the *specific* element they want
/// announced (typically a single Badge), or `aria-label` on the wrapper if a
/// labeled landmark is desired.
#[component]
pub fn StatusBar(
    start: Element,
    middle: Element,
    end: Element,
    #[props(default)] class: Option<String>,
) -> Element {
    let combined = merge_class("if-status-bar", "", class.as_deref());
    rsx! {
        div { class: "{combined}",
            div { class: "if-status-bar__start",  {start}  }
            div { class: "if-status-bar__middle", {middle} }
            div { class: "if-status-bar__end",    {end}    }
        }
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p inputforge-gui-dx`
Expected: builds cleanly. (StatusBar is not yet wired into `mod.rs`; this step is just a syntax check.)

- [ ] **Step 3: No commit yet**

Bundled with Tabs at Task 6.

---

## Task 4: Wire mod.rs re-exports for Tabs and StatusBar

**Files:**
- Modify: `crates/inputforge-gui-dx/src/components/mod.rs`

- [ ] **Step 1: Add module declarations and re-exports**

Edit `crates/inputforge-gui-dx/src/components/mod.rs`. The file currently ends at line ~88 with the `tooltip` module and re-exports. Add `status_bar` and `tabs` modules in alphabetical order alongside the others, and re-export `StatusBar`, `Tabs`:

```rust
// In the `pub mod ...;` block, alphabetical insertion:
pub mod status_bar;
pub mod tabs;

// In the `pub use ...;` block, alphabetical insertion (between separator and slider):
pub use status_bar::StatusBar;
// ... and after switch:
pub use tabs::Tabs;
```

Final `pub mod` ordering (alphabetical): badge, button, card, checkbox, field, icon, icon_button, label, layout, menu, number_input, select, separator, slider, spinner, status_bar, switch, tabs, text_input, tooltip.

Final `pub use` ordering follows the same alphabetical pattern, with each item's existing variant types kept on the same line.

- [ ] **Step 2: Verify**

Run: `cargo build -p inputforge-gui-dx`
Expected: builds cleanly. The two new primitives are now reachable from `inputforge_gui_dx::components::{Tabs, StatusBar}`.

- [ ] **Step 3: No commit yet**

Bundled with Tasks 5 and 6 below.

---

## Task 5: Stub CSS for Tabs and StatusBar; mount in ThemeProvider

Minimal CSS so the primitives render in the gallery for frontend-design to look at. Final values land in Task 8.

**Files:**
- Create: `crates/inputforge-gui-dx/assets/components/tabs.css`
- Create: `crates/inputforge-gui-dx/assets/components/status-bar.css`
- Modify: `crates/inputforge-gui-dx/src/theme/mod.rs`

- [ ] **Step 1: Write `tabs.css`**

Create `crates/inputforge-gui-dx/assets/components/tabs.css`:

```css
.if-tabs {
    display: flex;
    flex-direction: row;
    gap: var(--space-1);
    border-bottom: 1px solid var(--color-border);
}

.if-tab {
    appearance: none;
    background: transparent;
    border: 0;
    border-bottom: 2px solid transparent;
    color: var(--color-text-muted, var(--color-text));
    font: inherit;
    padding: var(--space-2) var(--space-3);
    cursor: pointer;
}

.if-tab:hover:not([disabled]) {
    color: var(--color-text);
}

.if-tab--active {
    color: var(--color-text);
    border-bottom-color: var(--color-primary);
}

.if-tab:focus-visible {
    outline: 2px solid var(--color-border-focus);
    outline-offset: -2px;
}

.if-tab[disabled],
.if-tabs--disabled .if-tab {
    opacity: 0.5;
    cursor: not-allowed;
}
```

- [ ] **Step 2: Write `status-bar.css`**

Create `crates/inputforge-gui-dx/assets/components/status-bar.css`:

```css
.if-status-bar {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    height: 28px;
    padding: 0 var(--space-3);
    border-top: 1px solid var(--color-border);
    background: var(--color-bg, transparent);
    gap: var(--space-3);
}

.if-status-bar__start,
.if-status-bar__middle,
.if-status-bar__end {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
}

.if-status-bar__middle {
    justify-content: flex-start;
}

.if-status-bar__end {
    justify-content: flex-end;
}
```

- [ ] **Step 3: Mount both stylesheets in `ThemeProvider`**

Edit `crates/inputforge-gui-dx/src/theme/mod.rs`. Add two `Asset` constants near the bottom of the existing block:

```rust
const TABS_CSS: Asset = asset!("/assets/components/tabs.css");
const STATUS_BAR_CSS: Asset = asset!("/assets/components/status-bar.css");
```

Insert into the rsx! list AFTER the existing component CSS lines (alphabetical position works: insert `Stylesheet { href: TABS_CSS }` after Stylesheet for `SWITCH_CSS` would be alphabetical; the existing list is not strictly alphabetical so just append both at the end of the component block, BEFORE `{children}`):

```rust
        Stylesheet { href: TABS_CSS }
        Stylesheet { href: STATUS_BAR_CSS }

        {children}
```

The exact position within the component-CSS block does not matter for cascade purposes (the `.if-tabs` / `.if-status-bar` selectors don't conflict with other primitives), but appending at the end mirrors the F2 convention of "newest primitives at the bottom."

- [ ] **Step 4: Verify build**

Run: `cargo build -p inputforge-gui-dx`
Expected: builds cleanly.

- [ ] **Step 5: No commit yet**

Bundled with the gallery sections in Task 6.

---

## Task 6: Add Tabs and StatusBar gallery sections

`component_gallery.rs` is the visual harness. Two new sections demonstrate variants and states. The Tabs section must include a working keyboard demo (interactive `use_signal` for `value`).

**Files:**
- Modify: `crates/inputforge-gui-dx/examples/component_gallery.rs`

- [ ] **Step 1: Update imports**

Edit the `use inputforge_gui_dx::components::{...}` import block in `examples/component_gallery.rs`. Add `StatusBar` and `Tabs` (alphabetical insertion):

```rust
use inputforge_gui_dx::components::{
    Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, Card, CardPadding, Checkbox, Cluster,
    Field, Icon, IconButton, InputSize, Label, MenuItem, MenuItems, MenuRoot, MenuTrigger,
    NumberInput, Select, Separator, SeparatorOrientation, Slider, Spinner, SpinnerSize, Stack,
    StatusBar, Switch, Tabs, TextInput, Tooltip, TooltipPlacement,
};
```

- [ ] **Step 2: Add an interactive Tabs demo signal**

Inside `gallery_root()`, alongside the existing `let mut number_demo = use_signal(...)` lines, add:

```rust
    let mut tabs_demo = use_signal(|| "first".to_owned());
```

- [ ] **Step 3: Append the Tabs gallery section**

Add a new `section { ... }` block, placed alphabetically (after `Switch`, before `TextInput`) inside the outer `Stack` of `gallery_root()`:

```rust
                    section {
                        h2 { "Tabs" }
                        Card { padding: CardPadding::Md,
                            Stack { gap: "--space-3".to_owned(),
                                p {
                                    "Active tab: "
                                    code { "{tabs_demo}" }
                                    " — use Left/Right or Home/End to cycle."
                                }
                                Tabs {
                                    items: vec![
                                        ("first".into(),  "First".into()),
                                        ("second".into(), "Second".into()),
                                        ("third".into(),  "Third".into()),
                                    ],
                                    value: tabs_demo.read().clone(),
                                    onchange: move |id: String| tabs_demo.set(id),
                                }
                                p { "Disabled state:" }
                                Tabs {
                                    items: vec![
                                        ("a".into(), "Disabled A".into()),
                                        ("b".into(), "Disabled B".into()),
                                    ],
                                    value: "a".to_owned(),
                                    onchange: move |_: String| {},
                                    disabled: true,
                                }
                            }
                        }
                    }
```

- [ ] **Step 4: Append the StatusBar gallery section**

Add another `section { ... }` block after the Tabs section:

```rust
                    section {
                        h2 { "StatusBar" }
                        Stack { gap: "--space-3".to_owned(),
                            p { "Composed slots (Badge + Separator + Badge / text / span):" }
                            Card { padding: CardPadding::Md,
                                StatusBar {
                                    start: rsx! {
                                        Badge { variant: BadgeVariant::Success, "Running" }
                                        Separator { orientation: SeparatorOrientation::Vertical }
                                        Badge { variant: BadgeVariant::Neutral, "Default" }
                                    },
                                    middle: rsx! { span { "2/3 devices" } },
                                    end: rsx! { span { "Demo Profile" } },
                                }
                            }
                            p { "Empty slots — verifies slot independence:" }
                            Card { padding: CardPadding::Md,
                                StatusBar {
                                    start:  rsx! {},
                                    middle: rsx! {},
                                    end:    rsx! {},
                                }
                            }
                        }
                    }
```

- [ ] **Step 5: Smoke-test the gallery**

Run: `dx serve --example component_gallery --platform desktop`
Expected: window opens, scroll down past existing primitives, see Tabs and StatusBar sections render. Verify:
- Clicking a tab updates the "Active tab:" text.
- Tab key brings focus to the active tab; Left/Right arrows cycle focus and activate.
- Disabled Tabs show but don't respond to clicks or keys.
- StatusBar composed slot demo shows: green badge, vertical separator, neutral badge on left; "2/3 devices" middle; "Demo Profile" right.
- Empty-slots StatusBar renders the bar at fixed height with no content.

Close the window.

- [ ] **Step 6: Verify gui-dioxus app still compiles**

Run: `cargo build -p inputforge-app --no-default-features --features gui-dioxus`
Expected: builds cleanly.

- [ ] **Step 7: Commit**

Invoke the `conventional-commits` skill, then:

```bash
git add crates/inputforge-gui-dx/src/components/tabs.rs \
        crates/inputforge-gui-dx/src/components/status_bar.rs \
        crates/inputforge-gui-dx/src/components/mod.rs \
        crates/inputforge-gui-dx/assets/components/tabs.css \
        crates/inputforge-gui-dx/assets/components/status-bar.css \
        crates/inputforge-gui-dx/src/theme/mod.rs \
        crates/inputforge-gui-dx/examples/component_gallery.rs
git commit -m "feat(gui-dx): add Tabs and StatusBar F2-style primitives"
```

---

## Task 7: Capture screenshots for `impeccable:frontend-design` brief

Frontend-design needs to see what currently renders so it can evolve, not invent in a vacuum.

**Files:**
- Create: `docs/superpowers/assets/f3/dioxus-gallery-tabs.png`
- Create: `docs/superpowers/assets/f3/dioxus-gallery-status-bar.png`
- Create: `docs/superpowers/assets/f3/egui-statusbar.png` (re-screenshot the egui status bar — F2's `egui-main.png` may already cover it, but a focused crop is more useful for the brief)

- [ ] **Step 1: Make the screenshots directory**

```bash
mkdir -p docs/superpowers/assets/f3
```

- [ ] **Step 2: Capture Tabs and StatusBar from `dx serve --example component_gallery`**

Run: `dx serve --example component_gallery --platform desktop`
Use Win+Shift+S (or your screenshot tool) to capture two screenshots:
- The Tabs gallery section, both the active demo and the disabled row.
- The StatusBar gallery section, both demos.

Save as `docs/superpowers/assets/f3/dioxus-gallery-tabs.png` and `docs/superpowers/assets/f3/dioxus-gallery-status-bar.png`.

- [ ] **Step 3: Capture the egui status bar for context**

Run: `cargo run -p inputforge-app` (default features = egui).
Crop a screenshot of the egui status bar (bottom of the window). Save as `docs/superpowers/assets/f3/egui-statusbar.png`.

- [ ] **Step 4: Commit**

Invoke the `conventional-commits` skill, then:

```bash
git add docs/superpowers/assets/f3
git commit -m "docs(superpowers): capture F3 frontend-design brief screenshots"
```

---

## Task 8: Invoke `impeccable:frontend-design` and apply revised CSS

Single brief covering all three F3 visual surfaces. The output revises `tabs.css`, `status-bar.css`, and produces a draft `placeholder-shell.css` that Task 17 will commit.

**Files:**
- Modify: `crates/inputforge-gui-dx/assets/components/tabs.css`
- Modify: `crates/inputforge-gui-dx/assets/components/status-bar.css`
- Create (or stage for Task 17): `crates/inputforge-gui-dx/assets/shell/placeholder-shell.css`

- [ ] **Step 1: Invoke the skill**

Use the `Skill` tool with `impeccable:frontend-design`. Provide this brief verbatim:

> **Task: Polish F3 visual treatment for InputForge Dioxus rewrite — tab bar, status bar, placeholder shell.**
>
> **Context.** F3 is the third foundation feature in the egui→Dioxus rewrite. F2 shipped tokens and 17 primitives. F3 adds two more primitives (Tabs, StatusBar) and an *intentionally disposable* placeholder shell — F5 will redesign IA and may replace the entire grid. Visual direction is "Evolved Glass Cockpit." Serious sim/HOTAS configuration tool, instrument-cluster heritage.
>
> **Inputs.**
> - Spec: `docs/superpowers/specs/2026-04-26-f3-app-shell-tray-bridge-design.md`
> - Tokens (DO NOT rename — values were finalized at F2): `crates/inputforge-gui-dx/assets/tokens/{colors,typography,spacing,radii,elevation,motion}.css`
> - Current placeholder CSS: `crates/inputforge-gui-dx/assets/components/{tabs,status-bar}.css`
> - Screenshots: `docs/superpowers/assets/f3/{dioxus-gallery-tabs,dioxus-gallery-status-bar,egui-statusbar}.png` plus the F2 set at `docs/superpowers/assets/f2/`
>
> **Scope (in scope).**
> 1. **Tab bar treatment.** Active/inactive/hover/focus-visible/disabled states; underline vs. pill vs. segmented; dimensions; spacing; transition feel.
> 2. **Status bar treatment.** Slot rhythm (start/middle/end gutters), height (currently 28px — confirm or revise), border, divider style for the in-slot Separator, badge tightness.
> 3. **Placeholder shell layout.** Four-region grid (top toolbar / left panel / center / status bar). Gridding, gutters, region affordances (borders, subtle elevation, scroll behavior). The shell IS DISPOSABLE — F5 may replace it entirely. Keep the visual just polished enough that a maintainer is not embarrassed by it.
>
> **Out of scope (deferred to later F's).**
> - **IA — F5 owns this.** Do NOT redesign the top toolbar contents, propose a different left-panel structure, or invent a different center-panel hierarchy. Polish the regions as containers.
> - Top toolbar contents (F5), left-panel device list (F6), center mappings/modes (F7+/F11).
> - Toast queue, modal, dirty-state confirmation (F4).
>
> **Constraints (must preserve).**
> - All token NAMES from F2 are stable. Use them; don't rename them.
> - ARIA contracts on Tabs (`role="tablist"`/`tab`, `aria-selected`, `tabindex` 0|-1, focus-roving) and StatusBar (`role`-neutral wrapper).
> - StatusBar three-slot layout: start / middle / end (in DOM order; `end` right-anchored).
> - Tabs `disabled` short-circuits and visually de-emphasizes.
> - Dark theme only; light theme deferred.
>
> **Output.** Revised `tabs.css` and `status-bar.css` (NOT renaming any class), plus a draft `placeholder-shell.css` for the four-region grid (selectors `.if-placeholder-shell`, `.if-placeholder-shell__top`, `.if-placeholder-shell__left`, `.if-placeholder-shell__center`, `.if-placeholder-shell__status`, `.if-placeholder-shell__center-body`). Each file with a one-paragraph rationale at the top in CSS comments. .impeccable.md.

- [ ] **Step 2: Apply revised `tabs.css` and `status-bar.css`**

Replace the two files with frontend-design's output. Keep the class names exactly. Token references (`var(--...)`) must reference names declared in F2's token files; if frontend-design suggests a value that needs a NEW token, push back — token names are F2 frozen, and F3 must not coin new ones.

- [ ] **Step 3: Stage the draft `placeholder-shell.css`**

Save the draft `placeholder-shell.css` somewhere temporary (e.g., your scratch dir) — Task 17 will write it into `crates/inputforge-gui-dx/assets/shell/placeholder-shell.css` together with shell HTML. Alternatively, write it to the repo path now (`mkdir -p crates/inputforge-gui-dx/assets/shell` first), and Task 17 will only need to reference it.

To keep Task 17 self-contained, **prefer the latter** — write the draft now into the repo and only stage it at Task 17's commit. Use:

```bash
mkdir -p crates/inputforge-gui-dx/assets/shell
```

Write the draft to `crates/inputforge-gui-dx/assets/shell/placeholder-shell.css`. Do NOT git-add it in this task.

- [ ] **Step 4: Smoke-test gallery against revised CSS**

Run: `dx serve --example component_gallery --platform desktop`
Expected: window opens, Tabs and StatusBar render with the revised treatment. Cycle tabs to verify hover/focus/active states still display.

- [ ] **Step 5: Commit revised primitive CSS only**

Invoke the `conventional-commits` skill, then:

```bash
git add crates/inputforge-gui-dx/assets/components/tabs.css \
        crates/inputforge-gui-dx/assets/components/status-bar.css
git commit -m "feat(gui-dx): apply frontend-design revisions to Tabs and StatusBar CSS"
```

(`placeholder-shell.css` exists on disk but is unstaged; Task 17 will stage and commit it together with shell HTML.)

---

## Task 9: `tray/action.rs` — `TrayAction`, `TrayMenuIds`, routing (TDD)

Pure logic. The `from_id` function is the unit-testable core; `from_event` is a one-line wrapper.

**Files:**
- Create: `crates/inputforge-gui-dx/src/tray/action.rs`

- [ ] **Step 1: Create the file with the type definitions and a failing test**

Create `crates/inputforge-gui-dx/src/tray/action.rs`:

```rust
//! Tray menu routing — pure functions, no Dioxus or tao dependencies.

use muda::{MenuEvent, MenuId};

/// Internal action set produced by the tray menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrayAction {
    Show,
    Toggle,
    Quit,
}

/// The three menu ids this app builds (cloned from `tray_icon::AppTray`).
#[derive(Debug, Clone)]
pub(crate) struct TrayMenuIds {
    pub show: MenuId,
    pub toggle: MenuId,
    pub quit: MenuId,
}

impl TrayAction {
    /// Pure routing function — testable without constructing a `MenuEvent`.
    pub(crate) fn from_id(id: &MenuId, ids: &TrayMenuIds) -> Option<Self> {
        if *id == ids.show {
            return Some(Self::Show);
        }
        if *id == ids.toggle {
            return Some(Self::Toggle);
        }
        if *id == ids.quit {
            return Some(Self::Quit);
        }
        None
    }

    /// Thin adapter for the live event-loop closure.
    pub(crate) fn from_event(ev: &MenuEvent, ids: &TrayMenuIds) -> Option<Self> {
        Self::from_id(&ev.id, ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_ids() -> TrayMenuIds {
        TrayMenuIds {
            show: MenuId::new("show-gui"),
            toggle: MenuId::new("toggle-activation"),
            quit: MenuId::new("quit"),
        }
    }

    #[test]
    fn from_id_routes_show() {
        let ids = fixture_ids();
        assert_eq!(
            TrayAction::from_id(&MenuId::new("show-gui"), &ids),
            Some(TrayAction::Show),
        );
    }

    #[test]
    fn from_id_routes_toggle() {
        let ids = fixture_ids();
        assert_eq!(
            TrayAction::from_id(&MenuId::new("toggle-activation"), &ids),
            Some(TrayAction::Toggle),
        );
    }

    #[test]
    fn from_id_routes_quit() {
        let ids = fixture_ids();
        assert_eq!(
            TrayAction::from_id(&MenuId::new("quit"), &ids),
            Some(TrayAction::Quit),
        );
    }

    #[test]
    fn from_id_returns_none_for_unknown() {
        let ids = fixture_ids();
        assert_eq!(
            TrayAction::from_id(&MenuId::new("not-our-id"), &ids),
            None,
        );
    }

    #[test]
    fn from_event_delegates_to_from_id() {
        let ids = fixture_ids();
        let ev = MenuEvent {
            id: MenuId::new("toggle-activation"),
        };
        assert_eq!(TrayAction::from_event(&ev, &ids), Some(TrayAction::Toggle));
    }
}
```

- [ ] **Step 2: Add `pub(crate) mod tray;` to `lib.rs`**

The `tray::action` module needs a parent `tray` module declared in `lib.rs` to compile. Edit `crates/inputforge-gui-dx/src/lib.rs` and insert near the existing `mod app;` / `mod bridge;` / `mod context;` block:

```rust
mod tray;
```

Then create a placeholder `crates/inputforge-gui-dx/src/tray/mod.rs` declaring the action submodule:

```rust
//! Tray bridge — observes `dioxus-desktop`'s forwarded muda events via
//! `Config::with_custom_event_handler`, routes through a bounded
//! `tokio::sync::mpsc`, and dispatches in a Dioxus task.

pub(crate) mod action;
```

Task 10 fills out the rest of `tray/mod.rs`.

- [ ] **Step 3: Run tests and verify all five pass**

Run: `cargo test -p inputforge-gui-dx --lib tray::action::tests`
Expected: 5 tests pass.

If `MenuEvent { id: ... }` doesn't compile because the field is private in the muda version actually pulled in, drop that single test (`from_event_delegates_to_from_id`) and rely on the four `from_id` tests — `from_event` is a one-line wrapper trivial to verify by inspection. Adjust the file accordingly.

- [ ] **Step 4: Verify whole crate still compiles**

Run: `cargo build -p inputforge-gui-dx`
Expected: builds cleanly.

- [ ] **Step 5: Commit**

Invoke the `conventional-commits` skill, then:

```bash
git add crates/inputforge-gui-dx/src/tray
git add crates/inputforge-gui-dx/src/lib.rs
git commit -m "feat(gui-dx): add TrayAction routing with unit tests"
```

---

## Task 10: `tray/mod.rs` — event handler factory and listener task

The closure passed to `Config::with_custom_event_handler` runs on the tao event-loop thread. It must not block. The listener task runs inside the Dioxus runtime and dispatches.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/tray/mod.rs`

- [ ] **Step 1: Replace the placeholder `tray/mod.rs` with the full body**

Edit `crates/inputforge-gui-dx/src/tray/mod.rs`:

```rust
//! Tray bridge — observes `dioxus-desktop`'s forwarded muda events via
//! `Config::with_custom_event_handler`, routes through a bounded
//! `tokio::sync::mpsc`, and dispatches in a Dioxus task.

pub(crate) mod action;

use dioxus::desktop::ipc::UserWindowEvent;
use dioxus::desktop::tao::event::Event as TaoEvent;
use dioxus::desktop::tao::event_loop::EventLoopWindowTarget;
use dioxus::prelude::*;
use tokio::sync::mpsc;

use inputforge_core::engine::EngineCommand;
use inputforge_core::state::EngineStatus;

use crate::context::AppContext;
use crate::lifecycle;

use self::action::{TrayAction, TrayMenuIds};

/// Capacity for the tray-action channel. Sized for human-click cadence
/// (≪ 1 Hz peak realistic burst); 8 is a comfortable safety margin.
pub(crate) const CHANNEL_CAPACITY: usize = 8;

/// Build the closure passed to `Config::with_custom_event_handler`.
///
/// The closure runs on the tao event-loop thread; it must not block. We
/// `try_send` and log any overflow rather than wait — overflow is effectively
/// impossible at human input rates, but a dropped send must never deadlock
/// the event loop. The handler is observe-only; we never mutate `ControlFlow`.
pub(crate) fn make_event_handler(
    ids: TrayMenuIds,
    tx: mpsc::Sender<TrayAction>,
) -> impl FnMut(&TaoEvent<'_, UserWindowEvent>, &EventLoopWindowTarget<UserWindowEvent>) + 'static {
    move |event, _target| {
        if let TaoEvent::UserEvent(UserWindowEvent::MudaMenuEvent(menu_ev)) = event {
            if let Some(action) = TrayAction::from_event(menu_ev, &ids) {
                if let Err(err) = tx.try_send(action) {
                    tracing::warn!(?err, "tray channel overflow; dropping action");
                }
            }
        }
    }
}

/// Spawn the listener task. Called from `app_root`'s `use_hook` so the task
/// is tied to the Dioxus runtime lifetime and auto-cancelled on teardown.
pub(crate) fn spawn_listener_task(mut rx: mpsc::Receiver<TrayAction>, ctx: AppContext) {
    spawn(async move {
        while let Some(action) = rx.recv().await {
            match action {
                TrayAction::Show => lifecycle::show_window(),
                TrayAction::Toggle => dispatch_toggle(&ctx),
                TrayAction::Quit => lifecycle::request_quit(),
            }
        }
    });
}

/// Translate a Toggle action into the appropriate `EngineCommand` based on
/// current engine status, then send it on the engine command channel.
///
/// `AppContext.commands` is `std::sync::mpsc::Sender<EngineCommand>` (an
/// unbounded std channel from F1) — its `send` is non-blocking for unbounded
/// channels, returning `Err` only if the receiver has been dropped. We
/// discard the error: at that point the engine is already gone and the user
/// is about to learn so via the normal shutdown path.
fn dispatch_toggle(ctx: &AppContext) {
    let status = ctx.state.read().engine_status;
    let cmd = match status {
        EngineStatus::Running => EngineCommand::Deactivate,
        EngineStatus::Paused | EngineStatus::Stopped => EngineCommand::Activate,
    };
    let _ = ctx.commands.send(cmd);
}
```

**On import paths.** If `dioxus::desktop::ipc::UserWindowEvent` / `dioxus::desktop::tao::event::Event` / `dioxus::desktop::tao::event_loop::EventLoopWindowTarget` don't resolve in the pinned 0.7.6, fall back to `dioxus_desktop::*` and add `dioxus-desktop = { workspace = true }` to `crates/inputforge-gui-dx/Cargo.toml`'s `[dependencies]`. Verify with `cargo check -p inputforge-gui-dx --no-default-features --features inputforge-app/gui-dioxus` — actually simpler: just run `cargo build -p inputforge-gui-dx` and adjust if errors.

- [ ] **Step 2: Note: `lifecycle` module does not yet exist**

The module references `crate::lifecycle::{show_window, request_quit}` — this won't compile until Task 11 lands. That's intentional. To preserve task atomicity, defer the `cargo build` check to Task 11 step 4.

- [ ] **Step 3: No commit yet**

Bundled with `lifecycle` in Task 11.

---

## Task 11: `lifecycle/mod.rs` — visibility + quit + start-minimized

Three pure helpers. No close-hook gate. `lifecycle/mod.rs` has no unit tests — see spec rationale; behavior is exercised by S2 / S5 manual scenarios.

**Files:**
- Create: `crates/inputforge-gui-dx/src/lifecycle/mod.rs`
- Modify: `crates/inputforge-gui-dx/src/lib.rs`

- [ ] **Step 1: Write `lifecycle/mod.rs`**

Create `crates/inputforge-gui-dx/src/lifecycle/mod.rs`:

```rust
//! Window-lifecycle helpers. Three functions, all called from inside a
//! Dioxus scope (so `dioxus::desktop::window()` resolves correctly).
//!
//! No close-hook gate: Dioxus owns close-requested handling. X-click hide
//! is set up at launch via `WindowCloseBehaviour::WindowHides` in
//! `lib.rs::launch_gui`; tray Quit flips the per-window close behavior to
//! `WindowCloses` then triggers close — the event loop exits because
//! `exit_on_last_window_close` is true (the default; F3 does not override).

use dioxus::desktop::{WindowCloseBehaviour, window};

/// Tray Show — bring the window back to foreground.
pub(crate) fn show_window() {
    let w = window();
    w.set_visible(true);
    w.set_focus();
}

/// Tray Quit — switch this window's close behavior to `WindowCloses`,
/// then trigger close. Dioxus destroys the window, observes zero remaining
/// webviews, and the event loop exits because `exit_on_last_window_close`
/// is true (the default — F3 does not override). `launch_gui` returns;
/// `main.rs::shutdown()` then runs.
///
/// `quit_requested` in `AppState` is **not** read on the Dioxus path
/// (egui still uses it). The close-behavior switch is the entire Quit
/// pathway — there is no flag to gate, no close-hook to wire.
pub(crate) fn request_quit() {
    let w = window();
    w.set_close_behavior(WindowCloseBehaviour::WindowCloses);
    w.close();
}

/// Apply --start-minimized once during `app_root` mount.
pub(crate) fn apply_start_minimized(start_minimized: bool) {
    if start_minimized {
        window().set_visible(false);
    }
}
```

If the `dioxus::desktop::WindowCloseBehaviour` / `dioxus::desktop::window` paths don't resolve (e.g., 0.7.x exposes them at a different path), fall back to `dioxus_desktop::WindowCloseBehaviour` / `dioxus_desktop::window` and add the `dioxus-desktop` direct dep — same fallback as Task 10.

- [ ] **Step 2: Add `mod lifecycle;` to `lib.rs`**

Edit `crates/inputforge-gui-dx/src/lib.rs`. Add alongside `mod tray;`:

```rust
mod lifecycle;
```

- [ ] **Step 3: Verify the crate compiles**

Run: `cargo build -p inputforge-gui-dx`
Expected: builds cleanly. Both `tray/mod.rs` (Task 10) and `lifecycle/mod.rs` (this task) must resolve.

If a `dioxus::desktop::set_close_behavior` doesn't exist as a method on the window handle, check `DesktopService::set_close_behavior` — it may need `let w = window(); w.set_close_behavior(...)` where `w` is a `Rc<DesktopService>`. The 0.7.6 API: `dioxus_desktop::window() -> Rc<DesktopService>` (it derefs to provide the methods). Verify the method name on the actual API surface — if it's `set_close_behaviour` (UK spelling) or `set_close_behavior_for_all_windows` or similar, adjust both occurrences in `request_quit` accordingly.

- [ ] **Step 4: Verify tests still pass**

Run: `cargo test -p inputforge-gui-dx`
Expected: F1 + F2 tests + the 5 new TrayAction tests (Task 9) all pass.

- [ ] **Step 5: Commit**

Invoke the `conventional-commits` skill, then:

```bash
git add crates/inputforge-gui-dx/src/tray/mod.rs \
        crates/inputforge-gui-dx/src/lifecycle \
        crates/inputforge-gui-dx/src/lib.rs
git commit -m "feat(gui-dx): add tray event handler factory, listener task, lifecycle helpers"
```

---

## Task 12: Mirror `start_minimized` on `inputforge-gui::launch_gui` (egui-side parity)

Egui side ignores the parameter. F1 already established the cfg-gated `use ... launch_gui` selection, so `main.rs`'s call site stays identical under both feature flags. The parameter deletes at F16.

**Files:**
- Modify: `crates/inputforge-gui/src/lib.rs`

- [ ] **Step 1: Add the parameter**

Edit `crates/inputforge-gui/src/lib.rs::launch_gui`. Insert `start_minimized: bool` as the last parameter:

```rust
pub fn launch_gui(
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    tray_menu_ids: (MenuId, MenuId, MenuId),
    settings: AppSettings,
    start_minimized: bool,
) -> anyhow::Result<()> {
    let _ = start_minimized; // egui already gates startup launch from main.rs

    let options = eframe::NativeOptions {
        // ... existing body unchanged
```

- [ ] **Step 2: Update the doc-comment**

Append to the existing `launch_gui` doc-comment a single line under `# Errors`:

```rust
/// `start_minimized` is accepted for signature parity with
/// `inputforge_gui_dx::launch_gui` and is ignored here — `main.rs` already
/// gates the egui startup launch on `cli.start_minimized`. The parameter is
/// removed at F16 cleanup when the egui crate is deleted.
```

- [ ] **Step 3: Handle any fresh `clippy::needless_pass_by_value`**

If `cargo clippy -p inputforge-gui` flags `start_minimized: bool` as needlessly-pass-by-value (unlikely for a `bool` since it's `Copy`, but check), extend the existing `#[expect(clippy::needless_pass_by_value, reason = "signature parity with inputforge_gui_dx::launch_gui ...")]` annotation if one is on the function. Do NOT add a separate `#[allow]`.

- [ ] **Step 4: Verify default-features build still works**

Run: `cargo build` (default = `gui-egui`)
Expected: build fails because `main.rs` doesn't yet pass the new arg. **This is expected.** It will be fixed in Task 20. To confirm the egui crate itself compiles in isolation:

Run: `cargo build -p inputforge-gui`
Expected: builds cleanly.

- [ ] **Step 5: No commit yet**

Bundle with the Dioxus-side signature change in Task 13.

---

## Task 13: Extend `inputforge-gui-dx::launch_gui` — channel + Config builder + LaunchParams

The construction sequence: build `TrayMenuIds`, create the bounded channel, install the custom-event handler in `Config`, install `LaunchParams` via `with_context` alongside `RawHandles`. `WindowCloseBehaviour::WindowHides` set at launch wires X-click hide. Existing 1280×800 window builder preserved.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/lib.rs`

- [ ] **Step 1: Define `LaunchParams`**

Add `LaunchParams` to `crates/inputforge-gui-dx/src/lib.rs` at the top of the crate (right after the `mod ...;` block, before the `use` block, or in a small dedicated `pub(crate) struct LaunchParams { ... }` definition near `RawHandles`-style types). Place it directly in `lib.rs` rather than `context.rs` because it only exists during launch wiring:

```rust
use crate::tray::action::TrayAction;

/// Per-launch parameters carried from `launch_gui` into `app_root` via
/// `LaunchBuilder::with_context`.
///
/// `listener_rx` is wrapped in `Arc<Mutex<Option<...>>>` because:
///
/// - `with_context` requires `Send + Sync + 'static` (`dioxus-0.7.6/src/launch.rs:256`),
///   and `tokio::sync::mpsc::Receiver` is `Send` but `!Sync`.
/// - Take-once-on-mount: `lock().unwrap().take()` empties the slot so any
///   subsequent re-mount of `app_root` (e.g. `dx serve` hot-reload) becomes
///   a no-op rather than double-spawning the listener. Production never
///   re-mounts; this is belt-and-braces for the dev loop.
/// - No contention possible — single producer (this struct's constructor in
///   `launch_gui`), single consumer (`app_root`'s `use_hook`). The `unwrap()`
///   on `lock()` cannot panic from poisoning: no panic path holds the lock
///   across `.take()`.
#[derive(Clone)]
pub(crate) struct LaunchParams {
    pub start_minimized: bool,
    pub listener_rx:
        std::sync::Arc<std::sync::Mutex<Option<tokio::sync::mpsc::Receiver<TrayAction>>>>,
}
```

- [ ] **Step 2: Extend `launch_gui` signature and body**

Replace the existing `launch_gui` body:

```rust
#[expect(
    clippy::needless_pass_by_value,
    reason = "signature parity with inputforge_gui::launch_gui — main.rs dispatches \
              both crates via a cfg-gated `use` line; changing to `&` here would \
              break the call site when the egui crate is swapped in"
)]
pub fn launch_gui(
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    tray_menu_ids: (MenuId, MenuId, MenuId),
    settings: AppSettings,
    start_minimized: bool,
) -> anyhow::Result<()> {
    let (show, toggle, quit) = tray_menu_ids;
    let menu_ids = crate::tray::action::TrayMenuIds { show, toggle, quit };

    let (tx, rx) = tokio::sync::mpsc::channel(crate::tray::CHANNEL_CAPACITY);

    let handles = RawHandles {
        state,
        commands,
        settings: Arc::new(settings),
    };
    let params = LaunchParams {
        start_minimized,
        listener_rx: std::sync::Arc::new(std::sync::Mutex::new(Some(rx))),
    };

    let window = WindowBuilder::new()
        .with_title("InputForge")
        .with_inner_size(LogicalSize::new(1280.0, 800.0))
        .with_min_inner_size(LogicalSize::new(800.0, 500.0));

    let cfg = Config::new()
        .with_window(window)
        .with_close_behaviour(dioxus::desktop::WindowCloseBehaviour::WindowHides)
        .with_custom_event_handler(crate::tray::make_event_handler(menu_ids, tx));
    // exit_on_last_window_close left at its default (true).

    LaunchBuilder::desktop()
        .with_cfg(cfg)
        .with_context(handles)
        .with_context(params)
        .launch(app::app_root);

    Ok(())
}
```

Notes:

- The `with_close_behaviour` builder method is the API in `dioxus_desktop` 0.7 (UK spelling, matching the spec). If the actual method name is `with_close_behavior` (US), adjust here and in `lifecycle/mod.rs::request_quit`. The `WindowCloseBehaviour` enum name itself is UK in 0.7.
- `with_custom_event_handler` takes `impl FnMut(&Event<UserWindowEvent>, &EventLoopWindowTarget<UserWindowEvent>) + 'static`. The closure returned by `make_event_handler` matches.
- `with_context(params)` requires `LaunchParams: Any + Clone + Send + Sync + 'static` — `Arc<Mutex<Option<Receiver>>>` satisfies this.
- The pre-F3 `tracing::debug!(?tray_menu_ids, "tray wiring stubbed until F3");` line is deleted (no longer stubbed).

- [ ] **Step 3: Update `tray_menu_ids` doc-comment**

Replace the existing F1 doc-fragment that says "is accepted for signature parity ... but is stubbed at F1; F3 wires the listener task that consumes it" with the F3 reality:

```rust
/// `tray_menu_ids` are forwarded into the custom event-handler closure
/// installed via `Config::with_custom_event_handler`. The handler observes
/// `UserWindowEvent::MudaMenuEvent` (which `dioxus-desktop` already forwards
/// from its own muda handler), routes by ID, and pushes onto a bounded
/// `tokio::sync::mpsc` consumed by a Dioxus task spawned from `app_root`.
```

- [ ] **Step 4: Verify the gui-dx crate builds**

Run: `cargo build -p inputforge-gui-dx`
Expected: build fails because `app::app_root` doesn't yet read `LaunchParams` — but the crate compiles up to the `app_root` call site if `app_root` is unchanged from F2. Specifically, the F2 `app_root` doesn't `use_context::<LaunchParams>()`, so it should still build. If the build fails here, diagnose import errors in `lib.rs` first (Task 13's changes) before moving on.

If the failure is from `dioxus::desktop::WindowCloseBehaviour` not resolving, switch to `dioxus_desktop::WindowCloseBehaviour` and add the direct dep.

- [ ] **Step 5: bridge_demo will fail to compile**

`bridge_demo.rs` calls `launch_gui(state, commands, menu_ids, AppSettings::default())` — only 4 args. It's now wrong. Don't fix it yet — Task 19 owns the bridge_demo update. The build of `inputforge-app` will likewise fail because `main.rs` doesn't pass `start_minimized`. Both are intentional and fixed in later tasks (19, 20).

- [ ] **Step 6: Commit**

Invoke the `conventional-commits` skill, then:

```bash
git add crates/inputforge-gui-dx/src/lib.rs \
        crates/inputforge-gui/src/lib.rs
git commit -m "feat(gui-dx): wire tray bridge into launch_gui; add start_minimized parity"
```

---

## Task 14: Shell scaffold — `mod.rs` and `placeholder.rs`

Disposable layout. The `pub(crate)` re-exports keep the surface narrow — F5 deletes this whole module.

**Files:**
- Create: `crates/inputforge-gui-dx/src/shell/mod.rs`
- Create: `crates/inputforge-gui-dx/src/shell/placeholder.rs`

- [ ] **Step 1: Write `shell/mod.rs`**

Create `crates/inputforge-gui-dx/src/shell/mod.rs`:

```rust
//! Placeholder shell — disposable at F5.
//!
//! This module exists to give F3 a coherent four-region grid that the
//! tray-bridge lifecycle can be observed against (open the window, watch
//! the status bar reflect engine state, click tray Toggle, watch the badge
//! flip). F5 will redesign IA and may replace the entire grid template
//! (not just slot contents). Treat every line of CSS in
//! `assets/shell/placeholder-shell.css` and every grid-area definition in
//! `placeholder.rs` as scratch.

mod placeholder;
mod status_bar_view;

pub(crate) use placeholder::PlaceholderShell;
```

- [ ] **Step 2: Write `shell/placeholder.rs`**

Create `crates/inputforge-gui-dx/src/shell/placeholder.rs`:

```rust
use dioxus::prelude::*;

use crate::components::Tabs;
use crate::shell::status_bar_view::StatusBarView;

const PLACEHOLDER_SHELL_CSS: Asset = asset!("/assets/shell/placeholder-shell.css");

#[component]
pub(crate) fn PlaceholderShell() -> Element {
    let mut center_tab = use_signal(|| "mappings".to_owned());

    rsx! {
        document::Stylesheet { href: PLACEHOLDER_SHELL_CSS }
        div { class: "if-placeholder-shell",
            div { class: "if-placeholder-shell__top",
                "Top toolbar (F5 owns contents)"
            }
            div { class: "if-placeholder-shell__left",
                "Left panel — devices (F6)"
            }
            div { class: "if-placeholder-shell__center",
                Tabs {
                    items: vec![
                        ("mappings".into(), "Mappings".into()),
                        ("modes".into(),    "Modes".into()),
                    ],
                    value: center_tab.read().clone(),
                    onchange: move |id: String| center_tab.set(id),
                }
                div { class: "if-placeholder-shell__center-body",
                    "Center placeholder — F7+ owns content"
                }
            }
            StatusBarView {}
        }
    }
}
```

`document::Stylesheet` is mounted from inside the component because `placeholder-shell.css` is shell-scoped (not a design-system token), so it lives outside `assets/components/` and is NOT mounted from `theme/mod.rs`. The component carries its own asset reference — when F5 deletes the file, the asset reference goes too.

- [ ] **Step 3: Note: `status_bar_view` does not yet exist**

The module references `crate::shell::status_bar_view::StatusBarView` — this won't compile until Task 15 lands. Defer the build check to Task 16.

- [ ] **Step 4: No commit yet**

Bundle with status_bar_view in Task 15.

---

## Task 15: Shell consumer of StatusBar — `status_bar_view.rs`

The only F3 surface that subscribes to AppContext signals. Reads `meta` and `config`, composes a real status bar.

**Files:**
- Create: `crates/inputforge-gui-dx/src/shell/status_bar_view.rs`

- [ ] **Step 1: Write the file**

Create `crates/inputforge-gui-dx/src/shell/status_bar_view.rs`:

```rust
use dioxus::prelude::*;

use inputforge_core::state::EngineStatus;

use crate::components::{Badge, BadgeVariant, Separator, SeparatorOrientation, StatusBar};
use crate::context::AppContext;

/// Signal-bound consumer of the StatusBar primitive.
///
/// Subscribes to `meta.engine_status` / `meta.current_mode` /
/// `meta.profile_name` / `config.devices` and composes the four readouts:
/// engine-status badge (with `role="status" aria-live="polite"` on its
/// wrapper for AT announcements), mode badge (always rendered — Neutral
/// when the value is "Default"), `connected/total devices` text, and
/// profile-name span (plain `<span>`, not clickable in F3 — F14 owns
/// profile-manager wiring).
#[component]
pub(crate) fn StatusBarView() -> Element {
    let ctx = use_context::<AppContext>();

    let status = use_memo(move || ctx.meta.read().engine_status);
    let mode = use_memo(move || ctx.meta.read().current_mode.clone());
    let profile = use_memo(move || ctx.meta.read().profile_name.clone());
    let dev_count = use_memo(move || {
        let cfg = ctx.config.read();
        let connected = cfg.devices.iter().filter(|d| d.connected).count();
        (connected, cfg.devices.len())
    });

    // Capture Memo values as locals before rsx! — Memo<T> does not implement
    // Display directly, and rsx! does not accept top-level `let` bindings
    // between elements inside a slot. Lifting both is the idiomatic 0.7 form.
    let status_value = *status.read();
    let mode_str = mode.read().clone();
    let profile_str = profile.read().clone();
    let (connected, total) = *dev_count.read();

    rsx! {
        StatusBar {
            class: "if-placeholder-shell__status".to_owned(),
            start: rsx! {
                span { role: "status", "aria-live": "polite",
                    Badge {
                        variant: status_to_variant(status_value),
                        "{status_label(status_value)}"
                    }
                }
                Separator { orientation: SeparatorOrientation::Vertical }
                Badge { variant: BadgeVariant::Neutral, "{mode_str}" }
            },
            middle: rsx! {
                span { "{connected}/{total} devices" }
            },
            end: rsx! {
                if let Some(name) = profile_str.as_ref() {
                    span { "{name}" }
                }
            },
        }
    }
}

fn status_to_variant(s: EngineStatus) -> BadgeVariant {
    match s {
        EngineStatus::Running => BadgeVariant::Success,
        EngineStatus::Paused => BadgeVariant::Warning,
        EngineStatus::Stopped => BadgeVariant::Neutral,
    }
}

fn status_label(s: EngineStatus) -> &'static str {
    match s {
        EngineStatus::Running => "Running",
        EngineStatus::Paused => "Paused",
        EngineStatus::Stopped => "Stopped",
    }
}
```

- [ ] **Step 2: No commit yet**

Bundle with the placeholder-shell.css finalization in Task 16, since none of these compile until lib.rs declares the `shell` module.

---

## Task 16: `placeholder-shell.css` finalization + `lib.rs` module declarations + AppContext attribute cleanup

Brings the shell online. After this task, `cargo build -p inputforge-gui-dx` succeeds again (it has been broken since Task 13 because `app_root` still uses F2 shape; Task 18 fixes that).

Wait — `app_root` still works in F2 shape because `LaunchParams` is consumed in Task 18, not here. The crate compiles after this task with shell wired in but app_root unchanged. (Task 18 will move app_root to use `LaunchParams`.) Verify by running the build after Step 4.

**Files:**
- Modify: `crates/inputforge-gui-dx/assets/shell/placeholder-shell.css` (already drafted in Task 8 — this task ensures it's correct)
- Modify: `crates/inputforge-gui-dx/src/lib.rs`
- Modify: `crates/inputforge-gui-dx/src/context.rs`

- [ ] **Step 1: Verify `placeholder-shell.css` exists at the right path**

Run: `ls crates/inputforge-gui-dx/assets/shell/placeholder-shell.css`
Expected: file exists (drafted by frontend-design in Task 8 step 3).

If it doesn't exist (frontend-design output was scratched and forgotten), write a baseline matching the spec's CSS sketch:

```css
/* Placeholder shell — disposable at F5. */

.if-placeholder-shell {
    display: grid;
    grid-template-areas:
        "top    top"
        "left   center"
        "status status";
    grid-template-columns: 240px 1fr;
    grid-template-rows: 40px 1fr 28px;
    height: 100vh;
}

.if-placeholder-shell__top {
    grid-area: top;
    display: flex;
    align-items: center;
    padding: 0 var(--space-3);
    border-bottom: 1px solid var(--color-border);
}

.if-placeholder-shell__left {
    grid-area: left;
    border-right: 1px solid var(--color-border);
    padding: var(--space-3);
    overflow: auto;
}

.if-placeholder-shell__center {
    grid-area: center;
    display: flex;
    flex-direction: column;
    min-width: 0;
}

.if-placeholder-shell__center-body {
    flex: 1 1 auto;
    padding: var(--space-3);
    overflow: auto;
}

.if-placeholder-shell__status {
    grid-area: status;
}
```

- [ ] **Step 2: Confirm token references resolve**

Run:

```bash
grep -E 'var\(--' crates/inputforge-gui-dx/assets/shell/placeholder-shell.css
```

Every `var(--...)` reference must match a declared token in `assets/tokens/*.css`. If frontend-design used a token name that doesn't exist (e.g. `--color-shell-divider`), either (a) substitute the closest declared token (e.g., `--color-border`), or (b) revisit Task 8 to push back — F3 must NOT coin new tokens.

- [ ] **Step 3: Add `mod shell;` to `lib.rs`**

Edit `crates/inputforge-gui-dx/src/lib.rs`. Add alongside `mod tray;` / `mod lifecycle;`:

```rust
mod shell;
```

- [ ] **Step 4: Remove the now-stale `#[expect(dead_code)]` attributes from `AppContext`**

Edit `crates/inputforge-gui-dx/src/context.rs`. Find:

```rust
    #[expect(dead_code, reason = "used in later tasks (engine command dispatch)")]
    pub commands: mpsc::Sender<EngineCommand>,
    #[expect(dead_code, reason = "used in later tasks (settings reads)")]
    pub settings: Arc<AppSettings>,
```

Remove the `commands` `#[expect]` attribute — Task 10's `dispatch_toggle` reads `ctx.commands`, so the dead-code claim is now false. **Keep** the `settings` `#[expect]` attribute — F3 doesn't read `settings` at runtime; F14 owns settings reads. If clippy flags `commands` as dead-code after the removal (i.e., `dispatch_toggle` is somehow not actually reachable from F3 wiring), DO NOT re-add the `#[expect]` — instead, debug why the wiring is broken.

- [ ] **Step 5: Verify the crate compiles**

Run: `cargo build -p inputforge-gui-dx`
Expected: builds cleanly. The `shell` module is declared, all submodules resolve, AppContext attributes are correct.

If `cargo build` says "function never used: dispatch_toggle" — this means Task 10's `tray::mod::spawn_listener_task` isn't being called from anywhere yet (Task 18 wires it). Add `#[allow(dead_code)]` temporarily on `dispatch_toggle` and remove it in Task 18 after wiring lands. Better: skip this concern; the dead-code is `pub(crate)` on a `tray` module function and lints permit pub(crate) dead-code by default in many configs. If the workspace lint config flags it, use `#[expect(dead_code, reason = "wired by app_root in Task 18")]` and remove in Task 18.

- [ ] **Step 6: Run tests**

Run: `cargo test -p inputforge-gui-dx`
Expected: all tests pass.

- [ ] **Step 7: Commit**

Invoke the `conventional-commits` skill, then:

```bash
git add crates/inputforge-gui-dx/src/shell \
        crates/inputforge-gui-dx/assets/shell \
        crates/inputforge-gui-dx/src/lib.rs \
        crates/inputforge-gui-dx/src/context.rs
git commit -m "feat(gui-dx): add disposable placeholder shell with signal-bound StatusBarView"
```

---

## Task 17: Smoke-test the shell renders inside the gallery

A sanity check before wiring up `app_root` and main.rs. Render `PlaceholderShell` from a temporary harness to confirm CSS resolves and the layout doesn't visibly explode.

This task is OPTIONAL but cheap. If the implementer is confident in the F2 dev loop, skip to Task 18 and rely on Task 19's `dx serve --example bridge_demo` smoke test.

**Files:**
- Read-only: `crates/inputforge-gui-dx/examples/component_gallery.rs` (do NOT modify; this task is a manual run only)

- [ ] **Step 1: Run the existing gallery**

Run: `dx serve --example component_gallery --platform desktop`
Expected: window opens, scroll to Tabs and StatusBar sections, both render with the revised CSS from Task 8.

- [ ] **Step 2: No code change, no commit**

The shell itself isn't yet rendered (it requires `AppContext` which the gallery doesn't provide). The full shell smoke happens at Task 19 via `bridge_demo`.

---

## Task 18: Rewrite `app_root` for `LaunchParams` consumption

The biggest single-file rewrite of F3. The F2 `app_root` lives at `src/app.rs:9-34`. Replace it with the F3 shape.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/app.rs`

- [ ] **Step 1: Replace `app_root` and delete `F1Readout`**

Edit `crates/inputforge-gui-dx/src/app.rs`. The full new contents are:

```rust
use dioxus::prelude::*;

use crate::bridge::spawn_polling_task;
use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, RawHandles};
use crate::lifecycle;
use crate::shell::PlaceholderShell;
use crate::theme::ThemeProvider;
use crate::tray;
use crate::LaunchParams;

/// Root Dioxus component — assembles `AppContext`, installs it for descendants,
/// spawns polling + tray-listener tasks, applies `--start-minimized`, and
/// renders the placeholder shell.
pub(crate) fn app_root() -> Element {
    let raw = use_context::<RawHandles>();
    let params = use_context::<LaunchParams>();

    let meta = use_signal(MetaSnapshot::default);
    let config = use_signal(ConfigSnapshot::default);
    let live = use_signal(LiveSnapshot::default);

    let ctx = AppContext {
        state: std::sync::Arc::clone(&raw.state),
        commands: raw.commands.clone(),
        settings: std::sync::Arc::clone(&raw.settings),
        meta,
        config,
        live,
    };
    use_context_provider(|| ctx.clone());

    use_hook(|| spawn_polling_task(ctx.clone()));

    // Tray listener — take the receiver once, spawn the task. Subsequent
    // re-mounts (e.g. dx serve hot-reload of app_root) will see `None` and
    // become no-ops, preventing double-spawn.
    use_hook(|| {
        if let Some(rx) = params
            .listener_rx
            .lock()
            .expect("listener_rx mutex poisoned — no panic path holds the lock")
            .take()
        {
            tray::spawn_listener_task(rx, ctx.clone());
        }
    });

    // --start-minimized — applied once on first mount.
    use_hook(|| lifecycle::apply_start_minimized(params.start_minimized));

    rsx! {
        ThemeProvider { PlaceholderShell {} }
    }
}
```

The F1Readout component (lines 36–95 of the old file) is deleted entirely. Its data-binding contract test in `context.rs` stays — `f1_readout_data_binding_contract` still asserts the snapshot shape, just no longer pinned to a specific consumer.

- [ ] **Step 2: Make `LaunchParams` accessible from `app.rs`**

Task 13 placed `LaunchParams` in `lib.rs` as `pub(crate)`. The `crate::LaunchParams` import path resolves it. If the visibility was wrong (e.g., the implementer wrote `struct LaunchParams` without `pub(crate)`), upgrade it to `pub(crate)`.

- [ ] **Step 3: Verify build**

Run: `cargo build -p inputforge-gui-dx`
Expected: builds cleanly. If there are dead-code lints on `dispatch_toggle` (carried over from Task 16), they should now resolve because `spawn_listener_task` is reachable from `app_root` via the second `use_hook`.

If `bridge_demo` build fails because of the `launch_gui` signature change (still 4 args), that's expected — Task 19 fixes it.

- [ ] **Step 4: Commit**

Invoke the `conventional-commits` skill, then:

```bash
git add crates/inputforge-gui-dx/src/app.rs
git commit -m "feat(gui-dx): rewrite app_root for LaunchParams + listener task + placeholder shell"
```

---

## Task 19: Update `bridge_demo.rs` for the new `launch_gui` signature

`bridge_demo` already calls `launch_gui` directly, so it inherits the new shell automatically — no `PlaceholderShell` wrap is needed at this level. The only change is to pass `start_minimized: false`.

**Files:**
- Modify: `crates/inputforge-gui-dx/examples/bridge_demo.rs`

- [ ] **Step 1: Update the final call site**

Edit the last line of `examples/bridge_demo.rs`. Replace:

```rust
    inputforge_gui_dx::launch_gui(state, commands, menu_ids, AppSettings::default())
```

with:

```rust
    inputforge_gui_dx::launch_gui(state, commands, menu_ids, AppSettings::default(), false)
```

The seeded data (engine status Running, "Demo" mode, 1 device, 1 virtual device) drives the StatusBarView's badges — the smoke test proves the shell is wired.

- [ ] **Step 2: Smoke test via dx serve**

Run: `dx serve --example bridge_demo --platform desktop`
Expected: window opens. The placeholder shell appears with:
- "Top toolbar (F5 owns contents)" in the top region.
- "Left panel — devices (F6)" in the left region.
- Mappings/Modes Tabs in the center, with "Center placeholder — F7+ owns content" below.
- Status bar at the bottom showing: green "Running" badge + vertical separator + neutral "Demo" badge in the start slot; "1/1 devices" middle; (no profile name in end — `bridge_demo`'s seeded state has no profile loaded).

Click between Mappings and Modes tabs — center-tab signal updates, no console errors. Edit the file (e.g., change "Mappings" label) — RSX hot-reload applies within ~1s. Close the window with Ctrl+C in the terminal (or by closing the window — `bridge_demo` has no tray, so X-click WILL hide the window with no way to recover; expect that and Ctrl+C the dev server).

If RSX hot-reload doesn't apply, that's an F1 issue, not F3 — refer to F1 plan README for diagnosis.

- [ ] **Step 3: Commit**

Invoke the `conventional-commits` skill, then:

```bash
git add crates/inputforge-gui-dx/examples/bridge_demo.rs
git commit -m "feat(gui-dx): adapt bridge_demo to new launch_gui signature"
```

---

## Task 20: `main.rs` Shape A divergence + F1 cleanup

The largest behavioral change for the egui-default flow happens here, but the egui side stays byte-identical to today — only the Dioxus side's startup is rewritten. F1's `IS_GUI_DIOXUS` sentinel + the two cfg-guards delete entirely. `launch_gui_blocking`, `run_tray_loop`, `drain_stale_gui_events` become egui-only.

**Files:**
- Modify: `crates/inputforge-app/src/main.rs`

- [ ] **Step 1: Delete the `IS_GUI_DIOXUS` sentinel const**

Find these lines in `main.rs` (currently 44–47):

```rust
#[cfg(feature = "gui-egui")]
const IS_GUI_DIOXUS: bool = false;
#[cfg(feature = "gui-dioxus")]
const IS_GUI_DIOXUS: bool = true;
```

Delete all four lines.

- [ ] **Step 2: Restructure the GUI-launch block in `main()`**

Find the block (currently lines 128–154) starting with `// Launch the GUI immediately unless --start-minimized.` through the closing of the `if !cli.start_minimized { ... }` block (the line `quit_requested = true;` followed by the brace closing the inner `if IS_GUI_DIOXUS`).

Replace from `let mut quit_requested = false;` through to the `if !quit_requested { run_tray_loop(...); }` block (currently lines 129–159) with:

```rust
    // GUI launch — Shape A: cfg-split because the Dioxus and egui lifecycles
    // diverge. The egui flow is byte-identical to today.

    #[cfg(feature = "gui-dioxus")]
    {
        if let Err(e) = launch_gui(
            Arc::clone(&state),
            cmd_tx.clone(),
            tray.menu_item_ids(),
            settings.clone(),
            cli.start_minimized,
        ) {
            tracing::error!(%e, "GUI exited with error");
        }
        // launch_gui only returns on real Quit (tray Quit click). Fall
        // through to shutdown — no run_tray_loop, no drain_stale_gui_events,
        // no quit_requested flag. The window-hides-on-X behavior is owned
        // by Dioxus via WindowCloseBehaviour::WindowHides set in launch_gui.
    }

    #[cfg(feature = "gui-egui")]
    {
        let mut quit_requested = false;
        if !cli.start_minimized {
            for action in launch_gui_blocking(&tray, &state, &cmd_tx, &settings) {
                match action {
                    TrayAction::Quit => quit_requested = true,
                    TrayAction::ToggleActivation => {
                        let status = state.read().engine_status;
                        let cmd = match status {
                            EngineStatus::Running => EngineCommand::Deactivate,
                            EngineStatus::Paused | EngineStatus::Stopped => {
                                EngineCommand::Activate
                            }
                        };
                        let _ = cmd_tx.send(cmd);
                    }
                    TrayAction::ShowGui => {} // already drained, satisfy exhaustiveness
                }
            }
        }
        if !quit_requested {
            run_tray_loop(&tray, &state, &cmd_tx, &settings);
        }
    }

    // Graceful shutdown — runs on both feature flags.
    shutdown(cmd_tx, engine_handle);

    Ok(())
}
```

The `shutdown(...)` and final `Ok(())` lines remain at the end of `main()`; only the GUI-launch block is replaced.

- [ ] **Step 3: Pass `start_minimized` through `launch_gui_blocking` to the egui `launch_gui`**

Edit `launch_gui_blocking` in `main.rs` (currently lines 231–259). Add the new parameter to the signature and pass it through:

```rust
fn launch_gui_blocking(
    tray: &AppTray,
    state: &Arc<RwLock<AppState>>,
    cmd_tx: &mpsc::Sender<EngineCommand>,
    settings: &AppSettings,
) -> Vec<TrayAction> {
    let gui_state = Arc::clone(state);
    let gui_tx = cmd_tx.clone();
    let menu_ids = tray.menu_item_ids();

    if let Err(e) = launch_gui(gui_state, gui_tx, menu_ids, settings.clone(), false) {
        // start_minimized: false — main.rs gates the egui startup launch
        // from cli.start_minimized itself; once we're in launch_gui_blocking,
        // we always want the window visible. Parameter exists only for
        // signature parity with the Dioxus crate (deletes at F16).
        tracing::error!(%e, "GUI exited with error");
    }
    // ... rest unchanged
```

The function signature itself is unchanged (still takes `tray`, `state`, `cmd_tx`, `settings`). Only the inner `launch_gui(...)` call changes.

- [ ] **Step 4: cfg-gate `launch_gui_blocking`, `run_tray_loop`, `drain_stale_gui_events`**

Add `#[cfg(feature = "gui-egui")]` directly above each function definition:

```rust
#[cfg(feature = "gui-egui")]
fn launch_gui_blocking(...) -> Vec<TrayAction> { ... }

#[cfg(feature = "gui-egui")]
fn drain_stale_gui_events(tray: &AppTray) -> Vec<TrayAction> { ... }

#[cfg(feature = "gui-egui")]
#[expect(unsafe_code, reason = "...")]
fn run_tray_loop(...) { ... }
```

The `#[expect(unsafe_code, reason = "...")]` attribute on `run_tray_loop` stays — it's a separate annotation from the new `#[cfg]`.

- [ ] **Step 5: Delete the `IS_GUI_DIOXUS` reference inside `run_tray_loop`**

Find inside `run_tray_loop` (currently lines 344–349):

```rust
                    if IS_GUI_DIOXUS {
                        // F1: tao EventLoop::run is one-shot. Return from the tray loop so
                        // main()'s shutdown() runs. F3 restores tray re-open via
                        // DesktopService::set_visible(true) signaled from the listener.
                        return;
                    }
```

Delete those six lines. `run_tray_loop` is now `gui-egui`-only (Task 20 step 4) and the `IS_GUI_DIOXUS` symbol no longer exists, so the guard is structurally impossible.

- [ ] **Step 6: Verify both feature flags build**

Default features (gui-egui):
```bash
cargo build -p inputforge-app
```
Expected: builds cleanly. Behavior unchanged from before F3.

Dioxus:
```bash
cargo build -p inputforge-app --no-default-features --features gui-dioxus
```
Expected: builds cleanly.

Compile-error guards (no GUI / both GUIs) should still fire:
```bash
cargo build -p inputforge-app --no-default-features
# Expected: compile error "one of `gui-egui` or `gui-dioxus` must be enabled"

cargo build -p inputforge-app --features gui-dioxus
# Expected: compile error "features `gui-egui` and `gui-dioxus` are mutually exclusive"
```

- [ ] **Step 7: Run all tests under both feature flags**

```bash
cargo test
cargo test --no-default-features --features gui-dioxus
```

Expected: all tests pass under both. The Dioxus path runs F1 + F2 + F3 unit tests including the 5 new `tray::action::tests`.

- [ ] **Step 8: Commit**

Invoke the `conventional-commits` skill, then:

```bash
git add crates/inputforge-app/src/main.rs
git commit -m "feat(app): cfg-split GUI launch for F3 Dioxus tray bridge; remove F1 lifecycle workarounds"
```

---

## Task 21: Update `inputforge-gui-dx/README.md`

Document the tray bridge model, hide-to-tray lifecycle, and new primitives.

**Files:**
- Modify: `crates/inputforge-gui-dx/README.md`

- [ ] **Step 1: Append F3 sections**

Open the existing `README.md` and append (at the end of the file, after F1/F2 sections):

```markdown
## F3 — Tray bridge & hide-to-tray lifecycle

### Tray bridge

Under `--features gui-dioxus`, tray menu events are observed via
`Config::with_custom_event_handler`, NOT via `muda::MenuEvent::receiver()`.
This is because Dioxus 0.7.6 itself unconditionally registers
`muda::MenuEvent::set_event_handler` and forwards every event as
`UserWindowEvent::MudaMenuEvent` (see `dioxus-desktop-0.7.6/src/app.rs:449`).
F3 installs a closure that pattern-matches the user-event variant, routes
the menu id to a `TrayAction`, and forwards through a bounded
`tokio::sync::mpsc` channel. A Dioxus task spawned from `app_root` drains
the channel and dispatches: `Show` → `set_visible(true)`, `Toggle` →
`EngineCommand::Activate`/`Deactivate`, `Quit` → flip
`WindowCloseBehaviour` to `WindowCloses` and call `window().close()`.

### Hide-to-tray window lifecycle

`Config::with_close_behaviour(WindowCloseBehaviour::WindowHides)` makes
X-click hide the window natively (Dioxus calls `set_visible(false)` and
consumes the close-requested event; F3 has no close-handler code path).
Tray Show re-opens via `set_visible(true)` + `set_focus()`. Tray Quit flips
this window's close behavior to `WindowCloses` then calls `close()`; with
the default `exit_on_last_window_close = true` the event loop exits and
`launch_gui` returns. `main.rs::shutdown()` then runs, the engine thread
joins, `HidHide` unhide and `vJoy` release fire via `Drop`.

`--start-minimized` is plumbed via the `start_minimized: bool` parameter
on `launch_gui`. The Dioxus side calls `set_visible(false)` once during
`app_root` mount when the flag is set; tray Show works identically. The
egui side ignores the parameter — it already gates startup launch from
`cli.start_minimized` in `main.rs`.

### New primitives (F3)

- `Tabs` — full WAI-ARIA Tabs pattern (`role="tablist"`/`tab`,
  focus-roving with `tabindex` 0|-1, arrow keys + Home/End for cycle and
  activate). Stateless: caller owns `value`. Reused by F11 (Modes).
- `StatusBar` — three-slot horizontal bar (start / middle / end). Fixed
  28px height. ARIA-neutral wrapper — consumers add `role="status"` /
  `aria-live` only on the specific elements they want announced (e.g.,
  the engine-status badge in `StatusBarView`).

### Placeholder shell

`shell/placeholder.rs` and `assets/shell/placeholder-shell.css` are
**explicitly disposable at F5**. Treat them as scratch — F5 may replace
the entire grid template, not just slot contents. The shell exists so
F3's tray-bridge lifecycle can be observed against a coherent layout
(open the window, watch the status bar reflect engine state, click tray
Toggle, watch the badge flip).
```

- [ ] **Step 2: Verify the build matrix table is still accurate**

If the README has an existing build-matrix table (from F1), verify the row
`cargo run --no-default-features --features gui-dioxus` now means "Dioxus
shell + tray + lifecycle, production-viable" rather than F1's "Dioxus smoke
readout." Update the row's description if needed.

- [ ] **Step 3: Commit**

Invoke the `conventional-commits` skill, then:

```bash
git add crates/inputforge-gui-dx/README.md
git commit -m "docs(gui-dx): document F3 tray bridge, hide-to-tray lifecycle, and new primitives"
```

---

## Task 22: Manual lifecycle pass S1–S10 + final acceptance

Final acceptance gate. Walk through each lifecycle scenario from the spec and tick the corresponding acceptance bullets. This task does NOT modify code — it's the gating verification.

If any scenario fails, file the diagnosis as a new task at the end of this plan. Do NOT silently patch; F3 must close cleanly.

**Files:**
- None modified.

- [ ] **Step 1: Default-features regression check (S1 baseline + acceptance bullet 1)**

```bash
cargo run -p inputforge-app
```

Verify all today-egui behavior:
- Window opens, the egui status bar reflects engine state, tray icon visible.
- X-click closes window; tray icon stays.
- Tray Show GUI re-opens.
- Tray Activate/Deactivate flips engine status.
- Tray Quit exits cleanly.
- `--start-minimized` does not show the window.

If anything regressed, the egui flow has been disturbed — diagnose before continuing.

- [ ] **Step 2: gui-dioxus build (acceptance bullet 2)**

```bash
cargo build --no-default-features --features gui-dioxus
```

Expected: builds cleanly. All workspace lints pass.

If the workspace has clippy in CI, also run:
```bash
cargo clippy --no-default-features --features gui-dioxus -- -D warnings
```

- [ ] **Step 3: S1 — Dioxus normal startup (acceptance bullets 3 + 4)**

```bash
cargo run --no-default-features --features gui-dioxus
```

Verify:
- Window appears at 1280×800 with the placeholder shell.
- Top region: "Top toolbar (F5 owns contents)".
- Left region: "Left panel — devices (F6)".
- Center: Mappings/Modes Tabs above center placeholder.
- Status bar shows engine status badge, vertical separator, mode badge in the start slot; device count in middle; profile name in the end slot if a profile is loaded (default profile from `ensure_default_profile()` should appear).
- Engine status badge color matches `EngineStatus::Stopped` (Neutral) initially, then transitions when activated.
- Mode badge always renders (Neutral when value is "Default").

- [ ] **Step 4: S2 — X-click hides window (acceptance bullet 5)**

With the window open from Step 3, click the X close button.

Verify:
- Window vanishes. Tray icon still present.
- No console error.
- (Engine continues — verify by clicking tray Activate in next step.)

- [ ] **Step 5: S3 — Tray Show re-opens window**

Right-click the tray icon → "Show GUI".

Verify:
- Window reappears.
- All state intact (no flicker, status bar shows the same readouts).

- [ ] **Step 6: S4 + S7 — Tray Toggle in both visibility states (acceptance bullets 6 + 9)**

While the window is **visible**:
- Click tray "Activate" (or "Deactivate" if engine is already running).
- Verify status-bar badge color flips within ~16ms (Running ↔ Stopped).
- Click again — verify it flips back.

X-close the window again. While **hidden**:
- Click tray "Activate" / "Deactivate".
- Tray Show GUI to re-open.
- Verify status-bar badge reflects the new state immediately on Show.

- [ ] **Step 7: S5 — Tray Quit (acceptance bullet 7)**

Right-click tray → "Quit".

Verify:
- Window closes (if it was open).
- Process exits cleanly.
- In the terminal where `cargo run` was launched, look for the log lines:
  - "engine thread exited cleanly" (or similar)
  - HidHide unhide log if HidHide was active.
- After exit, verify no orphaned hidden devices: open Windows Device Manager → "View" → "Show Hidden Devices" — there should be no inputforge-managed entries marked as "this device is currently disconnected from the computer." (If HidHide is unavailable in the test environment, skip this check; the warning logged at startup confirms.)

- [ ] **Step 8: S5 — In-window tray Quit (acceptance bullet 8)**

```bash
cargo run --no-default-features --features gui-dioxus
```

With the window open and visible, right-click tray → "Quit".

Verify:
- Same outcome as Step 7. The listener task fires regardless of window visibility — the F1 in-window vs. out-of-window split is gone.

- [ ] **Step 9: S6 — `--start-minimized` (acceptance bullet 8)**

```bash
cargo run --no-default-features --features gui-dioxus -- --start-minimized
```

Verify:
- No window appears.
- Tray icon present.
- Tray Show GUI reveals the window.
- Tray Quit exits.

Repeat under egui:
```bash
cargo run -p inputforge-app -- --start-minimized
```

Verify identical behavior (today's egui already supports this).

- [ ] **Step 10: F1 cleanup verified (acceptance bullet 10)**

```bash
grep -n IS_GUI_DIOXUS crates/inputforge-app/src/main.rs
```

Expected: zero matches.

```bash
grep -nE 'fn (launch_gui_blocking|run_tray_loop|drain_stale_gui_events)' crates/inputforge-app/src/main.rs
```

Expected: each function preceded by a `#[cfg(feature = "gui-egui")]` line on the immediately-prior line. Verify by reading.

- [ ] **Step 11: Dev-loop continuity (acceptance bullets 11 + 12)**

```bash
dx serve --example bridge_demo --platform desktop
```

Verify:
- Placeholder shell renders with status bar reflecting seeded state ("Demo" mode, "Running" engine status, 1/1 devices).
- Edit `bridge_demo.rs` (e.g., change the seeded `current_mode` value) — RSX hot-reload applies within ~1s.
- Close (Ctrl+C in terminal — the example has no tray, so X-click hides the window with no recovery; this is expected).

```bash
dx serve --example component_gallery --platform desktop
```

Verify:
- Tabs section: arrow keys cycle, Home/End jump, Tab key roves focus into and out of the tablist. Disabled row doesn't respond.
- StatusBar section: composed-slots demo and empty-slots demo both render.
- Close.

- [ ] **Step 12: ARIA spot-check (acceptance bullets 13 + 14)**

While the gallery is running (re-launch if needed), open DevTools (right-click → Inspect; or in dev builds Ctrl+Shift+I).

In the Elements panel, find the Tabs demo and verify:
- Outer `<div role="tablist" aria-orientation="horizontal">`
- Each `<button role="tab" aria-selected="true|false" tabindex="0|-1">`
- The active tab has `aria-selected="true"` and `tabindex="0"`; others have `aria-selected="false"` and `tabindex="-1"`.

For the StatusBar demo composed-slot variant, verify:
- The outer wrapper is `<div class="if-status-bar">` with NO `role` attribute.
- Internally, the composed `start` slot's badges are not wrapped in any role at the gallery level (the gallery's StatusBar demo is plain composition, no live-region semantics — `role="status"` only appears in `StatusBarView`'s start slot).

Then run the actual app:
```bash
cargo run --no-default-features --features gui-dioxus
```

Inspect the running app's status bar. Verify:
- `<div class="if-status-bar if-placeholder-shell__status">` — no `role`.
- The engine-status `<span role="status" aria-live="polite">` wraps the engine-status `Badge` only.
- The mode `Badge`, device-count `<span>`, and profile `<span>` are NOT wrapped in `role="status"`.

- [ ] **Step 13: Run all tests one final time (acceptance bullet 15)**

```bash
cargo test
cargo test --no-default-features --features gui-dioxus
```

Expected: all tests pass under both feature flags.

- [ ] **Step 14: Final spec acceptance review**

Open `docs/superpowers/specs/2026-04-26-f3-app-shell-tray-bridge-design.md`. For each of the 15 acceptance bullets in §"Acceptance criteria", verify it has been hit. Tick each in your task tracker (or check off in the spec if working in a worktree).

- [ ] **Step 15: No commit**

Verification only.

---

## Risks summary (deferred from spec, surfaced for the implementer)

If you hit any of these mid-implementation, refer back to the spec's "Risks" section for context — they are documented decisions, not unknowns.

- **`set_close_behaviour` UK vs US spelling.** Both `request_quit` (Task 11) and `with_close_behaviour` (Task 13) must use the same spelling as the actual 0.7.x API. Verify with `cargo check` and adjust both call sites together.
- **`dioxus::desktop` vs `dioxus_desktop` import paths.** If the prelude doesn't re-export everything F3 needs, fall back to `dioxus_desktop::*` and add `dioxus-desktop = { workspace = true }` to `crates/inputforge-gui-dx/Cargo.toml`.
- **`muda::MenuEvent { id }` field constructor.** If muda's `MenuEvent` field becomes private in a future version, drop the one test that depends on the constructor and rely on the four `from_id` tests.
- **`std::sync::mpsc::Sender::send` vs `tokio::sync::mpsc::Sender::try_send`.** The engine command channel is std (synchronous, unbounded); the tray-action channel is tokio (bounded). Keep them straight — use `.send()` for engine commands, `.try_send()` for tray actions.

---

## Self-review checklist (run before handoff)

The author of this plan ran this self-review on 2026-04-26. The implementer should re-run if they make significant scope changes mid-execution.

- **Spec coverage.** Each of the 15 acceptance bullets in the spec is exercised by Tasks 22 (most), Tasks 6–8 (Tabs/StatusBar gallery + ARIA), Task 11 (lifecycle helpers), Task 9 (TrayAction tests), Task 20 (F1 cleanup), Task 18+19 (placeholder shell render). The "frontend-design invoked early" process check is covered by Task 8.
- **Placeholder scan.** No "TBD", no "implement later", no "similar to Task N" without code, no unexplained "appropriate error handling." Every code step has a code block.
- **Type consistency.** `TrayAction` (3 variants), `TrayMenuIds` (3 fields), `make_event_handler` (signature matches `with_custom_event_handler`'s expected closure type), `LaunchParams` (Send+Sync via `Arc<Mutex<Option<Receiver>>>`), `dispatch_toggle` (uses `ctx.commands.send`, not `try_send`), `apply_start_minimized` (called from a Dioxus scope so `window()` resolves), `request_quit` (no `&AppContext` argument; doesn't read `quit_requested`). All consistent across tasks.
- **Existing F1/F2 conventions honored.** Tabs and StatusBar follow the F2 primitive pattern (`merge_class`, `class: Option<String>`, sibling CSS, `.if-<name>` BEM-ish prefix). The disposable shell is shell-scoped (lives in `assets/shell/`, not `assets/components/`) and mounts its own CSS via `document::Stylesheet` rather than going through `ThemeProvider`.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-04-26-f3-app-shell-tray-bridge.md`. Two execution options:

1. **Subagent-Driven (recommended)** — Dispatch a fresh subagent per task, review between tasks, fast iteration. Use `superpowers:subagent-driven-development`.
2. **Inline Execution** — Execute tasks in this session using `superpowers:executing-plans`, batch execution with checkpoints.

Which approach?
