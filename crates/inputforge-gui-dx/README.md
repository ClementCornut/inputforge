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
| `cargo run --no-default-features --features gui-dioxus`   | Dioxus |
| `cargo build --features gui-dioxus` (default still on)    | compile error |
| `cargo build --no-default-features`                       | compile error |
