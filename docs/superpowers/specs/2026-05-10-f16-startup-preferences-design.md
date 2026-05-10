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

InputForge is a tray-resident input remapper. Users expect it to be running whenever they game, the same way Discord, qBittorrent, OBS, and reWASD are. Today the app supports a `--start-minimized` CLI flag and a working tray, but lacks two preferences mature peers ship: launch at OS sign-in, and start in the tray on every launch (not just CLI-flagged ones).

This spec adds those two preferences. Linux is the second supported platform target on the project roadmap, so the implementation must work on both Windows and Linux from day one of the abstraction. Today the binary only ships on Windows; the Linux concrete impl is in scope for the spec but not on the build matrix until the platform itself is.

### Existing scaffolding (audit before design)

Verified against current source on `main` (commit `e037595`):

- `crates/inputforge-app/src/cli.rs:23` defines `Cli::start_minimized: bool` (clap flag `--start-minimized`).
- `crates/inputforge-app/src/main.rs:107` plumbs `cli.start_minimized` into `launch_gui`.
- `crates/inputforge-gui-dx/src/lib.rs:74` carries `start_minimized: bool` into `LaunchParams`.
- `crates/inputforge-gui-dx/src/lifecycle/mod.rs:48` `apply_start_minimized` calls `window().set_visible(false)` (so the existing flag means "hidden in tray", not "iconified to taskbar"; see Choice 1).
- `crates/inputforge-app/src/tray.rs:1-98` defines `AppTray` with Show, Activate, Quit menu items; tray icon is always created at boot.
- `crates/inputforge-gui-dx/src/lib.rs:97` configures `WindowCloseBehaviour::WindowHides` so the X button hides to tray.
- `crates/inputforge-core/src/settings.rs:25-43` defines `AppSettings` with `last_profile`, `snapshot`, `device_aliases`, `device_registry`. F6 already proved the `#[serde(default)]` + TOML load/save path; this spec adds two fields to that same struct.
- `crates/inputforge-gui-dx/src/frame/settings_panel/mod.rs:1-181` renders only `SnapshotsSection` today; `panel_renders_field_rows_without_heading` (line 67) asserts no `<h2>` or `<h3>` is emitted by the panel itself.

The persisted-setting flow does not yet exist. There is no autostart code anywhere in the workspace (verified via `search_text` for `auto-launch`, `HKCU`, `autostart`).

### What the design adds

1. Two new boolean fields on `AppSettings`.
2. A new crate `inputforge-autostart` with a trait abstraction over the OS write.
3. Two new `EngineCommand` variants that own the OS write atomically with the settings persistence.
4. A new `StartupSection` in the settings panel above `SnapshotsSection`.
5. One-line change in `main.rs` to OR the CLI flag with the persisted setting.

---

## Confirmed design choices

The decisions below were each surfaced and approved during brainstorming.

### Behavior model

**1. "Start minimized" semantics: hidden in tray.** Today's `apply_start_minimized` sets the window invisible, not iconified. We keep that semantic and label it accordingly. There is no taskbar entry; the tray icon is the only visible affordance until the user clicks Show. Rationale: matches reWASD's "background agent" framing, OBS's "minimize to system tray" wording, and qBittorrent #8644's resolution; tray-resident input remappers do not need a taskbar entry on launch.

**2. Two independent toggles.** "Launch InputForge at startup" and "Start minimized to tray" are independent settings. Either may be on alone. When auto-launch is on, the registered argv adds `--start-minimized` if and only if the start-minimized toggle is on. Rationale: dominant pattern across Discord, qBittorrent, OBS, Slack, reWASD; the gated and tri-state alternatives both break the manual-launch tray flow.

