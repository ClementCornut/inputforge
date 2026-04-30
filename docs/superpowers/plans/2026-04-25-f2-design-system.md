# F2, Design System & Theme Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `docs/superpowers/specs/2026-04-25-f2-design-system-design.md`

**Goal:** Establish the visual language and 17-primitive component library for the Dioxus rewrite of `inputforge-gui`, replacing F1's inline-styled placeholder with a token-driven, theme-ready system gated through a `ThemeProvider`.

**Architecture:** Six token CSS files (colors / typography / spacing / radii / elevation / motion) + a `global.css` baseline, mounted by a `ThemeProvider` Dioxus component using `document::Stylesheet` + `manganis::asset!()`. Component primitives live one-per-file in `src/components/` with sibling CSS in `assets/components/`, all sharing a BEM-ish `.if-<name>` class prefix and a `class: Option<String>` caller-composition prop. Icons stored as raw SVG files, included via `include_str!()` and exposed through an `Icon` enum. A standalone `examples/component_gallery.rs` mounts the provider and demos every primitive, it doubles as the visual smoke harness from Phase 1 onward.

**Tech Stack:** Rust 2024 / rustc 1.85, Dioxus 0.7.6 (desktop / wry / WebView2), `manganis` (transitive via Dioxus 0.7), CSS variables, Phosphor Icons (MIT), Inter + JetBrains Mono fonts (OFL-1.1).

---

## Context

F1 (`docs/superpowers/plans/2026-04-25-f1-dioxus-scaffold-state-bridge.md`) shipped a state-bridge scaffold for `crates/inputforge-gui-dx`: signals, 60 Hz polling task, `AppContext` provider, and a single inline-styled `F1Readout` placeholder. There is **no** styling infrastructure yet, no CSS file, no fonts, no design tokens, no reusable components.

F2 is the second sequential foundation step. Every subsequent feature (F3 shell through F14 profile surface) builds on F2's tokens and primitives, so F2 must be:

- **Theme-ready** (semantic naming so a future light theme is an override block, not a refactor)
- **Complete** in primitives so later screens compose without revisiting F2
- **Production-grade** in personality, first impression of the new GUI

The visual direction is "Evolved Glass Cockpit." The existing egui identity (dark navy `#1A1A2E` base, semantic accent quintet, Inter + JetBrains Mono, 8/4 spacing, 6px radius) is the baseline, but `impeccable:frontend-design` is **explicitly invited to challenge every aspect** mid-plan (Phase 2). Component CSS is written **after** frontend-design output is applied to the token files, to avoid rework.

Outcome at F2: under `--features gui-dioxus` the rewritten `F1Readout` renders the same six fields it did at F1 (status, mode, profile, devices, virtual devices, warnings) but composed from `Card` + `Badge` + new typography. `dx serve --example component_gallery --platform desktop` opens a window showing every variant of all 17 primitives. Default features (`gui-egui`) remain untouched.

---

## Critical Files To Modify

All paths relative to `E:\Git\Perso\inputforge\` unless otherwise noted.

**Created (in `crates/inputforge-gui-dx/`):**

- `assets/tokens/{colors,typography,spacing,radii,elevation,motion}.css`
- `assets/global.css`
- `assets/components/{button,icon-button,text-input,number-input,select,slider,switch,checkbox,card,badge,tooltip,menu,separator,field,label,spinner,icon}.css` (17 files)
- `assets/fonts/{Inter-Regular,Inter-SemiBold,JetBrainsMono-Regular}.ttf` (copied from `crates/inputforge-gui/assets/fonts/`)
- `src/theme/mod.rs`, `ThemeProvider` component, `Asset` constants for every CSS file
- `src/components/mod.rs`, re-exports
- `src/components/{button,icon_button,text_input,number_input,select,slider,switch,checkbox,card,badge,tooltip,menu,separator,field,label,spinner,icon}.rs` (17 files)
- `src/icons/mod.rs`, `Icon` enum + `svg()` impl + `IconSize` enum
- `src/icons/svg/*.svg` (~30 Phosphor SVGs)
- `examples/component_gallery.rs`, visual harness for all 17 primitives
- `THIRD_PARTY_LICENSES.md`, Phosphor MIT + Inter OFL-1.1 + JetBrains Mono OFL-1.1 attributions

**Modified (in `crates/inputforge-gui-dx/`):**

- `src/lib.rs`, add `pub mod theme;` + `pub mod components;` + `pub mod icons;`
- `src/app.rs`, wrap `app_root`'s rsx output in `ThemeProvider`; rewrite `F1Readout` using primitives
- `README.md`, gallery example, `ThemeProvider` usage, how to add a new icon, `dx serve` + WebView2 prerequisites
- `Cargo.toml`, likely **no change** (manganis is transitive via dioxus 0.7.6); only add a direct `manganis` dep if Task 1 step 4 reveals it isn't reachable

**Reused (do not modify):**

- `src/context.rs`, `AppContext`, `MetaSnapshot`, `ConfigSnapshot`, `LiveSnapshot`
- `src/bridge.rs`, polling task
- F1 `launch_gui` signature in `lib.rs`

---

## Existing Utilities To Reuse

- **Egui font files** at `crates/inputforge-gui/assets/fonts/`, copy `Inter-Regular.ttf`, `Inter-SemiBold.ttf`, `JetBrainsMono-Regular.ttf` verbatim. No reformatting / re-encoding.
- **Egui DARK color palette** for placeholder token values, defined in `crates/inputforge-gui/src/theme.rs::DARK`. Values: base `#1A1A2E`, mantle `#16163A`, surface0 `#2A2A3E`, surface1 `#3A3A4E`, text `#E0E0E8`, text_dim `#A0A0B8`, primary `#4A9EFF`, live `#00E5A0`, warning `#FFB347`, error `#FF6B6B`, special `#B07FFF`, indicator_idle `#555570`.
- **F1 snapshot factories** `MetaSnapshot::from_state(&AppState)` and `ConfigSnapshot::from_state(&AppState)` for the `F1Readout` regression test in Task 21 (`crates/inputforge-gui-dx/src/context.rs`).
- **F1 `bridge_demo.rs` shape** at `crates/inputforge-gui-dx/examples/bridge_demo.rs`, model `examples/component_gallery.rs` after it (no engine, no I/O, hot-reload safe).
- **F1 build matrix** documented in `crates/inputforge-gui-dx/README.md`, verification commands `dx serve --example bridge_demo --platform desktop` and `cargo build -p inputforge-app --no-default-features --features gui-dioxus` are reused below.

---

## Dioxus 0.7 / `manganis` Footguns To Heed

Surface these in the implementer's mind before they hit them:

- **`asset!()` paths must start with `/`** and are resolved relative to the **crate root**, not workspace root. On Windows, easy to mis-slash, use forward slashes inside the macro string.
- **`document::Stylesheet { href: ASSET }` mounts in render order, not declaration order.** The order of `Stylesheet { … }` calls inside `ThemeProvider`'s rsx! determines cascade priority. Locked order: tokens (colors → typography → spacing → radii → elevation → motion) → `global.css` → component files. Comment this in `theme/mod.rs`.
- **`children: Element` is required by default**; use `Option<Element>` only where empty children are valid (Tooltip's wrapped trigger, Field's helper area). For most primitives `children: Element` is correct.
- **`EventHandler<T>` already encodes optionality** (`.call(evt)` is idempotent if no handler attached). Do **not** wrap in `Option<EventHandler<_>>`.
- **`ReadOnlySignal<T>` for read-only reactive props**, use for `value` props on inputs (TextInput, NumberInput, Slider, Switch, Checkbox) to avoid spurious clones. Spec doesn't mention this; it's a real 0.7 ergonomics gain.
- **`dangerous_inner_html` argument type** in Dioxus 0.7.6, verify with a single-line probe before scaling out the icon registry: it may want `String` not `&'static str`. Adjust the icon component to `.to_string()` if so.
- **`include_str!()` icon SVGs do not hot-reload**, editing an SVG triggers a rebuild, not a Dioxus refresh. Document in README.
- **`manganis::asset!` is re-exported via `dioxus::prelude::asset!`** in 0.7. Use the prelude form to avoid an extra direct dep.
- **`ReadOnlySignal<T>` props accept `T` directly via blanket `From<T>`.** The gallery exercises this implicit conversion (e.g., `TextInput { value: "hello".to_string() }`). If you see an `Into`/`From` error at a call site, the impl moved or specialization changed, workaround: wrap the call site with `Signal::new(...)` or `value.into()`.

---

## Phase Overview

- **Phase 0** (Tasks 1-5), Scaffolding & legal: directory skeleton, fonts, ThemeProvider shell, gallery skeleton, license attribution.
- **Phase 1** (Tasks 6-8), Token CSS files with placeholder values + asset-pipeline smoke test (proves the foundation works before scaling).
- **Phase 2** (Tasks 9-10), Frontend-design invocation & token finalization (locks visual values before component CSS is written).
- **Phase 3** (Tasks 11-13), Icon system: SVG files, `Icon` enum, `Icon` component.
- **Phase 4** (Tasks 14-20), Component primitives, grouped by behavioral family (7 tasks; Task 17 splits into 17a + 17b for risk isolation, see Task 17 below). Each appends a section to the gallery.
- **Phase 5** (Task 21), `F1Readout` rewrite using primitives, with data-binding regression test.
- **Phase 6** (Tasks 22-23), README updates and final verification.

---

## Task 1: Create directory skeleton and copy fonts

**Files:**
- Create directories: `crates/inputforge-gui-dx/assets/tokens/`, `crates/inputforge-gui-dx/assets/components/`, `crates/inputforge-gui-dx/assets/fonts/`, `crates/inputforge-gui-dx/src/theme/`, `crates/inputforge-gui-dx/src/components/`, `crates/inputforge-gui-dx/src/icons/svg/`
- Copy: `crates/inputforge-gui/assets/fonts/Inter-Regular.ttf`, `Inter-SemiBold.ttf`, `JetBrainsMono-Regular.ttf` → `crates/inputforge-gui-dx/assets/fonts/`

- [ ] **Step 1: Create directory tree**

```bash
mkdir -p crates/inputforge-gui-dx/assets/tokens
mkdir -p crates/inputforge-gui-dx/assets/components
mkdir -p crates/inputforge-gui-dx/assets/fonts
mkdir -p crates/inputforge-gui-dx/src/theme
mkdir -p crates/inputforge-gui-dx/src/components
mkdir -p crates/inputforge-gui-dx/src/icons/svg
```

- [ ] **Step 2: Copy three font files**

```bash
cp crates/inputforge-gui/assets/fonts/Inter-Regular.ttf      crates/inputforge-gui-dx/assets/fonts/
cp crates/inputforge-gui/assets/fonts/Inter-SemiBold.ttf     crates/inputforge-gui-dx/assets/fonts/
cp crates/inputforge-gui/assets/fonts/JetBrainsMono-Regular.ttf crates/inputforge-gui-dx/assets/fonts/
```

Verify: `ls crates/inputforge-gui-dx/assets/fonts/` lists all three TTFs.

- [ ] **Step 3: Verify `manganis` is reachable transitively**

Run: `cargo tree -p inputforge-gui-dx --depth 5 | grep manganis`
Expected: at least one line mentioning `manganis v0.7.x` (transitive through `dioxus`). The Cargo.lock check during F1 explored confirmed transitive presence; this step exists to fail-fast if that changes.

If the grep returns nothing, add manganis as a direct dep. This requires **two atomic edits in a single commit**, the workspace declaration must exist before the crate can reference `{ workspace = true }`:

1. Root `Cargo.toml` `[workspace.dependencies]`:
   ```toml
   manganis = "0.7"
   ```
   (Match the dioxus minor.)
2. Crate `crates/inputforge-gui-dx/Cargo.toml` `[dependencies]`:
   ```toml
   manganis = { workspace = true }
   ```

Then re-run `cargo tree -p inputforge-gui-dx --depth 5 | grep manganis` to confirm it now appears.

- [ ] **Step 4: Verify workspace still compiles**

Run: `cargo build -p inputforge-gui-dx`
Expected: builds cleanly (we've added zero code, only directories and assets).

- [ ] **Step 5: Commit**

Empty directories created in step 1 are not tracked by git; they will materialize as files land in subsequent tasks. This commit therefore stages only the three TTFs copied in step 2 (and any Cargo.toml edits from step 3's fallback, if applied).

Invoke the `conventional-commits` skill, then:

```bash
git add crates/inputforge-gui-dx/assets/fonts
# also stage Cargo.toml edits if step 3's fallback was applied:
# git add Cargo.toml crates/inputforge-gui-dx/Cargo.toml
git commit -m "feat(gui-dx): copy F2 font assets (Inter + JetBrains Mono)"
```

---

## Task 2: lib.rs module declarations + ThemeProvider skeleton

**Files:**
- Modify: `crates/inputforge-gui-dx/src/lib.rs`, add three `pub mod` declarations
- Create: `crates/inputforge-gui-dx/src/theme/mod.rs`, empty `ThemeProvider` (passes children through, no stylesheets)
- Create: `crates/inputforge-gui-dx/src/components/mod.rs`, empty (placeholder for re-exports)
- Create: `crates/inputforge-gui-dx/src/icons/mod.rs`, empty (placeholder for `Icon` enum)

- [ ] **Step 1: Edit `src/lib.rs`**

Insert immediately after the existing `mod app;` / `mod bridge;` / `mod context;` lines (top of file, before the `use` block):

```rust
pub mod theme;
pub mod components;
pub mod icons;
```

These are `pub` (not `pub(crate)`) so the `examples/component_gallery.rs` binary can import `ThemeProvider`, `Icon`, and primitives. The `launch_gui` public surface is unchanged.

- [ ] **Step 2: Write `src/theme/mod.rs`**

```rust
//! Mounts the F2 design-system stylesheets and exposes them to descendants.
//!
//! Stylesheet load order (cascade priority, lowest first):
//! tokens → global → components. Order matters, do not reshuffle.

use dioxus::prelude::*;

#[component]
pub fn ThemeProvider(children: Element) -> Element {
    rsx! { {children} }
}
```

- [ ] **Step 3: Write `src/components/mod.rs`**

```rust
//! Re-exports for the F2 component primitives. Populated as primitives land.
```

- [ ] **Step 4: Write `src/icons/mod.rs`**

```rust
//! Icon enum + SVG registry. Populated in Task 12.
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo build -p inputforge-gui-dx`
Expected: builds cleanly.

- [ ] **Step 6: Commit**

Invoke the `conventional-commits` skill, then commit:
```
feat(gui-dx): add theme/components/icons modules and ThemeProvider skeleton
```

---

## Task 3: Wire empty ThemeProvider into `app_root`

Wrap the existing `F1Readout` in `ThemeProvider`. No styling change yet, this just lands the integration point.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/app.rs`

- [ ] **Step 1: Update the `use` block in `src/app.rs`**

Add to the existing imports near the top:

```rust
use crate::theme::ThemeProvider;
```

- [ ] **Step 2: Wrap `F1Readout` in `app_root`**

Replace the existing final `rsx! { F1Readout {} }` (last line of `app_root`) with:

```rust
    rsx! {
        ThemeProvider {
            F1Readout {}
        }
    }
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build -p inputforge-gui-dx`
Expected: builds cleanly.

- [ ] **Step 4: Verify gui-dioxus app still compiles**

Run: `cargo build -p inputforge-app --no-default-features --features gui-dioxus`
Expected: builds cleanly.

- [ ] **Step 5: Commit**

Invoke the `conventional-commits` skill, then commit:
```
feat(gui-dx): wrap F1Readout in ThemeProvider scaffold
```

---

## Task 4: Create THIRD_PARTY_LICENSES.md

Lands legal compliance **before** any font or icon ships. Required by spec verification check #11.

**Files:**
- Create: `crates/inputforge-gui-dx/THIRD_PARTY_LICENSES.md`

- [ ] **Step 1: Write the attribution file**

Write `crates/inputforge-gui-dx/THIRD_PARTY_LICENSES.md`:

```markdown
# Third-Party Licenses

This crate bundles assets governed by upstream licenses. They are reproduced below in summary; see `THIRD_PARTY_LICENSES_FULL/` (if present) or the upstream sources for full text.

## Phosphor Icons, MIT License

Copyright (c) 2020 Phosphor Icons

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND.

Source: https://github.com/phosphor-icons/core

## Inter, SIL Open Font License 1.1

Copyright (c) 2016 The Inter Project Authors (https://github.com/rsms/inter)

This Font Software is licensed under the SIL Open Font License, Version 1.1.
This license is copied below, and is also available with a FAQ at:
https://openfontlicense.org

Source: https://github.com/rsms/inter

## JetBrains Mono, SIL Open Font License 1.1

Copyright 2020 The JetBrains Mono Project Authors (https://github.com/JetBrains/JetBrainsMono)

This Font Software is licensed under the SIL Open Font License, Version 1.1.
This license is copied below, and is also available with a FAQ at:
https://openfontlicense.org

Source: https://github.com/JetBrains/JetBrainsMono
```

- [ ] **Step 2: Commit**

Invoke the `conventional-commits` skill, then commit:
```
docs(gui-dx): add THIRD_PARTY_LICENSES.md for Phosphor + Inter + JetBrains Mono
```

---

## Task 5: Component gallery skeleton

Lands the visual smoke harness that Phases 1-4 will use. Mounts `ThemeProvider` directly (no engine, no signals); each subsequent primitive task appends a `<section>`.

**Files:**
- Create: `crates/inputforge-gui-dx/examples/component_gallery.rs`

- [ ] **Step 1: Write the gallery skeleton**

```rust
//! Visual harness for all F2 primitives.
//!
//! Run via:
//!     dx serve --example component_gallery --platform desktop
//!
//! Mounts `ThemeProvider` directly, no engine state required.
//! Hot-reload friendly: editing CSS or RSX updates instantly.

use dioxus::prelude::*;
use inputforge_gui_dx::theme::ThemeProvider;

fn main() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    LaunchBuilder::desktop().launch(gallery_root);
}

fn gallery_root() -> Element {
    rsx! {
        ThemeProvider {
            main {
                style: "padding: 24px;",
                h1 { "InputForge, Component Gallery (F2)" }
                p { "Primitives appear in sections below as Phase 4 lands them." }
            }
        }
    }
}
```

The inline `padding: 24px;` style is intentional and temporary, it gives the page chrome before `global.css` exists. It's removed once `Card` is available (Task 17, Card+Badge+Separator+Spinner task) and the gallery sections wrap each primitive in a `Card`.

- [ ] **Step 2: Verify the example compiles**

Run: `cargo build -p inputforge-gui-dx --example component_gallery`
Expected: builds cleanly.

- [ ] **Step 3: Smoke-run the gallery**

Run: `dx serve --example component_gallery --platform desktop`
Expected: a window opens showing the heading "InputForge, Component Gallery (F2)" and the placeholder paragraph. Close the window.

(If `dx` is not installed, run `cargo install dioxus-cli --version 0.7.6` first.)

- [ ] **Step 4: Commit**

Invoke the `conventional-commits` skill, then commit:
```
feat(gui-dx): add component_gallery example skeleton
```

---

## Task 6: Token CSS files with placeholder values

Six token CSS files seeded with values from the egui DARK palette. These will be **revised** in Task 10 after frontend-design output.

**Files:**
- Create: `crates/inputforge-gui-dx/assets/tokens/colors.css`
- Create: `crates/inputforge-gui-dx/assets/tokens/typography.css`
- Create: `crates/inputforge-gui-dx/assets/tokens/spacing.css`
- Create: `crates/inputforge-gui-dx/assets/tokens/radii.css`
- Create: `crates/inputforge-gui-dx/assets/tokens/elevation.css`
- Create: `crates/inputforge-gui-dx/assets/tokens/motion.css`

- [ ] **Step 1: Write `assets/tokens/colors.css`**

```css
/* Dark theme is the default. A future light theme can be added as
   :root[data-theme="light"] { ... } without touching component CSS. */
