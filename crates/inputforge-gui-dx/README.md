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

`app_root` already wraps `F1Readout` in `ThemeProvider` — every screen
mounted under `app_root` inherits it.

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

For asymmetric grids (e.g. F1Readout's two-column key/value layout), keep an
inline `style:` — these primitives intentionally don't model `display: grid`.

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
