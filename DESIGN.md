---
name: InputForge
description: Sim-input precision tool, Evolved Glass Cockpit dark theme.
colors:
  bg: "#14172A"
  bg-elevated: "#1E223A"
  bg-sunken: "#0E1020"
  text: "#E4E6F0"
  text-muted: "#9CA0BC"
  text-subtle: "#8589A7"
  text-inverse: "#0E1020"
  border: "#2A2E48"
  border-strong: "#424766"
  border-focus: "#5AB0FF"
  primary: "#4FA8FF"
  primary-hover: "#6FBAFF"
  primary-active: "#2D7FD9"
  primary-fg: "#06101F"
  live: "#2EE0A0"
  warning: "#FFB347"
  error: "#F25555"
  error-hover: "#FF6F6F"
  error-active: "#DD4846"
  info: "#4FA8FF"
  processing: "#3FB8B0"
  output: "#C99846"
  control: "#9A78D6"
  control-badge-text: "#B89BEA"
typography:
  display:
    fontFamily: "Inter, system-ui, -apple-system, sans-serif"
    fontSize: "32px"
    fontWeight: 600
    lineHeight: 1.15
    letterSpacing: "-0.01em"
  headline:
    fontFamily: "Inter, system-ui, sans-serif"
    fontSize: "24px"
    fontWeight: 600
    lineHeight: 1.15
  title:
    fontFamily: "Inter, system-ui, sans-serif"
    fontSize: "20px"
    fontWeight: 600
    lineHeight: 1.45
  emphasized-body:
    fontFamily: "Inter, system-ui, sans-serif"
    fontSize: "16px"
    fontWeight: 600
    lineHeight: 1.45
  body:
    fontFamily: "Inter, system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.45
  label:
    fontFamily: "Inter, system-ui, sans-serif"
    fontSize: "12px"
    fontWeight: 500
    lineHeight: 1.45
  caption:
    fontFamily: "Inter, system-ui, sans-serif"
    fontSize: "11px"
    fontWeight: 400
    lineHeight: 1.45
  mono:
    fontFamily: "JetBrainsMono, ui-monospace, 'Cascadia Code', monospace"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.45
rounded:
  none: "0"
  sm: "2px"
  md: "4px"
  lg: "8px"
  full: "9999px"
spacing:
  "0": "0"
  "1": "4px"
  "2": "8px"
  "3": "12px"
  "4": "16px"
  "6": "24px"
  "8": "32px"
  "12": "48px"
  "16": "64px"
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.primary-fg}"
    rounded: "{rounded.md}"
    padding: "8px 16px"
    typography: "{typography.body}"
  button-primary-hover:
    backgroundColor: "{colors.primary-hover}"
    textColor: "{colors.primary-fg}"
  button-primary-active:
    backgroundColor: "{colors.primary-active}"
    textColor: "{colors.primary-fg}"
  button-secondary:
    backgroundColor: "{colors.bg-elevated}"
    textColor: "{colors.text}"
    rounded: "{rounded.md}"
    padding: "8px 16px"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.text}"
    rounded: "{rounded.md}"
    padding: "8px 16px"
  button-danger:
    backgroundColor: "{colors.error}"
    textColor: "{colors.text-inverse}"
    rounded: "{rounded.md}"
    padding: "8px 16px"
  card:
    backgroundColor: "{colors.bg-elevated}"
    rounded: "{rounded.md}"
    padding: "16px"
  text-input:
    backgroundColor: "{colors.bg-sunken}"
    textColor: "{colors.text}"
    rounded: "{rounded.md}"
    padding: "8px 12px"
  number-input:
    backgroundColor: "{colors.bg-sunken}"
    textColor: "{colors.text}"
    rounded: "{rounded.md}"
    typography: "{typography.mono}"
  badge-success:
    backgroundColor: "rgba(46, 224, 160, 0.14)"
    textColor: "{colors.live}"
    rounded: "{rounded.full}"
    padding: "2px 8px"
    typography: "{typography.caption}"
  badge-warning:
    backgroundColor: "rgba(255, 179, 71, 0.14)"
    textColor: "{colors.warning}"
    rounded: "{rounded.full}"
    padding: "2px 8px"
    typography: "{typography.caption}"
  badge-error:
    backgroundColor: "rgba(242, 85, 85, 0.14)"
    textColor: "{colors.error}"
    rounded: "{rounded.full}"
    padding: "2px 8px"
    typography: "{typography.caption}"
  switch-track-checked:
    backgroundColor: "{colors.primary}"
    rounded: "{rounded.full}"
  switch-track-off:
    backgroundColor: "{colors.bg-sunken}"
    rounded: "{rounded.full}"
  tab-active:
    textColor: "{colors.text}"
  tab-inactive:
    textColor: "{colors.text-muted}"
  status-bar:
    backgroundColor: "{colors.bg-sunken}"
    textColor: "{colors.text-muted}"
    typography: "{typography.label}"
  dialog-panel:
    backgroundColor: "{colors.bg-elevated}"
    rounded: "{rounded.md}"
    padding: "16px"