:root {
    /* Surface */
    --color-bg:           #1A1A2E;
    --color-bg-elevated:  #2A2A3E;
    --color-bg-sunken:    #121228;
    --color-bg-overlay:   rgba(18, 18, 40, 0.85);

    /* Text */
    --color-text:         #E0E0E8;
    --color-text-muted:   #A0A0B8;
    --color-text-subtle:  #707088;
    --color-text-inverse: #1A1A2E;

    /* Border */
    --color-border:        #3A3A4E;
    --color-border-strong: #555570;
    --color-border-focus:  #4A9EFF;

    /* Action */
    --color-primary:        #4A9EFF;
    --color-primary-hover:  #6BB0FF;
    --color-primary-active: #2E7DD9;
    --color-primary-fg:     #0A1020;

    /* Status */
    --color-live:    #00E5A0;
    --color-warning: #FFB347;
    --color-error:   #FF6B6B;
    --color-info:    #4A9EFF;

    /* Categories, three INDEPENDENT accent hues (NOT aliased to
       action/status). Frontend-design (Task 10) will pick final values. */
    --color-processing:    #4A9EFF;   /* placeholder */
    --color-processing-bg: rgba(74, 158, 255, 0.12);
    --color-processing-fg: #0A1020;

    --color-output:        #00E5A0;   /* placeholder */
    --color-output-bg:     rgba(0, 229, 160, 0.12);
    --color-output-fg:     #0A1020;

    --color-control:       #B07FFF;   /* placeholder */
    --color-control-bg:    rgba(176, 127, 255, 0.12);
    --color-control-fg:    #0A1020;
}
```

- [ ] **Step 2: Write `assets/tokens/typography.css`**

`@font-face` `src` paths use absolute crate-relative paths. Dioxus 0.7's asset pipeline serves anything under `/assets/` to the WebView; no `asset!()` macro is required inside the CSS itself (the macro lives in `theme/mod.rs` for the stylesheet `href`).

```css
@font-face {
    font-family: 'Inter';
    src: url('/assets/fonts/Inter-Regular.ttf') format('truetype');
    font-weight: 400;
    font-display: swap;
}

@font-face {
    font-family: 'Inter';
    src: url('/assets/fonts/Inter-SemiBold.ttf') format('truetype');
    font-weight: 600;
    font-display: swap;
}

@font-face {
    font-family: 'JetBrainsMono';
    src: url('/assets/fonts/JetBrainsMono-Regular.ttf') format('truetype');
    font-weight: 400;
    font-display: swap;
}

:root {
    --font-sans: 'Inter', system-ui, -apple-system, sans-serif;
    --font-mono: 'JetBrainsMono', ui-monospace, 'Cascadia Code', monospace;

    --text-xs:   12px;
    --text-sm:   13px;
    --text-base: 14px;
    --text-md:   15px;
    --text-lg:   18px;
    --text-xl:   22px;
    --text-2xl:  28px;

    --weight-regular:  400;
    --weight-medium:   500;
    --weight-semibold: 600;

    --leading-tight:   1.2;
    --leading-base:    1.5;
    --leading-relaxed: 1.7;
}
```

- [ ] **Step 3: Write `assets/tokens/spacing.css`**

```css
:root {
    --space-0:  0;
    --space-1:  4px;
    --space-2:  8px;
    --space-3:  12px;
    --space-4:  16px;
    --space-6:  24px;
    --space-8:  32px;
    --space-12: 48px;
    --space-16: 64px;
}
```

- [ ] **Step 4: Write `assets/tokens/radii.css`**

```css
:root {
    --radius-none: 0;
    --radius-sm:   3px;
    --radius-md:   6px;
    --radius-lg:   10px;
    --radius-full: 9999px;
}
```

- [ ] **Step 5: Write `assets/tokens/elevation.css`**

```css
:root {
    --shadow-0: none;
    --shadow-1: 0 1px 2px rgba(0, 0, 0, 0.30);
    --shadow-2: 0 2px 8px rgba(0, 0, 0, 0.35);
    --shadow-3: 0 8px 24px rgba(0, 0, 0, 0.45);
}
```

- [ ] **Step 6: Write `assets/tokens/motion.css`**

```css
:root {
    --duration-fast: 120ms;
    --duration-base: 180ms;
    --duration-slow: 260ms;

    --easing-standard:    cubic-bezier(0.2, 0, 0, 1);
    --easing-emphasized:  cubic-bezier(0.3, 0, 0, 1);
}
```

- [ ] **Step 7: Commit**

Invoke the `conventional-commits` skill, then commit:
```
feat(gui-dx): add token CSS files with placeholder values from egui dark palette
```

---

## Task 7: global.css baseline

Box-sizing reset, body baseline, scrollbar styling. Loaded **after** all tokens, **before** any component CSS.

**Files:**
- Create: `crates/inputforge-gui-dx/assets/global.css`

- [ ] **Step 1: Write `assets/global.css`**

```css
*, *::before, *::after { box-sizing: border-box; }

body {
    margin: 0;
    background: var(--color-bg);
    color: var(--color-text);
    font-family: var(--font-sans);
    font-size: var(--text-base);
    line-height: var(--leading-base);
}

/* Components opt in to spacing via tokens; no implicit heading/paragraph margins. */
h1, h2, h3, h4, h5, h6, p { margin: 0; }

/* Custom scrollbar, Webkit + Firefox. */
::-webkit-scrollbar       { width: 10px; height: 10px; }
::-webkit-scrollbar-track { background: var(--color-bg-elevated); }
::-webkit-scrollbar-thumb { background: var(--color-border-strong); border-radius: var(--radius-sm); }

* {
    scrollbar-color: var(--color-border-strong) var(--color-bg-elevated);
    scrollbar-width: thin;
}
```

- [ ] **Step 2: Commit**

Invoke the `conventional-commits` skill, then commit:
```
feat(gui-dx): add global.css baseline (box-sizing, body, scrollbars)
```

---

## Task 8: Wire token + global CSS into ThemeProvider; asset-pipeline smoke test

Mounts the seven CSS files and proves the asset pipeline end-to-end. This is spec verification check #10 executed early (fail-fast).

**Files:**
- Modify: `crates/inputforge-gui-dx/src/theme/mod.rs`
- Modify: `crates/inputforge-gui-dx/src/app.rs`, drop the inline `font-family` / `background` / `color` overrides on the `F1Readout`'s `<main>` so the body baseline shows through
- Modify: `crates/inputforge-gui-dx/examples/component_gallery.rs`, drop the temporary `padding: 24px;` inline style (now use `var(--space-6)`)

- [ ] **Step 1: Replace `src/theme/mod.rs`**

```rust
//! Mounts the F2 design-system stylesheets and exposes them to descendants.
//!
//! Stylesheet load order (cascade priority, lowest first):
//!     tokens → global → components.
//! `document::Stylesheet` mounts in render order, so the rsx! sequence
//! below IS the cascade order. Do not reshuffle.

use dioxus::prelude::*;

const COLORS_CSS:     Asset = asset!("/assets/tokens/colors.css");
const TYPOGRAPHY_CSS: Asset = asset!("/assets/tokens/typography.css");
const SPACING_CSS:    Asset = asset!("/assets/tokens/spacing.css");
const RADII_CSS:      Asset = asset!("/assets/tokens/radii.css");
const ELEVATION_CSS:  Asset = asset!("/assets/tokens/elevation.css");
const MOTION_CSS:     Asset = asset!("/assets/tokens/motion.css");
const GLOBAL_CSS:     Asset = asset!("/assets/global.css");

#[component]
pub fn ThemeProvider(children: Element) -> Element {
    rsx! {
        // Tokens first (lowest cascade priority).
        document::Stylesheet { href: COLORS_CSS }
        document::Stylesheet { href: TYPOGRAPHY_CSS }
        document::Stylesheet { href: SPACING_CSS }
        document::Stylesheet { href: RADII_CSS }
        document::Stylesheet { href: ELEVATION_CSS }
        document::Stylesheet { href: MOTION_CSS }

        // Body baseline.
        document::Stylesheet { href: GLOBAL_CSS }

        // Component CSS will be appended here as primitives land (Tasks 13-20).

        {children}
    }
}
```

- [ ] **Step 2: Strip inline styles from `F1Readout` in `src/app.rs`**

Locate the `<main style: "..." >` line in `F1Readout`. Replace the `style:` attribute with one that only sets layout (padding/min-height) using tokens; let body baseline handle font and background:

```rust
        main {
            style: "padding: var(--space-6); min-height: 100vh;",
            h1 { "InputForge, Dioxus (F1 bridge smoke test)" }
            // ... rest unchanged ...
        }
```

The `font-family: system-ui` and `background: #1A1A2E` and `color: #ddd` are removed, they now come from `body` via `global.css`.

- [ ] **Step 3: Update gallery to use spacing token**

In `examples/component_gallery.rs`, change `style: "padding: 24px;"` to:

```rust
                style: "padding: var(--space-6);",
```

- [ ] **Step 4: Build to confirm compile**

Run: `cargo build -p inputforge-gui-dx --example component_gallery`
Expected: builds cleanly.

- [ ] **Step 5: Smoke test the asset pipeline**

Run: `dx serve --example component_gallery --platform desktop`

Expected:
- Window opens with the dark navy background (`#1A1A2E`), proves `colors.css` loaded.
- Heading "InputForge, Component Gallery (F2)" renders in **Inter** (not system-ui), proves `typography.css` and the font asset both loaded.
- Custom thin scrollbar appears if you resize the window narrow enough, proves `global.css` loaded.

If the font fallback (system-ui) is showing, inspect the DevTools Network tab for the TTF request. If it 404s, double-check the `@font-face` `src: url(...)` paths in `typography.css`, they must start with `/assets/fonts/`.

