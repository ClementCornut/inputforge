# F3 — Application Shell + Tray Bridge: Design Spec

**Status:** Design approved, ready for implementation plan
**Date:** 2026-04-26
**Parent spec:** [`2026-04-24-egui-to-dioxus-rewrite-design.md`](./2026-04-24-egui-to-dioxus-rewrite-design.md) — Foundation feature F3
**Predecessors:** [F1](./2026-04-24-f1-dioxus-scaffold-state-bridge-design.md) (state bridge), [F2](./2026-04-25-f2-design-system-design.md) (design system)

---

## Context

F3 is the third and final foundation feature. It does two things at once:

1. **Tray bridge.** Replace today's egui per-frame `MenuEvent::receiver().try_recv()` polling with a Dioxus-native event-driven flow: observe `UserWindowEvent::MudaMenuEvent` (which `dioxus-desktop` already forwards from its own muda handler) via `Config::with_custom_event_handler`, route through a bounded `tokio::sync::mpsc` channel, drain in a Dioxus task. Restore the full hide-to-tray window lifecycle that F1 punted on (X-click hides; tray Show/Toggle/Quit work whether the window is visible or hidden).
2. **Shell scaffold.** Stand up an "intentionally minimal" placeholder layout (top toolbar / left panel / center / status bar) so F5's IA redesign has something to reshape, plus two new F2-style design-system primitives (Tabs, StatusBar) that any future IA is likely to keep using regardless of how F5 reorganizes the surfaces.

Together they make `--features gui-dioxus` production-viable as a default at F14: tray works, window lifecycle works, the shell exists even though its contents are placeholders.

The egui GUI stays the default runtime behavior. F3 changes `inputforge-app/src/main.rs` only along `#[cfg(feature = "gui-dioxus")]` boundaries; the egui code paths and behavior are byte-identical to today.

---

## Confirmed design choices

Decisions made during brainstorming that shape this spec:

1. **Tray bridge pattern: observe Dioxus's forwarded muda events.** `dioxus-desktop` 0.7.6 itself unconditionally registers `muda::MenuEvent::set_event_handler` and forwards every event as `UserWindowEvent::MudaMenuEvent(MenuEvent)` (`dioxus-desktop-0.7.6/src/app.rs:449`, `src/ipc.rs:6,12`). F3 does **not** call `set_event_handler` (Dioxus would clobber it on next launch tick). Instead it installs a `Config::with_custom_event_handler(...)` closure that pattern-matches `tao::event::Event::UserEvent(UserWindowEvent::MudaMenuEvent(ev))`, routes via `TrayAction::from_event`, and forwards through `tokio::sync::mpsc::Sender` (bounded channel). A Dioxus task `await`s `rx.recv()` and dispatches. The egui crate keeps using `MenuEvent::receiver()` and never coexists with the Dioxus path at runtime (mutually exclusive features).
2. **Hide-to-tray lifecycle: per-window close-behavior switch on Quit.** `Config::with_close_behaviour(WindowCloseBehaviour::WindowHides)` (X-click hides natively — Dioxus does the work, F3 has no close-handler code path). `exit_on_last_window_close` is left at the default `true`. On Tray Quit, F3 calls `window().set_close_behavior(WindowCloseBehaviour::WindowCloses)` then `window().close()`: Dioxus destroys the window, observes zero remaining webviews, and the event loop exits naturally. The `quit_requested` flag stays in `AppState` (egui still uses it) but is **not read** on the Dioxus path — the close-behavior switch is the single Quit pathway. Verified against `dioxus-desktop-0.7.6/src/desktop_context.rs:177` (`set_close_behavior` is mutable post-launch) and `src/app.rs:209-219` (last-window-closed exit).
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
│   ├── context.rs                    # AppContext, snapshots (F1; only the #[expect(dead_code)] on commands removed)
│   ├── tray/
│   │   ├── mod.rs                    # NEW — make_event_handler (closure for Config::with_custom_event_handler), spawn_listener_task
│   │   └── action.rs                 # NEW — TrayAction enum, MenuId routing, from_event
│   ├── lifecycle/
│   │   └── mod.rs                    # NEW — show_window, request_quit, apply_start_minimized
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
├── assets/
│   ├── components/
│   │   ├── tabs.css                  # NEW
│   │   ├── status-bar.css            # NEW
│   │   └── ... (existing F2 CSS, unchanged)
│   └── shell/
│       └── placeholder-shell.css     # NEW — disposable at F5; not a design-system token
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

