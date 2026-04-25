# F1 — Dioxus Crate Scaffold & State Bridge

**Status:** Design approved, ready for implementation planning
**Date:** 2026-04-24
**Parent plan:** [2026-04-24-egui-to-dioxus-rewrite-design.md](./2026-04-24-egui-to-dioxus-rewrite-design.md) — Foundation feature F1
**Scope:** Stand up `crates/inputforge-gui-dx` as a parallel Dioxus Desktop GUI crate, bridge the existing engine state into Dioxus via a three-signal live snapshot, and wire a feature-flag dispatch in `inputforge-app` — with egui remaining the default runtime behavior.

## Motivation

F1 is pure foundation: no user-visible UI beyond a readout proving the bridge works. Its value is de-risking every downstream feature. A clean state-flow pattern, a working hot-reload loop, and a zero-regression switchover on `main` are what make F2–F14 safely incremental.

The master plan's rationale for the overall rewrite (paradigm friction, styling, layout, LLM productivity) is not re-litigated here — see the parent doc.

## Confirmed design choices

Decisions made during brainstorming that shape this spec:

1. **F1 readout scope:** bare-minimum smoke test — engine status, device count, current mode, profile name, virtual-device count, warning count. No theme, no components, no layout skeleton. F3 owns any shell scaffolding.
2. **Dispatch mechanism:** parallel `launch_gui` functions with **identical signatures** in both GUI crates. `main.rs` has one call site; only a `use` line is `cfg`-gated. The Dioxus crate accepts `tray_menu_ids` in F1 but ignores them — F3 wires the real tray bridge.
3. **Signal granularity:** **three signals split by update frequency** established from day one — `meta`, `config`, `live`. All three are populated by the polling task at F1; only `meta` and `config` have subscribers at F1. The pattern is the load-bearing bit; `live` having no subscribers until F12 is fine.

## Non-goals (deferred to named later features)

