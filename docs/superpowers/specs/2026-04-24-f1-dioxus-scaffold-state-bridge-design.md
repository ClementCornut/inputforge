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
├── examples/
│   └── bridge_demo.rs              # fallback dev harness (only if dx serve against full app needs it)
└── tests/
    └── snapshot.rs                 # from_state unit tests
```

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
muda            = { workspace = true }   # tray IDs accepted (stubbed until F3)
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

`crates/inputforge-app/src/main.rs` — two compile-time guards and a `cfg`-gated import (≈ 6 new lines):

```rust
#[cfg(all(feature = "gui-egui", feature = "gui-dioxus"))]
compile_error!("features `gui-egui` and `gui-dioxus` are mutually exclusive");

#[cfg(not(any(feature = "gui-egui", feature = "gui-dioxus")))]
compile_error!("one of `gui-egui` or `gui-dioxus` must be enabled");

#[cfg(feature = "gui-egui")]
use inputforge_gui::launch_gui;
#[cfg(feature = "gui-dioxus")]
use inputforge_gui_dx::launch_gui;
```

The existing `launch_gui_blocking` call site stays structurally identical — only the error-type variable flowing through `tracing::error!` changes.

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
    AxisPolarity, HatDirection, InputAddress, VJoyAxis, VirtualDeviceConfig,
};

/// Raw, signal-free handles installed via `LaunchBuilder::with_context`.
#[derive(Clone)]
pub(crate) struct RawHandles {
    pub state:    Arc<RwLock<AppState>>,       // direct read access when needed
    pub commands: mpsc::Sender<EngineCommand>, // write path
    pub settings: Arc<AppSettings>,            // read-only in F1; F14 addresses mutation
}

/// Full per-window context: raw handles plus the three reactive signals.
/// Assembled inside `app_root` (signals must be created within the runtime).
#[derive(Clone)]
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

impl MetaSnapshot   { pub fn from_state(s: &AppState) -> Self { /* field copy */ } }
impl ConfigSnapshot { pub fn from_state(s: &AppState) -> Self { /* field copy + mapping summary */ } }
impl LiveSnapshot   { pub fn from_state(s: &AppState, cfg: &ConfigSnapshot) -> Self { /* input_cache + output_cache reads, parallel to cfg */ } }
```

Shapes of `DeviceInputValues` / `VjoyOutputValues` mirror today's `DeviceInputSnapshot` / `VjoyOutputSnapshot` in the egui crate but live inside `inputforge-gui-dx` — no cross-crate leakage.

The field-copy bodies of the `from_state` helpers follow the same extraction logic as today's `refresh_cache`, `snapshot_device_inputs`, and `snapshot_vjoy_outputs` in `crates/inputforge-gui/src/app.rs`. They are pure functions, fully unit-testable against an `AppState` fixture.

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

**Primary validation command** (documented in `crates/inputforge-gui-dx/README.md`):

```bash
cd crates/inputforge-app
dx serve --platform desktop --no-default-features --features gui-dioxus
```

Expected: full build, window opens with F1 readout, editing the RSX in `inputforge-gui-dx/src/app.rs` updates the window within ~1s without restarting the process.

**Fallback** — if `dx serve` has issues running against a feature-gated workspace binary on the author's Dioxus version:

- Add `crates/inputforge-gui-dx/examples/bridge_demo.rs` — standalone example constructing a mock `AppState` with seeded device entries and calling `launch_gui` directly (no engine thread, no tray).
- Run: `cd crates/inputforge-gui-dx && dx serve --example bridge_demo --platform desktop`.
- Exercises RSX hot reload in isolation with predictable data.
- Side benefit: the example doubles as a stable design-system harness for F2+.

**Validation criterion** — one of the two paths must demonstrably hot-reload an RSX edit within ~1s, documented in the new crate's README with the exact `dx` and `dioxus` versions used.

**Explicitly deferred:** no CI hot-reload check, no cross-platform hot-reload investigation (project target is Windows 10+), no IDE-integration notes.

## Files

**Created:**

