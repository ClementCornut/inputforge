# F7 Tabs primitive audit

## Inventory

### F2 `Tabs` primitive — current API surface

File: `crates/inputforge-gui-dx/src/components/tabs.rs` (146 LOC).

`TabItem` struct (3 fields):

```rust
pub struct TabItem {
    pub id: String,
    pub label: String,
    pub controls: Option<String>, // aria-controls panel id
}
```

`Tabs` component props (5 inputs):

| Prop       | Type                  | Notes                               |
| ---------- | --------------------- | ----------------------------------- |
| `value`    | `String`              | id of active tab (caller-owned)     |
| `onchange` | `EventHandler<String>`| fires on click + arrow/Home/End     |
| `items`    | `Vec<TabItem>`        | display order                       |
| `class`    | `Option<String>`      | wrapper class merge                 |
| `disabled` | `bool`                | short-circuits keyboard + click     |

Internal state: one `Signal<Vec<Option<Rc<MountedData>>>>` of per-tab refs. Populated in each button's `onmounted`; consumed in `onkeydown` to call `MountedData::set_focus(true).await` so the focus ring follows the active tab (WAI-ARIA APG automatic-activation pattern).

Keydown match arms (lines 83–97 of `tabs.rs`):

| Key            | Behavior                              |
| -------------- | ------------------------------------- |
| `ArrowRight`   | `(idx + 1) % len`                     |
| `ArrowLeft`    | `(idx + len - 1) % len`               |
| `Home`         | `0`                                   |
| `End`          | `len - 1`                             |
| `" "` / `Enter`| `prevent_default` (no activation)     |

Per-tab render: `<button role="tab">` with `id=tab-{id}`, `aria-selected`, `aria-controls`, `tabindex` (roving 0/-1), label as a single `{label}` text node — there is no decoration slot, no contextmenu hook, no inline-edit swap, no tail button. Confirmed.

### Call sites in the workspace

Source-code call sites today (excluding plan/spec markdown):

1. `crates/inputforge-gui-dx/examples/component_gallery.rs:546` — three-tab demo.
2. `crates/inputforge-gui-dx/examples/component_gallery.rs:591` — disabled-tabs demo.
3. `crates/inputforge-gui-dx/src/shell/placeholder.rs:24` — `PlaceholderShell` Mappings/Modes split.

Site #3 is **deleted by F7 Task 32** (the placeholder shell goes away when the F7 frame replaces it). Effective post-F7 call sites: **2, both inside `component_gallery.rs`**.

There are no production-code call sites today and there will not be any after F7 lands.

### F7 mode-tab needs vs. what the primitive provides

| Need (F7 Tasks 28–31)                                          | Provided by F2 Tabs?                |
| -------------------------------------------------------------- | ----------------------------------- |
| `role="tablist"` + per-tab `role="tab"` + `aria-selected`      | yes                                 |
| `aria-orientation="horizontal"`                                | yes                                 |
| `aria-label` on tablist (custom, "Editing mode")               | **no** — currently no prop          |
| Roving `tabindex` (0 active, -1 inactive)                      | yes                                 |
| Arrow / Home / End navigation with focus-follow                | yes                                 |
| Per-tab decoration slot (runtime-marker dot + sr-only sibling) | **no**                              |
| Per-tab `oncontextmenu` (right-click → menu)                   | **no**                              |
| Shift+F10 keyboard-equivalent for context menu                 | **no** — keydown match arms missing |
| `Delete` key on a tab → open F4 confirm                        | **no**                              |
| `+` tail button (not a real tab; triggers add-mode flow)       | **no**                              |
| Inline-rename swap (replace tab button with `TextInput`)       | **no**                              |
| Arrow-roving must skip a renaming index                        | **no** — primitive has no concept   |

Seven of twelve needs are not covered.

## Path A: Extend F2 `Tabs`

- **Cost: ~85 LOC across 3 files** (`tabs.rs` + 2 gallery call sites in `component_gallery.rs`).

Breakdown of the primitive delta (`tabs.rs`):

