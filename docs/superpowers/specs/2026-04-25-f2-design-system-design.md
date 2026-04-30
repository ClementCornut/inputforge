# F2, Design System & Theme: Design Spec

**Status:** Design approved, ready for implementation plan
**Date:** 2026-04-25
**Parent spec:** `docs/superpowers/specs/2026-04-24-egui-to-dioxus-rewrite-design.md` (egui → Dioxus master plan)
**Feature:** F2 (Foundation, second sequential step after F1)

---

## Context

F2 establishes the visual language and primitive component library for the Dioxus rewrite of `inputforge-gui`. F1 left `crates/inputforge-gui-dx` as a state-bridge scaffold (signals, 60Hz polling, `AppContext` provider) with **zero styling infrastructure**, only one inline-style placeholder component, no CSS file, no assets, no design tokens, no reusable components.

Every subsequent feature (F3 shell through F14 profile surface) builds on F2's tokens and primitives. F2 must therefore be:

- **Theme-ready** (semantic naming so a future light theme is an override block, not a refactor)
- **Complete** in primitives so later screens compose without revisiting F2
- **Production-grade** in personality, first impression of the new GUI

**Visual direction:** "Evolved Glass Cockpit." The existing egui identity (dark navy `#1A1A2E` base, semantic accent quintet, Inter + JetBrains Mono fonts, 8/4 spacing, 6px radius, instrument-cluster aesthetic) is the baseline. `impeccable:frontend-design` is invited to challenge every aspect, colors, type scale, spacing rhythm, motion, fonts. The brief is "evolve, don't replicate."

**Why now:** F2 is sequential foundation per the master plan. Without it, F3 has no atomic vocabulary to build the shell from, and frontend-design has no scaffold to operate on.

---

## File Layout

All paths relative to `crates/inputforge-gui-dx/`.

```
assets/
  tokens/
    colors.css          :root semantic color variables (dark default; theme-ready)
    typography.css      @font-face declarations + type scale variables
    spacing.css         --space-{0..16} 4px-base scale
    radii.css           --radius-{none,sm,md,lg,full}
    elevation.css       --shadow-{0,1,2,3} (used sparingly)
    motion.css          --duration-* and --easing-*
  global.css            body baseline, box-sizing reset, scrollbar styling
  components/
    button.css
    icon-button.css
    text-input.css
    number-input.css
    select.css
    slider.css
    switch.css
    checkbox.css
    card.css
    badge.css
    tooltip.css
    menu.css
    separator.css
    field.css
    label.css
    spinner.css
  fonts/
    Inter-Regular.ttf            (copied from crates/inputforge-gui/assets/fonts/)
    Inter-SemiBold.ttf
    JetBrainsMono-Regular.ttf
src/
  theme/
    mod.rs              ThemeProvider component (mounts all stylesheets)
  components/
    mod.rs              re-exports
    button.rs · icon_button.rs · text_input.rs · number_input.rs ·
    select.rs · slider.rs · switch.rs · checkbox.rs · card.rs ·
    badge.rs · tooltip.rs · menu.rs · separator.rs · field.rs ·
    label.rs · spinner.rs · icon.rs
  icons/
    mod.rs              pub enum Icon + impl Icon { fn svg(&self) -> &'static str }
    svg/
      joystick.svg · device.svg · axis.svg · button.svg · hat.svg · mode.svg ·
      profile.svg · save.svg · copy.svg · eye.svg · eye-off.svg · link.svg ·
      plus.svg · minus.svg · trash.svg · settings.svg · chevron-down.svg ·
      chevron-up.svg · chevron-left.svg · chevron-right.svg · x.svg ·
      check.svg · warning.svg · info.svg · error.svg · play.svg · pause.svg ·
      refresh.svg · drag-handle.svg · dots-vertical.svg
examples/
  component_gallery.rs  visual harness for all 17 primitives + every variant/state
```

**ThemeProvider** is mounted at the root of `app_root` (in `app.rs`, replacing the inline-styled `F1Readout`). It uses Dioxus's `document::Stylesheet` component to load each token CSS file, then `global.css`, then each component CSS file. Load order matters: colors → typography → spacing → radii → elevation → motion → **global** → components.