**3. CLI override semantics.** The `--start-minimized` flag is preserved unchanged. The effective value at boot is `cli.start_minimized || settings.start_minimized_to_tray`. There is no `--no-start-minimized` flag; the OR semantic is sufficient because (a) the auto-launch path passes the flag explicitly when both toggles are on, (b) a manual launcher who wants normal start with the setting on is an unsupported edge case (peer apps don't expose this either).

### Data model

**4. Two new fields on `AppSettings`.** In `crates/inputforge-core/src/settings.rs`:

```rust
#[serde(default)]
pub launch_at_startup: bool,

#[serde(default)]
pub start_minimized_to_tray: bool,
```

Both default to `false`, both gated by `#[serde(default)]` so pre-F16 settings.toml files load with no migration. Field names match user-facing labels (avoid mixing the `auto-launch` crate name into our domain term; avoid the ambiguity that bit qBittorrent's `start_minimized` field).

No `[startup]` sub-table. Two scalars do not warrant grouping; if a third related field appears, group then. YAGNI.

### Architecture

**5. New crate `inputforge-autostart`.** Mirrors the existing `device/` trait pattern in `inputforge-core` (where `Sdl3Input`, `MockInput`, `NoOpDeviceHider` live). Layout:

```
crates/inputforge-autostart/
├── Cargo.toml      # auto-launch = "0.6", thiserror, tracing
├── src/
│   ├── lib.rs      # pub trait AutostartManager + impl selection by cfg
│   ├── windows.rs  # #[cfg(target_os = "windows")] HKCU\...\Run
│   ├── linux.rs    # #[cfg(target_os = "linux")]   ~/.config/autostart/*.desktop
│   ├── mock.rs     # always available; used by tests
│   └── error.rs    # AutostartError variants
```

The trait is the seam:

```rust
pub trait AutostartManager: Send {
    fn is_enabled(&self) -> Result<bool, AutostartError>;
    fn set_enabled(&mut self, enabled: bool, args: &[&str]) -> Result<(), AutostartError>;
}
```

`Send` (not `Sync`) because only the engine thread mutates it. Args are passed at call time so the engine decides whether to include `--start-minimized`; the autostart impl is dumb about start-minimized.

Concrete impls wrap `auto_launch::AutoLaunch` (crate `auto-launch = "0.6"`, MIT, last release 2026-01-10, ~444k dl/month, the same code Tauri's autostart plugin uses internally). On `set_enabled(true, args)` the impls rebuild the `AutoLaunch` with current args via `AutoLaunchBuilder` and call `enable()`. On `set_enabled(false, _)` they call `disable()` and ignore args. App name is `"InputForge"`.

**6. Engine owns the OS write.** Two new `EngineCommand` variants in `crates/inputforge-core/src/engine/command.rs`:

```rust
SetAutostart(bool),
SetStartMinimizedToTray(bool),
```

The engine handles each:

`SetStartMinimizedToTray(value)`:
1. `self.settings.start_minimized_to_tray = value`.
2. Persist `settings.toml`.
3. If `self.settings.launch_at_startup`, re-register the autostart entry with the new args (best-effort; toast on failure but do not revert the toggle, the persisted setting is unrelated to autostart).

`SetAutostart(value)`:
1. `args = if self.settings.start_minimized_to_tray { &["--start-minimized"] } else { &[] }`.
2. `self.autostart.set_enabled(value, args)`.
3. On `Ok(())`: write `self.settings.launch_at_startup = value`, persist.
4. On `Err(e)`: log + push `ToastLevel::Error`. Do NOT mutate `self.settings`. The mirror chain (engine writes `AppState`, GUI polls `SettingsSnapshot`) means the UI switch reverts visually on the next poll because `AppState` was never updated.

Rationale: atomicity. A crash between an OS write and a settings write would leave the registry value and the TOML disagreeing. Engine is single-threaded for this state, so the (OS, then settings) sequence is linearizable.

**7. OS-wins reconciliation on engine startup.** In `Engine::new` (or first `run()` iteration), before processing the first command:

1. Call `autostart.is_enabled()`.
2. On `Ok(actual)` with `actual != settings.launch_at_startup`: set the field to `actual`, persist `settings.toml`, log at `tracing::info`.
3. On `Err(e)`: log a warning, leave settings as-is, do not toast.

Rationale: external changes (Task Manager, gnome-tweaks, manual registry edit) should be authoritative. The user wants the OS state to be the source of truth for OS-level settings. Polling beyond startup is rejected (waste, complexity); a single startup sync is sufficient.

### UI

**8. New `StartupSection` above `SnapshotsSection`** in `crates/inputforge-gui-dx/src/frame/settings_panel/`:

```rust
#[component]
pub(crate) fn StartupSection() -> Element {
    let ctx = use_context::<AppContext>();
    let settings = ctx.settings.read();

    rsx! {
        SettingsSection { heading: "Startup",
            SettingsFieldRow {
                label: "Launch InputForge at startup",
                helper: "Run automatically after sign-in.",
                control: rsx! {
                    Switch {
                        checked: settings.launch_at_startup,
                        on_change: move |v| { let _ = ctx.commands.send(EngineCommand::SetAutostart(v)); },
                    }
                },
            }
            SettingsFieldRow {
                label: "Start minimized to tray",
                helper: "Open without showing the main window. Use the tray icon to bring it back.",
                control: rsx! {
                    Switch {
                        checked: settings.start_minimized_to_tray,
                        on_change: move |v| { let _ = ctx.commands.send(EngineCommand::SetStartMinimizedToTray(v)); },
                    }
                },
            }
        }
    }
}
```

Wired into `SettingsPanel`:

```rust
rsx! {
    Stylesheet { href: SETTINGS_PANEL_CSS }
    div { class: "if-settings-panel",
        StartupSection {}
        SnapshotsSection {}
    }
}
```

Order is fixed: Startup first because "what does this app do at sign-in" reads before "internal storage detail". Components are reused verbatim from F15 (`SettingsSection`, `SettingsFieldRow`) and F2 (`Switch`); no new design-system primitives.

**9. SettingsSnapshot mirror.** `SettingsSnapshot` in `crates/inputforge-gui-dx/src/context.rs` (the polled view) gains:

```rust
pub launch_at_startup: bool,
pub start_minimized_to_tray: bool,
```

Populated by `SettingsSnapshot::from_state` (or whatever the existing constructor is named) reading from `AppState.app_settings`. The bridge polling cadence is whatever F1 already established; no new polling added.

**10. Reactivity contract.** When the user flips a switch:

1. Switch fires `on_change(true)`, GUI sends `EngineCommand::SetAutostart(true)`.
2. Engine handles, on success persists settings + writes `AppState`.
3. Bridge poll updates `SettingsSnapshot` signal.
4. `StartupSection` re-renders with the new value.
5. On failure: step 2 did not write `AppState`; the switch never updated visually; a toast appears.

The UI is read-only on this state. The engine is the single writer. Failure-snap-back is implicit in the mirror chain; no manual revert in the component.

**11. No gating, no helper hint, no per-field validation.** Both switches are independent. No "applies on next launch" hint (peer survey: none of Discord, qBittorrent, OBS, Slack, reWASD ship one). The labels and helper text are accurate by themselves.

### CLI and lifecycle

**12. One-line change in `main.rs`.** Insert one helper and use its result:

```rust
fn resolve_start_minimized(cli_flag: bool, settings_flag: bool) -> bool {
    cli_flag || settings_flag
}

// in main():
let effective_start_minimized = resolve_start_minimized(cli.start_minimized, settings.start_minimized_to_tray);
// pass effective_start_minimized to launch_gui in place of cli.start_minimized.
```

The helper exists for testability. `launch_gui`, `LaunchParams.start_minimized`, and `lifecycle::apply_start_minimized` are untouched; they already operate on a single bool.

**13. Toggle does not affect the current window.** Both settings are "applies on next launch". Flipping `start_minimized_to_tray` does not minimize the running window. Flipping `launch_at_startup` does not launch anything. Matches every peer surveyed; no inline hint.

---

## Cross-coupling rules

A small table because this is the most error-prone part:

| Trigger | `launch_at_startup` value | `start_minimized_to_tray` value | OS write performed |
|---|---|---|---|
| `SetAutostart(true)` | -> `true` (on success) | unchanged | `set_enabled(true, args_from_setting)` |
| `SetAutostart(false)` | -> `false` (on success) | unchanged | `set_enabled(false, &[])` |
| `SetStartMinimizedToTray(true)` while `launch_at_startup == true` | unchanged | -> `true` | `set_enabled(true, &["--start-minimized"])` (re-register, best-effort) |
| `SetStartMinimizedToTray(false)` while `launch_at_startup == true` | unchanged | -> `false` | `set_enabled(true, &[])` (re-register, best-effort) |
| `SetStartMinimizedToTray(_)` while `launch_at_startup == false` | unchanged | -> new value | none |
| Engine startup, `is_enabled() = true`, `settings.launch_at_startup = false` | -> `true` (synced from OS) | unchanged | none |
| Engine startup, `is_enabled() = false`, `settings.launch_at_startup = true` | -> `false` (synced from OS) | unchanged | none |

The "re-register, best-effort" rows do not revert the start-minimized toggle on failure, only log + toast; the persisted setting is what the user asked for and the autostart argv is a downstream consequence the next reconcile (or the next manual autostart toggle) will fix.

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

- `SetAutostart` failure: toast at `ToastLevel::Error` with text "Could not change launch-at-startup setting." (No diagnostic detail in the toast; details go to `tracing::warn`.)
- `SetStartMinimizedToTray` re-register failure (autostart on, argv re-write failed): toast at `ToastLevel::Warn` with text "Saved, but could not update the auto-launch arguments. Restart of InputForge may use the previous setting." Do not revert the toggle.
- Startup reconciliation failure: log only, no toast.

### Recovery paths

- The user can toggle off and on again to retry.
- The next engine startup does not retry (reconciliation queries the OS, doesn't re-push our state at it).

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

- `mock.rs::MockAutostart` tests: enable, is_enabled, disable, is_enabled round-trip; `fail_next_set` returns the configured error and leaves state unchanged; `fail_next_is_enabled` likewise.
- `windows.rs` integration tests behind `#[cfg(target_os = "windows")]` and a `--features integration-tests` gate, using a randomized `app_name` like `"inputforge-autostart-test-{nanos}"` plus a `Drop` guard to delete the registry value. Off by default in `cargo test` to avoid surprising side effects.
- `linux.rs` integration tests symmetric to the above, writing into `XDG_CONFIG_HOME` set to a tempdir.

### `inputforge-core` settings (`settings.rs`)

- `pre_f16_settings_loads_with_default_startup_fields`: writes a TOML lacking both new fields; asserts both load as `false`.
- `settings_round_trips_launch_at_startup_and_start_minimized`: sets both, save+load, assert equality.

### `inputforge-core` engine (`engine/tests.rs`)

- `set_autostart_persists_and_calls_manager` (mock returns Ok; assert state + recorded call).
- `set_autostart_failure_does_not_persist` (mock returns Err; assert settings unchanged + toast emitted).
- `set_start_minimized_resyncs_autostart_args_when_enabled` (autostart on, flip start-minimized, assert manager re-called with new args).
- `set_start_minimized_when_autostart_off_does_not_call_manager` (assert no extra `set_enabled` call).
- `set_start_minimized_persists_when_autostart_resync_fails` (mock fails on re-register; assert the start-minimized field is still persisted, only a warn-level toast emitted).
- `engine_startup_reconciles_settings_to_os` (mock `is_enabled` returns true, settings file says false; assert settings updated).
- `engine_startup_tolerates_is_enabled_error` (mock returns Err; assert no panic, settings unchanged, no toast).

### `inputforge-app` (`main.rs`)

- `resolve_start_minimized_or_logic`: table test covering the four (cli, settings) combinations.

### `inputforge-gui-dx` (`frame/settings_panel/startup_section.rs`)

- `renders_two_switches_with_persisted_state` (PolledHarness style, seed both true, assert both render checked).
- `flipping_launch_at_startup_dispatches_set_autostart_command`.
- `flipping_start_minimized_dispatches_set_start_minimized_command`.
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
- A "Close to tray instead of quitting" setting (the X-button-hides behavior is already shipped via `WindowCloseBehaviour::WindowHides` at `lib.rs:97` and is not adjustable).
- i18n for the new labels.
- Tracking which DE the user is on to prefer systemd `--user` services over XDG autostart on systemd-using distros.

---

## Definition of done

- `inputforge-autostart` crate exists, builds, has unit tests passing on Windows.
- `AppSettings` has the two new fields with `#[serde(default)]`.
- Two new `EngineCommand` variants are handled by the engine with the documented atomicity, reconciliation, and toast behavior.
- `StartupSection` renders above `SnapshotsSection`, switches reflect `SettingsSnapshot`, dispatch the right commands.
- `main.rs` ORs CLI flag and persisted setting via `resolve_start_minimized`.
- All listed tests pass under `cargo test --workspace`.
- Manual smoke list passes on Windows.
- No new public exports outside the new crate's `AutostartManager` trait, `AutostartError`, and concrete impl selection.
