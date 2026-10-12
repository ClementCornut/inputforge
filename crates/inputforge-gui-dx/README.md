# inputforge-gui-dx

Dioxus Desktop GUI for InputForge: the sole frontend after the egui crate
(`inputforge-gui`) was removed. `inputforge-app` depends on this crate
unconditionally; build with `cargo build -p inputforge-app` or
`dx run -p inputforge-app`.

## Pinned versions

- `dioxus`: `0.7.6` (workspace-pinned, `desktop` feature)
- `dioxus-cli`: `0.7.6`

## Dev workflow: primary RSX loop (recommended)

The `bridge_demo` example seeds a mock `AppState` and hosts the editor through
`render_editor`. No engine, no tray, no profile I/O; safe to hot-reload.

```bash
cargo install dioxus-cli --version 0.7.6
dx serve -p inputforge-gui-dx --example bridge_demo --platform desktop
```

Edit RSX in `src/app.rs`; the running window updates within ~1s without
restarting. Rust logic / state / non-RSX changes still require a full rebuild.

## Dev workflow: full app integration smoke

Exercises the real engine thread, tray, profile autoload, and HidHide
warning scan. **Not** the daily loop; each hot-reload respawns the engine
thread, re-registers the tray, re-runs HidHide detection.

```bash
dx serve -p inputforge-app --platform desktop
```

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

`app_root` mounts `frame::Layout` inside `ThemeProvider`; every
screen mounted under `app_root` inherits the design system.

## Adding a new icon

1. Drop the `.svg` file under `src/icons/svg/<name>.svg` (Phosphor regular weight, `viewBox="0 0 256 256"`).
2. Add a variant to the `Icon` enum in `src/icons/mod.rs`.
3. Add a match arm in `Icon::svg()` mapping the variant to `include_str!("svg/<name>.svg")`.
4. Run `cargo test -p inputforge-gui-dx --lib icons::tests`; the well-formedness test will catch corrupt files.

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
`style:`; these primitives intentionally don't model `display: grid`.

## Status backgrounds

Use the `--color-{info,success,warning,error}-bg` tokens for tinted status
surfaces (Badge, Toast). Never embed `rgba()` literals in component CSS; the
revised palette would silently drift from the foreground/border tokens.

## Reduced motion

`motion.css` zeroes the `--duration-*` tokens under
`@media (prefers-reduced-motion: reduce)`. Component CSS that pipes
animation duration through these tokens disables motion automatically;
component CSS that hard-codes `ms` in transition shorthands does not; keep
all timing in tokens.

## Toolchain prerequisites

- `dx` (dioxus-cli) version 0.7.6: install via `cargo install dioxus-cli --version 0.7.6`. Required for hot-reload (`dx serve`).
- WebView2 runtime: bundled with Windows 11. On Windows 10 or earlier, install the Evergreen Standalone runtime from https://developer.microsoft.com/microsoft-edge/webview2/.

## SDL3.dll placement

Windows builds need `SDL3.dll` next to the executable. `crates/inputforge-app/build.rs`
copies it from `<workspace>/SDL/SDL3.dll` into both:

- `target/<cargo-profile>/SDL3.dll`, alongside the `cargo` binary
- `target/dx/inputforge-app/<dx-profile>/windows/app/SDL3.dll`, alongside the `dx` binary

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
treats *missing* destination files as untracked rather than changed; so if
you `rm -rf target/dx` and the next `dx run` fails with
`STATUS_DLL_NOT_FOUND` (`0xC0000135`), the script did not re-run because
cargo's fingerprint of `inputforge-app` is still fresh. Force a rerun with:

```bash
cargo clean -p inputforge-app
dx run -p inputforge-app
```

A full `cargo clean` is unnecessary; the per-package clean is enough.

## F3: Tray bridge & hide-to-tray lifecycle

Tray construction, native event dispatch, labels, availability policy and desktop
window lifecycle belong entirely to `inputforge-app`. This crate exposes
`render_editor` and `EditorNavigation`, ordinary presentation state shared with
panel controls. `render_editor` also requires `routing::Bindings`: the app's
pure routing projection and revalidating action callback. The existing snapshot
bridge publishes that projection; it does not add a polling loop. The seeded
demo supplies fixed presentation data and an inert callback.

Routing eligibility, Start/Stop/Retry/Refresh dispatch, and review navigation
belong to the app's shared `routing` module. GUI controls consume its prepared
model. Controller configuration editing and other audited workflows still
remain in this crate; see the [boundary audit](../../docs/gui-rendering-boundary-audit.md).
It has no direct muda, GTK timer or Windows API dependency.