---

# Design System: InputForge

## 1. Overview

**Creative North Star: "The Evolved Glass Cockpit (Dark)"**

InputForge's interface is an instrument-panel for a sim-input pipeline. Every surface borrows from the visual grammar of avionics multi-function displays and software DAW chrome: precise typography, deep navy-tinted neutrals, HUD-blue actions, CRT-phosphor green for live signal, taxonomy hues that stay subordinate to alerts. Density is high because the audience is high. Hierarchy and contrast carry the weight, never decoration. Color appears where it makes a value mechanically more legible (live, dirty, error), never as garnish.

The system is *evolved* glass cockpit because it deliberately rejects two adjacent failure modes. It rejects the literal-skeumorphic cockpit (rivets, brushed-metal bezels, fake chamfers as ornament) in favor of a flat, panel-luminance-layered surface where chamfers exist as 1px inset highlights, not raised plastic. It also rejects the CRT-as-aesthetic move (scanlines, glow bloom, retro-amber-monochrome) because that nostalgia treats the instrument as decoration. InputForge's instrument is functional: the HUD-blue is a real action color, the CRT-phosphor green is a real status color, and they're used the way a real cluster uses them.

The system explicitly rejects the four registers named in PRODUCT.md's anti-references: the JoystickGremlin Tk look, the SaaS-dashboard look, the gaming-RGB look, and the Apple glassmorphism look. Most of the rules below are about preventing slow drifts toward those four directions.

**Key Characteristics:**

- **Layering by luminance, not by shadow.** Sunken / surface / elevated / overlay are all flat at rest. Lift comes from background luminance shift plus a 1px hairline.
- **Information dense.** Tightened leading (1.15 for headers, 1.45 for body and labels), 4px default radius, 11-14px primary text. The grid feels packed because it is.
- **Color used as signal.** Status hues carry full saturation. Taxonomy hues sit ~30% lower chroma so they read as classification, not alert.
- **Inter with deliberate OpenType.** Tabular figures (`tnum`), single-story 'a' (`cv11`), slashed zero (`ss03`) at the body level. The system reads as Inter wearing engineering glasses, not default-Inter SaaS.
- **Cockpit-brisk motion.** 100 / 180 / 240ms with custom easings. No bounce, no elastic, no overshoot. Motion confirms causality, never performs.

## 2. Colors

The palette is the "Evolved Glass Cockpit (Dark)" palette: deep navy surfaces tuned cooler than the egui prototype, HUD cyan-blue as the single action color, CRT phosphor green as the single live-signal color, three muted taxonomy hues for category chips, and amber / desaturated red for warning / error. WCAG 2.2 AA holds for all status text on `bg` and `bg-elevated` (verified: `live` 9.1-10.4×, `warning` 8.8-9.9×, `error` 4.6-5.2×, `primary` 6.2-7.1×). Button-fill pairs (foreground on coloured surface) clear AA in every documented state: `primary-fg` on `primary` 7.6×, on `primary-hover` 9.2×, on `primary-active` 4.7×; `text-inverse` on `error` 5.6×, on `error-hover` 7.0×, on `error-active` 4.6×.

### Primary

- **HUD Cyan-Blue** (`#4FA8FF`): the only action color. Used for primary buttons, focus rings, the active tab indicator, switch-on state, the mapped-input dot. Hover brightens (`#6FBAFF`), active deepens (`#2D7FD9`), foreground on the action surface is the deepest navy (`#06101F`). Info is aliased to this same blue, by design.

### Status

- **CRT Phosphor Green** (`#2EE0A0`): live / running / connected. Reserved for "this is the engine's truth". Slightly cooler than mint to read CRT-instrument rather than fintech.
- **Annunciator Amber** (`#FFB347`): warning. Standard cockpit-amber tuning. Never used for branding or attention-grabbing decoration.
- **Annunciator Red** (`#F25555`, hover `#FF6F6F`, active `#D43F3F`): error. Desaturated a touch to avoid clipping on this surface. The only color permitted on destructive controls.

### Tertiary (Categories)

These are taxonomy markers, not alerts. They get ~30% less chroma than the action / status palette so a row of category chips never visually overpowers a live-status indicator on the same screen.

