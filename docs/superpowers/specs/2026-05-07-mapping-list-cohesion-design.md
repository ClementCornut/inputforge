# Mapping List Cohesion Design

## Summary

The mapping-list rail (F8) carries the most surface area of any region in the
GUI but predates the cohesion pass that recently aligned the right-side
panels (Profiles, Devices). It still uses ad-hoc chips, a hand-rolled output
badge, a selected-row treatment using primary 18% on transparent
(`mapping_list.css:141`) against the right-panel canon at 8% on bg, and a
hover that paints the seam color over the seams.

This pass raises the rail's visual floor to match the right panels without
changing its information architecture or interaction semantics. It treats
the rail as one surface region: the device filter chips and the selected
row read as the same shape (one signal: `border-focus` +
`primary 8% on bg`). Mode tabs keep the canonical 3px primary
bottom-underline per DESIGN.md section 7; tabs read as navigation, not
selection, and the unified treatment does not extend to them. Ad-hoc chip
CSS is replaced by a new `Chip` primitive. The output identifier moves
from a right-floating mono pill to an inline echo on the source line
(name, trigger, vJoy out, separated by an arrow glyph).

The pass is anchored by Approach 2 from the brainstorming session
(interpretive mirror): port the right-panel idioms, plus the targeted
layout adjustments where the rail's role differs from the panels.

## Goals

- Align row chrome (radius, padding, base, hover, selected, focus-visible)
  with `.if-device-row`. Profile-row has no hover and uses a different
  focus-offset, so it is not the alignment target.
- Replace `.if-row__output-badge`, `.if-rail__device-chip`, `.if-row__chip`
  with a new `Chip` primitive (`components/chip.rs`, new). Badge stays at
  five status variants per DESIGN.md section 7; classification chips with
  mono fonts and `data-kind` hues belong on Chip, not Badge.
- Unify the selected/active treatment across the rail: row, device chip,
  "Add mapping" hover all read as the same shape
  (`1px var(--color-border-focus)` + `color-mix(--color-primary var(--tint-selected), --color-bg)`).
  Mode tabs are excluded; they keep the canonical 3px primary bottom-underline
  per DESIGN.md section 7.
- Move the vJoy output identifier from a right-floating pill to the source
  line as a quieter inline chip after an arrow glyph separator.
- Migrate the mode tab strip to the canonical `Tabs` primitive
  (`components/tabs.rs`); add a `running: bool` extension to `TabItem` for
  the running-mode pip, and place the trailing "+" outside the
  `role="tablist"` container.
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
- PrimaryNav (`top_bar/primary_nav.rs`) is hand-rolled `<nav>` plus
  buttons; not a Tabs consumer; not in scope. Response-curve toolbar
  (`mapping_editor/pipeline/stage_body/response_curve/toolbar.rs:172`)
  uses the Tabs primitive and stays on the canonical underline; no fill
  change there.

## Foundations

### Inherited contracts

The Pinned-Inspector vs Collapsible-Drawer rule (DESIGN.md section 6)
does not apply to the rail: the rule scopes side-panel anchored regions,
and the rail is its own region owned by `.if-layout__rail` in
`assets/frame/layout.css`, not a panel slot. The rail keeps its existing
`--color-bg-elevated` surface. The status bar is shell-level, governed by
DESIGN.md section 7 Status Bar (`DESIGN.md:441-446`); its surface
contract is locked separately from the Drawer rule (see region H).

The F8 "hardware as protagonist" line stays. The trigger (device + input)
remains the protagonist of the source line; the mapping name stays as the
row's label anchor. This pass does not flip those weights.

### Banned

