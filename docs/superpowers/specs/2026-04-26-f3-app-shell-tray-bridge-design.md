# F3 — Application Shell + Tray Bridge: Design Spec

**Status:** Design approved, ready for implementation plan
**Date:** 2026-04-26
**Parent spec:** [`2026-04-24-egui-to-dioxus-rewrite-design.md`](./2026-04-24-egui-to-dioxus-rewrite-design.md) — Foundation feature F3
**Predecessors:** [F1](./2026-04-24-f1-dioxus-scaffold-state-bridge-design.md) (state bridge), [F2](./2026-04-25-f2-design-system-design.md) (design system)

---

## Context

F3 is the third and final foundation feature. It does two things at once:

1. **Tray bridge.** Replace today's egui per-frame `MenuEvent::receiver().try_recv()` polling with a Dioxus-native event-driven flow: register a `muda::MenuEvent` callback on launch, forward events through a `tokio::sync::mpsc` channel, drain in a Dioxus task. Restore the full hide-to-tray window lifecycle that F1 punted on (X-click hides; tray Show/Toggle/Quit work whether the window is visible or hidden).
2. **Shell scaffold.** Stand up an "intentionally minimal" placeholder layout (top toolbar / left panel / center / status bar) so F5's IA redesign has something to reshape, plus two new F2-style design-system primitives (Tabs, StatusBar) that any future IA is likely to keep using regardless of how F5 reorganizes the surfaces.

Together they make `--features gui-dioxus` production-viable as a default at F14: tray works, window lifecycle works, the shell exists even though its contents are placeholders.

The egui GUI stays the default runtime behavior. F3 changes `inputforge-app/src/main.rs` only along `#[cfg(feature = "gui-dioxus")]` boundaries; the egui code paths and behavior are byte-identical to today.

---

## Confirmed design choices

Decisions made during brainstorming that shape this spec:

1. **Tray bridge pattern: callback-into-channel.** `muda::MenuEvent::set_event_handler(...)` registered exactly once inside the Dioxus crate's `launch_gui`. Handler runs on muda's worker thread and forwards a routed `TrayAction` through `tokio::sync::mpsc::UnboundedSender`. A Dioxus task `await`s `rx.recv()` and dispatches. No polling, no busy loop. Mutually exclusive with muda's `MenuEvent::receiver()` channel — by feature-flag separation, the egui crate keeps using `receiver()` and never coexists with the Dioxus handler at runtime.
2. **Hide-to-tray lifecycle: quit-aware close hook.** `Config::with_close_behaviour(WindowCloseBehaviour::WindowHides)` + `with_exits_when_last_window_closes(false)`. Close-event hook reads `state.quit_requested`: hides the window if `false`; allows the close to proceed (and exits the loop) if `true`. The flag already exists in `AppState` and is the same one today's egui Quit path sets — no new conceptual surface.
3. **`main.rs` divergence: Shape A (cfg-gated).** Under `gui-dioxus`: a single unconditional `launch_gui(...)` then `shutdown(...)`. Under `gui-egui`: today's flow unchanged. The two F1 `IS_GUI_DIOXUS` cfg-guards and the `IS_GUI_DIOXUS` sentinel const delete entirely. `run_tray_loop` and `drain_stale_gui_events` become `#[cfg(feature = "gui-egui")]`-only (deleted with the egui crate at F16).
4. **`--start-minimized` plumbing: launch_gui parameter.** A new `start_minimized: bool` argument on `launch_gui`, mirrored on the egui side (where it's ignored — today's egui gates startup launch from `main.rs` itself). The Dioxus side calls `set_visible(false)` once during `app_root` mount when the flag is set.
5. **Shell content calibration: α (pure scaffold + functional status bar).** Top toolbar and left/center panels render labeled placeholder text. Status bar is functional, reading `MetaSnapshot` and `ConfigSnapshot` for engine status / mode / device count / profile name. This is the cheapest way to prove the shell *plus* the tray bridge are wired in the same window — flip Activate from the tray and watch the engine-status badge change.
6. **Tabs as F2-style primitive.** `components/tabs.rs` + `assets/components/tabs.css` + gallery section. Full WAI-ARIA Tabs pattern (`role="tablist"`/`tab`, focus-roving, arrow keys, Home/End). F11 (Modes) reuses it.
7. **StatusBar as F2-style primitive.** `components/status_bar.rs` + `assets/components/status-bar.css` + gallery section. Presentation-only (`start` / `middle` / `end` slots, fixed 28px height). Status content (badges, separators, profile span) composed by the consumer using existing F2 primitives.
8. **No `AppShell` abstraction.** The placeholder layout grid lives in `shell/placeholder.rs` and is explicitly disposable. F5 may replace the entire grid template (not just slot contents). `app_root` directly mounts `ThemeProvider { PlaceholderShell {} }`.
9. **`impeccable:frontend-design` invoked early in F3 implementation.** Brief scoped to: shell layout visual direction (windows chrome polish, gridding, gutters, region affordances), tab-bar visual treatment, status-bar visual treatment. NOT IA — IA is F5.