- **Processing Teal-Cyan** (`#3FB8B0`): pipeline-stage processing nodes (curves, conditionals, transforms). Background tint `rgba(63, 184, 176, 0.14)` mixed into `bg-elevated`; the canonical hue itself reads as text (5.06× contrast, clears AA).
- **Output Burnished Gold** (`#C99846`): vJoy output nodes and emit stages. Background tint `rgba(201, 152, 70, 0.14)` mixed into `bg-elevated`; the canonical hue itself reads as text (4.76× contrast, clears AA).
- **Control Muted Violet** (`#9A78D6` canonical, `#B89BEA` for badge text). The canonical hue serves borders, fills, and chip surfaces; the brighter ramp step (`control-badge-text`) is the legibility-tuned text color used inside category badges, where the canonical hue cannot clear AA against its own 14% tint. Background tint `rgba(154, 120, 214, 0.14)` mixed into `bg-elevated`; badge text uses `#B89BEA` (5.48× contrast, clears AA).

### Neutral

- **Hangar Navy** (`#14172A`): the default surface (`bg`). Cooler and slightly deeper than the egui prototype's `#1A1A2E` for cleaner layering.
- **Panel Navy** (`#1E223A`): elevated surfaces (`bg-elevated`). Cards, dialogs, menus, button defaults. Reads as "lifted" purely through luminance against `bg`.
- **Recess Navy** (`#0E1020`): sunken surfaces (`bg-sunken`). Inputs, selects, switch tracks, status bar. Reads as "set into the panel".
- **Overlay Indigo** (`rgba(8, 10, 22, 0.78)`): backdrop and tooltip surface. The only place where partial transparency is intentional.
- **Instrument Text** (`#E4E6F0`): body / primary text. Pulled cool to harmonize with the navy.
- **Telemetry Text** (`#9CA0BC`): muted secondary text. Used in the status bar, helper text, inactive tabs, captions.
- **Subtle Text** (`#8589A7`): tertiary / placeholder text. Tuned to clear WCAG 2.2 AA on `bg` (5.2×) and `bg-elevated` (4.6×); the prior value (`#686C88`) failed AA at 3.0× on `bg-elevated` and was lifted in line with the 1.4.3 commitment for placeholder text.
- **Hairline Border** (`#2A2E48`) and **Strong Border** (`#424766`): the panel seams. Strong is reserved for "this is a real surface boundary" (status bar top, dialog panel edge, tablist baseline). Hairline is for sub-divisions inside a surface.
- **Focus Cyan** (`#5AB0FF`): focus-ring outlines. One step lighter than the primary so the ring stays distinct against primary surfaces.

### Named Rules

**The One Action Color Rule.** HUD Cyan-Blue is the *only* action color in the system. Primary buttons, focus rings, active tabs, switch-on, mapped-input indicators all use it. There is no second action color. There is no accent color. If a screen has more than one element trying to be "the action", remove all but one.

**The Subordinate Categories Rule.** Processing teal, output gold, and control violet are deliberately desaturated relative to the alert palette. They are never used at full chroma, never used for actions, and never used as decoration. If a category chip on a screen visually competes with a `live` indicator, the category chroma is too high; reduce it.

**The No Pure Black, No Pure White Rule.** All neutrals are tinted toward the navy hue. `#0E1020` is the deepest, `#E4E6F0` is the brightest. `#000` and `#fff` are forbidden anywhere in component CSS.

## 3. Typography

**Body Font:** Inter (with `system-ui`, `-apple-system`, `sans-serif` fallbacks). Loaded via `@font-face` at weights 400 / 600 / 700.
**Mono Font:** JetBrainsMono (with `ui-monospace`, `Cascadia Code`, `monospace` fallbacks). Loaded at 400.

**Character.** Inter with engineering opt-ins. The body inherits `font-feature-settings: "tnum" 1, "ss03" 1, "cv11" 1`, which gives tabular numerals (so columns of axis values align), a slashed zero (so `0` and `O` cannot collide in device IDs), and the single-story `cv11` glyph for `a`. JetBrainsMono is the live-numeric face: number inputs, calibrated values, axis readouts, count badges.

### Hierarchy

Seven sizes, hand-tuned for the dense instrument-cluster range (11-16px) and a modular ratio above (16-32px). In the dense range the steps are weight-distinguished, not size-distinguished: the 1px gap between caption (11) and label (12) is sub-pixel on a standard-resolution display, so weight contrast (regular 400 vs. medium 500) is what actually carries the hierarchy. Above 16px the scale lands at ~1.25 (16 → 20 → 24 → 32) for clean visual hierarchy at heading level.

