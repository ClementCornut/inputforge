# inputforge-gui-dx

Dioxus Desktop GUI for InputForge — parallel runtime, opt-in via the
`gui-dioxus` feature on `inputforge-app`. The egui crate (`inputforge-gui`)
remains the default until the F16 cutover.

## Pinned versions

- `dioxus`: `0.7.6` (workspace-pinned, `desktop` feature)
- `dioxus-cli`: `0.7.6`

## Dev workflow — primary RSX loop (recommended)

The `bridge_demo` example seeds a mock `AppState` and calls `launch_gui`
directly. No engine, no tray, no profile I/O — safe to hot-reload.

```bash
cargo install dioxus-cli --version 0.7.6
dx serve -p inputforge-gui-dx --example bridge_demo --platform desktop
```

Edit RSX in `src/app.rs` — the running window updates within ~1s without
restarting. Rust logic / state / non-RSX changes still require a full rebuild.

## Dev workflow — full app integration smoke

Exercises the real engine thread, tray, profile autoload, and HidHide
warning scan. **Not** the daily loop — each hot-reload respawns the engine
thread, re-registers the tray, re-runs HidHide detection.

```bash
cd crates/inputforge-app
dx serve --platform desktop --no-default-features --features gui-dioxus
```

## Build / run matrix

| Command | Result |
|---|---|
| `cargo build` / `cargo run` | egui (default) |
| `cargo build --no-default-features --features gui-dioxus` | Dioxus |
| `cargo run --no-default-features --features gui-dioxus`   | Dioxus shell + tray + lifecycle, production-viable |
| `cargo build --features gui-dioxus` (default still on)    | compile error |
| `cargo build --no-default-features`                       | compile error |

## Component Gallery (F2)

The gallery is the primary visual surface for design-system development. Run via:

    dx serve --example component_gallery --platform desktop

Hot-reload: editing any `.rs`, `.css`, or `.rsx` file in `src/components/`,
`src/theme/`, or `assets/` updates the running gallery within ~1 second.
**Note:** editing an SVG file in `src/icons/svg/` requires a full rebuild
(see "Adding a new icon" below).

## ThemeProvider

`crate::theme::ThemeProvider` mounts every token CSS file, `global.css`,
and every component CSS file in cascade order. Wrap the root of any Dioxus
component tree that should use the design system:

```rust
use inputforge_gui_dx::theme::ThemeProvider;

rsx! {
    ThemeProvider {
        // your components
    }
}
```

`app_root` already wraps `PlaceholderShell` in `ThemeProvider` — every
screen mounted under `app_root` inherits it.

## Adding a new icon

1. Drop the `.svg` file under `src/icons/svg/<name>.svg` (Phosphor regular weight, `viewBox="0 0 256 256"`).
2. Add a variant to the `Icon` enum in `src/icons/mod.rs`.
3. Add a match arm in `Icon::svg()` mapping the variant to `include_str!("svg/<name>.svg")`.
4. Run `cargo test -p inputforge-gui-dx --lib icons::tests` — the well-formedness test will catch corrupt files.

## Layout primitives

`Stack`, `Cluster`, and `Inset` retire most inline `style:` attributes in
consumer code. Each accepts a CSS custom-property *name* (e.g. `"--space-4"`)
for `gap`/`padding` so magic px values stay out of the consumer side:

| Primitive | Direction         | Defaults                        |
|---        |---                |---                              |
| `Stack`   | column            | `gap: --space-4`, padding none  |
| `Cluster` | row, wraps, ⤳cent | `gap: --space-3`, padding none  |
| `Inset`   | block             | `padding: --space-4`            |

For asymmetric grids (e.g. a two-column key/value layout), keep an inline
`style:` — these primitives intentionally don't model `display: grid`.

## Status backgrounds

Use the `--color-{info,success,warning,error}-bg` tokens for tinted status
surfaces (Badge, Toast). Never embed `rgba()` literals in component CSS — the
revised palette would silently drift from the foreground/border tokens.

## Reduced motion

`motion.css` zeroes the `--duration-*` tokens under
`@media (prefers-reduced-motion: reduce)`. Component CSS that pipes
animation duration through these tokens disables motion automatically;
component CSS that hard-codes `ms` in transition shorthands does not — keep
all timing in tokens.

## Toolchain prerequisites

- `dx` (dioxus-cli) version 0.7.6 — install via `cargo install dioxus-cli --version 0.7.6`. Required for hot-reload (`dx serve`).
- WebView2 runtime — bundled with Windows 11. On Windows 10 or earlier, install the Evergreen Standalone runtime from https://developer.microsoft.com/microsoft-edge/webview2/.

## SDL3.dll placement

Windows builds need `SDL3.dll` next to the executable. `crates/inputforge-app/build.rs`
copies it from `<workspace>/SDL/SDL3.dll` into both:

- `target/<cargo-profile>/SDL3.dll` — alongside the `cargo` binary
- `target/dx/inputforge-app/<dx-profile>/windows/app/SDL3.dll` — alongside the `dx` binary

The `<dx-profile>` segment uses cargo's `PROFILE` env var (`debug` or
`release`), which collapses custom cargo profiles like dx-cli's `desktop-dev`
to one of those two values based on profile inheritance. dx names its bundle
output dirs `debug` and `release` regardless of the underlying cargo profile.