- [ ] **Step 6: DevTools-based verification (spec check #10)**

With the gallery still running, attach DevTools (right-click → Inspect, or Ctrl+Shift+I in dev builds) and run in the Console:

```javascript
getComputedStyle(document.body).getPropertyValue('--color-bg')
// expect: " #1A1A2E" (or similar non-empty)

getComputedStyle(document.body).fontFamily
// expect: includes "Inter"
```

Both must resolve. If either is empty or the font is `system-ui`, the asset pipeline is broken, stop and diagnose before proceeding.

Also, in the same DevTools console, confirm the **stylesheet cascade order** matches the rsx render order:

```javascript
Array.from(document.head.querySelectorAll('link[rel=stylesheet]')).map(l => l.href)
```

Expected: an array of 7 hrefs in this order, `colors.css`, `typography.css`, `spacing.css`, `radii.css`, `elevation.css`, `motion.css`, `global.css`.

If the order is scrambled, `document::Stylesheet` doesn't preserve rsx render order in this Dioxus version, and the locked cascade (token → global → component) is unreliable. Mitigation: replace the multi-`Stylesheet` setup in `theme/mod.rs` with a single inlined `document::Style { ... include_str!("/* concatenated tokens.css */") ... }`, or introduce an `assets/tokens/tokens.css` that `@import`s the six token files in deterministic order. Document the deviation in `theme/mod.rs` so later contributors know why the structure changed.

- [ ] **Step 7: Verify gui-dioxus app still compiles**

Run: `cargo build -p inputforge-app --no-default-features --features gui-dioxus`
Expected: builds cleanly.

- [ ] **Step 8: Commit**

Invoke the `conventional-commits` skill, then commit:
```
feat(gui-dx): wire token + global CSS into ThemeProvider, prove asset pipeline
```

---

## Task 9: Capture egui screenshots for frontend-design

Frontend-design needs to see the existing identity it's evolving away from.

**Files:**
- Create: `docs/superpowers/assets/f2/egui-{main,profile,mapping}.png` (3 screenshots minimum)

- [ ] **Step 1: Build and run the egui GUI**

```bash
cargo run -p inputforge-app --features gui-egui
```

(Default features already include `gui-egui`; the explicit flag is for clarity.)

- [ ] **Step 2: Capture 3-5 screenshots**

Use OS screenshot tool (Win+Shift+S on Windows). Capture at minimum:
- Main view with the device list and engine status
- Profile editor view
- Input mapping detail view

Save under `docs/superpowers/assets/f2/` as `egui-main.png`, `egui-profile.png`, `egui-mapping.png`.

If the directory doesn't exist:
```bash
mkdir -p docs/superpowers/assets/f2
```

- [ ] **Step 3: Commit screenshots**

Invoke the `conventional-commits` skill, then commit:
```
docs(superpowers): add egui screenshots for F2 frontend-design brief
```

---

## Task 10: Invoke `impeccable:frontend-design` and apply revised tokens

Locks visual values **before** any component CSS is written.

**Files:**
- Modify: `crates/inputforge-gui-dx/assets/tokens/{colors,typography,spacing,radii,elevation,motion}.css` (whichever frontend-design revises)

- [ ] **Step 1: Invoke the skill**

Use the `Skill` tool with `impeccable:frontend-design`. Provide this brief verbatim:

> **Task: Finalize F2 design tokens for InputForge (Dioxus rewrite).**
>
> **Context:** Replacing an egui-based desktop GUI with Dioxus. The product is a serious sim/HOTAS configuration tool for expert users, instrument-cluster heritage. Visual direction: "Evolved Glass Cockpit." Evolve the existing identity, don't replicate it.
>
> **Inputs:**
> - Design spec: `docs/superpowers/specs/2026-04-25-f2-design-system-design.md`
> - Existing egui screenshots: `docs/superpowers/assets/f2/egui-{main,profile,mapping}.png`
> - Current placeholder token files: `crates/inputforge-gui-dx/assets/tokens/{colors,typography,spacing,radii,elevation,motion}.css` (seeded from egui dark palette)
>
> **Constraints (must preserve):**
> - Token NAMES (the API): all token names already in the placeholder files stay. You're revising VALUES, not renaming.
> - Three-category accent system: `--color-processing`, `--color-output`, `--color-control` are reserved as **independent** hues (not aliased to action/status). Pick three hues that complement and don't overlap action/status.
> - Dark theme only in F2 (light theme deferred).
>
> **Explicit permission to challenge:**
> - Color values, accent saturation, type scale, spacing rhythm, motion language, font choice (Inter+JetBrainsMono is the default but not sacred), elevation philosophy, layout.
>
> **Specific questions to answer:**
> 1. Type scale 12/13/14/15/18/22/28 clusters four sizes within 6px, please rationalize. Aim for 5-6 sizes max with clearer hierarchy.
> 2. Pick the three category hues (processing/output/control), must complement action (`--color-primary`) and status (`--color-live` / `--color-warning` / `--color-error`) without visual overlap.
> 3. Confirm or revise `--radius-md: 6px` for the "evolved glass cockpit" aesthetic.
> 4. Elevation philosophy: spec proposes "borders over shadows." Confirm or shift.
> 5. Motion: 120/180/260 ms with cubic-bezier easings, are these right for instrument-cluster aesthetic?
>
> **Output:** revised values for the six token CSS files + a written rationale answering each question. The token NAMES must not change. .impeccable.md.

- [ ] **Step 2: Apply the revised values**

Edit each of the six files in `crates/inputforge-gui-dx/assets/tokens/` with the values frontend-design produced. Keep the token names unchanged.

- [ ] **Step 3: Smoke-test the gallery again**

Run: `dx serve --example component_gallery --platform desktop`
Expected: window opens, body background reflects the revised `--color-bg`, body font reflects the revised `--font-sans` (or new font if frontend-design swapped it). Close.

**If frontend-design swapped one or both font families** (e.g., to a new face like Geist), do all of the following in the same commit:

1. Copy the new TTFs into `assets/fonts/` and update `typography.css`'s `@font-face` rules accordingly.
2. Add the new font's attribution to `crates/inputforge-gui-dx/THIRD_PARTY_LICENSES.md` using the existing entry as a template (font name, foundry, license abbreviation, URL).
3. **Remove** the entry for any font that's no longer used.
4. **Update Task 23 Check #11's grep terms** to match the post-swap font set. The check currently asserts strings "Phosphor", "Inter", "JetBrains Mono"; revise both the asserted strings *and* the corresponding license sections.
5. Delete the unused `.ttf` files from `assets/fonts/`.
6. Re-run the asset-pipeline smoke (Task 8 Step 5+6) to confirm the new font loads.

- [ ] **Step 4: Commit**

Invoke the `conventional-commits` skill, then commit:
```
feat(gui-dx): apply frontend-design revised token values
```

(If new fonts shipped, name them in the commit body.)

---

## Task 11: Add Phosphor SVG files to `src/icons/svg/`

The 30 initial icons listed in the spec.

**Files:**
- Create 30 files under `crates/inputforge-gui-dx/src/icons/svg/`

- [ ] **Step 1: Source Phosphor SVGs**

Phosphor source: https://github.com/phosphor-icons/core (MIT). Use the **regular** weight (24×24 viewBox, 1.5 stroke). Download the latest tagged release (or `git clone` and check out the latest tag) so attribution and content are reproducible. Record the tag in a code comment at the top of `src/icons/mod.rs` in Task 12.

Required icons (filename → Phosphor source name):
- `joystick.svg` → `joystick.svg`
- `device.svg` → `game-controller.svg` (rename to `device.svg`)
- `axis.svg` → `crosshair.svg` (rename), best fit; substitute if a more apt Phosphor icon exists
- `button.svg` → `circle.svg` (rename), substitute if a better fit appears
- `hat.svg` → `arrows-out-cardinal.svg` (rename)
- `mode.svg` → `swap.svg` (rename)
- `profile.svg` → `user.svg` (rename)
- `save.svg` → `floppy-disk.svg` (rename)
- `copy.svg` → `copy.svg`
- `eye.svg` → `eye.svg`
- `eye-off.svg` → `eye-slash.svg` (rename)
- `link.svg` → `link.svg`
- `plus.svg` → `plus.svg`
- `minus.svg` → `minus.svg`
- `trash.svg` → `trash.svg`
- `settings.svg` → `gear.svg` (rename)
- `chevron-down.svg`, `chevron-up.svg`, `chevron-left.svg`, `chevron-right.svg`, `caret-down/up/left/right.svg` (rename)
- `x.svg` → `x.svg`
- `check.svg` → `check.svg`
- `warning.svg` → `warning.svg`
- `info.svg` → `info.svg`
- `error.svg` → `x-circle.svg` (rename)
- `play.svg` → `play.svg`
- `pause.svg` → `pause.svg`
- `refresh.svg` → `arrows-clockwise.svg` (rename)
- `drag-handle.svg` → `dots-six-vertical.svg` (rename)
- `dots-vertical.svg` → `dots-three-vertical.svg` (rename)

Drop each `.svg` file into `crates/inputforge-gui-dx/src/icons/svg/`. Do NOT modify the SVG content, keep the upstream `<svg>` tag attributes intact (viewBox, fill, etc.).

- [ ] **Step 2: Sanity check**

```bash
ls crates/inputforge-gui-dx/src/icons/svg/ | wc -l
```
Expected: 30.

```bash
head -1 crates/inputforge-gui-dx/src/icons/svg/joystick.svg
```
Expected: starts with `<svg` and includes `viewBox=`.

Also run a BOM-presence check (UTF-8 BOM survives `trim_start` in Rust and would fail the well-formedness test in Task 12):

```bash
for f in crates/inputforge-gui-dx/src/icons/svg/*.svg; do
  head -c 3 "$f" | xxd | grep -q 'efbbbf' && echo "BOM in $f"
done
```

Expected: no output (no BOM). If any line prints, re-export the SVG without BOM (most editors have a "save without BOM" toggle, or `sed -i '1s/^\xEF\xBB\xBF//' "$f"` strips it in place).

- [ ] **Step 3: Commit**

Invoke the `conventional-commits` skill, then commit:
```
feat(gui-dx): add 30 Phosphor SVG icons (regular weight)
```

---

## Task 12: `src/icons/mod.rs`, Icon enum and svg() registry (TDD)

**Files:**
- Modify: `crates/inputforge-gui-dx/src/icons/mod.rs`

- [ ] **Step 1: Write the failing test**

Replace the placeholder `src/icons/mod.rs` with:

```rust
//! Icon enum + SVG registry. SVGs sourced from Phosphor Icons (MIT).
//! Phosphor release tag used: <RELEASE_TAG_FROM_TASK_11>
//!
//! Each variant maps to a `.svg` file under `src/icons/svg/` via
//! `include_str!()`. SVG content is compile-time embedded, so adding
//! a new icon = drop a `.svg` file + add a variant + add a match arm.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    Joystick, Device, Axis, Button, Hat, Mode, Profile, Save, Copy,
    Eye, EyeOff, Link, Plus, Minus, Trash, Settings,
    ChevronDown, ChevronUp, ChevronLeft, ChevronRight,
    X, Check, Warning, Info, Error,
    Play, Pause, Refresh, DragHandle, DotsVertical,
}

impl Icon {
    pub fn svg(&self) -> &'static str {
        match self {
            Icon::Joystick => include_str!("svg/joystick.svg"),
            Icon::Device => include_str!("svg/device.svg"),
            Icon::Axis => include_str!("svg/axis.svg"),
            Icon::Button => include_str!("svg/button.svg"),
            Icon::Hat => include_str!("svg/hat.svg"),
            Icon::Mode => include_str!("svg/mode.svg"),
            Icon::Profile => include_str!("svg/profile.svg"),
            Icon::Save => include_str!("svg/save.svg"),
            Icon::Copy => include_str!("svg/copy.svg"),
            Icon::Eye => include_str!("svg/eye.svg"),
            Icon::EyeOff => include_str!("svg/eye-off.svg"),
            Icon::Link => include_str!("svg/link.svg"),
            Icon::Plus => include_str!("svg/plus.svg"),
            Icon::Minus => include_str!("svg/minus.svg"),
            Icon::Trash => include_str!("svg/trash.svg"),
            Icon::Settings => include_str!("svg/settings.svg"),
            Icon::ChevronDown => include_str!("svg/chevron-down.svg"),
            Icon::ChevronUp => include_str!("svg/chevron-up.svg"),
            Icon::ChevronLeft => include_str!("svg/chevron-left.svg"),
            Icon::ChevronRight => include_str!("svg/chevron-right.svg"),
            Icon::X => include_str!("svg/x.svg"),
            Icon::Check => include_str!("svg/check.svg"),
            Icon::Warning => include_str!("svg/warning.svg"),
            Icon::Info => include_str!("svg/info.svg"),
            Icon::Error => include_str!("svg/error.svg"),
            Icon::Play => include_str!("svg/play.svg"),
            Icon::Pause => include_str!("svg/pause.svg"),
            Icon::Refresh => include_str!("svg/refresh.svg"),
            Icon::DragHandle => include_str!("svg/drag-handle.svg"),
            Icon::DotsVertical => include_str!("svg/dots-vertical.svg"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconSize { Sm, Md, Lg }

impl IconSize {
    pub fn class(&self) -> &'static str {
        match self {
            IconSize::Sm => "if-icon--sm",
            IconSize::Md => "if-icon--md",
            IconSize::Lg => "if-icon--lg",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: &[Icon] = &[
        Icon::Joystick, Icon::Device, Icon::Axis, Icon::Button, Icon::Hat,
        Icon::Mode, Icon::Profile, Icon::Save, Icon::Copy, Icon::Eye,
        Icon::EyeOff, Icon::Link, Icon::Plus, Icon::Minus, Icon::Trash,
        Icon::Settings, Icon::ChevronDown, Icon::ChevronUp, Icon::ChevronLeft,
        Icon::ChevronRight, Icon::X, Icon::Check, Icon::Warning, Icon::Info,
        Icon::Error, Icon::Play, Icon::Pause, Icon::Refresh, Icon::DragHandle,
        Icon::DotsVertical,
    ];

    #[test]
    fn every_variant_returns_non_empty_svg() {
        for icon in ALL {
            let svg = icon.svg();
            assert!(!svg.is_empty(), "{icon:?} svg is empty");
        }
    }

    #[test]
    fn every_variant_returns_well_formed_svg() {
        for icon in ALL {
            let svg = icon.svg();
            // Defensive: strip optional UTF-8 BOM (trim_start does NOT strip \u{FEFF}).
            let head = svg.trim_start_matches('\u{FEFF}').trim_start();
            assert!(
                head.starts_with("<svg") || head.starts_with("<?xml"),
                "{icon:?} does not start with <svg or <?xml prologue (got: {:?})",
                &svg[..svg.len().min(40)]
            );
            assert!(
                svg.contains("viewBox"),
                "{icon:?} missing viewBox attribute"
            );
        }
    }

    #[test]
    fn icon_size_class_names() {
        assert_eq!(IconSize::Sm.class(), "if-icon--sm");
        assert_eq!(IconSize::Md.class(), "if-icon--md");
        assert_eq!(IconSize::Lg.class(), "if-icon--lg");
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p inputforge-gui-dx --lib icons::tests`
Expected: all three tests PASS.

If `every_variant_returns_well_formed_svg` fails, the offending SVG file is corrupt, re-download from the Phosphor source.

- [ ] **Step 3: Commit**

Invoke the `conventional-commits` skill, then commit:
```
feat(gui-dx): add Icon enum with svg() registry and well-formedness tests
```

---

## Task 13: `Icon` component + `icon.css` + wire into ThemeProvider + gallery section

**Files:**
- Create: `crates/inputforge-gui-dx/src/components/icon.rs`
- Create: `crates/inputforge-gui-dx/assets/components/icon.css`
- Modify: `crates/inputforge-gui-dx/src/components/mod.rs`, re-export `Icon` component
- Modify: `crates/inputforge-gui-dx/src/theme/mod.rs`, append `document::Stylesheet { href: ICON_CSS }`
- Modify: `crates/inputforge-gui-dx/examples/component_gallery.rs`, append Icon section

- [ ] **Step 1: Write `src/components/icon.rs`**

Note on `dangerous_inner_html`: if Dioxus 0.7.6 rejects `&'static str`, change `Icon::svg()` call to `.to_string()` at the call site below. Probe locally first:

```rust
//! Renders an SVG icon. SVG content is trusted (Phosphor upstream),
//! injected via `dangerous_inner_html`, never feed user-provided
//! SVG through this component.

use dioxus::prelude::*;

use crate::icons::{Icon as IconKind, IconSize};

#[component]
pub fn Icon(
    name: IconKind,
    #[props(default = IconSize::Md)] size: IconSize,
    #[props(default)] class: Option<String>,
) -> Element {
    let combined = merge_class("if-icon", size.class(), class.as_deref());
    rsx! {
        span {
            class: "{combined}",
            dangerous_inner_html: name.svg(),
        }
    }
}

/// Joins our default class, the variant class, and an optional caller class.
/// Pure-Rust, exported for re-use by other primitives. Skips empty parts so
/// callers may pass `""` as the variant (used by primitives without size/variant
/// modifiers like Slider, Label, Field) without producing double spaces.
pub(crate) fn merge_class(base: &str, variant: &str, caller: Option<&str>) -> String {
    let mut out = String::from(base);
    if !variant.is_empty() {
        out.push(' ');
        out.push_str(variant);
    }
    if let Some(c) = caller {
        if !c.is_empty() {
            out.push(' ');
            out.push_str(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::merge_class;

    #[test]
    fn merge_with_caller() {
        assert_eq!(merge_class("if-icon", "if-icon--md", Some("custom")), "if-icon if-icon--md custom");
    }

    #[test]
    fn merge_without_caller() {
        assert_eq!(merge_class("if-icon", "if-icon--md", None), "if-icon if-icon--md");
    }

    #[test]
    fn merge_with_empty_caller() {
        assert_eq!(merge_class("if-icon", "if-icon--md", Some("")), "if-icon if-icon--md");
    }
}
```

- [ ] **Step 2: Write `assets/components/icon.css`**

```css
.if-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    line-height: 0;
    color: currentColor;
}

/*
 * Phosphor regular weight icons are FILL-based (their paths use
 * fill="currentColor"). Forcing fill: none + stroke would draw thin
 * outlines along path borders instead of rendering filled shapes -
 * every icon would look fundamentally wrong. Keep this rule fill-based.
 * If a stroked aesthetic is wanted later, switch the icon weight in
 * Task 11 to "light" or "thin" (which are stroke-based) AND change
 * this rule to match.
 */
.if-icon svg {
    width:  100%;
    height: 100%;
    fill:   currentColor;
}

.if-icon--sm { width: 14px; height: 14px; }
.if-icon--md { width: 18px; height: 18px; }
.if-icon--lg { width: 24px; height: 24px; }
```

- [ ] **Step 3: Re-export in `src/components/mod.rs`**

```rust
//! Re-exports for the F2 component primitives.

pub mod icon;
pub use icon::Icon;
```

- [ ] **Step 4: Wire `icon.css` into `ThemeProvider`**

In `src/theme/mod.rs`, add:

```rust
const ICON_CSS: Asset = asset!("/assets/components/icon.css");
```

…alongside the other `Asset` constants. Then inside the `rsx!` block, after the `// Component CSS will be appended here ...` comment, add:

```rust
        document::Stylesheet { href: ICON_CSS }
```

- [ ] **Step 5: Append Icon section to gallery**

In `examples/component_gallery.rs`, expand the imports and add a section. Replace the current `gallery_root` body:

```rust
use inputforge_gui_dx::components::Icon;
use inputforge_gui_dx::icons::{Icon as IconKind, IconSize};

fn gallery_root() -> Element {
    rsx! {
        ThemeProvider {
            main {
                style: "padding: var(--space-6); display: flex; flex-direction: column; gap: var(--space-8);",
                h1 { "InputForge, Component Gallery (F2)" }

                section {
                    h2 { "Icon" }
                    div {
                        style: "display: flex; gap: var(--space-4); align-items: center;",
                        Icon { name: IconKind::Joystick, size: IconSize::Sm }
                        Icon { name: IconKind::Joystick, size: IconSize::Md }
                        Icon { name: IconKind::Joystick, size: IconSize::Lg }
                        Icon { name: IconKind::Settings }
                        Icon { name: IconKind::Save }
                        Icon { name: IconKind::Trash }
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 6: Run tests + smoke**

```bash
cargo test -p inputforge-gui-dx --lib components::icon::tests
cargo build -p inputforge-gui-dx --example component_gallery
```
Expected: tests PASS, build OK.

Then `dx serve --example component_gallery --platform desktop`, expect to see the Joystick icon in three sizes plus three more icons.

- [ ] **Step 7: Commit**

Invoke the `conventional-commits` skill, then commit:
```
feat(gui-dx): add Icon component, icon.css, gallery section
```

---

## Task 14: Button + IconButton (canonical interactive pattern)

This is the **worked example**. Subsequent primitive tasks adopt the same pattern: BEM classes, variant + size enums, all five interactive states (default / hover / focus-visible / active / disabled), `class: Option<String>` prop merged via `components::icon::merge_class` (move it to a shared module if needed, see step 1).

**Files:**
- Create: `crates/inputforge-gui-dx/src/components/button.rs`
- Create: `crates/inputforge-gui-dx/src/components/icon_button.rs`
- Create: `crates/inputforge-gui-dx/assets/components/button.css`
- Create: `crates/inputforge-gui-dx/assets/components/icon-button.css`
- Modify: `crates/inputforge-gui-dx/src/components/mod.rs`, re-export `Button` + `IconButton` + relocate `merge_class` to `components/mod.rs` as `pub(crate)`
- Modify: `crates/inputforge-gui-dx/src/theme/mod.rs`, append two stylesheet mounts
- Modify: `crates/inputforge-gui-dx/examples/component_gallery.rs`, append Button and IconButton sections

- [ ] **Step 1: Lift `merge_class` from `components::icon` to `components::mod`**

In `src/components/mod.rs`:

```rust
//! Re-exports for the F2 component primitives.

pub mod icon;
pub mod button;
pub mod icon_button;

pub use icon::Icon;
pub use button::{Button, ButtonVariant, ButtonSize};
pub use icon_button::IconButton;

/// Joins a base class, a variant class, and an optional caller class.
/// Used by every primitive to honor the `class: Option<String>` prop.
/// Empty parts are skipped so primitives without a size/variant modifier
/// (Slider, Label, Field, etc.) may pass `""` as the variant.
pub(crate) fn merge_class(base: &str, variant: &str, caller: Option<&str>) -> String {
    let mut out = String::from(base);
    if !variant.is_empty() {
        out.push(' ');
        out.push_str(variant);
    }
    if let Some(c) = caller {
        if !c.is_empty() {
            out.push(' ');
            out.push_str(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::merge_class;

    #[test]
    fn with_caller()        { assert_eq!(merge_class("a", "b", Some("c")), "a b c"); }
    #[test]
    fn without_caller()     { assert_eq!(merge_class("a", "b", None), "a b"); }
    #[test]
    fn empty_caller()       { assert_eq!(merge_class("a", "b", Some("")), "a b"); }
    #[test]
    fn empty_variant()      { assert_eq!(merge_class("a", "", Some("c")), "a c"); }
    #[test]
    fn empty_variant_no_caller() { assert_eq!(merge_class("a", "", None), "a"); }
    #[test]
    fn no_trailing_space()  { assert!(!merge_class("a", "b", None).ends_with(' ')); }
}
```

In `src/components/icon.rs`, delete the local `merge_class` function and its tests, and replace its call site to use `super::merge_class` instead. Re-run `cargo test -p inputforge-gui-dx --lib` to verify nothing broke.

- [ ] **Step 2: Write `src/components/button.rs`**

```rust
use dioxus::prelude::*;

use super::merge_class;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonVariant { Primary, Secondary, Ghost, Danger }

impl ButtonVariant {
    fn class(&self) -> &'static str {
        match self {
            ButtonVariant::Primary   => "if-button--primary",
            ButtonVariant::Secondary => "if-button--secondary",
            ButtonVariant::Ghost     => "if-button--ghost",
            ButtonVariant::Danger    => "if-button--danger",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonSize { Sm, Md, Lg }

impl ButtonSize {
    fn class(&self) -> &'static str {
        match self {
            ButtonSize::Sm => "if-button--sm",
            ButtonSize::Md => "if-button--md",
            ButtonSize::Lg => "if-button--lg",
        }
    }
}

#[component]
pub fn Button(
    #[props(default = ButtonVariant::Primary)] variant: ButtonVariant,
    #[props(default = ButtonSize::Md)] size: ButtonSize,
    #[props(default)] disabled: bool,
    #[props(default)] class: Option<String>,
    onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let variant_class = format!("{} {}", variant.class(), size.class());
    let combined = merge_class("if-button", &variant_class, class.as_deref());
    rsx! {
        button {
            class: "{combined}",
            disabled,
            onclick: move |evt| {
                if let Some(handler) = &onclick {
                    handler.call(evt);
                }
            },
            {children}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: every component must compose its class string via `merge_class`,
    /// not inline `format!`, to avoid the trailing-space bug when no caller class is
    /// provided. If this test fails, a primitive likely reverted to inline `format!`.
    #[test]
    fn class_string_has_no_trailing_space_when_no_caller_class() {
        let v_class = ButtonVariant::Primary.class();
        let s_class = ButtonSize::Md.class();
        let combined = merge_class("if-button", &format!("{v_class} {s_class}"), None);
        assert!(!combined.ends_with(' '), "got: {combined:?}");
        assert_eq!(combined, "if-button if-button--primary if-button--md");
    }
}
```

- [ ] **Step 3: Write `assets/components/button.css`**

```css
.if-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    font-family: var(--font-sans);
    font-weight: var(--weight-medium);
    background: var(--color-bg-elevated);
    color: var(--color-text);
    cursor: pointer;
    user-select: none;
    transition: background var(--duration-fast) var(--easing-standard),
                border-color var(--duration-fast) var(--easing-standard),
                color var(--duration-fast) var(--easing-standard);
}

.if-button:hover:not(:disabled) {
    background: var(--color-bg-elevated);
    border-color: var(--color-border-strong);
}

.if-button:focus-visible {
    outline: 2px solid var(--color-border-focus);
    outline-offset: 2px;
}

.if-button:active:not(:disabled) {
    transform: translateY(1px);
}

.if-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

/* Variants */
.if-button.if-button--primary {
    background: var(--color-primary);
    color: var(--color-primary-fg);
    border-color: var(--color-primary);
}
.if-button.if-button--primary:hover:not(:disabled)  { background: var(--color-primary-hover); border-color: var(--color-primary-hover); }
.if-button.if-button--primary:active:not(:disabled) { background: var(--color-primary-active); border-color: var(--color-primary-active); }

.if-button.if-button--secondary {
    background: var(--color-bg-elevated);
    color: var(--color-text);
    border-color: var(--color-border-strong);
}

.if-button.if-button--ghost {
    background: transparent;
    border-color: transparent;
    color: var(--color-text);
}
.if-button.if-button--ghost:hover:not(:disabled) {
    background: var(--color-bg-elevated);
    border-color: var(--color-border);
}

.if-button.if-button--danger {
    background: var(--color-error);
    color: var(--color-text-inverse);
    border-color: var(--color-error);
}

/* Sizes */
.if-button.if-button--sm { padding: var(--space-1) var(--space-3); font-size: var(--text-sm); }
.if-button.if-button--md { padding: var(--space-2) var(--space-4); font-size: var(--text-base); }
.if-button.if-button--lg { padding: var(--space-3) var(--space-6); font-size: var(--text-md); }
```

- [ ] **Step 4: Write `src/components/icon_button.rs`**

```rust
use dioxus::prelude::*;

use super::merge_class;
use crate::components::button::{ButtonSize, ButtonVariant};
use crate::components::Icon;
use crate::icons::Icon as IconKind;

#[component]
pub fn IconButton(
    icon: IconKind,
    label: &'static str,
    #[props(default = ButtonVariant::Ghost)] variant: ButtonVariant,
    #[props(default = ButtonSize::Md)] size: ButtonSize,
    #[props(default)] disabled: bool,
    #[props(default)] class: Option<String>,
    onclick: Option<EventHandler<MouseEvent>>,
) -> Element {
    let variant_class = match variant {
        ButtonVariant::Primary   => "if-icon-button--primary",
        ButtonVariant::Secondary => "if-icon-button--secondary",
        ButtonVariant::Ghost     => "if-icon-button--ghost",
        ButtonVariant::Danger    => "if-icon-button--danger",
    };
    let size_class = match size {
        ButtonSize::Sm => "if-icon-button--sm",
        ButtonSize::Md => "if-icon-button--md",
        ButtonSize::Lg => "if-icon-button--lg",
    };
    let combined = merge_class(
        "if-icon-button",
        &format!("{variant_class} {size_class}"),
        class.as_deref(),
    );
    rsx! {
        button {
            class: "{combined}",
            "aria-label": label,
            disabled,
            onclick: move |evt| {
                if let Some(handler) = &onclick {
                    handler.call(evt);
                }
            },
            Icon { name: icon }
        }
    }
}
```

- [ ] **Step 5: Write `assets/components/icon-button.css`**

```css
.if-icon-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg-elevated);
    color: var(--color-text);
    cursor: pointer;
    transition: background var(--duration-fast) var(--easing-standard),
                border-color var(--duration-fast) var(--easing-standard);
}

.if-icon-button--sm { width: 28px; height: 28px; }
.if-icon-button--md { width: 36px; height: 36px; }
.if-icon-button--lg { width: 44px; height: 44px; }

.if-icon-button--ghost {
    background: transparent;
    border-color: transparent;
}
.if-icon-button--ghost:hover:not(:disabled) {
    background: var(--color-bg-elevated);
    border-color: var(--color-border);
}

.if-icon-button--primary {
    background: var(--color-primary);
    color: var(--color-primary-fg);
    border-color: var(--color-primary);
}

.if-icon-button--secondary {
    border-color: var(--color-border-strong);
}

.if-icon-button--danger {
    background: var(--color-error);
    color: var(--color-text-inverse);
    border-color: var(--color-error);
}

.if-icon-button:focus-visible { outline: 2px solid var(--color-border-focus); outline-offset: 2px; }
.if-icon-button:active:not(:disabled) { transform: translateY(1px); }
.if-icon-button:disabled { opacity: 0.5; cursor: not-allowed; }
```

- [ ] **Step 6: Wire CSS into `ThemeProvider`**

Add `BUTTON_CSS` and `ICON_BUTTON_CSS` `Asset` constants in `src/theme/mod.rs` and append two `document::Stylesheet { href: ... }` lines after the existing `ICON_CSS` mount.

```rust
const BUTTON_CSS:      Asset = asset!("/assets/components/button.css");
const ICON_BUTTON_CSS: Asset = asset!("/assets/components/icon-button.css");
```

```rust
        document::Stylesheet { href: BUTTON_CSS }
        document::Stylesheet { href: ICON_BUTTON_CSS }
```

- [ ] **Step 7: Append gallery sections**

In `examples/component_gallery.rs`:

```rust
use inputforge_gui_dx::components::{Button, ButtonVariant, ButtonSize, IconButton};
```

Add two new `<section>` blocks inside `gallery_root`:

```rust
                section {
                    h2 { "Button" }
                    div {
                        style: "display: flex; gap: var(--space-3); flex-wrap: wrap; align-items: center;",
                        Button { variant: ButtonVariant::Primary,   "Primary" }
                        Button { variant: ButtonVariant::Secondary, "Secondary" }
                        Button { variant: ButtonVariant::Ghost,     "Ghost" }
                        Button { variant: ButtonVariant::Danger,    "Danger" }
                        Button { disabled: true, "Disabled" }
                    }
                    div {
                        style: "display: flex; gap: var(--space-3); margin-top: var(--space-3);",
                        Button { size: ButtonSize::Sm, "Small" }
                        Button { size: ButtonSize::Md, "Medium" }
                        Button { size: ButtonSize::Lg, "Large" }
                    }
                }

                section {
                    h2 { "IconButton" }
                    div {
                        style: "display: flex; gap: var(--space-3); align-items: center;",
                        IconButton { icon: IconKind::Settings, label: "Settings" }
                        IconButton { icon: IconKind::Save,     label: "Save",  variant: ButtonVariant::Primary }
                        IconButton { icon: IconKind::Trash,    label: "Delete", variant: ButtonVariant::Danger }
                        IconButton { icon: IconKind::Eye,      label: "Show",   disabled: true }
                    }
                }
```

- [ ] **Step 8: Run tests + smoke**

```bash
cargo test -p inputforge-gui-dx
cargo build -p inputforge-gui-dx --example component_gallery
```

Then `dx serve --example component_gallery --platform desktop`. Expect: Button section shows 4 variants + Disabled in row 1, three sizes in row 2; IconButton section shows 4 buttons. **Manually verify all five states:** default visible, hover changes color, Tab moves focus and shows the focus ring, click triggers active visual, disabled does not respond.

- [ ] **Step 9: Commit**

Invoke the `conventional-commits` skill, then commit:
```
feat(gui-dx): add Button and IconButton primitives with full state coverage
```

---

## Task 15: Form input family, TextInput, NumberInput, Select, Slider

Four primitives sharing the same `if-control` baseline (border, padding, focus ring) plus per-type accents.

**Files:**
- Create: `crates/inputforge-gui-dx/src/components/{text_input,number_input,select,slider}.rs`
- Create: `crates/inputforge-gui-dx/assets/components/{text-input,number-input,select,slider}.css`
- Modify: `src/components/mod.rs`, add `pub mod` + re-exports
- Modify: `src/theme/mod.rs`, four `Asset` consts + four `Stylesheet` mounts
- Modify: `examples/component_gallery.rs`, four sections

- [ ] **Step 1: `src/components/text_input.rs`**

```rust
use dioxus::prelude::*;

use super::merge_class;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputSize { Sm, Md, Lg }

impl InputSize {
    pub(crate) fn class(&self) -> &'static str {
        match self {
            InputSize::Sm => "if-text-input--sm",
            InputSize::Md => "if-text-input--md",
            InputSize::Lg => "if-text-input--lg",
        }
    }
}

#[component]
pub fn TextInput(
    value: ReadOnlySignal<String>,
    oninput: Option<EventHandler<FormEvent>>,
    #[props(default)] placeholder: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] invalid: bool,
    #[props(default = InputSize::Md)] size: InputSize,
    #[props(default)] class: Option<String>,
) -> Element {
    let variant_class = if invalid {
        format!("{} if-text-input--invalid", size.class())
    } else {
        size.class().to_string()
    };
    let classes = merge_class("if-text-input", &variant_class, class.as_deref());
    rsx! {
        input {
            r#type: "text",
            class: "{classes}",
            value: "{value}",
            placeholder: placeholder.as_deref().unwrap_or(""),
            disabled,
            oninput: move |evt| { if let Some(h) = &oninput { h.call(evt); } },
        }
    }
}
```

- [ ] **Step 2: `assets/components/text-input.css`**

```css
.if-text-input {
    display: inline-block;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg-sunken);
    color: var(--color-text);
    font-family: var(--font-sans);
    padding: var(--space-2) var(--space-3);
    transition: border-color var(--duration-fast) var(--easing-standard);
}

.if-text-input::placeholder { color: var(--color-text-subtle); }

.if-text-input:hover:not(:disabled)        { border-color: var(--color-border-strong); }
.if-text-input:focus-visible               { outline: 2px solid var(--color-border-focus); outline-offset: 2px; border-color: var(--color-border-focus); }
.if-text-input:disabled                    { opacity: 0.5; cursor: not-allowed; }
.if-text-input.if-text-input--invalid      { border-color: var(--color-error); }
.if-text-input.if-text-input--invalid:focus-visible { outline-color: var(--color-error); border-color: var(--color-error); }

.if-text-input.if-text-input--sm { font-size: var(--text-sm);  padding: var(--space-1) var(--space-2); }
.if-text-input.if-text-input--md { font-size: var(--text-base); padding: var(--space-2) var(--space-3); }
.if-text-input.if-text-input--lg { font-size: var(--text-md);  padding: var(--space-3) var(--space-4); }
```

- [ ] **Step 3: `src/components/number_input.rs`**

```rust
use dioxus::prelude::*;

use super::merge_class;
use crate::components::text_input::InputSize;
use crate::components::{Icon, IconButton, ButtonVariant, ButtonSize};
use crate::icons::Icon as IconKind;

#[component]
pub fn NumberInput(
    value: ReadOnlySignal<f64>,
    oninput: Option<EventHandler<FormEvent>>,
    #[props(default = f64::NEG_INFINITY)] min: f64,
    #[props(default = f64::INFINITY)] max: f64,
    #[props(default = 1.0)] step: f64,
    #[props(default)] disabled: bool,
    #[props(default = InputSize::Md)] size: InputSize,
    #[props(default)] class: Option<String>,
) -> Element {
    let size_class = match size {
        InputSize::Sm => "if-number-input--sm",
        InputSize::Md => "if-number-input--md",
        InputSize::Lg => "if-number-input--lg",
    };
    let combined = merge_class("if-number-input", size_class, class.as_deref());
    rsx! {
        div { class: "{combined}",
            input {
                r#type: "number",
                class: "if-number-input__field",
                value: "{value}",
                min: "{min}",
                max: "{max}",
                step: "{step}",
                disabled,
                oninput: move |evt| { if let Some(h) = &oninput { h.call(evt); } },
            }
            div {
                class: "if-number-input__steppers",
                IconButton { icon: IconKind::Plus,  label: "Increment", size: ButtonSize::Sm, variant: ButtonVariant::Ghost, disabled }
                IconButton { icon: IconKind::Minus, label: "Decrement", size: ButtonSize::Sm, variant: ButtonVariant::Ghost, disabled }
            }
        }
    }
}
```

(Stepper buttons are visual scaffolding only at F2; wiring them to actually mutate `value` is each consumer's responsibility, F2 ships the layout primitive.)

- [ ] **Step 4: `assets/components/number-input.css`**

```css
.if-number-input {
    display: inline-flex;
    align-items: stretch;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg-sunken);
    overflow: hidden;
}

.if-number-input:focus-within { outline: 2px solid var(--color-border-focus); outline-offset: 2px; }

.if-number-input__field {
    border: none;
    background: transparent;
    color: var(--color-text);
    font-family: var(--font-mono);
    padding: var(--space-2) var(--space-3);
    width: 6ch;
    text-align: right;
    -moz-appearance: textfield;
}
.if-number-input__field::-webkit-outer-spin-button,
.if-number-input__field::-webkit-inner-spin-button { -webkit-appearance: none; margin: 0; }
.if-number-input__field:focus { outline: none; }

.if-number-input__steppers {
    display: flex;
    flex-direction: column;
    border-left: 1px solid var(--color-border);
}
.if-number-input__steppers .if-icon-button { width: 22px; height: 50%; border: none; border-radius: 0; }
```

- [ ] **Step 5: `src/components/select.rs`**

```rust
use dioxus::prelude::*;

use super::merge_class;
use crate::components::text_input::InputSize;

#[component]
pub fn Select(
    value: ReadOnlySignal<String>,
    onchange: Option<EventHandler<FormEvent>>,
    options: Vec<(String, String)>, // (value, label)
    #[props(default)] disabled: bool,
    #[props(default = InputSize::Md)] size: InputSize,
    #[props(default)] class: Option<String>,
) -> Element {
    let size_class = match size {
        InputSize::Sm => "if-select--sm",
        InputSize::Md => "if-select--md",
        InputSize::Lg => "if-select--lg",
    };
    let combined = merge_class("if-select", size_class, class.as_deref());
    rsx! {
        select {
            class: "{combined}",
            value: "{value}",
            disabled,
            onchange: move |evt| { if let Some(h) = &onchange { h.call(evt); } },
            for (val, label) in options.iter() {
                option { value: "{val}", "{label}" }
            }
        }
    }
}
```

- [ ] **Step 6: `assets/components/select.css`**

```css
.if-select {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg-sunken);
    color: var(--color-text);
    font-family: var(--font-sans);
    padding: var(--space-2) var(--space-3);
    cursor: pointer;
}
.if-select:hover:not(:disabled) { border-color: var(--color-border-strong); }
.if-select:focus-visible        { outline: 2px solid var(--color-border-focus); outline-offset: 2px; border-color: var(--color-border-focus); }
.if-select:disabled             { opacity: 0.5; cursor: not-allowed; }

