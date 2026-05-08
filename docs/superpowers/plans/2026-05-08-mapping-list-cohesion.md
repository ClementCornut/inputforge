# Mapping List Cohesion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Raise the F8 mapping-list rail's visual floor to match the right panels by introducing a `Chip` primitive, unifying the row/chip/create-row active treatment around two new tint tokens, migrating the mode tab strip to the canonical `Tabs` primitive, moving the vJoy output identifier inline on the source line, and codifying the result with row-tokens, active-treatment, and status-bar surface contract tests.

**Architecture:** The pass is a cohesion delta, not a rewrite. State machines (`add_inline.rs`, `keyboard.rs`, `rename_inline.rs`) and behaviour (filter, capture, drag-drop) stay untouched. The work is concentrated in:

1. New `Chip` primitive (`components/chip.rs` + `assets/components/chip.css`) with three variants (Outline, Output, Capture). The hand-rolled `.if-rail__device-chip`, `.if-row__output-badge`, `.if-row__chip`, and `.if-add-inline__chip` selectors fold into Chip variants.
2. Two new color tokens (`--tint-selected` = 8%, `--tint-create` = 5%) added to `assets/tokens/colors.css`, encoding the "selected" vs "create" intensities once for the rail.
3. A `running: bool` field on `TabItem` (`components/tabs.rs`), letting the mode tab strip render its 6px live-mode pip without duplicating Tabs.
4. The `mode_tabs/` cluster collapses from a hand-rolled `<nav role="tablist">` into a Tabs consumer, with the trailing "+" rendered as a sibling outside the `role="tablist"` container.
5. CSS rewrites in `assets/frame/mapping_list.css`: row chrome aligned with `.if-device-row`, dashed-row footer aligned with profiles, drag-handle gutter dropped.
6. Status bar gets two typography deltas (warning glyph inside the existing Badge, mono numerator on the device-count slot) and a surface-contract test in a new `frame/status_bar/tests.rs`.

**Tech Stack:** Rust 2024, Dioxus 0.7, SSR-only component tests via `dioxus-ssr`, CSS assets under `crates/inputforge-gui-dx/assets/`.

---

## File Structure

New files:

- `crates/inputforge-gui-dx/src/components/chip.rs` (Chip primitive, `ChipVariant` enum, smoke tests)
- `crates/inputforge-gui-dx/assets/components/chip.css` (Chip variant CSS)
- `crates/inputforge-gui-dx/src/frame/status_bar/tests.rs` (surface-contract tests)

Modified files:

- `crates/inputforge-gui-dx/assets/tokens/colors.css`: add `--tint-selected`, `--tint-create`
- `crates/inputforge-gui-dx/src/components/mod.rs`: re-export `Chip`, `ChipVariant`
- `crates/inputforge-gui-dx/src/components/tabs.rs`: add `running: bool` to `TabItem`, render the live-mode pip
- `crates/inputforge-gui-dx/assets/components/tabs.css`: optional pip rule
- `crates/inputforge-gui-dx/src/theme/mod.rs`: register `chip.css` asset
- `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/mod.rs`: migrate to Tabs primitive, sibling "+" outside tablist
- `crates/inputforge-gui-dx/assets/frame/top_bar.css`: drop `.if-mode-tab*` rules superseded by `.if-tabs`/`.if-tab` (keep only the layout adjustments specific to the bar)
- `crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs`: post-filter group counts, Chip-based device filter, layout deltas
- `crates/inputforge-gui-dx/src/frame/mapping_list/row.rs`: arrow + inline output Chip, qualifier Chip migration, drop `if-row__output-badge`
- `crates/inputforge-gui-dx/src/frame/mapping_list/add_inline.rs`: Chip Capture in pad, Badge Warning in collision text, pad shell border drop
- `crates/inputforge-gui-dx/src/frame/mapping_list/empty.rs`: confirm `ButtonSize::Sm` on ghost buttons
- `crates/inputforge-gui-dx/assets/frame/mapping_list.css`: row contract, device-filter wrap, group header padding, dashed-row deltas, dropped chip selectors
- `crates/inputforge-gui-dx/src/frame/status_bar/mod.rs`: warning glyph, mono numerator
- `crates/inputforge-gui-dx/assets/frame/status_bar.css`: numerator class
- `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`: row tokens contract, active treatment unification, output chip migration, mode tab canonical class
- `crates/inputforge-gui-dx/examples/component_gallery.rs`: Chip section parallel to Badge

Each task below carries its own test code; the smoke command at every step is `cargo test -p inputforge-gui-dx`. The Dioxus GUI is verified manually via `dx run -p inputforge-app` only at the very end of the plan, not during step-level smoke checks.

---

## Task 1: Add `--tint-selected` and `--tint-create` tokens

**Files:**
- Modify: `crates/inputforge-gui-dx/assets/tokens/colors.css`

- [ ] **Step 1: Write the failing token-presence test**

Add to `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs` at the end of the file:

```rust
#[test]
fn colors_css_declares_tint_selected_and_tint_create() {
    let css = include_str!("../../../assets/tokens/colors.css");
    assert!(
        css.contains("--tint-selected: 8%;"),
        "--tint-selected token must be declared so the rail row, device chip, and \
         create-row hover can color-mix from one source: {css}",
    );
    assert!(
        css.contains("--tint-create: 5%;"),
        "--tint-create token must be declared so the dashed footer hover reads \
         as create rather than selected: {css}",
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-gui-dx colors_css_declares_tint_selected_and_tint_create -- --exact`
Expected: FAIL on either `--tint-selected: 8%;` or `--tint-create: 5%;` substring assertion.

- [ ] **Step 3: Add the tokens**

Append to the `:root { ... }` block in `crates/inputforge-gui-dx/assets/tokens/colors.css`, immediately above the closing brace:

```css
    /* Active-treatment tints (rail cohesion pass).
       --tint-selected encodes the "selected/active" intensity used by the
       row, the device-filter chip, and the canonical .if-device-row.
       --tint-create encodes the slightly cooler "create" intensity used by
       the dashed `+ Add mapping` row and (visually identical) the mode-tab
       trailing `+`. Keeping the percentages here means any future
       intensity tweak ships from one file. */
    --tint-selected: 8%;
    --tint-create:   5%;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p inputforge-gui-dx colors_css_declares_tint_selected_and_tint_create -- --exact`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/assets/tokens/colors.css crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs
git commit -m "feat(tokens): add --tint-selected and --tint-create for rail cohesion"
```

---

## Task 2: Create `Chip` primitive (Outline variant)

**Files:**
- Create: `crates/inputforge-gui-dx/src/components/chip.rs`
- Create: `crates/inputforge-gui-dx/assets/components/chip.css`
- Modify: `crates/inputforge-gui-dx/src/components/mod.rs`
- Modify: `crates/inputforge-gui-dx/src/theme/mod.rs`

- [ ] **Step 1: Write the failing primitive smoke test**

Create `crates/inputforge-gui-dx/src/components/chip.rs` with the test stub but NOT the implementation:

```rust
//! Chip primitive. Three variants (Outline, Output, Capture) cover the
//! rail's chip-like surfaces (device filter idle, qualifier, vJoy output,
//! capture chip). Status semantics live on Badge; classification chips
//! with mono fonts and per-kind hues live here. See DESIGN.md section 7
//! for the Badge vs Chip split.

use dioxus::prelude::*;

use super::merge_class;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChipVariant {
    /// Transparent fill, --color-border-strong border, --color-text-muted
    /// label. Used by device-chip idle and row qualifier chips.
    Outline,
    /// --color-output label, --font-mono, faint output-tinted surface.
    /// Used by the row's vJoy out chip.
    Output,
    /// kind-tinted via data-kind="axis|button|hat", mono. Used by the
    /// add-inline pad's input identifier chip.
    Capture,
}

#[component]
pub fn Chip(
    #[props(default = ChipVariant::Outline)] variant: ChipVariant,
    #[props(default)] class: Option<String>,
    #[props(default)] title: Option<String>,
    children: Element,
) -> Element {
    let v = match variant {
        ChipVariant::Outline => "if-chip--outline",
        ChipVariant::Output  => "if-chip--output",
        ChipVariant::Capture => "if-chip--capture",
    };
    let combined = merge_class("if-chip", v, class.as_deref());
    rsx! {
        span {
            class: "{combined}",
            title: title.as_deref(),
            {children}
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case, reason = "Dioxus components are PascalCase")]

    use dioxus::prelude::*;
    use dioxus_ssr::render;

    use super::{Chip, ChipVariant};

    fn render_with(variant: ChipVariant) -> String {
        fn make(variant: ChipVariant) -> Element {
            rsx! { Chip { variant: variant, "x" } }
        }
        let mut vdom = VirtualDom::new_with_props(make, variant);
        vdom.rebuild_in_place();
        render(&vdom)
    }

    #[test]
    fn chip_outline_variant_emits_outline_class() {
        let html = render_with(ChipVariant::Outline);
        assert!(html.contains("if-chip"), "base class missing: {html}");
        assert!(
            html.contains("if-chip--outline"),
            "outline variant class missing: {html}",
        );
    }

    #[test]
    fn chip_renders_title_when_provided() {
        fn TestComponent() -> Element {
            rsx! {
                Chip {
                    variant: ChipVariant::Outline,
                    title: "tooltip text".to_owned(),
                    "Label"
                }
            }
        }
        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            html.contains("title=\"tooltip text\""),
            "Chip must forward title prop to span: {html}",
        );
    }

    #[test]
    fn chip_omits_title_attribute_when_absent() {
        fn TestComponent() -> Element {
            rsx! { Chip { variant: ChipVariant::Outline, "Label" } }
        }
        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            !html.contains("title="),
            "Chip must not emit a title attribute when prop is None: {html}",
        );
    }
}
```

- [ ] **Step 2: Wire the module so the test compiles**

Edit `crates/inputforge-gui-dx/src/components/mod.rs`. Add the `pub mod chip;` line in the alphabetic block between `card` and `checkbox`:

```rust
pub mod card;
pub mod checkbox;
pub mod chip;
pub mod click_away_listener;
```

And add the re-export between `card` and `checkbox`:

```rust
pub use card::{Card, CardPadding};
pub use checkbox::Checkbox;
pub use chip::{Chip, ChipVariant};
pub use click_away_listener::ClickAwayListener;
```

(Ordering matches the file's alphabetic re-export block; place the new line between `card` and `checkbox`.)

- [ ] **Step 3: Create the empty CSS file so the asset!() macro can resolve**

Create `crates/inputforge-gui-dx/assets/components/chip.css` with:

```css
/* Chip primitive. Three variants cover the rail's classification chips,
   output chips, and capture chips. Status semantics live on Badge per
   DESIGN.md section 7. */