- `aria_label: Option<String>` prop + attribute on the wrapper — ~3 LOC.
- `tab_decoration: Option<RenderFn<&TabItem>>` (or callback returning `Element`) for the marker dot — ~12 LOC (prop, render-prop wiring inside the `button` body).
- `oncontextmenu: Option<EventHandler<(usize, MouseEvent)>>` on each tab — ~8 LOC.
- `oncontextmenu_kbd: Option<EventHandler<usize>>` for Shift+F10 + new keydown arm — ~6 LOC.
- `ondelete: Option<EventHandler<usize>>` + new keydown arm for `Delete` — ~6 LOC.
- `tail_button: Option<RenderFn<()>>` rendered after the per-tab loop — ~10 LOC.
- `editing_index: Option<Signal<Option<usize>>>` + `inline_editor: Option<RenderFn<&TabItem>>` for rename swap — ~20 LOC (two props, conditional render swap, plus skip-while-renaming logic in the keydown walk).
- Skip-renaming bounded walk replacing the simple `next_idx` calculation — ~10 LOC.

Subtotal in `tabs.rs`: **~75 LOC of additions** (the file roughly doubles from 146 to ~221 LOC).

Call-site updates: each existing `Tabs { ... }` invocation gains 7 new optional props that default to `None` / no-op — no source changes are strictly required (Dioxus `#[props(default)]` covers it), but discoverability suffers and any prop-spread / explicit `None` style adds ~3 LOC × 2 sites = **~6 LOC**.

Plus: F7's mode tab does not have a stable `id` / `controls` panel id (the editing surface tabpanel is owned by F11/F13 and may not be mounted yet), so either `TabItem` adopts everything-optional, or `mode_tabs` builds throwaway `TabItem`s purely to feed the primitive.

**Pros:**
- Single source of truth for ARIA tablist + roving + focus-walker.
- Future tablist features (F11/F13 left-rail mode tree, any plugin tabs, etc.) inherit the upgrades for free.
- Tested once, used N times — keydown logic does not get re-verified per consumer.

**Cons:**
- Prop bloat for one consumer: 5 → 12 props, four of them render-props or signal-coupled. Two call sites pay no benefit and must read past the new noise.
- Render-prop ergonomics in Dioxus 0.7 are awkward — `RenderFn<&TabItem>` is not a stable idiom in this codebase yet (no precedent component uses one). Adopting it here sets a pattern that must be consistent across other primitives going forward, which is a scope leak from this task.
- The "tail button" isn't actually a tab — making it a prop on `Tabs` lies about what `Tabs` is. The semantically-honest fix is a sibling element, which means `Tabs` doesn't actually solve the problem.
- `editing_index` + `inline_editor` couples the primitive to a Signal-based external editing surface. This is a leak from the rename-flow concern (F7-internal) into a generic UI primitive that the gallery never exercises — a clear single-responsibility violation.
- Extending the primitive's keydown arms (Shift+F10, Delete) makes the F2 dropdown / disabled-tabs gallery consumer behave differently than today (Delete previously did nothing; now it can fire a callback if wired). Even when the callbacks default to `None`, the prop surface telegraphs behavior the gallery never wants.
- Each new prop added later (F11 mode-tree drag-reorder? plugin tabs?) repeats the prop-bloat pressure.

## Path B: Rebuild locally in `mode_tabs/mod.rs`

- **Cost: ~50 LOC across 1 file** (new `frame/top_bar/mode_tabs/mod.rs`).

The Path B count is *only* the LOC needed to recreate what F2 `Tabs` already provides for free — i.e., the parts that would otherwise be a single `<Tabs items={...} value={...} onchange={...} />` call. Per the T29 implementation already drafted in the F7 plan (lines 5276–5402), the duplicated subset is:

- `<div role="tablist" aria-orientation="horizontal" aria-label="Editing mode">` wrapper — ~3 LOC.
- The per-tab `<button>` with `role="tab"`, `aria-selected`, `tabindex` (roving 0/-1), `id="mode-tab-{name}"` — ~15 LOC.
- Per-tab `Signal<Vec<Option<Rc<MountedData>>>>` ref array + `onmounted` callback — ~8 LOC.
- Roving keydown handler (Arrow / Home / End → `set_focus(true).await`) — ~25 LOC.

Total reused-from-scratch: **~50 LOC**.

The other ~70 LOC of T29 (marker dot, sr-only sibling, the bounded walk-past-renaming loop) is F7-specific and lands somewhere either way — that doesn't count against either path. Tasks 30 and 31 (context menu, inline rename) are not part of the primitive's responsibility under either choice.