.if-select.if-select--sm { font-size: var(--text-sm); padding: var(--space-1) var(--space-2); }
.if-select.if-select--md { font-size: var(--text-base); }
.if-select.if-select--lg { font-size: var(--text-md); padding: var(--space-3) var(--space-4); }
```

- [ ] **Step 7: `src/components/slider.rs`**

```rust
use dioxus::prelude::*;

use super::merge_class;

#[component]
pub fn Slider(
    value: ReadOnlySignal<f64>,
    oninput: Option<EventHandler<FormEvent>>,
    #[props(default = 0.0)] min: f64,
    #[props(default = 1.0)] max: f64,
    #[props(default = 0.01)] step: f64,
    #[props(default)] disabled: bool,
    #[props(default)] class: Option<String>,
) -> Element {
    let combined = merge_class("if-slider", "", class.as_deref());
    rsx! {
        input {
            r#type: "range",
            class: "{combined}",
            value: "{value}",
            min: "{min}", max: "{max}", step: "{step}",
            disabled,
            oninput: move |evt| { if let Some(h) = &oninput { h.call(evt); } },
        }
    }
}
```

- [ ] **Step 8: `assets/components/slider.css`**

```css
.if-slider {
    -webkit-appearance: none;
    width: 100%;
    height: 4px;
    background: var(--color-border);
    border-radius: var(--radius-full);
    outline: none;
    cursor: pointer;
}
.if-slider:disabled { opacity: 0.5; cursor: not-allowed; }