The app configures hidden startup before constructing the window and dispatches
tray actions independently of WebView render acknowledgements. Its event-loop
exit callback stops and joins the engine before Tao terminates the process.
See [Linux routing and tray acceptance](../../docs/linux-routing.md#tray-and-background-acceptance)
for policy, regression fixtures and native acceptance checks.

### New primitives (F3)

- `Tabs`: full WAI-ARIA Tabs pattern (`role="tablist"`/`tab`,
  focus-roving with `tabindex` 0|-1, arrow keys + Home/End for cycle
  and activate). Each tab button gets `id="tab-{id}"`; set
  `TabItem::controls` to wire `aria-controls` at the consumer's
  tabpanel `id` (the consumer renders the `role="tabpanel"` div with
  matching `id` and an `aria-labelledby` back-reference to `tab-{id}`).
  Focus is moved imperatively via `MountedData::set_focus` after each
  keyboard activation so the focus ring follows selection. Stateless:
  caller owns `value`. Reused by F11 (Modes); see `examples/component_gallery.rs`
  for the canonical full-pattern wiring.
- `StatusBar`: three-slot horizontal bar (start / middle / end). Fixed
  28px height. ARIA-neutral wrapper; consumers add `role="status"` /
  `aria-live` only on the specific elements they want announced.
  Currently exercised only by the component gallery example; the
  application frame mounts no status bar of its own.

### F7 frame layout

The application chrome lives under `frame/`: top bar (engine pill,
profile name, mode tabs, tools cluster), conditional banner
(Diverged / Forced / ForcedAndDiverged), and main row (mapping list
rail + editor center + right panel slot).
Each region is a Dioxus `#[component]` with paired pure-logic in
`frame/<region>/logic.rs` so render-free unit tests cover state
derivation. The shared chrome state (`editing_mode`, `panel_slot`,
`via_calibration`) is provided by `ViewState` from
`frame::use_view_state_provider` and read via `use_context::<ViewState>`.
Mount happens in `app_root_view()` (not `app_root` directly), so the
SSR mount-regression test in `app::tests` can render the same rsx
tree with stub contexts.

## F4: Toast & Dialog Infrastructure

### `ToastQueue`

Global toast queue installed via `use_context_provider` in `app_root`. Producers
reach it with:

```rust
use inputforge_gui_dx::{ToastLevel, ToastQueue};

let toasts = use_context::<ToastQueue>();
toasts.push(ToastLevel::Warning, "HidHide unavailable");
```

- **Levels:** `Info`, `Success`, `Warning`, `Error`. Info/Success render in
  `role="status" aria-live="polite"`; Warning/Error in
  `role="alert" aria-live="assertive"`.
- **Dedupe:** identical `(level, message)` against any non-dismissed toast
  increments its count (`×N` badge) and resets the auto-dismiss timer.
- **Cap:** at most 5 visible toasts; FIFO drain of the oldest non-dismissed
  entry on overflow.
- **Auto-dismiss:** 8 s. Hover or focus on a toast pauses the timer; ESC while
  focused dismisses; click × dismisses.

### Dialog primitive

Compound API on the native `<dialog>` element:

```rust
use dioxus::prelude::*;
use inputforge_gui_dx::components::{
    DialogBody, DialogDescription, DialogFooter, DialogRoot, DialogTitle,
};

let mut open = use_signal(|| false);

rsx! {
    DialogRoot {
        open: open,
        onclose: move |()| {},
        DialogTitle { "Title" }
        DialogDescription { "Body description (rendered in a <p>)." }
        DialogBody { /* optional scrollable region */ }
        DialogFooter { /* action buttons */ }
    }
}
```

- **`dismissible: bool` (default `true`)**: when `false`, ESC is suppressed.
  Read once at mount; flipping after mount has no effect.
- **`close_on_backdrop_click: bool` (default `false`)**: backdrop click
  resolves the dialog when `true`.
- Native `<dialog>` provides focus trap, inert background, `aria-modal`, and
  focus restore on close.
- **`onclose` semantics:** Dioxus 0.7 has no `onclose` event for `<dialog>`;
  the prop fires on ESC dismissal (when `dismissible: true`) and on backdrop
  click (when `close_on_backdrop_click: true`). Programmatic `open.set(false)`
  closes the dialog but does NOT fire `onclose`; consumer-driven flows should
  call their own callbacks alongside `open.set(false)`.

### `DirtyConfirmDialog`

Presentational reusable composing the dialog primitives with default copy and
Cancel/Discard/Save buttons. Cancel-first for default focus; ESC routes to
`oncancel`; `close_on_backdrop_click: false`.

```rust
use inputforge_gui_dx::patterns::DirtyConfirmDialog;

let mut open = use_signal(|| false);

rsx! {
    DirtyConfirmDialog {
        open: open,
        oncancel:  move |()| { /* abort the action; distinct from discard */ },
        ondiscard: move |()| { /* drop unsaved changes, proceed */ },
        onsave:    move |()| { /* persist, then proceed */ },
    }
}
```

Override `title`, `message`, and `save_label` for context-specific phrasing
(e.g., `save_label: Some("Save & Switch".to_owned())`).

### Warnings bridge

`MetaSnapshot.warnings` (populated by the engine via the F1 polling task)
flows into the toast queue as Warning-level toasts. Producers do not need to
opt in; append to `AppState.warnings` and the bridge handles delivery.
