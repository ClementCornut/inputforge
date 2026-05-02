# Menu Primitive Consolidation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Consolidate four menu surfaces (`AddPalette`, `StageActionsMenu`, `ModeTabContextMenu`, plus the existing `MenuRoot` consumer in the component gallery) onto one shared menu primitive that handles backdrop, ESC, arrow-key navigation, ARIA, and both trigger-attached and cursor-anchored positioning.

**Architecture:** Extend `components/menu/mod.rs` with three additions: an `unstyled` prop on `MenuTrigger`, an `anchor` prop on `MenuItems` (Start / Center / End), and a sibling `AnchoredMenu` component for cursor-anchored (right-click) menus. Refactor `MenuState` to dispatch closes via a `Callback<CloseReason>` so both `MenuRoot` and `AnchoredMenu` can share `MenuItem`. Migrate the three hand-rolled menus to consume the shared primitive. Delete the now-redundant CSS rules.

**Tech Stack:** Rust 2024, Dioxus 0.7, custom CSS tokens (`--shadow-3`, `--radius-md`, `--color-bg-elevated`, `--color-border-focus`). Tests via `cargo test --workspace` plus manual GUI verification via Chrome DevTools MCP per `CLAUDE.md`.

**Commit discipline:** All commits in this plan must be created via the `conventional-commits` skill (per `CLAUDE.md`). The proposed commit messages in each task are templates pending skill validation; the worker invokes the skill at commit time.

---

## Context

### Current state

Four menu surfaces exist:

| Menu | Anchor | State today |
|---|---|---|
| Component-gallery sample | trigger-attached | uses `MenuRoot` |
| `AddPalette` (the `+` button) | trigger-attached | hand-rolled local `Signal<bool>`, **no backdrop**, **no ESC**, menu lands flush-left under a full-width trigger |
| `StageActionsMenu` (right-click stage) | cursor xy | hand-rolled `.if-stage-menu` + `.if-stage-menu__backdrop`, fixed-positioned wrapper at `editor.stage_menu` coords |
| `ModeTabContextMenu` (right-click mode tab) | cursor xy or tab rect | hand-rolled `.if-modetab-context-menu` + duplicate `.if-menu__backdrop` (z-index 100), with `CloseReason` re-focus dance |

The two right-click menus opted out of `MenuRoot` for one documented reason: it does not accept anchor coordinates. The `AddPalette` opt-out is undocumented and is the original bug.

### Existing primitive

`components/menu/mod.rs` exposes:

- `MenuRoot { class }` — provides `MenuState { open: Signal<bool>, menu_id: Signal<String> }` via context; renders a `<div class="if-menu">` wrapper.
- `MenuTrigger { class, children }` — `<button>` that toggles `open`, wired with `aria-haspopup`, `aria-expanded`, `aria-controls`.
- `MenuItems { class, children }` — `<div role="menu" id=menu_id hidden=!open>`; renders backdrop + list; handles ESC, arrow keys, Home, End; auto-focuses first item on open.
- `MenuItem { onclick, disabled, class, children }` — `<button role="menuitem">` that fires `onclick` then sets `open` to false.

CSS in `assets/components/menu.css` already tokenizes correctly (`--shadow-3`, `--radius-md`, etc.) and uses a local stacking context (`.if-menu__items` z-index 1000) so the backdrop + list compose without leaking.

### What needs adding