**Pros:**
- Zero impact on existing call sites and on the F2 `Tabs` primitive — gallery and any future `Tabs` consumers stay simple.
- The local tablist owns concepts F2 should not own: "+" tail (not a tab), inline-edit swap (component-state coupling), per-tab right-click and Delete bindings (F7-specific commands).
- Faster to land: the implementation is already drafted in the plan as Task 29 Step 2.
- Easier to test: pure-logic helpers (`runtime_marker`, `validate_mode_name`) sit in `mode_tabs/logic.rs` (Task 28) and the render is local; no need to test through a primitive-prop matrix.
- `mode_tabs` becomes a self-contained feature module — the chrome around editing modes does not bleed into the design system. A future maintainer reading `frame/top_bar/mode_tabs/` sees the whole thing in one place.

**Cons:**
- ARIA roles and roving-tabindex logic exist in two places (here + `components/tabs.rs`). Future a11y bugs must be fixed in both. Mitigation: the mode-tabs duplication is small (~50 LOC) and the patterns are straightforward enough that drift is unlikely to escape review.
- F2 `Tabs` becomes single-consumer (gallery only). It is still useful as a documented design-system primitive but no longer load-bearing. Mitigation: a single F2 call site is no worse than the gallery's other primitives, all of which exist primarily for documentation and future reuse.

## Decision

**Path B — rebuild locally in `frame/top_bar/mode_tabs/mod.rs`.** The proposal to extend F2 `Tabs` adds ~75 LOC and seven new props to a 146-LOC primitive in service of *one* consumer that doesn't even fit the abstraction (the "+" tail isn't a tab, the rename swap couples the primitive to external Signal state). Rebuilding a ~50-LOC tablist locally avoids that cost entirely and keeps the primitive single-responsibility.

The dominant cost of Path A isn't the line count — it's the prop-surface bloat in a primitive that has no other complex consumer to amortize the complexity against. F2 `Tabs` today is small, comprehensible, and well-tested via the gallery. After Path A it would be the most prop-heavy component in `components/`, with most of those props existing solely for `mode_tabs`. The render-prop pattern (`RenderFn<&TabItem>`) and the Signal-coupling for `editing_index` would also be the first instances of those patterns in the codebase, setting precedents on a path of least resistance rather than design intent.

Testability favors Path B too. The mode-tabs render is a thin shim over `runtime_marker` and `validate_mode_name` — both unit-testable without Dioxus. Going through F2 `Tabs` would mean exercising the marker-decoration render-prop path, the editing-index swap path, etc., from inside whatever integration test we eventually write — slower, more brittle.

The F11/F13 future expansion test resolves clearly. F11 (Modes) is described in the F2 source comment as a planned `Tabs` reuser, but F11's needs (left-rail tree of modes with parent/child, drag-reorder) are tree, not tablist — `Tabs` is the wrong primitive there regardless. F13 (panel-slot consumers) is plugin chrome and doesn't render tabs. No future feature is staring at us saying "I also need a tablist with custom decoration"; the speculative reuse case for Path A is empty.

## What gets reused regardless of path

- **`MENU_FOCUS_JS` → `components/menu/focus_walker.rs`.** Currently inlined in `components/menu.rs` (line 15, with call sites at lines 105 and 118). The F7 mode-tab context menu (Task 30) needs the same focus-walker behavior. Extract to `pub(crate) fn focus_menu_item(menu_id, FocusAction)` so the F2 dropdown and the F7 context menu share one implementation. Independent of Path A vs B — Task 30 Step 1 already plans this.
- **`MountedData::set_focus(true).await` for keyboard-activation focus movement.** Same idiom in both `components/tabs.rs` (lines 110–113) and the drafted `mode_tabs/mod.rs` (lines 5347–5352 of the plan). Not worth extracting — it's a 3-line spawn-and-await — but the pattern is the same.
- **ARIA role + attribute names.** Copy-paste, not a primitive: `role="tablist"`, `role="tab"`, `aria-orientation`, `aria-selected`, `aria-controls`, `tabindex` 0/-1. Stays consistent across both paths by convention, not by code reuse.

## Implications for downstream tasks

The F7 plan as written assumes Path B — Tasks 28–31 implement the local tablist in `frame/top_bar/mode_tabs/` and the F2 `Tabs` primitive is left untouched. No ripple effect; the controller proceeds directly to Task 16 (frame skeleton scaffolding).

If, hypothetically, Path A had been chosen, Tasks 29–31 would have needed full rewrites (consume the extended F2 `Tabs` instead of building a local tablist), and a new sub-task between Task 15 and Task 28 would have been required to land the F2 prop additions plus the per-call-site `None` defaults. That branch is not taken.

Note for Task 30 (context menu): the `MENU_FOCUS_JS` extraction step stays as-planned regardless of the audit outcome; the audit ratifies it, it does not re-decide it.
