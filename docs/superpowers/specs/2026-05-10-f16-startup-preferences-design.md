# F16, Startup Preferences (launch at startup, start minimized to tray): Design Spec

**Status:** Design approved, ready for implementation plan
**Date:** 2026-05-10
**Parent specs:**
- [`2026-04-26-f3-app-shell-tray-bridge-design.md`](./2026-04-26-f3-app-shell-tray-bridge-design.md), tray + window-lifecycle plumbing this builds on
- [`2026-04-28-f6-snapshot-preferences-core-design.md`](./2026-04-28-f6-snapshot-preferences-core-design.md), `AppSettings` schema and persistence model
- [`2026-05-09-f15-settings-ui-design.md`](./2026-05-09-f15-settings-ui-design.md), settings panel surface this section plugs into

**Predecessors:** F1 (state bridge), F3 (tray bridge, `WindowCloseBehaviour::WindowHides`, lifecycle helpers), F6 (`AppSettings`, settings.toml at `%APPDATA%/inputforge/settings.toml`), F15 (panel surface, `SettingsSection` + `SettingsFieldRow` primitives, polled `SettingsSnapshot`).

---

## Context

InputForge is a tray-resident input remapper. Users expect it to be running whenever they game, the same way Discord, qBittorrent, OBS, and reWASD are. Today the app supports a `--start-minimized` CLI flag and a working tray, but lacks two preferences mature peers ship: launch at startup, and start in the tray on every launch (not just CLI-flagged ones).

This spec adds those two preferences. Linux is the second supported platform target on the project roadmap, so the implementation must work on both Windows and Linux from day one of the abstraction. Today the binary only ships on Windows; the Linux concrete impl is in scope for the spec but not on the build matrix until the platform itself is.

### Existing scaffolding (audit before design)

Verified against current source on `main` (commit `e0ca46f`):

- `crates/inputforge-app/src/cli.rs:24` defines `Cli::start_minimized: bool` (clap flag `--start-minimized`).
- `crates/inputforge-app/src/main.rs:110` plumbs `cli.start_minimized` into the `launch_gui(...)` call (call site begins at line 105).
- `crates/inputforge-gui-dx/src/lib.rs:46` declares `start_minimized: bool` on `LaunchParams`; the `launch_gui` argument feeding it is at line 76 and the literal initialiser at line 83.
- `crates/inputforge-gui-dx/src/lifecycle/mod.rs:48` `apply_start_minimized` calls `window().set_visible(false)` (so the existing flag means "hidden in tray", not "iconified to taskbar"; see Choice 1).
- `crates/inputforge-app/src/tray.rs:1-99` defines `AppTray` with Show, Activate, Quit menu items; tray icon is always created at boot.
- `crates/inputforge-gui-dx/src/lib.rs:103` configures `WindowCloseBehaviour::WindowHides` so the X button hides to tray.
- `crates/inputforge-core/src/settings.rs:26-45` defines `AppSettings` with `last_profile`, `snapshot`, `device_aliases`, `device_registry`. F6 already proved the `#[serde(default)]` + TOML load/save path; this spec adds two fields to that same struct.
- `crates/inputforge-gui-dx/src/frame/settings_panel/mod.rs:1-181` renders only `SnapshotsSection` today; `panel_renders_field_rows_without_heading` (line 67) asserts no `<h2>` or `<h3>` is emitted by the panel itself.

The persisted-setting flow does not yet exist. There is no autostart code anywhere in the workspace (verified via `search_text` for `auto-launch`, `HKCU`, `autostart`).

### What the design adds

1. A new `StartupSettings` struct nested under `AppSettings` (carrying two booleans), mirrored on `AppState` as `state.startup`.
2. A new crate `inputforge-autostart` with a trait abstraction over the OS write.
3. Two new struct-style `EngineCommand` variants that own the OS write atomically with the settings persistence.
4. A new `StartupSection` in the settings panel above `SnapshotsSection`.
5. A minimal `main.rs` change (one helper plus one call-site update) to OR the CLI flag with the persisted setting.

---

## Confirmed design choices

The decisions below were each surfaced and approved during brainstorming.

### Behavior model

**1. "Start minimized" semantics: hidden in tray.** Today's `apply_start_minimized` sets the window invisible, not iconified. We keep that semantic and label it accordingly. There is no taskbar entry; the tray icon is the only visible affordance until the user clicks Show. Rationale: matches reWASD's "background agent" framing, OBS's "minimize to system tray" wording, and qBittorrent #8644's resolution; tray-resident input remappers do not need a taskbar entry on launch.