1. `MenuTrigger.unstyled: bool` — opt out of `if-menu__trigger` defaults so consumers can supply their own surface (AddPalette's dashed-violet "next slot" treatment).
2. `MenuItems.anchor: Anchor` — `Start | Center | End`. AddPalette wants `Center` under its full-width trigger.
3. `.if-menu--block` CSS modifier — flips `.if-menu` from `inline-flex` to `block` so the trigger inside fills the row.
4. `AnchoredMenu` — sibling component that takes `open: Option<MenuAnchor>` + `on_close: EventHandler<CloseReason>`, renders backdrop + fixed-positioned list at `(x, y)`, provides the same `MenuState` context so `MenuItem` works inside it.
5. `CloseReason` enum — `Escape | ClickOutside | Tab | ItemActivated`. Already informally exists in `mode_tabs/context_menu.rs`; promote to the shared module.
6. `MenuState.close: Callback<CloseReason>` — refactor `MenuItem` to invoke `state.close.call(CloseReason::ItemActivated)` instead of `state.open.set(false)` directly. `MenuRoot` provides a close that ignores reason; `AnchoredMenu` provides one that fires `on_close`.

### Decisions baked in

| Decision | Choice |
|---|---|
| One component or two for trigger vs anchored? | Two (`MenuRoot` and `AnchoredMenu`). Open-state shapes are genuinely different (`Signal<bool>` vs `Option<MenuAnchor>`); folding into one prop matrix would couple unrelated APIs. |
| Where does AnchoredMenu live? | Same `components/menu/mod.rs`. Single concern, easier to keep `MenuState` shape consistent. |
| MenuItem auto-close for AddPalette early-return paths | Keep MenuItem's auto-close. AddPalette's current `open_sig.set(false)` calls on engine-disconnect become redundant; the menu closes regardless of whether the insert succeeded. Behavior identical. |
| Z-index for AnchoredMenu | `1000` to match `.if-menu__items` convention. The old hand-rolled menus used 99/100/101 ad-hoc; consolidating fixes the implicit drift. |
| CSS modifier for AnchoredMenu list | Reuse `.if-menu__list` (same surface treatment) directly. AnchoredMenu has no `.if-menu__items` outer wrapper; it applies `position: fixed` + inline `top`/`left`/`z-index` via inline `style` on the list itself. The new `.if-menu__backdrop--anchored` modifier (Step 3.3) handles the backdrop's z-index lift. (Earlier draft mentioned a `.if-menu__items--anchored` modifier; not delivered.) |
| Tests | Components have no render tests today (existing tests are pure-logic). Match that: rely on `cargo check` + `cargo clippy -D warnings` + the existing logic tests passing unchanged + manual GUI verification per task. New types (`CloseReason`, `Anchor`, `MenuAnchor`) need no tests. |

---

## Task ordering and rationale

Order is chosen so each commit compiles cleanly and leaves the existing behavior working:

1. **Task 1: extend MenuTrigger + MenuItems** — pure additions. No consumer is forced to use them yet. Gallery still works.
2. **Task 2: refactor MenuState close callback + add CloseReason** — internal restructure. MenuRoot's close ignores reason, MenuItem unchanged externally. Gallery still works.
3. **Task 3: add AnchoredMenu** — new component, no consumers. Compile-only verification.
4. **Task 4: migrate AddPalette** — first real consumer. Manual GUI test fixes the originally-reported bug.
5. **Task 5: migrate StageActionsMenu** — second consumer. Existing logic tests must still pass.
6. **Task 6: migrate ModeTabContextMenu** — third consumer. Preserves the `CloseReason` re-focus dance.
7. **Task 7: cleanup + final verification** — delete dead CSS, run full test + clippy sweep, manual GUI sweep of all four menus.

Each task gets one commit (Conventional Commits per `CLAUDE.md`).

---

## Critical files

**Modify:**

- `crates/inputforge-gui-dx/src/components/menu/mod.rs` — Tasks 1, 2, 3
- `crates/inputforge-gui-dx/assets/components/menu.css` — Tasks 1, 2, 3
- `crates/inputforge-gui-dx/src/components/mod.rs` — Tasks 1, 3 (re-exports)
- `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/add_palette.rs` — Task 4
- `crates/inputforge-gui-dx/assets/frame/mapping_editor.css` — Tasks 4, 5
- `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_actions_menu.rs` — Task 5
- `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/context_menu.rs` — Task 6
- `crates/inputforge-gui-dx/assets/frame/top_bar.css` — Task 6 (delete dupe `.if-menu__backdrop` + `.if-modetab-context-menu`)

**Read-only references:**

- `crates/inputforge-gui-dx/src/components/menu/focus_walker.rs` — used by `MenuItems` and (will be) `AnchoredMenu`. No changes.
- `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs` (`StageMenuState`, `EditorState`) — Task 5 reads these.
- `crates/inputforge-gui-dx/examples/component_gallery.rs:487-499` — sanity-check the existing `MenuRoot` consumer still renders correctly after Tasks 1 and 2.

---

## Task 1: Extend MenuTrigger and MenuItems API

Adds the `unstyled` and `anchor` props plus the `.if-menu--block` modifier. No semantic changes; existing consumers (gallery) keep working.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/components/menu/mod.rs`
- Modify: `crates/inputforge-gui-dx/assets/components/menu.css`

- [ ] **Step 1.1: Add `Anchor` enum**

In `crates/inputforge-gui-dx/src/components/menu/mod.rs`, add this above `pub fn MenuRoot`:

```rust
/// Where the dropdown attaches to its trigger horizontally. `Start` = left edge,
/// `Center` = under the trigger's centerline, `End` = right edge. `Start` matches
/// the historical default and is the right pick for small triggers (icon button,
/// label-and-caret); `Center` is the right pick for full-width triggers like the
/// AddPalette `+` slot, where left-anchoring would float the menu off the trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Anchor {
    #[default]
    Start,
    Center,
    End,
}
```

- [ ] **Step 1.2: Add `unstyled` prop to MenuTrigger**

Replace the existing `MenuTrigger` body in `crates/inputforge-gui-dx/src/components/menu/mod.rs`:

```rust
#[component]
pub fn MenuTrigger(
    #[props(default)] class: Option<String>,
    /// When `true`, the `if-menu__trigger` base class is omitted, so the
    /// caller's `class` is the only surface styling. Use for triggers that
    /// already carry a non-trivial visual treatment (e.g. AddPalette's
    /// dashed-violet "next slot"). The structural attributes
    /// (`aria-haspopup`, `aria-expanded`, `aria-controls`) are unaffected.
    #[props(default)] unstyled: bool,
    /// Accessible name for icon-only triggers. When `Some`, an `aria-label`
    /// attribute is emitted; when `None`, the attribute is omitted entirely
    /// (Dioxus 0.7 skips `Option<String>` attribute values that are `None`).
    /// Required for any trigger whose visible content is an icon with no
    /// adjacent text, per WCAG 2.1 SC 4.1.2 (Name, Role, Value).
    #[props(default)] aria_label: Option<String>,
    children: Element,
) -> Element {
    let mut state = use_context::<MenuState>();
    let combined = if unstyled {
        class.as_deref().unwrap_or("").to_string()
    } else {
        merge_class("if-menu__trigger", "", class.as_deref())
    };
    let menu_id = state.menu_id.read().clone();
    let onclick = move |_| {
        let now = !*state.open.read();
        state.open.set(now);
    };
    rsx! {
        button {
            class: "{combined}",
            onclick,
            "aria-haspopup": "true",
            "aria-expanded": "{state.open.read()}",
            "aria-controls": "{menu_id}",
            "aria-label": aria_label,
            {children}
        }
    }
}
```

- [ ] **Step 1.3: Add `anchor` prop to MenuItems**

Replace the existing `MenuItems` body in `crates/inputforge-gui-dx/src/components/menu/mod.rs`:

```rust
#[component]
pub fn MenuItems(
    /// Class extension for the OUTER positioned container (`.if-menu__items`),
    /// NOT the visible list. The visible chrome (background, border, shadow,
    /// `min-width`) lives on the inner `.if-menu__list`. If you need to
    /// customise the list surface, use a descendant selector
    /// (e.g. `.your-class .if-menu__list { ... }`) rather than expecting
    /// `your-class` to land on the surface itself.
    #[props(default)] class: Option<String>,
    /// Horizontal alignment of the dropdown relative to its trigger.
    /// Defaults to `Start` (the historical behaviour). `Center` and `End`
    /// switch on CSS modifier classes that override the default `left: 0`.
    #[props(default)] anchor: Anchor,
    children: Element,
) -> Element {
    let state = use_context::<MenuState>();
    let mut open_signal = state.open;
    let menu_id = state.menu_id.read().clone();
    let anchor_class = match anchor {
        Anchor::Start => "",
        Anchor::Center => "if-menu__items--center",
        Anchor::End => "if-menu__items--end",
    };
    let combined = merge_class("if-menu__items", anchor_class, class.as_deref());

    let target_id_for_keydown = menu_id.clone();
    let onkeydown = move |evt: KeyboardEvent| {
        let action = match evt.key() {
            Key::Escape => {
                open_signal.set(false);
                return;
            }
            Key::ArrowDown => FocusAction::Next,
            Key::ArrowUp => FocusAction::Prev,
            Key::Home => FocusAction::First,
            Key::End => FocusAction::Last,
            _ => return,
        };
        focus_menu_item(&target_id_for_keydown, action);
    };
    let onclick = move |_| {
        open_signal.set(false);
    };

    let target_id_for_focus = menu_id.clone();
    use_effect(move || {
        if *open_signal.read() {
            focus_menu_item(&target_id_for_focus, FocusAction::First);
        }
    });

    let is_open = *open_signal.read();
    rsx! {
        div {
            class: "{combined}",
            id: "{menu_id}",
            role: "menu",
            tabindex: "-1",
            hidden: !is_open,
            onkeydown,
            div {
                class: "if-menu__backdrop",
                onclick,
            }
            div { class: "if-menu__list", {children} }
        }
    }
}
```

- [ ] **Step 1.4: Re-export `Anchor` from `components::menu`**

In `crates/inputforge-gui-dx/src/components/mod.rs`, change:

```rust
pub use menu::{MenuItem, MenuItems, MenuRoot, MenuTrigger};
```

to:

```rust
pub use menu::{Anchor, MenuItem, MenuItems, MenuRoot, MenuTrigger};
```

- [ ] **Step 1.5: Add CSS modifiers**

Append to `crates/inputforge-gui-dx/assets/components/menu.css`:

```css
/* Block-level container: flips .if-menu from inline-flex to block so a
   full-width trigger inside (e.g. AddPalette's "+ next slot" row) actually
   fills its parent row instead of shrink-wrapping to its content. */
.if-menu--block {
    display: block;
}

/* Anchor variants for the dropdown. Default is left-aligned (`left: 0`,
   set on `.if-menu__items`); --center pulls the menu under the trigger's
   horizontal centerline; --end pins to the trigger's right edge. */
.if-menu__items--center {
    left: 50%;
    transform: translateX(-50%);
}
.if-menu__items--end {
    left: auto;
    right: 0;
}
```

- [ ] **Step 1.6: Verify compile + gallery still renders**

Run:

```
cargo check -p inputforge-gui-dx
```

Expected: clean.

Run:

```
cargo clippy -p inputforge-gui-dx --all-targets -- -D warnings
```

Expected: clean.

Manual: `dx run -p inputforge-app --no-default-features --features gui-dioxus`. Open the gallery example route if exposed; otherwise skip until Task 4. The existing MenuRoot has no production consumer, so a clean compile is sufficient.

- [ ] **Step 1.7: Commit**

```
git add crates/inputforge-gui-dx/src/components/menu/mod.rs \
        crates/inputforge-gui-dx/src/components/mod.rs \
        crates/inputforge-gui-dx/assets/components/menu.css