- Visual styling, theme tokens, atomic components → **F2**
- Window chrome, tab bar, menu bar, layout regions → **F3**
- Tray event listener task (replaces egui's per-frame `MenuEvent::receiver().try_recv()`) → **F3**
- Real-time in-window tray dispatch under `--features gui-dioxus` → **F3** (the `muda::MenuEvent` channel is a global process singleton; at F1 events queue until window close and are drained by `main.rs::drain_stale_gui_events`).
- Hide-to-tray window-close behavior under `--features gui-dioxus` → **F3** (requires `WindowCloseBehaviour::WindowHides` + `Config::with_exits_when_last_window_closes(false)` + a tray→Dioxus `set_visible(true)` signaling channel; at F1 window close = app exit because `tao::EventLoop::run` is one-shot and `launch_gui_blocking` cannot be re-entered).
- Deletion of `inputforge-app/src/main.rs::drain_stale_gui_events` (obsolete once the listener task lands) → **F3**
- Toast queue, modal dialog primitive, dirty-state confirmation flow → **F4**
- IA / navigation redesign → **F5**
- Any device / mapping / mode / curve / deadzone / calibration / profile UI → **F6–F14**
- `AppSettings` mutation path (today: passed by value; likely becomes `Arc<Mutex<_>>` or a reactive handle) → **F14**
- UI rendering test strategy (egui_kittest analogue) → **F15 / F16** per parent-plan open question
- Light theme → out of scope for the whole rewrite until explicitly needed

## Review of egui-specific plumbing

Before settling on the design, we audited the app/engine side for egui-immediate-mode plumbing worth reworking now. Findings:

**Inside the egui GUI crate (not reworked here — F1 doesn't touch it):**

- `CachedState` + `refresh_cache()` + per-frame snapshot rebuild
- `ctx.request_repaint_after(Duration::from_millis(16))`
- `clear_color` matching theme base to prevent black flashes
- `process_tray_events` polling `MenuEvent::receiver().try_recv()` inside `update()`

All of these are replaced natively by the Dioxus crate (three signals + polling task + dedicated listener task in F3). The egui crate keeps them until F16 (cutover).

**Inside `inputforge-app` and `inputforge-core` (reviewed, kept):**

- `AppState::quit_requested` — framework-agnostic lifecycle signal. Works for Dioxus unchanged.
- `Arc<RwLock<AppState>>` + `mpsc::Sender<EngineCommand>` — the explicit engine contract per parent plan. No rework.
- `muda::MenuId` triple in `launch_gui` signature — identity tokens, not egui-specific. F3 revisits the tray bridge shape; F1 just preserves the signature.
- `AppSettings` passed by value — unrelated to egui paradigm; F14 revisits.

**Flagged for F3** (not F1):

- `inputforge-app/src/main.rs::drain_stale_gui_events` — exists to mop up events lost to egui's per-frame race. Once F3's listener task lands, this function should be deletable. Left alone for F1.

**Net: nothing in `inputforge-app` or `inputforge-core` needs rework in F1's scope.**

## Architecture

### Crate layout

New crate at `crates/inputforge-gui-dx`:

```
crates/inputforge-gui-dx/
├── Cargo.toml
├── README.md                       # dev workflow, dx serve commands
├── src/
│   ├── lib.rs                      # pub fn launch_gui(...)
│   ├── context.rs                  # AppContext, snapshots, from_state helpers
│   ├── bridge.rs                   # spawn_polling_task
│   └── app.rs                      # app_root + F1Readout
└── examples/
    └── bridge_demo.rs              # primary RSX dev-loop harness (seeded AppState, no engine/tray)
```

`from_state` unit tests live as a `#[cfg(test)] mod tests` block inside `src/context.rs` (not in `tests/`): the snapshot types are `pub(crate)`, so integration tests can't reach them without promoting them to `pub`. An in-crate test module keeps the API surface unchanged.

### Dependencies

`crates/inputforge-gui-dx/Cargo.toml`:

```toml
[package]
name = "inputforge-gui-dx"
version.workspace      = true
edition.workspace      = true
license.workspace      = true
rust-version.workspace = true
description.workspace  = true
repository.workspace   = true
readme.workspace       = true
keywords.workspace     = true
categories.workspace   = true

[dependencies]
inputforge-core = { workspace = true }
dioxus          = { workspace = true, features = ["desktop"] }
tokio           = { workspace = true, features = ["rt", "time", "sync"] }
muda            = { workspace = true }   # tray IDs accepted (stubbed until F3). Depended on directly (not via `tray-icon` re-export) for type-identity with `main.rs::launch_gui_blocking`'s `MenuId` arguments
parking_lot     = { workspace = true }
tracing         = { workspace = true }
serde           = { workspace = true }
anyhow          = { workspace = true }

[lints]
workspace = true
```

Workspace additions (root `Cargo.toml`):

```toml
# Add to [workspace.dependencies]:
dioxus = "<latest 0.7.x>"            # pin exact latest stable at implementation start via `latest-packages` skill
tokio  = { version = "1", features = ["rt", "time", "sync"] }

inputforge-gui-dx = { path = "crates/inputforge-gui-dx" }

# Add to [workspace] members:
members = [
  "crates/inputforge-core",
  "crates/inputforge-gui",
  "crates/inputforge-gui-dx",
  "crates/inputforge-app",
]
```

### Feature-flag dispatch in `inputforge-app`

`crates/inputforge-app/Cargo.toml` — feature section and optional deps:

```toml
[features]
default    = ["gui-egui"]
gui-egui   = ["dep:inputforge-gui"]
gui-dioxus = ["dep:inputforge-gui-dx"]

[dependencies]
inputforge-gui    = { workspace = true, optional = true }
inputforge-gui-dx = { workspace = true, optional = true }
# ...rest unchanged
```

`crates/inputforge-app/src/main.rs` — two compile-time guards, a `cfg`-gated import, an `IS_GUI_DIOXUS` runtime sentinel, and two narrow `cfg`-expressed branches in the existing lifecycle (≈ 20 new lines total; see §Lifecycle for the full sketch):

```rust
#[cfg(all(feature = "gui-egui", feature = "gui-dioxus"))]
compile_error!("features `gui-egui` and `gui-dioxus` are mutually exclusive");

#[cfg(not(any(feature = "gui-egui", feature = "gui-dioxus")))]
compile_error!("one of `gui-egui` or `gui-dioxus` must be enabled");

#[cfg(feature = "gui-egui")]
use inputforge_gui::launch_gui;
#[cfg(feature = "gui-dioxus")]
use inputforge_gui_dx::launch_gui;

// Sentinel used at both `launch_gui_blocking` return sites to gate the
// one-shot-event-loop behavior described in §Lifecycle below.
#[cfg(feature = "gui-egui")]
const IS_GUI_DIOXUS: bool = false;
#[cfg(feature = "gui-dioxus")]
const IS_GUI_DIOXUS: bool = true;
```

The first `launch_gui_blocking` call site (today's line 115, inside `if !cli.start_minimized`) is structurally identical; only the error-type variable flowing through `tracing::error!` changes and the `IS_GUI_DIOXUS` post-return guard is added. The **second** `launch_gui_blocking` call site (today's line 300, inside `run_tray_loop`'s `ShowGui` arm) needs a parallel guard — see §Lifecycle below.

### Lifecycle under `--features gui-dioxus`

**Why this subsection exists:** `tao::EventLoop::run` (the underlying Dioxus Desktop loop on Windows) is a one-shot-per-process API. Today's `main.rs` calls `launch_gui_blocking` at **two** sites: once on startup (main.rs line 115 today, when `!cli.start_minimized`) and once inside `run_tray_loop`'s `ShowGui` arm (main.rs line 300 today, fires on every tray **Show GUI** click). Under `gui-dioxus` the second call to `tao::EventLoop::run` panics. Both sites must be guarded.

**F1 resolution — scope-preserving:** under `--features gui-dioxus`, `launch_gui_blocking` can fire at most once per process; whichever call site fires first is the only window the user gets this session. When that window closes, the app runs its normal shutdown path and exits.

This preserves `shutdown(cmd_tx, engine_handle)` (today's main.rs line 137 — joins the engine thread, flushes the `HidHide` unhide logic and the `vJoy` release paths via engine `Drop` impls) on both the window-close and tray-Quit paths. Skipping `shutdown()` would leak the engine thread and skip device cleanup — **not acceptable** even under a known F1 regression.

Two `cfg`-expressed guards; both fall through to the existing `shutdown()` call at today's main.rs line 137:

**Guard 1 — startup path, after the first `launch_gui_blocking` returns (around today's main.rs line 128):**

```rust
if !cli.start_minimized {
    for action in launch_gui_blocking(&tray, &state, &cmd_tx, &settings) {
        match action { /* existing TrayAction handling — unchanged */ }
    }
    if IS_GUI_DIOXUS {
        // F1: tao EventLoop is one-shot. Skip `run_tray_loop` to avoid a
        // second `launch_gui_blocking` on tray Show-GUI. F3 restores the
        // full lifecycle with `WindowCloseBehaviour::WindowHides`.
        tracing::info!("Dioxus window closed; treating as app exit (hide-to-tray lands in F3).");
        quit_requested = true;
    }
}

if !quit_requested {
    run_tray_loop(&tray, &state, &cmd_tx, &settings);
}

// shutdown() runs next — unchanged at main.rs line 137. Reached on both paths.
shutdown(cmd_tx, engine_handle);
```

**Guard 2 — tray `ShowGui` path, after the second `launch_gui_blocking` returns (inside `run_tray_loop`, today's line 300–318):**

```rust
TrayAction::ShowGui => {
    let deferred = launch_gui_blocking(tray, state, cmd_tx, settings);
    tray.refresh_toggle_label();
    for deferred_action in deferred {
        match deferred_action { /* existing TrayAction handling — unchanged */ }
    }
    if IS_GUI_DIOXUS {
        // F1: tao EventLoop is one-shot. Return from the tray loop so
        // `main()`'s `shutdown()` path runs. F3 restores tray re-open via
        // `DesktopService::set_visible(true)` signaled from the tray listener.
        return;
    }
}
```

Both guards exit the post-GUI flow cleanly: `shutdown()` runs, the engine thread joins, `HidHide` unhide and `vJoy` release fire via engine `Drop`. The only behavior change under `gui-dioxus` at F1 is "no second GUI window this session" — not "leaked threads / unflushed devices."

`drain_stale_gui_events` is called *inside* `launch_gui_blocking` before it returns (via the egui path's on-close drain; the Dioxus path's equivalent will be added in the Dioxus `launch_gui` implementation per the same pattern). Any queued tray actions the user clicked while the window was open are surfaced through the returned `Vec<TrayAction>` and processed by `main()`'s action loop immediately before the cfg-gated guard fires.

**Behavior delta at F1 under `--features gui-dioxus`:**
- First `launch_gui_blocking` call opens the Dioxus window. Window close exits the app (today's egui: window close keeps tray alive for re-open).
- `--start-minimized` is supported — the first `launch_gui_blocking` happens inside `run_tray_loop` on the first **Show GUI**, works once, then the process exits when the window closes.
- Second and subsequent **Show GUI** tray clicks **never reach** `launch_gui_blocking` (guards fire first). In practice the user cannot see this — by the time they could click, `shutdown()` has completed and the tray icon is gone.
- In-window tray clicks (Quit, Activate) are accepted by `muda` but not polled Dioxus-side (F3 adds the poller); they sit in the global channel until `launch_gui`'s internal drain surfaces them on window close.

**F3 will restore today's full lifecycle** with three well-documented Dioxus 0.7 APIs:
- `dioxus_desktop::Config::with_close_behaviour(WindowCloseBehaviour::WindowHides)` — window close hides the window instead of destroying it.
- `Config::with_exits_when_last_window_closes(false)` — app stays alive when all windows are hidden.
- `DesktopService::set_visible(true)` via `use_window()` (or the module-level `dioxus_desktop::window()` outside hooks) — called from a tray-event listener task to re-show the hidden window on tray **Show GUI**.

At that point, under `gui-dioxus`, `launch_gui_blocking` becomes long-running (single call, lives until real quit); both F1 guards become effectively unreachable without code change. F1 deliberately does NOT ship these — wiring a tray→Dioxus channel is F3's defining scope.

**Build/run matrix after F1:**

| Command | Result |
|---|---|
| `cargo build` / `cargo run` | egui (default) |
| `cargo build --no-default-features --features gui-dioxus` | Dioxus |
| `cargo run  --no-default-features --features gui-dioxus` | Dioxus |
| `cargo build --features gui-dioxus` (default still on) | compile error (guard triggers) |
| `cargo build --no-default-features` | compile error (guard triggers) |

CI gains one matrix entry: `--no-default-features --features gui-dioxus build` to keep both paths green per the parent plan's merge rule.

### Return-type adaptation on the egui crate

The Dioxus crate's `launch_gui` returns `anyhow::Result<()>` (Dioxus Desktop's `LaunchBuilder::launch` is non-returning, so the wrapper is a formality). To keep both crates' signatures identical, the egui crate's `launch_gui` is also changed to return `anyhow::Result<()>`:

```rust
// crates/inputforge-gui/src/lib.rs — single-line body change
eframe::run_native(..., Box::new(...)).map_err(anyhow::Error::from)
```

Verified: `tracing::error!(%e, ...)` formats `anyhow::Error` cleanly via `Display`; `main.rs` is the only caller of `inputforge_gui::launch_gui` and does not depend on `eframe::Error` specifically.

### State bridge: `AppContext`, snapshots, polling task

**`context.rs` — handles, AppContext, and three snapshot structs:**

The split below is deliberate. Signals in Dioxus are registered with the reactive runtime when created; safely creating them requires being inside the Dioxus runtime's lifetime. `launch_gui` runs *before* `LaunchBuilder::launch`, so signal construction is delegated to the root component via `use_signal`. Raw handles (which need no runtime) are installed via `LaunchBuilder::with_context` pre-launch and picked up in `app_root`.

```rust
use std::sync::Arc;
use std::sync::mpsc;
use std::path::PathBuf;
use std::collections::{HashMap, HashSet};

use parking_lot::RwLock;
use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;
use inputforge_core::settings::AppSettings;
use inputforge_core::state::{AppState, DeviceState, EngineStatus};
use inputforge_core::types::{
    AxisPolarity, HatDirection, InputAddress, InputId, VJoyAxis, VirtualDeviceConfig,
};

/// Raw, signal-free handles installed via `LaunchBuilder::with_context`.
/// `Arc<AppSettings>` is a zero-cost read-only handle at F1; F14 will unwind
/// this wrapping when adding the mutation path.
#[derive(Clone, Debug)]
pub(crate) struct RawHandles {
    pub state:    Arc<RwLock<AppState>>,       // direct read access when needed
    pub commands: mpsc::Sender<EngineCommand>, // write path
    pub settings: Arc<AppSettings>,            // read-only in F1; F14 addresses mutation
}

/// Full per-window context: raw handles plus the three reactive signals.
/// Assembled inside `app_root` (signals must be created within the runtime).
/// `Signal<T>` in Dioxus 0.7 implements `Debug` on stable release lines, so the
/// derive is straightforward. If the pinned 0.7.x version chosen at implementation
/// start does not yet carry the impl (unlikely but unverified at spec time),
/// hand-write a `Debug` impl for `AppContext` that skips the three signal fields
/// and logs only `state` / `commands` / `settings` — lint tolerance is `warn`, not
/// `deny`, but the hand-impl is trivial.
#[derive(Clone, Debug)]
pub(crate) struct AppContext {
    pub state:    Arc<RwLock<AppState>>,
    pub commands: mpsc::Sender<EngineCommand>,
    pub settings: Arc<AppSettings>,
    pub meta:     Signal<MetaSnapshot>,
    pub config:   Signal<ConfigSnapshot>,
    pub live:     Signal<LiveSnapshot>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct MetaSnapshot {
    pub engine_status: EngineStatus,
    pub current_mode: String,
    pub profile_name: Option<String>,
    pub profile_path: Option<PathBuf>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ConfigSnapshot {
    pub devices: Vec<DeviceState>,
    pub virtual_devices: Vec<VirtualDeviceConfig>,
    pub mapped_inputs: HashSet<InputAddress>,
    pub mapping_names: HashMap<InputAddress, String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct LiveSnapshot {
    pub device_inputs: Vec<DeviceInputValues>,  // parallel to ConfigSnapshot::devices
    pub output_values: Vec<VjoyOutputValues>,   // parallel to ConfigSnapshot::virtual_devices
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct DeviceInputValues {
    pub axes:    Vec<(f64, AxisPolarity)>,
    pub buttons: Vec<bool>,
    pub hats:    Vec<HatDirection>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct VjoyOutputValues {
    pub axes:    Vec<(VJoyAxis, f64)>,
    pub buttons: Vec<bool>,
    pub hats:    Vec<HatDirection>,
}

impl MetaSnapshot {
    pub fn from_state(s: &AppState) -> Self {
        Self {
            engine_status: s.engine_status,
            current_mode:  s.current_mode.clone(),
            profile_name:  s.active_profile.as_ref().map(|p| p.name().to_owned()),
            profile_path:  s.profile_path.clone(),
            warnings:      s.warnings.clone(),
        }
    }
}

impl ConfigSnapshot {
    pub fn from_state(s: &AppState) -> Self {
        let mut mapped_inputs  = HashSet::new();
        let mut mapping_names  = HashMap::new();
        if let Some(profile) = &s.active_profile {
            for mapping in profile.mappings() {
                mapped_inputs.insert(mapping.input.clone());
                if let Some(name) = &mapping.name {
                    mapping_names.insert(mapping.input.clone(), name.clone());
                }
            }
        }
        Self {
            devices:         s.devices.clone(),
            virtual_devices: s.virtual_devices.clone(),
            mapped_inputs,
            mapping_names,
        }
    }
}

impl LiveSnapshot {
    /// Takes a pre-built `ConfigSnapshot` so device / virtual-device shape is read
    /// from a single coherent source — see "Signal coherency" note below.
    pub fn from_state(s: &AppState, cfg: &ConfigSnapshot) -> Self {
        let device_inputs: Vec<DeviceInputValues> = cfg.devices.iter().map(|device| {
            let did = &device.info.id;
            DeviceInputValues {
                axes: (0..device.info.axes).map(|i| {
                    let addr = InputAddress { device: did.clone(), input: InputId::Axis { index: i } };
                    let pol  = device.info.axis_polarities.get(usize::from(i)).copied().unwrap_or_default();
                    (s.input_cache.get_axis(&addr), pol)
                }).collect(),
                buttons: (0..device.info.buttons).map(|i| {
                    let addr = InputAddress { device: did.clone(), input: InputId::Button { index: i } };
                    s.input_cache.get_button(&addr)
                }).collect(),
                hats: (0..device.info.hats).map(|i| {
                    let addr = InputAddress { device: did.clone(), input: InputId::Hat { index: i } };
                    s.input_cache.get_hat(&addr)
                }).collect(),
            }
        }).collect();

        let output_values: Vec<VjoyOutputValues> = cfg.virtual_devices.iter().map(|v| {
            VjoyOutputValues {
                axes:    v.axes.iter().map(|&a| (a, s.output_cache.get_axis(v.device_id, a))).collect(),
                buttons: (1..=v.button_count).map(|i| s.output_cache.get_button(v.device_id, i)).collect(),
                hats:    (0..v.hat_count).map(|i| s.output_cache.get_hat(v.device_id, i)).collect(),
            }
        }).collect();

        Self { device_inputs, output_values }
    }
}
```

Shapes of `DeviceInputValues` / `VjoyOutputValues` mirror today's `DeviceInputSnapshot` / `VjoyOutputSnapshot` in the egui crate but live inside `inputforge-gui-dx` — no cross-crate leakage.

**Prerequisite core derive additions** (these snapshot structs won't compile as written without them):
- `crates/inputforge-core/src/state/status.rs` — add `Default` to `EngineStatus`'s derive list with `#[default] Stopped`. "Engine is Stopped before initialization" is a sensible semantic default and keeps the snapshot structs pure-derive (no hand-impl drift needed for `MetaSnapshot::default()`).
- `crates/inputforge-core/src/state/device.rs` — add `PartialEq, Eq` to `DeviceState`'s derive list. All transitively-owned fields (`DeviceInfo`, `DeviceId`, `AxisPolarity`, primitives) already derive `PartialEq` — one-line change, no cascade. Required for `ConfigSnapshot`'s `PartialEq` gate in the polling task.

Both core changes are additive derives; no behavior change, no impact on other crates.

**Excluded from the Dioxus extraction** (these exist in today's egui `refresh_cache` at `crates/inputforge-gui/src/app.rs:303–317`, and stay with the view, not the bridge):
- `toast_manager.push(...)` for new warnings → **F4** owns toasts.
- `selection.selected_device_idx` bounds-check when devices change → UI selection state stays inside the component, not in the snapshot bridge.

The `from_state` helpers are pure functions of `&AppState`: trivially unit-testable against a synthetic `AppState` fixture without a runtime, engine thread, or Dioxus runtime.

**Signal coherency between `config` and `live`.** `LiveSnapshot::output_values[i]` is positional-parallel to `ConfigSnapshot::virtual_devices[i]`; likewise `LiveSnapshot::device_inputs[i]` parallels `ConfigSnapshot::devices[i]`. `LiveSnapshot::from_state` reads this shape from the caller-supplied `&ConfigSnapshot`, so a single `(config, live) = from_state(&state)` call pair built in one `try_read()` window is internally coherent.

However, the three signals write **independently** in `spawn_polling_task` (see `bridge.rs` below): a subscriber reading `config` and `live` during the tick window where one has been updated and the other hasn't may observe a transient length mismatch. At F1 nothing subscribes to `live`, so this is dormant. **Called out as an F12 precondition**: pick one resolution before adding live subscribers —
- (a) make `config`+`live` updates atomic in the polling task (if `config` changed, force-re-`set` `live` even when `PartialEq`-equal), **or**
- (b) embed enough shape in `LiveSnapshot` to render without reading `config` (self-describing live data).

Also named under Risks.

**`bridge.rs` — polling task:**

```rust
use std::time::Duration;
use dioxus::prelude::*;

use crate::context::{AppContext, MetaSnapshot, ConfigSnapshot, LiveSnapshot};

pub(crate) fn spawn_polling_task(ctx: AppContext) {
    spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_millis(16));
        loop {
            tick.tick().await;

            // Non-blocking: if the engine currently holds the write lock, skip this tick.
            // One missed tick at 60Hz is imperceptible.
            let Some(guard) = ctx.state.try_read() else { continue };

            let meta   = MetaSnapshot::from_state(&guard);
            let config = ConfigSnapshot::from_state(&guard);
            let live   = LiveSnapshot::from_state(&guard, &config);
            drop(guard);

            // PartialEq-gated writes: idle state produces no wake-ups, even while ticking.
            if *ctx.meta.peek()   != meta   { ctx.meta.set(meta); }
            if *ctx.config.peek() != config { ctx.config.set(config); }
            if *ctx.live.peek()   != live   { ctx.live.set(live); }
        }
    });
}
```

Key properties:

- `try_read()` matches today's egui pattern — never blocks the UI runtime on engine writes.
- `.peek()` reads without subscribing, so the diff gate itself doesn't wake any component.
- Task spawned once on root mount via `use_hook`; tied to window lifetime; auto-cancelled when Dioxus tears down the runtime on window close.
- All three snapshots always rebuilt each tick; micro-optimisation of "skip rebuild if source fields unchanged" is explicitly deferred — not measurable at F1 scale.

### Root component & F1 readout

**`lib.rs` — launch_gui:**

```rust
use std::sync::Arc;
use std::sync::mpsc;

use parking_lot::RwLock;
use muda::MenuId;
use dioxus::prelude::*;
use dioxus::desktop::{Config, LogicalSize, WindowBuilder};

use inputforge_core::engine::EngineCommand;
use inputforge_core::settings::AppSettings;
use inputforge_core::state::AppState;

use crate::context::RawHandles;

mod app;
mod bridge;
mod context;

pub fn launch_gui(
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    tray_menu_ids: (MenuId, MenuId, MenuId),
    settings: AppSettings,
) -> anyhow::Result<()> {
    tracing::debug!(?tray_menu_ids, "tray wiring stubbed until F3");
    let _ = tray_menu_ids;  // F3: replaced by a dedicated listener task

    let handles = RawHandles {
        state,
        commands,
        settings: Arc::new(settings),
    };

    let window = WindowBuilder::new()
        .with_title("InputForge")
        .with_inner_size(LogicalSize::new(1280.0, 800.0))
        .with_min_inner_size(LogicalSize::new(800.0, 500.0));

    let cfg = Config::new().with_window(window);

    LaunchBuilder::desktop()
        .with_cfg(cfg)
        .with_context(handles)   // Signals are created inside `app_root`.
        .launch(app::app_root);

    Ok(())
}
```

`LaunchBuilder::launch` blocks the calling thread on the OS event loop (wry/tao underneath) — matches `eframe::run_native`'s blocking semantics. `launch_gui_blocking` in `main.rs` keeps working unchanged.

**`app.rs` — root component and F1 readout:**

```rust
use dioxus::prelude::*;
use crate::bridge::spawn_polling_task;
use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, RawHandles};

pub(crate) fn app_root() -> Element {
    // Pick up the signal-free handles installed by `launch_gui`.
    let raw = use_context::<RawHandles>();

    // Signals must be created inside the runtime — not in `launch_gui`.
    let meta   = use_signal(MetaSnapshot::default);
    let config = use_signal(ConfigSnapshot::default);
    let live   = use_signal(LiveSnapshot::default);

    // Assemble the full context and install it for descendants.
    let ctx = AppContext {
        state:    raw.state.clone(),
        commands: raw.commands.clone(),
        settings: raw.settings.clone(),
        meta, config, live,
    };
    use_context_provider(|| ctx.clone());

    // One-shot task spawn; `use_hook` guarantees single call per scope mount.
    use_hook(|| spawn_polling_task(ctx.clone()));

    rsx! { F1Readout {} }
}

#[component]
fn F1Readout() -> Element {
    let ctx = use_context::<AppContext>();

    // Each use_memo tracks only the projected slice; components using the memo
    // rerender only when the projection (via PartialEq) actually differs.
    let status    = use_memo(move || format!("{:?}", ctx.meta.read().engine_status));
    let mode      = use_memo(move || ctx.meta.read().current_mode.clone());
    let profile   = use_memo(move || ctx.meta.read().profile_name.clone()
                                        .unwrap_or_else(|| "<none>".into()));
    let devices   = use_memo(move || ctx.config.read().devices.len());
    let vdevices  = use_memo(move || ctx.config.read().virtual_devices.len());
    let warnings  = use_memo(move || ctx.meta.read().warnings.len());

    rsx! {
        main {
            style: "font-family: system-ui; padding: 24px; color: #ddd; \
                    background: #1A1A2E; min-height: 100vh;",
            h1 { "InputForge — Dioxus (F1 bridge smoke test)" }
            p { "Engine status: "     strong { "{status}" } }
            p { "Current mode: "      strong { "{mode}" } }
            p { "Active profile: "    strong { "{profile}" } }
            p { "Connected devices: " strong { "{devices}" } }
            p { "Virtual devices: "   strong { "{vdevices}" } }
            p { "Warnings: "          strong { "{warnings}" } }
            hr {}
            small { "Tray wiring: stubbed (F3). Theme: F2. Layout: F3." }
        }
    }
}
```

What this proves (= F1 acceptance, qualitatively):

- Window opens at correct size with correct title.
- `AppContext` is installed in Dioxus' runtime context and retrievable via `use_context`.
- All three signals tick at ~60Hz; this view subscribes to `meta` and `config`. `live` exists, is populated, has no subscribers yet — exercises the third tier of the pattern without needing live UI.
- `PartialEq`-gated writes mean idle state ⇒ no component rerenders even while the task keeps ticking.

The inline `style=` on `<main>` is deliberate throwaway. Just enough dark-background contrast so "the window opened and data is flowing" is obvious. No CSS file, no tokens, no reusable components — F2 owns the design system.

## Dev workflow: hot reload

**Scope of Dioxus hot reload:**

- Reloads: RSX, CSS, static assets.
- Does **not** reload Rust logic — component function / state / non-RSX changes still require full rebuild.
- For F1, this is enough; RSX iteration value compounds from F2 onward.

**Tooling:** `cargo install dioxus-cli` — pinned to a version compatible with the Dioxus crate version chosen for the workspace, confirmed at implementation start via the `latest-packages` skill. No `xtask`, no `justfile`, no `cargo-watch` wrapper — `dx` is the standard Dioxus CLI and satisfies the 100%-Rust constraint.

**Primary dev loop — `examples/bridge_demo.rs`** (documented in `crates/inputforge-gui-dx/README.md`):

`crates/inputforge-gui-dx/examples/bridge_demo.rs` is a standalone example that builds a mock `AppState` with seeded device / virtual-device / profile entries, wraps it in `Arc<RwLock<_>>`, builds a drop-channel `mpsc::Sender<EngineCommand>` whose receiver is leaked, and calls `launch_gui` directly. No engine thread, no tray, no profile I/O, no `HidHide` scan — zero side effects per hot-reload cycle, predictable seeded data.

```bash
cd crates/inputforge-gui-dx
dx serve --example bridge_demo --platform desktop
```

Expected: full build, window opens with F1 readout showing the seeded values, editing the RSX in `inputforge-gui-dx/src/app.rs` updates the window within ~1s without restarting the process.

Side benefit: `bridge_demo.rs` doubles as a stable design-system harness for F2+ (theme / atomic components iterate against predictable data, not against a running engine).

**Secondary — end-to-end smoke against the full app binary:**

```bash
cd crates/inputforge-app
dx serve --platform desktop --no-default-features --features gui-dioxus
```

This path exercises the real engine thread, real tray, real profile autoload, real HidHide warning scan. Useful as a full-integration smoke test when verifying the feature-flag wiring or a lifecycle change, **not** recommended as the daily RSX-iteration loop — each hot-reload cycle respawns the engine thread, re-registers the tray icon, re-runs HidHide detection, and re-reads any profile on disk. Use sparingly.

**Validation criterion** — the primary `bridge_demo.rs` path must demonstrably hot-reload an RSX edit within ~1s, documented in the new crate's README with the exact `dx` and `dioxus` versions used.

**Explicitly deferred:** no CI hot-reload check, no cross-platform hot-reload investigation (project target is Windows 10+), no IDE-integration notes.

## Files

**Created:**

```
crates/inputforge-gui-dx/Cargo.toml
crates/inputforge-gui-dx/README.md               # dev workflow, dx serve commands
crates/inputforge-gui-dx/src/lib.rs              # launch_gui
crates/inputforge-gui-dx/src/context.rs          # RawHandles, AppContext, 3 snapshots, from_state; `#[cfg(test)] mod tests` holds from_state unit tests in-crate (types are pub(crate))
crates/inputforge-gui-dx/src/bridge.rs           # spawn_polling_task
crates/inputforge-gui-dx/src/app.rs              # app_root + F1Readout
crates/inputforge-gui-dx/examples/bridge_demo.rs # primary RSX dev-loop harness (seeded AppState, no engine/tray, no side effects per hot-reload cycle)
```

**Modified:**

```
Cargo.toml                                # workspace members, new deps (dioxus, tokio), new internal crate
crates/inputforge-app/Cargo.toml          # gui-egui / gui-dioxus features, optional deps
crates/inputforge-app/src/main.rs         # compile_error! guards, cfg-gated use, IS_GUI_DIOXUS sentinel, two cfg-expressed post-launch guards (≈ 20 lines added — see §Lifecycle)
crates/inputforge-gui/src/lib.rs          # return type: eframe::Result → anyhow::Result (one .map_err line)
crates/inputforge-core/src/state/status.rs   # add `Default` (with `#[default] Stopped`) to `EngineStatus` derive
crates/inputforge-core/src/state/device.rs   # add `PartialEq, Eq` to `DeviceState` derive (all transitive fields already PartialEq)
```

## Acceptance criteria

- [ ] `cargo build` (default) succeeds; egui crate built; runtime behavior unchanged.
- [ ] `cargo run`  (default) launches egui GUI identically to today (tray, engine, profile loading, HidHide warning handling all intact).
- [ ] `cargo build --no-default-features --features gui-dioxus` succeeds with all workspace lints passing.
- [ ] `cargo run  --no-default-features --features gui-dioxus` opens a Dioxus window titled "InputForge" at 1280×800 (min 800×500) rendering the F1 readout.
- [ ] Readout values reflect engine state and update live: passing `--profile <path>` surfaces the profile name; plugging a joystick changes "Connected devices"; `--enable` moves engine status to `Running`.
- [ ] Feature-exclusivity guards fire: `cargo check --features gui-dioxus` (defaults still on) produces a clear `compile_error!` message.
- [ ] `dx serve` (the primary `bridge_demo.rs` path; see §Dev workflow) hot-reloads an RSX edit in the running Dioxus window within ~1s.
- [ ] **Profile autoload, `HidHide` warning surfacing:** unchanged under both feature flags. (Driven by `inputforge-app` and `inputforge-core`, independent of GUI runtime.)
- [ ] **Engine shutdown path on app exit:** unchanged under both feature flags — `shutdown(cmd_tx, engine_handle)` runs, the engine thread joins, `HidHide` unhide and `vJoy` release execute via engine `Drop`. Under `--features gui-dioxus`, the F1 lifecycle cfg-guards (§Lifecycle) reach this path via `quit_requested = true` / early `return` from `run_tray_loop`; they do not bypass `shutdown()`.
- [ ] **Tray lifecycle under `--features gui-egui`:** unchanged — window close keeps the tray alive; tray **Show GUI** re-opens the window.
- [ ] **Tray lifecycle under `--features gui-dioxus`:** window close exits the app (see §Lifecycle under `--features gui-dioxus`). Known F1-scoped regression vs. today's egui behavior; F3 restores hide-to-tray via `WindowCloseBehaviour::WindowHides` + tray→Dioxus signaling.
- [ ] **In-window tray actions under `--features gui-dioxus`:** Quit / Activate clicked while the Dioxus window is open are queued in `muda`'s global `MenuEvent` channel (no F1 Dioxus-side poller) and drained by `main.rs::drain_stale_gui_events` on window close, immediately before process exit. Usable but latent; F3 adds the real-time poller.

## Test strategy

- **Unit tests** in `crates/inputforge-gui-dx/src/context.rs` under `#[cfg(test)] mod tests`: construct `AppState` fixtures, call `MetaSnapshot::from_state`, `ConfigSnapshot::from_state`, `LiveSnapshot::from_state`, assert expected fields. Pure functions, trivial to test; pin the snapshot contracts every downstream feature will read from. (In-crate rather than `tests/` because snapshot types are `pub(crate)`.)
- **Build smoke:** CI matrix gains one entry (`--no-default-features --features gui-dioxus` build) to keep both paths green per the parent-plan merge rule.
- **UI rendering tests:** explicitly deferred. Parent-plan open question on Dioxus testing stands; F1 commits to nothing here.
- **Lock-behaviour regression:** optional — a small test verifying `spawn_polling_task` does not block when a write lock is held by a spawned helper thread. Drop if it becomes fiddly.

## Risks

- **Dioxus 0.7 API drift in snippets.** `LaunchBuilder` / `WindowBuilder` / `spawn` APIs cited here target the version pinned at implementation start. Minor adaptations expected; the pattern is stable.
- **`dx serve` against a feature-gated workspace binary.** Known-rough on older Dioxus versions. Mitigated by the primary `examples/bridge_demo.rs` dev-loop path (see §Dev workflow).
- **Return-type rework on the egui crate.** `eframe::Result → anyhow::Result`. Verified: `tracing::error!(%e, ...)` formats `anyhow::Error` via `Display`; `main.rs` is the only caller and doesn't depend on `eframe::Error` specifically.
- **Three-signal write-storm at scale.** If `from_state` becomes measurably slow (three clones + three `PartialEq` compares per tick at device counts >> current scale), the pattern may need per-signal "rebuild only if source changed" gating. Flag for F12 when `live` has real subscribers; not an F1 concern.
- **Initial window size difference vs. egui under DPI scaling.** F1 uses Dioxus' `LogicalSize::new(1280.0, 800.0)`; today's egui passes a raw `[1280.0, 800.0]` to `ViewportBuilder::with_inner_size`. On high-DPI displays the two may produce subtly different physical window sizes on first open. Accepted as cosmetic.
- **Signal-coherency across independent writes.** `LiveSnapshot::output_values[i]` is positional-parallel to `ConfigSnapshot::virtual_devices[i]` (same for device inputs). The three signals write independently, so a subscriber reading both during the tick window where one was updated and the other wasn't may observe a transient length mismatch. At F1 there are no `live` subscribers, so this is dormant — **called out as an F12 precondition**: either make config/live updates atomic (force-re-set `live` when `config` changes), or make `LiveSnapshot` self-describing (embed enough shape to render without reading `config`). Pick one in F12 before adding live subscribers.
- **`tao::EventLoop::run` is one-shot per process.** Dioxus Desktop wraps `wry`+`tao`; `tao::EventLoop::run` does not support being called twice on Windows (documented tao limitation; winit parallels). Today's `crates/inputforge-app/src/main.rs` calls `launch_gui_blocking` at **two** sites: startup (line 115 today, inside `if !cli.start_minimized`) and inside `run_tray_loop`'s `ShowGui` arm (line 300 today). Under `--features gui-dioxus`, a second call to either site panics. F1 handles both via cfg-expressed guards using an `IS_GUI_DIOXUS` sentinel (see §Lifecycle under `--features gui-dioxus`) — window close ⇒ `quit_requested = true` / early `return` from `run_tray_loop` ⇒ existing `shutdown()` path runs cleanly on both feature flags. **F3 restores hide-to-tray** via `WindowCloseBehaviour::WindowHides` + `Config::with_exits_when_last_window_closes(false)` + a tray→Dioxus `set_visible(true)` signaling channel. All three are stable Dioxus 0.7 APIs; the design is de-risked.
- **Dead-branch lint noise from the `IS_GUI_DIOXUS` sentinel.** Using a module-level `const IS_GUI_DIOXUS: bool` makes Guards 1 and 2 statically unreachable under one feature. Rust's own dead-code lint is silent here (constant-folded branches are not flagged), but `clippy::if_same_then_else` / `clippy::needless_if` may warn depending on workspace lint profile. Implementer notes: if clippy complains, either (a) replace the `if IS_GUI_DIOXUS { … }` bodies with direct `#[cfg(feature = "gui-dioxus")] { … }` blocks at the guard sites (slightly noisier but clippy-clean), or (b) targeted `#[allow(clippy::…)]` on the guards. Not an architectural concern.

## Open questions (inherited, not decided here)

- Testing story for the Dioxus GUI — parent-plan open question. F1 does not commit to Playwright / Dioxus-native renderer testing / anything else.
- Exact Dioxus and `dioxus-cli` versions — pinned at implementation start via the `latest-packages` skill against <https://crates.io>.