.if-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 16px; height: 16px;
    background: var(--color-primary);
    border-radius: var(--radius-full);
    border: 2px solid var(--color-bg);
    cursor: grab;
}
.if-slider::-moz-range-thumb {
    width: 16px; height: 16px;
    background: var(--color-primary);
    border-radius: var(--radius-full);
    border: 2px solid var(--color-bg);
    cursor: grab;
}

.if-slider:focus-visible { outline: 2px solid var(--color-border-focus); outline-offset: 4px; }
```

- [ ] **Step 9: Wire mod re-exports, ThemeProvider mounts, gallery sections**

In `src/components/mod.rs`, add:

```rust
pub mod text_input;
pub mod number_input;
pub mod select;
pub mod slider;

pub use text_input::{TextInput, InputSize};
pub use number_input::NumberInput;
pub use select::Select;
pub use slider::Slider;
```

In `src/theme/mod.rs`, add four `Asset` consts (`TEXT_INPUT_CSS`, `NUMBER_INPUT_CSS`, `SELECT_CSS`, `SLIDER_CSS`) and four `document::Stylesheet { href: ... }` mounts after the IconButton mount.

In `examples/component_gallery.rs`, add four sections. Example for TextInput:

```rust
                section {
                    h2 { "TextInput" }
                    div {
                        style: "display: flex; flex-direction: column; gap: var(--space-3); max-width: 320px;",
                        TextInput { value: "hello".to_string(), placeholder: "Type here…".to_string() }
                        TextInput { value: "".to_string(), placeholder: "Disabled".to_string(), disabled: true }
                        TextInput { value: "wrong".to_string(), invalid: true }
                    }
                }
```

(Mirror the pattern for NumberInput / Select / Slider with sensible seed values.)

- [ ] **Step 10: Smoke**

```bash
cargo test -p inputforge-gui-dx
cargo build -p inputforge-gui-dx --example component_gallery
```

Then `dx serve --example component_gallery --platform desktop`. Verify all four input variants render and respond to interaction (typing, hovering, focus ring on Tab, disabled state visually distinct).

- [ ] **Step 11: Commit**

Invoke the `conventional-commits` skill, then commit:
```
feat(gui-dx): add TextInput, NumberInput, Select, Slider primitives
```

---

## Task 16: Toggle family, Switch + Checkbox

**Files:**
- Create: `src/components/{switch,checkbox}.rs`
- Create: `assets/components/{switch,checkbox}.css`
- Modify: `mod.rs`, `theme/mod.rs`, gallery

- [ ] **Step 1: `src/components/switch.rs`**

```rust
use dioxus::prelude::*;

use super::merge_class;