- **Display** (semibold 600, 32px, leading 1.15, letter-spacing -0.01em): empty states, wizard intros, hero numerics. Rare.
- **Headline** (semibold 600, 24px, leading 1.15): screen headings inside the shell.
- **Title** (semibold 600, 20px, leading 1.45): section headings, panel titles, dialog title.
- **Emphasized Body** (semibold 600, 16px, leading 1.45): button labels, card titles, dialog description.
- **Body** (regular 400, 14px, leading 1.45): default inputs, default labels, default prose. Cap line length at 65-75ch where prose runs.
- **Label** (medium 500, 12px, leading 1.45): helper text, badges, dense cells, status-bar text. Adjacent to a `caption` at 11/400, the +500 weight delta carries the hierarchy.
- **Caption** (regular 400, 11px, leading 1.45): tabular captions, tick labels, kbd chips. The smallest legitimate text in the system. Adjacent to a `label` at 12/500, the −500 weight delta is the perceptual difference, not the 1px size delta.

### Named Rules

**The Tabular Figures Rule.** Numeric values that line up in a column (axis readouts, calibration numbers, action-card values) use `font-feature-settings: "tnum" 1`. The body already opts in; if a component overrides to `--font-features-prose`, it must be a freeform-prose surface only.

**The Mono For Live Numerics Rule.** JetBrainsMono is reserved for inputs and readouts whose value the engine actually owns: the number-input field, axis percentage readouts, calibration thresholds, count badges on coalesced toasts. UI labels and prose stay Inter. Mono is not a stylistic choice; it is a precision affordance.

**The 65ch Rule.** Any prose longer than two lines is capped at 65-75ch. Dialogs hit this limit; help-style text hits this limit; tooltips do not (tooltips stay one line).

## 4. Elevation

**Borders over shadows, made literal.** This system layers surfaces by background luminance and 1px hairlines, not by drop shadows. Cards never lift. Real shadow blur is reserved for surfaces that genuinely float above the canvas: menus, tooltips, dialogs, toasts.

There is no `shadow-1` "slight lift" or "card hover shadow" pattern. If a surface needs to read as elevated relative to its parent, change its background to `bg-elevated` and let the 1px hairline carry the seam.

### Shadow Vocabulary

- **shadow-0** (`none`): default. Most components have no shadow.
- **shadow-1** (`inset 0 1px 0 rgba(255, 255, 255, 0.04)`): the chamfer highlight. A 1px light bloom on the upper edge of an elevated panel, mimicking how light catches the chamfer of a real instrument bezel. Subtle on purpose.
- **shadow-2** (`0 1px 0 rgba(0, 0, 0, 0.5), inset 0 1px 0 rgba(255, 255, 255, 0.05)`): the stamped-panel shadow. A 1px hard ridge below plus the chamfer highlight above. No blur. Reads as a recessed bezel rather than a hovering card. Used by Card.
- **shadow-3** (`0 8px 32px rgba(0, 0, 0, 0.55), 0 0 0 1px rgba(0, 0, 0, 0.4)`): the genuine overlay shadow. Deeper diffusion plus a 1px outline so the edge stays readable on dark backgrounds where soft shadows can vanish. Used only by menus, tooltips, dialogs, toasts.

### Named Rules

**The Cards Don't Float Rule.** A Card uses `shadow-2` (stamped) or `shadow-0` (flat). It never uses `shadow-3`. If you find yourself reaching for a soft drop shadow on a Card, the surface needs a luminance change instead.

**The Real Float Rule.** `shadow-3` is permitted only on a surface that genuinely floats above the canvas: a menu list, a tooltip bubble, a dialog panel, a toast item. Anything else using `shadow-3` is impersonating a float and is wrong.

**The Glassmorphism Exception Rule.** Tooltips are the single permitted use of `backdrop-filter: blur()`. The reason is mechanical: a tooltip floats over arbitrary surfaces, and a small blur prevents the bubble from picking up the underlying surface's color. This is not glassmorphism as decoration; it is a legibility tool. Nothing else gets `backdrop-filter`.

## 5. Motion

Three duration tokens (100 / 180 / 240ms) and two easing tokens. Motion confirms causality and never performs.

### Duration

- **fast** (100ms): hover, focus, press, opacity fade, colour transition. Anything that should feel synchronous with the input event.
- **standard** (180ms): tab indicator slide, toast entry transform, dialog backdrop fade.
- **slow** (240ms): dialog open scale + opacity, larger panel enter / exit. Reserved for spatial transitions large enough to want a touch of weight.

### Easing