`Dioxus.toml`'s `[bundle].resources` is **not** an option here: in dioxus-cli
0.7.6 that key is consumed only by the `bundler` module, which is invoked
exclusively from `dx bundle` (production packaging). `dx serve`, `dx run`,
and `dx build` never copy `bundle.resources` into the dev output dir
(verified empirically and by reading `packages/cli/src/bundler/` and
`packages/cli/src/cli/bundle.rs` at the v0.7.6 tag).

**Recovery if SDL3.dll goes missing.** The build script declares
`cargo:rerun-if-changed=` on the source DLL and both destinations, but cargo
treats *missing* destination files as untracked rather than changed — so if
you `rm -rf target/dx` and the next `dx run` fails with
`STATUS_DLL_NOT_FOUND` (`0xC0000135`), the script did not re-run because
cargo's fingerprint of `inputforge-app` is still fresh. Force a rerun with:

```bash
cargo clean -p inputforge-app
dx run -p inputforge-app --no-default-features --features gui-dioxus
```

A full `cargo clean` is unnecessary; the per-package clean is enough.

## F3 — Tray bridge & hide-to-tray lifecycle

### Tray bridge

Under `--features gui-dioxus`, tray menu events are observed via the
`use_muda_event_handler` hook (`dioxus_desktop`'s public hooks API). The
spec originally specified `Config::with_custom_event_handler` matching
on `UserWindowEvent::MudaMenuEvent`, but `dioxus_desktop::ipc` is a
private module in 0.7.6 — the type is unreachable from external crates.
The hook performs the equivalent pattern-match internally and delivers
the same `muda::MenuEvent` payload through a callback that runs on the
event-loop thread. F3 routes the menu id to a `TrayAction` via pure
logic in `tray::action::TrayAction::from_event`, then `try_send`s on a
bounded `tokio::sync::mpsc` channel created inside `app_root`. A Dioxus
task (also spawned from `app_root`) drains the channel and dispatches:
`Show` → `lifecycle::show_window`, `Toggle` → engine command via
`AppContext::commands`, `Quit` → `lifecycle::request_quit`.

The handler is observe-only: it only `try_send`s and logs overflow. It
never mutates `ControlFlow` or interacts with the event loop directly.
Routing logic is pure (`tray::action`) and unit-tested.

### Hide-to-tray window lifecycle

`Config::with_close_behaviour(WindowCloseBehaviour::WindowHides)` makes
X-click hide the window natively (Dioxus calls `set_visible(false)` and
consumes the close-requested event; F3 has no close-handler code path).
Tray Show re-opens via `set_visible(true)` + `set_focus()`. Tray Quit
flips this window's close behavior to `WindowCloses` then calls
`close()`; with the default `exit_on_last_window_close = true` the
event loop exits and `launch_gui` returns. `main.rs::shutdown()` then
runs, the engine thread joins, `HidHide` unhide and `vJoy` release fire
via `Drop`.

`--start-minimized` is plumbed via the `start_minimized: bool`
parameter on `launch_gui`. The Dioxus side calls `set_visible(false)`
once during `app_root` mount when the flag is set; tray Show works
identically. The egui side ignores the parameter — it already gates
startup launch from `cli.start_minimized` in `main.rs`.

**Cost asymmetry.** The egui path skips `launch_gui_blocking` entirely
when `--start-minimized` is set, so no window or webview exists until
tray Show. The Dioxus path always creates the WebView2 window (plus the
polling and listener tasks) and merely hides it. End-user UX is the same;
startup memory/CPU is not. Dioxus 0.7's `tao::EventLoop::run` is one-shot,
so a tray-triggered relaunch isn't viable without restructuring the whole
event loop — defer-launch parity is left as a future investigation.

Note the dioxus-desktop 0.7.6 API spelling asymmetry:
- `Config::with_close_behaviour` — UK (builder method)
- `WindowCloseBehaviour` — UK (enum)
- `DesktopService::set_close_behavior` — US (per-window setter)

### New primitives (F3)

- `Tabs` — full WAI-ARIA Tabs pattern (`role="tablist"`/`tab`,
  focus-roving with `tabindex` 0|-1, arrow keys + Home/End for cycle
  and activate). Each tab button gets `id="tab-{id}"`; set
  `TabItem::controls` to wire `aria-controls` at the consumer's
  tabpanel `id` (the consumer renders the `role="tabpanel"` div with
  matching `id` and an `aria-labelledby` back-reference to `tab-{id}`).
  Focus is moved imperatively via `MountedData::set_focus` after each
  keyboard activation so the focus ring follows selection. Stateless:
  caller owns `value`. Reused by F11 (Modes); see `examples/component_gallery.rs`
  for the canonical full-pattern wiring.
- `StatusBar` — three-slot horizontal bar (start / middle / end). Fixed
  28px height. ARIA-neutral wrapper — consumers add `role="status"` /
  `aria-live` only on the specific elements they want announced (e.g.,
  the engine-status badge in `StatusBarView`).

### Placeholder shell

`shell/placeholder.rs` and `assets/shell/placeholder-shell.css` are
**explicitly disposable at F5**. Treat them as scratch — F5 may replace
the entire grid template, not just slot contents. The shell exists so
F3's tray-bridge lifecycle can be observed against a coherent layout
(open the window, watch the status bar reflect engine state, click
tray Toggle, watch the badge flip).