git commit -m "feat(menu): add unstyled MenuTrigger and anchor prop on MenuItems"
```

---

## Task 2: Refactor MenuState close callback + add CloseReason

Introduces a uniform close pathway so `MenuItem` works under both `MenuRoot` (Task 0 baseline) and `AnchoredMenu` (Task 3). The reason payload lets cursor-anchored consumers distinguish click-outside from ESC from Tab.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/components/menu/mod.rs`

- [ ] **Step 2.1: Add `CloseReason` enum**

In `crates/inputforge-gui-dx/src/components/menu/mod.rs`, add above the `MenuState` struct:

```rust
/// Reason a menu was closed. Trigger-attached menus (`MenuRoot`) discard
/// this; cursor-anchored menus (`AnchoredMenu`) surface it through `on_close`
/// so the parent can decide whether to re-focus the originating element.
///
/// `Escape` and `ClickOutside` mean the user dismissed without picking
/// anything; the parent typically re-focuses the originating trigger.
/// `Tab` means the user pressed Tab to leave the menu; the parent must NOT
/// re-focus the trigger because the browser's natural Tab traversal is
/// moving focus to the next element. `ItemActivated` fires after a
/// `MenuItem` click; the parent's behaviour is item-specific (often
/// re-focus the trigger as a default landing spot before the activated
/// item's own follow-on focus takes over).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseReason {
    Escape,
    ClickOutside,
    Tab,
    ItemActivated,
}
```

- [ ] **Step 2.2: Add `close` field to `MenuState`**

Replace the `MenuState` struct in `crates/inputforge-gui-dx/src/components/menu/mod.rs`:

```rust
/// Shared open-state context for menu compound. Both `MenuRoot` and
/// `AnchoredMenu` install one of these so `MenuItem` works under either.
#[derive(Clone, Copy)]
struct MenuState {
    /// Open-state signal. `MenuRoot` owns this directly; `AnchoredMenu`
    /// mirrors its prop into here so `MenuItems`'s hidden+focus logic
    /// keeps reading from one place.
    open: Signal<bool>,
    /// Stable DOM id for the items wrapper.
    menu_id: Signal<String>,
    /// Close dispatcher. `MenuRoot` provides one that flips `open` to
    /// false and discards the reason; `AnchoredMenu` provides one that
    /// fires its `on_close` handler with the reason.
    close: Callback<CloseReason>,
}
```

- [ ] **Step 2.3: Update MenuRoot to provide close**

Replace `MenuRoot` in `crates/inputforge-gui-dx/src/components/menu/mod.rs`:

```rust
#[component]
pub fn MenuRoot(
    /// Class extension for the OUTER wrapper (`.if-menu`). Use for layout-flow
    /// modifiers like `if-menu--block` (which flips the wrapper from
    /// `inline-flex` to `block`).
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let open = use_signal(|| false);
    let menu_id = use_signal(|| {
        format!(
            "if-menu-{}",
            MENU_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
        )
    });
    let close = use_callback(move |_reason: CloseReason| {
        let mut o = open;
        o.set(false);
    });
    let state = MenuState {
        open,
        menu_id,
        close,
    };
    use_context_provider(|| state);

    let combined = merge_class("if-menu", "", class.as_deref());
    rsx! { div { class: "{combined}", {children} } }
}
```

- [ ] **Step 2.4: Update MenuItem to dispatch via close**

Replace `MenuItem` in `crates/inputforge-gui-dx/src/components/menu/mod.rs`:

```rust
#[component]
pub fn MenuItem(
    onclick: Option<EventHandler<MouseEvent>>,
    #[props(default)] disabled: bool,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let state = use_context::<MenuState>();
    let combined = merge_class("if-menu__item", "", class.as_deref());
    let handler = onclick;
    let close = state.close;
    let onclick = move |evt: MouseEvent| {
        if let Some(h) = &handler {
            h.call(evt);
        }
        close.call(CloseReason::ItemActivated);
    };
    rsx! {
        button {
            class: "{combined}",
            role: "menuitem",
            disabled,
            "aria-disabled": "{disabled}",
            onclick,
            {children}
        }
    }
}
```

Both `disabled` (HTML boolean attribute) and `aria-disabled` (ARIA state) are emitted. They serve different a11y roles: HTML `disabled` blocks pointer events and removes the button from sequential focus; `aria-disabled` informs assistive tech of the disabled state without removing it from the accessibility tree. Emitting both is the conventional combination, and the SSR render tests in `pipeline/tests.rs:1078-1086, 1104-1113` assert against the `aria-disabled="true"` substring specifically.

- [ ] **Step 2.5: Re-export `CloseReason`**

In `crates/inputforge-gui-dx/src/components/mod.rs`:

```rust
pub use menu::{Anchor, CloseReason, MenuItem, MenuItems, MenuRoot, MenuTrigger};
```

- [ ] **Step 2.6: Verify**

Run:

```
cargo check -p inputforge-gui-dx
cargo clippy -p inputforge-gui-dx --all-targets -- -D warnings
```