- **easing-fast** `cubic-bezier(0.16, 1, 0.3, 1)` (ease-out-expo): the synchronous curve. Used for every ≤180ms transition on hover, focus, press, opacity, colour. The response begins immediately with the input event; the curve carries deceleration, not initial dwell.
- **easing-standard** `cubic-bezier(0.32, 0.08, 0.24, 1)` (the "needle finding its mark" curve): a small initial dwell plus a clean decelerate. Used only for ≥240ms container enter / exit, where the dwell reads as panel weight rather than as latency. Dialog open and Toast slide use this.

### Named Rules

**The Sub-Pixel Curve Rule.** `easing-standard` has a tiny ease-in (Y at P1 = 0.08). At ≥240ms it reads as the panel "settling on its detent". At ≤180ms it reads as latency between the user's gesture and the response. If you find yourself wanting `easing-standard` on a hover or focus transition, you want `easing-fast`.

**The No Bounce Rule.** No bounce, no elastic, no overshoot. The two easing tokens above are the entire vocabulary. If a component spec calls for spring physics, the spec is wrong.

### Motion under `prefers-reduced-motion`

Universal rule, applied at the system root rather than per component:

- Durations collapse to ≤100ms.
- Easing collapses to `linear`.
- Transforms involving spatial motion (slide, scale, translate) drop to opacity-only or no transition at all.
- Colour and luminance changes pass through unchanged; they are never the spatial-motion problem this media query addresses.

Components that document additional reduced-motion behaviour reference this rule plus their per-component note.

## 6. Surfaces

Surfaces are the structural defence against the JoystickGremlin Tk anti-reference (PRODUCT.md L42). The cure for "click through nested windows to change one number" is not better-looking dialogs; it is editing surfaces that don't open dialogs in the first place. The system specifies four kinds of surface, with strict assignment rules.

### Inline-edit primitive (default)

The mapping editor, the curve editor, deadzone fields, calibration thresholds, axis labels, and profile metadata are edited *in place*. A numeric field accepts a click, a focus ring appears, the value can be typed or scrubbed, and the change commits on blur or Enter. There is no "Edit" button that opens a dialog with the same field. Property changes happen on the surface that displays the property.

### Expanding row

Tabular data (the input list, the action list, the binding list) can expand a single row inline to reveal detail without leaving the table context. The expansion is part of the table, not a layer above it. Keyboard navigation in the table continues to work while a row is expanded.

### Side panel (secondary)

For multi-field edits that warrant their own scrollable surface (a complex action's full pipeline, a profile's mode tree), a side panel slides in from the right edge of the workspace. The panel does *not* dim the underlying canvas, does *not* trap focus from the underlying list, and does *not* block keyboard navigation outside its boundary. It is a second region, not a modal layer.

A side panel may anchor a secondary region at its bottom edge. The system specifies two such regions, each with a different surface contract because each carries a different meaning:

- **Pinned Inspector.** Stays on the panel surface (`--color-bg-elevated`), separated from the list above by a 1px `--color-border-strong` top border and `--space-3` padding. Always visible while a list item is selected; never collapsible. Reads as "more detail about the selected item." Used by the Devices panel for the alias edit + hardware/usage facts.
- **Collapsible Drawer.** Flips to `--color-bg-sunken`, with a 44px header bar that carries the title, a count badge, and the open/close affordance. The bar's top edge is `--color-border-strong`; the body slides via the Drawer primitive's persistent variant (see section 7). Reads as "a different region or mode inside the panel." Used by the Profiles panel for snapshots.

### Dialog (rare)

The Dialog primitive (section 7) is reserved for surfaces where the alternative is data loss or destructive action. Permitted uses:

- Dirty-state confirmation when discarding unsaved changes.
- Destructive confirmation for irrevocable actions (delete profile, reset device).
- OS-owned modals that the operating system itself owns (file picker, save dialog).

Forbidden uses (each is a slide back toward the Tk anti-reference):

- Settings opened from a settings panel.
- Property inspector for an item that could be edited inline.
- Profile management as its own dialog instead of an in-shell surface.
- Confirmation chains (a second dialog opening from the first).
- "Add new" flows that take more than two fields: those go to a side panel.

### Named Rules

**The Inline-First Rule.** If a property has a single editable value, it is edited where it is displayed. The dialog opening on click is the canonical Tk failure; the system rejects it as a default. A dialog is justified only when this rule has been actively considered and rejected for a stated reason.

**The No Modal-On-Modal Rule.** A Dialog never opens another Dialog. If a confirmation is needed inside a flow already inside a Dialog, the flow itself is too deep; flatten it to an in-shell surface.

**The Inspector Vs Drawer Rule.** A side-panel anchored region is a Pinned Inspector when its content is *more detail about the currently selected item*; it is a Collapsible Drawer when its content is *a different region or mode inside the panel*. The surface treatment follows from this distinction: Inspector keeps the panel surface so the eye reads continuity; Drawer flips to sunken so the luminance shift signals a different region. If a region is undecided between the two, the question is not "which surface should it use" but "what is this region for"; answer that first, then the surface assignment follows.