#[component]
pub fn Switch(
    checked: ReadOnlySignal<bool>,
    onchange: Option<EventHandler<FormEvent>>,
    #[props(default)] disabled: bool,
    #[props(default)] label: Option<String>,
    #[props(default)] class: Option<String>,
) -> Element {
    let combined = merge_class("if-switch", "", class.as_deref());
    rsx! {
        label { class: "{combined}",
            input {
                r#type: "checkbox",
                class: "if-switch__input",
                checked: "{checked}",
                disabled,
                onchange: move |evt| { if let Some(h) = &onchange { h.call(evt); } },
            }
            span { class: "if-switch__track", span { class: "if-switch__thumb" } }
            if let Some(l) = label.as_deref() { span { class: "if-switch__label", "{l}" } }
        }
    }
}
```

- [ ] **Step 2: `assets/components/switch.css`**

```css
.if-switch { display: inline-flex; align-items: center; gap: var(--space-2); cursor: pointer; user-select: none; }

.if-switch__input { position: absolute; opacity: 0; pointer-events: none; }

.if-switch__track {
    position: relative;
    width: 36px; height: 20px;
    background: var(--color-bg-sunken);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-full);
    transition: background var(--duration-fast) var(--easing-standard);
}
.if-switch__thumb {
    position: absolute;
    top: 1px; left: 1px;
    width: 16px; height: 16px;
    background: var(--color-text-muted);
    border-radius: var(--radius-full);
    transition: transform var(--duration-fast) var(--easing-standard),
                background var(--duration-fast) var(--easing-standard);
}

.if-switch__input:checked ~ .if-switch__track                   { background: var(--color-primary); border-color: var(--color-primary); }
.if-switch__input:checked ~ .if-switch__track .if-switch__thumb { transform: translateX(16px); background: var(--color-primary-fg); }

.if-switch__input:focus-visible ~ .if-switch__track { outline: 2px solid var(--color-border-focus); outline-offset: 2px; }
.if-switch__input:disabled ~ .if-switch__track       { opacity: 0.5; cursor: not-allowed; }

.if-switch__label { color: var(--color-text); font-size: var(--text-base); }
```

- [ ] **Step 3: `src/components/checkbox.rs`**

```rust
use dioxus::prelude::*;

use super::merge_class;

#[component]
pub fn Checkbox(
    checked: ReadOnlySignal<bool>,
    onchange: Option<EventHandler<FormEvent>>,
    #[props(default)] disabled: bool,
    #[props(default)] indeterminate: bool,
    #[props(default)] class: Option<String>,
) -> Element {
    let variant_class = if indeterminate { "if-checkbox--indeterminate" } else { "" };
    let combined = merge_class("if-checkbox", variant_class, class.as_deref());
    rsx! {
        label { class: "{combined}",
            input {
                r#type: "checkbox",
                class: "if-checkbox__input",
                checked: "{checked}",
                disabled,
                onchange: move |evt| { if let Some(h) = &onchange { h.call(evt); } },
            }
            span { class: "if-checkbox__box" }
        }
    }
}
```

- [ ] **Step 4: `assets/components/checkbox.css`**

```css
.if-checkbox { display: inline-flex; align-items: center; cursor: pointer; }

.if-checkbox__input { position: absolute; opacity: 0; pointer-events: none; }

.if-checkbox__box {
    position: relative;
    width: 16px; height: 16px;
    background: var(--color-bg-sunken);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    transition: background var(--duration-fast) var(--easing-standard),
                border-color var(--duration-fast) var(--easing-standard);
}

.if-checkbox__input:checked ~ .if-checkbox__box {
    background: var(--color-primary);
    border-color: var(--color-primary);
}
.if-checkbox__input:checked ~ .if-checkbox__box::after {
    content: '';
    position: absolute;
    top: 1px; left: 5px;
    width: 4px; height: 8px;
    border: solid var(--color-primary-fg);
    border-width: 0 2px 2px 0;
    transform: rotate(45deg);
}

.if-checkbox.if-checkbox--indeterminate .if-checkbox__box {
    background: var(--color-primary);
    border-color: var(--color-primary);
}
.if-checkbox.if-checkbox--indeterminate .if-checkbox__box::after {
    content: '';
    position: absolute;
    top: 7px; left: 3px;
    width: 8px; height: 2px;
    background: var(--color-primary-fg);
}

.if-checkbox__input:focus-visible ~ .if-checkbox__box { outline: 2px solid var(--color-border-focus); outline-offset: 2px; }
.if-checkbox__input:disabled ~ .if-checkbox__box       { opacity: 0.5; cursor: not-allowed; }
```

- [ ] **Step 5: Mod re-exports, theme mounts, gallery sections**

In `src/components/mod.rs`:

```rust
pub mod switch;
pub mod checkbox;
pub use switch::Switch;
pub use checkbox::Checkbox;
```

In `src/theme/mod.rs`, add `SWITCH_CSS` and `CHECKBOX_CSS` consts and mount.

Gallery sections (in `examples/component_gallery.rs`):

```rust
                section {
                    h2 { "Switch" }
                    div { style: "display: flex; gap: var(--space-4);",
                        Switch { checked: false, label: "Off".to_string() }
                        Switch { checked: true,  label: "On".to_string() }
                        Switch { checked: false, disabled: true, label: "Disabled".to_string() }
                    }
                }
                section {
                    h2 { "Checkbox" }
                    div { style: "display: flex; gap: var(--space-4); align-items: center;",
                        Checkbox { checked: false }
                        Checkbox { checked: true }
                        Checkbox { checked: false, indeterminate: true }
                        Checkbox { checked: false, disabled: true }
                    }
                }
```

- [ ] **Step 6: Smoke + commit**

```bash
cargo build -p inputforge-gui-dx --example component_gallery
dx serve --example component_gallery --platform desktop
```

Verify: Switch toggles visually on click, Checkbox shows the checkmark when checked, indeterminate state shows the bar, disabled states are dimmed, focus rings appear on Tab.

Invoke the `conventional-commits` skill, then commit:
```
feat(gui-dx): add Switch and Checkbox primitives
```

---

## Task 17: Display family, Card + Badge + Separator + Spinner

Non-interactive primitives. After this task, the gallery can wrap each section in a `Card`.

**Files:**
- Create: `src/components/{card,badge,separator,spinner}.rs`
- Create: `assets/components/{card,badge,separator,spinner}.css`
- Modify: mod / theme / gallery; **also wrap each existing gallery section in a `Card`**

- [ ] **Step 1: `src/components/card.rs`**

```rust
use dioxus::prelude::*;

use super::merge_class;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CardPadding { Sm, Md, Lg }

#[component]
pub fn Card(
    #[props(default = CardPadding::Md)] padding: CardPadding,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let pad_class = match padding {
        CardPadding::Sm => "if-card--pad-sm",
        CardPadding::Md => "if-card--pad-md",
        CardPadding::Lg => "if-card--pad-lg",
    };
    let combined = merge_class("if-card", pad_class, class.as_deref());
    rsx! { div { class: "{combined}", {children} } }
}
```

- [ ] **Step 2: `assets/components/card.css`**

```css
.if-card {
    background: var(--color-bg-elevated);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
}
.if-card.if-card--pad-sm { padding: var(--space-3); }
.if-card.if-card--pad-md { padding: var(--space-4); }
.if-card.if-card--pad-lg { padding: var(--space-6); }
```

- [ ] **Step 3: `src/components/badge.rs`**

```rust
use dioxus::prelude::*;

use super::merge_class;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BadgeVariant { Neutral, Info, Success, Warning, Error }

#[component]
pub fn Badge(
    #[props(default = BadgeVariant::Neutral)] variant: BadgeVariant,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let v = match variant {
        BadgeVariant::Neutral => "if-badge--neutral",
        BadgeVariant::Info    => "if-badge--info",
        BadgeVariant::Success => "if-badge--success",
        BadgeVariant::Warning => "if-badge--warning",
        BadgeVariant::Error   => "if-badge--error",
    };
    let combined = merge_class("if-badge", v, class.as_deref());
    rsx! { span { class: "{combined}", {children} } }
}
```

- [ ] **Step 4: `assets/components/badge.css`**

```css
.if-badge {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: 2px var(--space-2);
    border-radius: var(--radius-full);
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
    line-height: 1;
    border: 1px solid transparent;
}
.if-badge.if-badge--neutral { background: var(--color-bg-sunken);  color: var(--color-text-muted); border-color: var(--color-border); }
.if-badge.if-badge--info    { background: rgba(74,158,255,0.15);   color: var(--color-info);       border-color: var(--color-info); }
.if-badge.if-badge--success { background: rgba(0,229,160,0.15);    color: var(--color-live);       border-color: var(--color-live); }
.if-badge.if-badge--warning { background: rgba(255,179,71,0.15);   color: var(--color-warning);    border-color: var(--color-warning); }
.if-badge.if-badge--error   { background: rgba(255,107,107,0.15);  color: var(--color-error);      border-color: var(--color-error); }
```

- [ ] **Step 5: `src/components/separator.rs`**

```rust
use dioxus::prelude::*;

use super::merge_class;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SeparatorOrientation { Horizontal, Vertical }

#[component]
pub fn Separator(
    #[props(default = SeparatorOrientation::Horizontal)] orientation: SeparatorOrientation,
    #[props(default)] class: Option<String>,
) -> Element {
    let v = match orientation {
        SeparatorOrientation::Horizontal => "if-separator--horizontal",
        SeparatorOrientation::Vertical   => "if-separator--vertical",
    };
    let combined = merge_class("if-separator", v, class.as_deref());
    rsx! { div { class: "{combined}", role: "separator" } }
}
```

- [ ] **Step 6: `assets/components/separator.css`**

```css
.if-separator { background: var(--color-border); }
.if-separator.if-separator--horizontal { width: 100%; height: 1px; }
.if-separator.if-separator--vertical   { width: 1px;  height: 100%; }
```

- [ ] **Step 7: `src/components/spinner.rs`**

```rust
use dioxus::prelude::*;

use super::merge_class;

/// Dedicated size enum so a future `SpinnerSize::Xl` can land without
/// polluting `text_input::InputSize`. Conceptually unrelated to input sizing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SpinnerSize {
    Sm,
    #[default]
    Md,
    Lg,
}

impl SpinnerSize {
    pub(crate) fn class(&self) -> &'static str {
        match self {
            SpinnerSize::Sm => "if-spinner--sm",
            SpinnerSize::Md => "if-spinner--md",
            SpinnerSize::Lg => "if-spinner--lg",
        }
    }
}

#[component]
pub fn Spinner(
    #[props(default = SpinnerSize::Md)] size: SpinnerSize,
    #[props(default)] class: Option<String>,
) -> Element {
    let combined = merge_class("if-spinner", size.class(), class.as_deref());
    rsx! { div { class: "{combined}", "aria-busy": "true", role: "status" } }
}
```

- [ ] **Step 8: `assets/components/spinner.css`**

```css
.if-spinner {
    display: inline-block;
    border: 2px solid var(--color-border);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: if-spinner-rotate var(--duration-slow) linear infinite;
}
.if-spinner.if-spinner--sm { width: 12px; height: 12px; }
.if-spinner.if-spinner--md { width: 18px; height: 18px; }
.if-spinner.if-spinner--lg { width: 28px; height: 28px; border-width: 3px; }

@keyframes if-spinner-rotate { to { transform: rotate(360deg); } }
```

- [ ] **Step 9: Wire re-exports + theme mounts**

In `src/components/mod.rs`:

```rust
pub mod card;
pub mod badge;
pub mod separator;
pub mod spinner;
pub use card::{Card, CardPadding};
pub use badge::{Badge, BadgeVariant};
pub use separator::{Separator, SeparatorOrientation};
pub use spinner::{Spinner, SpinnerSize};
```

In `src/theme/mod.rs`, add four `Asset` consts and mounts.

- [ ] **Step 10 (Task 17a): Add four new gallery sections demonstrating the new primitives**

Append four new sections to `examples/component_gallery.rs`. These exercise Card / Badge / Separator / Spinner themselves; **do not** yet refactor the existing sections, that's Task 17b's job. Limiting the change set to "new primitives only" makes any failure attributable to the four files added in steps 1-8, not to a Card-wrap regression in older sections.

Imports to add at the top of the gallery: `Card, CardPadding, Badge, BadgeVariant, Separator, SeparatorOrientation, Spinner, SpinnerSize`.

```rust
                section {
                    h2 { "Card" }
                    div { style: "display: flex; gap: var(--space-3);",
                        Card { padding: CardPadding::Sm, "Small padding" }
                        Card { padding: CardPadding::Md, "Medium padding" }
                        Card { padding: CardPadding::Lg, "Large padding" }
                    }
                }
                section {
                    h2 { "Badge" }
                    div { style: "display: flex; gap: var(--space-2);",
                        Badge { variant: BadgeVariant::Neutral, "Neutral" }
                        Badge { variant: BadgeVariant::Info,    "Info" }
                        Badge { variant: BadgeVariant::Success, "Success" }
                        Badge { variant: BadgeVariant::Warning, "Warning" }
                        Badge { variant: BadgeVariant::Error,   "Error" }
                    }
                }
                section {
                    h2 { "Separator" }
                    Separator {}
                    div { style: "display: flex; gap: var(--space-3); align-items: center; height: 30px;",
                        span { "Left" }
                        Separator { orientation: SeparatorOrientation::Vertical }
                        span { "Right" }
                    }
                }
                section {
                    h2 { "Spinner" }
                    div { style: "display: flex; gap: var(--space-3); align-items: center;",
                        Spinner { size: SpinnerSize::Sm }
                        Spinner { size: SpinnerSize::Md }
                        Spinner { size: SpinnerSize::Lg }
                    }
                }
```

- [ ] **Step 11 (Task 17a): Smoke checkpoint + commit**

```bash
cargo test -p inputforge-gui-dx --lib
cargo build -p inputforge-gui-dx --example component_gallery
dx serve --example component_gallery --platform desktop
```

Verify: lib tests pass; the four new sections render; badges show in five colors; separator divides content; spinner animates; **prior sections (Icon, Button, IconButton, TextInput, NumberInput, Select, Slider, Switch, Checkbox) still render unchanged** (sanity-check by scrolling through them).

Invoke the `conventional-commits` skill, then commit:
```
feat(gui-dx): add Card, Badge, Separator, Spinner primitives + gallery section
```

---

## Task 17b: Wrap existing gallery sections in `Card` (refactor)

A standalone refactor pass following Task 17a's primitive landings. Splitting the Card-wrap from the new-primitive work isolates failure attribution: if a wrapped section regresses (RSX nesting, signal capture, prop coercion through Card), the diff under suspicion is small.

**Files:**
- Modify: `crates/inputforge-gui-dx/examples/component_gallery.rs`

- [ ] **Step 1: Wrap each existing section in a `Card`**

For each of these sections in the gallery, Icon, Button, IconButton, TextInput, NumberInput, Select, Slider, Switch, Checkbox, wrap the section's content in a `Card { padding: CardPadding::Md, … }`. Example for the Icon section:

```rust
                section {
                    h2 { "Icon" }
                    Card { padding: CardPadding::Md,
                        div {
                            style: "display: flex; gap: var(--space-4); align-items: center;",
                            Icon { name: IconKind::Joystick, size: IconSize::Sm }
                            // … etc.
                        }
                    }
                }