```
crates/inputforge-gui-dx/Cargo.toml
crates/inputforge-gui-dx/README.md               # dev workflow, dx serve commands
crates/inputforge-gui-dx/src/lib.rs              # launch_gui
crates/inputforge-gui-dx/src/context.rs          # RawHandles, AppContext, 3 snapshots, from_state
crates/inputforge-gui-dx/src/bridge.rs           # spawn_polling_task
crates/inputforge-gui-dx/src/app.rs              # app_root + F1Readout
crates/inputforge-gui-dx/tests/snapshot.rs       # from_state unit tests
crates/inputforge-gui-dx/examples/bridge_demo.rs # fallback dev harness — only if dx serve path needs it
```

**Modified:**

```
Cargo.toml                                # workspace members, new deps (dioxus, tokio), new internal crate
crates/inputforge-app/Cargo.toml          # gui-egui / gui-dioxus features, optional deps
crates/inputforge-app/src/main.rs         # compile_error! guards + cfg-gated use (≈ 6 lines added)
crates/inputforge-gui/src/lib.rs          # return type: eframe::Result → anyhow::Result (one .map_err line)
```

## Acceptance criteria

- [ ] `cargo build` (default) succeeds; egui crate built; runtime behavior unchanged.
- [ ] `cargo run`  (default) launches egui GUI identically to today (tray, engine, profile loading, HidHide warning handling all intact).
- [ ] `cargo build --no-default-features --features gui-dioxus` succeeds with all workspace lints passing.
- [ ] `cargo run  --no-default-features --features gui-dioxus` opens a Dioxus window titled "InputForge" at 1280×800 (min 800×500) rendering the F1 readout.
- [ ] Readout values reflect engine state and update live: passing `--profile <path>` surfaces the profile name; plugging a joystick changes "Connected devices"; `--enable` moves engine status to `Running`.
- [ ] Feature-exclusivity guards fire: `cargo check --features gui-dioxus` (defaults still on) produces a clear `compile_error!` message.
- [ ] `dx serve` (one of the two paths above) hot-reloads an RSX edit in the running Dioxus window within ~1s.
- [ ] Tray icon behavior, profile autoload, engine shutdown, `HidHide` warning handling are unchanged under both feature flags.

## Test strategy

- **Unit tests** in `crates/inputforge-gui-dx/tests/snapshot.rs`: construct `AppState` fixtures, call `MetaSnapshot::from_state`, `ConfigSnapshot::from_state`, `LiveSnapshot::from_state`, assert expected fields. Pure functions, trivial to test; pin the snapshot contracts every downstream feature will read from.
- **Build smoke:** CI matrix gains one entry (`--no-default-features --features gui-dioxus` build) to keep both paths green per the parent-plan merge rule.
- **UI rendering tests:** explicitly deferred. Parent-plan open question on Dioxus testing stands; F1 commits to nothing here.
- **Lock-behaviour regression:** optional — a small test verifying `spawn_polling_task` does not block when a write lock is held by a spawned helper thread. Drop if it becomes fiddly.

## Risks

- **Dioxus 0.7 API drift in snippets.** `LaunchBuilder` / `WindowBuilder` / `spawn` APIs cited here target the version pinned at implementation start. Minor adaptations expected; the pattern is stable.
- **`dx serve` against a feature-gated workspace binary.** Known-rough on older Dioxus versions. Mitigated by the example-target fallback documented above.
- **Return-type rework on the egui crate.** `eframe::Result → anyhow::Result`. Verified: `tracing::error!(%e, ...)` formats `anyhow::Error` via `Display`; `main.rs` is the only caller and doesn't depend on `eframe::Error` specifically.
- **Three-signal write-storm at scale.** If `from_state` becomes measurably slow (three clones + three `PartialEq` compares per tick at device counts >> current scale), the pattern may need per-signal "rebuild only if source changed" gating. Flag for F12 when `live` has real subscribers; not an F1 concern.

## Open questions (inherited, not decided here)

- Testing story for the Dioxus GUI — parent-plan open question. F1 does not commit to Playwright / Dioxus-native renderer testing / anything else.
- Exact Dioxus and `dioxus-cli` versions — pinned at implementation start via the `latest-packages` skill against <https://crates.io>.