## 7. Components

Every component prefixes its CSS classes with `.if-` (InputForge). Hover, focus, active states are inline in the same stylesheet as the default; states are part of the component, not an afterthought.

### Buttons

Five variants in three sizes. Button shape and motion is consistent across variants; only color and border change.

- **Shape:** rounded 4px (`radius-md`), 1px border, padding 8/16 at default size. Sizes `sm` (4/12), `md` (8/16), `lg` (12/24).
- **Primary:** HUD Cyan-Blue background, deepest navy foreground, no border tone shift (`border-color` matches background). Hover brightens to `primary-hover`, active deepens to `primary-active`.
- **Secondary:** elevated-navy background, regular text, **strong** border. Hover raises border to focus-cyan only; the surface stays.
- **Ghost:** transparent background, no border. Hover fills with elevated-navy and shows a hairline border (so a ghost button on a card surface still reads on hover, since the card is already elevated-navy).
- **Danger:** error red surface, inverse text. Reserved for irrevocable actions. Hover brightens, active deepens.
- **Active state across all variants:** `transform: translateY(1px)`. The button mechanically depresses by one pixel. No bounce. On standard-resolution displays (`@media (resolution < 2dppx)`) the press becomes `translateY(2px)` so the displacement remains perceptible; a 1px translate at 1× DPR can render as sub-pixel and disappear.
- **Focus:** 2px focus-cyan outline at 2px offset. Visible against every background in the system.
- **Disabled:** opacity 0.5, cursor not-allowed. No surface tinting, no removed border, no decorative crossed-out style.

### Icon Buttons

Same color and state rules as Button, sized as squares: 28 / 36 / 44 px. The `ghost` icon-button hovers to `border` color, not `bg-elevated`, so the hover surface stays visible when the icon-button sits inside a Card.

### Inputs (Text, Select, Number)

- **Surface:** sunken navy. Inputs read as recessed into the panel.
- **Border:** 1px hairline, raises to strong on hover, raises to focus-cyan on focus with a 2px outline at 2px offset.
- **Radius:** 4px (`radius-md`).
- **Invalid:** error red border, error red focus outline. The only invalid signal is color plus the surrounding helper text; no shake animation, no icon stamp.
- **Number input:** mono font, right-aligned text, 6ch wide field with vertical steppers on the right separated by a 1px hairline. Native spinner buttons hidden cross-browser.
- **Dense-row variant (`.if-input--inset`):** when inputs ship in tightly-packed rows (number-input columns in calibration, axis readout fields in the input list), the focus ring switches from the default 2px outline at 2px offset to an inset variant: a 2px focus-cyan outline at `outline-offset: -2px` plus a matching 1px inset `box-shadow` to seal the inner edge. The ring sits inside the input boundary and does not clip into neighbouring inputs at 4px gaps. The default ring stays for isolated inputs (form fields, dialog inputs).

### Switch

- **Track:** sunken navy with hairline border, 36×20, full-radius pill.
- **Thumb:** 16×16, full-radius. Off-state thumb is muted-text gray. On-state track flips to primary, thumb flips to primary-foreground.
- **Translation:** thumb position is calculated from token math (`calc(--control-size-lg - --control-size-sm - 4 × --control-border-w)`), not hard-coded, so resizing the track propagates.
- **Focus:** 2px focus-cyan outline at 2px offset, on the track.

### Checkbox

- **Box:** 16×16 sunken navy with hairline border, `radius-sm` (2px).
- **Checked:** primary surface, primary-fg checkmark drawn as a rotated CSS border (no SVG glyph, no font icon).
- **Indeterminate:** primary surface, primary-fg horizontal bar.
- **Focus:** 2px focus-cyan outline.

### Slider

- **Track:** 4px tall, hairline-border color, full-radius. Native `<input type="range">` styled cross-browser.
- **Thumb:** 16×16 primary disc with a 2px `bg`-color border (so the thumb visually detaches from the track). Cursor `grab`.
- **Focus:** 2px focus-cyan outline at 4px offset (further than other components, because the thumb rides on the track and a closer offset would collide).

### Card

- **Shape:** elevated-navy background, 1px hairline, radius 4px (`radius-md`).
- **Padding:** sm 12 / md 16 / lg 24.
- **Shadow:** none by default. The Card primitive itself does not stamp; surfaces that want the stamped read-out use `shadow-2` per the Elevation rules. Most Cards in InputForge are flat-with-hairline.
- **Nested cards:** never. A Card containing a Card is always wrong.