**2. Two independent toggles.** "Launch InputForge at startup" and "Start minimized to tray" are independent settings. Either may be on alone. When auto-launch is on, the registered argv adds `--start-minimized` if and only if the start-minimized toggle is on. Rationale: dominant pattern across Discord, qBittorrent, OBS, Slack, reWASD; the gated and tri-state alternatives both break the manual-launch tray flow.

**3. CLI override semantics.** The `--start-minimized` flag is preserved unchanged. The effective value at boot is `cli.start_minimized || settings.startup.start_minimized_to_tray`. There is no `--no-start-minimized` flag; the OR semantic is sufficient because (a) the auto-launch path passes the flag explicitly when both toggles are on, (b) a manual launcher who wants normal start with the setting on is an unsupported edge case (peer apps don't expose this either).

### Data model

**4. Nested `StartupSettings` on `AppSettings`.** In `crates/inputforge-core/src/settings.rs`, alongside the existing `SnapshotConfig` import:

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartupSettings {
    #[serde(default)]
    pub launch_at_startup: bool,
    #[serde(default)]
    pub start_minimized_to_tray: bool,
}
```

`AppSettings` gains:

```rust
#[serde(default)]
pub startup: StartupSettings,
```

Both inner fields default to `false`; the outer `#[serde(default)]` lets pre-F16 settings.toml files load with no migration (mirrors the F6 `[snapshot]` precedent at `settings.rs:37-38`). Persisted on disk as a `[startup]` sub-table; users can hand-edit if they want. Field names match user-facing labels (avoid mixing the `auto-launch` crate name into our domain term; avoid the ambiguity that bit qBittorrent's `start_minimized` field).

The grouping mirrors the `SnapshotConfig` precedent so the engine-side mirror in `AppState` can reuse the same struct, populated by a single `clone_from`, exactly like `state.snapshot_config` is maintained today (verified at `engine/mod.rs:133`, `engine/run.rs:552`, and `engine/run.rs:565`).

### Architecture

**5. New crate `inputforge-autostart`.** The autostart concern writes OS state outside the app process (HKCU registry on Windows, `~/.config/autostart` on Linux) and pulls in `auto-launch` plus its platform-specific transitive deps. A separate crate keeps these dependencies and the OS-write surface out of `inputforge-core`'s dep tree, isolates platform impls behind a thin trait, and makes the autostart concern testable and replaceable independently of the input-engine core. (The `device/` module inside `inputforge-core` is a sibling module rather than a sibling crate, so that precedent does not apply.) Layout:

```
crates/inputforge-autostart/
├── Cargo.toml      # auto-launch = "0.6", thiserror, tracing
├── src/
│   ├── lib.rs      # pub trait AutostartManager + new_for_current_platform factory
│   ├── windows.rs  # #[cfg(target_os = "windows")] HKCU\...\Run
│   ├── linux.rs    # #[cfg(target_os = "linux")]   ~/.config/autostart/*.desktop
│   ├── noop.rs     # NoOpAutostart fallback when current_exe() fails
│   ├── mock.rs     # gated behind `mock` feature; used by inputforge-core tests
│   └── error.rs    # AutostartError variants
```

The trait is the seam:

```rust
pub trait AutostartManager {
    fn is_enabled(&self) -> Result<bool, AutostartError>;
    fn set_enabled(&mut self, enabled: bool, args: &[&str]) -> Result<(), AutostartError>;
}
```

The trait carries no `Send`/`Sync` bound. The engine itself is `!Send` (per `engine/mod.rs:90-93`, because `InputSource` and `DeviceHider` are `!Send` for SDL3 same-thread reasons), and the engine is the sole owner of the manager, so adding `Send` to the trait would not unlock anything. Args are passed at call time so the engine decides whether to include `--start-minimized`; the autostart impl is dumb about start-minimized.

Concrete impls wrap `auto_launch::AutoLaunch`, built via `AutoLaunchBuilder` with `WindowsEnableMode::CurrentUser` on Windows (avoids UAC; auto-launch's default fallback chain tries HKLM first then HKCU) and the default `XDG_CONFIG_HOME/autostart/InputForge.desktop` path on Linux. On `set_enabled(true, args)` the impl rebuilds the `AutoLaunch` with current args via `AutoLaunchBuilder` and calls `enable()`. On `set_enabled(false, _)` it calls `disable()` and ignores args. App name is `"InputForge"`.

Crate provenance: `auto-launch = "0.6"` (MIT, last release 2026-01-10, ~444k dl/month). It is also the upstream crate behind `tauri-plugin-autostart` (currently pinned at 0.5; we adopt 0.6 for the explicit `WindowsEnableMode` API).

`app_path` contract: the absolute executable path is resolved once in each concrete impl's constructor from `std::env::current_exe()`. On Err, the constructor returns `AutostartError::NotSupported`. The factory `new_for_current_platform` falls back to `NoOpAutostart`, which reports `is_enabled() == Ok(false)` and rejects `set_enabled` with `NotSupported`, so the engine and UI degrade gracefully (the toggle stays off, dispatching it pushes a warning to `state.warnings`).

`Engine::new` (currently 8 args, gated by `#[allow(clippy::too_many_arguments)]` at `engine/mod.rs:95-99`) gains a 9th argument `autostart: Box<dyn AutostartManager>`. The existing allow continues to apply. The test harness in `engine/tests.rs` passes `Box::new(MockAutostart::new())` in every constructor call; the harness will need a sweep update.

**6. Engine owns the OS write.** Two new `EngineCommand` variants in `crates/inputforge-core/src/engine/command.rs`, in struct style to match `SetSnapshotConfig { config }` and other existing setters:

```rust
SetAutostart { enabled: bool },
SetStartMinimizedToTray { enabled: bool },
```

The engine handles each:

`SetStartMinimizedToTray { enabled }`:
1. `self.settings.startup.start_minimized_to_tray = enabled`.
2. Mirror into `AppState`: `self.state.write().startup = self.settings.startup.clone()`.
3. Persist `settings.toml`. On save failure, roll back the in-memory settings and the `AppState` mirror, push `format!("Could not save settings: {e}")` to `state.warnings`, return. Matches `SetSnapshotConfig` at `engine/run.rs:555-580`.
4. If `self.settings.startup.launch_at_startup`, re-register the autostart entry with the new args. Best-effort: on Err, push `"Saved, but could not update the auto-launch arguments. Restart of InputForge may use the previous setting."` to `state.warnings`. Do NOT revert the persisted toggle, the auto-launch argv is a downstream consequence and the next reconcile (or the next manual autostart toggle) will fix it.

`SetAutostart { enabled }`:
1. `args = if self.settings.startup.start_minimized_to_tray { &["--start-minimized"] } else { &[] }`.
2. `self.autostart.set_enabled(enabled, args)`.
3. On `Ok(())`: write `self.settings.startup.launch_at_startup = enabled`, mirror into `AppState`, persist `settings.toml`. On settings save failure, follow the same rollback shape as `SetSnapshotConfig` (revert in-memory + mirror, push warning).
4. On `Err(e)` from `set_enabled`: log via `tracing::warn`, push `"Could not change launch-at-startup setting."` to `state.warnings`. Do NOT mutate `self.settings`. The mirror chain (engine writes `AppState`, GUI polls `SettingsSnapshot`) means the UI switch reverts visually on the next poll because `AppState` was never updated.

All failure feedback flows via `state.warnings: Vec<String>` (`state/mod.rs:119`), bridged to `ToastLevel::Warning` by the existing warnings bridge (`toast/warnings_bridge.rs:37`). F16 does not introduce a new toast channel; matches the `SetSnapshotConfig` save-failure precedent at `engine/run.rs:578`.

Rationale for the (OS, then settings) sequence on `SetAutostart`: atomicity. A crash between an OS write and a settings write would leave the registry value and the TOML disagreeing. Engine is single-threaded for this state, so the (OS, then settings) sequence is linearizable.

**7. OS-wins reconciliation on engine startup.** In `Engine::new` (or first `run()` iteration), before processing the first command:

1. Call `autostart.is_enabled()`.
2. On `Ok(actual)` with `actual != settings.startup.launch_at_startup`: set the field to `actual`, mirror into `AppState`, persist `settings.toml`, log at `tracing::info`.
3. On `Err(e)`: log via `tracing::warn`, leave settings as-is, push nothing to `state.warnings`.
4. If after step 2 `settings.startup.launch_at_startup == true` (whether unchanged or newly synced from the OS), call `self.autostart.set_enabled(true, args_from_setting)` once to converge the registered argv with the persisted `start_minimized_to_tray`. This is the recovery path for the case where `SetStartMinimizedToTray` succeeded the persisted write but the re-register failed; without this step the stale argv would persist indefinitely because step 1 only checks the boolean. Best-effort: on Err, push `"Could not refresh auto-launch arguments at startup."` to `state.warnings`. Do not block engine init.

Rationale: external changes (Task Manager, gnome-tweaks, manual registry edit) should be authoritative for the on/off state. The user wants the OS state to be the source of truth for OS-level settings. The argv re-push is unconditional (when enabled) because cheap idempotence at one extra OS write per launch is preferable to chasing a divergent argv across an unbounded number of launches. Polling beyond startup is rejected (waste, complexity); a single startup sync is sufficient.

### UI

**8. New `StartupSection` above `SnapshotsSection`** in `crates/inputforge-gui-dx/src/frame/settings_panel/startup_section.rs`. The implementation follows the verified `snapshots_section.rs` pattern (polled-into-local Signal mirror with `use_effect(use_reactive!)` to avoid the double-click race documented at `snapshots_section.rs:72-75`):

```rust
use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;

use crate::components::Switch;
use crate::context::AppContext;
use crate::frame::settings_panel::field_row::SettingsFieldRow;
use crate::frame::settings_panel::section::SettingsSection;

const LAUNCH_AT_STARTUP_ID: &str = "if-settings-startup-launch-at-startup";
const START_MINIMIZED_ID: &str = "if-settings-startup-start-minimized";

#[component]
pub(crate) fn StartupSection() -> Element {
    let ctx = use_context::<AppContext>();
    let settings = ctx.settings;
    let commands = ctx.commands.clone();

    let polled = settings.read().startup.clone();
    let polled_launch = polled.launch_at_startup;
    let polled_start_min = polled.start_minimized_to_tray;

    let mut launch_local = use_signal(|| polled_launch);
    use_effect(use_reactive!(|polled_launch| {
        launch_local.set(polled_launch);
    }));

    let mut start_min_local = use_signal(|| polled_start_min);
    use_effect(use_reactive!(|polled_start_min| {
        start_min_local.set(polled_start_min);
    }));

    let commands_for_launch = commands.clone();
    let on_launch_change = move |_evt: FormEvent| {
        let new_value = !launch_local();
        launch_local.set(new_value);
        let _ = commands_for_launch.send(EngineCommand::SetAutostart { enabled: new_value });
    };

    let commands_for_start_min = commands.clone();
    let on_start_min_change = move |_evt: FormEvent| {
        let new_value = !start_min_local();
        start_min_local.set(new_value);
        let _ = commands_for_start_min.send(EngineCommand::SetStartMinimizedToTray { enabled: new_value });
    };

    rsx! {
        SettingsSection {
            children: rsx! {
                SettingsFieldRow {
                    label: "Launch InputForge at startup".to_owned(),
                    helper: "Run automatically after sign-in.".to_owned(),
                    control_id: LAUNCH_AT_STARTUP_ID.to_owned(),
                    control: rsx! {
                        Switch {
                            id: Some(LAUNCH_AT_STARTUP_ID.to_owned()),
                            checked: launch_local,
                            onchange: on_launch_change,
                        }
                    },
                }
                SettingsFieldRow {
                    label: "Start minimized to tray".to_owned(),
                    helper: "Open without showing the main window. Use the tray icon to bring it back.".to_owned(),
                    control_id: START_MINIMIZED_ID.to_owned(),
                    control: rsx! {
                        Switch {
                            id: Some(START_MINIMIZED_ID.to_owned()),
                            checked: start_min_local,
                            onchange: on_start_min_change,
                        }
                    },
                }
            },
        }
    }
}
```

Wired into `SettingsPanel` (`frame/settings_panel/mod.rs:14-23`):

```rust
rsx! {
    Stylesheet { href: SETTINGS_PANEL_CSS }
    div { class: "if-settings-panel",
        StartupSection {}
        SnapshotsSection {}
    }
}
```

Order is fixed: Startup first because "what does this app do at sign-in" reads before "internal storage detail". `SettingsSection` carries no heading prop (verified at `frame/settings_panel/section.rs:8`); per the established `panel_renders_field_rows_without_heading` convention at `frame/settings_panel/mod.rs:67`, the visible "Startup" string lives only in the field-row labels. Components reused verbatim: `SettingsSection` and `SettingsFieldRow` from F15, `Switch` from F2; no new design-system primitives.

**9. `SettingsSnapshot` mirror.** `SettingsSnapshot` in `crates/inputforge-gui-dx/src/context.rs:39` (the polled view) gains:

```rust
pub startup: StartupSettings,
```

(Re-export `StartupSettings` from `inputforge_core::settings` so the GUI crate can name the type.) Populated by `SettingsSnapshot::from_state` at `context.rs:52`, which already reads `state.snapshot_config`; the addition is `startup: state.startup.clone()`.

`AppState` in `crates/inputforge-core/src/state/mod.rs` gains `pub startup: StartupSettings` next to `pub snapshot_config: SnapshotConfig` (currently at `state/mod.rs:92`), populated by the engine from `AppSettings` on load and on each `Set*` command, exactly as `state.snapshot_config` is maintained today (verified at `engine/mod.rs:133` for init, `engine/run.rs:552` for `ReloadSettings`, `engine/run.rs:565` for `SetSnapshotConfig` success, and `engine/run.rs:577` for the rollback path). The existing F6 mirror tests `engine_initialisation_mirrors_settings_snapshot_into_state` (`engine/tests.rs:4024`) and `reload_settings_mirrors_into_state_snapshot_config` (`engine/tests.rs:4042`) are the templates for the F16 analogues listed under Test plan. The bridge polling cadence is whatever F1 already established; no new polling added.

**10. Reactivity contract.** When the user flips a switch:

1. Switch fires `onchange(FormEvent)`. The handler reads `local_signal()`, flips it, and dispatches `EngineCommand::SetAutostart { enabled: new_value }` (matching the `snapshots_section.rs:124-136` pattern, including the local-Signal optimistic update that resolves the double-click-within-one-tick race).
2. Engine handles. On success: writes `AppState.startup`, persists `settings.toml`. On `SetSnapshotConfig`-style save failure: rolls back the in-memory + `AppState` mirror, pushes a warning. On `SetAutostart` OS-write failure: pushes a warning, does not mutate settings or `AppState`.
3. Bridge poll updates the `SettingsSnapshot` signal.
4. `use_effect(use_reactive!)` on the polled value snaps the local Signal back to the polled value if they differ.
5. Toasts surface from `state.warnings` via the existing warnings bridge.

Limitation, surfaced honestly so the implementer is not surprised: on the `SetAutostart` OS-write-failure path, `AppState.startup` is intentionally not mutated, so polled does not change, `use_reactive!` does not re-fire, and the local Signal stays at the user-attempted value. The user-visible feedback is the warning toast. The user can click the toggle again to retry; if the OS write succeeds, polled converges and the toggle renders correctly. This same limitation already exists in `SnapshotsSection` and is tolerated there. A future F16.1 could close it via a `state.startup_revision: u64` counter bumped on every command attempt (out of scope here).

**11. No gating, no helper hint, no per-field validation.** Both switches are independent. No "applies on next launch" hint (peer survey: none of Discord, qBittorrent, OBS, Slack, reWASD ship one). The labels and helper text are accurate by themselves.

### CLI and lifecycle

**12. Minimal `main.rs` change.** Insert one helper and use its result at the existing `launch_gui` call site (`main.rs:105-111`):

```rust
fn resolve_start_minimized(cli_flag: bool, settings_flag: bool) -> bool {
    cli_flag || settings_flag
}

// in main():
let effective_start_minimized = resolve_start_minimized(
    cli.start_minimized,
    settings.startup.start_minimized_to_tray,
);
// pass effective_start_minimized to launch_gui in place of cli.start_minimized.
```

The helper exists for testability. `launch_gui`, `LaunchParams.start_minimized`, and `lifecycle::apply_start_minimized` are untouched; they already operate on a single bool. (Diff is one new helper plus one call-site argument change, both in `main.rs`.)

**13. Toggle does not affect the current window.** Both settings are "applies on next launch". Flipping `start_minimized_to_tray` does not minimize the running window. Flipping `launch_at_startup` does not launch anything. Matches every peer surveyed; no inline hint.

---

## Cross-coupling rules

A small table because this is the most error-prone part:

| Trigger | `launch_at_startup` value | `start_minimized_to_tray` value | OS write performed |
|---|---|---|---|
| `SetAutostart { enabled: true }` | -> `true` (on success) | unchanged | `set_enabled(true, args_from_setting)` |
| `SetAutostart { enabled: false }` | -> `false` (on success) | unchanged | `set_enabled(false, &[])` |
| `SetStartMinimizedToTray { enabled: true }` while `launch_at_startup == true` | unchanged | -> `true` | `set_enabled(true, &["--start-minimized"])` (re-register, best-effort) |
| `SetStartMinimizedToTray { enabled: false }` while `launch_at_startup == true` | unchanged | -> `false` | `set_enabled(true, &[])` (re-register, best-effort) |
| `SetStartMinimizedToTray { enabled: _ }` while `launch_at_startup == false` | unchanged | -> new value | none |
| Engine startup, `is_enabled() = true`, `settings.startup.launch_at_startup = false` | -> `true` (synced from OS) | unchanged | `set_enabled(true, args_from_setting)` (argv resync) |
| Engine startup, `is_enabled() = false`, `settings.startup.launch_at_startup = true` | -> `false` (synced from OS) | unchanged | none |
| Engine startup, `is_enabled() = true`, `settings.startup.launch_at_startup = true` | unchanged | unchanged | `set_enabled(true, args_from_setting)` (unconditional argv resync, best-effort) |

The "re-register, best-effort" rows do not revert the start-minimized toggle on failure: the engine pushes a warning and proceeds. The persisted setting is what the user asked for; the autostart argv is a downstream consequence the next engine startup will heal via the unconditional argv resync row above.

---

## Errors

### Variants

`crates/inputforge-autostart/src/error.rs`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum AutostartError {
    #[error("autostart not supported on this platform")]
    NotSupported,
    #[error("registry write denied")]
    RegistryDenied,
    #[error("autostart directory not writable: {0}")]
    DirectoryNotWritable(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("backend error: {0}")]
    Backend(String),
}
```

`Backend(String)` wraps the lower-level `auto_launch::Error` opaquely; we do not match on its internal variants.

### Surfacing

All F16 failure feedback flows through the existing engine -> UI warnings channel: the engine pushes a `String` to `state.warnings` (`state/mod.rs:119`); the bridge polls `MetaSnapshot.warnings` and the install_warnings_bridge effect (`toast/warnings_bridge.rs:25-45`) emits each new tail entry as `ToastLevel::Warning`. F16 does not introduce a new toast channel.

- `SetAutostart` OS-write failure: push `"Could not change launch-at-startup setting."` to `state.warnings`. No diagnostic detail in the user-visible message; details go to `tracing::warn`. Settings are not mutated.
- `SetAutostart` settings-save failure (autostart write succeeded but `settings.toml` write failed): push `format!("Could not save settings: {e}")` (matches the `SetSnapshotConfig` precedent at `engine/run.rs:578`); roll back the in-memory + `AppState` mirror per the `SetSnapshotConfig` template at `engine/run.rs:572-578`.
- `SetStartMinimizedToTray` settings-save failure: same pattern as `SetSnapshotConfig`; roll back, push `"Could not save settings: {e}"`.
- `SetStartMinimizedToTray` re-register failure (autostart on, argv re-write failed after the persisted setting was already saved): push `"Saved, but could not update the auto-launch arguments. Restart of InputForge may use the previous setting."` to `state.warnings`. Do not revert the toggle; the next engine startup heals the argv via the choice 7 unconditional resync.
- Startup reconciliation `is_enabled()` failure: log via `tracing::warn` only, no warnings push.
- Startup reconciliation argv resync failure: push `"Could not refresh auto-launch arguments at startup."` to `state.warnings`.

### Recovery paths

- The user can toggle off and on again to retry an `SetAutostart` OS-write failure.
- The next engine startup unconditionally re-pushes argv when `launch_at_startup == true` (choice 7, step 4), so a stale argv from a `SetStartMinimizedToTray` re-register failure heals on the next launch.

---

## Linux future-proofing

Linux is not on the build matrix at landing, but the design must not back into corners that will require redesign when it joins.

1. `auto-launch 0.6` writes `~/.config/autostart/<app_name>.desktop` with `Type`, `Name`, `Exec`, `Terminal=false`, `StartupNotify=false`. That is enough for GNOME, KDE, XFCE, Cinnamon, MATE.
2. We do not set `X-GNOME-Autostart-enabled`. gnome-tweaks treats absent as enabled. Disabling from gnome-tweaks rewrites the file with `Hidden=true`, which our OS-wins reconciliation reads as "disabled" via `auto_launch::is_enabled`.
3. Wayland and tiling WMs may ignore iconification, but our "start minimized to tray" semantic is `set_visible(false)`, which is honored by both X11 and Wayland under Tao's GTK backend.
4. WebKitGTK does not have the WebView2 hidden-init blank-render bug, so `apply_start_minimized` ports cleanly. Tao's `Window::set_visible` calls `gtk_widget_set_visible`, safe pre- and post-realize.
5. Tray support on Linux uses StatusNotifierItem (D-Bus). Some minimal WMs (sway, i3) lack a SNI host; the tray icon will not render in those environments. Out of scope here. "Hidden in tray" still hides the window even if no tray host is running, which is graceful degradation; users on those WMs simply lose the convenient Show affordance until they configure their bar.
6. AppImage usage breaks `std::env::current_exe()` stability across mounts. Documented limitation; not blocking.
7. App name `"InputForge"` is the binary stem and the desktop-file basename. No collisions expected.

---

## Test plan

### `inputforge-autostart`

- `mock.rs::MockAutostart` tests: enable, is_enabled, disable, is_enabled round-trip; `fail_next_set` returns the configured error and leaves state unchanged; `fail_next_is_enabled` likewise; recorded-call inspection (`enable`/`disable` calls and the args passed).
- `windows.rs` integration tests gated `#[cfg(target_os = "windows")]` and `#[ignore]`, run via `cargo test --workspace -- --ignored`. Use a randomized `app_name` like `"inputforge-autostart-test-{nanos}"` plus a `Drop` guard that removes the registry value, even on panic. Reusing `#[ignore]` instead of inventing an `integration-tests` feature follows the workspace precedent (no such feature exists in `inputforge-core/Cargo.toml`).
- `linux.rs` integration tests symmetric to the above, gated `#[cfg(target_os = "linux")]` and `#[ignore]`, with `XDG_CONFIG_HOME` set to a tempdir for the duration of each test.

### `inputforge-core` settings (`settings.rs`)

- `pre_f16_settings_loads_with_default_startup`: writes a TOML lacking the `[startup]` table; asserts `settings.startup == StartupSettings::default()`.
- `pre_f16_settings_loads_with_partial_startup_table`: writes a TOML with `[startup]` containing only `launch_at_startup = true`; asserts the missing inner field defaults to `false`.
- `settings_round_trips_startup_table`: sets both inner fields true, save+load, assert equality and that the on-disk TOML contains a `[startup]` sub-table (mirrors `settings_round_trips_with_custom_snapshot_table` at `settings.rs:374`).

### `inputforge-core` engine (`engine/tests.rs`)

Mirror chain (templated on `engine_initialisation_mirrors_settings_snapshot_into_state` at `engine/tests.rs:4024` and `reload_settings_mirrors_into_state_snapshot_config` at `:4042`):

- `engine_initialisation_mirrors_startup_into_state`: settings has both inner fields true at construction; assert `state.startup == settings.startup` after `Engine::new`.
- `reload_settings_mirrors_startup_into_state`: write a fresh `settings.toml` with a non-default `[startup]`, dispatch `ReloadSettings`, assert `state.startup` matches.

`SetAutostart` handler:

- `set_autostart_persists_and_calls_manager`: mock returns Ok; assert `mock` recorded `set_enabled(true, args_from_settings)`, `state.startup.launch_at_startup == true`, on-disk TOML reflects it.
- `set_autostart_writes_state_field_on_success`: same as above, focused on the `state.startup` mirror update specifically.
- `set_autostart_passes_start_minimized_arg_when_enabled`: pre-seed `state.startup.start_minimized_to_tray = true`; dispatch `SetAutostart { enabled: true }`; assert mock recorded args `&["--start-minimized"]`.
- `set_autostart_passes_no_args_when_start_minimized_off`: pre-seed both off; dispatch `SetAutostart { enabled: true }`; assert mock recorded args `&[]`.
- `set_autostart_failure_leaves_state_field_unchanged`: mock `set_enabled` returns Err; assert `state.startup` unchanged, on-disk TOML unchanged, exactly one new entry in `state.warnings` matching `"Could not change launch-at-startup setting."`.
- `set_autostart_off_preserves_start_minimized_to_tray`: pre-seed both on; dispatch `SetAutostart { enabled: false }`; assert `state.startup.launch_at_startup == false` and `state.startup.start_minimized_to_tray == true`.

`SetStartMinimizedToTray` handler:

- `set_start_minimized_persists_and_mirrors`: dispatch `SetStartMinimizedToTray { enabled: true }`; assert state mirror, on-disk TOML, no extra autostart call when `launch_at_startup == false`.
- `set_start_minimized_resyncs_autostart_args_when_enabled`: pre-seed `launch_at_startup = true`; dispatch `SetStartMinimizedToTray { enabled: true }`; assert mock recorded `set_enabled(true, &["--start-minimized"])`.
- `set_start_minimized_when_autostart_off_does_not_call_manager`: pre-seed `launch_at_startup = false`; dispatch the command; assert no `set_enabled` call recorded.
- `set_start_minimized_persists_when_resync_fails`: pre-seed `launch_at_startup = true`; mock `set_enabled` returns Err; assert `state.startup.start_minimized_to_tray` is still updated, on-disk TOML reflects it, exactly one warning push matching `"Saved, but could not update the auto-launch arguments. Restart of InputForge may use the previous setting."`.

Engine startup reconciliation:

- `engine_startup_reconciles_settings_to_os`: mock `is_enabled() = Ok(true)`, settings file says `launch_at_startup = false`; assert `state.startup.launch_at_startup == true` after `Engine::new`, on-disk TOML reflects it.
- `engine_startup_repushes_argv_when_autostart_enabled`: mock `is_enabled() = Ok(true)`, settings says `launch_at_startup = true && start_minimized_to_tray = true`; assert mock recorded a `set_enabled(true, &["--start-minimized"])` call after init.
- `engine_startup_repush_failure_warns_does_not_block`: same setup; mock `set_enabled` returns Err; assert `Engine::new` succeeds, exactly one warning push matching `"Could not refresh auto-launch arguments at startup."`.
- `engine_startup_tolerates_is_enabled_error`: mock `is_enabled` returns Err; assert no panic, `state.startup` unchanged, no warnings push.

### `inputforge-app` (`main.rs`)

- `resolve_start_minimized_or_logic`: table test covering the four `(cli, settings)` combinations.

### `inputforge-gui-dx` (`frame/settings_panel/startup_section.rs`)

- `renders_two_switches_with_persisted_state` (PolledHarness style, seed `state.startup` with both true, assert both `<input type="checkbox" checked>` render).
- `flipping_launch_at_startup_dispatches_set_autostart_command`: assert one `EngineCommand::SetAutostart { enabled: true }` is sent on the channel.
- `flipping_start_minimized_dispatches_set_start_minimized_command`: assert one `EngineCommand::SetStartMinimizedToTray { enabled: true }` is sent.
- `panel_renders_field_rows_without_heading` in `settings_panel/mod.rs:67` continues to pass: `SettingsSection` does not emit `<h2>` or `<h3>`; `StartupSection` follows the same convention.

### Manual verification (deferred to plan, kept here so it does not get lost)

Smoke tests in plan steps must use `cargo`. The list below is for `dx run -p inputforge-app` interactive verification only.

1. Toggle Launch on; confirm `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\InputForge` contains the current exe path.
2. Reboot or sign out / sign in: app launches.
3. Toggle Start minimized on; reboot: app launches with no main window, tray icon present, click Show restores.
4. Toggle Launch off: registry value gone.
5. External edit: while running, delete the registry value via `regedit`, restart the app, observe the panel switch reflects "off" and `settings.toml` was rewritten with `launch_at_startup = false`.

---

## Out of scope

- macOS support. The chosen crate covers it (Launch Agent, AppleScript Login Items, SMAppService) but no macOS target is in the project roadmap.
- Per-profile startup behavior (e.g., "auto-load this profile on startup" beyond the existing `last_profile`).
- Delayed-launch scheduling.
- "Apply now" affordance on the start-minimized toggle (the running window does not minimize when the toggle is flipped on; this matches every peer surveyed).
- A `--no-start-minimized` CLI flag.
- A "Close to tray instead of quitting" setting (the X-button-hides behavior is already shipped via `WindowCloseBehaviour::WindowHides` at `lib.rs:103` and is not adjustable).
- i18n for the new labels.
- Tracking which DE the user is on to prefer systemd `--user` services over XDG autostart on systemd-using distros.

---

## Definition of done

- `inputforge-autostart` crate exists, builds, has unit tests passing on Windows.
- `AppSettings` has the new `startup: StartupSettings` field with `#[serde(default)]` and the inner `StartupSettings` struct gates each bool with `#[serde(default)]` as well.
- `AppState.startup: StartupSettings` mirrors `settings.startup` on engine init and on every `Set*` command.
- Two new struct-style `EngineCommand` variants (`SetAutostart { enabled }`, `SetStartMinimizedToTray { enabled }`) are handled by the engine with the documented atomicity, reconciliation including the unconditional argv resync at startup when autostart is enabled, and warnings-channel surfacing.
- `StartupSection` renders above `SnapshotsSection`, switches reflect `SettingsSnapshot.startup`, dispatch the right commands using the local-Signal optimistic-update pattern from `snapshots_section.rs:67-79` and `:124-136`.
- `main.rs` ORs CLI flag and persisted setting via `resolve_start_minimized(cli.start_minimized, settings.startup.start_minimized_to_tray)`.
- All listed tests pass under `cargo test --workspace`. Integration tests gated `#[ignore]` pass under `cargo test --workspace -- --ignored` on Windows (and on Linux when that target joins the build matrix).
- Manual smoke list passes on Windows.
- Public exports from `inputforge-autostart`: the `AutostartManager` trait, `AutostartError`, and a `new_for_current_platform(app_name: &str) -> Box<dyn AutostartManager>` factory. `MockAutostart` is exposed behind a `mock` feature so `inputforge-core`'s tests can depend on it without leaking the mock into release builds. Concrete cfg-gated types (`WindowsAutostart`, `LinuxAutostart`, `NoOpAutostart`) remain crate-private.