**Asset registration (Dioxus 0.7 / `manganis`).** Each CSS file, font, and any runtime-fetched asset is registered with the `asset!("/...")` proc macro at its use site. There is **no** `Cargo.toml` `assets = [...]` field in Dioxus 0.7, that's not how the asset pipeline works. Stylesheets mount via `document::Stylesheet`; fonts referenced by `@font-face` use `asset!()` in their `src: url(...)` value. Asset paths are absolute from the crate root and start with `/`.

Sketch (illustrative, final API per `dioxus-document::Stylesheet` and `manganis::asset!`):

```rust
// theme/mod.rs
use dioxus::prelude::*;

const COLORS_CSS:     Asset = asset!("/assets/tokens/colors.css");
const TYPOGRAPHY_CSS: Asset = asset!("/assets/tokens/typography.css");
// … all token files …
const GLOBAL_CSS:     Asset = asset!("/assets/global.css");
const BUTTON_CSS:     Asset = asset!("/assets/components/button.css");
// … all component files …

#[component]
pub fn ThemeProvider(children: Element) -> Element {
    rsx! {
        document::Stylesheet { href: COLORS_CSS }
        document::Stylesheet { href: TYPOGRAPHY_CSS }
        // … remaining token files in order …
        document::Stylesheet { href: GLOBAL_CSS }
        document::Stylesheet { href: BUTTON_CSS }
        // … remaining component files …
        {children}
    }
}
```

```css
/* tokens/typography.css */
@font-face {
    font-family: 'Inter';
    src: url('/assets/fonts/Inter-Regular.ttf') format('truetype');
    font-weight: 400;
    font-display: swap;
}
```

---

## Token System

All tokens defined in `:root` selectors. Naming is semantic (intent), never raw (value). Dark theme is the default. Light theme is **not shipped in F2** but the system is structured so adding `:root[data-theme="light"] { … }` later requires no component CSS changes.

### colors.css

| Group | Tokens |
|---|---|
| Surface | `--color-bg`, `--color-bg-elevated`, `--color-bg-sunken`, `--color-bg-overlay` |
| Text | `--color-text`, `--color-text-muted`, `--color-text-subtle`, `--color-text-inverse` |
| Border | `--color-border`, `--color-border-strong`, `--color-border-focus` |
| Action | `--color-primary`, `--color-primary-hover`, `--color-primary-active`, `--color-primary-fg` |
| Status | `--color-live`, `--color-warning`, `--color-error`, `--color-info` |
| Categories | `--color-processing`, `--color-output`, `--color-control`, **three distinct accent hues** (not aliases of action/status) |
| Each accent | `-bg` (subtle tint for backgrounds), `-fg` (text on solid accent) variants |

Placeholder values for action/status tokens seeded from existing egui DARK palette (`#1A1A2E`, `#4A9EFF`, `#00E5A0`, `#FFB347`, `#FF6B6B`, `#B07FFF`).