### Badge

A pill with five color modes. Each mode uses a composited background tint (~14% of the level hue mixed into the canvas) plus matching foreground text plus matching border. Same hue carries through all three properties so a single badge reads as one cohesive chip.

- **Shape:** full-radius pill, 2px / 8px padding.
- **Type:** caption-size (11px), medium weight, line-height 1, gap 4px between icon and label.
- **Variants:** neutral (sunken bg, muted text), info (HUD-blue), success (CRT-green), warning (amber), error (red).
- **Use:** status indicators, count chips, classification markers. Not for navigation, not for action.

### Tabs

- **Tablist:** horizontal flex, 1px strong-border bottom edge.
- **Tab:** transparent background, muted-text label, 3px transparent bottom-border indicator. Active tab raises text to bright and flips the bottom-border to primary. The tab's bottom-margin is -1px so the active 3px indicator overlaps the tablist border, anchoring the indicator to the panel edge rather than floating above it.
- **Hover:** muted-text raises to bright. No background fill. The tablist surface stays flat.
- **Focus:** 2px focus-cyan outline inside the tab (negative offset) so the outline never collides with the underline indicator.
- **Disabled active:** indicator desaturates via `color-mix` so a suspended-but-selected state is still visible.

### Menu

- **Trigger:** secondary-button styling.
- **List:** elevated-navy surface, 1px border, radius-md, `shadow-3` (the only floating overlay shadow), 4px gap below the trigger.
- **Item:** transparent ground, hover background = `border` color (a subtle gray neutral, not elevated-navy, so a menu inside a Card still shows hover). Padding 8/12. Border-radius `sm`.
- **Item focus:** 2px focus-cyan outline with negative inset offset so the ring sits inside the item's bounds.

### Tooltip

- **Bubble:** overlay-indigo surface with `backdrop-filter: blur(8px)` (the system's only `backdrop-filter`, see Elevation rules). 1px hairline border. Caption-size text. Padding 4/8.
- **Position:** four sides, set by modifier class.
- **Motion:** 100ms opacity fade on hover or focus-within.

### Status Bar

- **Surface:** sunken navy with a 1px **strong** border on top, 28px tall, padding 0 / 12.
- **Type:** label size (12px), muted-text color. The status bar is for *glancing*, not acting; consumers raise specific badges or pills to bright text via their own component, never through the status-bar text color.
- **Layout:** three slots (start / middle / end). Middle claims `flex: 1` so the end slot anchors to the right.
- **Overflow:** the middle slot's text truncates with ellipsis when the window is too narrow for start + middle + end to coexist. No marquee, no horizontal scroll, no wrap to a second line; the bar's height is fixed at 28px.

### Dialog

- **Panel:** native `<dialog>` reset, elevated-navy surface, 1px **strong** border, radius-md, 16px padding. `shadow-3` plus the chamfer highlight (`shadow-1` inset) compounded.
- **Layout:** flex column with title, description, body (`flex: 1, min-height: 0` for scroll), footer. Body collapses cleanly when empty; footer always parks at the bottom of the panel.
- **Title:** title typography, letter-spacing -0.01em, `leading-tight`.
- **Description:** body typography in `text-muted`.
- **Footer:** flex-end aligned action row with a 1px hairline divider above. Padding extends through the panel padding via negative-margin trick so the divider runs the full panel width.
- **Backdrop:** overlay-indigo. Fades in alongside the panel (180ms with `easing-standard`).
- **Open motion:** scale 0.96 → 1.0 plus opacity 0 → 1 via `@starting-style`, 240ms with `easing-standard`. Discrete display switch is timeline-aware. Reduced motion follows the global rule: the scale transform drops, the opacity fade remains.

### Toast (Annunciator)

The signature component. Each toast is a backlit cockpit annunciator: chamfer-aligned 3px left accent in `currentColor` (the level color), a soft glow in the same color, a 6-8% wash of the level hue mixed into the elevated-navy surface via `color-mix`. Right-anchored stack at the top-right of the viewport, pointer-events transparent except on the toast itself.

- **Surface:** elevated-navy mixed with the level color at 6-8%. 1px border. Radius-md. Chamfer highlight + `shadow-3`.
- **Level coloring:** info / success / warning / error. The accent stripe and icon use `currentColor`; the surface tint and border are derived from the same level hue.
- **Motion:** 240ms opacity with `easing-standard`, 180ms slide transform on entry. Slide in from +12px right plus fade. Hover nudges left 2px and brightens the border (100ms with `easing-fast`). Reduced motion follows the global rule: the slide drops, the opacity fade remains.
- **Coalesce badge:** when the same message repeats, a count chip appears between the message and the close button. Mono numerals, currentColor border at 28%, currentColor background at 14%. The chip never shifts the close button; the layout is fixed-tail.

The 3px left accent is the system's one documented exception to the "no side-stripe borders" rule (see Don'ts). The exception is functional, not stylistic: a Toast surfaces in the top-right of the viewport while the user's eyes are on the game window in another window. The stripe is the *peripheral-vision* level channel: recognizable as a colour cue at 30°+ off-axis, before the icon or the surface tint resolves. The icon, count chip, and surface tint are the foveal channel for when the user actually turns to look at the toast. Other surfaces (cards, list items, callouts, alerts) sit in the foveal field and have no peripheral-channel argument; the rule does not extend to them. The stripe sits inside the chamfer highlight rather than glued onto the panel edge.