.if-chip {
    display: inline-flex;
    align-items: baseline;
    gap: 4px;
    padding: 1px var(--space-2);
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    font-size: 11px;
    line-height: 1.4;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.if-chip.if-chip--outline {
    background: transparent;
    border-color: var(--color-border-strong);
    color: var(--color-text-muted);
}
```

- [ ] **Step 4: Register the CSS asset**

Edit `crates/inputforge-gui-dx/src/theme/mod.rs`. Add the asset constant beside `BADGE_CSS`:

```rust
const BADGE_CSS: Asset = asset!("/assets/components/badge.css");
const CHIP_CSS: Asset = asset!("/assets/components/chip.css");
```

And in the `rsx!` block, add the stylesheet line directly after `Stylesheet { href: BADGE_CSS }`:

```rust
        Stylesheet { href: BADGE_CSS }
        Stylesheet { href: CHIP_CSS }
        Stylesheet { href: SEPARATOR_CSS }
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p inputforge-gui-dx chip_outline_variant_emits_outline_class -- --exact`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/src/components/chip.rs \
        crates/inputforge-gui-dx/assets/components/chip.css \
        crates/inputforge-gui-dx/src/components/mod.rs \
        crates/inputforge-gui-dx/src/theme/mod.rs
git commit -m "feat(components): add Chip primitive with Outline variant"
```

---

## Task 3: Add Chip `Output` variant

**Files:**
- Modify: `crates/inputforge-gui-dx/src/components/chip.rs`
- Modify: `crates/inputforge-gui-dx/assets/components/chip.css`

- [ ] **Step 1: Write the failing test**

Append to the `mod tests` block in `crates/inputforge-gui-dx/src/components/chip.rs`:

```rust
    #[test]
    fn chip_output_variant_emits_output_class_and_css_rule() {
        let html = render_with(ChipVariant::Output);
        assert!(
            html.contains("if-chip--output"),
            "output variant class missing: {html}",
        );
        let css = include_str!("../../assets/components/chip.css");
        assert!(
            css.contains(".if-chip--output"),
            "Chip Output CSS rule must land in chip.css: {css}",
        );
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-gui-dx chip_output_variant_emits_output_class_and_css_rule -- --exact`
Expected: FAIL on the CSS-rule assertion. The variant arm landed in Task 2 but `chip.css` has no `.if-chip--output` rule yet; Step 3 below adds it.

- [ ] **Step 3: Add the CSS rule**

Append to `crates/inputforge-gui-dx/assets/components/chip.css`:

```css
/* No max-width: Output chips have no intrinsic length cap. The rail row
   consumer (`.if-row__output-chip`) sets `max-width: none` explicitly,
   and any future consumer that wants truncation pins it locally. */
.if-chip.if-chip--output {
    background: var(--color-bg-sunken);
    border-color: var(--color-border);
    color: var(--color-output);
    font-family: var(--font-mono);
}
```

- [ ] **Step 4: Re-run the smoke**

Run: `cargo test -p inputforge-gui-dx chip_ -- --test-threads=1`
Expected: All Chip tests pass (Outline + Output).

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/components/chip.rs crates/inputforge-gui-dx/assets/components/chip.css
git commit -m "feat(components): add Chip Output variant"
```

---

## Task 4: Add Chip `Capture` variant with `data-kind` hues

**Files:**
- Modify: `crates/inputforge-gui-dx/src/components/chip.rs`
- Modify: `crates/inputforge-gui-dx/assets/components/chip.css`

- [ ] **Step 1: Write the failing test**

Append to the `mod tests` block in `crates/inputforge-gui-dx/src/components/chip.rs`:

```rust
    #[test]
    fn chip_capture_variant_emits_capture_class() {
        let html = render_with(ChipVariant::Capture);
        assert!(
            html.contains("if-chip--capture"),
            "capture variant class missing: {html}",
        );
    }

    #[test]
    fn chip_css_keys_capture_hues_off_parent_data_kind() {
        let css = include_str!("../../../assets/components/chip.css");
        assert!(
            css.contains("[data-kind=\"axis\"]   > .if-chip--capture")
                || css.contains("[data-kind=\"axis\"] > .if-chip--capture"),
            "axis hue rule (parent-attribute selector) missing: {css}",
        );
        assert!(
            css.contains("[data-kind=\"button\"] > .if-chip--capture"),
            "button hue rule (parent-attribute selector) missing: {css}",
        );
        assert!(
            css.contains("[data-kind=\"hat\"]    > .if-chip--capture")
                || css.contains("[data-kind=\"hat\"] > .if-chip--capture"),
            "hat hue rule (parent-attribute selector) missing: {css}",
        );
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-gui-dx chip_ -- --test-threads=1`
Expected: `chip_css_keys_capture_hues_off_data_kind` FAILS on the missing `[data-kind="axis"]` selector.

- [ ] **Step 3: Add the CSS rules**

Append to `crates/inputforge-gui-dx/assets/components/chip.css`:

```css
.if-chip.if-chip--capture {
    background: color-mix(in srgb, currentColor 14%, transparent);
    border-color: currentColor;
    font-family: var(--font-mono);
    font-weight: 500;
    line-height: 1.15;
    min-width: 32px;
    min-height: 20px;
    padding: 2px var(--space-1);
    justify-content: center;
}

/* Per-input-kind hue. Chip itself does not own a `data-kind` prop
   (Capture is the only consumer that needs the taxonomy hook); the
   call site wraps Chip in a parent span carrying `data-kind`, so the
   selector keys off the parent attribute. Same taxonomy as the legacy
   .if-add-inline__chip[data-kind=...] block in mapping_list.css; that
   block is dropped in Task 15 once the Capture chip ships. */
[data-kind="axis"]   > .if-chip--capture { color: var(--color-output); }
[data-kind="button"] > .if-chip--capture { color: var(--color-control-badge-text); }
[data-kind="hat"]    > .if-chip--capture { color: var(--color-processing); }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx chip_ -- --test-threads=1`
Expected: PASS for all four Chip tests.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/components/chip.rs crates/inputforge-gui-dx/assets/components/chip.css
git commit -m "feat(components): add Chip Capture variant with data-kind hues"
```

---

## Task 5: Add `running: bool` field to `TabItem`

**Files:**
- Modify: `crates/inputforge-gui-dx/src/components/tabs.rs`
- Modify: `crates/inputforge-gui-dx/assets/components/tabs.css`

- [ ] **Step 1: Write the failing test**

Add to `crates/inputforge-gui-dx/src/components/tabs.rs` at the end of the file:

```rust
#[cfg(test)]
mod tests {
    #![allow(non_snake_case, reason = "Dioxus components are PascalCase")]

    use dioxus::prelude::*;
    use dioxus_ssr::render;

    use super::{TabItem, Tabs};

    #[test]
    fn tabs_renders_running_pip_when_tab_running_is_true() {
        fn TestComponent() -> Element {
            let items = vec![
                TabItem {
                    id: "default".to_owned(),
                    label: "Default".to_owned(),
                    controls: None,
                    running: true,
                },
                TabItem {
                    id: "combat".to_owned(),
                    label: "Combat".to_owned(),
                    controls: None,
                    running: false,
                },
            ];
            rsx! {
                Tabs {
                    value: "combat".to_owned(),
                    onchange: move |_: String| {},
                    items,
                }
            }
        }
        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            html.contains("if-tab__running-pip"),
            "running pip element missing on the running tab: {html}",
        );
    }

    #[test]
    fn tabs_does_not_render_running_pip_for_non_running_tabs() {
        fn TestComponent() -> Element {
            let items = vec![TabItem {
                id: "default".to_owned(),
                label: "Default".to_owned(),
                controls: None,
                running: false,
            }];
            rsx! {
                Tabs {
                    value: "default".to_owned(),
                    onchange: move |_: String| {},
                    items,
                }
            }
        }
        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            !html.contains("if-tab__running-pip"),
            "running pip must NOT appear when no tab is running: {html}",
        );
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx tabs_renders_running_pip -- --exact`
Expected: FAIL at compile time because `TabItem` has no `running` field.

- [ ] **Step 3: Add the `running` field and pip rendering**

Edit `crates/inputforge-gui-dx/src/components/tabs.rs`. Update the `TabItem` struct (around line 14):

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabItem {
    pub id: String,
    pub label: String,
    pub controls: Option<String>,
    /// `true` marks this tab as the runtime-live one (orthogonal to
    /// `value`/`is_active`). The Tabs primitive renders a 6px
    /// `--color-live` pip before the label when set. Default `false`
    /// for consumers that do not need the indicator.
    pub running: bool,
}
```

In the `rsx! { button { ... "{label}" } }` block (around line 138), replace the bare `"{label}"` with a pip-aware fragment:

```rust
                            if item.running {
                                span {
                                    class: "if-tab__running-pip",
                                    "aria-hidden": "true",
                                }
                            }
                            "{label}"
```

(The `item` binding lives outside the destructure-let; capture the boolean before the destructure to avoid a move conflict. In `crates/inputforge-gui-dx/src/components/tabs.rs`, change the `let TabItem { id, label, controls } = item;` line (currently at line 68) to `let TabItem { id, label, controls, running } = item;` and use `if running { ... }` instead of `if item.running { ... }`.)

- [ ] **Step 4: Add the CSS rule**

Append to `crates/inputforge-gui-dx/assets/components/tabs.css`:

```css
/* Runtime-live pip. Independent of the active-tab indicator (a tab can
   be running while another tab is currently being edited). Carries the
   --color-live LED hue idiom established by the engine pill, with a
   flat halo at low alpha for the panel-mounted indicator reading. */
.if-tab__running-pip {
    width: 6px;
    height: 6px;
    border-radius: var(--radius-full);
    background: var(--color-live);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-live) 24%, transparent);
    margin-right: var(--space-1);
    flex: 0 0 auto;
    align-self: center;
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx tabs_ -- --test-threads=1`
Expected: PASS for both new tests; existing tabs tests, if any, still pass.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/src/components/tabs.rs crates/inputforge-gui-dx/assets/components/tabs.css
git commit -m "feat(tabs): add running pip via TabItem.running"
```

---

## Task 6: Reskin `mode_tabs` to canonical Tabs class shape; lift "+" outside tablist

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/mod.rs`
- Modify: `crates/inputforge-gui-dx/assets/frame/top_bar.css`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`

**Approach note.** The `Tabs` primitive owns its own keyboard handling (ArrowLeft/Right, Home/End) and does NOT expose per-item event hooks. The mode-tab cluster needs per-tab `oncontextmenu` (right-click + Shift+F10), `onkeydown` for Delete-opens-F4, `onmounted` for focus restoration after rename/delete close, and inline-rename swap-in. Migrating those to wrapper-level event resolution would re-architect the cluster's keyboard plumbing without changing what the user sees. This task delivers the spec's visible contract (canonical `.if-tab--active` underline, `+` outside `role="tablist"`, live-mode pip beside the running tab's label) by reskinning the existing hand-rolled cluster: rename classes to match the canonical primitive, and lift the trailing `+` button to a sibling of the tablist `<div>`. The `running: bool` field added to `TabItem` in Task 5 stays in place for future Tabs consumers; mode_tabs renders the pip with the same `.if-tab__running-pip` class so the visual treatment is shared via CSS.

- [ ] **Step 1: Write the failing canonical-class and "+" position tests**

Add to `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs` at the end of the file:

```rust
#[test]
fn mode_tabs_active_tab_renders_canonical_if_tab_active_class() {
    use crate::frame::top_bar::mode_tabs::ModeTabs;
    use inputforge_core::action::Mapping;
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    use std::collections::HashMap;

    fn TestComponent() -> Element {
        let map = HashMap::from([
            ("Default".to_owned(), vec![]),
            ("Combat".to_owned(), vec![]),
        ]);
        let modes = ModeTree::from_adjacency(&map).unwrap();
        let mappings = vec![Mapping {
            input: InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            actions: vec![],
        }];
        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);

        // `provide_minimal_contexts` supplies AppContext, ViewState,
        // ToastQueue, and live-capture. ModeTabs additionally needs
        // ModeDeleteSignal, which we provide inline below.
        provide_minimal_contexts();
        let ctx_app = use_context::<AppContext>();
        let mut cfg_signal = ctx_app.config;
        let mut meta_signal = ctx_app.meta;
        // ModeDeleteSignal is provided shell-side normally; provide a
        // local stub so ModeTabs can mount in isolation.
        let dt: Signal<Option<String>> = use_signal(|| None);
        use_context_provider(|| crate::frame::top_bar::mode_tabs::ModeDeleteSignal(dt));
        use_hook(move || {
            cfg_signal.set(ConfigSnapshot::from_state(&state, None));
            meta_signal.set(MetaSnapshot::from_state(&state));
        });

        rsx! { ModeTabs {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-tab--active"),
        "mode tabs must use the canonical .if-tab--active underline class: {html}",
    );
    assert!(
        !html.contains("if-mode-tab--active"),
        "legacy hand-rolled .if-mode-tab--active class must be retired: {html}",
    );
}

#[test]
fn mode_tabs_add_button_lives_outside_tablist() {
    use crate::frame::top_bar::mode_tabs::ModeTabs;
    use inputforge_core::action::Mapping;
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    use std::collections::HashMap;

    fn TestComponent() -> Element {
        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();
        let mappings = vec![Mapping {
            input: InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            actions: vec![],
        }];
        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);

        provide_minimal_contexts();
        let ctx_app = use_context::<AppContext>();
        let mut cfg_signal = ctx_app.config;
        let mut meta_signal = ctx_app.meta;
        let dt: Signal<Option<String>> = use_signal(|| None);
        use_context_provider(|| crate::frame::top_bar::mode_tabs::ModeDeleteSignal(dt));
        use_hook(move || {
            cfg_signal.set(ConfigSnapshot::from_state(&state, None));
            meta_signal.set(MetaSnapshot::from_state(&state));
        });

        rsx! { ModeTabs {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);
    // Locate the role="tablist" opening, find the matching `</div>`
    // (the tablist contains only buttons, no nested divs after the
    // reskin), and assert the Add-mode button appears AFTER that close.
    let tablist_open = html.find("role=\"tablist\"").expect("tablist must render");
    let tablist_close_relative = html[tablist_open..]
        .find("</div>")
        .expect("tablist closes");
    let tablist_close = tablist_open + tablist_close_relative;
    let plus_idx = html
        .find("aria-label=\"Add mode\"")
        .expect("Add mode button must render");
    assert!(
        plus_idx > tablist_close,
        "Add-mode `+` must render OUTSIDE the role=tablist container so AT \
         tab counts stay honest. tablist_close={tablist_close}, plus_idx={plus_idx}",
    );
}

#[test]
fn mode_tabs_running_pip_uses_canonical_class() {
    use crate::frame::top_bar::mode_tabs::ModeTabs;
    use inputforge_core::action::Mapping;
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    use std::collections::HashMap;

    fn TestComponent() -> Element {
        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();
        let mappings = vec![Mapping {
            input: InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            actions: vec![],
        }];
        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let mut state = AppState::with_profile(profile);
        // Force the runtime mode to match the only tab so the marker
        // resolves to tab_index = Some(0).
        state.runtime_mode = Some("Default".to_owned());

        provide_minimal_contexts();
        let ctx_app = use_context::<AppContext>();
        let mut cfg_signal = ctx_app.config;
        let mut meta_signal = ctx_app.meta;
        let dt: Signal<Option<String>> = use_signal(|| None);
        use_context_provider(|| crate::frame::top_bar::mode_tabs::ModeDeleteSignal(dt));
        use_hook(move || {
            cfg_signal.set(ConfigSnapshot::from_state(&state, None));
            meta_signal.set(MetaSnapshot::from_state(&state));
        });

        rsx! { ModeTabs {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-tab__running-pip"),
        "running tab must render the canonical .if-tab__running-pip class so \
         the live-mode marker is shared with future Tabs primitive consumers: {html}",
    );
    assert!(
        !html.contains("if-mode-tab__marker"),
        "legacy bespoke .if-mode-tab__marker class must be retired: {html}",
    );
}
```

(If `AppState` does not have a public `runtime_mode` field, fall back to setting the `MetaSnapshot.current_mode` directly so `runtime_marker` resolves the live tab index. The test still asserts the same contract: `if-tab__running-pip` present, `if-mode-tab__marker` absent.)

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx mode_tabs_ -- --test-threads=1`
Expected: FAIL on `if-tab--active`, `if-tab__running-pip`, and the "+" position assertion.

- [ ] **Step 3: Reskin per-tab classes in `mode_tabs/mod.rs`**

Open `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/mod.rs`. Three concrete edits inside the existing function body (no behaviour or signal changes):

(3a) Tablist wrapper class. Replace `class: "if-mode-tabs"` (around line 112) with `class: "if-tabs if-mode-tabs-wrap"`. The canonical `.if-tabs` carries the bottom-hairline + tab gap rhythm; the bar-specific `.if-mode-tabs-wrap` keeps the 40px height and scroll-x layout (added in Step 4 below).

(3b) Per-tab class. Replace the `class: if is_active { "if-mode-tab if-mode-tab--active" } else { "if-mode-tab" }` line (around line 299) with:

```rust
                                class: if is_active { "if-tab if-tab--active" } else { "if-tab" },
```

(3c) Running pip class. Replace the `class: "if-mode-tab__marker"` line (around line 318) with:

```rust
                                        class: "if-tab__running-pip",
```

(3c.1) Move the running-pip span ABOVE the `"{name}"` text node. Today the per-tab button rsx (around lines 314-328) renders `"{name}"` first, then `if show_marker { span { ... } }`. Cut the entire `if show_marker { ... }` block (now using `if-tab__running-pip` class per 3c) and paste it immediately above `"{name}"`. The pip must render before the label so the canonical Tabs `margin-right: var(--space-1)` on `.if-tab__running-pip` (added in Task 5) opens a gap between pip and label, not trailing whitespace after the label.

(3d) Lift the trailing `+` outside the tablist. Today the `if *adding.read() { ... } else { button { class: "if-mode-tab if-mode-tab--add", ... "+" } }` block (around lines 425-435) sits as the last child of the `<div role="tablist">`. Wrap the existing tablist in an outer flex container, lift the `+` block out, and update the add button's class:

```rust
    rsx! {
        div { class: "if-mode-tabs-outer",
            div { class: "if-tabs if-mode-tabs-wrap", role: "tablist",
                "aria-orientation": "horizontal", "aria-label": "Editing mode",
                // for (idx, name) in modes_now.iter().cloned().enumerate() { ... existing per-tab body ... }
                // (existing context_menu mount block remains here, INSIDE
                //  the wrapper but conceptually a sibling of the tablist
                //  buttons; AT readers ignore role-less divs so the
                //  context menu's anchor placement is unaffected.)
            }
            // T31: tail `+` add tab, swaps to inline editor when open.
            // Lifted OUTSIDE role="tablist" so screen-reader tab counts
            // stay honest.
            if *adding.read() {
                add_inline::AddInline { open: adding, pending_focus }
            } else {
                button {
                    r#type: "button",
                    class: "if-mode-tab--add",
                    onclick: move |_| adding.set(true),
                    "aria-label": "Add mode",
                    "+"
                }
            }
        }
    }
```

The previous render placed the context-menu mount and the add button INSIDE the tablist `<div>`. After this edit, the add button moves to a sibling. The context-menu mount conditionally renders a `<div class="if-row-menu" role="menu">` overlay; that overlay can stay inside the tablist (it does not affect AT tab counts because `role="menu"` is its own AT region) OR it can move to the outer `if-mode-tabs-outer` flex container. Pick the outer placement for symmetry with the add button.

- [ ] **Step 4: Update `top_bar.css` to match the canonical class shape**

Open `crates/inputforge-gui-dx/assets/frame/top_bar.css`. Replace the legacy `.if-mode-tabs`, `.if-mode-tab`, `.if-mode-tab--active`, `.if-mode-tab--active::after`, `.if-mode-tab:focus-visible`, `.if-mode-tab__marker`, and `.if-mode-tab--add` rules (lines 373-527 today) with:

```css
/* Mode tabs strip. The canonical .if-tab / .if-tab--active visuals come
   from assets/components/tabs.css (3px primary bottom-underline on
   active, no fill); the wrapper class below adds bar-specific layout
   (40px height inheritance, scroll-x at narrow viewports, ellipsis
   truncation per tab, Webkit scrollbar styling). The .if-tabs base
   class brings the bottom hairline that doubles as the bar's seam
   anchor, so margin-bottom: -1px on .if-tab is not required here. */
.if-mode-tabs-outer {
    display: flex;
    align-items: stretch;
    flex: 1 1 0;
    min-width: 0;
    height: 100%;
}

/* `.if-tabs` already carries display:flex + flex-direction:row + gap.
   This rule only adds the bar-specific layout: 40 px height (via 100%
   inside the outer), gap 0 (override the canonical --space-1 gap so
   tabs sit shoulder to shoulder under the underline), scroll-x, and
   the bar's left margin. */
.if-mode-tabs-wrap {
    align-items: stretch;
    gap: 0;
    margin-left: var(--space-2);
    height: 100%;
    flex: 1 1 0;
    min-width: 0;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: thin;
    scrollbar-color: var(--color-border-strong) transparent;
    border-bottom: 0;
    padding: 0;
}

.if-mode-tabs-wrap::-webkit-scrollbar          { height: 6px; }
.if-mode-tabs-wrap::-webkit-scrollbar-track    { background: transparent; }
.if-mode-tabs-wrap::-webkit-scrollbar-thumb    { background: var(--color-border-strong); border-radius: var(--radius-full); }

/* Density compaction inside the bar. Canonical .if-tab uses
   --space-2 / --space-3 padding; the bar tightens to keep label rhythm
   with the engine pill and tools cluster at 40 px height, and pins
   max-width per tab so long mode names truncate with ellipsis. */
.if-mode-tabs-wrap > .if-tab {
    padding: 0 0.875rem;
    max-width: 14rem;
    flex-shrink: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: flex;
    align-items: center;
    gap: 0.375rem;
}

/* Tail "+" add affordance, sibling of the tablist. Mirrors the resting
   .if-tab visual (muted text raising on hover) so the bar reads as one
   strip; padding tightened so the `+` sits flush after the last tab. */
.if-mode-tab--add {
    background: transparent;
    border: 0;
    color: var(--color-text-muted);
    padding: 0 var(--space-2);
    font: inherit;
    font-weight: var(--weight-medium);
    cursor: pointer;
}

.if-mode-tab--add:hover { color: var(--color-text); }

.if-mode-tab--add:focus-visible {
    outline: 2px solid var(--color-border-focus);
    outline-offset: -3px;
    border-radius: var(--radius-sm);
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx mode_tabs_ -- --test-threads=1`
Expected: PASS for all three. The full suite stays green: `cargo test -p inputforge-gui-dx -- --test-threads=1`.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/mod.rs \
        crates/inputforge-gui-dx/assets/frame/top_bar.css \
        crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs
git commit -m "refactor(mode-tabs): reskin to canonical Tabs class shape; lift + outside tablist"
```

---

## Task 7: Apply row token contract (radius, padding, hover, selected, focus-visible)

**Files:**
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_list.css`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`

- [ ] **Step 1: Write the failing row-tokens contract test**

Add to `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs` at the end of the file:

```rust
#[test]
fn mapping_list_css_locks_row_token_contract() {
    let css = include_str!("../../../assets/frame/mapping_list.css");

    // Row resting block. Padding is uniform var(--space-3) (the 10px
    // drag-handle gutter is dropped in this pass; the SortableHandle is
    // a 0-width hover-only overlay). Radius bumps to --radius-md to
    // match .if-device-row and .profile-row.
    let block = css
        .split(".if-row {")
        .nth(1)
        .expect(".if-row rule present")
        .split('}')
        .next()
        .expect(".if-row rule closed");
    assert!(
        block.contains("padding: var(--space-3);"),
        ".if-row padding must be uniform var(--space-3): {block}",
    );
    assert!(
        block.contains("border-radius: var(--radius-md);"),
        ".if-row must use --radius-md (matches .if-device-row): {block}",
    );
    assert!(
        block.contains("background: var(--color-bg);"),
        ".if-row base must use --color-bg (rows sit on bg, not bg-elevated): {block}",
    );
    assert!(
        block.contains("border: 1px solid transparent;"),
        ".if-row must reserve a 1px transparent border so hover/selected swaps \
         do not reflow geometry: {block}",
    );

    // Hover.
    let hover = css
        .split(".if-row:hover {")
        .nth(1)
        .expect(".if-row:hover rule present")
        .split('}')
        .next()
        .expect(".if-row:hover rule closed");
    assert!(
        hover.contains("background: var(--color-bg-elevated);"),
        ".if-row:hover background must be --color-bg-elevated (the \
         --color-border substitution is dropped): {hover}",
    );
    assert!(
        hover.contains("border-color: var(--color-border-strong);"),
        ".if-row:hover border must be --color-border-strong (matches \
         .if-device-row:hover): {hover}",
    );

    // Selected. The active row mixes --color-primary at --tint-selected
    // into --color-bg, with a --color-border-focus border.
    let selected = css
        .split(".if-row.is-active {")
        .nth(1)
        .expect(".if-row.is-active rule present")
        .split('}')
        .next()
        .expect(".if-row.is-active rule closed");
    assert!(
        selected.contains(
            "background: color-mix(in srgb, var(--color-primary) var(--tint-selected), var(--color-bg));"
        ),
        ".if-row.is-active background must mix --color-primary at \
         --tint-selected into --color-bg (no `transparent` parent): {selected}",
    );
    assert!(
        selected.contains("border-color: var(--color-border-focus);"),
        ".if-row.is-active must carry the focus border idiom: {selected}",
    );

    // Focus-visible. Inset 2px to match .if-device-row's offset:-2px.
    assert!(
        css.contains(".if-row:focus-visible {"),
        ".if-row:focus-visible rule must exist: {css}",
    );
    let focus = css
        .split(".if-row:focus-visible {")
        .nth(1)
        .expect(".if-row:focus-visible rule present")
        .split('}')
        .next()
        .expect(".if-row:focus-visible rule closed");
    assert!(
        focus.contains("outline: 2px solid var(--color-border-focus);"),
        "row focus ring must be 2px var(--color-border-focus): {focus}",
    );
    assert!(
        focus.contains("outline-offset: -2px;"),
        "row focus offset must be inset (-2px) per .if-device-row contract: {focus}",
    );
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p inputforge-gui-dx mapping_list_css_locks_row_token_contract -- --exact`
Expected: FAIL on multiple assertions (today's row uses `var(--space-2) ... + 10px` padding, `--radius-sm`, `--color-border` hover, primary-on-transparent selected, etc.).

- [ ] **Step 3: Rewrite the row block in `mapping_list.css`**

Open `crates/inputforge-gui-dx/assets/frame/mapping_list.css`. Replace the existing `.if-row { ... }`, `.if-row:hover { ... }`, and `.if-row.is-active { ... }` rules (lines 102-146 in the current file) with:

```css
/* Row chrome. Token-aligned with .if-device-row (panel_slot.css) so the
 * rail and the right panels read as one design system. Selected state
 * uses surface-tint over the row's --color-bg base, NOT
 * primary-on-transparent (which composited against the rail's elevated
 * surface previously). The 1px transparent border reserves geometry so
 * hover/selected swaps do not reflow.
 *
 * Selected signal: surface-tint + focus-border. We deliberately do NOT
 * use a `border-left` accent stripe; DESIGN.md section 8 names that
 * pattern as banned (Toast accent stripe is the only documented
 * exception, and only because toasts sit in the user's peripheral
 * field; rows sit foveal). */
.if-row {
    position: relative;
    padding: var(--space-3);
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    background: var(--color-bg);
    cursor: pointer;
    user-select: none;
    transition:
        background var(--duration-fast) var(--easing-fast),
        border-color var(--duration-fast) var(--easing-fast),
        color var(--duration-fast) var(--easing-fast);
}

.if-row:hover {
    background: var(--color-bg-elevated);
    border-color: var(--color-border-strong);
}

.if-row.is-active {
    background: color-mix(in srgb, var(--color-primary) var(--tint-selected), var(--color-bg));
    border-color: var(--color-border-focus);
}

.if-row.is-active .if-row__name {
    font-weight: 700;
}

.if-row:focus-visible {
    outline: 2px solid var(--color-border-focus);
    outline-offset: -2px;
}
```

Drop the `.if-row:focus-visible` line out of the combined selector at the bottom of the file (the one that bundles `.if-row:focus-visible, .if-rail__filter input:focus-visible, .if-row-rename:focus-visible, .if-add-inline--captured input:focus-visible` together) so the row owns its own focus-visible block above. Keep the other three selectors in a separate combined rule:

```css
.if-rail__filter input:focus-visible,
.if-row-rename:focus-visible,
.if-add-inline--captured input:focus-visible {
    outline: 2px solid var(--color-border-focus);
    outline-offset: 1px;
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p inputforge-gui-dx mapping_list_css_locks_row_token_contract -- --exact`
Expected: PASS.

Also run the full mapping_list test bundle for regressions: `cargo test -p inputforge-gui-dx -- --test-threads=1`
Expected: existing snapshot tests adjust automatically (they assert presence of `is-active`, not exact CSS); all PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/assets/frame/mapping_list.css crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs
git commit -m "feat(mapping-list): align row chrome with .if-device-row contract"
```

---

## Task 8: Drop drag-handle gutter; row gap = 2px

**Files:**
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_list.css`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`

- [ ] **Step 1: Write the failing row-gap test**

Append to `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`:

```rust
#[test]
fn mapping_list_css_uses_row_gap_2px_inside_groups() {
    let css = include_str!("../../../assets/frame/mapping_list.css");
    let block = css
        .split(".if-rail__group {")
        .nth(1)
        .expect(".if-rail__group rule present")
        .split('}')
        .next()
        .expect(".if-rail__group rule closed");
    assert!(
        block.contains("display: flex;") && block.contains("flex-direction: column;"),
        ".if-rail__group must be a column flex container so `gap` applies \
         between rows: {block}",
    );
    assert!(
        block.contains("gap: 2px;"),
        ".if-rail__group must use a 2px gap between rows; got: {block}",
    );
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p inputforge-gui-dx mapping_list_css_uses_row_gap_2px_inside_groups -- --exact`
Expected: FAIL.

- [ ] **Step 3: Update the group container**

Open `crates/inputforge-gui-dx/assets/frame/mapping_list.css`. Replace the existing `.if-rail__group { margin-bottom: var(--space-2); }` rule with:

```css
.if-rail__group {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-bottom: var(--space-2);
}
```

The drag-handle gutter has already been dropped in Task 7 (uniform `var(--space-3)` padding). The `SortableHandle` already absolute-positions itself at `left: 4px` (`assets/components/sortable.css:28`); with the row's left padding now starting at `var(--space-3)` (`12px`), the handle floats over the source line on hover. That is the intended overlay reading.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p inputforge-gui-dx mapping_list_css_uses_row_gap_2px_inside_groups -- --exact`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/assets/frame/mapping_list.css crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs
git commit -m "feat(mapping-list): drop drag-handle gutter and tighten row gap to 2px"
```

---

## Task 9: Migrate device filter chips to `Chip` Outline + active variant

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs`
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_list.css`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`

Depends on Task 1 (`--tint-selected` token) for the `color-mix(var(--color-primary) var(--tint-selected), ...)` declaration in this task's CSS block.

- [ ] **Step 1: Write failing tests**

Append to `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`:

```rust
#[test]
fn device_filter_active_chip_emits_unified_active_class() {
    use crate::context::{GlyphFlags, MappingSummary};
    use inputforge_core::state::DeviceState;
    use inputforge_core::types::{
        AxisPolarity, DeviceDiagnostics, DeviceId, DeviceInfo, InputAddress, InputId,
    };

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let ctx = use_context::<AppContext>();
        let mut cfg_signal = ctx.config;
        let view = use_context::<crate::frame::view_state::ViewState>();
        // Pre-select the device by simulating a click; we set the
        // `selected_device` signal indirectly by fixing the chip state via
        // the rail's internal API. Easiest path: assert the active class
        // on the chip is rendered when a single matching device is the
        // only filter candidate AND a row in that mode references it.
        let _ = view; // silence unused
        use_hook(move || {
            cfg_signal.set(ConfigSnapshot {
                devices: vec![DeviceState {
                    info: DeviceInfo {
                        id: DeviceId("stick".to_owned()),
                        name: "Twin Stick".to_owned(),
                        axes: 1,
                        buttons: 1,
                        hats: 0,
                        instance_path: None,
                        axis_polarities: vec![AxisPolarity::Bipolar],
                    },
                    connected: true,
                    diagnostics: DeviceDiagnostics::default(),
                }],
                mappings: vec![MappingSummary {
                    input: InputAddress::Bound {
                        device: DeviceId("stick".to_owned()),
                        input: InputId::Button { index: 0 },
                    },
                    mode: "Default".to_owned(),
                    name: Some("Boost".to_owned()),
                    glyphs: GlyphFlags::default(),
                    referenced_devices: vec![DeviceId("stick".to_owned())],
                    first_vjoy_output: None,
                }],
                device_display_names: std::collections::HashMap::from([(
                    DeviceId("stick".to_owned()),
                    "Twin Stick".to_owned(),
                )]),
                ..ConfigSnapshot::default()
            });
        });
        rsx! { MappingList {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-chip"),
        "device filter chip must render the canonical .if-chip class \
         (Chip primitive): {html}",
    );
    assert!(
        html.contains("if-chip--outline"),
        "idle device chip must use the Outline variant: {html}",
    );
    assert!(
        !html.contains("if-rail__device-chip"),
        "legacy hand-rolled .if-rail__device-chip class must be retired: {html}",
    );
}

#[test]
fn mapping_list_css_wraps_device_filter_chips_into_multiple_rows() {
    let css = include_str!("../../../assets/frame/mapping_list.css");
    let block = css
        .split(".if-rail__device-filter {")
        .nth(1)
        .expect(".if-rail__device-filter rule present")
        .split('}')
        .next()
        .expect(".if-rail__device-filter rule closed");
    assert!(
        block.contains("flex-wrap: wrap;"),
        "device filter strip must wrap to a multi-row layout (no scroll-x); got: {block}",
    );
    assert!(
        !block.contains("overflow-x: auto;"),
        "device filter strip must NOT use overflow-x scrolling after the wrap migration; got: {block}",
    );
}
```

(The earlier `mapping_list_css_keeps_device_chips_one_row` test, around `tests.rs:241`, must be REPLACED in this task; it asserts the inverse contract. Delete that test in the same diff.)

- [ ] **Step 2: Run tests to verify the new ones fail and the legacy one is gone**

Run: `cargo test -p inputforge-gui-dx mapping_list_css_wraps_device_filter_chips_into_multiple_rows mapping_list_css_keeps_device_chips_one_row device_filter_active_chip_emits_unified_active_class -- --test-threads=1`
Expected: `mapping_list_css_keeps_device_chips_one_row` is gone (compile-time absence is the assertion); the two new tests FAIL.

- [ ] **Step 3: Migrate `DeviceFilterRow` to `Chip`**

Open `crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs`. Add `Chip, ChipVariant` to the components import line:

```rust
use crate::components::{Chip, ChipVariant, InputSize, TextInput};
```

Replace the `DeviceFilterRow` body (around line 573-608) with:

```rust
fn DeviceFilterRow(chips: Vec<DeviceChip>, selected: Signal<Option<DeviceId>>) -> Element {
    if chips.is_empty() {
        return rsx! {};
    }
    rsx! {
        div {
            class: "if-rail__device-filter",
            role: "group",
            "aria-label": "Filter mappings by device",
            for chip in chips {
                {
                    let active = selected.read().as_ref() == Some(&chip.id);
                    let id = chip.id.clone();
                    let label = chip.label.clone();
                    // The wrapping button carries `aria-pressed`; the CSS
                    // rule `.if-rail__device-chip[aria-pressed="true"] >
                    // .if-chip` styles the active state. Chip itself stays
                    // ARIA-neutral and class-neutral.
                    rsx! {
                        button {
                            r#type: "button",
                            class: "if-rail__device-chip",
                            "aria-pressed": if active { "true" } else { "false" },
                            title: "{label}",
                            onclick: move |_| {
                                let mut selected = selected;
                                if selected.peek().as_ref() == Some(&id) {
                                    selected.set(None);
                                } else {
                                    selected.set(Some(id.clone()));
                                }
                            },
                            Chip {
                                variant: ChipVariant::Outline,
                                "{label}"
                            }
                        }
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 4: Update CSS for the chip strip and the active extension**

Open `crates/inputforge-gui-dx/assets/frame/mapping_list.css`. Replace the existing `.if-rail__device-filter`, `.if-rail__device-chip`, `.if-rail__device-chip.is-active`, and `.if-rail__device-chip:focus-visible` rules (lines 41-76 today) with:

```css
.if-rail__device-filter {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
    padding: var(--space-1) var(--space-3) var(--space-2);
    flex: 0 0 auto;
}

/* The wrapping button stays a click target. Visual chrome lives on the
 * inner Chip (.if-chip / .if-chip--outline). The button reset below
 * keeps it ARIA-correct without painting any chrome of its own. */
.if-rail__device-chip {
    background: transparent;
    border: 0;
    padding: 0;
    cursor: pointer;
    font: inherit;
}

/* Active extension: the wrapping button's `aria-pressed="true"` is the
 * single source of active-state truth (both for ARIA and for CSS via
 * the attribute selector). The treatment is the unified row-active
 * idiom (border-focus + primary-tinted parent surface), parameterized
 * to the rail's --color-bg-elevated since the chip strip sits on the
 * rail's elevated bar. */
.if-rail__device-chip[aria-pressed="true"] > .if-chip {
    border-color: var(--color-border-focus);
    background: color-mix(in srgb, var(--color-primary) var(--tint-selected), var(--color-bg-elevated));
    color: var(--color-primary);
}

.if-rail__device-chip:focus-visible {
    outline: 2px solid var(--color-border-focus);
    outline-offset: 2px;
    border-radius: var(--radius-sm);
}
```

(The button's `aria-pressed` attribute is the single source of active-state truth, both for ARIA and for CSS via the attribute selector.)

- [ ] **Step 5: Update legacy chip strip test**

Edit `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`. Update `mapping_list_renders_single_row_device_filter_chips` and `mapping_list_device_chips_are_toggle_buttons` to match the new markup: replace `if-rail__device-chip` button-class assertions with assertions on the inner `.if-chip` rendering (existing tests still need the toggle button title/aria-pressed shape). Concretely, the existing assertions on `if-rail__device-chip`, `type="button"`, `aria-pressed="false"`, `title="Twin Stick"` all stay (the wrapping button retains those); the `mapping_list_css_keeps_device_chips_one_row` test was already removed in Step 1 of this task. No new assertions needed beyond what Step 1 added.

- [ ] **Step 6: Run all device-filter tests**

Run: `cargo test -p inputforge-gui-dx mapping_list_renders_single_row_device_filter_chips mapping_list_device_chips_are_toggle_buttons mapping_list_css_wraps_device_filter_chips_into_multiple_rows device_filter_active_chip_emits_unified_active_class -- --test-threads=1`
Expected: all PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs \
        crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs \
        crates/inputforge-gui-dx/assets/frame/mapping_list.css
git commit -m "feat(mapping-list): migrate device filter chips to Chip Outline with unified active treatment"
```

---

## Task 10: Filter input bottom hairline cleanup

**Files:**
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_list.css`

- [ ] **Step 1: Drop the substituted-comment line**

Edit `crates/inputforge-gui-dx/assets/frame/mapping_list.css`. Replace the `.if-rail__filter` block (lines 31-35 today) with:

```css
.if-rail__filter {
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--color-border);
}
```

(The `/* substituted --color-border for missing --color-border-subtle */` comment is dropped; `--color-border` is the correct token name.)

- [ ] **Step 2: Smoke check**

Run: `cargo test -p inputforge-gui-dx -- --test-threads=1`
Expected: all tests PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/assets/frame/mapping_list.css
git commit -m "chore(mapping-list): drop misleading filter hairline substitution comment"
```

---

## Task 11: Group header padding + post-filter row count

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/group.rs`
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_list.css`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`

- [ ] **Step 1: Write failing tests for header markup and CSS**

Append to `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`:

```rust
#[test]
fn group_header_renders_post_filter_row_count() {
    use inputforge_core::action::Mapping;
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    use std::collections::HashMap;

    fn TestComponent() -> Element {
        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();
        let mut mappings = vec![];
        for i in 0..3 {
            mappings.push(Mapping {
                input: InputAddress::Bound {
                    device: DeviceId("dev".to_owned()),
                    input: InputId::Axis { index: i },
                },
                mode: "Default".to_owned(),
                name: Some(format!("Axis{i}")),
                actions: vec![],
            });
        }
        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);

        provide_minimal_contexts();
        let mut cfg_signal = use_context::<AppContext>().config;
        let mut meta_signal = use_context::<AppContext>().meta;
        use_hook(move || {
            cfg_signal.set(ConfigSnapshot::from_state(&state, None));
            meta_signal.set(MetaSnapshot::from_state(&state));
        });

        rsx! { MappingList {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-rail__group-header__count"),
        "group header must render a count slot class so the count reads as data: {html}",
    );
    assert!(
        html.contains("\"if-rail__group-header__count\">3"),
        "axes group with 3 mappings must show the count `3` inside the canonical group-header count slot: {html}",
    );
}

#[test]
fn mapping_list_css_aligns_group_header_gutter_with_row_padding() {
    let css = include_str!("../../../assets/frame/mapping_list.css");
    let block = css
        .split(".if-rail__group-header {")
        .nth(1)
        .expect(".if-rail__group-header rule present")
        .split('}')
        .next()
        .expect(".if-rail__group-header rule closed");
    assert!(
        block.contains("padding: var(--space-3) var(--space-3) var(--space-1);"),
        "group header horizontal gutter must match the new row padding (--space-3): {block}",
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx group_header_renders_post_filter_row_count mapping_list_css_aligns_group_header_gutter_with_row_padding -- --test-threads=1`
Expected: FAIL on both.

- [ ] **Step 3: Add a `header_with_count` helper to `group.rs`**

Open `crates/inputforge-gui-dx/src/frame/mapping_list/group.rs`. Add (or update) the `GroupKind::header()` adjacent helper. If `header()` already returns a `&'static str`, leave it alone; add a sibling free helper that takes `(group, count)` and returns a `(label, count)` pair. The exact signature does not matter; what matters is that the orchestrator passes the post-filter count of rows in that group to the header rendering.

- [ ] **Step 4: Update the orchestrator to render the count**

Open `crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs`. Around line 494, replace:

```rust
                div { class: "if-rail__group-header", {group.header()} }
```

with:

```rust
                div { class: "if-rail__group-header",
                    span { class: "if-rail__group-header__label", {group.header()} }
                    span { class: "if-rail__group-header__count", "{group_len}" }
                }
```

`group_len` is already in scope (computed two lines above on line 419 via `let group_len = group_rows.len();`). It is the post-filter count, which is what the user sees scrolling.

- [ ] **Step 5: Update CSS for the group header**

Open `crates/inputforge-gui-dx/assets/frame/mapping_list.css`. Replace the existing `.if-rail__group-header { ... }` rule with:

```css
.if-rail__group-header {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-3) var(--space-1);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    color: var(--color-text-muted);
    text-transform: uppercase;
}

.if-rail__group-header__count {
    font-family: var(--font-mono);
    font-weight: 500;
    color: var(--color-border-strong);
    letter-spacing: 0;
    text-transform: none;
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx group_header_ mapping_list_css_aligns_group_header -- --test-threads=1`
Expected: PASS for both. The existing `mapping_list_renders_axes_and_buttons_groups_in_order` test still passes (it only asserts `AXES`/`BUTTONS` substrings, which the label span still contains).

- [ ] **Step 7: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs \
        crates/inputforge-gui-dx/src/frame/mapping_list/group.rs \
        crates/inputforge-gui-dx/assets/frame/mapping_list.css \
        crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs
git commit -m "feat(mapping-list): add post-filter group counts and align header gutter"
```

---

## Task 12: Inline output Chip on the source line (drop `if-row__output-badge`)

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/row.rs`
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_list.css`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`

Multi-out mappings: `MappingSummary` exposes only `first_vjoy_output` today. The inline chip renders only the primary output; secondary outputs are not signalled in the rail row, matching the existing rail information density. The mapping editor's pipeline is the canonical place to inspect all outputs of a multi-out mapping.

- [ ] **Step 1: Write failing tests**

Append to `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`:

```rust
#[test]
fn row_output_chip_replaces_legacy_output_badge() {
    use crate::context::{GlyphFlags, MappingSummary};
    use crate::frame::mapping_list::row::Row;
    use inputforge_core::types::{
        DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis,
    };

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let summary = MappingSummary {
            input: InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Axis { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Pitch".to_owned()),
            glyphs: GlyphFlags::default(),
            referenced_devices: vec![DeviceId("dev".to_owned())],
            first_vjoy_output: Some(OutputAddress {
                device: 2,
                output: OutputId::Axis { id: VJoyAxis::X },
            }),
        };
        let renaming: Signal<Option<InputAddress>> = use_signal(|| None);
        let sortable = use_sortable_state::<u32>();
        rsx! {
            Row {
                summary: summary,
                is_active: false,
                renaming: renaming,
                sortable: sortable,
                filter_active: false,
                on_open_menu: move |_: (InputAddress, f64, f64)| {},
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        !html.contains("if-row__output-badge"),
        "legacy .if-row__output-badge class must NOT render after the migration: {html}",
    );
    let chip_count = html.matches("if-chip--output").count();
    assert_eq!(
        chip_count, 1,
        "row with first_vjoy_output must render exactly one .if-chip--output element; got {chip_count} in: {html}",
    );
    assert!(
        html.contains("\u{2192}"),
        "source line must include the arrow glyph U+2192 separating trigger from output: {html}",
    );
    assert!(
        html.contains("aria-hidden=\"true\""),
        "arrow glyph must be aria-hidden so screen readers rely on label sequence: {html}",
    );
}
```

(Update the existing `row_renders_compact_vjoy_output_badge` test in the same diff: change the final two assertions from `if-row__output-badge` (present) and `!if-row__source-input` (absent) to `if-chip--output` (present) plus the arrow glyph. The first three assertions on `vJoy 2` and `X` and `if-row__source-input` absence stay as-is.)

- [ ] **Step 2: Run tests to verify the new ones fail**

Run: `cargo test -p inputforge-gui-dx row_output_chip_replaces_legacy_output_badge row_renders_compact_vjoy_output_badge -- --test-threads=1`
Expected: FAIL on both.

- [ ] **Step 3: Update `Row` to inline the output**

Open `crates/inputforge-gui-dx/src/frame/mapping_list/row.rs`. Add `Chip, ChipVariant` to the imports:

```rust
use crate::components::{Chip, ChipVariant};
```

Replace the `div { class: "if-row__source-primary", ... }` block (lines 180-189) with:

```rust
                div { class: "if-row__source-primary",
                    span { class: "if-row__source-device", "{device_label}" }
                    if let Some(output) = &summary.first_vjoy_output {
                        span { class: "if-row__source-arrow", "aria-hidden": "true", "\u{2192}" }
                        Chip {
                            variant: ChipVariant::Output,
                            class: "if-row__output-chip".to_owned(),
                            title: compact_output_label(output),
                            "{compact_output_label(output)}"
                        }
                    }
                }
```

The `title` prop on Chip lands in Task 2 and forwards to the rendered `<span>`. No wrapper span is needed.

- [ ] **Step 4: Drop legacy CSS for the badge**

Open `crates/inputforge-gui-dx/assets/frame/mapping_list.css`. Delete the `.if-row__output-badge { ... }` rule (lines 148-161 today). Replace with a tiny rule for the arrow glyph and the consumer-side shape of the inline chip:

```css
.if-row__source-arrow {
    flex: 0 0 auto;
    color: var(--color-border-strong);
    font-family: var(--font-mono);
    font-size: 11px;
}

.if-row__output-chip {
    /* Output chips never wrap or truncate; vJoy identifiers are short.
       Rail row at 280px width fits the chip without ellipsis. */
    max-width: none;
    flex: 0 0 auto;
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx row_ -- --test-threads=1`
Expected: all row tests PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/row.rs \
        crates/inputforge-gui-dx/assets/frame/mapping_list.css \
        crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs
git commit -m "feat(mapping-list): inline vJoy output as Chip with arrow separator"
```

---

## Task 13: Migrate qualifier chips to `Chip` Outline

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/row.rs`
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_list.css`
- Modify: `crates/inputforge-gui-dx/assets/components/chip.css`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`

- [ ] **Step 1: Write a failing test**

Append to `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`:

```rust
#[test]
fn qualifier_chips_render_as_chip_outline_with_glyph_class() {
    use crate::context::{GlyphFlags, MappingSummary};
    use crate::frame::mapping_list::row::Row;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let summary = MappingSummary {
            input: InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Axis { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Throttle".to_owned()),
            glyphs: GlyphFlags {
                merge_secondary: Some(InputAddress::Bound {
                    device: DeviceId("dev".to_owned()),
                    input: InputId::Axis { index: 1 },
                }),
                first_input_predicate: Some(InputAddress::Bound {
                    device: DeviceId("dev".to_owned()),
                    input: InputId::Button { index: 3 },
                }),
            },
            referenced_devices: vec![DeviceId("dev".to_owned())],
            first_vjoy_output: None,
        };
        let renaming: Signal<Option<InputAddress>> = use_signal(|| None);
        let sortable = use_sortable_state::<u32>();
        rsx! {
            Row {
                summary: summary,
                is_active: false,
                renaming: renaming,
                sortable: sortable,
                filter_active: false,
                on_open_menu: move |_: (InputAddress, f64, f64)| {},
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-chip--outline"),
        "qualifier chips must use Chip Outline variant: {html}",
    );
    assert!(
        html.contains("glyph-merge"),
        "merge glyph class must remain so the leading glyph keeps its --color-output hue: {html}",
    );
    assert!(
        html.contains("glyph-cond"),
        "conditional glyph class must remain so the leading glyph keeps its --color-control-badge-text hue: {html}",
    );
    assert!(
        !html.contains("if-row__chip\""),
        "legacy .if-row__chip class (without the chip-glyph suffix) must be retired: {html}",
    );
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p inputforge-gui-dx qualifier_chips_render_as_chip_outline_with_glyph_class -- --exact`
Expected: FAIL.

- [ ] **Step 3: Update `Row` to render Chip Outline for qualifiers**

Edit `crates/inputforge-gui-dx/src/frame/mapping_list/row.rs`. Replace the qualifier rendering block (lines 191-208) with:

```rust
                if merge_glyph.is_some() || cond_glyph.is_some() {
                    div { class: "if-row__source-qualifiers",
                        if let Some(secondary_label) = merge_glyph {
                            Chip {
                                variant: ChipVariant::Outline,
                                class: "if-row__qualifier glyph-merge".to_owned(),
                                span { class: "if-row__chip-glyph", "+" }
                                span { class: "if-row__chip-text", "{secondary_label}" }
                            }
                        }
                        if let Some(predicate_label) = cond_glyph {
                            Chip {
                                variant: ChipVariant::Outline,
                                class: "if-row__qualifier glyph-cond".to_owned(),
                                span { class: "if-row__chip-glyph", "\u{2295}" }
                                span { class: "if-row__chip-text", "{predicate_label}" }
                            }
                        }
                    }
                }
```

(Wrap the `Chip` body in a `title` span at the call site if a tooltip is needed; the legacy `title="Merge: {secondary_label}"` attribute belongs on the wrapping element, not on Chip. Kept inside an enclosing `span { title: ..., Chip { ... } }` if the tooltip is load-bearing for keyboard or AT users.)

- [ ] **Step 4: Drop legacy `.if-row__chip` CSS, lift the glyph rules**

Open `crates/inputforge-gui-dx/assets/frame/mapping_list.css`. Delete the `.if-row__chip { ... }`, `.if-row__chip-glyph { ... }`, `.glyph-merge .if-row__chip-glyph { ... }`, `.glyph-cond .if-row__chip-glyph { ... }`, and `.if-row__chip-text { ... }` rules (lines 221-250). Replace with a thin block keyed off `.if-row__qualifier`:

```css
.if-row__qualifier {
    /* Qualifier chips inherit Chip Outline base. Override only what
       differs from a generic outline chip: italic body text + a
       per-kind glyph hue carried by `.glyph-merge` / `.glyph-cond`. */
    font-style: italic;
    font-size: 10px;
}

.if-row__qualifier .if-row__chip-glyph {
    flex: 0 0 auto;
    font-family: var(--font-mono);
    font-weight: 600;
    font-style: normal;
}

.if-row__qualifier.glyph-merge .if-row__chip-glyph { color: var(--color-output); }
.if-row__qualifier.glyph-cond  .if-row__chip-glyph { color: var(--color-control-badge-text); }

.if-row__qualifier .if-row__chip-text {
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx qualifier_chips_render_as_chip_outline_with_glyph_class row_glyphs_render_for_merge_and_conditional -- --test-threads=1`
Expected: PASS for both.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/row.rs \
        crates/inputforge-gui-dx/assets/frame/mapping_list.css \
        crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs
git commit -m "feat(mapping-list): migrate qualifier chips to Chip Outline"
```

---

## Task 14: Dashed `+ Add mapping` footer cohesion delta

**Files:**
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_list.css`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`

- [ ] **Step 1: Write a failing test for the dashed footer hover treatment**

Append to `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`:

```rust
#[test]
fn mapping_list_css_locks_dashed_add_row_cohesion() {
    let css = include_str!("../../../assets/frame/mapping_list.css");

    let block = css
        .split(".if-add-inline__dashed-row {")
        .nth(1)
        .expect(".if-add-inline__dashed-row rule present")
        .split('}')
        .next()
        .expect(".if-add-inline__dashed-row rule closed");
    assert!(
        block.contains("border: 1px dashed var(--color-border-strong);"),
        "dashed footer must use --color-border-strong (matches profiles' + New profile): {block}",
    );
    assert!(
        block.contains("border-radius: var(--radius-md);"),
        "dashed footer must bump radius to --radius-md (parity with rows): {block}",
    );

    let hover = css
        .split(".if-add-inline__dashed-row:hover {")
        .nth(1)
        .expect(".if-add-inline__dashed-row:hover rule present")
        .split('}')
        .next()
        .expect(".if-add-inline__dashed-row:hover rule closed");
    assert!(
        hover.contains("border-color: var(--color-border-focus);"),
        "dashed footer hover must use --color-border-focus (unified active border idiom): {hover}",
    );
    assert!(
        hover.contains(
            "background: color-mix(in srgb, var(--color-primary) var(--tint-create), var(--color-bg));"
        ),
        "dashed footer hover must mix primary at --tint-create into --color-bg \
         (reads as create rather than selected): {hover}",
    );
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p inputforge-gui-dx mapping_list_css_locks_dashed_add_row_cohesion -- --exact`
Expected: FAIL on multiple substring assertions.

- [ ] **Step 3: Update the dashed footer CSS**

Open `crates/inputforge-gui-dx/assets/frame/mapping_list.css`. Replace the `.if-add-inline__dashed-row { ... }` and `.if-add-inline__dashed-row:hover { ... }` rules (lines 293-311 today) with:

```css
.if-add-inline__dashed-row {
    width: 100%;
    border: 1px dashed var(--color-border-strong);
    background: transparent;
    color: var(--color-text);
    font: inherit;
    font-size: 12px;
    font-weight: 500;
    padding: var(--space-2);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition:
        background var(--duration-fast) var(--easing-fast),
        border-color var(--duration-fast) var(--easing-fast);
}

.if-add-inline__dashed-row:hover {
    background: color-mix(in srgb, var(--color-primary) var(--tint-create), var(--color-bg));
    border-color: var(--color-border-focus);
    color: var(--color-text);
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p inputforge-gui-dx mapping_list_css_locks_dashed_add_row_cohesion -- --exact`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/assets/frame/mapping_list.css crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs
git commit -m "feat(mapping-list): align dashed Add mapping row with profiles + tint-create hover"
```

---

## Task 15: Pad shell border drop + Capture chip migration

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/add_inline.rs`
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_list.css`

- [ ] **Step 1: Update `AddInline` to render Chip Capture**

Edit `crates/inputforge-gui-dx/src/frame/mapping_list/add_inline.rs`. Add `Chip, ChipVariant` to the components import:

```rust
use crate::components::{Button, ButtonSize, ButtonVariant, Chip, ChipVariant, IconButton, InputSize, TextInput};
```

In the `if is_capturing { ... } else { ... }` block (around lines 499-512), replace the legacy `span { class: "if-add-inline__chip", ... }` render path. The `data-kind` attribute lives on a parent span of the Chip (Task 4's CSS already keys off `[data-kind="..."] > .if-chip--capture`), and `aria-label`/text content live on the Chip itself:

```rust
                        if is_capturing {
                            span { "aria-label": "Listening for input",
                                Chip {
                                    variant: ChipVariant::Capture,
                                    class: "if-add-inline__chip if-add-inline__chip--listening".to_owned(),
                                }
                            }
                        } else {
                            span { "data-kind": "{kind_class}",
                                Chip {
                                    variant: ChipVariant::Capture,
                                    class: "if-add-inline__chip".to_owned(),
                                    "{chip_label}"
                                }
                            }
                        }
```

The listening branch does not need `data-kind` (the listening modifier overrides currentColor explicitly per Step 3 below). The `aria-label` lives on the wrapping span so AT readers still hear the listening cue (Chip itself is ARIA-neutral).

- [ ] **Step 2: Drop the pad shell's focus-cyan border, lower to border-strong**

In `crates/inputforge-gui-dx/assets/frame/mapping_list.css`, replace:

```css
.if-add-inline--pad {
    background: var(--color-bg-elevated);
    border: 1px solid var(--color-border-focus);
    border-radius: var(--radius-sm);
    padding: var(--space-2);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
}
```

with:

```css
.if-add-inline--pad {
    background: var(--color-bg-elevated);
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-md);
    padding: var(--space-2);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
}
```

(Border tier moves from `--color-border-focus` to `--color-border-strong`; focus-ring semantics are reserved for actually-focused elements like the input, refresh button, and action buttons. Radius bumps to `--radius-md` for parity with the dashed row.)

- [ ] **Step 3: Drop the legacy `.if-add-inline__chip` block, keep listening modifier**

In the same file, delete the `.if-add-inline__chip { ... }` rule (around line 350-365) and the per-`data-kind` rules (lines 367-370). Keep the `.if-add-inline__chip--listening { ... }` rule and the `::before` keyframes (lines 376-401). Add a small consumer-side override on the listening modifier so it resolves over Chip Capture's currentColor base:

```css
/* Listening modifier on Chip Capture: neutral chip with a phosphor dot
   pulsing inside. Reuses --color-live, the system's universal "this
   surface is listening" signal. Overrides Chip Capture's currentColor
   resolution so the chip stays neutral until capture commits the
   taxonomy hue. */
.if-chip.if-add-inline__chip--listening {
    color: var(--color-text-muted);
    background: var(--color-bg-sunken);
    border-color: var(--color-border-strong);
}

.if-chip.if-add-inline__chip--listening::before {
    content: "";
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-live);
    animation: if-add-pulse-dot 1100ms ease-in-out infinite;
}
```

- [ ] **Step 4: Smoke check**

Run: `cargo test -p inputforge-gui-dx -- --test-threads=1`
Expected: existing `add_inline_force_expanded_arms_capture` test still passes (it asserts `if-add-inline__chip--listening` substring, which the migrated render path keeps as a class on Chip Capture).

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/add_inline.rs \
        crates/inputforge-gui-dx/assets/frame/mapping_list.css
git commit -m "feat(add-inline): migrate capture chip to Chip Capture and lower pad border tier"
```

---

## Task 16: Collision text leads with Badge Warning

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/add_inline.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`

- [ ] **Step 1: Write a failing source-shape test**

The Collision state is driven by an asynchronous capture event that SSR cannot replay deterministically without exposing internal state. Assert the contract at the source level instead: the `AddState::Collision` arm must reference `BadgeVariant::Warning` before the prose. Append to `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`:

```rust
#[test]
fn add_inline_collision_arm_leads_with_warning_badge() {
    let src = include_str!("../../../src/frame/mapping_list/add_inline.rs");
    let arm_start = src
        .find("AddState::Collision {")
        .expect("Collision arm must exist in add_inline.rs");
    // Bound the arm window at the next top-level state arm or the
    // function close. 2000 chars is comfortably more than the arm body
    // and stops before any sibling code that would mention Badge for
    // unrelated reasons.
    let arm_window_end = (arm_start + 2000).min(src.len());
    let arm_window = &src[arm_start..arm_window_end];

    let badge_pos = arm_window
        .find("BadgeVariant::Warning")
        .unwrap_or_else(|| panic!(
            "Collision arm must reference Badge variant=Warning so the visual \
             scan parity with the status bar's `1 warning` badge holds. Window:\n{arm_window}",
        ));
    let prose_pos = arm_window
        .find("already mapped to")
        .expect("Collision arm must keep the existing prose sentence");
    assert!(
        badge_pos < prose_pos,
        "Badge Warning must render BEFORE the `already mapped to` prose. \
         badge_pos={badge_pos}, prose_pos={prose_pos}",
    );
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p inputforge-gui-dx add_inline_collision_arm_leads_with_warning_badge -- --exact`
Expected: FAIL on the `BadgeVariant::Warning` lookup (today the Collision arm renders bare `<em>` + `<strong>`).

- [ ] **Step 3: Update the Collision arm in `AddInline`**

Edit `crates/inputforge-gui-dx/src/frame/mapping_list/add_inline.rs`. Add `Badge, BadgeVariant` to imports:

```rust
use crate::components::{Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, Chip, ChipVariant, IconButton, InputSize, TextInput};
```

In the Collision arm (around lines 583-590), replace the `div { class: "if-add-inline__collision-text", em { ... } strong { ... } "." }` block with:

```rust
                    div { class: "if-add-inline__collision-text",
                        Badge { variant: BadgeVariant::Warning, "Collision" }
                        span { class: "if-add-inline__collision-text-body",
                            " {captured_label} already mapped to "
                            strong { "{existing_name}" }
                            "."
                        }
                    }
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p inputforge-gui-dx add_inline_collision_arm_leads_with_warning_badge -- --exact`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/add_inline.rs \
        crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs
git commit -m "feat(add-inline): lead collision text with Warning Badge for status bar parity"
```

---

## Task 17: Action-row hairline cleanup (drop substituted comment)

**Files:**
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_list.css`

- [ ] **Step 1: Drop the substituted-comment line**

Open `crates/inputforge-gui-dx/assets/frame/mapping_list.css`. Replace the `.if-add-inline__actions { ... }` rule's `border-top` line. Today:

```css
    border-top: 1px solid var(--color-border);
```

Sweep all `/* substituted ... */` comments left in the file. The Task 7 and Task 14 rewrites should have already removed several; this task verifies and drops any residue. Run `rg "/\* substituted" crates/inputforge-gui-dx/assets/frame/mapping_list.css` and delete every match. Expected residue locations after Tasks 7 and 14:

- Line 33: `/* substituted --color-border for missing --color-border-subtle */` -> delete line
- Line 124 area (post-Task-7): verify gone; if present, delete
- Line 308 area (post-Task-14): verify gone; if present, delete
- Lines 451-453: `/* substituted --color-border-focus for missing --color-focus-cyan */` -> drop line, keep declaration
- Lines 463-465: `/* substituted --color-border-focus for missing --color-focus-ring */` -> drop line, keep declaration
- Lines 477-478: `/* substituted --color-bg-elevated for missing --color-surface-2 */` -> drop line
- Lines 481-482: `/* substituted --shadow-3 for missing --shadow-lg ... */` -> drop line
- Lines 501-502: `/* substituted --color-border for missing --color-surface-hover */` -> drop line

- [ ] **Step 2: Smoke check**

Run: `cargo test -p inputforge-gui-dx -- --test-threads=1`
Expected: all tests PASS (no test asserts the comments).

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/assets/frame/mapping_list.css
git commit -m "chore(mapping-list): drop misleading substitution comments throughout"
```

---

## Task 18: Status bar typography deltas (warning glyph + mono numerator)

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/status_bar/mod.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/status_bar/logic.rs`
- Modify: `crates/inputforge-gui-dx/assets/frame/status_bar.css`
- Create: `crates/inputforge-gui-dx/src/frame/status_bar/tests.rs`

- [ ] **Step 1: Create the tests file with failing assertions**

Create `crates/inputforge-gui-dx/src/frame/status_bar/tests.rs`:

```rust
//! Tests for the F7 status bar. Lock the typography deltas and the
//! surface contract in shape parallel to the right-panel passes.

#![allow(non_snake_case, reason = "Dioxus components are PascalCase")]

use std::sync::{Arc, mpsc};

use dioxus::prelude::*;
use dioxus_ssr::render;
use parking_lot::RwLock;

use inputforge_core::settings::AppSettings;
use inputforge_core::state::AppState;

use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot};
use crate::frame::status_bar::StatusBar;
use crate::patterns::live_capture::use_live_capture_provider;
use crate::toast::{ToastQueue, ToastState};

fn provide_minimal_contexts() {
    let (cmd_tx, _cmd_rx) = mpsc::channel();
    let ctx = AppContext {
        state: Arc::new(RwLock::new(AppState::new())),
        commands: cmd_tx,
        settings: Arc::new(AppSettings::default()),
        meta: use_signal(MetaSnapshot::default),
        config: use_signal(ConfigSnapshot::default),
        live: use_signal(LiveSnapshot::default),
    };
    use_context_provider(|| ctx.clone());
    let view = crate::frame::use_view_state_provider(ctx.meta);
    use_context_provider(|| view);
    let toast_state = use_signal(ToastState::default);
    use_context_provider(|| ToastQueue { state: toast_state });
    use_live_capture_provider();
}

#[test]
fn status_bar_warning_badge_includes_leading_glyph() {
    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let ctx = use_context::<AppContext>();
        let mut meta = ctx.meta;
        use_hook(move || {
            let mut snap = MetaSnapshot::default();
            snap.warnings.push("dummy".to_owned());
            meta.set(snap);
        });
        rsx! { StatusBar {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-badge--warning"),
        "warning slot must remain a Badge Warning: {html}",
    );
    assert!(
        html.contains("\u{26A0}"),
        "warning Badge must include the leading U+26A0 glyph for visual scan parity: {html}",
    );
}

#[test]
fn status_bar_device_count_numerator_uses_mono_text_class() {
    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let ctx = use_context::<AppContext>();
        let mut cfg = ctx.config;
        use_hook(move || {
            cfg.set(ConfigSnapshot {
                devices: vec![],
                ..ConfigSnapshot::default()
            });
        });
        rsx! { StatusBar {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-frame-status-bar__count-numerator"),
        "device count numerator must carry a mono class so the digits read as data \
         rather than chrome: {html}",
    );
}

#[test]
fn status_bar_css_locks_surface_contract() {
    // DESIGN.md section 7 Status Bar contract. The bar lives at shell
    // level (NOT under the Pinned-Inspector vs Collapsible-Drawer rule);
    // surface stays --color-bg-sunken with a 1px --color-border-strong
    // top hairline. This test mirrors the shape of profiles'
    // _collapsible_drawer_surface_contract test.
    let css = include_str!("../../../assets/components/status-bar.css");
    let block = css
        .split(".if-status-bar {")
        .nth(1)
        .expect(".if-status-bar rule present")
        .split('}')
        .next()
        .expect(".if-status-bar rule closed");
    assert!(
        block.contains("background: var(--color-bg-sunken);"),
        ".if-status-bar must declare bg-sunken per DESIGN.md section 7: {block}",
    );
    assert!(
        block.contains("border-top: 1px solid var(--color-border-strong);"),
        ".if-status-bar must declare a 1px strong-border top hairline per \
         DESIGN.md section 7: {block}",
    );
    assert!(
        block.contains("height: 28px;"),
        ".if-status-bar height contract is 28px (matches egui shell): {block}",
    );
}
```

Wire the new `tests` module by appending to `crates/inputforge-gui-dx/src/frame/status_bar/mod.rs` (top of file, after existing `mod logic;`):

```rust
mod logic;

#[cfg(test)]
mod tests;
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --test-threads=1 status_bar_warning_badge_includes_leading_glyph status_bar_device_count_numerator_uses_mono_text_class status_bar_css_locks_surface_contract`
Expected: FAIL on the warning glyph and the numerator class. The surface contract test passes today (the contract is already met by `assets/components/status-bar.css:20-21`); confirm anyway so the test guards against future regressions.

- [ ] **Step 3: Add the warning glyph inside the Badge**

Edit `crates/inputforge-gui-dx/src/frame/status_bar/mod.rs`. Replace the `start: rsx! { ... }` slot:

```rust
            start: rsx! {
                if let Some(text) = w.as_ref() {
                    Badge { variant: BadgeVariant::Warning,
                        span { class: "if-frame-status-bar__warning-glyph", "aria-hidden": "true", "\u{26A0}" }
                        " {text}"
                    }
                }
            },
```

- [ ] **Step 4: Update `device_count_label` to split numerator from prose**

Edit `crates/inputforge-gui-dx/src/frame/status_bar/logic.rs`. Either: (a) leave `device_count_label` returning a single string and let the renderer split it, or (b) introduce a sibling `device_count_parts(&[DeviceState]) -> (String, String)` returning `("3/3", "devices")` so the renderer can wrap each independently. Option (b) is cleaner; add the function and keep `device_count_label` as a delegate for the existing tests.

```rust
/// Numerator + label split so the count digits can render with their
/// own typography (mono, --color-text) while the trailing prose stays
/// muted.
#[allow(dead_code, reason = "consumed by StatusBar component")]
pub(crate) fn device_count_parts(devices: &[DeviceState]) -> (String, String) {
    let connected = devices.iter().filter(|d| d.connected).count();
    (format!("{}/{}", connected, devices.len()), "devices".to_owned())
}

/// Single-string variant retained for existing tests and callers that
/// do not need split typography. Delegates to `device_count_parts`.
pub(crate) fn device_count_label(devices: &[DeviceState]) -> String {
    let (numerator, label) = device_count_parts(devices);
    format!("{numerator} {label}")
}
```

Edit `crates/inputforge-gui-dx/src/frame/status_bar/mod.rs`. Replace the `devices_label` memo and the middle-slot rsx with:

```rust
    let devices_parts = use_memo(move || device_count_parts(&ctx.config.read().devices));

    // ... below ...

    let (numerator, suffix) = devices_parts.read().clone();
    // ... in rsx! ...
            middle: rsx! {
                span {
                    span { class: "if-frame-status-bar__count-numerator", "{numerator}" }
                    " {suffix}"
                }
            },
```

Update the `use logic::{...}` line to import `device_count_parts` instead of (or alongside) `device_count_label`.

- [ ] **Step 5: Add the CSS for the numerator slot**

Edit `crates/inputforge-gui-dx/assets/frame/status_bar.css`. Append:

```css
.if-frame-status-bar__count-numerator {
    color: var(--color-text);
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
}

.if-frame-status-bar__warning-glyph {
    margin-right: var(--space-1);
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx status_bar_ -- --test-threads=1`
Expected: PASS for all status-bar tests.

- [ ] **Step 7: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/status_bar/mod.rs \
        crates/inputforge-gui-dx/src/frame/status_bar/logic.rs \
        crates/inputforge-gui-dx/src/frame/status_bar/tests.rs \
        crates/inputforge-gui-dx/assets/frame/status_bar.css
git commit -m "feat(status-bar): add warning glyph and mono numerator; lock surface contract"
```

---

## Task 19: Confirm `ButtonSize::Sm` on the empty-state ghost buttons

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/empty.rs`

- [ ] **Step 1: Confirm or add `size: ButtonSize::Sm`**

Edit `crates/inputforge-gui-dx/src/frame/mapping_list/empty.rs`. Both `Button` invocations in `EmptyZeroFilterResults` (around lines 53-65) currently omit `size`. Add it:

```rust
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Sm,
                        onclick: move |_| on_clear_text.call(()),
                        "Clear text"
                    }
                    // ...
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Sm,
                        onclick: move |_| clear_device.call(()),
                        "Clear device"
                    }
```

Also add `ButtonSize` to the import:

```rust
use crate::components::{Button, ButtonSize, ButtonVariant};
```

- [ ] **Step 2: Smoke check**

Run: `cargo test -p inputforge-gui-dx empty_zero_filter_results -- --test-threads=1`
Expected: all PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/empty.rs
git commit -m "chore(mapping-list): pin empty-state ghost buttons to ButtonSize::Sm"
```

---

## Task 20: Add Chip section to the component gallery

**Files:**
- Modify: `crates/inputforge-gui-dx/examples/component_gallery.rs`

- [ ] **Step 1: Add the section after the Badge block**

Edit `crates/inputforge-gui-dx/examples/component_gallery.rs`. Add `Chip, ChipVariant` to the existing components import line. Add this section directly after the Badge `section { ... }` block (around line 452):

```rust
                    section {
                        h2 { "Chip" }
                        Cluster { gap: "--space-2".to_owned(),
                            Chip { variant: ChipVariant::Outline, "Outline" }
                            Chip { variant: ChipVariant::Output, "vJoy 2 . X" }
                            span { "data-kind": "axis",
                                Chip { variant: ChipVariant::Capture, "AXIS 0" }
                            }
                            span { "data-kind": "button",
                                Chip { variant: ChipVariant::Capture, "BTN 5" }
                            }
                            span { "data-kind": "hat",
                                Chip { variant: ChipVariant::Capture, "HAT 0" }
                            }
                        }
                    }
```

- [ ] **Step 2: Smoke check**

Run: `cargo build -p inputforge-gui-dx --example component_gallery`
Expected: builds cleanly.

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/examples/component_gallery.rs
git commit -m "docs(component-gallery): add Chip section parallel to Badge"
```

---

## Task 21: Active-treatment unification test (cross-cutting)

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`

- [ ] **Step 1: Write the unification test**

Append to `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`:

```rust
#[test]
fn active_treatment_shape_is_unified_across_row_chip_and_create_row() {
    // Encodes the spec contract: row-selected, chip-active, and the
    // dashed create-row hover all share the same border + tint shape.
    // Differences allowed: parent surface (--color-bg vs
    // --color-bg-elevated) and tint percent (--tint-selected vs
    // --tint-create). Mode tabs are NOT part of this contract;
    // they keep the canonical 3px primary bottom-underline asserted in
    // mode_tabs_active_tab_renders_canonical_if_tab_active_class.
    let css = include_str!("../../../assets/frame/mapping_list.css");

    // Row selected.
    let row_active = css
        .split(".if-row.is-active {")
        .nth(1)
        .expect(".if-row.is-active block")
        .split('}')
        .next()
        .expect(".if-row.is-active closed");
    assert!(row_active.contains("border-color: var(--color-border-focus);"));
    assert!(row_active.contains(
        "background: color-mix(in srgb, var(--color-primary) var(--tint-selected), var(--color-bg));"
    ));

    // Device chip active (parent surface = --color-bg-elevated since the
    // chip strip sits on the rail's elevated bar).
    let chip_active = css
        .split(".if-rail__device-chip[aria-pressed=\"true\"] > .if-chip {")
        .nth(1)
        .expect(".if-rail__device-chip pressed block")
        .split('}')
        .next()
        .expect(".if-rail__device-chip pressed closed");
    assert!(chip_active.contains("border-color: var(--color-border-focus);"));
    assert!(chip_active.contains(
        "background: color-mix(in srgb, var(--color-primary) var(--tint-selected), var(--color-bg-elevated));"
    ));

    // Dashed footer hover (tint swaps to --tint-create so the affordance
    // reads as `create` rather than `selected`; border idiom matches).
    let dashed_hover = css
        .split(".if-add-inline__dashed-row:hover {")
        .nth(1)
        .expect(".if-add-inline__dashed-row hover block")
        .split('}')
        .next()
        .expect(".if-add-inline__dashed-row hover closed");
    assert!(dashed_hover.contains("border-color: var(--color-border-focus);"));
    assert!(dashed_hover.contains(
        "background: color-mix(in srgb, var(--color-primary) var(--tint-create), var(--color-bg));"
    ));
}
```

- [ ] **Step 2: Run the test**

Run: `cargo test -p inputforge-gui-dx active_treatment_shape_is_unified_across_row_chip_and_create_row -- --exact`
Expected: PASS (Tasks 7, 9, 14 already established the three CSS rules referenced).

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs
git commit -m "test(mapping-list): lock unified active-treatment shape across row, chip, create-row"
```

---

## Task 22: Manual GUI verification

**Files:**
- (no code changes; verification only)

- [ ] **Step 1: Launch the GUI**

Run: `dx run -p inputforge-app`

This opens the desktop window. Wait for the Profiles panel to load and pick (or create) a profile with at least one mapping per group (Axes, Buttons) plus a vJoy output.

- [ ] **Step 2: Walk the cohesion checklist**

Verify by inspection:

1. Mapping list rail rows: hover paints `--color-bg-elevated` background + `--color-border-strong` border; selected row shows the cooler bordered tint with bolder name; focus-visible inset 2px ring matches the Devices panel.
2. Device filter chips: idle reads as Outline, active mirrors the row selected state, chips wrap onto multiple rows at 280px rail width.
3. Group headers: post-filter row count appears in mono `--color-border-strong` after the label.
4. Source line: device label, input id, arrow glyph (right-pointing), output Chip in a single horizontal flow (no right-anchored badge).
5. Qualifier chips: italic body text, leading mono `+` (gold) and `\u{2295}` (violet).
6. Dashed `+ Add mapping` footer: hover shows the `--tint-create` mixed background with the focus-cyan border, radius matches a row.
7. Mode tab strip: active tab carries the 3px primary underline (no fill), live-mode pip appears beside the running tab's label, trailing `+` sits flush after the last tab and is reachable by Tab key but NOT by ArrowRight from the last tab.
8. Status bar: warning chip leads with `\u{26A0}` glyph, `3/3 devices` numerator reads as bright mono text, `devices` label stays muted, profile path stays right-aligned.

If any of the above are wrong, do NOT mark this task complete; capture the deviation, identify the failing task, and fix in place.

- [ ] **Step 3: Run the full test suite once more**

Run: `cargo test -p inputforge-gui-dx -- --test-threads=1`
Expected: all PASS.

- [ ] **Step 4: Commit (if anything was tweaked)**

If Step 2 surfaced a tweak, commit it with the appropriate scope. Otherwise nothing to commit; mark this task complete and move on.

---