Expected: clean. (Existing consumers don't observe `CloseReason`; behaviour is unchanged.)

- [ ] **Step 2.7: Commit**

```
git add crates/inputforge-gui-dx/src/components/menu/mod.rs \
        crates/inputforge-gui-dx/src/components/mod.rs

git commit -m "refactor(menu): route MenuItem closes through MenuState callback"
```

---

## Task 3: Add AnchoredMenu component

Sibling to `MenuRoot` for cursor-anchored (right-click) menus. Reuses `MenuItem`, `focus_walker`, and the `if-menu__list` surface vocabulary; differs only in positioning (fixed at `(x, y)` instead of trigger-relative) and open-state shape (external `Option<MenuAnchor>` instead of internal `Signal<bool>`).

**Files:**
- Modify: `crates/inputforge-gui-dx/src/components/menu/mod.rs`
- Modify: `crates/inputforge-gui-dx/assets/components/menu.css`
- Modify: `crates/inputforge-gui-dx/src/components/mod.rs`

- [ ] **Step 3.1: Add `MenuAnchor` type**

In `crates/inputforge-gui-dx/src/components/menu/mod.rs`, add above `AnchoredMenu` (which we'll add next):

```rust
/// Anchor coordinates for `AnchoredMenu`. Values are page-space pixels
/// (the same coordinate system as `MouseEvent::page_coordinates`). The
/// menu renders with `position: fixed` at these coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuAnchor {
    pub x: f64,
    pub y: f64,
}
```

- [ ] **Step 3.2: Add `AnchoredMenu` component**

Append to `crates/inputforge-gui-dx/src/components/menu/mod.rs`:

```rust
/// Cursor-anchored menu (right-click style). The parent owns an
/// `Option<MenuAnchor>` signal: `None` = closed, `Some(coords)` = open at
/// those coordinates. `on_close` fires whenever the menu wants to close
/// (Escape, click-outside, Tab, item-activated); the parent decides what
/// to do (typically: clear its anchor signal to None, possibly re-focus
/// the originating element based on the `CloseReason`).
///
/// Inside, render `MenuItem`s as children; they auto-close via the same
/// `MenuState` mechanism `MenuRoot` uses. The wrapper handles backdrop,
/// keyboard navigation (Arrow keys, Home, End), Escape, Tab, and
/// auto-focuses the first non-disabled item on open. Space and Enter
/// activation on items works via native `<button>` semantics (each
/// `MenuItem` is a `<button>`); the keydown handler does not need to
/// handle them explicitly.
///
/// `aria_labelledby` is the DOM id of the element that named this menu
/// (typically the originating right-click target). When `Some`, written
/// to `aria-labelledby`; when `None`, the attribute is omitted entirely
/// (an empty `aria-labelledby` would point to a nonexistent element and
/// is invalid ARIA).
#[component]
#[expect(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures."
)]
pub fn AnchoredMenu(
    /// Anchor coordinates and open-state, fused into one Option so an
    /// open-with-no-coords state is unrepresentable.
    open: Option<MenuAnchor>,
    /// Fires when the menu wants to close, with the reason. The parent
    /// must clear its anchor signal in response; otherwise the menu
    /// stays rendered.
    on_close: EventHandler<CloseReason>,
    /// Optional id of the element that names this menu (written to
    /// `aria-labelledby`). Pass the originating trigger's DOM id.
    #[props(default)]
    aria_labelledby: Option<String>,
    /// Class extension for the inner LIST surface (`.if-menu__list`).
    /// NOTE: this is the visible chrome layer, NOT a wrapper. AnchoredMenu
    /// has no positioned outer wrapper because it applies `position: fixed`
    /// directly on the list. This differs from `MenuItems.class` (which
    /// targets the outer `.if-menu__items` wrapper) and from `MenuRoot.class`
    /// (which targets the outer `.if-menu` wrapper). If you need wrapper-level
    /// styling, lift it to a parent component.
    #[props(default)]
    class: Option<String>,
    children: Element,
) -> Element {
    // Hooks must run in the same order on every render, so allocate them
    // BEFORE the early-return below. Current consumers gate AnchoredMenu
    // mounting at the parent level (the parent early-returns on `None`
    // before mounting AnchoredMenu), but a future consumer that mounts
    // AnchoredMenu unconditionally and toggles `open` between None / Some
    // would otherwise trip a hook-order panic on the first toggle.

    // Allocate a stable menu id for this instance. Using a counter (not
    // anchor coordinates) so the id is consistent across re-renders even
    // if the anchor moves (the menu can be re-positioned without losing
    // its identity for ARIA / focus walking).
    let menu_id = use_signal(|| {
        format!(
            "if-menu-{}",
            MENU_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
        )
    });

    // Mirror open-state into a Signal so MenuItem (which reads
    // MenuState.open via context) sees a live signal. Stays `true` for
    // the lifetime of this AnchoredMenu mount; AnchoredMenu's own
    // visibility is gated by the parent's anchor signal (the
    // `let Some(coords) = open` early return below), not by this Signal.
    let open_signal = use_signal(|| true);
    let close_handler = on_close;
    let close = use_callback(move |reason: CloseReason| {
        close_handler.call(reason);
    });
    let state = MenuState {
        open: open_signal,
        menu_id,
        close,
    };
    use_context_provider(|| state);

    let menu_id_str_for_focus = menu_id.read().clone();
    use_effect(move || {
        if *open_signal.read() {
            focus_menu_item(&menu_id_str_for_focus, FocusAction::First);
        }
    });

    let Some(coords) = open else {
        return rsx! {};
    };

    let menu_id_str = menu_id.read().clone();
    let menu_id_for_keys = menu_id_str.clone();

    let onkeydown = {
        let close = close;
        move |evt: KeyboardEvent| match evt.key() {
            Key::Escape => {
                evt.prevent_default();
                close.call(CloseReason::Escape);
            }
            Key::ArrowDown => {
                evt.prevent_default();
                focus_menu_item(&menu_id_for_keys, FocusAction::Next);
            }
            Key::ArrowUp => {
                evt.prevent_default();
                focus_menu_item(&menu_id_for_keys, FocusAction::Prev);
            }
            Key::Home => {
                evt.prevent_default();
                focus_menu_item(&menu_id_for_keys, FocusAction::First);
            }
            Key::End => {
                evt.prevent_default();
                focus_menu_item(&menu_id_for_keys, FocusAction::Last);
            }
            Key::Tab => {
                // Do NOT prevent_default; let the browser advance focus
                // to the next focusable element. CloseReason::Tab tells
                // the parent to NOT re-focus the trigger so the user's
                // Tab traversal is honoured.
                close.call(CloseReason::Tab);
            }
            _ => {}
        }
    };

    let backdrop_onclick = move |_| {
        close.call(CloseReason::ClickOutside);
    };

    let combined = merge_class("if-menu__list", "", class.as_deref());
    let style = format!(
        "position: fixed; left: {}px; top: {}px; z-index: 1001;",
        coords.x, coords.y
    );

    rsx! {
        // Backdrop sits at z-index 1000, list at 1001. Both fixed so they
        // escape any ancestor stacking context.
        div {
            class: "if-menu__backdrop if-menu__backdrop--anchored",
            "aria-hidden": "true",
            onclick: backdrop_onclick,
        }
        div {
            class: "{combined}",
            id: "{menu_id_str}",
            role: "menu",
            tabindex: "-1",
            "aria-labelledby": aria_labelledby,
            style: "{style}",
            onkeydown,
            {children}
        }
    }
}
```

- [ ] **Step 3.3: Add CSS for the anchored backdrop**

Append to `crates/inputforge-gui-dx/assets/components/menu.css`:

```css
/* Anchored-menu backdrop: must escape any ancestor stacking context, so
   z-index sits above all editor content but below the list (which gets
   1001 inline, see AnchoredMenu render). The trigger-attached variant
   relies on `.if-menu__items` z-index 1000 to lift its own backdrop;
   anchored variants render the backdrop and list as siblings of <body>
   children, so they need their own explicit z-index. */
.if-menu__backdrop--anchored {
    z-index: 1000;
}
```

- [ ] **Step 3.4: Re-export `AnchoredMenu` and `MenuAnchor`**

In `crates/inputforge-gui-dx/src/components/mod.rs`:

```rust
pub use menu::{
    Anchor, AnchoredMenu, CloseReason, MenuAnchor, MenuItem, MenuItems, MenuRoot, MenuTrigger,
};
```

- [ ] **Step 3.5: Verify**

Run:

```
cargo check -p inputforge-gui-dx
cargo clippy -p inputforge-gui-dx --all-targets -- -D warnings
```

Expected: clean. No consumers yet; this commit only adds the API.

- [ ] **Step 3.6: Commit**

```
git add crates/inputforge-gui-dx/src/components/menu/mod.rs \
        crates/inputforge-gui-dx/src/components/mod.rs \
        crates/inputforge-gui-dx/assets/components/menu.css

git commit -m "feat(menu): add AnchoredMenu for cursor-anchored right-click menus"
```

---

## Task 4: Migrate AddPalette to MenuRoot

Fixes the originally-reported bug (menu opens flush-left, no click-away). Deletes the local `open` signal and the bespoke menu div; gains backdrop, ESC, arrow-key navigation, ARIA for free.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/add_palette.rs`
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_editor.css`

- [ ] **Step 4.1: Update imports in add_palette.rs**

In `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/add_palette.rs`, change the imports block. Add:

```rust
use crate::components::{Anchor, MenuItem, MenuItems, MenuRoot, MenuTrigger};
```

(Keep all existing imports; this is additional.)

- [ ] **Step 4.2: Rewrite the AddPalette body**

In `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/add_palette.rs`, replace the function body of `AddPalette` (everything from `let ctx = use_context::<AppContext>();` to the end of the function) with:

```rust
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();

    let path_prefix_clone = path_prefix.clone();
    let mapping_key_clone = mapping_key.clone();
    let root_actions_clone = root_actions.clone();

    // Shared do_insert closure factory. Returns a MouseEvent handler that
    // inserts `action` at the target position. Menu auto-closes via
    // MenuItem; this closure no longer touches an open signal.
    let make_insert_handler = move |action: Action| {
        let key = mapping_key_clone.clone();
        let prefix = path_prefix_clone.clone();
        let root = root_actions_clone.clone();
        let cmd_tx = ctx.commands.clone();
        let cfg_sig = ctx.config;
        let mut undo_log = editor.undo_log;
        let mut expanded = editor.expanded_stages;
        let mut malformed = editor.malformed_hints;
        let insert_len = target_len;

        move |_: MouseEvent| {
            let mut path_segs = prefix.clone();
            path_segs.push(StageIdSegment::Index(insert_len));
            let insert_path = StageId(path_segs);

            let Some(new_actions) = insert_at_path(&root, &insert_path, action.clone()) else {
                return;
            };

            let cfg = cfg_sig.read();
            let current_name = cfg.mapping_names.get(&key.1).cloned();
            drop(cfg);

            let before = Mapping {
                input: key.1.clone(),
                mode: key.0.clone(),
                name: current_name.clone(),
                actions: root.clone(),
            };

            let stage_title = action_palette_label(&action);

            if cmd_tx
                .send(EngineCommand::SetMapping {
                    input: key.1.clone(),
                    mode: key.0.clone(),
                    name: current_name,
                    actions: new_actions,
                })
                .is_err()
            {
                tracing::warn!(
                    target: "f9::mapping_editor",
                    action = "add_palette_drop_offline",
                    "stage add dropped: engine channel disconnected"
                );
                return;
            }

            let label = format_undo_label(
                UndoKind::StageAdd,
                LabelArgs {
                    stage_name: Some(stage_title),
                    index: Some(insert_len),
                    ..LabelArgs::default()
                },
            );
            undo_log
                .write()
                .push_edit(key.clone(), before, UndoKind::StageAdd, label);

            let parent_path = insert_path.0[..insert_path.0.len() - 1].to_vec();
            let insert_idx = insert_len;
            expanded
                .write()
                .retain(|p| !path_invalidated_by_mutation(p, &parent_path, insert_idx));
            malformed
                .write()
                .retain(|p, _| !path_invalidated_by_mutation(p, &parent_path, insert_idx));
            expanded.write().insert(insert_path);
        }
    };

    let trigger_class = if louder {
        "if-add-palette__trigger if-add-palette__trigger--louder"
    } else {
        "if-add-palette__trigger"
    };

    let compact_aria_label = if louder {
        None
    } else {
        // Icon-only trigger: must carry an accessible name. WCAG 2.1 SC 4.1.2.
        Some("Add stage".to_owned())
    };

    rsx! {
        MenuRoot { class: "if-add-palette if-menu--block".to_owned(),
            MenuTrigger {
                class: trigger_class.to_owned(),
                unstyled: true,
                aria_label: compact_aria_label,
                if louder {
                    Icon { name: IconKind::Plus, size: IconSize::Sm }
                    "Add first stage"
                } else {
                    Icon { name: IconKind::Plus, size: IconSize::Sm }
                }
            }
            MenuItems { class: "if-add-palette__menu".to_owned(), anchor: Anchor::Center,
                div { class: "if-add-palette__section is-processing",
                    div { class: "if-add-palette__section-title", "Processing" }
                    for item in PROCESSING_ITEMS {
                        MenuItem {
                            class: "if-add-palette__item".to_owned(),
                            onclick: make_insert_handler((item.make)()),
                            "{item.label}"
                        }
                    }
                }
                div { class: "if-add-palette__section is-output",
                    div { class: "if-add-palette__section-title", "Output" }
                    for item in OUTPUT_ITEMS {
                        MenuItem {
                            class: "if-add-palette__item".to_owned(),
                            onclick: make_insert_handler((item.make)()),
                            "{item.label}"
                        }
                    }
                }
                div { class: "if-add-palette__section is-control",
                    div { class: "if-add-palette__section-title", "Control" }
                    for item in CONTROL_ITEMS {
                        MenuItem {
                            class: "if-add-palette__item".to_owned(),
                            onclick: make_insert_handler((item.make)()),
                            "{item.label}"
                        }
                    }
                }
            }
        }
    }
}
```

The icon-only compact trigger (the `else` arm of `if louder`) keeps its `aria-label="Add stage"` via the new `MenuTrigger.aria_label` prop added in Step 1.2. The louder branch's trigger has visible "Add first stage" text and needs no `aria-label`, so `compact_aria_label` is `None` there and the attribute is omitted entirely.

- [ ] **Step 4.3: Update CSS in mapping_editor.css**

In `crates/inputforge-gui-dx/assets/frame/mapping_editor.css`, replace the entire add-palette section (lines around 692-819, from `/* Task 28: categorized add palette */` through the last `.if-add-palette__item:hover` rule) with:

```css
/* Task 28: categorized add palette. Now built on the shared MenuRoot
   primitive (components/menu); local rules style only the trigger
   surface (dashed-violet next-slot treatment) and the section / item
   chrome inside the menu list. Open-state, backdrop, ESC, arrow-key
   navigation, ARIA, and centered-anchor positioning come from the
   primitive. */

/* The MenuRoot wrapper carries `if-add-palette if-menu--block`; the
   if-menu--block modifier (defined in components/menu.css) flips
   .if-menu from inline-flex to block so the trigger row fills its
   parent column. No additional positioning rule is needed here. */

/* End-of-pipeline trigger: full-width "next slot" row. Same dashed-
   violet family as the louder empty-state placeholder. unstyled=true
   on MenuTrigger means we own the surface entirely; the .if-menu__trigger
   defaults are skipped. */
.if-add-palette__trigger {
    width: 100%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-height: 28px;
    background: rgba(154, 120, 214, 0.03);
    border: 1px dashed rgba(184, 155, 234, 0.20);
    border-radius: 6px;
    padding: 6px 12px;
    color: var(--color-control-badge-text);
    font-family: var(--font-sans);
    font-size: 12px;
    font-weight: 500;
    line-height: 1;
    cursor: pointer;
    transition:
        background var(--duration-fast) var(--easing-fast),
        border-color var(--duration-fast) var(--easing-fast);
}

.if-add-palette__trigger:hover {
    background: rgba(154, 120, 214, 0.06);
    border-color: rgba(184, 155, 234, 0.40);
}

.if-add-palette__trigger:focus-visible {
    outline: 2px solid var(--color-border-focus);
    outline-offset: 2px;
}

.if-add-palette__trigger--louder {
    background: rgba(154, 120, 214, 0.06);
    border-color: rgba(184, 155, 234, 0.40);
    padding: 10px 12px;
    line-height: 1.45;
}

.if-add-palette__trigger--louder:hover {
    background: rgba(154, 120, 214, 0.10);
    border-color: rgba(184, 155, 234, 0.60);
}

/* Menu list: extends .if-menu__list with palette-specific min-width.
   `MenuItems.class` lands on `.if-menu__items` (the absolutely-positioned
   outer wrapper), but the visible chrome (background, border, radius,
   shadow, baseline `min-width: 160px`) lives on the inner `.if-menu__list`.
   Use a descendant selector to target the actual surface. */
.if-add-palette__menu .if-menu__list {
    min-width: 180px;
}

.if-add-palette__section {
    padding: 4px;
}

.if-add-palette__section-title {
    font-family: var(--font-mono);
    font-size: 11px;
    text-transform: uppercase;
    font-weight: 500;
    color: var(--color-text-subtle);
    padding: 2px 8px;
}

.if-add-palette__section.is-processing {
    background: var(--color-stage-tint-processing);
}

.if-add-palette__section.is-output {
    background: var(--color-stage-tint-output);
}

.if-add-palette__section.is-control {
    background: var(--color-stage-tint-control);
}

.if-add-palette__section.is-processing .if-add-palette__section-title {
    color: var(--color-processing);
}

.if-add-palette__section.is-output .if-add-palette__section-title {
    color: var(--color-output);
}

.if-add-palette__section.is-control .if-add-palette__section-title {
    color: var(--color-control-badge-text);
}

/* Per-item modifier: tightens MenuItem's default padding to keep the
   palette dense. The base .if-menu__item supplies hover bg, focus ring,
   and disabled state. */
.if-add-palette__item {
    padding: 4px 8px !important;
    font-size: 12px;
}
```

The `!important` on padding overrides `.if-menu__item`'s `var(--space-2) var(--space-3)`. Acceptable here because consumer-specific density tuning is the documented use case for `MenuItem.class`.

- [ ] **Step 4.4: Verify compile**

Run:

```
cargo check -p inputforge-gui-dx
cargo clippy -p inputforge-gui-dx --all-targets -- -D warnings
```

Expected: clean.

- [ ] **Step 4.5: Run mapping_editor tests**

Run:

```
cargo test -p inputforge-gui-dx --lib frame::mapping_editor
```

Expected: all pass. Existing tests are logic-only and don't touch the AddPalette render path.

- [ ] **Step 4.6: Manual GUI verification**

Launch:

```
dx run -p inputforge-app --no-default-features --features gui-dioxus
```

In a debug build on Windows, the WebView exposes CDP on `127.0.0.1:9222`. Use `chrome-devtools` MCP to:

1. Navigate to a mapping editor with at least one input selected.
2. Click the `+` button in the pipeline. Expected: menu opens **centered** under the trigger, not flush-left.
3. Click anywhere outside the menu. Expected: menu closes.
4. Re-open, press Escape. Expected: menu closes.
5. Re-open, press ArrowDown. Expected: focus moves to the first item (Invert), then walks through Processing → Output → Control.
6. Click an item (e.g. Deadzone). Expected: stage is added, menu closes.
7. On an empty pipeline, repeat with the louder `+ Add first stage` trigger. Expected: same behaviours.

If any step fails, debug before committing.

- [ ] **Step 4.7: Commit**

```
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/add_palette.rs \
        crates/inputforge-gui-dx/assets/frame/mapping_editor.css

git commit -m "refactor(add-palette): adopt MenuRoot primitive for backdrop and centered anchor"
```

---

## Task 5: Migrate StageActionsMenu to AnchoredMenu

Replaces the hand-rolled backdrop + ul + li + button structure with `AnchoredMenu` + `MenuItem`. Preserves all swap/delete logic, undo entries, and disabled-state semantics.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_actions_menu.rs`
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_editor.css`

- [ ] **Step 5.1: Update imports**

In `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_actions_menu.rs`, replace the imports block above the `// Private path helpers` comment with:

```rust
use dioxus::prelude::*;

use inputforge_core::action::{Action, Mapping};
use inputforge_core::engine::EngineCommand;

use crate::components::{AnchoredMenu, CloseReason, MenuAnchor, MenuItem};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::pipeline::stage::stage_title_for;
use crate::frame::mapping_editor::pipeline::{
    path_invalidated_by_mutation, remove_at_path, replace_at_path,
};
use crate::frame::mapping_editor::undo_log::{
    LabelArgs, StageId, StageIdSegment, UndoKind, format_undo_label,
};
use crate::frame::mapping_editor::{EditorState, StageMenuState};
```

- [ ] **Step 5.2: Replace the render block**

In `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_actions_menu.rs`, find the section starting at the `// Close-on-backdrop handler` comment (around line 449) and replace everything from there to the end of the function (the closing `}` of `pub(crate) fn StageActionsMenu`) with:

```rust
    // ---------------------------------------------------------------------------
    // on_close handler
    // ---------------------------------------------------------------------------

    let on_close = {
        let mut stage_menu = editor.stage_menu;
        move |_reason: CloseReason| {
            stage_menu.set(None);
        }
    };

    // ---------------------------------------------------------------------------
    // Render
    // ---------------------------------------------------------------------------

    let anchor = MenuAnchor {
        x: menu.x,
        y: menu.y,
    };

    rsx! {
        AnchoredMenu {
            open: Some(anchor),
            on_close,
            class: "if-stage-menu".to_owned(),
            MenuItem {
                disabled: move_up_disabled,
                onclick: on_move_up,
                "Move up"
            }
            MenuItem {
                disabled: move_down_disabled,
                onclick: on_move_down,
                "Move down"
            }
            MenuItem {
                class: "if-stage-menu__item--danger".to_owned(),
                onclick: on_delete,
                "Delete"
            }
        }
    }
}
```

- [ ] **Step 5.3: Strip stage_menu.set(None) from action handlers**

The three handlers (`on_move_up`, `on_move_down`, `on_delete`) currently each call `stage_menu.set(None)` at the end of their bodies. With `MenuItem` auto-closing via `state.close.call(CloseReason::ItemActivated)` and `on_close` clearing `editor.stage_menu`, the explicit `stage_menu.set(None)` calls are redundant but harmless. Remove them for clarity.

In `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_actions_menu.rs`, find each of these three lines:

```rust
                stage_menu.set(None);
                return;
```

(in the `if cmd_tx.send(...).is_err()` blocks of `on_move_up`, `on_move_down`, `on_delete`)

and replace each with just:

```rust
                return;
```

Also find each trailing `stage_menu.set(None);` at the end of the three handlers' success paths and remove those lines.

Also remove the `let mut stage_menu = editor.stage_menu;` line from inside each of the three handlers (`on_move_up`, `on_move_down`, `on_delete`), since `stage_menu` is no longer captured by them. The shared close path (`on_close` above) now owns the clearing.

Verify by searching: after the edits, `rg "stage_menu" crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_actions_menu.rs` should show only the imports, the `let menu_state` read at the top, the `let mut stage_menu` inside `on_close`, and the malformed-path early-return.

- [ ] **Step 5.4: Strip the malformed-path stage_menu clear**

In `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_actions_menu.rs`, find the malformed-path early return (around the top of the function, where `split_stage_path` returns None):

```rust
    let Some((parent_path, current_idx)) = split_stage_path(&stage_id) else {
        // Malformed path: close and bail.
        let mut stage_menu = editor.stage_menu;
        stage_menu.set(None);
        return rsx! {};
    };
```

Replace with:

```rust
    let Some((parent_path, current_idx)) = split_stage_path(&stage_id) else {
        // Malformed path: bail; render nothing. The parent's stage_menu
        // signal stays Some until the user clicks outside, which will
        // route through the AnchoredMenu's backdrop on the NEXT render.
        // In practice this is unreachable (split_stage_path only fails
        // on empty / non-Index paths, which the caller never produces).
        tracing::warn!(
            target: "f9::mapping_editor",
            action = "stage_menu_malformed_path",
            "stage menu opened with malformed StageId"
        );
        return rsx! {};
    };
```

- [ ] **Step 5.5: Add CSS modifier for the danger item**

In `crates/inputforge-gui-dx/assets/frame/mapping_editor.css`, find the existing `/* Task 29: right-click stage actions menu */` block (around line 821) and replace from there through the end of the `.if-stage-menu` related rules with:

```css
/* Task 29: right-click stage actions menu. Built on AnchoredMenu
   (components/menu); local rules style the danger item only. The
   menu's surface, backdrop, positioning, and keyboard handling all
   come from the shared primitive. The `class: "if-stage-menu"` on
   the AnchoredMenu invocation is a no-op pass-through (no rule
   matches it today); kept on the Rust side so future palette tuning
   has a hook to land on. */

.if-stage-menu__item--danger {
    /* Hex fallback protects against silent invisible-text in any
       partially-loaded theme where --color-error is undefined. */
    color: var(--color-error, #f25555);
}
```

Find the rules below `.if-stage-menu__backdrop` and `.if-stage-menu__item` (the original Task 29 styles, lines roughly 826-862 — confirm by reading the file). Delete them all up to but not including the next unrelated section. The `.if-stage-menu` and `.if-stage-menu__backdrop` selectors should no longer appear in the file. Confirm with `rg "if-stage-menu" crates/inputforge-gui-dx/assets/frame/mapping_editor.css`.

- [ ] **Step 5.6: Verify**

Run:

```
cargo check -p inputforge-gui-dx
cargo clippy -p inputforge-gui-dx --all-targets -- -D warnings
cargo test -p inputforge-gui-dx --lib frame::mapping_editor
```

Expected: all clean. The unit tests in `stage_actions_menu.rs` (`tests` module at the bottom) only exercise `split_stage_path` and `make_stage_id`, so they pass unchanged.

The SSR render tests in `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/tests.rs:1025-1114` (`right_click_on_stage_opens_actions_menu`, `stage_menu_disables_move_up_at_first_position`, `stage_menu_disables_move_down_at_last_position`) DO exercise the migrated `StageActionsMenu` end-to-end via SSR. They assert:

- `"Move up"`, `"Move down"`, `"Delete"` substrings (still present as MenuItem children).
- `"left: 100px"` and `"top: 200px"` substrings (satisfied by AnchoredMenu's inline `style: "position: fixed; left: ...px; top: ...px; z-index: 1001;"`).
- `aria-disabled="true"` substring on the disabled Move-up / Move-down button (satisfied by the `aria-disabled` attribute added to `MenuItem` in Step 2.4).

All three tests must pass after this task. If `aria-disabled` assertions fail, re-check that Step 2.4 emits both `disabled` and `"aria-disabled": "{disabled}"`.

- [ ] **Step 5.7: Manual GUI verification**

Launch the GUI and:

1. Right-click a stage. Expected: menu opens at cursor position.
2. Click outside. Expected: menu closes.
3. Re-open, press Escape. Expected: menu closes.
4. Re-open, press ArrowDown / ArrowUp. Expected: focus walks through Move up / Move down / Delete (skipping disabled items).
5. With a single-stage pipeline, right-click that stage. Expected: Move up and Move down are disabled (visually muted). Click them. Expected: nothing happens.
6. Click Delete. Expected: stage is removed, menu closes, undo log gains a "Delete <stage>" entry.
7. Right-click on the first stage of a multi-stage pipeline, click Move down. Expected: stage swaps with the next, menu closes, undo log gains a "Reorder" entry.

- [ ] **Step 5.8: Commit**

```
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_actions_menu.rs \
        crates/inputforge-gui-dx/assets/frame/mapping_editor.css

git commit -m "refactor(stage-actions): adopt AnchoredMenu primitive for backdrop and a11y"
```

---

## Task 6: Migrate ModeTabContextMenu to AnchoredMenu

Same migration shape as Task 5, but the consumer's `open` signal carries a `(String, AnchorRect)` payload. `AnchorRect.left/bottom` map to `MenuAnchor.x/y`. The local `CloseReason` enum gets dropped in favour of the shared one.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/context_menu.rs`
- Modify: `crates/inputforge-gui-dx/assets/frame/top_bar.css`

- [ ] **Step 6.1: Drop the local `CloseReason` and update imports**

In `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/context_menu.rs`, delete the local `CloseReason` enum (lines around 35-40):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CloseReason {
    Escape,
    ClickOutside,
    Tab,
    ItemActivated,
}
```

Replace with a re-export at the top of the file (after the other `pub(crate) use` lines or the imports):

```rust
pub(crate) use crate::components::CloseReason;
```

Update the imports block at the top of the file. Replace the existing menu-related imports with:

```rust
use crate::components::{AnchoredMenu, MenuAnchor, MenuItem};
```

(Keep `use inputforge_core::engine::EngineCommand;` and `use crate::context::AppContext;`.)

Drop the `use crate::components::menu::{FocusAction, focus_menu_item};` line — `AnchoredMenu` handles focus walking internally.

- [ ] **Step 6.2: Replace the render block**

In `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/context_menu.rs`, find the `rsx! {` block at the bottom of `ModeTabContextMenu` (around line 233) and replace everything from there to the function's closing `}` with:

```rust
    let anchor = MenuAnchor {
        x: anchor.left,
        y: anchor.bottom,
    };

    let labelled_by_owned = labelled_by.clone();

    rsx! {
        AnchoredMenu {
            open: Some(anchor),
            on_close: move |reason: CloseReason| {
                let mut open = open;
                open.set(None);
                on_close.call((tab_name.clone(), reason));
            },
            aria_labelledby: labelled_by_owned,
            class: "if-modetab-context-menu".to_owned(),
            MenuItem {
                disabled: flags.activate_disabled,
                onclick: activate_onclick,
                "Activate"
            }
            MenuItem {
                disabled: flags.rename_disabled,
                onclick: rename_onclick,
                "Rename"
            }
            MenuItem {
                disabled: flags.delete_disabled,
                onclick: delete_onclick,
                "Delete"
            }
            MenuItem {
                disabled: flags.set_default_disabled,
                onclick: default_onclick,
                "Set as default"
            }
        }
    }
}
```

The `activate_onmounted` (auto-focus dance) is no longer needed — `AnchoredMenu` auto-focuses the first item on open. Drop the `let activate_onmounted = ...` block above.

- [ ] **Step 6.3: Simplify the click handlers**

The four onclick handlers (`activate_onclick`, `rename_onclick`, `delete_onclick`, `default_onclick`) currently each:
1. Bail if disabled (`if flags.activate_disabled { return; }`)
2. Run the action (send EngineCommand or call event handler)
3. Set `open` to None
4. Call `on_close` with `CloseReason::ItemActivated`

`MenuItem` auto-closes via `AnchoredMenu`'s `on_close`, which now handles steps 3 and 4. The disabled bail in step 1 is also redundant — `MenuItem.disabled` blocks the click at the `<button disabled>` level. Remove both from each handler.

In `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/context_menu.rs`, replace each of the four handlers. For example, `activate_onclick`:

```rust
    let activate_onclick = {
        move |_| {
            let _ = cmd_activate.send(EngineCommand::ForceMode {
                mode: activate_name.clone(),
            });
        }
    };
```

(Drop the `let mut open = open;`, `let on_close = on_close;`, the `if flags.activate_disabled { return; }`, `open.set(None);`, and `on_close.call(...)` lines.)

Repeat for `rename_onclick`, `delete_onclick`, `default_onclick`. Each becomes a 1-3 line action call only.

Drop the now-unused `close_name_for_*` String clones and the `let cmd_activate = ctx.commands.clone();` / `let cmd_default = ctx.commands.clone();` clones IF they're now unused — keep them if the action handlers still need them.

After: `rg "open\.set" crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/context_menu.rs` should show only the call inside the `on_close` closure.

- [ ] **Step 6.4: Drop the menu_id and menu_onkeydown plumbing**

`AnchoredMenu` allocates its own menu id and owns the keydown handlers. In `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/context_menu.rs`, delete:

- `let menu_id = format!("mode-tab-menu-{tab_idx}");`
- `let menu_id_for_keys = menu_id.clone();`
- The entire `let menu_onkeydown = { ... };` block.

Keep `let labelled_by = format!("mode-tab-{tab_idx}");` — it's still needed for the `aria_labelledby` prop.

After: searching the file for `menu_id` or `menu_onkeydown` (use the `Grep` tool with pattern `menu_id|menu_onkeydown` — ripgrep regex alternation, NOT bash-escaped `\|`) should return nothing.

- [ ] **Step 6.5: Strip duplicate CSS in top_bar.css**

In `crates/inputforge-gui-dx/assets/frame/top_bar.css`, find the comment block around line 444-450 (`The menu backdrop is a pointer-event sink...`) and the rule below it (`.if-menu__backdrop { ... z-index: 100; }`). Delete the comment AND the rule — the consolidated `.if-menu__backdrop--anchored` in `assets/components/menu.css` (z-index 1000) replaces it.

Also find the `.if-modetab-context-menu` rule block and its descendants (`.if-modetab-context-menu li`, `.if-modetab-context-menu button`, `.if-modetab-context-menu button:hover...`, `.if-modetab-context-menu button[aria-disabled="true"]`). The list-surface chrome (background, border, shadow, min-width) is now provided by `.if-menu__list`. The button styling is now provided by `.if-menu__item`. Replace the entire `.if-modetab-context-menu`-prefixed rule block with:

```css
/* Mode-tab context menu, AnchoredMenu consumer. Surface, items, and
   backdrop come from the shared primitive (.if-menu__list,
   .if-menu__item, .if-menu__backdrop--anchored). The 12rem min-width
   here documents the structural constraint that the longest action
   label ("Set as default") fits without per-locale recalculation. */
.if-modetab-context-menu {
    min-width: 12rem;
}
```

Confirm with `rg "if-modetab-context-menu\|if-menu__backdrop" crates/inputforge-gui-dx/assets/frame/top_bar.css` — only the consolidated min-width rule should remain.

- [ ] **Step 6.6: Verify**

Run:

```
cargo check -p inputforge-gui-dx
cargo clippy -p inputforge-gui-dx --all-targets -- -D warnings
cargo test -p inputforge-gui-dx --lib frame::top_bar
```

Expected: clean. The unit tests at the bottom of `context_menu.rs` exercise `flags_for` (a pure-logic helper) and pass unchanged.

- [ ] **Step 6.7: Manual GUI verification**

Launch the GUI and:

1. Right-click a mode tab. Expected: menu opens at cursor.
2. Click outside. Expected: menu closes; the originating tab gets focus back (because `CloseReason::ClickOutside` triggers re-focus in the parent's `on_close`).
3. Re-open, press Escape. Expected: menu closes; tab re-focused.
4. Re-open, press Tab. Expected: menu closes via `CloseReason::Tab`. NOTE: Dioxus signal updates are async, so the `<div role="menu">` may still be in the DOM at the moment the browser advances focus. If Tab focus appears to land inside the menu first (on the next non-disabled `MenuItem`) before the next render unmounts the menu, this is a pre-existing behavior shared with the unmigrated code at `mode_tabs/context_menu.rs:156-165`. Track as a follow-up if the user-visible result is wrong; do not block the migration on it.
5. Re-open on a non-startup tab, click "Set as default". Expected: command fires, menu closes.
6. Re-open on the startup tab. Expected: "Set as default" is disabled (visually muted, can't be clicked).
7. Re-open with keyboard via Shift+F10 if that path exists. Expected: same behaviours.

- [ ] **Step 6.8: Commit**

```
git add crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/context_menu.rs \
        crates/inputforge-gui-dx/assets/frame/top_bar.css

git commit -m "refactor(mode-tabs): adopt AnchoredMenu primitive, drop duplicate menu CSS"
```

---

## Task 7: Final verification + cleanup sweep

Confirm nothing regressed across the full workspace and that no orphaned CSS or dead code lingers.

**Files:**
- Read-only verification across the crate.

- [ ] **Step 7.1: Workspace test sweep**

Run:

```
cargo test --workspace
```

Expected: all tests pass. Note any new warnings.

- [ ] **Step 7.2: Workspace clippy sweep**

Run:

```
cargo clippy --workspace --all-targets -- -D warnings
```

Expected: clean.

- [ ] **Step 7.3: Hunt for orphaned CSS**

Run from repo root:

```
rg "if-stage-menu" crates/inputforge-gui-dx
rg "if-add-palette__menu" crates/inputforge-gui-dx
rg "if-modetab-context-menu" crates/inputforge-gui-dx
```

Expected: each shows ONLY references in CSS (the surviving consolidated rules) plus possibly the `.if-stage-menu__item--danger` modifier from Task 5. No Rust source should reference the old class names except via the new `MenuItem.class` extension hook.

If any orphan rules are found, delete them in this commit.

- [ ] **Step 7.4: Hunt for orphaned Rust**

Use the `Grep` tool (project's mandated content-search interface per `CLAUDE.md`; do NOT shell out to raw `rg` in PowerShell since the project's regex-syntax hygiene assumes ripgrep alternation, not bash-escaped `\|`):

- Pattern: `if-menu__backdrop|focus_walker::`
- Path: `crates/inputforge-gui-dx/src`

Expected: `focus_walker::` references appear only in `components/menu/mod.rs` (the only file that should still use it directly). If any frame/* source still references `focus_walker`, that consumer wasn't fully migrated — investigate.

- [ ] **Step 7.5: Manual GUI sweep across all four menus**

Launch the GUI:

```
dx run -p inputforge-app --no-default-features --features gui-dioxus
```

Verify in this order:

1. **Component gallery**: verify with `cargo check --example component_gallery -p inputforge-gui-dx` (compile-only). The gallery's `MenuRoot` consumer has no runtime behavior the migration alters beyond what Tasks 1 and 2 already validate; a clean compile is sufficient. Skip launching the gallery binary.
2. **AddPalette (`+` button)**: trigger the menu, verify centered position, click-outside closes, ESC closes, ArrowDown navigates, item-click adds a stage.
3. **StageActionsMenu (right-click stage)**: cursor anchor, click-outside closes, ESC closes, ArrowDown navigates, disabled items are skipped, all three actions execute.
4. **ModeTabContextMenu (right-click mode tab)**: cursor anchor, click-outside re-focuses tab, ESC re-focuses tab, Tab does NOT re-focus tab, disabled items are skipped, all four actions execute.

If any menu misbehaves, record the failure mode and amend the relevant task before declaring done.

- [ ] **Step 7.6: Commit (if cleanup was needed in step 7.3 or 7.4)**

If steps 7.3 / 7.4 surfaced orphans:

```
git add <affected files>
git commit -m "chore(menu): remove orphaned CSS and dead imports after consolidation"
```

If nothing was orphaned, no commit is needed; declare done.

---

## Self-review checklist (after writing the plan)

- [x] Spec coverage: each of the user's three asks (`unstyled` MenuTrigger, `anchor` MenuItems, AnchoredMenu for the two right-click consumers) maps to a task or sub-step. Plus a fourth ask (migrate AddPalette) mapped to Task 4. The `aria_label` prop on `MenuTrigger` (Step 1.2) and the `aria-disabled` attribute on `MenuItem` (Step 2.4) are supporting amendments added during plan review to preserve a11y on the AddPalette icon-only trigger and to keep the existing SSR render tests in `pipeline/tests.rs:1078-1113` green.
- [x] No placeholders: every step shows the actual code, file, and command. The one explicit "amendment if reviewer requests" note in Step 4.2 (the dropped `aria-label`) is flagged as a future amendment, not a placeholder.
- [x] Type consistency: `MenuState`, `MenuAnchor`, `CloseReason`, `Anchor`, `AnchoredMenu`, `MenuTrigger.unstyled`, `MenuItems.anchor` are referenced consistently across tasks.
- [x] Existing tests preserved: no test file is rewritten, only consumers; logic-only tests in `stage_actions_menu.rs` and `context_menu.rs` pass unchanged.
- [x] Each commit leaves the workspace compiling and tests green (Task 1 add-only, Task 2 internal refactor, Task 3 add-only, Tasks 4-6 are atomic per-consumer migrations).
- [x] Conventional Commits per `CLAUDE.md`: `feat(menu)`, `refactor(menu)`, `refactor(add-palette)`, `refactor(stage-actions)`, `refactor(mode-tabs)`, `chore(menu)`. All include scope.
- [x] `ms-rust` is mandatory before any `.rs` write per the user's global instructions; the executing agent will invoke it before each Rust edit.
