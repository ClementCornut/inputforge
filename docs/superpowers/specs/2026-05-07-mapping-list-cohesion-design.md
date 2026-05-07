# Mapping List Cohesion Design

## Summary

The mapping-list rail (F8) carries the most surface area of any region in the
GUI but predates the cohesion pass that recently aligned the right-side
panels (Profiles, Devices). It still uses ad-hoc chips, a hand-rolled output
badge, a non-canonical selected-row treatment, and a hover that paints the
seam color over the seams.

This pass raises the rail's visual floor to match the right panels without
changing its information architecture or interaction semantics. It treats
the rail as one surface region: the mode tabs, the device filter chips, and
the selected row all read as the same shape (one signal: `border-focus` +
`primary 8% on bg`). Ad-hoc chip CSS is replaced by extensions to the
`Badge` primitive. The output identifier moves from a right-floating mono
pill to an inline echo on the source line (name → trigger → vJoy out).

The pass is anchored by Approach 2 from the brainstorming session
(interpretive mirror): port the right-panel idioms, plus the targeted
layout adjustments where the rail's role differs from the panels.

## Goals

- Align row chrome (radius, padding, base, hover, selected, focus-visible)
  with `.profile-row` and `.device-row`.
- Replace `.if-row__output-badge`, `.if-rail__device-chip`, `.if-row__chip`
  with `Badge` primitive variants. Codify the new variants in
  `components/badge.rs` with regression tests.
- Unify the selected/active treatment across the rail: row, device chip,
  mode tab, "Add mapping" hover all read as the same shape
  (`1px var(--color-border-focus)` + `color-mix(--color-primary 8%, --color-bg)`).
- Move the vJoy output identifier from a right-floating pill to the source
  line as a quieter inline badge after a `→` separator.
- Migrate the mode tab strip to the canonical `Tabs` primitive
  (`components/tabs.rs`) where a clean migration is possible; preserve the
  running-mode pip and the trailing "+" affordance.
- Codify the new contracts as tests parallel to the right-panel pass:
  one row-tokens test, one device-chip-active-class test, one tab-shell test.

## Non-Goals

- No change to the F8 keyboard model, drag-drop semantics, context menu,
  inline rename, "+ Add mapping" state machine, or capture flow. State
  machines stay where `keyboard.rs`, `add_inline.rs`, and `rename_inline.rs`
  put them.
- No change to filter behavior. Substring match on name plus source label
  stays. Device-filter behavior from the F8.5 device-filter pass stays.
- No icon SVG edits. Per project policy, Phosphor icons stay verbatim;
  rendering corrections happen at `icon.css` or `Icon` component level.
- No effort estimates. Sequencing is implicit in the implementation plan
  (separate doc), not in the design.

## Foundations

### Inherited contracts

The Pinned-Inspector vs Collapsible-Drawer rule (DESIGN.md §6) does not
directly apply: the rail is its own region owned by `.if-layout__rail` in
`assets/frame/layout.css`, not a panel slot. The rail keeps its existing
`--color-bg-elevated` surface. The status bar (region H, see below) is the
one place where the Drawer rule does fire, and it fires Drawer.

The F8 "hardware as protagonist" line stays. The trigger (device + input)
remains the protagonist of the source line; the mapping name stays as the
row's label anchor. This pass does not flip those weights.

### Banned