Side-stripe `border-left` accents on rows. DESIGN.md section 8 names
this pattern as banned (Toast accent stripe is the documented exception,
since toasts sit in the user's peripheral field; rows sit foveal). The
selected state relies on a 1px inset border + tint, not an accent
stripe.

### Token map

| Use                     | Token                                                                                |
| ----------------------- | ------------------------------------------------------------------------------------ |
| Rail surface            | `--color-bg-elevated` (owned by `.if-layout__rail`, unchanged)                       |
| Row base                | `--color-bg`                                                                         |
| Row hover background    | `--color-bg-elevated`                                                                |
| Row hover border        | `1px solid var(--color-border-strong)`                                               |
| Row selected fill       | `color-mix(in srgb, var(--color-primary) var(--tint-selected), var(--color-bg))`     |
| Row selected border     | `1px solid var(--color-border-focus)`                                                |
| Focus-visible outline   | `2px solid var(--color-border-focus)`, offset -2px (inset)                           |
| Selected tint percent   | `--tint-selected` (= 8%, new token in `assets/tokens/colors.css`)                    |
| Create tint percent     | `--tint-create` (= 5%, new token in `assets/tokens/colors.css`)                      |
| Drag-source dim         | `opacity: 0.4`                                                                       |
| Seam hairline           | `1px solid var(--color-border)`                                                      |
| Strong-tier hairline    | `1px solid var(--color-border-strong)`                                               |
| Status bar surface      | `--color-bg-sunken`                                                                  |
| Status bar top hairline | `1px solid var(--color-border-strong)`                                               |
| Output mono color       | `--color-output`                                                                     |

The previous CSS used `--color-border` (the seam color) as the hover
background by substitution comment ("substituted --color-border for missing
--color-surface-hover"). That substitution is dropped. Hover is
`--color-bg-elevated`.

### Active treatment, parent-surface-relative

The unified active treatment used by rows, device chips, and the
dashed-row hover all share the same shape:

```
1px solid var(--color-border-focus)
+ color-mix(in srgb, var(--color-primary) var(--tint-selected), <parent-surface>)
```

`<parent-surface>` is whatever the element sits on:

- Rows sit on `--color-bg`, so row-selected mixes with `--color-bg`.
- Chips sit on the rail's `--color-bg-elevated`, so chip-active mixes
  with `--color-bg-elevated`.
- The dashed-row footer hover swaps `var(--tint-selected)` for
  `var(--tint-create)` (5% instead of 8%) on `--color-bg`, so it reads
  as "create" rather than "selected", but the border idiom is the same.

This is one rule, not three; the visual outcomes look slightly different
because they paint over different parent surfaces, which is correct.
The tint percentages live in `--tint-selected` (= 8%) and
`--tint-create` (= 5%) tokens added to
`crates/inputforge-gui-dx/assets/tokens/colors.css`, so any future
intensity tweak ships from one file.

Mode tabs do NOT participate in this rule. They keep the canonical 3px
primary bottom-underline per DESIGN.md section 7. Tabs read as
navigation, not selection.

## Region Designs

### A · Mode tabs (`Default | Combat | +`)

Today the strip is a hand-rolled flex container in
`crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/` (siblings:
`mod.rs`, `add_inline.rs`, `context_menu.rs`, `delete_dialog.rs`,
`logic.rs`, `rename_inline.rs`) with a custom underline/active
treatment.

Migrate to the canonical `Tabs` primitive
(`crates/inputforge-gui-dx/src/components/tabs.rs:38`). The active tab
keeps the canonical `.if-tab--active` shape per DESIGN.md section 7:
3px primary bottom-underline, no fill, transparent background
(`assets/components/tabs.css:48-51`). Hover on inactive tabs raises
text color to `--color-text` only (no fill, no background change),
per `assets/components/tabs.css:44-46`. Mode tabs are excluded from
the unified row/chip/create-row treatment because tabs read as
navigation, not selection.

Two deviations from a vanilla tab strip:

- A leading 6px `--color-live` pip on the tab whose mode is the runtime
  live one. Implemented as a new `running: bool` prop on `TabItem`,
  rendered inside the tab button, before the label. The pip is
  independent of the active tab: the user can be editing Combat while
  Default is the runtime live mode, and the pip stays on Default. Other
  Tabs consumers (notably
  `mapping_editor/pipeline/stage_body/response_curve/toolbar.rs:172`)
  ignore the field by leaving it at its default `false`.
- A trailing "+" affordance to add a mode. Rendered as a SIBLING
  element of the `Tabs` primitive, OUTSIDE the `role="tablist"`
  container, with `role="button"` and `aria-label="Add mode"`. Reachable
  by Tab key, NOT by ArrowRight from the last tab so screen-reader tab
  counts stay honest. Visually styled to mirror the rail footer's
  `+ Add mapping` dashed row so the "create" gesture reads as one shape
  across the rail.

Tab tests already exist in `components/tabs.rs`; this pass adds a
regression test in the rail's tests confirming the mode-tab cluster
renders the canonical `.if-tab--active` class for the active tab
(value-by-value contract test on the computed class string, not a
snapshot).

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
`assets/frame/mapping_list.css` lines 52-76. Idle is bordered, active
flips border + text color to `--color-primary`. The row uses
`overflow-x: auto, flex-wrap: nowrap` at line 45 specifically.

Migrate to the new `Chip` primitive
(`crates/inputforge-gui-dx/src/components/chip.rs`, new file; see "Chip
Primitive" below).

Idle chip: `Chip variant=Outline`. Active chip: visual matches the row
selected state, sourced from the parent surface `--color-bg-elevated`:
1px `--color-border-focus` border,
`color-mix(in srgb, var(--color-primary) var(--tint-selected), var(--color-bg-elevated))`
fill, label color `--color-primary`.

Active chips today are conveyed by the `is-active` class. After the
migration, the chip is a button wrapping a `Chip`, and the active
variant swap happens at the call site in
`mapping_list/mod.rs::DeviceFilterRow`. The `aria-pressed` attribute
stays on the wrapping button (Chip does not own ARIA state).

Wrap behavior: the chip row becomes `flex-wrap: wrap` instead of
`nowrap + overflow-x: auto`. At rail width (~280px) the practical wrap
threshold is 2 to 3 chips; a multi-row chip area is the common case,
not the exception, and group headers shift down accordingly. The
trade-off preserves discoverability (no hidden chips behind a scroll)
at the cost of vertical real estate. A future `+N more` overflow chip
can replace this if the multi-row footprint becomes load-bearing. Not
in this pass.

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

| Property         | Value                                                                                          |
| ---------------- | ---------------------------------------------------------------------------------------------- |
| Padding          | `var(--space-3)` all sides                                                                     |
| Border-radius    | `var(--radius-md)`                                                                             |
| Base background  | `var(--color-bg)`                                                                              |
| Hover background | `var(--color-bg-elevated)`                                                                     |
| Hover border     | `1px solid var(--color-border-strong)` (matches `.if-device-row:hover`, `panel_slot.css:135-138`) |
| Selected fill    | `color-mix(in srgb, var(--color-primary) var(--tint-selected), var(--color-bg))`               |
| Selected border  | `1px solid var(--color-border-focus)` inset                                                    |
| Focus-visible    | `2px solid var(--color-border-focus)`, offset -2px (inset, matches `.if-device-row:focus-visible`, `panel_slot.css:140-143`) |
| Selected name    | `font-weight: 700`                                                                             |
| Drag-source      | `opacity: 0.4` (sortable primitive owns this)                                                  |
| Row gap          | `2px` between rows (CSS gap on the group container)                                            |

The 10px reserved-left gutter for the drag handle (today's
`calc(var(--space-3) + 10px)` left padding) is dropped. The
`SortableHandle` becomes a 0-width overlay anchored at the row's left
edge, visible on hover. This keeps density without sacrificing the
hover-only handle affordance.

#### Source line

Today the source line is a flex column with two cells (device, input
id), with the output mono pill as a separate right-anchored child of
the row. After the pass, the source line becomes a single horizontal
flow:

```
{device-label}  {input-id}  →  {output-chip}
```

The arrow is a `--color-border-strong` glyph (`\u{2192}`) wrapped in
`<span aria-hidden="true">→</span>`, separating trigger from output
visually. Screen readers skip the glyph and rely on label sequence
(device, input id, vJoy out) for semantics. The output is a
`Chip variant=Output`, see "Chip Primitive" below.

The right-anchored `.if-row__output-badge` is removed. Truncation of
the device label happens within the source-line flow as today; the
output chip is `flex: 0 0 auto` and never wraps or truncates (output
identifiers are short).

The unnamed-row case (no `summary.name`) renders no name line, only
the source line, identical to today.

#### Qualifier chips (region F)

Today merge and conditional qualifiers render as `.if-row__chip` spans on
a `.if-row__source-qualifiers` line below the source line. They use
italic text and a leading mono glyph (`+` for merge, `⊕` for conditional).

After the pass, qualifier chips become `Chip variant=Outline` with:

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

- Dashed border: `1px dashed var(--color-border-strong)`. Profiles'
  `+ New profile` row baseline (`profiles.css:263-274`) uses the lighter
  `--color-border` tier and only raises to `--color-border-strong` on
  hover. The rail's footer goes one tier brighter than profiles at
  baseline, deliberately, for two reasons: (a) it sits on the rail's
  elevated surface where the lighter border tier washes out, and (b)
  the unified hover treatment below (focus-cyan plus tint-create) needs
  a baseline border strong enough that the hover reads as a tier change,
  not a color change. This is a deliberate departure from profiles, not
  an attempt to mirror them.
- Hover: `border-color: var(--color-border-focus)` plus
  `color-mix(in srgb, var(--color-primary) var(--tint-create), var(--color-bg))`
  tint. Same border idiom as the device chip's active state, but the
  tint percentage swaps from `--tint-selected` (8%) to `--tint-create`
  (5%) so the footer reads as "create" rather than "selected".
- Border-radius: bumps from `var(--radius-sm)` to `var(--radius-md)` for
  parity with rows.

### H · Status bar

`crates/inputforge-gui-dx/src/frame/status_bar/mod.rs` owns the bar that
spans below the entire window. The bar contains: the warning chip
(`1 warning`), the device count (`3/3 devices`), and the active profile
path (right-aligned).

The status bar is shell-level and governed by DESIGN.md section 7
Status Bar (`DESIGN.md:441-446`), not by the Pinned-Inspector vs
Collapsible-Drawer rule (which scopes side-panel anchored regions). The
bar already uses `--color-bg-sunken` with a
`1px solid var(--color-border-strong)` top hairline
(`assets/components/status-bar.css:20-21`). This pass locks that
contract via a new test mirroring profiles'
`_collapsible_drawer_surface_contract` idiom; it does not change the
surface.

Sub-element treatments:

- `1 warning`: the warning chip is already a `Badge variant=Warning`
  at `frame/status_bar/mod.rs:46`. Add a leading `⚠` glyph inside the
  existing Badge. The default Badge size already fits the bar's 28px
  height; no `size` prop addition is needed.
- `3/3 devices`: the `3/3` numerator becomes `--font-mono` with
  `--color-text` (today is plain `--color-text-muted`), so the count
  reads as data instead of chrome. The trailing `devices` label stays
  muted.
- Path: right-aligned mono `--color-text-muted`. The earlier draft of
  this spec called for `--color-border-strong` here, but that is a
  border tier, not a text color; the path is filesystem readout text
  and stays on the muted text tier. Path slot truncates with standard
  `text-overflow: ellipsis`. Existing `truncate_path` helper at
  `frame/status_bar/logic.rs` continues to gate label length.

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
  `Chip variant=Capture` with a `data-kind` attr driving the hue. See
  "Chip Primitive" below.
- Listening modifier (`.if-add-inline__chip--listening` and the
  `if-add-pulse-dot` keyframes) stays as a class override on Chip.
  Animation lives on the Chip's `::before` pseudo-element; this is one
  of the cases where a class override on the primitive beats a new
  variant.
- Pad shell: idle Capturing state border drops from
  `1px solid var(--color-border-focus)` to
  `1px solid var(--color-border-strong)`. Focus-ring semantics are reserved
  for actually-focused elements (input, refresh button, action buttons).
- Collision: keep the warning bg + border. Switch the inline em/strong
  collision text to a leading `Badge variant=Warning` followed by the
  collision sentence, for visual scan parity with the status bar's
  `1 warning` badge (Warning stays on Badge per DESIGN.md section 7;
  it is a status badge, not a classification chip). Existing strings
  unchanged.
- Action-row footer hairline migrates to `1px solid var(--color-border)`
  directly. The existing negative-horizontal-margin trick that extends the
  hairline to the panel inner edge stays (it is a deliberate F4 dialog-
  footer parallel and the radius-md bump does not conflict with it).

## Chip Primitive

Per DESIGN.md section 7 (`DESIGN.md:411-418`), Badge is scoped to
status, count, and classification with five fixed variants and label
typography (medium weight, no mono fonts). The rail's chip-like
surfaces (idle device chip, qualifier chip, output identifier, capture
chip) are not status indicators, and several need mono fonts or
`data-kind` hues. Adding those uses to Badge would silently widen its
documented scope, so this pass introduces a separate `Chip` primitive.

New files:

- `crates/inputforge-gui-dx/src/components/chip.rs`
- `crates/inputforge-gui-dx/assets/components/chip.css`

Initial API:

```rust
pub enum ChipVariant {
    Outline,    // transparent fill, --color-border-strong border,
                // --color-text-muted label. Used by device chip idle,
                // qualifier chips.
    Output,     // --color-output label, --font-mono, faint
                // output-tinted surface. Used by row's vJoy out.
    Capture,    // kind-tinted via data-kind="axis|button|hat", mono.
                // Used by add-inline chip.
}

#[component]
pub fn Chip(
    #[props(default = ChipVariant::Outline)] variant: ChipVariant,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element { ... }
```

No `size` prop on Chip; density variants ship later if a real consumer
demands one. The Capture variant's `data-kind` hue logic is already
present in `assets/frame/mapping_list.css` lines 367-370; that block
migrates into `assets/components/chip.css` keyed on
`.chip--capture[data-kind="axis|button|hat"]`.

Each Chip variant ships with a CSS rule in
`assets/components/chip.css` and a unit test in `components/chip.rs`
asserting the rendered class includes the variant token. A
component-gallery entry parallel to Badge's lands in
`examples/component_gallery.rs`.

Badge stays untouched: still five status variants (`Neutral`, `Info`,
`Success`, `Warning`, `Error`), no `size` prop, no new variants. The
status bar's existing `Badge variant=Warning` use at
`frame/status_bar/mod.rs:46` continues to fit per DESIGN.md section 7.

## Tabs Primitive Usage

The mode tab strip in `frame/top_bar/mode_tabs/` migrates to the
`Tabs` primitive at `components/tabs.rs:38`. The active treatment is
the canonical `.if-tab--active` underline per DESIGN.md section 7
(3px primary bottom-border, no fill,
`assets/components/tabs.css:48-51`). The two deviations from a vanilla
tab strip:

- Adding a `running: bool` field to `TabItem`. The Tabs primitive
  renders a 6px `--color-live` pip before the label when
  `running == true`. Independent of `value` (the active tab id).
  Other Tabs consumers (notably
  `mapping_editor/pipeline/stage_body/response_curve/toolbar.rs:172`)
  leave the field at its default `false`.
- Rendering the trailing "+" OUTSIDE the `Tabs` primitive's
  `role="tablist"` container, as a sibling `role="button"` element
  with `aria-label="Add mode"`. Reachable by Tab key only, NOT by
  ArrowRight from the last tab (so screen-reader tab counts stay
  honest). Visually styled with the existing rail footer's dashed-row
  class so the "create" affordance reads as one shape across the
  rail.

The `running` pip is a legitimate Tabs primitive extension; any
multi-mode UI in the future may need a similar live-state indicator.
The trailing "+" stays outside the primitive because it is not a tab,
and putting non-tab children inside `role="tablist"` violates
WAI-ARIA tab semantics.

## Test Contracts

New tests, parallel to the right-panel pass:

1. **Row tokens contract** (`mapping_list/tests.rs`): asserts the row
   class string is exactly the contract above (one explicit value per
   property, not a snapshot). Mirrors the devices-panel hover and
   focus-visible rules at `panel_slot.css:117-143`.
2. **Active treatment unification** (`mapping_list/tests.rs`): asserts
   the selected row, the active device chip, and the create-row hover
   all resolve to the same class shape (border + tint), encoded as a
   shared constant. Mode tabs are NOT part of this contract; they
   keep the canonical `.if-tab--active` underline, asserted in the
   tab regression test below.
3. **Mode tab canonical class** (`mapping_list/tests.rs` or
   `mode_tabs/tests.rs`): asserts the active mode tab renders the
   canonical `.if-tab--active` class and does NOT render any
   unified-treatment class. Locks the decision that mode tabs read as
   navigation, not selection.
4. **Output chip migration** (`mapping_list/tests.rs`): asserts the
   `if-row__output-badge` class is no longer rendered and the row
   contains exactly one `chip chip--output` element when the mapping
   has a `first_vjoy_output`.
5. **Status bar surface** (`status_bar/tests.rs`, new file): asserts
   the bar's computed class implies `--color-bg-sunken` and a
   `1px solid var(--color-border-strong)` top hairline, locking the
   existing DESIGN.md section 7 Status Bar surface contract against
   regression. Mirrors profiles'
   `_collapsible_drawer_surface_contract` idiom in shape, even though
   the bar's contract does not stem from the Drawer rule.
6. **Chip variant smoke tests** (`components/chip.rs`): one test per
   Chip variant asserting the variant token reaches the DOM class.

The existing `mapping_list/tests.rs::*` snapshot tests get refreshed
for the markup changes; nothing semantically asserted there should
regress.

## Audit Trail

Real source references confirmed before writing:

Primitives:
- `crates/inputforge-gui-dx/src/components/badge.rs:6` (`BadgeVariant` enum, five status variants)
- `crates/inputforge-gui-dx/src/components/badge.rs:15` (`Badge` component, no `size` prop)
- `crates/inputforge-gui-dx/src/components/tabs.rs:14-47` (`TabItem`, `Tabs` component, no `running` prop today)
- `crates/inputforge-gui-dx/assets/components/tabs.css:48-51` (`.if-tab--active` canonical underline)
- `crates/inputforge-gui-dx/assets/components/badge.css:1-20` (current Badge CSS)
- `crates/inputforge-gui-dx/assets/components/status-bar.css:20-21` (existing `--color-bg-sunken` surface and `--color-border-strong` top hairline)

Rail and panels:
- `crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs` (rail orchestrator)
- `crates/inputforge-gui-dx/src/frame/mapping_list/row.rs` (row component)
- `crates/inputforge-gui-dx/src/frame/mapping_list/empty.rs` (empty states)
- `crates/inputforge-gui-dx/src/frame/mapping_list/add_inline.rs` (pad)
- `crates/inputforge-gui-dx/src/frame/status_bar/mod.rs:46` (`Badge variant=Warning` already in place)
- `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/` (hand-rolled mode tab cluster, current home)
- `crates/inputforge-gui-dx/assets/frame/mapping_list.css` (current CSS)
- `crates/inputforge-gui-dx/assets/frame/panel_slot.css:117-143` (`.if-device-row` row contract: hover, selected, focus-visible)
- `crates/inputforge-gui-dx/assets/frame/profiles.css:240-284` (`.profile-row` and `.profile-row--create`, no hover, focus-offset 2px positive)

Tokens (existence confirmed):
- `crates/inputforge-gui-dx/assets/tokens/colors.css` (all color tokens; `--tint-selected` and `--tint-create` are new)
- `crates/inputforge-gui-dx/assets/tokens/spacing.css` (`--space-1` through `--space-3`)
- `crates/inputforge-gui-dx/assets/tokens/radii.css` (`--radius-sm`, `--radius-md`)
- `crates/inputforge-gui-dx/assets/tokens/typography.css` (`--font-mono`)

Companion specs:
- `docs/superpowers/specs/2026-04-30-f8-mapping-list-design.md` (parent spec)
- `docs/superpowers/specs/2026-05-04-mapping-list-device-filter-vjoy-badges-design.md` (device filter pass)

Design references:
- `DESIGN.md` section 6 (Pinned-Inspector vs Collapsible-Drawer rule, scopes side-panel anchored regions; does NOT apply to the rail or the status bar)
- `DESIGN.md` section 7 (codified contracts for Status Bar, Tabs, Badge)
- `DESIGN.md` section 8 (banned side-stripe accent, toast as named exception)

Recent cohesion lineage:
- `caf7848` (profiles inline affordances)
- `6bd4b83` (devices Badge alignment + row-token contract test idiom)
- `9e9097f` (anchored regions codified + surface-lock contract test idiom)
- `63cf2e9` (mapping-list footer pin + empty-state copy)