// Intentionally no `from_tuple` helper — `launch_gui` destructures the
// `tray_menu_ids` argument inline (`let (show, toggle, quit) = ids;`).

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

**`tray/mod.rs` — custom event handler factory and listener task:**

```rust
use dioxus::prelude::*;
use dioxus_desktop::tao::event::Event as TaoEvent;
use dioxus_desktop::tao::event_loop::EventLoopWindowTarget;
use dioxus_desktop::ipc::UserWindowEvent;
use tokio::sync::mpsc;

use crate::context::AppContext;
use crate::lifecycle;
use self::action::{TrayAction, TrayMenuIds};

pub(crate) mod action;

/// Capacity for the tray-action channel. Sized for human-click cadence
/// (≪ 1 Hz peak realistic burst); 8 is a comfortable safety margin.
pub(crate) const CHANNEL_CAPACITY: usize = 8;

/// Build the closure passed to `Config::with_custom_event_handler`. The
/// closure runs on the tao event-loop thread; it must not block. We
/// `try_send` and log any overflow rather than wait — overflow is
/// effectively impossible at human input rates, but a dropped send must
/// never deadlock the event loop.
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
        // All other tao events fall through to Dioxus's own handling.
        // The handler is observe-only; we never mutate ControlFlow.
    }
}

/// Spawn the listener task. Called from `app_root`'s `use_hook` so the task
/// is tied to the Dioxus runtime lifetime and auto-cancelled on teardown.
pub(crate) fn spawn_listener_task(
    mut rx: mpsc::Receiver<TrayAction>,
    ctx: AppContext,
) {
    spawn(async move {
        while let Some(action) = rx.recv().await {
            match action {
                TrayAction::Show   => lifecycle::show_window(),
                TrayAction::Toggle => dispatch_toggle(&ctx),
                TrayAction::Quit   => lifecycle::request_quit(),
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
    let _ = ctx.commands.try_send(cmd);
}
```

**Channel pairing.** `launch_gui` creates `let (tx, rx) = mpsc::channel(tray::CHANNEL_CAPACITY);`, threads `tx` into `Config::with_custom_event_handler(tray::make_event_handler(menu_ids, tx))`, and threads `rx` (wrapped per C2) into `LaunchParams.listener_rx`. There is no `install_event_handler` to call separately — handler installation is part of the `Config` builder chain; receiver delivery is part of the `with_context` payload.

### Lifecycle module

**`lifecycle/mod.rs` — visibility helpers and quit pathway. No close-hook gate (Dioxus owns close-requested handling).**

```rust
use dioxus_desktop::{window, WindowCloseBehaviour};

/// Tray Show — bring the window back to foreground.
pub(crate) fn show_window() {
    let w = window();
    w.set_visible(true);
    w.set_focus();
}

/// Tray Quit — switch this window's close behavior to `WindowCloses`,
/// then trigger close. Dioxus destroys the window, observes zero
/// remaining webviews, and the event loop exits because
/// `exit_on_last_window_close` is true (the default — F3 does not
/// override it). `launch_gui` returns; `main.rs::shutdown()` then runs.
///
/// `quit_requested` in `AppState` is **not** read on the Dioxus path
/// (egui still uses it). The close-behavior switch is the entire Quit
/// pathway — there is no flag to gate, no close-hook to wire.
pub(crate) fn request_quit() {
    let w = window();
    w.set_close_behavior(WindowCloseBehaviour::WindowCloses);
    w.close();
}

/// Apply --start-minimized once during app_root mount.
pub(crate) fn apply_start_minimized(start_minimized: bool) {
    if start_minimized {
        window().set_visible(false);
    }
}
```