Side-stripe `border-left` accents on rows. DESIGN.md §8 names this pattern
as banned (Toast accent stripe is the documented exception, since toasts
sit in the user's peripheral field; rows sit foveal). The selected state
relies on a 1px inset border + tint, not an accent stripe.

### Token map

| Use                     | Token                                                                  |
| ----------------------- | ---------------------------------------------------------------------- |
| Rail surface            | `--color-bg-elevated` (owned by `.if-layout__rail`, unchanged)         |
| Row base                | `--color-bg`                                                           |
| Row hover               | `--color-bg-elevated`                                                  |
| Row selected fill       | `color-mix(in srgb, var(--color-primary) 8%, var(--color-bg))`         |
| Row selected border     | `1px solid var(--color-border-focus)`                                  |
| Focus-visible outline   | `2px solid var(--color-border-focus)`, offset 1px                      |
| Drag-source dim         | `opacity: 0.4`                                                         |
| Seam hairline           | `1px solid var(--color-border)`                                        |
| Strong-tier hairline    | `1px solid var(--color-border-strong)`                                 |
| Status bar surface      | `--color-bg-sunken`                                                    |
| Output mono color       | `--color-output`                                                       |

The previous CSS used `--color-border` (the seam color) as the hover
background by substitution comment ("substituted --color-border for missing
--color-surface-hover"). That substitution is dropped. Hover is
`--color-bg-elevated`.

### Active treatment, parent-surface-relative

The "unified active treatment" used by rows, device chips, mode tabs, and
the dashed-row hover all share the same shape:

```
1px solid var(--color-border-focus)
+ color-mix(in srgb, var(--color-primary) 8%, <parent-surface>)
```

`<parent-surface>` is whatever the element sits on:

- Rows sit on `--color-bg`, so row-selected mixes with `--color-bg`.
- Chips and tabs sit on the rail's `--color-bg-elevated`, so chip-active
  and tab-active mix with `--color-bg-elevated`.
- The dashed-row footer hover uses 5% (not 8%) on `--color-bg`, so it
  reads as "create" rather than "selected", but the border idiom is the
  same.

This is one rule, not three; the visual outcomes look slightly different
because they paint over different parent surfaces, which is correct.

## Region Designs

### A · Mode tabs (`Default | Combat | +`)

Today the strip is a hand-rolled flex container in
`crates/inputforge-gui-dx/src/frame/top_bar/` (mode tab cluster) with custom
underline/active treatment.

Migrate to the canonical `Tabs` primitive
(`crates/inputforge-gui-dx/src/components/tabs.rs:38`) with two
deviations from a vanilla tab strip, both encoded as either `class` overrides
or new optional props on `TabItem`:

- A leading 6px `--color-live` pip on the tab whose mode is the runtime
  live one. Implemented as an optional `running: bool` prop on `TabItem`,
  rendered before the label. The pip is independent of the active tab:
  the user can be editing Combat while Default is the runtime live mode,
  and the pip stays on Default.
- A trailing "+" affordance to add a mode. Rendered as a separate ghost
  dashed-bordered tab cell that sits outside the canonical tab list,
  styled to mirror the rail footer's "+ Add mapping" dashed row so the
  "create" gesture reads as one shape across the rail.

The active tab uses the unified active treatment (`1px border-focus` +
`primary 8% on bg-elevated`). Hover on inactive tabs uses
`--color-bg-elevated`.

Tab tests already exist in `components/tabs.rs`; this pass adds a regression
test in the rail's tests confirming the tab cluster's active class equals
the row-selected class shape (a value-by-value contract test on the
computed class string, not a snapshot).

### B · Filter input

Density unchanged. The input is the existing `TextInput` with
`size=InputSize::Sm` and `placeholder="Filter mappings…"` from
`mapping_list/mod.rs::FilterInput`.

Bottom hairline migrates to `1px solid var(--color-border)` directly. The
existing CSS comment "substituted --color-border for missing
--color-border-subtle" is dropped; the tier name `--color-border` is the
correct token, and the comment misled future readers.

Esc-clears-query and ⌘F-focus-filter behavior is unchanged
(`mapping_list/keyboard.rs`).

### C · Device filter chips (emphasis)

Today, `.if-rail__device-chip` is a hand-rolled chip in
`assets/frame/mapping_list.css` lines 52-76. Idle is bordered, active flips
border + text color to `--color-primary`. The row uses
`overflow-x: auto, flex-wrap: nowrap` (line 43-45).

Migrate to the `Badge` primitive
(`crates/inputforge-gui-dx/src/components/badge.rs:15`). The Badge primitive
needs two new variants for this and the next region; see "Badge primitive
extensions" below.

Idle chip: `Badge variant=Outline`. Active chip: visual matches the row
selected state (1px `--color-border-focus` border, `primary 8% on
bg-elevated` fill, label color `--color-primary`).

Active chips today are conveyed by the `is-active` class. After the
migration, the chip is a button wrapping a Badge, and the active variant
swap happens at the call site in
`mapping_list/mod.rs::DeviceFilterRow`. The `aria-pressed` attribute stays
on the wrapping button (Badge does not own ARIA state).

Wrap behavior: the chip row becomes `flex-wrap: wrap` instead of
`nowrap + overflow-x: auto`. Rationale: typical rigs run 2-4 devices, where
horizontal scroll is wasted scroll-discoverability. For 6+ devices the
wrap may push the rows down by one line, which is acceptable; if this
becomes load-bearing later, a follow-up can introduce a "+N more"
overflow chip. Not in this pass.

### D · Group headers (`AXES`, `BUTTONS`, `HATS`)

Typography unchanged: 10px, 600 weight, 0.08em letter-spaced uppercase,
`--color-text-muted`. (`assets/frame/mapping_list.css` lines 93-100.)

Padding migrates to `var(--space-3) var(--space-3) var(--space-1)` so the
horizontal gutter matches the row's new `--space-3` padding.

Add an optional trailing row count, mono, `--color-border-strong`. Format
is `AXES 8`. The count is computed at the same time the group is filtered
(in `mapping_list/mod.rs::view_state_memo`) so it costs no extra walk.
The count is post-filter (reflects the visible row count, not the total),
matching what the user sees scrolling. Empty groups still drop entirely
(unchanged).

### E · Row anatomy (emphasis)

Row token contract:

| Property         | Value                                                      |
| ---------------- | ---------------------------------------------------------- |
| Padding          | `var(--space-3)` all sides                                 |
| Border-radius    | `var(--radius-md)`                                         |
| Base background  | `var(--color-bg)`                                          |
| Hover background | `var(--color-bg-elevated)`                                 |
| Selected fill    | `color-mix(in srgb, var(--color-primary) 8%, var(--color-bg))` |
| Selected border  | `1px solid var(--color-border-focus)` inset                |
| Focus-visible    | `2px solid var(--color-border-focus)`, offset 1px          |
| Selected name    | `font-weight: 700`                                         |
| Drag-source     | `opacity: 0.4` (sortable primitive owns this)              |
| Row gap          | `2px` between rows (CSS gap on the group container)        |

The 10px reserved-left gutter for the drag handle (today's
`calc(var(--space-3) + 10px)` left padding) is dropped. The
`SortableHandle` becomes a 0-width overlay anchored at the row's left
edge, visible on hover. This keeps density without sacrificing the
hover-only handle affordance.

#### Source line

Today the source line is a flex column with two cells (device, input id),
with the output mono pill as a separate right-anchored child of the row.
After the pass, the source line becomes a single horizontal flow:

```
{device-label}  {input-id}  →  {output-badge}
```

`→` is a `--color-border-strong` arrow glyph (`\u{2192}`), separating
trigger from output. The output is a `Badge variant=Output`, see Badge
primitive extensions.

The right-anchored `.if-row__output-badge` is removed. Truncation of the
device label happens within the source-line flow as today; the output
badge is `flex: 0 0 auto` and never wraps or truncates (output identifiers
are short).

The unnamed-row case (no `summary.name`) renders no name line, only the
source line, identical to today.

#### Qualifier chips (region F)

Today merge and conditional qualifiers render as `.if-row__chip` spans on
a `.if-row__source-qualifiers` line below the source line. They use
italic text and a leading mono glyph (`+` for merge, `⊕` for conditional).

After the pass, qualifier chips become `Badge variant=Outline` with:

- a leading mono glyph (`+` or `⊕`) in `--color-output` (merge) or
  `--color-control-badge-text` (conditional), as today
- italic body text
- standard Outline border + transparent fill

The qualifier line stays under the source line, only when at least one
qualifier exists. Layout shape (gap, baseline) unchanged.

### F · Qualifier chips

Folded into E above.

### G · Add-mapping affordance

The dashed footer row (`.if-add-inline__dashed-row` in
`assets/frame/mapping_list.css` lines 287-311) is already pinned to the
bottom of the rail and has the right copy and shape (commit
`63cf2e9`). Cohesion delta:

- Dashed border: `1px dashed var(--color-border-strong)` matches profiles'
  `+ New profile` row exactly.
- Hover: `border-color: var(--color-border-focus)` plus
  `color-mix(in srgb, var(--color-primary) 5%, var(--color-bg))` tint. This
  is the same hover idiom as the device chip and the mode tab, just at a
  different intensity (5% vs 8%) so the footer reads as "create" rather
  than "selected".
- Border-radius: bumps from `var(--radius-sm)` to `var(--radius-md)` for
  parity with rows.

### H · Status bar

`crates/inputforge-gui-dx/src/frame/status_bar/mod.rs` owns the bar that
spans below the entire window. The bar contains: the warning chip
(`1 warning`), the device count (`3/3 devices`), and the active profile
path (right-aligned).

This is the one region where the Collapsible-Drawer rule from DESIGN.md §6
fires: the status bar is a different region from the workspace above it,
and a luminance shift is the codified signal for region change. Surface
flips to `--color-bg-sunken` with a `1px var(--color-border)` top hairline.

Sub-element treatments:

- `1 warning`: migrate the hand-rolled chip to `Badge variant=Warning
  size=Sm` with a leading `⚠` glyph. The Badge primitive does not currently
  own a `size` prop; see Badge primitive extensions.
- `3/3 devices`: the `3/3` numerator becomes `--font-mono` with
  `--color-text` (today is plain `--color-text-muted`), so the count reads
  as data instead of chrome. The trailing `devices` label stays muted.
- Path: right-aligned mono `--color-border-strong`. Front-ellipsis is a
  polish nicety (CSS `direction: rtl` on the inner span) but not a hard
  requirement; if it ships unstable on Windows WebView2, drop it back to
  back-ellipsis. Tracked as a follow-up on the implementation side, not a
  spec mandate.

### I · Empty states

`mapping_list/empty.rs::EmptyZeroMappings` and `EmptyZeroFilterResults`
are already simplified by commit `63cf2e9`. Cohesion-only deltas:

- Title typography unchanged (18px, 600 weight, `--color-text`).
- Helper unchanged (12px, `--color-text-muted`).
- Ghost-button alignment: confirm both call sites pass `size=ButtonSize::Sm`
  (today they do not specify a size). Vertical rhythm gap stays
  `var(--space-2)`.
- Copy stays as committed in `63cf2e9`.

### J · Add-mapping pad (Capturing / Captured / Collision)

`mapping_list/add_inline.rs` owns the state machine. Cohesion-only deltas:

- Migrate `.if-add-inline__chip` (axis/button/hat tinted chip) to
  `Badge variant=Capture` with a `data-kind` attr driving the hue. See
  Badge primitive extensions.
- Listening modifier (`.if-add-inline__chip--listening` and the
  `if-add-pulse-dot` keyframes) stays as a class override on Badge.
  Animation lives on the Badge's `::before` pseudo-element; this is one
  of the cases where a class override on the primitive beats a new
  variant.
- Pad shell: idle Capturing state border drops from
  `1px solid var(--color-border-focus)` to
  `1px solid var(--color-border-strong)`. Focus-ring semantics are reserved
  for actually-focused elements (input, refresh button, action buttons).
- Collision: keep the warning bg + border. Switch the inline em/strong
  collision text to a leading `Badge variant=Warning` followed by the
  collision sentence, for visual scan parity with the status bar's
  `1 warning` badge. Existing strings unchanged.
- Action-row footer hairline migrates to `1px solid var(--color-border)`
  directly. The existing negative-horizontal-margin trick that extends the
  hairline to the panel inner edge stays (it is a deliberate F4 dialog-
  footer parallel and the radius-md bump does not conflict with it).

## Badge Primitive Extensions

The pass adds three variants to `BadgeVariant` in
`crates/inputforge-gui-dx/src/components/badge.rs:6` and one new prop:

```rust
pub enum BadgeVariant {
    Neutral,    // existing
    Info,       // existing
    Success,    // existing
    Warning,    // existing
    Error,      // existing
    Outline,    // new: transparent fill, --color-border-strong border,
                //      --color-text-muted label. Used by device chip idle,
                //      qualifier chips.
    Output,     // new: --color-output label, --font-mono, faint
                //      output-tinted surface. Used by row's vJoy out.
    Capture,    // new: kind-tinted via data-kind="axis|button|hat",
                //      mono, used by add-inline chip.
}

#[component]
pub fn Badge(
    #[props(default = BadgeVariant::Neutral)] variant: BadgeVariant,
    #[props(default = BadgeSize::Md)] size: BadgeSize,  // new prop
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element { ... }

pub enum BadgeSize {
    Sm,  // 11px font, 1px/6px padding. Used by status bar.
    Md,  // existing visual default. Used everywhere else.
}
```

Each new variant ships with a CSS rule in
`assets/components/badge.css` and a unit test in `components/badge.rs`
asserting the rendered class includes the variant token. The Capture
variant's `data-kind` hue logic is already present in
`assets/frame/mapping_list.css` lines 367-370; that block migrates into
`assets/components/badge.css` keyed on
`.badge--capture[data-kind="axis|button|hat"]`.

## Tabs Primitive Usage

The mode tab strip in `frame/top_bar/` migrates to the `Tabs` primitive
at `components/tabs.rs:38`. The two deviations (running-mode pip and
trailing "+") require:

- Adding an optional `running: bool` field to `TabItem`. The Tabs primitive
  renders a 6px `--color-live` pip before the label when `running == true`.
  Independent of `value` (the active tab id).
- Rendering the trailing "+" outside the `Tabs` items, as a sibling
  element styled with the existing rail footer's dashed-row class, so the
  affordance reads as the same shape across the rail.

If the migration uncovers tab cases that the primitive does not cover (for
example, a confirm-before-switch hook), this spec defers to keeping the
hand-rolled cluster and noting the gap in a follow-up. The migration is
opportunistic; the cohesion of the tab visuals is the load-bearing goal.

## Test Contracts

New tests, parallel to the right-panel pass:

1. **Row tokens contract** (`mapping_list/tests.rs`): asserts the row class
   string is exactly the contract above (one explicit value per property,
   not a snapshot). Mirrors the devices-panel Pinned-Inspector test.
2. **Active treatment unification** (`mapping_list/tests.rs`): asserts the
   selected row, the active device chip, and the active mode tab all
   resolve to the same class shape, encoded as a shared constant.
3. **Output badge migration** (`mapping_list/tests.rs`): asserts the
   `if-row__output-badge` class is no longer rendered and the row contains
   exactly one `badge badge--output` element when the mapping has a
   `first_vjoy_output`.
4. **Status bar surface** (`status_bar/tests.rs`, new file): asserts the
   bar's computed class implies `--color-bg-sunken` and a top hairline,
   locking the Collapsible-Drawer surface contract for this region.
5. **Badge variant smoke tests** (`components/badge.rs`): one test per new
   variant asserting the variant token reaches the DOM class.

The existing `mapping_list/tests.rs::*` snapshot tests get refreshed for
the markup changes; nothing semantically asserted there should regress.

## Open Questions

- **Front-ellipsis on the path**: WebView2 on Windows has had quirks with
  `direction: rtl` text layout in the past. If the implementation finds a
  rendering glitch (RTL flipping punctuation, or ellipsis on the wrong
  side), we drop back to back-ellipsis silently. Not a spec-blocking
  decision.
- **Mode tab migration scope**: if the `Tabs` primitive does not support a
  per-item `running` pip cleanly, the implementation either (a) extends
  the primitive (preferred, since the running pip is a candidate for any
  multi-mode UI), or (b) keeps the hand-rolled mode strip and applies the
  cohesion class shape there directly. (a) is the implementation default;
  (b) is the documented fallback.
- **Group header counts**: if the implementation finds the post-filter
  count flickers under rapid filter typing, the count switches to "total
  in mode" instead of "visible after filter". Document the choice on
  whichever variant ships.

## Audit Trail

Real source references confirmed before writing:

- `crates/inputforge-gui-dx/src/components/badge.rs:6` (`BadgeVariant` enum)
- `crates/inputforge-gui-dx/src/components/badge.rs:15` (`Badge` component)
- `crates/inputforge-gui-dx/src/components/tabs.rs:38` (`Tabs` component)
- `crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs` (rail orchestrator)
- `crates/inputforge-gui-dx/src/frame/mapping_list/row.rs` (row component)
- `crates/inputforge-gui-dx/src/frame/mapping_list/empty.rs` (empty states)
- `crates/inputforge-gui-dx/src/frame/mapping_list/add_inline.rs` (pad)
- `crates/inputforge-gui-dx/src/frame/status_bar/mod.rs` (status bar)
- `crates/inputforge-gui-dx/assets/frame/mapping_list.css` (current CSS)
- `docs/superpowers/specs/2026-04-30-f8-mapping-list-design.md` (parent spec)
- `docs/superpowers/specs/2026-05-04-mapping-list-device-filter-vjoy-badges-design.md` (device filter pass)
- `DESIGN.md §6` (Pinned-Inspector vs Collapsible-Drawer rule)
- `DESIGN.md §8` (banned side-stripe accent)
- Recent commits: `caf7848` (profiles inline affordances), `6bd4b83`
  (devices Badge alignment), `9e9097f` (anchored regions codified),
  `63cf2e9` (mapping-list footer pin + empty-state copy).