## Non-goals (deferred to named later features)

- Top toolbar contents, navigation primary chrome → **F5**
- Left-panel device list / input tree → **F6**
- Center-panel mappings/modes content → **F7+ / F11**
- Toast queue, modal dialog primitive, dirty-state confirmation flow → **F4**
- Profile-name click → open profile manager (today's egui status-bar affordance) → **F14**
- IA redesign / wireframes / screen inventory → **F5**
- Light theme → out of scope for the whole rewrite until explicitly needed
- UI rendering test strategy → **F15 / F16** per parent-plan open question
- Multi-window support — F3's hide-to-tray model assumes one window. F12/F13/F14 may revisit if they introduce a second window.

---

## Architecture

### Crate layout (additions on top of F2)

```
crates/inputforge-gui-dx/
├── src/
│   ├── lib.rs                        # launch_gui — extended signature
│   ├── app.rs                        # app_root: spawns polling + tray listener; renders ThemeProvider { PlaceholderShell {} }
│   ├── bridge.rs                     # spawn_polling_task (F1, unchanged)
│   ├── context.rs                    # AppContext, snapshots (F1, unchanged)
│   ├── tray/
│   │   ├── mod.rs                    # NEW — install_event_handler, spawn_listener_task
│   │   └── action.rs                 # NEW — TrayAction enum, MenuId routing, from_event
│   ├── lifecycle/
│   │   └── mod.rs                    # NEW — close-hook gate, show_window, request_quit, apply_start_minimized
│   ├── shell/
│   │   ├── mod.rs                    # NEW — pub(crate) PlaceholderShell, StatusBarView; explicitly disposable at F5
│   │   ├── placeholder.rs            # NEW — the four-region grid (top/left/center/status)
│   │   └── status_bar_view.rs        # NEW — signal-bound consumer of the StatusBar primitive
│   ├── components/                   # F2 + new
│   │   ├── tabs.rs                   # NEW — F2-style primitive
│   │   ├── status_bar.rs             # NEW — F2-style primitive (presentation only)
│   │   ├── mod.rs                    # MODIFIED — re-export Tabs + StatusBar
│   │   └── ... (existing F2 primitives, unchanged)
│   ├── theme/                        # F2, unchanged
│   └── icons/                        # F2; may grow by ~2 icons (e.g., minimize/close)
├── assets/components/
│   ├── tabs.css                      # NEW
│   ├── status-bar.css                # NEW
│   └── ... (existing F2 CSS, unchanged)
└── examples/
    ├── bridge_demo.rs                # MODIFIED — wraps in PlaceholderShell so the dev loop renders the real shell
    └── component_gallery.rs          # MODIFIED — adds Tabs and StatusBar sections (variants, states, keyboard demo)
```

### Tray bridge

**`tray/action.rs` — internal action enum and routing:**

```rust
use muda::{MenuEvent, MenuId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrayAction {
    Show,
    Toggle,
    Quit,
}

#[derive(Debug, Clone)]
pub(crate) struct TrayMenuIds {
    pub show:   MenuId,
    pub toggle: MenuId,
    pub quit:   MenuId,
}

impl TrayMenuIds {
    pub fn from_tuple(ids: (MenuId, MenuId, MenuId)) -> Self {
        Self { show: ids.0, toggle: ids.1, quit: ids.2 }
    }
}

impl TrayAction {
    pub fn from_event(ev: &MenuEvent, ids: &TrayMenuIds) -> Option<Self> {
        if ev.id == ids.show   { return Some(Self::Show);   }
        if ev.id == ids.toggle { return Some(Self::Toggle); }
        if ev.id == ids.quit   { return Some(Self::Quit);   }
        None
    }
}
```

`from_event` is a pure function — unit-testable against synthetic `MenuEvent` fixtures.

**`tray/mod.rs` — handler installation and listener task:**

```rust
use tokio::sync::mpsc;
use dioxus::prelude::*;

use crate::context::AppContext;
use crate::lifecycle;
use self::action::{TrayAction, TrayMenuIds};

pub(crate) mod action;

/// Register the muda event handler exactly once. Returns the receiver half
/// for the listener task to consume. Called from `launch_gui` before
/// `LaunchBuilder::launch`.
pub(crate) fn install_event_handler(
    ids: TrayMenuIds,
) -> mpsc::UnboundedReceiver<TrayAction> {
    let (tx, rx) = mpsc::unbounded_channel();
    muda::MenuEvent::set_event_handler(Some(Box::new(move |ev| {
        if let Some(action) = TrayAction::from_event(&ev, &ids) {
            // Send is infallible until the receiver is dropped (which only
            // happens on app exit). A dropped send is harmless.
            let _ = tx.send(action);
        }
    })));
    rx
}

/// Spawn the listener task. Called from `app_root`'s `use_hook` so the task
/// is tied to the Dioxus runtime lifetime and auto-cancelled on teardown.
pub(crate) fn spawn_listener_task(
    mut rx: mpsc::UnboundedReceiver<TrayAction>,
    ctx: AppContext,
) {
    spawn(async move {
        while let Some(action) = rx.recv().await {
            match action {
                TrayAction::Show   => lifecycle::show_window(),
                TrayAction::Toggle => dispatch_toggle(&ctx),
                TrayAction::Quit   => lifecycle::request_quit(&ctx),
            }
        }
    });
}

fn dispatch_toggle(ctx: &AppContext) {
    use inputforge_core::engine::EngineCommand;
    use inputforge_core::state::EngineStatus;

    let status = ctx.state.read().engine_status;
    let cmd = match status {
        EngineStatus::Running                          => EngineCommand::Deactivate,
        EngineStatus::Paused | EngineStatus::Stopped   => EngineCommand::Activate,
    };
    let _ = ctx.commands.send(cmd);
}
```

### Lifecycle module

**`lifecycle/mod.rs` — close-hook gate, visibility helpers, quit pathway:**

```rust
use dioxus_desktop::{window, WindowCloseBehaviour};

use crate::context::AppContext;

/// Outcome of evaluating a close-requested event against the quit flag.
/// Pure-function output type so the gate is unit-testable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CloseGate {
    KeepHidden,
    AllowExit,
}

pub(crate) fn evaluate_close(quit_requested: bool) -> CloseGate {
    if quit_requested { CloseGate::AllowExit } else { CloseGate::KeepHidden }
}

/// Tray Show — bring the window back to foreground.
pub(crate) fn show_window() {
    let w = window();
    w.set_visible(true);
    w.set_focus();
}

/// Tray Quit — set the quit flag and trigger the close pathway. The
/// close hook then sees `quit_requested = true` and allows the loop
/// to exit. Falls back to an event-loop-proxy exit if the close hook
/// is unavailable on the pinned Dioxus 0.7.x; both paths converge on
/// `launch_gui` returning so `main.rs::shutdown()` runs.
pub(crate) fn request_quit(ctx: &AppContext) {
    ctx.state.write().quit_requested = true;
    window().close();
}

/// Apply --start-minimized once during app_root mount.
pub(crate) fn apply_start_minimized(start_minimized: bool) {
    if start_minimized {
        window().set_visible(false);
    }
}
```

**Close-hook integration.** Dioxus 0.7's exact API for the close-requested hook varies across minor versions (it has been moved between `Config::with_close_behaviour`, `use_window`, and a per-window event handler). The implementation plan verifies the current shape against the pinned `dioxus 0.7.x` at impl start using `latest-packages`. Two paths are acceptable:

- **Primary path:** the close-requested hook (whichever 0.7.x exposes it) reads `quit_requested` via the AppContext and returns `CloseGate::KeepHidden` ⇒ `set_visible(false)` and consume the event, or `CloseGate::AllowExit` ⇒ allow the close to propagate. Loop exits because `with_exits_when_last_window_closes` becomes irrelevant once the close is allowed (the close path itself terminates the loop on the only window).
- **Fallback path:** if the pinned 0.7.x has no cancellable close hook, register a tao-level `EventLoopProxy` user event during launch; `request_quit` sends the user event; an `event_handler` on the desktop config matches that event and calls the appropriate exit-loop API. Slightly more code, semantically identical.

Both paths route through the same `evaluate_close` pure function so unit tests don't depend on which is selected.

### Shell

**`shell/placeholder.rs` — disposable layout:**

```rust
use dioxus::prelude::*;
use crate::components::Tabs;
use crate::shell::status_bar_view::StatusBarView;

#[component]
pub(crate) fn PlaceholderShell() -> Element {
    let center_tab = use_signal(|| "mappings".to_owned());

    rsx! {
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

**CSS** (sketch — `assets/components/placeholder-shell.css`, NOT a design-system token; deletes at F5):

```css
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
.if-placeholder-shell__top    { grid-area: top;    border-bottom: 1px solid var(--color-border); }
.if-placeholder-shell__left   { grid-area: left;   border-right:  1px solid var(--color-border); padding: var(--space-3); }
.if-placeholder-shell__center { grid-area: center; padding: var(--space-3); }
.if-placeholder-shell__status { grid-area: status; }   /* applied by StatusBarView via the StatusBar `class` prop */
```

**Disposable** is a load-bearing word: the entire `shell/placeholder.rs` and `assets/components/placeholder-shell.css` are expected to be deleted by F5 once the IA redesign produces a real layout. Their token usage stays correct because they reference F2 tokens — no tokens are coined in F3.

### Tabs primitive

**`components/tabs.rs`:**

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
    #[props(default)] pub class: Option<String>,
    #[props(default)] pub disabled: bool,
}

#[component]
pub fn Tabs(props: TabsProps) -> Element {
    let combined = merge_class("if-tabs", "", props.class.as_deref());

    rsx! {
        div {
            class: "{combined}",
            role: "tablist",
            "aria-orientation": "horizontal",
            // … per-tab buttons rendered with role="tab", aria-selected,
            // tabindex 0/-1 (focus-roving), onkeydown for Left/Right/Home/End.
        }
    }
}
```

**Contract:**
- `role="tablist"`, each tab is `role="tab"`, `aria-selected={is_active}`, `tabindex={if active { 0 } else { -1 }}`.
- Left/Right arrow keys move focus between tabs **and activate** (automatic activation, matches today's egui Mappings/Modes behavior).
- Home / End jump to first / last (and activate).
- The component emits `onchange(id)` on activation; it does not own the tab panel — the caller swaps content. Keeping it stateless avoids over-coupling for F11.
- `disabled` short-circuits keyboard and click interaction; visible state via `.if-tabs--disabled`.

**Gallery coverage:** active/inactive/hover/focus-visible/disabled per tab, plus a "tab with three items" demo that shows arrow-key cycling.

### StatusBar primitive

**`components/status_bar.rs`:**

```rust
use dioxus::prelude::*;
use super::merge_class;

#[component]
pub fn StatusBar(
    start:  Element,
    middle: Element,
    end:    Element,
    #[props(default)] class: Option<String>,
) -> Element {
    let combined = merge_class("if-status-bar", "", class.as_deref());
    rsx! {
        div { class: "{combined}", role: "status",
            div { class: "if-status-bar__start",  {start}  }
            div { class: "if-status-bar__middle", {middle} }
            div { class: "if-status-bar__end",    {end}    }
        }
    }
}
```

**Contract:**
- Three named slots; `start` flows left, `end` is right-anchored, `middle` fills the gap.
- Fixed height of 28px (matches today's egui status bar; will be reviewed by `impeccable:frontend-design` and may change).
- Presentation-only — no clickability, no badges, no separators baked in. Consumers compose using existing F2 primitives (`Button` with `variant=ghost`, `Badge`, `Separator`).
- `role="status"` for assistive tech.

**Gallery coverage:** one section showing a representative composition (e.g., `Badge + Separator + Badge` in start, plain text middle, profile-name span at end), plus an empty-slots demo to verify slot independence.

### Shell consumer of StatusBar

**`shell/status_bar_view.rs`** — the only F3 surface that subscribes to AppContext signals. Reads `meta` and `config`, composes a real status bar that reflects engine state live.

```rust
use dioxus::prelude::*;
use inputforge_core::state::EngineStatus;

use crate::components::{Badge, BadgeVariant, Separator, SeparatorOrientation, StatusBar};
use crate::context::AppContext;

#[component]
pub(crate) fn StatusBarView() -> Element {
    let ctx = use_context::<AppContext>();

    let status     = use_memo(move || ctx.meta.read().engine_status);
    let mode       = use_memo(move || ctx.meta.read().current_mode.clone());
    let profile    = use_memo(move || ctx.meta.read().profile_name.clone());
    let dev_count  = use_memo(move || {
        let cfg = ctx.config.read();
        let connected = cfg.devices.iter().filter(|d| d.connected).count();
        (connected, cfg.devices.len())
    });

    rsx! {
        StatusBar {
            class: "if-placeholder-shell__status",
            start: rsx! {
                Badge {
                    variant: status_to_variant(*status.read()),
                    "{status_label(*status.read())}"
                }
                Separator { orientation: SeparatorOrientation::Vertical }
                Badge { variant: BadgeVariant::Neutral, "{mode}" }
            },
            middle: rsx! {
                let (c, t) = *dev_count.read();
                span { "{c}/{t} devices" }
            },
            end: rsx! {
                if let Some(name) = profile.read().as_ref() {
                    span { "{name}" }
                }
            },
        }
    }
}

fn status_to_variant(s: EngineStatus) -> BadgeVariant {
    match s {
        EngineStatus::Running => BadgeVariant::Success,
        EngineStatus::Paused  => BadgeVariant::Warning,
        EngineStatus::Stopped => BadgeVariant::Neutral,
    }
}

fn status_label(s: EngineStatus) -> &'static str {
    match s {
        EngineStatus::Running => "Running",
        EngineStatus::Paused  => "Paused",
        EngineStatus::Stopped => "Stopped",
    }
}
```

Profile-name click ⇒ open profile manager is **not wired** in F3 (see Non-goals; F14 owns profile surface).

### `launch_gui` signature change

```rust
// before (F1):
pub fn launch_gui(
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    tray_menu_ids: (MenuId, MenuId, MenuId),
    settings: AppSettings,
) -> anyhow::Result<()>

// after (F3):
pub fn launch_gui(
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    tray_menu_ids: (MenuId, MenuId, MenuId),
    settings: AppSettings,
    start_minimized: bool,
) -> anyhow::Result<()>
```

**Mirrored on the egui crate** so `main.rs`'s call site is identical under both feature flags. The egui side ignores the parameter (`let _ = start_minimized;`) — today's egui already gates startup launch on `cli.start_minimized` from `main.rs` itself. The parameter is deleted at F16 cleanup.

### `app.rs` shape

```rust
use dioxus::prelude::*;

use crate::bridge::spawn_polling_task;
use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, RawHandles};
use crate::lifecycle;
use crate::shell::PlaceholderShell;
use crate::theme::ThemeProvider;
use crate::tray;

pub(crate) fn app_root() -> Element {
    let raw    = use_context::<RawHandles>();
    let params = use_context::<LaunchParams>();   // start_minimized + listener_rx

    let meta   = use_signal(MetaSnapshot::default);
    let config = use_signal(ConfigSnapshot::default);
    let live   = use_signal(LiveSnapshot::default);

    let ctx = AppContext { /* same as F1/F2 */ };
    use_context_provider(|| ctx.clone());

    use_hook(|| spawn_polling_task(ctx.clone()));
    use_hook(|| {
        if let Some(rx) = params.listener_rx.take() {
            tray::spawn_listener_task(rx, ctx.clone());
        }
    });
    use_hook(|| lifecycle::apply_start_minimized(params.start_minimized));

    rsx! { ThemeProvider { PlaceholderShell {} } }
}
```

`LaunchParams` is a small `Clone` struct installed via `LaunchBuilder::with_context` alongside `RawHandles`:

```rust
#[derive(Clone)]
pub(crate) struct LaunchParams {
    pub start_minimized: bool,
    /// Wrapped in `Rc<Cell<Option<_>>>` so the receiver can move into the
    /// listener task on first scope mount. `Cell::take()` empties it after
    /// use, so re-mounts (theoretically possible during recovery) become
    /// no-ops rather than double-spawning the task.
    pub listener_rx: std::rc::Rc<std::cell::Cell<Option<tokio::sync::mpsc::UnboundedReceiver<TrayAction>>>>,
}
```

This mirrors F1's `RawHandles` pattern — params installed pre-launch via `with_context`, picked up inside the runtime via `use_context`. The `Rc<Cell<Option<_>>>` wrapping is the idiomatic Dioxus "move-once into a hook" pattern for non-`Clone` resources like an mpsc receiver.

---

## App-side changes (`crates/inputforge-app/src/main.rs`)

Shape A — divergence under `cfg`. Surgical changes; the egui flow stays byte-identical.

**Removed unconditionally** (F1's lifecycle workarounds):

```rust
// DELETE — F1 sentinel and cfg consts:
#[cfg(feature = "gui-egui")] const IS_GUI_DIOXUS: bool = false;
#[cfg(feature = "gui-dioxus")] const IS_GUI_DIOXUS: bool = true;
```

**Restructured startup branch:**

```rust
// BEFORE (F1):
let mut quit_requested = false;
if !cli.start_minimized {
    for action in launch_gui_blocking(&tray, &state, &cmd_tx, &settings) {
        match action { /* ... */ }
    }
    if IS_GUI_DIOXUS { /* F1 guard 1 — DELETED */ }
}
if !quit_requested {
    run_tray_loop(&tray, &state, &cmd_tx, &settings);
}
shutdown(cmd_tx, engine_handle);

// AFTER (F3):
#[cfg(feature = "gui-dioxus")]
{
    if let Err(e) = launch_gui(
        Arc::clone(&state), cmd_tx.clone(), tray.menu_item_ids(),
        settings.clone(), cli.start_minimized,
    ) {
        tracing::error!(%e, "GUI exited with error");
    }
    // launch_gui only returns on real Quit. Fall through to shutdown.
}

#[cfg(feature = "gui-egui")]
{
    let mut quit_requested = false;
    if !cli.start_minimized {
        for action in launch_gui_blocking(&tray, &state, &cmd_tx, &settings) {
            match action {
                TrayAction::Quit => quit_requested = true,
                TrayAction::ToggleActivation => { /* unchanged */ }
                TrayAction::ShowGui => {}
            }
        }
    }
    if !quit_requested {
        run_tray_loop(&tray, &state, &cmd_tx, &settings);
    }
}

shutdown(cmd_tx, engine_handle);
```

**Functions newly gated to `gui-egui`-only** (definitions stay in `main.rs` but with `#[cfg(feature = "gui-egui")]` on the items themselves):

- `launch_gui_blocking` — the Dioxus path calls `launch_gui` directly; no post-close action drain is needed.
- `run_tray_loop` — its purpose was the post-window-close Win32 message pump. Under Dioxus, tao owns the pump for the entire process lifetime.
- `drain_stale_gui_events` — its purpose was draining queued `MenuEvent`s during the window-closed gap, which no longer exists.

F2's F1-introduced cfg-gated `use inputforge_gui::launch_gui;` / `use inputforge_gui_dx::launch_gui;` selection stays — `main.rs` still has one symbolic `launch_gui` resolved at compile time. The Dioxus path calls it directly; the egui path goes through `launch_gui_blocking` for its action drain, calling `launch_gui` inside.

**`launch_gui_blocking` signature update** (egui-only after F3, but mirrored before): the existing internal call to `launch_gui(...)` adds the `cli.start_minimized` argument (passed as a parameter). Function signature unchanged — the call site inside the function gains the new arg.

**Build matrix** (unchanged from F1, but the meaning of "Dioxus" rows is now stronger):

| Command | Result |
|---|---|
| `cargo build` / `cargo run` | egui (default), today's lifecycle preserved |
| `cargo build --no-default-features --features gui-dioxus` | Dioxus, hide-to-tray fully wired |
| `cargo run --no-default-features --features gui-dioxus` | Dioxus, X-click hides, tray Show/Toggle/Quit work |
| `cargo build --features gui-dioxus` (default still on) | compile error (F1 guard) |
| `cargo build --no-default-features` | compile error (F1 guard) |

CI matrix entry from F1 (`--no-default-features --features gui-dioxus build`) stays as-is; no new entry needed.

---

## Lifecycle scenarios end-to-end

**S1 — Normal startup, no flags.**
1. `main()` builds engine + tray + state.
2. Calls `launch_gui(state, cmd_tx, menu_ids, settings, start_minimized: false)`.
3. Inside Dioxus crate: `tray::install_event_handler(menu_ids)` registers muda callback, returns `rx`. `LaunchBuilder::desktop().with_cfg(...)` includes `WindowCloseBehaviour::WindowHides` and `with_exits_when_last_window_closes(false)`.
4. `app_root` mounts: signals created, polling task spawned, listener task spawned. Renders `ThemeProvider { PlaceholderShell {} }`.
5. Window appears at 1280×800 with the placeholder shell visible. Status bar reflects engine state immediately (polling task already running).

**S2 — User clicks X to close window.**
1. Tao fires close-requested. The close hook reads `state.read().quit_requested → false`.
2. `evaluate_close(false) → CloseGate::KeepHidden`. Hook calls `set_visible(false)` and consumes the event.
3. Window vanishes from taskbar. Tray icon stays. Engine unaffected. Polling task keeps ticking (idle = no Signal writes via PartialEq gate).

**S3 — Tray Show GUI clicked while window hidden.**
1. muda callback fires on muda's worker thread, `TrayAction::Show` arrives in the listener channel.
2. Listener task wakes (`rx.recv().await`), calls `lifecycle::show_window()` → `set_visible(true)` + `set_focus()`.
3. Window reappears with current state already rendered (polling never stopped).

**S4 — Tray Activate/Deactivate clicked (window hidden or visible).**
1. Listener receives `TrayAction::Toggle`. Reads `ctx.state` for current `EngineStatus`, sends `EngineCommand::Activate`/`Deactivate` over `ctx.commands`.
2. Engine flips status. Polling picks it up next tick. If window visible: status-bar badge color changes within ~16ms. If hidden: change is observable on next Show.

**S5 — Tray Quit clicked.**
1. Listener receives `TrayAction::Quit`. Calls `lifecycle::request_quit(&ctx)`:
   - `ctx.state.write().quit_requested = true;`
   - `dioxus_desktop::window().close();` → triggers close-requested event.
2. Close hook now reads `quit_requested = true`, returns `CloseGate::AllowExit`. Allows the close to propagate. Loop exits.
3. `LaunchBuilder::launch` returns. `launch_gui` returns `Ok(())`. `main()` runs `shutdown(cmd_tx, engine_handle)` → engine thread joins → `HidHide` unhide + `vJoy` release fire via `Drop`.

**S6 — `--start-minimized`.**
1. `main()` calls `launch_gui(..., start_minimized: true)`.
2. `app_root` mounts; the `apply_start_minimized(true)` hook calls `set_visible(false)` immediately. Window never appears in foreground.
3. Tray Show works identically to S3.

**S7 — Window-shown tray click (e.g., Quit while window has focus).**
Same code path as S4/S5 — the listener task is the single dispatch point regardless of window visibility. **The F1 latent in-window-vs-out-of-window split is eliminated** (F1 left in-window tray clicks queued in muda's global channel until window close).

**S8 — Engine warning surfaces while window hidden.**
Polling task updates `MetaSnapshot.warnings` Signal. No subscribers act on it directly (toast queue is F4). Warning is visible next time user opens the window via the readout count. Matches today's egui under start-minimized.

**S9 — Engine thread panic (cosmetic; no F3-specific change).**
Engine thread exits, `state.engine_status` stays at last value. Polling continues, status bar stays stale. Pre-existing, not introduced by F3.

**S10 — User clicks profile name in status bar.**
Out of scope. The profile slot renders a `<span>`, not a button. Wiring it to "open profile manager" is F14's job.

---

## Files

**Created:**

```
crates/inputforge-gui-dx/src/tray/mod.rs                   # install_event_handler, spawn_listener_task, dispatch_toggle
crates/inputforge-gui-dx/src/tray/action.rs                # TrayAction, TrayMenuIds, from_event (+ unit tests)
crates/inputforge-gui-dx/src/lifecycle/mod.rs              # CloseGate, evaluate_close (+ unit tests), show_window, request_quit, apply_start_minimized
crates/inputforge-gui-dx/src/shell/mod.rs                  # pub(crate) re-exports
crates/inputforge-gui-dx/src/shell/placeholder.rs          # PlaceholderShell — disposable at F5
crates/inputforge-gui-dx/src/shell/status_bar_view.rs      # StatusBarView — signal-bound consumer of StatusBar primitive
crates/inputforge-gui-dx/src/components/tabs.rs            # Tabs primitive
crates/inputforge-gui-dx/src/components/status_bar.rs      # StatusBar primitive
crates/inputforge-gui-dx/assets/components/tabs.css        # Tabs CSS
crates/inputforge-gui-dx/assets/components/status-bar.css  # StatusBar CSS
crates/inputforge-gui-dx/assets/components/placeholder-shell.css  # disposable at F5
```

**Modified:**

```
crates/inputforge-gui-dx/src/lib.rs                       # launch_gui signature: + start_minimized; install_event_handler call; new module decls
crates/inputforge-gui-dx/src/app.rs                       # app_root spawns listener_task, applies start_minimized; renders PlaceholderShell
crates/inputforge-gui-dx/src/components/mod.rs            # re-export Tabs and StatusBar
crates/inputforge-gui-dx/examples/bridge_demo.rs          # wrap in PlaceholderShell so dev loop renders the real shell
crates/inputforge-gui-dx/examples/component_gallery.rs    # add Tabs and StatusBar gallery sections
crates/inputforge-gui-dx/README.md                        # document tray bridge model, hide-to-tray lifecycle, new primitives
crates/inputforge-gui/src/lib.rs                          # launch_gui signature: + start_minimized: bool (ignored)
crates/inputforge-app/src/main.rs                         # delete IS_GUI_DIOXUS sentinel, F1 guards; cfg-split startup branch; cfg-gate run_tray_loop, drain_stale_gui_events, launch_gui_blocking on gui-egui
```

**Reused (not modified) from F1/F2:**
- `crates/inputforge-gui-dx/src/context.rs` — `AppContext`, snapshots
- `crates/inputforge-gui-dx/src/bridge.rs` — polling task
- `crates/inputforge-gui-dx/src/theme/mod.rs` — `ThemeProvider`
- All F2 components except `mod.rs` (re-exports updated)

---

## Acceptance criteria

- [ ] `cargo build` (default) and `cargo run` (default) produce today's egui behavior. Manual smoke: launch, X-click closes window and keeps tray alive, tray Show re-opens, tray Quit exits cleanly.
- [ ] `cargo build --no-default-features --features gui-dioxus` succeeds with all workspace lints passing.
- [ ] `cargo run --no-default-features --features gui-dioxus` opens a window, shows the placeholder shell with labeled regions and a working status bar.
- [ ] **Status bar reflects live engine state.** Engine status badge color matches `EngineStatus`; mode badge shows `current_mode`; device count `c/t devices` updates as devices connect/disconnect; profile name shows when present.
- [ ] **X-click hides the window.** Tray icon stays. Window reappears via tray Show with all state intact (no re-init flicker).
- [ ] **Tray Toggle** flips engine status. Status-bar badge updates within ~16ms when window is visible; visible immediately on next Show when window is hidden.
- [ ] **Tray Quit** ends the process cleanly: `shutdown()` runs, engine thread joins, `HidHide` unhide and `vJoy` release fire via `Drop`. Verified by absence of orphaned hidden-device entries after exit.
- [ ] **`--start-minimized`** never shows the window in foreground; tray Show reveals it. Verified under both `gui-egui` (today's behavior, unchanged) and `gui-dioxus`.
- [ ] **In-window tray Quit** behaves identically to out-of-window. Test both: open window → click tray Quit; close window via X → click tray Quit.
- [ ] **In-window tray Toggle** updates status-bar badge in real time (proves the listener task fires regardless of window visibility, fixing F1's latent in-window race).
- [ ] **F1 cleanup completed:** `IS_GUI_DIOXUS` const, both F1 cfg-guards in `main()` and `run_tray_loop` are deleted; `run_tray_loop`, `drain_stale_gui_events`, `launch_gui_blocking` are gated `#[cfg(feature = "gui-egui")]`.
- [ ] `dx serve --example bridge_demo` (F1 path, modified) renders the new placeholder shell with status bar reflecting seeded state. RSX hot-reload works.
- [ ] `dx serve --example component_gallery` (F2 path, modified) renders Tabs and StatusBar sections with all interaction states demonstrable manually. Keyboard nav for Tabs verified (Left/Right, Home/End, focus-roving via Tab key).
- [ ] **Tabs ARIA contract:** `role="tablist"` / `role="tab"` / `aria-selected` / `tabindex` 0|-1 verified via DevTools accessibility inspector.
- [ ] **`impeccable:frontend-design`** invoked early in F3 implementation, brief scoped to "shell layout direction + tab-bar + status-bar visual treatment, scoped to placeholder regions"; output committed before component CSS finalization.
- [ ] `cargo test -p inputforge-gui-dx` passes — unit tests on `TrayAction::from_event`, `lifecycle::evaluate_close`, plus inherited F1 `from_state` tests.

---

## Test strategy

- **Unit tests** in `crates/inputforge-gui-dx/src/tray/action.rs` under `#[cfg(test)] mod tests`: synthesize `MenuEvent` fixtures, assert `TrayAction::from_event` routing.
- **Unit tests** in `crates/inputforge-gui-dx/src/lifecycle/mod.rs` under `#[cfg(test)] mod tests`: `evaluate_close(true) == AllowExit`, `evaluate_close(false) == KeepHidden`. Pure-function gate.
- **No new CI matrix entry.** F1's `--no-default-features --features gui-dioxus build` is sufficient.
- **Manual interaction pass** documented in this spec (lifecycle scenarios S1–S10). Each scenario has an explicit acceptance bullet above.
- **UI rendering automation deferred** per parent-plan open question (Dioxus testing story).

---

## Risks

- **Dioxus 0.7 close-event hook API drift.** The exact name and shape of the close-requested hook varies across `0.7.x` minor versions. Mitigation: pin and verify against `latest-packages` at impl start; if the chosen 0.7.x lacks a cancellable close hook, fall back to a tao `EventLoopProxy` user-event path. Both paths route through the same `evaluate_close` pure function — unit tests are unaffected.
- **muda `set_event_handler` mutual exclusivity.** Registering a handler stops `MenuEvent::receiver()` from receiving. The egui crate uses `receiver()`, the Dioxus crate uses `set_event_handler` — they never coexist (mutually-exclusive feature flags, F1's `compile_error!` guard ensures this at compile time). Structural rather than runtime risk.
- **Listener task leak on muda handler re-registration.** If anything calls `install_event_handler` twice, the older `tx` half is dropped and its `rx` task exits cleanly, but a transient burst of double-dispatch is possible during the swap. Mitigation: `install_event_handler` is called exactly once from `launch_gui`, never inside any RSX `use_hook`. Documented as a code-review checkpoint.
- **Multiple-window risk.** The hide-to-tray model assumes one window. Today's GUI is single-window; F12/F13/F14 *might* introduce a second window (e.g., calibration as separate). Documented as a precondition for those features — they'll need to revisit `WindowCloseBehaviour` per-window and decide whether `request_quit` should close all windows or just the main one.
- **`set_visible` race on rapid tray-click bursts.** Spam-clicking Show/Hide could interleave. Tao serializes window-visibility calls on Windows so the worst case is brief flicker, not state corruption. Acceptable.
- **Egui crate's `start_minimized: bool` parameter.** Added to keep `main.rs`'s call site identical under both feature flags; the egui crate ignores it because today's main.rs already gates startup launch from `cli.start_minimized` itself. Deleted at F16 cleanup.
- **F1 signal-coherency open question stays open.** F3 doesn't add live subscribers, so the F12 precondition stands unchanged.
- **Placeholder shell visual rough edges.** The disposable `placeholder-shell.css` is intentionally simple. `impeccable:frontend-design` is invited to polish the shell-level chrome (gutters, borders, region affordances) but **NOT** to redesign IA — that's F5. Mitigation: brief frontend-design with explicit scope.

---

## Open questions (inherited, not decided here)

- **Testing story for the Dioxus GUI** — parent-plan open question; F3 adds two more pure-function unit tests but commits to nothing on rendering automation.
- **Exact Dioxus and `dioxus-cli` versions** — pinned at implementation start via `latest-packages` against <https://crates.io>.
- **Live-signal coherency** (F1 deferral) — F3 doesn't touch this; F12 precondition unchanged.

---

## Next steps

1. Commit this spec to git.
2. Invoke `superpowers:writing-plans` to produce a step-by-step implementation plan with TDD-friendly checkpoints. The plan should sequence:
   - `impeccable:frontend-design` invocation early (after token verification, before component CSS finalization),
   - Tabs and StatusBar primitives + gallery sections (verification continuity with F2),
   - tray bridge + lifecycle module + unit tests,
   - placeholder shell + StatusBarView wiring,
   - `main.rs` Shape A divergence + F1-cleanup deletions,
   - end-to-end manual lifecycle pass (S1–S10) as the final acceptance gate.