```

Optional: insert `Separator { orientation: SeparatorOrientation::Horizontal }` between sections if visual rhythm wants it.

- [ ] **Step 2: Verification checkpoint**

```bash
cargo build -p inputforge-gui-dx --example component_gallery
dx serve --example component_gallery --platform desktop
```

Visually compare against the pre-refactor screenshot from Task 17a's verification. All interactive primitives must still respond to hover/focus/click. Tab through the gallery, focus order unchanged. No closure capture regressions (signals inside Switch, Checkbox, etc. should still update the visible state on click). If anything regresses, the suspect surface is just the Card wrappers added in step 1.

- [ ] **Step 3: Commit**

Invoke the `conventional-commits` skill, then commit:
```
refactor(gui-dx): wrap gallery sections in Card for visual consistency
```

---

## Task 18: Tooltip (CSS-only)

CSS-only via `:hover`/`:focus-within`, no JS positioning. Edge clipping is acceptable per spec risk note.

**Files:**
- Create: `src/components/tooltip.rs`
- Create: `assets/components/tooltip.css`
- Modify: mod / theme / gallery

- [ ] **Step 1: `src/components/tooltip.rs`**

```rust
use dioxus::prelude::*;

use super::merge_class;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TooltipPlacement { Top, Bottom, Left, Right }

#[component]
pub fn Tooltip(
    content: String,
    #[props(default = TooltipPlacement::Top)] placement: TooltipPlacement,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let p = match placement {
        TooltipPlacement::Top    => "if-tooltip--top",
        TooltipPlacement::Bottom => "if-tooltip--bottom",
        TooltipPlacement::Left   => "if-tooltip--left",
        TooltipPlacement::Right  => "if-tooltip--right",
    };
    let combined = merge_class("if-tooltip", p, class.as_deref());
    rsx! {
        span { class: "{combined}",
            {children}
            span { class: "if-tooltip__bubble", role: "tooltip", "{content}" }
        }
    }
}
```

- [ ] **Step 2: `assets/components/tooltip.css`**

```css
.if-tooltip { position: relative; display: inline-flex; }

.if-tooltip__bubble {
    position: absolute;
    background: var(--color-bg-overlay);
    color: var(--color-text);
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-sm);
    font-size: var(--text-xs);
    white-space: nowrap;
    pointer-events: none;
    opacity: 0;
    transition: opacity var(--duration-fast) var(--easing-standard);
    z-index: 1000;
    border: 1px solid var(--color-border);
}

.if-tooltip:hover .if-tooltip__bubble,
.if-tooltip:focus-within .if-tooltip__bubble { opacity: 1; }

.if-tooltip.if-tooltip--top    .if-tooltip__bubble { bottom: calc(100% + 4px); left: 50%; transform: translateX(-50%); }
.if-tooltip.if-tooltip--bottom .if-tooltip__bubble { top:    calc(100% + 4px); left: 50%; transform: translateX(-50%); }
.if-tooltip.if-tooltip--left   .if-tooltip__bubble { right:  calc(100% + 4px); top:  50%; transform: translateY(-50%); }
.if-tooltip.if-tooltip--right  .if-tooltip__bubble { left:   calc(100% + 4px); top:  50%; transform: translateY(-50%); }
```

- [ ] **Step 3: Wire mod / theme / gallery**

`src/components/mod.rs`:
```rust
pub mod tooltip;
pub use tooltip::{Tooltip, TooltipPlacement};
```

`src/theme/mod.rs`: add `TOOLTIP_CSS` const and mount.

Gallery section:
```rust
                section {
                    h2 { "Tooltip" }
                    Card {
                        div { style: "display: flex; gap: var(--space-6); align-items: center;",
                            Tooltip { content: "Hovers up".to_string(),    Button { "Top" } }
                            Tooltip { content: "Hovers down".to_string(),  placement: TooltipPlacement::Bottom, Button { "Bottom" } }
                            Tooltip { content: "Hovers left".to_string(),  placement: TooltipPlacement::Left,   Button { "Left" } }
                            Tooltip { content: "Hovers right".to_string(), placement: TooltipPlacement::Right,  Button { "Right" } }
                        }
                    }
                }
```

- [ ] **Step 4: Smoke + commit**

Verify hovering each Button reveals the bubble in the right position. Tab focus also reveals (focus-within).

Invoke the `conventional-commits` skill, then commit:
```
feat(gui-dx): add Tooltip primitive (CSS-only)
```

---

## Task 19: Menu (compound: MenuRoot / MenuTrigger / MenuItems / MenuItem)

The most complex primitive in F2. Click-outside, ESC, arrow nav.

**Files:**
- Create: `src/components/menu.rs` (4 components in one file)
- Create: `assets/components/menu.css`
- Modify: mod / theme / gallery

- [ ] **Step 1: `src/components/menu.rs`**

```rust
use dioxus::prelude::*;

use super::merge_class;

/// Shared open-state context for menu compound.
#[derive(Clone, Copy)]
struct MenuState { open: Signal<bool> }

#[component]
pub fn MenuRoot(
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let state = MenuState { open: use_signal(|| false) };
    use_context_provider(|| state);

    let combined = merge_class("if-menu", "", class.as_deref());
    rsx! { div { class: "{combined}", {children} } }
}

#[component]
pub fn MenuTrigger(children: Element) -> Element {
    let mut state = use_context::<MenuState>();
    rsx! {
        button {
            class: "if-menu__trigger",
            onclick: move |_| { let now = !*state.open.read(); state.open.set(now); },
            "aria-haspopup": "true",
            "aria-expanded": "{state.open.read()}",
            {children}
        }
    }
}

#[component]
pub fn MenuItems(children: Element) -> Element {
    let state = use_context::<MenuState>();
    let mut open_signal = state.open;
    if !*open_signal.read() {
        return rsx! {};
    }
    rsx! {
        div {
            class: "if-menu__items",
            role: "menu",
            tabindex: "-1",
            // Close on Escape (focus is on items by default after open).
            onkeydown: move |evt| {
                if evt.key() == Key::Escape { open_signal.set(false); }
            },
            // Backdrop captures outside clicks.
            div {
                class: "if-menu__backdrop",
                onclick: move |_| { open_signal.set(false); },
            }
            div { class: "if-menu__list", {children} }
        }
    }
}

#[component]
pub fn MenuItem(
    onclick: Option<EventHandler<MouseEvent>>,
    #[props(default)] disabled: bool,
    children: Element,
) -> Element {
    let mut state = use_context::<MenuState>();
    rsx! {
        button {
            class: "if-menu__item",
            role: "menuitem",
            disabled,
            onclick: move |evt| {
                if let Some(h) = &onclick { h.call(evt); }
                state.open.set(false);
            },
            {children}
        }
    }
}
```

(Arrow-key roving focus is intentionally minimal at F2, F15 polish can extend this. Tab still navigates between items, and ESC closes.)

- [ ] **Step 2: `assets/components/menu.css`**

```css
.if-menu { position: relative; display: inline-block; }

.if-menu__trigger {
    background: var(--color-bg-elevated);
    color: var(--color-text);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-2) var(--space-3);
    cursor: pointer;
    font-family: var(--font-sans);
}
.if-menu__trigger:hover  { border-color: var(--color-border-strong); }
.if-menu__trigger:focus-visible { outline: 2px solid var(--color-border-focus); outline-offset: 2px; }

.if-menu__backdrop {
    position: fixed;
    inset: 0;
    z-index: 999;
}
.if-menu__items {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    z-index: 1000;
}
.if-menu__list {
    position: relative;
    background: var(--color-bg-elevated);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-1);
    min-width: 160px;
    box-shadow: var(--shadow-2);
    display: flex;
    flex-direction: column;
}

.if-menu__item {
    background: transparent;
    border: none;
    color: var(--color-text);
    padding: var(--space-2) var(--space-3);
    text-align: left;
    cursor: pointer;
    border-radius: var(--radius-sm);
    font-family: var(--font-sans);
}
.if-menu__item:hover:not(:disabled) { background: var(--color-bg-sunken); }
.if-menu__item:focus-visible        { outline: 2px solid var(--color-border-focus); outline-offset: -2px; }
.if-menu__item:disabled             { opacity: 0.5; cursor: not-allowed; }
```

- [ ] **Step 3: Wire mod / theme / gallery**

`src/components/mod.rs`:
```rust
pub mod menu;
pub use menu::{MenuRoot, MenuTrigger, MenuItems, MenuItem};
```

`src/theme/mod.rs`: add `MENU_CSS` and mount.

Gallery section:
```rust
                section {
                    h2 { "Menu" }
                    Card {
                        MenuRoot {
                            MenuTrigger { "Open menu" }
                            MenuItems {
                                MenuItem { "First action" }
                                MenuItem { "Second action" }
                                MenuItem { disabled: true, "Disabled action" }
                            }
                        }
                    }
                }
```

- [ ] **Step 4: Smoke + commit**

Verify: clicking the trigger opens the panel; clicking an item closes it; clicking outside (the backdrop) closes it; pressing Escape closes it; disabled item does not respond.

Invoke the `conventional-commits` skill, then commit:
```
feat(gui-dx): add Menu compound primitive (MenuRoot/Trigger/Items/Item)
```

---

## Task 20: Form-wrapper pair, Field + Label

Field couples a label, an input area (children), helper text, and an error message. Label is the underlying type-consistent label primitive.

**Files:**
- Create: `src/components/{field,label}.rs`
- Create: `assets/components/{field,label}.css`
- Modify: mod / theme / gallery

- [ ] **Step 1: `src/components/label.rs`**

```rust
use dioxus::prelude::*;

use super::merge_class;

#[component]
pub fn Label(
    for_id: Option<String>,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let combined = merge_class("if-label", "", class.as_deref());
    rsx! {
        label {
            class: "{combined}",
            r#for: for_id.as_deref().unwrap_or(""),
            {children}
        }
    }
}
```

- [ ] **Step 2: `assets/components/label.css`**

```css
.if-label {
    display: inline-block;
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    color: var(--color-text);
}
```

- [ ] **Step 3: `src/components/field.rs`**

```rust
use dioxus::prelude::*;

use super::merge_class;
use crate::components::Label;

#[component]
pub fn Field(
    label: String,
    #[props(default)] for_id: Option<String>,
    #[props(default)] helper: Option<String>,
    #[props(default)] error: Option<String>,
    #[props(default)] required: bool,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let combined = merge_class("if-field", "", class.as_deref());
    rsx! {
        div { class: "{combined}",
            Label { for_id: for_id.clone(),
                "{label}"
                if required { span { class: "if-field__required", " *" } }
            }
            div { class: "if-field__control", {children} }
            if let Some(err) = error.as_deref() {
                span { class: "if-field__error", role: "alert", "{err}" }
            } else if let Some(h) = helper.as_deref() {
                span { class: "if-field__helper", "{h}" }
            }
        }
    }
}
```

- [ ] **Step 4: `assets/components/field.css`**

```css
.if-field { display: flex; flex-direction: column; gap: var(--space-1); }

.if-field__required { color: var(--color-error); }
.if-field__control  { display: flex; }
.if-field__helper   { font-size: var(--text-xs); color: var(--color-text-muted); }
.if-field__error    { font-size: var(--text-xs); color: var(--color-error); }
```

- [ ] **Step 5: Wire mod / theme / gallery**

`src/components/mod.rs`:
```rust
pub mod label;
pub mod field;
pub use label::Label;
pub use field::Field;
```

`src/theme/mod.rs`: add `LABEL_CSS` + `FIELD_CSS` consts + mounts.

Gallery section:
```rust
                section {
                    h2 { "Field + Label" }
                    Card {
                        div { style: "display: flex; flex-direction: column; gap: var(--space-4); max-width: 320px;",
                            Field { label: "Profile name".to_string(), helper: "Used in dropdowns.".to_string(), required: true,
                                TextInput { value: "".to_string(), placeholder: "My profile".to_string() }
                            }
                            Field { label: "Sensitivity".to_string(), error: "Must be between 0 and 1.".to_string(),
                                NumberInput { value: 1.5, min: 0.0, max: 1.0, step: 0.01 }
                            }
                        }
                    }
                }
```

- [ ] **Step 6: Smoke + commit**

Verify Field renders the label, the wrapped input, and either helper text OR error text (error takes precedence). Required marker shows the red asterisk.

Invoke the `conventional-commits` skill, then commit:
```
feat(gui-dx): add Field and Label form-wrapper primitives
```

---

## Task 21: Rewrite F1Readout using primitives + data-binding regression test

Demonstrates the design system on the F1 surface. Same six fields, same data, different presentation.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/app.rs`, rewrite `F1Readout`
- Modify: `crates/inputforge-gui-dx/src/context.rs`, add a unit test asserting `MetaSnapshot::from_state` + `ConfigSnapshot::from_state` produce the six values F1Readout displays for a known seeded `AppState`. (The existing F1 tests cover the factories generally; this new test pins the exact UI-data contract.)

- [ ] **Step 0: Pre-flight verification of upstream API surface**

The regression test in step 1 mutates `AppState` fields directly and the rewritten `F1Readout` matches on `EngineStatus` variants. Both assumptions held when this plan was written but should be verified before relying on them, `inputforge-core` may have evolved.

```bash
grep -n "pub enum EngineStatus" crates/inputforge-core/src/state/status.rs
grep -n "pub engine_status\|pub current_mode\|pub warnings\|pub devices\|pub virtual_devices" crates/inputforge-core/src/state/mod.rs
```

Expected:
- `EngineStatus` has variants `Running`, `Paused`, `Stopped` (no others). The `use_memo` match in step 2 is exhaustive against this set; **if a variant has been added since this plan was written, add a `_ => BadgeVariant::Neutral` wildcard arm to the match**.
- All five `AppState` fields used by the regression test (`engine_status`, `current_mode`, `warnings`, `devices`, `virtual_devices`) appear as `pub` in `state/mod.rs`. **If any have been wrapped in accessor methods**, swap field-access for the appropriate method calls in the test.

- [ ] **Step 1: Write the failing regression test**

Append to the `#[cfg(test)] mod tests` block in `crates/inputforge-gui-dx/src/context.rs`:

```rust
#[test]
fn f1_readout_data_binding_contract() {
    use inputforge_core::state::{AppState, EngineStatus, DeviceState};
    use inputforge_core::types::{AxisPolarity, DeviceId, DeviceInfo, VJoyAxis, VirtualDeviceConfig};

    let mut s = AppState::new();
    s.engine_status = EngineStatus::Running;
    "Combat".clone_into(&mut s.current_mode);
    s.warnings.push("low battery".to_owned());
    s.devices.push(DeviceState {
        info: DeviceInfo {
            id: DeviceId("dev-1".to_owned()),
            name: "Stick".to_owned(),
            axes: 2, buttons: 4, hats: 1,
            instance_path: None,
            axis_polarities: vec![AxisPolarity::Bipolar; 2],
        },
        connected: true,
    });
    s.virtual_devices.push(VirtualDeviceConfig {
        device_id: 1,
        axes: vec![VJoyAxis::X, VJoyAxis::Y],
        button_count: 4, hat_count: 1,
    });

    let meta = MetaSnapshot::from_state(&s);
    let cfg  = ConfigSnapshot::from_state(&s);

    // The exact six values F1Readout reads:
    assert_eq!(meta.engine_status, EngineStatus::Running);
    assert_eq!(meta.current_mode, "Combat");
    assert_eq!(meta.profile_name, None);   // no profile loaded
    assert_eq!(cfg.devices.len(), 1);
    assert_eq!(cfg.virtual_devices.len(), 1);
    assert_eq!(meta.warnings.len(), 1);
}
```

Run: `cargo test -p inputforge-gui-dx --lib context::tests::f1_readout_data_binding_contract`
Expected: PASS (the factories already work, this pins the contract so a future refactor can't silently change it).

- [ ] **Step 2: Rewrite `F1Readout` in `src/app.rs`**

Replace the entire `F1Readout` function body with:

```rust
#[component]
fn F1Readout() -> Element {
    let ctx = use_context::<AppContext>();

    let status_text = use_memo(move || format!("{:?}", ctx.meta.read().engine_status));
    let status_variant = use_memo(move || match ctx.meta.read().engine_status {
        inputforge_core::state::EngineStatus::Running => crate::components::BadgeVariant::Success,
        inputforge_core::state::EngineStatus::Paused  => crate::components::BadgeVariant::Warning,
        inputforge_core::state::EngineStatus::Stopped => crate::components::BadgeVariant::Neutral,
    });
    let mode = use_memo(move || ctx.meta.read().current_mode.clone());
    let profile = use_memo(move || {
        ctx.meta.read().profile_name.clone().unwrap_or_else(|| "<none>".into())
    });
    let devices  = use_memo(move || ctx.config.read().devices.len());
    let vdevices = use_memo(move || ctx.config.read().virtual_devices.len());
    let warnings = use_memo(move || ctx.meta.read().warnings.len());
    let warnings_variant = use_memo(move || {
        if *warnings.read() == 0 { crate::components::BadgeVariant::Neutral }
        else { crate::components::BadgeVariant::Warning }
    });

    rsx! {
        main {
            style: "padding: var(--space-6); display: flex; flex-direction: column; gap: var(--space-4);",
            h1 { "InputForge, Dioxus (F1 bridge smoke test)" }
            crate::components::Card { padding: crate::components::CardPadding::Md,
                div { style: "display: grid; grid-template-columns: max-content 1fr; gap: var(--space-2) var(--space-4);",
                    crate::components::Label { for_id: None::<String>, "Engine status:" }
                    div { crate::components::Badge { variant: *status_variant.read(), "{status_text}" } }

                    crate::components::Label { for_id: None::<String>, "Current mode:" }
                    div { strong { "{mode}" } }

                    crate::components::Label { for_id: None::<String>, "Active profile:" }
                    div { "{profile}" }

                    crate::components::Label { for_id: None::<String>, "Connected devices:" }
                    div { "{devices}" }

                    crate::components::Label { for_id: None::<String>, "Virtual devices:" }
                    div { "{vdevices}" }

                    crate::components::Label { for_id: None::<String>, "Warnings:" }
                    div { crate::components::Badge { variant: *warnings_variant.read(), "{warnings}" } }
                }
            }
            small { style: "color: var(--color-text-muted);", "Tray wiring: stubbed (F3). Theme: F2 ✓. Layout: F3." }
        }
    }
}
```

- [ ] **Step 3: Confirm the regression test still passes**

Run: `cargo test -p inputforge-gui-dx`
Expected: all tests PASS, including `f1_readout_data_binding_contract`.

- [ ] **Step 4: Smoke under full app**

Run: `cargo run -p inputforge-app --no-default-features --features gui-dioxus`
Expected: window opens with the new themed F1 readout, Card, Badges, Labels, bound to the same six fields. The success badge is green (engine running), warning badge is neutral if zero warnings.

If the engine is `Stopped`, status badge reads `Stopped` with neutral styling. Cycle the engine to verify badge colors update.

- [ ] **Step 5: Commit**

Invoke the `conventional-commits` skill, then commit:
```
feat(gui-dx): rewrite F1Readout using design-system primitives
```

---

## Task 22: README updates

**Files:**
- Modify: `crates/inputforge-gui-dx/README.md`

- [ ] **Step 1: Append four sections to README**

Append to the existing README (after the F1 build/run matrix section). Sections:

```markdown
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

`app_root` already wraps `F1Readout` in `ThemeProvider`, every screen
mounted under `app_root` inherits it.

## Adding a new icon

1. Drop the `.svg` file under `src/icons/svg/<name>.svg` (Phosphor regular weight, 24×24 viewBox).
2. Add a variant to the `Icon` enum in `src/icons/mod.rs`.
3. Add a match arm in `Icon::svg()` mapping the variant to `include_str!("svg/<name>.svg")`.
4. Run `cargo test -p inputforge-gui-dx --lib icons::tests`, the well-formedness test will catch corrupt files.

## Toolchain prerequisites

- `dx` (dioxus-cli) version 0.7.6, install via `cargo install dioxus-cli --version 0.7.6`. Required for hot-reload (`dx serve`).
- WebView2 runtime, bundled with Windows 11. On Windows 10 or earlier, install the Evergreen Standalone runtime from https://developer.microsoft.com/microsoft-edge/webview2/.
```

- [ ] **Step 2: Commit**

Invoke the `conventional-commits` skill, then commit:
```
docs(gui-dx): document gallery, ThemeProvider, icon-add workflow, toolchain prereqs
```

---

## Task 23: Final verification (spec checks 1-11)

Pure verification, no code changes unless a check fails (in which case fix and re-run).

- [ ] **Check 1: No new warnings in `inputforge-gui-dx` build**

Capture F1 baseline warning count (use `git stash`, `git checkout` the F1 tip, run, count, then return):
```bash
cargo build -p inputforge-gui-dx 2>&1 | grep -c '^warning:'
```
Expected: count is **less than or equal** to F1 count. (Proc-macro / transitive warnings are out of scope per spec.)

- [ ] **Check 2: gui-dioxus app build**
```bash
cargo build -p inputforge-app --no-default-features --features gui-dioxus
```
Expected: builds cleanly.

- [ ] **Check 3: gui-egui default build (regression check)**
```bash
cargo build -p inputforge-app
```
Expected: builds cleanly. Default feature is still `gui-egui`.

- [ ] **Check 4: F1 context tests still pass**
```bash
cargo test -p inputforge-gui-dx
```
Expected: all tests PASS, including the new `f1_readout_data_binding_contract`.

- [ ] **Check 5: Gallery opens and renders**
```bash
dx serve --example component_gallery --platform desktop
```
Expected: window opens; sections for all 17 primitives render; every variant visible. Leave running for checks 6 and 10.

- [ ] **Check 6: Manual interaction pass**

Inside the gallery window, verify each interactive primitive:
- Hover changes the visual state.
- Tab moves focus through every interactive primitive in document order.
- Focus ring is visible and uses the `--color-border-focus` color.
- Click triggers the active visual state.
- Disabled primitives are visually distinct AND do not respond to interaction.

- [ ] **Check 7: F1Readout regression pass**

Stop the gallery. Run the full app under gui-dioxus:
```bash
cargo run -p inputforge-app --no-default-features --features gui-dioxus
```
Verify the readout shows the same six fields with the same data semantics as F1, just styled with Card / Badge / Label. Stop the app.

- [ ] **Check 8: Frontend-design output committed**

Run: `git log --oneline -- crates/inputforge-gui-dx/assets/tokens/`
Expected: at least one commit reading "apply frontend-design revised token values" (from Task 10). Confirms the visual-direction sign-off is in history.

- [ ] **Check 9: ThemeProvider export check**
```bash
grep -E '^pub (use|mod) (theme|components)' crates/inputforge-gui-dx/src/lib.rs
```
Expected: lines for `pub mod theme;` and `pub mod components;`. Then verify the example builds against the public surface (already covered by check 5; this is the grep belt-and-braces).

- [ ] **Check 10: Asset pipeline DevTools probe**

Re-run the gallery: `dx serve --example component_gallery --platform desktop`. Open DevTools (Ctrl+Shift+I or right-click → Inspect). In the Console run:

```javascript
getComputedStyle(document.body).getPropertyValue('--color-bg')      // non-empty
getComputedStyle(document.body).fontFamily                          // includes "Inter"
[...document.styleSheets].some(s => [...s.cssRules].some(r => r.selectorText && r.selectorText.includes('.if-button')))
                                                                    // true
document.querySelectorAll('.if-icon svg').length                    // > 0
```

All four expectations must be met. Close.

- [ ] **Check 11: License attribution check**
```bash
ls crates/inputforge-gui-dx/THIRD_PARTY_LICENSES.md
grep -E '(Phosphor|Inter|JetBrains Mono)' crates/inputforge-gui-dx/THIRD_PARTY_LICENSES.md
```
Expected: file exists; grep returns at least three matching lines.

- [ ] **Final commit (if any cosmetic fix landed during verification)**

Invoke the `conventional-commits` skill if a fix was needed. Otherwise verification is read-only, no commit.

F2 complete. Hand off to F3 shell.

---

## Self-Review

The plan covers every section of the spec:

- File layout (assets/ + src/) → Tasks 1, 6, 7, 11-20
- ThemeProvider wiring → Tasks 2, 3, 8 + every primitive task adds a stylesheet mount
- Token system (six files + global) → Tasks 6, 7, with revision in Task 10
- 17 component primitives → Tasks 13 (Icon) + 14-20 (16 in 7 grouped tasks; Task 17 splits into 17a for new primitives + 17b for the Card-wrap refactor of prior gallery sections)
- Icon strategy → Tasks 11-13
- Test harness (`component_gallery.rs`) → Task 5 (skeleton) + every primitive task appends sections; Task 17b restructures existing sections to use Card
- Frontend-design integration → Tasks 9, 10 (with explicit screenshot capture and brief)
- Critical files to modify → covered in "Critical Files To Modify" section above
- Verification (11 checks) → Task 23 (plus a supplementary cascade-order check at Task 8 sub-step 6c)
- THIRD_PARTY_LICENSES.md → Task 4 (lifted to Phase 0 to prevent legal-compliance window); font-swap propagation rules in Task 10

Code-review fixes applied (2026-04-25): icon CSS aligned with Phosphor regular (fill-based, not stroke); `merge_class` upgraded to skip empty parts and now used by every primitive; Task 17 split into 17a/17b for risk isolation; pre-flight verification of `EngineStatus` variants and `AppState` field visibility added to Task 21; cascade-order check added to Task 8; manganis fallback made atomic; Task 1 commit message corrected (empty dirs not tracked); BOM-defensive SVG checks; dedicated `SpinnerSize` enum.

No placeholders detected. Type names referenced across tasks are consistent (`InputSize` defined in Task 15, reused for input/select/number-input sizing; `SpinnerSize` is its own enum in Task 17a; `BadgeVariant` defined in Task 17a, used in Task 21 F1Readout; `merge_class` lifted from `components::icon` to `components::mod` in Task 14 step 1, imported via `use super::merge_class;` from each primitive). Method signatures match across tasks.

---

## Post-review fixes (2026-04-26)

After the F2 branch reached 28 commits, an end-of-branch code review (`/superpowers:requesting-code-review`) flagged 4 Important and 7 Minor issues plus a few impeccable cross-cutting concerns. All addressed in this branch before merge:

**Important**
- [x] **I1**, Checkbox indeterminate now syncs the DOM `.indeterminate` IDL property via `use_effect` + `document::eval`, plus `aria-checked="mixed"`. Generates an internal stable id when no caller `id` is supplied. (`src/components/checkbox.rs`)
- [x] **I2**, Menu compound now handles ArrowDown/ArrowUp/Home/End keyboard nav and auto-focuses the first item when opening, via a small embedded JS focus-walker. (`src/components/menu.rs`)
- [x] **I3**, Badge backgrounds now reference new `--color-{info,success,warning,error}-bg` tokens; rgba literals removed. Token RGB matches the revised palette. (`assets/tokens/colors.css`, `assets/components/badge.css`)
- [x] **I4**, TextInput, NumberInput, Select, Slider, and Label now render `id`/`for` only when the prop is `Some`, eliminating HTML5-invalid empty-string attributes on default usage. (5 files under `src/components/`)

**Minor**
- [x] **M1**, `IconButton` now delegates to `ButtonVariant::class_for(prefix)` / `ButtonSize::class_for(prefix)` instead of duplicating match arms; exhaustive class-delegation tests added. (`src/components/{button,icon_button}.rs`)
- [x] **M2**, `IconButton` ghost hover background switched from `--color-bg-elevated` to `--color-border` so hover stays visible inside Card. (`assets/components/icon-button.css`)
- [x] **M3**, Inter-Bold v4.1 ttf shipped; `@font-face` block added; faux-bold warning comment removed; `THIRD_PARTY_LICENSES.md` lists the new file. (`assets/fonts/Inter-Bold.ttf`, `assets/tokens/typography.css`, `THIRD_PARTY_LICENSES.md`)
- [x] **M4**, `MenuItems` is now always-mounted with the HTML `hidden` attribute when closed; `MenuTrigger` advertises `aria-controls=<menu-id>`. (`src/components/menu.rs`)
- [x] **M5**, Slider/Switch raw px replaced with new `--control-size-{sm,md,lg}` + `--control-track-h` + `--control-border-w` tokens. (`assets/tokens/spacing.css`, `assets/components/{slider,switch}.css`)
- [x] **M6**, Stack/Cluster/Inset layout primitives extracted; F1Readout and component_gallery layout sites migrated. (`src/components/layout.rs`, `assets/components/layout.css`, `src/app.rs`, `examples/component_gallery.rs`)
- [x] **M7**, Tooltip overlays now apply `backdrop-filter: blur(8px)` so the translucency reads as a layer over Card surfaces. (`assets/components/tooltip.css`)

**Impeccable cross-cutting**
- [x] **A**, `prefers-reduced-motion` media query in `motion.css` zeroes `--duration-*` tokens. Component CSS that pipes durations through tokens disables motion automatically.
- [x] **B**, WCAG AA verification documented inline in `colors.css` for each new `*-bg` token (≥4.5:1 against `--color-text` on the composited surface).
- [x] **C**, `impeccable:teach-impeccable` skipped in favor of inline rationale comments on every new token (palette intent, motion philosophy, geometry tokens), same outcome with no tooling dependency.

Verification: `cargo check`, `cargo test` (22 lib tests), `cargo build --example component_gallery`, `cargo build -p inputforge-app` (default + `--no-default-features --features gui-dioxus`), `cargo clippy -- -D warnings` all clean.