**On the category tokens:** the egui implementation aliases them (`processing → primary`, `output → live`, `control → special`, see `crates/inputforge-gui/src/theme.rs:64-76`). F2 explicitly **does not** carry that aliasing: `--color-processing`, `--color-output`, `--color-control` are reserved as three independent accent hues. Placeholders match the egui aliases for parity, but `impeccable:frontend-design` is **explicitly authorized** to pick three hues that complement (and don't overlap) the action/status palettes. Components reference categories by name (`--color-processing` etc.), never by indirection.

### typography.css

```
--font-sans: 'Inter', system-ui, -apple-system, sans-serif;
--font-mono: 'JetBrainsMono', ui-monospace, 'Cascadia Code', monospace;

--text-xs:   12px;   --text-sm:  13px;   --text-base: 14px;
--text-md:   15px;   --text-lg:  18px;   --text-xl:   22px;   --text-2xl: 28px;

--weight-regular:   400;
--weight-medium:    500;
--weight-semibold:  600;

--leading-tight:    1.2;
--leading-base:     1.5;
--leading-relaxed:  1.7;
```

`@font-face` declarations reference `assets/fonts/*.ttf` via `manganis::asset!()` (see asset registration sketch above). Each declaration includes `font-display: swap`, desktop wry loads TTFs synchronously from disk so flash-of-unstyled-text is rare in practice, but the explicit hint keeps behavior predictable. TTFs were chosen over WOFF2 to reuse the existing `crates/inputforge-gui/assets/fonts/` files unchanged; payload size is not a concern on desktop.

### spacing.css

4px-base scale: `--space-0` (0) · `--space-1` (4px) · `--space-2` (8px) · `--space-3` (12px) · `--space-4` (16px) · `--space-6` (24px) · `--space-8` (32px) · `--space-12` (48px) · `--space-16` (64px).

### radii.css

`--radius-none` (0) · `--radius-sm` (3px) · `--radius-md` (6px, matches existing egui) · `--radius-lg` (10px) · `--radius-full` (9999px).

### elevation.css

`--shadow-0` (none) · `--shadow-1` (subtle, 1-2px) · `--shadow-2` (cards) · `--shadow-3` (overlays). Used sparingly, cockpit aesthetic favors borders over shadows. Frontend-design may flatten further.

### motion.css

`--duration-fast` (120ms) · `--duration-base` (180ms) · `--duration-slow` (260ms).
`--easing-standard` (`cubic-bezier(0.2, 0, 0, 1)`) · `--easing-emphasized` (`cubic-bezier(0.3, 0, 0, 1)`).

### global.css

Defines the **moderate baseline** that lets components rely on a known starting state without inline body styles. Loaded **after** all tokens, **before** any component CSS, so component CSS can override it cheaply if needed.

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

h1, h2, h3, h4, h5, h6, p { margin: 0; }      /* component CSS opts in to spacing via tokens */

/* Custom scrollbar, Webkit + Firefox */
::-webkit-scrollbar { width: 10px; height: 10px; }
::-webkit-scrollbar-track { background: var(--color-bg-elevated); }
::-webkit-scrollbar-thumb { background: var(--color-border-strong); border-radius: var(--radius-sm); }
* { scrollbar-color: var(--color-border-strong) var(--color-bg-elevated); scrollbar-width: thin; }
```

No reset beyond this, components own their own margins and paddings via spacing tokens.

---

## Component Primitives (17)

Each lives in `src/components/<name>.rs` with sibling `assets/components/<name>.css`. All accept a `class: Option<String>` prop for caller composition. CSS uses BEM-ish prefix `.if-<name>` (e.g., `.if-button`, `.if-button--primary`, `.if-button__icon`).

| Component | Props (sketch) | Notes |
|---|---|---|
| `Icon` | `name: Icon` (enum), `size: IconSize` (sm/md/lg) | Renders SVG via `dangerous_inner_html`; lookup via `Icon::svg()` |
| `Button` | `variant` (primary/secondary/ghost/danger), `size`, `disabled`, `onclick`, children | Styled `<button>`; default size md |
| `IconButton` | `icon: Icon`, `label: &'static str` (a11y), `variant`, `size`, `disabled`, `onclick` | Square; `aria-label` mandatory |
| `TextInput` | `value`, `oninput`, `placeholder`, `disabled`, `invalid`, `size` | Controlled `<input type="text">` |
| `NumberInput` | `value: f64`, `oninput`, `min`, `max`, `step`, `precision`, `disabled` | `<input type="number">` + stepper buttons |
| `Select` | `value`, `onchange`, `options: Vec<(K, String)>`, `disabled` | Native `<select>`; custom listbox deferred |
| `Slider` | `value: f64`, `oninput`, `min`, `max`, `step`, `disabled` | Styled `<input type="range">` |
| `Switch` | `checked`, `onchange`, `disabled`, `label: Option<String>` | Two-state toggle |
| `Checkbox` | `checked`, `onchange`, `disabled`, `indeterminate` | Standard checkbox + visual tri-state |
| `Card` | `padding: CardPadding` (sm/md/lg), children | Surface wrapper using `--color-bg-elevated` |
| `Badge` | `variant` (neutral/info/success/warning/error), children | Small inline tag |
| `Tooltip` | `content: String`, `placement`, children | CSS-only via `:hover`/`:focus-within`; no JS positioning lib in F2 |
| `Menu` | compound: `MenuRoot` + `MenuTrigger` + `MenuItems` + `MenuItem` | Click-outside, ESC, keyboard arrow nav |
| `Separator` | `orientation` (horizontal/vertical) | Styled `<hr>` or `<div role="separator">` |
| `Field` | `label`, `helper`, `error`, `required`, children | Form wrapper; couples label↔input by id; renders error text when present |
| `Label` | `for_id`, children | Bare typography-consistent label |
| `Spinner` | `size` | Pure-CSS rotation; `aria-busy` |

**State coverage required for every interactive primitive:** default, hover, focus-visible, active, disabled, invalid (where applicable). All focus rings use `--color-border-focus`. Force-state CSS hooks (e.g., `data-force-state="hover"`) for static-state documentation in the gallery are **deferred to F15 polish**, F2 verification relies on manual interaction in the gallery.

---

## Icon Strategy

- **Source:** Phosphor Icons (MIT). Regular weight default; fill weight where active states benefit.
- **Storage:** raw `.svg` files in `src/icons/svg/`. Loaded via `include_str!()`. No string-escaping ceremony when adding new icons, drop the `.svg` file in.
- **Registry:** `src/icons/mod.rs` exposes:
  ```rust
  pub enum Icon { Joystick, Device, Axis, Button, /* … */ }
  impl Icon {
      pub fn svg(&self) -> &'static str {
          match self {
              Icon::Joystick => include_str!("svg/joystick.svg"),
              /* … */
          }
      }
  }
  ```
- **Rendering:** `<Icon name=Icon::Joystick size=IconSize::Md />` outputs `<span class="if-icon if-icon--md" dangerous_inner_html=Icon::Joystick.svg() />`.
- **Caveats of the `include_str!()` approach:**
  - **Compile-time only.** Editing an SVG file triggers a rebuild, not Dioxus hot-reload. The gallery's RSX/CSS hot-reloads (see Test Harness below); icon SVGs do not. Acceptable since icon edits are rare and intentional.
  - **Trusted-source only.** SVGs go through `dangerous_inner_html` unsanitized. Phosphor icons are trusted upstream (committed source files, not user input). **Never feed user-provided SVG through this path.**
  - **Why not `asset!()` for icons?** `asset!()` would enable hot-reload and consistent asset handling, but require a path-based icon registry instead of the simpler enum-with-content registry. For ~30 trusted static SVGs, `include_str!()` is the right tradeoff.
- **Initial set (~30 icons):** device, joystick, axis, button, hat, mode, profile, save, copy, eye, eye-off, link, plus, minus, trash, settings, chevron-{up,down,left,right}, x, check, warning, info, error, play, pause, refresh, drag-handle, dots-vertical. Set grows per feature.
- **License attribution.** All third-party assets are credited in `crates/inputforge-gui-dx/THIRD_PARTY_LICENSES.md`:
  - Phosphor Icons (MIT)
  - Inter (SIL Open Font License 1.1)
  - JetBrains Mono (SIL Open Font License 1.1)

  The implementation plan adds creation of this file as a discrete sub-task.

---

## Test Harness

`examples/component_gallery.rs`, sibling to existing `bridge_demo.rs`.

- Single scrollable page, sectioned per primitive.
- Each section shows every variant × every interaction state (default/hover/focus/disabled/invalid where relevant).
- Mounts `ThemeProvider`. No engine state required.
- Run: `dx serve --example component_gallery --platform desktop`.
- Hot-reload friendly, editing CSS or RSX updates instantly.

The gallery doubles as visual regression reference for F15 polish work.

---

## Frontend-Design Integration

`impeccable:frontend-design` is invoked **early in F2 implementation**, after the file structure and token-file *skeletons* are scaffolded with placeholder values, and **before** component CSS is written.

**Brief delivered to frontend-design:**
- This design doc.
- Screenshots of the existing egui GUI (run `cargo run --features gui-egui` and capture).
- The product persona summary (serious sim/HOTAS configuration tool, expert users, instrument-cluster heritage).
- Existing semantic structure to preserve at the **token name** level (categories `processing/output/control`, status quintet), but not necessarily their values.
- Explicit permission to challenge: color values, accent saturation, type scale, spacing rhythm, motion language, font choice, elevation philosophy.

**Specific questions for frontend-design to answer:**
- The placeholder type scale (12/13/14/15/18/22/28) clusters four sizes within 6px, please rationalize. We expect 5-6 sizes max, with clearer hierarchy steps.
- We've reserved `--color-processing` / `--color-output` / `--color-control` as **three distinct accent hues** (not aliases of action/status). Please pick three that complement the action and status palettes without overlap.
- Is `--radius-md: 6px` right for the "evolved glass cockpit" aesthetic, or should the radii family shift up or down?
- Confirm or revise the elevation philosophy ("borders over shadows", egui heritage). Cockpit aesthetic suggests minimal shadows; verify.
- Motion: are the durations (120/180/260ms) and easings appropriate for an instrument-cluster aesthetic, or do they read too consumer-app?

**Output of frontend-design:** finalized values for `colors.css`, `typography.css`, `spacing.css`, `radii.css`, `elevation.css`, `motion.css` and a written rationale answering each question above.

**Why this ordering matters:** writing component CSS against placeholder tokens and then revising them post-frontend-design causes rework. Locking tokens before component CSS minimizes that.

---

## Critical Files To Modify

- **New:** all files under `crates/inputforge-gui-dx/assets/` (tokens, `global.css`, components, fonts).
- **New:** all files under `crates/inputforge-gui-dx/src/components/` and `src/icons/`.
- **New:** `crates/inputforge-gui-dx/src/theme/mod.rs`, defines the `pub` `ThemeProvider` component; re-exported from `lib.rs` so the gallery example (which constructs its own root) can mount it.
- **New:** `crates/inputforge-gui-dx/examples/component_gallery.rs`.
- **New:** `crates/inputforge-gui-dx/THIRD_PARTY_LICENSES.md`, Phosphor MIT + Inter OFL-1.1 + JetBrains Mono OFL-1.1 attributions.
- **Modify:** `crates/inputforge-gui-dx/src/app.rs`, replace inline-styled `F1Readout` with `ThemeProvider` wrapping the existing children; F1Readout itself rewritten using new primitives (Card, Badge, etc.) so the smoke-test screen demonstrates the design system.
- **Modify:** `crates/inputforge-gui-dx/src/lib.rs`, add `pub mod theme;` and `pub mod components;` (and the corresponding `pub use` re-exports) so both `app::app_root` *and* the standalone `examples/component_gallery.rs` binary can import `ThemeProvider` and primitives. **`launch_gui` itself is unchanged**, only the public surface grows.
- **Modify:** `crates/inputforge-gui-dx/Cargo.toml`, no `assets = [...]` field needed (assets are registered via `manganis::asset!()` at use sites). Add `manganis` as a direct dep only if it isn't already a transitive of `dioxus`; otherwise no Cargo.toml change at all.
- **Modify:** `crates/inputforge-gui-dx/README.md`, document the gallery example, theme provider usage, how to add a new icon, and the `dx serve` / WebView2 prerequisites (see Risks).

**Reused (do not modify) from F1:**
- `crates/inputforge-gui-dx/src/context.rs`, `AppContext`, snapshots
- `crates/inputforge-gui-dx/src/bridge.rs`, polling task

---

## Verification

End-to-end checks before declaring F2 complete:

1. `cargo build -p inputforge-gui-dx`, builds with **no new warnings** vs. the F1 baseline. Use `cargo build -p inputforge-gui-dx 2>&1 | grep -c '^warning:'` and compare against the F1 count rather than asserting zero (proc-macro and transitive warnings are out of scope).
2. `cargo build -p inputforge-app --no-default-features --features gui-dioxus`, app builds with Dioxus GUI.
3. `cargo build -p inputforge-app`, egui GUI still default and unchanged (no regression).
4. `cargo test -p inputforge-gui-dx`, F1 context tests still pass.
5. `dx serve --example component_gallery --platform desktop`, gallery window opens, all 17 primitives render in their sections, all variants visible, all states demonstrable by interaction.
6. **Manual interaction pass on gallery:** every interactive primitive responds to hover/focus/click/keyboard appropriately; tab key navigates focus through all primitives in document order; focus ring is visible on all and uses `--color-border-focus`; disabled state visually distinct from enabled.
7. **F1Readout regression pass:** under `gui-dioxus`, F1Readout still renders the same six fields (engine status, current mode, active profile, connected devices, virtual devices, warnings count) bound to the same `MetaSnapshot` / `ConfigSnapshot` data as F1. Visual styling changes (now uses `Card` / `Badge` primitives instead of inline styles) are expected; **data binding must be byte-identical** for the same seeded state.
8. **Visual direction signed off:** the frontend-design output (revised token values) has been reviewed and committed.
9. **ThemeProvider export check.** Either `cargo doc -p inputforge-gui-dx --open` shows `ThemeProvider` in the public surface, or a `grep -E '^pub (use|mod) (theme|components)' lib.rs` finds the re-exports. The example `component_gallery` must compile against this public surface, not internal `pub(crate)` symbols.
10. **Asset pipeline smoke check.** With the gallery running and DevTools attached: `getComputedStyle(document.body).getPropertyValue('--color-bg')` resolves non-empty (token CSS loaded), `getComputedStyle(document.body).fontFamily` includes `Inter` (font asset loaded), at least one `.if-button` rule applies (component CSS loaded), and at least one inline `<svg>` from an icon renders.
11. **License attribution check.** `crates/inputforge-gui-dx/THIRD_PARTY_LICENSES.md` exists and lists Phosphor (MIT) + Inter (OFL-1.1) + JetBrains Mono (OFL-1.1).

---

## Out of Scope (Deferred)

- **Light theme values**, semantic structure ready; values not shipped in F2.
- **Tabs / Radio / RadioGroup**, defer to first consumer (Tabs likely F3, Radio likely F8).
- **Toast / Dialog**, F4's responsibility.
- **Toolbar / SplitPane / StatusBar** composites, F3 shell composes these from F2 primitives.
- **Custom listbox**, native `<select>` until UX demands more.
- **Rich motion / animation**, F2 has minimal CSS transitions only; F15 polish.
- **Comprehensive a11y audit**, F2 ships semantic HTML and focus rings; full audit at F15.
- **Per-component visual regression tests**, testing story is an open question for the rewrite (see master plan); F2 ships the gallery as a manual-review artifact, not an automated harness.
- **Force-state CSS hooks**, `data-force-state="hover"` / `="focus-visible"` / `="active"` selectors that mirror live state CSS, enabling static documentation of all states side-by-side. Useful for visual-regression tooling but not required for F2 verification. **F15 polish.**

---

## Risks

- **Frontend-design output diverges significantly from existing palette.** Acceptable per "challenge every aspect" brief. Mitigation: invoke frontend-design *before* component CSS is written, so component styles reference the final token names from day one.
- **Native `<select>` styling caps.** Some browsers don't allow full theming. Mitigation: accept platform variation in F2; revisit only when a screen suffers visibly.
- **CSS-only Tooltip clipping at viewport edges.** Mitigation: F2 usage is short text, non-edge-adjacent; introduce floating-ui-style positioning if real screens demand it.
- **Dioxus 0.7 asset/stylesheet ergonomics.** F1 didn't exercise `document::Stylesheet`, `manganis::asset!()`, or font assets. Mitigation: prove the asset pipeline with `colors.css` + Inter font load before scaling out the rest.
- **`dx` toolchain prerequisite.** Verification check #5 (`dx serve`) requires the `dioxus-cli` (`cargo install dioxus-cli`). The F1 `bridge_demo.rs` already assumes this, but contributor docs (README) should call it out explicitly so a fresh clone doesn't surprise newcomers.
- **WebView2 / wry runtime on Windows.** The asset pipeline relies on the system WebView. WebView2 ships with Windows 11 (Inputforge's primary target); older systems would need the evergreen runtime installed. Document the prerequisite in README.
- **Font license tracking.** Inter and JetBrains Mono are OFL-1.1, not just MIT-style. Mitigation: `THIRD_PARTY_LICENSES.md` ships in F2 (see "Critical Files To Modify") and is verified in check #11.

---

## Next Steps

1. Commit this spec to git.
2. Invoke `superpowers:writing-plans` to produce a step-by-step implementation plan with TDD-friendly checkpoints (frontend-design invocation as an explicit early step).