## 8. Do's and Don'ts

These rules are forceful by design. The voice of a design director, not a stylebook.

### Do:

- **Do** layer surfaces by luminance plus a 1px hairline. Never by drop shadow.
- **Do** use `shadow-3` only on surfaces that genuinely float (menu, tooltip, dialog, toast).
- **Do** keep numeric values in a tabular column on JetBrainsMono with `tnum` enabled. Axis readouts, calibration thresholds, count badges.
- **Do** keep the action color singular. HUD Cyan-Blue is the only action color; if you need a second, you are inventing a problem.
- **Do** carry status through *shape and label*, not color alone. A live indicator is a filled dot plus a label; a paused indicator is a ring plus a label.
- **Do** raise inactive elements to bright text on hover. Tabs, ghost buttons, menu triggers all do this.
- **Do** put real device names and real input addresses on the screen. Never abbreviate "Stick X axis" to a generic "axis" if there is space for the real name.
- **Do** keep tightened leading (1.15 for headers, 1.45 for body) for instrument-cluster density.

### Don't:

- **Don't** use `#000` or `#fff` anywhere. All neutrals tint toward the navy hue.
- **Don't** add side-stripe borders (`border-left` or `border-right` greater than 1px as a colored accent) to cards, list items, callouts, or panels. The Toast accent stripe is the one documented exception (see Components / Toast); do not extend the pattern. The exception requires the surface to be peripheral-channel-eligible: sitting outside the user's foveal field at the moment of recognition. Cards, list items, callouts, and alerts all sit inside the foveal field and have no such argument. Use full borders, surface tints, leading icons, or nothing.
- **Don't** apply gradient text (`background-clip: text` plus a gradient). Decorative, never meaningful. Use a single solid color and adjust weight or size for emphasis.
- **Don't** use `backdrop-filter` outside of Tooltip. The Apple-glassmorphism look is a named anti-reference.
- **Don't** float Cards with soft drop shadows. A Card is flat or stamped (`shadow-2`), never hovering.
- **Don't** let category hues compete with status hues. Processing teal, output gold, and control violet are subordinated by chroma so they read as taxonomy. Do not raise their saturation.
- **Don't** add bounce, elastic, or overshoot to motion. Cockpit-brisk only. `easing-fast` for ≤180ms transitions, `easing-standard` for ≥240ms container enter / exit; see section 5 Motion.
- **Don't** render a `caption` (11/400) and a `label` (12/500) at the same weight when they sit adjacent. The 1px size delta is sub-pixel; weight contrast is what carries the hierarchy in the dense range. If two adjacent dense-range chips need to read as the same level, give them the same token.
- **Don't** use big number cards, soft pastel shadows, friendly empty-state illustrations, marketing-style stat tiles, or "Welcome back" greetings. The generic-SaaS-dashboard look is a named anti-reference. Empty states use Display-typography numerics (`0 profiles`, `No devices connected`) or instrument-style "no signal" framings; never SVG illustrations of people, devices, or data, and never "Get started!" copy.
- **Don't** use cyan glows, RGB edge lighting, gradient angles, or "esports" type. The gaming-RGB look is a named anti-reference.
- **Don't** use grey system-default forms, modal-after-modal flows, dialog-soup confirmation chains, or Tk-era widget chrome. The JoystickGremlin look is the whole reason for the rewrite.
- **Don't** wrap every region in a Card. Most surfaces in InputForge are panel regions, not contained cards. Use spacing and hairline dividers instead.
- **Don't** nest Cards. A Card inside a Card is always a structural mistake; flatten the outer surface to plain panel chrome.
- **Don't** use modals for confirmations that can be inline. The dirty-state confirmation is a modal because the alternative is data loss; routine confirmations should not be. See section 6 Surfaces for the full assignment of inline / expanding-row / side-panel / dialog and the list of forbidden dialog uses.