**Why no close-hook gate.** Dioxus 0.7.6 has no cancellable close-requested hook usable from outside the crate. `WindowCloseBehaviour::WindowHides` (set in `launch_gui`'s `Config` builder) instructs Dioxus to call `set_visible(false)` and consume close-requested unconditionally — exactly the desired X-click behavior, with no F3 code path required (`dioxus-desktop-0.7.6/src/app.rs:201-205`). `Config::with_custom_event_handler` runs *after* Dioxus's app-level handling, so it cannot pre-cancel close-requested even if F3 wanted to. The Quit path therefore inverts the question: instead of gating "should this close proceed?" before the fact, F3 flips the per-window close behavior to `WindowCloses` and then triggers close — Dioxus's own close-handling does the rest. `DesktopService::set_close_behavior(WindowCloseBehaviour)` is mutable post-launch (`desktop_context.rs:177-179`).

**Why `EventLoopProxy::send_event(UserWindowEvent::Shutdown)` is not used.** The proxy on `SharedContext` is `pub(crate)` in 0.7.6 (`desktop_context.rs:65`); user code cannot access it without forking the crate. The `set_close_behavior + close()` pattern achieves the same outcome (event-loop exit) using only public 0.7.6 surface.

**`window()` context safety.** `dioxus_desktop::window()` calls `dioxus_core::consume_context()` (`desktop_context.rs:34`), which panics outside a Dioxus scope. The listener task is spawned via `dioxus::prelude::spawn` from inside `app_root`'s `use_hook`, so it inherits `ScopeId::ROOT`'s context — every `window()` call from `show_window` / `request_quit` / `apply_start_minimized` resolves correctly. On Quit, the channel sender held by the custom-event-handler closure is dropped during runtime teardown, the listener exits cleanly via `rx.recv() == None`, and any in-flight call into `window()` completes before scope teardown.

**No unit tests on this module.** With `CloseGate` and `evaluate_close` deleted, there is no pure function to test in isolation; behavior is exercised by lifecycle scenarios S2 / S5 in §"Lifecycle scenarios end-to-end" and the corresponding acceptance bullets. `tray::action::TrayAction::from_event` keeps its unit tests (still pure).

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

**CSS** (sketch — `assets/shell/placeholder-shell.css`, NOT a design-system token; lives outside `assets/components/` because it's shell-scoped, not a primitive; deletes at F5):

```css
.if-placeholder-shell {
    display: grid;
    grid-template-areas:
        "top    top"
        "left   center"
        "status status";
    grid-template-columns: 240px 1fr;
    grid-template-rows: 40px 1fr 28px;
    height: 100vh;   /* reliable inside WebView2 — viewport tracks tao window inner size; 100dvh is unnecessary */
}
.if-placeholder-shell__top    { grid-area: top;    border-bottom: 1px solid var(--color-border); }
.if-placeholder-shell__left   { grid-area: left;   border-right:  1px solid var(--color-border); padding: var(--space-3); }
.if-placeholder-shell__center { grid-area: center; padding: var(--space-3); }
.if-placeholder-shell__status { grid-area: status; }   /* applied by StatusBarView via the StatusBar `class` prop */
```

**CSS load site.** `PlaceholderShell` includes its CSS via `asset!("/assets/shell/placeholder-shell.css")` mirroring the F2 component pattern (each F2 primitive's `.rs` file calls `asset!()` on its sibling `.css`). Both the `.rs` file and the `asset!()` invocation delete at F5.

**Token compatibility.** The CSS references `var(--color-border)` and `var(--space-3)` from F2's token table. Implementer should `cargo check` against the actual token names declared in `crates/inputforge-gui-dx/assets/tokens.css` and adjust if F2 ended up coining slightly different names (e.g. `--space-md` instead of `--space-3`).

**Disposable** is a load-bearing word: the entire `shell/placeholder.rs` and `assets/shell/placeholder-shell.css` are expected to be deleted by F5 once the IA redesign produces a real layout. Their token usage stays correct because they reference F2 tokens — no tokens are coined in F3.

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

**Why automatic activation.** WAI-ARIA Authoring Practices recommends *manual* activation by default (Space/Enter to activate after arrow-key focus) because manual avoids surprising side effects when activating a tab is expensive (async load, scroll reset, etc.). F3's tab-content swaps are synchronous and cheap (the panel is already mounted and the swap is just a render-time conditional), so automatic activation is appropriate and matches today's egui surface behavior — users don't need to learn a new interaction model. Space and Enter therefore become no-ops for active tabs (the arrow already activated); they remain handled to absorb the keystroke and prevent default scroll behavior.

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
        div { class: "{combined}",
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
- ARIA-neutral: the wrapper `<div>` has no `role` and no `aria-label`. `role="status"` is *deliberately not* applied at the primitive level — it's a [live region](https://www.w3.org/TR/wai-aria-1.2/#status), and a status bar full of badges that flip whenever engine state changes would generate disruptive AT announcements. Consumers add `role="status"` (or `aria-live`) on the *specific* element they want announced (typically a single Badge), or `aria-label` on the wrapper if a labeled landmark is desired.

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

    // Capture Memo values as locals before rsx! — Memo<T> does not implement
    // Display directly, and rsx! does not accept top-level `let` bindings
    // between elements inside a slot. Lifting both is the idiomatic 0.7 form.
    let status_value     = *status.read();
    let mode_str         = mode.read().clone();
    let profile_str      = profile.read().clone();
    let (connected, total) = *dev_count.read();

    rsx! {
        StatusBar {
            class: "if-placeholder-shell__status",
            start: rsx! {
                // role="status" + aria-live applied to a wrapping span so
                // assistive tech announces engine-state flips. Wrapping
                // (rather than passing role onto Badge) avoids depending on
                // Badge growing a generic `role` prop.
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

**Mode badge always renders.** The mode badge is shown unconditionally — when `MetaSnapshot::current_mode == "Default"` (the seed value, see `inputforge-core/src/state/mod.rs:68`), it renders as `Badge { variant: Neutral, "Default" }`. This is a deliberate choice: keeps `StatusBarView` trivial, surfaces the slot affordance from day one, and gives F11 (Modes) a concrete location to expand. F11 may introduce a hide-when-default toggle if it cares.

**Live-region semantics.** The engine-status `Badge` is wrapped in `<span role="status" aria-live="polite">` so assistive tech announces engine-state flips ("Running" → "Paused"). The mode badge, device-count text, and profile name are not announced — they're glanceable, not actionable, and would create noise. If F4 (toasts) supersedes status-bar announcements, this `role`/`aria-live` pair drops to ARIA-neutral.

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

**Clippy parity-only annotation.** The egui crate's `launch_gui` already carries `#[expect(clippy::needless_pass_by_value, reason = "signature parity with inputforge_gui::launch_gui")]` (or similar) on parameters consumed-but-unused. If adding `start_minimized: bool` trips a fresh `clippy::needless_pass_by_value` on the egui side, extend the existing `#[expect(...)]` reason to cover the new param rather than introducing a separate `#[allow]`.

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
        // Lock + take: take-once across the lifetime of the Dioxus runtime.
        // No contention possible — single producer (this hook), single
        // consumer (this hook). `unwrap()` cannot poison: no panic path
        // holds the lock across `.take()`.
        if let Some(rx) = params.listener_rx.lock().unwrap().take() {
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
    /// Carries the tray listener channel from `launch_gui` into `app_root`.
    /// Take-once on first scope mount; `Mutex::lock().unwrap().take()` empties
    /// the slot so subsequent mounts (e.g. `dx serve` hot-reload of
    /// `app_root`) become no-ops rather than double-spawning the listener.
    /// Production never re-mounts `app_root`; the take-once shape is purely
    /// belt-and-braces for the dev loop.
    pub listener_rx: std::sync::Arc<std::sync::Mutex<Option<tokio::sync::mpsc::Receiver<TrayAction>>>>,
}
```

`with_context` in `dioxus 0.7.6` requires `state: impl Any + Clone + Send + Sync + 'static` (`dioxus-0.7.6/src/launch.rs:256`). `Rc<Cell<>>` is `!Send + !Sync` and would fail to compile; `Arc<Mutex<Option<T>>>` satisfies the bound while preserving the same "move-once into a hook" semantics. This mirrors F1's `RawHandles` pattern — params installed pre-launch via `with_context`, picked up inside the runtime via `use_context`.

**`launch_gui` body sketch.** The construction sequence is:

```rust
pub fn launch_gui(
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    tray_menu_ids: (MenuId, MenuId, MenuId),
    settings: AppSettings,
    start_minimized: bool,
) -> anyhow::Result<()> {
    let (show, toggle, quit) = tray_menu_ids;
    let menu_ids = tray::action::TrayMenuIds { show, toggle, quit };

    let (tx, rx) = tokio::sync::mpsc::channel(tray::CHANNEL_CAPACITY);

    let raw = RawHandles { state, commands, settings };
    let params = LaunchParams {
        start_minimized,
        listener_rx: std::sync::Arc::new(std::sync::Mutex::new(Some(rx))),
    };

    let cfg = dioxus_desktop::Config::default()
        .with_close_behaviour(dioxus_desktop::WindowCloseBehaviour::WindowHides)
        .with_custom_event_handler(tray::make_event_handler(menu_ids, tx));
    // exit_on_last_window_close left at its default (true).

    LaunchBuilder::desktop()
        .with_cfg(cfg)
        .with_context(raw)
        .with_context(params)
        .launch(crate::app::app_root);

    Ok(())
}
```

Note inline destructure of `tray_menu_ids` instead of a `TrayMenuIds::from_tuple` helper — the helper would be one-line wrapping for no benefit.

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
3. Inside Dioxus crate: `(tx, rx) = mpsc::channel(8)` created. `Config::default().with_close_behaviour(WindowHides).with_custom_event_handler(make_event_handler(menu_ids, tx))`. `exit_on_last_window_close` left at default `true`. `LaunchParams { start_minimized: false, listener_rx: Arc<Mutex<Some(rx)>> }` installed via `with_context`.
4. `app_root` mounts: signals created, polling task spawned, listener task takes `rx` from the mutex and spawns. Renders `ThemeProvider { PlaceholderShell {} }`.
5. Window appears at 1280×800 with the placeholder shell visible. Status bar reflects engine state immediately (polling task already running).

**S2 — User clicks X to close window.**
1. Tao fires close-requested. Dioxus's `WindowHides` close-behavior calls `set_visible(false)` and consumes the event natively (`dioxus-desktop-0.7.6/src/app.rs:201–205`).
2. F3 has no close-handler code path here — Dioxus does the work.
3. Window vanishes from taskbar. Tray icon stays. Engine unaffected. Polling task keeps ticking (idle = no Signal writes via PartialEq gate). Webview count stays at 1 (window hidden ≠ destroyed), so `exit_on_last_window_close` does not fire.

**S3 — Tray Show GUI clicked while window hidden.**
1. muda fires `MenuEvent`. Dioxus's own `set_event_handler` (registered at `dioxus-desktop-0.7.6/src/app.rs:449`) forwards as `UserWindowEvent::MudaMenuEvent(ev)` to the tao event loop.
2. F3's `with_custom_event_handler` closure observes the user-event variant, calls `TrayAction::from_event`, gets `Some(Show)`, and `try_send`s it on the bounded channel.
3. Listener task wakes (`rx.recv().await`), calls `lifecycle::show_window()` → `set_visible(true)` + `set_focus()`.
4. Window reappears with current state already rendered (polling never stopped).

**S4 — Tray Activate/Deactivate clicked (window hidden or visible).**
1. Listener receives `TrayAction::Toggle`. Reads `ctx.state` for current `EngineStatus`, sends `EngineCommand::Activate`/`Deactivate` over `ctx.commands`.
2. Engine flips status. Polling picks it up next tick. If window visible: status-bar badge color changes within ~16ms. If hidden: change is observable on next Show.

**S5 — Tray Quit clicked.**
1. Listener receives `TrayAction::Quit`. Calls `lifecycle::request_quit()` (no `&ctx` argument; `quit_requested` flag is not read on the Dioxus path).
2. `window().set_close_behavior(WindowCloseBehaviour::WindowCloses)` flips this window's per-window close behavior atomically (`desktop_context.rs:177–179`).
3. `window().close()` triggers close-requested. Dioxus's close-handling now sees `WindowCloses`, destroys the window, and decrements the webview count to zero (`app.rs:209–219`).
4. With `exit_on_last_window_close = true` (the default — F3 leaves it alone), the event loop exits.
5. `LaunchBuilder::launch` returns. `launch_gui` returns `Ok(())`. `main()` runs `shutdown(cmd_tx, engine_handle)` → engine thread joins → `HidHide` unhide + `vJoy` release fire via `Drop`.

**Shutdown ordering between launch return and `shutdown()`.** When `LaunchBuilder::launch` returns at S5 step 5, the Dioxus runtime has already torn down. `spawn`-ed tasks (polling, listener) are auto-cancelled mid-`await`. The listener's `rx.recv().await` and the polling's `tick.tick().await` are cancellation-safe — neither holds a write lock across the await point. The custom-event-handler closure's `tx` half is dropped during teardown; if the listener was about to fire when teardown began, its next `recv()` returns `None` and the loop exits cleanly. Engine-thread teardown happens after, in `main()::shutdown(cmd_tx, engine_handle)` — engine never depends on Dioxus liveness.

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
crates/inputforge-gui-dx/src/tray/mod.rs                   # make_event_handler (closure for with_custom_event_handler), spawn_listener_task, dispatch_toggle, CHANNEL_CAPACITY const
crates/inputforge-gui-dx/src/tray/action.rs                # TrayAction, TrayMenuIds, from_event (+ unit tests)
crates/inputforge-gui-dx/src/lifecycle/mod.rs              # show_window, request_quit, apply_start_minimized — no unit tests (no pure-function gate; behavior covered by S2/S5 manual scenarios)
crates/inputforge-gui-dx/src/shell/mod.rs                  # pub(crate) re-exports
crates/inputforge-gui-dx/src/shell/placeholder.rs          # PlaceholderShell — disposable at F5
crates/inputforge-gui-dx/src/shell/status_bar_view.rs      # StatusBarView — signal-bound consumer of StatusBar primitive
crates/inputforge-gui-dx/src/components/tabs.rs            # Tabs primitive
crates/inputforge-gui-dx/src/components/status_bar.rs      # StatusBar primitive (ARIA-neutral wrapper)
crates/inputforge-gui-dx/assets/components/tabs.css        # Tabs CSS
crates/inputforge-gui-dx/assets/components/status-bar.css  # StatusBar CSS
crates/inputforge-gui-dx/assets/shell/placeholder-shell.css  # disposable at F5; shell-scoped, lives outside assets/components/
```

**Modified:**

```
crates/inputforge-gui-dx/src/lib.rs                       # launch_gui: + start_minimized; channel + Config builder + LaunchParams plumbing; new module decls (tray, lifecycle, shell)
crates/inputforge-gui-dx/src/app.rs                       # app_root takes LaunchParams via use_context, spawns listener_task with take-once Mutex, applies start_minimized, renders PlaceholderShell
crates/inputforge-gui-dx/src/context.rs                   # remove the F1 #[expect(dead_code, reason="used in later tasks (engine command dispatch)")] attribute on AppContext.commands (and any sibling #[expect] annotations whose dead_code claim becomes false once tray dispatch wires them)
crates/inputforge-gui-dx/src/components/mod.rs            # re-export Tabs and StatusBar
crates/inputforge-gui-dx/examples/bridge_demo.rs          # wrap in PlaceholderShell so dev loop renders the real shell
crates/inputforge-gui-dx/examples/component_gallery.rs    # add Tabs and StatusBar gallery sections
crates/inputforge-gui-dx/README.md                        # document tray bridge model (with_custom_event_handler observation), hide-to-tray lifecycle, new primitives
crates/inputforge-gui/src/lib.rs                          # launch_gui signature: + start_minimized: bool (ignored — parity-only param so main.rs's call site is identical under both feature flags; F1 already established the cfg-gated `use … launch_gui` selection at the call site; param deletes at F16)
crates/inputforge-app/src/main.rs                         # delete IS_GUI_DIOXUS sentinel, F1 guards; cfg-split startup branch; cfg-gate run_tray_loop, drain_stale_gui_events, launch_gui_blocking on gui-egui
```

**Reused (not modified) from F1/F2:**
- `crates/inputforge-gui-dx/src/bridge.rs` — polling task
- `crates/inputforge-gui-dx/src/theme/mod.rs` — `ThemeProvider`
- All F2 components except `mod.rs` (re-exports updated)
- `AppContext`, snapshots in `context.rs` — types unchanged; only the `#[expect(dead_code)]` annotation on `commands` is removed (see Modified)

---

## Acceptance criteria

- [ ] `cargo build` (default) and `cargo run` (default) produce today's egui behavior. Manual smoke: launch, X-click closes window and keeps tray alive, tray Show re-opens, tray Quit exits cleanly.
- [ ] `cargo build --no-default-features --features gui-dioxus` succeeds with all workspace lints passing.
- [ ] `cargo run --no-default-features --features gui-dioxus` opens a window, shows the placeholder shell with labeled regions and a working status bar.
- [ ] **Status bar reflects live engine state.** Engine status badge color matches `EngineStatus` (and is wrapped in `role="status" aria-live="polite"` so AT announces transitions); mode badge always renders (Neutral variant when value is `"Default"`); device count `c/t devices` updates as devices connect/disconnect; profile name shows when present (plain `<span>`, not clickable in F3).
- [ ] **X-click hides the window.** Tray icon stays. Window reappears via tray Show with all state intact (no re-init flicker).
- [ ] **Tray Toggle** flips engine status. Status-bar badge updates within ~16ms when window is visible; visible immediately on next Show when window is hidden.
- [ ] **Tray Quit** ends the process cleanly: `lifecycle::request_quit()` flips close behavior to `WindowCloses` and calls `window().close()`; Dioxus destroys the window, the event loop exits via `exit_on_last_window_close`; `launch_gui` returns; `shutdown()` runs, engine thread joins, `HidHide` unhide and `vJoy` release fire via `Drop`. Verified by absence of orphaned hidden-device entries after exit.
- [ ] **`--start-minimized`** never shows the window in foreground; tray Show reveals it. Verified under both `gui-egui` (today's behavior, unchanged) and `gui-dioxus`.
- [ ] **In-window tray Quit** behaves identically to out-of-window. Test both: open window → click tray Quit; close window via X → click tray Quit.
- [ ] **In-window tray Toggle** updates status-bar badge in real time (proves the listener task fires regardless of window visibility, fixing F1's latent in-window race).
- [ ] **F1 cleanup completed:** `IS_GUI_DIOXUS` const, both F1 cfg-guards in `main()` and `run_tray_loop` are deleted; `run_tray_loop`, `drain_stale_gui_events`, `launch_gui_blocking` are gated `#[cfg(feature = "gui-egui")]`.
- [ ] `dx serve --example bridge_demo` (F1 path, modified) renders the new placeholder shell with status bar reflecting seeded state. RSX hot-reload works.
- [ ] `dx serve --example component_gallery` (F2 path, modified) renders Tabs and StatusBar sections with all interaction states demonstrable manually. Keyboard nav for Tabs verified (Left/Right, Home/End, focus-roving via Tab key).
- [ ] **Tabs ARIA contract:** `role="tablist"` / `role="tab"` / `aria-selected` / `tabindex` 0|-1 verified via DevTools accessibility inspector.
- [ ] **StatusBar ARIA shape:** primitive wrapper has no `role`; `StatusBarView` adds `role="status" aria-live="polite"` only on the engine-status badge wrapper. Verified via DevTools.
- [ ] `cargo test -p inputforge-gui-dx` passes — unit tests on `TrayAction::from_event`, plus inherited F1 `from_state` tests. (`lifecycle/mod.rs` has no unit tests — no pure-function gate to test; behavior covered by S2/S5 manual scenarios.)

---

## Test strategy

- **Unit tests** in `crates/inputforge-gui-dx/src/tray/action.rs` under `#[cfg(test)] mod tests`: synthesize `MenuEvent` fixtures, assert `TrayAction::from_event` routing.
- **No unit tests in `lifecycle/mod.rs`.** With the close-hook gate removed (Dioxus owns close-requested handling natively), no pure function remains to test in isolation. Behavior is exercised by S2 / S5 manual scenarios and the corresponding acceptance bullets.
- **No new CI matrix entry.** F1's `--no-default-features --features gui-dioxus build` is sufficient.
- **Manual interaction pass** documented in this spec (lifecycle scenarios S1–S10). Each scenario has an explicit acceptance bullet above.
- **Process check (not a code-verifiable bullet):** `impeccable:frontend-design` should be invoked early in F3 implementation with brief scoped to "shell layout direction + tab-bar + status-bar visual treatment, scoped to placeholder regions." Its output must be committed before component CSS finalization.
- **UI rendering automation deferred** per parent-plan open question (Dioxus testing story).

---

## Risks

- **Dioxus 0.7.6 has no cancellable close-requested hook.** F3 does not need one: `WindowCloseBehaviour::WindowHides` (set at launch) makes Dioxus auto-hide on X-click; `WindowCloseBehaviour::WindowCloses` + default `exit_on_last_window_close = true` makes it exit cleanly when F3 flips behavior and calls `close()` on Quit. `set_close_behavior` is mutable post-launch (`DesktopService::set_close_behavior`, `desktop_context.rs:177-179`). If a future 0.7.x minor revision changes either the default `exit_on_last_window_close` value or the `set_close_behavior` mutability, F3's `request_quit` is the single point of churn. Verify against the pinned 0.7.x at impl start using `latest-packages`.
- **Dioxus owns muda's `set_event_handler` slot.** `dioxus-desktop 0.7.6` registers the handler itself and forwards every event as `UserWindowEvent::MudaMenuEvent` (`dioxus-desktop-0.7.6/src/app.rs:449`, `src/ipc.rs:6,12`). F3 observes via `Config::with_custom_event_handler` and never competes for the slot. The egui crate keeps using `MenuEvent::receiver()`; mutually-exclusive feature flags + F1's `compile_error!` guard prevent runtime coexistence. Structural rather than runtime risk.
- **EventLoopProxy is `pub(crate)` in 0.7.6.** F3 cannot directly send `UserWindowEvent::Shutdown` from user code. The `set_close_behavior + window().close()` Quit pathway uses only public 0.7.6 surface and achieves the same outcome (event-loop exit). If 0.7.x ever exposes the proxy publicly, `request_quit` could simplify to `proxy.send_event(Shutdown)` — drop-in change inside `lifecycle/mod.rs`.
- **Multiple-window risk.** The hide-to-tray model assumes one window. Today's GUI is single-window; F12/F13/F14 *might* introduce a second window (e.g., calibration as separate). Documented as a precondition for those features — they'll need to revisit `WindowCloseBehaviour` per-window and decide whether `request_quit` should close all windows or just the main one.
- **`set_visible` race on rapid tray-click bursts.** Spam-clicking Show/Hide could interleave. Tao serializes window-visibility calls on Windows so the worst case is brief flicker, not state corruption. Acceptable.
- **Egui crate's `start_minimized: bool` parameter.** Added to keep `main.rs`'s call site identical under both feature flags; the egui crate ignores it because today's main.rs already gates startup launch from `cli.start_minimized` itself. Deleted at F16 cleanup.
- **F1 signal-coherency open question stays open.** F3 doesn't add live subscribers, so the F12 precondition stands unchanged.
- **Placeholder shell visual rough edges.** The disposable `placeholder-shell.css` is intentionally simple. `impeccable:frontend-design` is invited to polish the shell-level chrome (gutters, borders, region affordances) but **NOT** to redesign IA — that's F5. Mitigation: brief frontend-design with explicit scope.

---

## Open questions (inherited, not decided here)

- **Testing story for the Dioxus GUI** — parent-plan open question; F3 adds one more pure-function unit-test surface (`TrayAction::from_event`) but commits to nothing on rendering automation.
- **Exact Dioxus and `dioxus-cli` versions** — pinned at implementation start via `latest-packages` against <https://crates.io>.
- **Live-signal coherency** (F1 deferral) — F3 doesn't touch this; F12 precondition unchanged.

---

## Next steps

1. Commit this spec to git.
2. Invoke `superpowers:writing-plans` to produce a step-by-step implementation plan with TDD-friendly checkpoints. The plan should sequence:
   - `impeccable:frontend-design` invocation early (after token verification, before component CSS finalization),
   - Tabs and StatusBar primitives + gallery sections (verification continuity with F2),
   - tray bridge (with_custom_event_handler + bounded channel + listener task) + `TrayAction::from_event` unit tests; lifecycle module (no unit tests),
   - placeholder shell + StatusBarView wiring,
   - `main.rs` Shape A divergence + F1-cleanup deletions,
   - end-to-end manual lifecycle pass (S1–S10) as the final acceptance gate.
