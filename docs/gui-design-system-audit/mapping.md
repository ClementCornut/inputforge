# Mapping editor, mapping rail and batch mapping: design-system audit

Read-only source audit, 2026-10-18. Paths below are relative to `crates/inputforge-gui-dx/`. Priorities: P1 user-facing accessibility/function issue; P2 consistency or reusable-component gap; P3 low-impact consolidation. No fixes, GUI launch or tests. Cartog stats and map succeeded (747 indexed files); multi-file outlines stalled and were cancelled. Exact mapped source files and CSS bodies were read directly. Native controls in test-only harnesses are excluded from migration counts.

## Findings and every native-control declaration site

### M1 [P1] Rail context menu bypasses the existing keyboard/overlay infrastructure

`src/frame/mapping_list/mod.rs:456` clickable generic backdrop, `:457` manually positioned role=menu, `:458` Rename button, `:459` Duplicate button, `:460` generic submenu host, `:462` submenu, `:465` per-mode button, `:471` Move up button, `:473` Move down button, `:475` Delete button. CSS `assets/frame/mapping_list.css`: `.if-row-menu-backdrop`:589; `.if-row-menu`:595; `.if-row-menu__item`:605; `:hover`:617; disabled selectors :626/:631; danger :635; submenu-host :639; submenu :643; hover/focus-within opening :654/:655.

This duplicates `AnchoredMenu`/`MenuItem` already used by `pipeline/stage_actions_menu.rs:84-92`. The rail mount never installs menu key handling, autofocus, focus return or viewport measurement. The duplicate-to-mode host is a nonfocusable `div`, and its child menu is `display:none` until hover/focus-within; keyboard users cannot focus a hidden descendant to open it. The fixed `left/top` menu and absolute `left:100%` submenu do not clamp to viewport. CSS uses independent z-index 100/101. Existing app-owned shortcut listeners elsewhere must be preserved, but they do not supply this missing submenu trigger.

Use `AnchoredMenu` + `MenuItem` for the flat actions; add shared submenu trigger/items support (or a keyboard-operable flat grouped submenu), because existing flat menu APIs do not model nested menus. Preserve prepared `Target` identity, filtered/boundary disabled reasons, failed-dispatch retention, app callbacks, pointer coordinates, Shift+F10/right-click invocation and delete confirmation. Do not make DS adoption close the menu on failed delivery. Suggested command: `$impeccable harden`.

### M2 [P2] Rebind field is copied three times with six native buttons

All six declaration sites:

- `src/frame/mapping_editor/header.rs:227` cancel; `:238` rebind; surrounding listening/idle chrome :220/:236.
- `src/frame/mapping_editor/pipeline/stage_body/merge_axis.rs:246` cancel; `:255` rebind; chrome :238.
- `src/frame/mapping_editor/pipeline/stage_body/predicate.rs:195` cancel; `:206` rebind; chrome :188/:204.

CSS in `mapping_editor.css`: `.if-rebind-composite`:184, label :196, listening :201, pulse :209, reduced motion :218, action :225, hover :241, focus :246, listening :251, action divider :255, unbound label :265. This is already one CSS contract, but every consumer reconstructs its structure and accessibility. A reusable `RebindField`/capture surface in `src/components` should own display/listening/unbound states, status announcement, action semantics and visual chrome; callbacks remain supplied by the existing capture pattern. Existing `Button` can cover simple actions but cannot preserve an entirely unstyled segmented field with current API without consumer overrides; add one appropriate chrome/unstyled variant or use a dedicated field component. Preserve Any vs AxesOnly capture filters, original action names, cancellation, source labels, unbound italics, polite announcement and ownership on dispatch failure. Suggested: `$impeccable distill`.

### M3 [P2] Physical-key capture surface is implemented twice

`src/frame/mapping_editor/pipeline/add_palette.rs:93` capture/cancel prompt button (autofocus, detailed label, clickable cancellation); `src/frame/mapping_editor/pipeline/stage_body/map_to_keyboard.rs:179` stable capture button (dynamic label, `data-key-capture`, surrounding hint text). CSS `mapping_editor.css` `.if-key-capture`:270, surface :277, hover :297, focus :302, listening :307, dot :313, hint :323, reduced motion :328. Extract a presentational `KeyCaptureButton` using shared button behavior while keeping app-owned capture hooks outside. `Button` needs autofocus/custom data forwarding if used directly. Preserve physical-key semantics, modifier-only/unsupported-key feedback, active-vs-idle DOM marker used by integration, Esc cancellation, stable button identity and successful-delivery-only publication. Suggested: `$impeccable distill`.

### M4 [P2] Editor title is a raw text input plus a custom interactive heading

`src/frame/mapping_editor/header.rs:183` native text input; `:208` focusable `h2` with F2/right-click rename. It manually invokes `components::behavior().text` at :170 to maintain DS text ownership, so replacing it with a naive controlled input would regress typing. CSS `mapping_editor.css` title :104, focus :116, input :124, input focus :137 duplicates input/focus styling; title typography is deliberately inherited for inline editing.

Extend `TextInput` with `aria_label`, required `data-editor-focus` forwarding and an inline/title visual treatment, or build a small reusable `InlineEditableTitle` from it. Preserve initial-value/owner transport, select-all mounted focus, Enter/blur commit, Esc cancel, F2/contextmenu entry, draft retention after failure, saved-name semantics and heading role. Keep focusable heading behavior explicit rather than treating it as an ordinary action button. Suggested: `$impeccable distill`.

### M5 [P2] Recovery action bypasses Button

`src/frame/mapping_editor/engine_offline_banner.rs:52` native recovery button; CSS `mapping_editor.css` `.if-editor__offline-action`:75-84 duplicates typography/border/padding/cursor without the DS hover/disabled contract. Use `Button` Sm Secondary (or a documented error-context action variant if required). Preserve dynamic `action.label()` and `routing.dispatch`, banner alert/status choice and cleanup-failure text. Related `mapping_issues.rs:28` already uses Button for Open Devices and is a good replacement example. Suggested: `$impeccable distill`.

### M6 [P2] Readout expansion icon bypasses IconButton

`src/frame/mapping_editor/live_readout/out_block.rs:182` native icon-only button. CSS `mapping_editor.css` `.if-editor__readout-chevron`:897; hover :914; focus :918; spacer :923. This duplicates IconButton ghost styling. Extend `IconButton` with `aria_expanded` (and optional `aria_controls`), keep label as current static expand/collapse strings, use Sm and retain only grid-column layout locally. Preserve expansion per output, `has_chain` conditional, frozen readout behavior and decorative spacer. Suggested: `$impeccable distill`.

### M7 [P2] Stage expansion header is another button family

`src/frame/mapping_editor/pipeline/stage_header.rs:77` full-width native button. CSS `mapping_editor.css` `.if-stage__header`:1136; title :1148; summary :1164; right-slot :1184; focus :1205. A generic Button currently accepts children but lacks `aria_expanded`/`aria_controls` and unstyled/disclosure chrome. Extend one reusable `DisclosureTrigger`/button primitive to own semantics and focus, retaining stage-specific title/summary/thumbnail layout and category surface locally. Do not replace the whole header with an IconButton: it would shrink the hit target and lose title/summary semantics. Preserve the single full-width keyboard activation, response-curve accessible summary, malformed state, body ID and thumbnail slot. `stage.rs:163` contextmenu on sortable `li` is a legitimate domain shell; keep it paired with the existing keyboard navigation rather than inventing another generic wrapper. Suggested: `$impeccable distill`.

### M8 [P2] Eight predicate checkboxes bypass Checkbox

`src/frame/mapping_editor/pipeline/stage_body/predicate.rs:507` native checkbox repeated for all eight `HAT_VARIANTS` (definition :95-104), inside label :506. CSS `mapping_editor.css` `.if-hat-direction-grid`:491 and cell :498. Use shared `Checkbox` with stable per-direction checked signals and `id`/external label association (Checkbox itself renders a label; do not nest it inside another label). Preserve direction text, independently toggled membership, compass order, engine dispatch and failure behavior. Grid geometry remains domain layout. Suggested: `$impeccable distill`.

### M9 [P2] Add mapping and Add stage duplicate a create-slot action style

`src/frame/mapping_list/add_inline_rendering.rs:35` raw Add mapping button; `.if-add-inline__dashed-row` in `mapping_list.css:438`, hover :454. `src/frame/mapping_editor/pipeline/add_palette.rs:229` already uses `MenuTrigger` but requests `unstyled:true` at :231, then recreates a full action surface via `.if-add-palette__trigger` `mapping_editor.css:1280`, hover :1302, focus :1307, louder :1312, louder hover :1319. These are the same add-next-slot affordance with separate border, font, focus and spacing definitions. Add a reusable create-slot button/variant, with a compatible trigger visual variant rather than nesting two buttons. Preserve loud empty-pipeline vs compact-plus labels, menu association, full-width hit area and the app-owned begin/capture flow. Suggested: `$impeccable distill`.

### M10 [P2] Device filter wraps Chip in an ad hoc toggle button

`src/frame/mapping_list/mod.rs:370` native button wrapping presentational Chip :378. CSS `mapping_list.css` reset :58; pressed descendant :72; base inner-chip override :90; hover :105; focus :110. Add a reusable toggle/filter chip (`pressed`, `disabled`, `onclick`, `title`) or extend Chip with an explicit interactive variant; do not turn every presentational Chip into an action. Existing `SegmentedControl` is not a drop-in: independent device selection can be cleared and this strip is a chip filter, not its present violet pill strategy selector. Preserve dynamic device label/title and `events.device` semantics. Suggested: `$impeccable distill`.

### M11 [P2] Batch action chips implement a separate button/toggle vocabulary

`src/frame/bulk_map/rows.rs:42` local Chip function renders a native button, invoked for Skip all conflicts :29, Replace all conflicts :30, Include all :31, Exclude all :32. `rows.rs:112` separate native per-row Replace/Replacing toggle, with `aria-pressed` :119. CSS `bulk_map.css` `.if-bulk-map__chip`:220; active :235. This local Chip conflicts conceptually with `components::Chip`, which is a span. Plain group actions can use Button Sm Ghost/Secondary; the row toggle needs Button pressed-state support or the same reusable toggle-chip introduced for device filters. Preserve selected replacement warning, per-kind group actions, app intent dispatch, conflict-only visibility and post-failure retained state. Suggested: `$impeccable distill`.

### M12 [P2] Selectable mapping rows need a shared row interaction contract, not Button wrapping

`src/frame/mapping_list/row.rs:185` generic `div role=button`, roving tabindex :188, onclick :190, contextmenu :191 and dragstart :192, containing `SortableHandle` :194 and optional TextInput editor :202. CSS `.if-row` `mapping_list.css:172`, hover :191, active :225, focus :254 and handle descendant override :205 duplicate list-row chrome also described in comments as matching device rows. This cannot become a native button around its nested handle/input. Prefer a reusable selectable-row surface/interaction primitive with an explicit roving-focus contract and separate child controls. Existing app-level keyboard/navigation listener already handles selection, so do not claim every div row lacks keyboard support. Preserve source qualifiers/live ring, selection navigation, rename, contextmenu and same-group reorder. Suggested: `$impeccable distill`.

## Existing primitives used, but naming/association/API needs improvement

### M13 [P1/P2] Missing explicit labels and label associations

Source inspection only: computed accessibility-tree names were not measured. The following two placeholder-only fields lack explicit persistent labels; some user agents may derive their accessible names from placeholders. Treat these as P2 naming robustness/visible-label gaps rather than proven absent computed names:

- `mapping_list/mod.rs:406` filter TextInput: placeholder only; no id, Field or explicit accessible-label association.
- `mapping_list/add_inline_rendering.rs:144` mapping name TextInput: placeholder only.

The remaining consumers have no explicit programmatic label association and no placeholder fallback in their source (P1):

- `mapping_list/rename_inline.rs:100` rename TextInput: no label/id/placeholder. Wrapper :93 forwards key/blur events; those are event-scope handlers and not a separate control to wrap.
- `mapping_editor/pipeline/stage_body/change_mode.rs:270` Target mode Select, visible label :269 has no `for`.
- `.../map_to_mouse.rs:93` Target Select, visible label :92 has no `for`.
- `.../map_to_vjoy.rs:245` Device Select + label :244, and :253 Output Select + label :252, neither linked.
- `.../merge_axis.rs:230` Operation Select + label :229, not linked.
- `.../predicate.rs:613` predicate kind Select has no label; :430 minimum NumberInput + label :429 and :452 maximum NumberInput + label :451 are not linked.
- `bulk_map/rows.rs:103` per-row Select has id :104 but no associated label/labelledby (visible source sibling :97 does not name it).

Use `Field`/`Label` and stable IDs; add `aria_label` to TextInput/Select/NumberInput for compact cases rather than adding hidden labels per callsite. Add describedby/invalid forwarding where errors are shown. Preserve text owner and commit callback contracts; this audit does not recommend moving workflows into DS. Header title input already has its own accessible name and is not part of this issue. Gesture threshold wrapping label and Switch label are already supplied. Suggested: `$impeccable harden`.

### M14 [P1] Deadzone toolbar reuses IDs across independently expanded stages

`mapping_editor/pipeline/stage_body/deadzone/toolbar.rs:20` fixed IDs `dz-low`, `dz-cl`, `dz-ch`, `dz-high`; used by Field :24 and NumberInput :25 for every toolbar instance. Multiple Deadzone stages can be expanded, so duplicate labels can target the first control. Use per-instance instrument/stage ID in these four IDs (the plot already uses unique mount IDs from instruments/wiring.rs:80). Keep bounds, Number commit vs step and errors. Suggested: `$impeccable harden`.

### M15 [P2] Domain plot frames are duplicated; keep SVG geometry local

`mapping_editor/pipeline/stage_body/response_curve/mod.rs:70` and `deadzone/mod.rs:58` are focusable generic instrument surfaces with native key handlers, mounted pointer observation, data-hovered/dragging and blur callbacks. CSS `response_curve.css:31-50` and `deadzone.css:25-40` duplicate square layout, pointer/text-selection behavior and focus ring. A small reusable `PlotFrame` may own that mechanical surface only; it needs an explicit role/name/keyboard instructions/value-description contract. Current aria-label on a generic div is not equivalent to a described custom control. Do not wrap every SVG circle/path in Button, Slider or a generic component: hit testing and keyboard editing belong to this specialized instrument and app bindings. Preserve pointer capture/mounted observation, key default prevention, blur commit, dragging/split/focus rings and sampled curve semantics. The SVG renderers/thumbnail files otherwise use tokenized colors and correct domain geometry. This is a DS extension opportunity; no assertion of broken pointer behavior is made without GUI validation. Suggested: `$impeccable harden`, then `$impeccable distill`.

## CSS duplication and token bypass inventory

### M16 [P2] Frame CSS overrides existing DS internals

- `mapping_list.css:473` `.if-add-inline--pad input`: rewrites TextInput font, background, color, border, radius and padding. `:572` `.if-row-rename`: rewrites the same control chrome. `:582-586` filter/rename/captured focus selectors replace the DS focus offset. `--captured` focus selector is stale: captured markup uses the common `--pad` class, not this modifier. Use component size/variant APIs and leave width/layout locally.
- `mapping_list.css:72,90,105` device filter modifies Chip internals; `:504` `.if-chip.if-add-inline__chip--listening` rewrites Chip colors/border; :510 pulse added at consumer. Formalize interactive/selected/listening Chip states. `:367-377` qualifier chip font-size 10px bypasses Chip typography; use a documented compact size if retained. `:270` output-chip width and :537 IconButton flex are layout-only and can stay.
- `mapping_editor.css:1373-1375` `.if-add-palette__item` overrides MenuItem padding/font based on stylesheet source order (comment :1369-1372 explicitly relies on it); add shared MenuItem size/density. Danger class :1386-1389 exists separately from rail `.if-row-menu__item--danger`:635; share semantic menu-item variant.
- `mapping_editor.css:372-428` owns SegmentedControl classes `if-stage__body-strategy` and `...-pill`, although exported from `components/segmented_control.rs`. Move vocabulary/styles to components and rename away from feature-owned BEM. Consumers: change_mode :249/:234/:250, map_to_keyboard :197/:198/:204, map_to_mouse :102/:103/:109. No rail/filter usage of SegmentedControl exists in this scope.
- `bulk_map.css:44-49` and :205-207 reach `.if-field`/`.if-select-wrapper`/`.if-select` for width. These are layout plumbing rather than forbidden restyling, but repeated wrapper/inner selectors indicate a missing full-width/stretch control API.
- `mapping_editor.css:1096` pipeline SortableGap height, :1115 handle top; `mapping_list.css:221` rail gap height and :205 handle halo are documented placement/density differences. Do not blindly delete these in a cosmetic migration; express common density through a supported sortable variable/prop if consolidating.

### M17 [P2] Hard-coded semantic colors and token names

All raw semantic color declaration locations in assigned CSS:

| File | Selector | Lines | Replacement |
|---|---|---|---|
| mapping_editor.css | `.if-editor__offline-banner` | 65,66 | error surface token or color-mix(error,bg); error-derived border |
| mapping_editor.css | `.if-editor__offline-action` | 80 | semantic Button/error border |
| mapping_editor.css | `.if-editor__readout-chip--live` | 699 | live-derived 10% tint |
| mapping_editor.css | `.if-editor__readout-button-pill--live` | 774 | live-derived 10% tint |
| mapping_editor.css | `.if-editor__readout-hat-spoke--live` | 822 | live-derived 10% tint |
| mapping_editor.css | `.if-editor__inactive-hint` | 1036,1037 | control-derived surface/border |
| mapping_editor.css | `.if-add-palette__trigger` | 1287,1288 | shared create variant tokens |
| mapping_editor.css | `.if-add-palette__trigger:hover` | 1303,1304 | same hover variant |
| mapping_editor.css | `.if-add-palette__trigger--louder` | 1313,1314 | same emphasized variant |
| mapping_editor.css | `.if-add-palette__trigger--louder:hover` | 1320,1321 | same emphasized hover variant |
| mapping_editor.css | `.if-stage-menu__item--danger` | 1389 | fallback literal #f25555 is conditional only; canonical danger token should be sufficient |
| bulk_map.css | `.if-bulk-map__kind` | 104 | category/action-derived tint, not unrelated fixed cyan |

`response_curve.css:13` relative `rgb(from var(--color-primary)...)` correctly derives from a token; it is not a hard-coded color issue. `mapping_list.css` has no raw color literals. SVG colors use token classes; pure `fill:none` and geometry values are not token violations.

`mapping_editor.css:1252-1253` uses undefined `--font-size-label` and `--font-weight-medium`; token catalog defines `--text-sm` and `--weight-medium`. These declarations fall back by CSS inheritance, defeating intended gesture label hierarchy.

### M18 [P3] Readout chips/status surfaces duplicate existing primitive styling

`live_readout/in_block.rs:89` predicate chip plus dot; :118-119 dot/pill; `out_block.rs:143` keyboard chip; `in_block.rs:144-154` hat compass is domain presentation and should remain domain layout. CSS `mapping_editor.css:684-717`, :742-775, :859-874 repeats Chip/Badge-like font, pill, border, active/idle behavior. Extend Chip/Badge only for genuinely reusable live/idle status treatment, keep predicate labels/value model and per-output geometry outside. `.if-editor__readout-button-pill--idle` has no distinct rule but falls back correctly; no defect inferred. Reusable status dot would also cover batch button readout `row_readout.rs:59` / `bulk_map.css:178-189`. Avoid promoting every axis bar/hat/signal plot into DS simply because it renders SVG or a div.

### M19 [P2] Motion/typography/radius bypasses are widespread, including one wrong ownership link

`mapping_editor.css:215,320` and `mapping_list.css:516` each hardcode 1100ms ease-in-out capture pulse; editor relies on keyframes declared in mapping_list.css:494. Put capture pulse in a shared component/motion token; all three already have reduced-motion rules (preserve them). `mapping_editor.css:1042` opacity 150ms ease-out and :1192 transform 180ms ease-out bypass canonical motion tokens even though each has local reduced-motion handling. Replace with documented duration/easing vocabulary. `deadzone.css:120-124` uses blanket `!important` reduced-motion overrides inside a consumer; prefer shared component/token handling once needed. Bulk live-bar width/left/right transitions at `bulk_map.css:172-175` are domain signal motion, not decorative chrome; do not report them as definitely slow without runtime profiling.

Complete raw scale declarations are listed in the appendix so migration does not miss callsites. Fixed plot size/stroke radii/axis geometry are intentionally not presented as arbitrary token violations. Main duplication candidates are control chrome and product typography, not custom data visualization geometry.

## Positive / preserved boundaries

Response curve toolbar already uses Tabs/Switch/Button; deadzone toolbar Field/NumberInput/Button; batch metadata Field/Select/Checkbox; add/collision confirmation Button/Badge/Chip/TextInput; empty rail clear actions Button; Delete mapping uses Dialog composition and safer initial Cancel focus; stage actions already uses AnchoredMenu/MenuItem. SortableHandle/Gap/LiveRegion are shared and should stay. Domain instrument SVG paths, grid, handles, thumbnails and readout geometry are purposeful; do not replace them with generic wrappers. No production textarea or raw select declarations occur in this assigned scope.

## Verification limit

Source review only; no runtime screenshots, measured contrast, keyboard execution, native hardware, GUI launches or test runs. Findings on accessibility names, duplicate IDs, raw declarations, selector ownership and hard-coded values are source-verifiable. Runtime rendering/performance or platform acceptance is not claimed.

## Exact file coverage inventory

Every production/support scoped `.rs` file was read for rendered controls/callback surfaces; production render bodies were inspected, test files inventoried as fixtures/regressions, not migrated. The obsolete source_label/keyboard support files still physically exist, whether or not current module exports use them. Tests are grouped below.

| File (`src/frame/`) | Disposition |
|---|---|
| `mapping_editor/content.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/empty_state.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/engine_offline_banner.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/header.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/inactive_hint.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/keyboard.rs` | Checked: nonrendering helper/state/labels/transport (embedded tests excluded) |
| `mapping_editor/live_readout/in_block.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/live_readout/mod.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/live_readout/out_block.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/live_readout/out_chain.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/live_readout/predicate.rs` | Checked: nonrendering helper/state/labels/transport (embedded tests excluded) |
| `mapping_editor/live_readout/value_helpers.rs` | Checked: nonrendering helper/state/labels/transport (embedded tests excluded) |
| `mapping_editor/mapping_issue_copy.rs` | Checked: nonrendering helper/state/labels/transport (embedded tests excluded) |
| `mapping_editor/mapping_issues.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/mod.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/pipeline/add_palette.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/pipeline/dnd.rs` | Checked: nonrendering helper/state/labels/transport (embedded tests excluded) |
| `mapping_editor/pipeline/mod.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/pipeline/stage.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/pipeline/stage_actions_menu.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/pipeline/stage_body/branches.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/pipeline/stage_body/change_mode.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/pipeline/stage_body/conditional.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/pipeline/stage_body/deadzone/mod.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/pipeline/stage_body/deadzone/rendering.rs` | Checked: domain SVG layers/thumbnail; retain geometry locally |
| `mapping_editor/pipeline/stage_body/deadzone/state.rs` | Checked: nonrendering helper/state/labels/transport (embedded tests excluded) |
| `mapping_editor/pipeline/stage_body/deadzone/thumbnail.rs` | Checked: domain SVG layers/thumbnail; retain geometry locally |
| `mapping_editor/pipeline/stage_body/deadzone/toolbar.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/pipeline/stage_body/gesture_body.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/pipeline/stage_body/instruments/bridge.rs` | Checked: nonrendering helper/state/labels/transport (embedded tests excluded) |
| `mapping_editor/pipeline/stage_body/instruments/geometry.rs` | Checked: nonrendering helper/state/labels/transport (embedded tests excluded) |
| `mapping_editor/pipeline/stage_body/instruments/live_axis.rs` | Checked: nonrendering helper/state/labels/transport (embedded tests excluded) |
| `mapping_editor/pipeline/stage_body/instruments/mod.rs` | Checked: nonrendering helper/state/labels/transport (embedded tests excluded) |
| `mapping_editor/pipeline/stage_body/instruments/stage_dispatch.rs` | Checked: nonrendering helper/state/labels/transport (embedded tests excluded) |
| `mapping_editor/pipeline/stage_body/instruments/wiring.rs` | Checked: nonrendering helper/state/labels/transport (embedded tests excluded) |
| `mapping_editor/pipeline/stage_body/invert.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/pipeline/stage_body/map_to_keyboard.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/pipeline/stage_body/map_to_mouse.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/pipeline/stage_body/map_to_vjoy.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/pipeline/stage_body/merge_axis.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/pipeline/stage_body/mod.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/pipeline/stage_body/predicate.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/pipeline/stage_body/press_gesture.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/pipeline/stage_body/response_curve/mod.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/pipeline/stage_body/response_curve/rendering.rs` | Checked: domain SVG layers/thumbnail; retain geometry locally |
| `mapping_editor/pipeline/stage_body/response_curve/state.rs` | Checked: nonrendering helper/state/labels/transport (embedded tests excluded) |
| `mapping_editor/pipeline/stage_body/response_curve/thumbnail.rs` | Checked: domain SVG layers/thumbnail; retain geometry locally |
| `mapping_editor/pipeline/stage_body/response_curve/toolbar.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/pipeline/stage_body/tap_gesture.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/pipeline/stage_header.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_editor/undo_log.rs` | Checked: nonrendering helper/state/labels/transport (embedded tests excluded) |
| `mapping_editor/undo_recap.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_list/add_inline.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_list/add_inline_rendering.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_list/empty.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_list/filter.rs` | Checked: nonrendering helper/state/labels/transport (embedded tests excluded) |
| `mapping_list/group.rs` | Checked: nonrendering helper/state/labels/transport (embedded tests excluded) |
| `mapping_list/keyboard.rs` | Checked: nonrendering helper/state/labels/transport (embedded tests excluded) |
| `mapping_list/mod.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_list/rename_inline.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_list/row.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `mapping_list/source_label.rs` | Checked: nonrendering helper/state/labels/transport (embedded tests excluded) |
| `bulk_map/empty_state.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `bulk_map/labels.rs` | Checked: nonrendering helper/state/labels/transport (embedded tests excluded) |
| `bulk_map/metadata.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `bulk_map/mod.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `bulk_map/row_readout.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |
| `bulk_map/rows.rs` | Checked: render/composition body and supplied callback wiring (embedded tests excluded) |

Test-only files/fixtures inventoried (no production migrations):

- `mapping_editor/`: `capture_events_tests/fixture.rs`, `capture_events_tests/mod.rs`, `capture_events_tests/rail.rs`, `capture_events_tests/rename.rs`, `keyboard_tests.rs`, `live_readout/events_tests.rs`, `pipeline/keyboard_events_tests.rs`, `pipeline/pointer_events_tests.rs`, `pipeline/stage_body/deadzone/tests.rs`, `pipeline/stage_body/instruments/events_tests/document.rs`, `pipeline/stage_body/instruments/events_tests/dom.rs`, `pipeline/stage_body/instruments/events_tests/fixture.rs`, `pipeline/stage_body/instruments/events_tests/mod.rs`, `pipeline/stage_body/instruments/events_tests/undo.rs`, `pipeline/stage_body/response_curve/tests.rs`, `pipeline/tests.rs`, `test_frame.rs`, `test_helpers.rs`, `tests.rs`, `typing_tests.rs`.
- `mapping_list/`: `rename_tests.rs`, `tests.rs`, `workflow_events_tests/document.rs`, `workflow_events_tests/fixture.rs`, `workflow_events_tests/mod.rs`, `workflow_events_tests/reorder.rs`.
- `bulk_map/`: `events_tests/apply.rs`, `events_tests/controls.rs`, `events_tests/mod.rs`, `events_tests/support.rs`, `tests.rs`.

Total physical Rust coverage: 68 production/support files + 31 test/fixture files. CSS coverage: `assets/frame/mapping_editor.css` (1390 lines), `mapping_list.css` (657), `bulk_map.css` (350), `response_curve.css` (146), `deadzone.css` (125).

## Appendix: exact raw scale declarations in assigned CSS

This is an occurrence census for token normalization, not one finding per declaration. Entries on plot SVG text/geometry and 50% circular radius are legitimate visualization shape/dimension choices. Zero resets, none, inherit, percentages and already token-based values are excluded; layout values are listed for review but may stay when domain-specific.

| CSS file | Selector | Declaration location/value |
|---|---|---|
| `mapping_editor.css` | `.if-editor` | :6 `gap: 0` |
| `mapping_editor.css` | `.if-editor__empty` | :16 `gap: 8px` |
| `mapping_editor.css` | `.if-editor__empty` | :18 `padding: 32px` |
| `mapping_editor.css` | `.if-editor__empty-title` | :24 `font-size: 16px` |
| `mapping_editor.css` | `.if-editor__empty-title` | :25 `font-weight: 600` |
| `mapping_editor.css` | `.if-editor__empty-helper` | :31 `font-size: 12px` |
| `mapping_editor.css` | `.if-editor__empty-helper` | :32 `line-height: 18px` |
| `mapping_editor.css` | `.if-editor__kbd` | :39 `font-size: 11px` |
| `mapping_editor.css` | `.if-editor__kbd` | :40 `padding: 2px 6px` |
| `mapping_editor.css` | `.if-editor__kbd` | :41 `border-radius: 4px` |
| `mapping_editor.css` | `.if-editor__offline-banner` | :55 `gap: 12px` |
| `mapping_editor.css` | `.if-editor__offline-banner` | :56 `padding: 8px 16px` |
| `mapping_editor.css` | `.if-editor__offline-text` | :71 `font-size: 12px` |
| `mapping_editor.css` | `.if-editor__offline-action` | :77 `font-size: 12px` |
| `mapping_editor.css` | `.if-editor__offline-action` | :81 `border-radius: 4px` |
| `mapping_editor.css` | `.if-editor__offline-action` | :82 `padding: 4px 10px` |
| `mapping_editor.css` | `.if-editor__header` | :99 `gap: 4px` |
| `mapping_editor.css` | `.if-editor__header` | :100 `padding: 12px 16px` |
| `mapping_editor.css` | `.if-editor__title` | :106 `font-size: 20px` |
| `mapping_editor.css` | `.if-editor__title` | :107 `font-weight: 600` |
| `mapping_editor.css` | `.if-editor__title` | :108 `line-height: 28px` |
| `mapping_editor.css` | `.if-editor__title:focus-visible` | :119 `border-radius: 2px` |
| `mapping_editor.css` | `.if-editor__title-input` | :127 `font-size: 20px` |
| `mapping_editor.css` | `.if-editor__title-input` | :128 `font-weight: 600` |
| `mapping_editor.css` | `.if-editor__title-input` | :129 `line-height: 28px` |
| `mapping_editor.css` | `.if-editor__title-input` | :131 `padding: 0` |
| `mapping_editor.css` | `.if-editor__title-input:focus-visible` | :140 `border-radius: 2px` |
| `mapping_editor.css` | `.if-editor__subtitle` | :152 `gap: 8px` |
| `mapping_editor.css` | `.if-editor__subtitle` | :154 `font-size: 12px` |
| `mapping_editor.css` | `.if-editor__subtitle` | :155 `line-height: 18px` |
| `mapping_editor.css` | `.if-rebind-composite` | :192 `font-size: 12px` |
| `mapping_editor.css` | `.if-rebind-composite` | :193 `line-height: 18px` |
| `mapping_editor.css` | `.if-rebind-composite__label` | :197 `padding: 4px 10px` |
| `mapping_editor.css` | `.if-rebind-composite__listening` | :204 `gap: 6px` |
| `mapping_editor.css` | `.if-rebind-composite__listening` | :205 `padding: 4px 10px` |
| `mapping_editor.css` | `.if-rebind-composite__listening::before` | :213 `border-radius: 50%` |
| `mapping_editor.css` | `.if-rebind-composite__listening::before` | :215 `animation: if-add-pulse-dot 1100ms ease-in-out infinite` |
| `mapping_editor.css` | `.if-rebind-composite__action` | :230 `padding: 4px 10px` |
| `mapping_editor.css` | `.if-key-capture` | :273 `gap: 6px` |
| `mapping_editor.css` | `.if-key-capture__surface` | :284 `padding: 0 12px` |
| `mapping_editor.css` | `.if-key-capture__surface` | :291 `font-size: 12px` |
| `mapping_editor.css` | `.if-key-capture__surface` | :292 `line-height: 1` |
| `mapping_editor.css` | `.if-key-capture__surface` | :293 `letter-spacing: 0` |
| `mapping_editor.css` | `.if-key-capture__surface.is-listening::before` | :318 `border-radius: 50%` |
| `mapping_editor.css` | `.if-key-capture__surface.is-listening::before` | :320 `animation: if-add-pulse-dot 1100ms ease-in-out infinite` |
| `mapping_editor.css` | `.if-key-capture__hint` | :325 `font-size: 12px` |
| `mapping_editor.css` | `.if-stage__body-merge-axis, .if-stage__body-vjoy, .if-stage__body-keyboard, .if-stage__body-mouse, .if-stage__body-change-mode` | :352 `column-gap: 16px` |
| `mapping_editor.css` | `.if-stage__body-merge-axis, .if-stage__body-vjoy, .if-stage__body-keyboard, .if-stage__body-mouse, .if-stage__body-change-mode` | :353 `row-gap: 8px` |
| `mapping_editor.css` | `.if-stage__body-label` | :364 `font-size: 12px` |
| `mapping_editor.css` | `.if-stage__body-strategy` | :375 `gap: 4px` |
| `mapping_editor.css` | `.if-stage__body-strategy` | :376 `padding: 2px` |
| `mapping_editor.css` | `.if-stage__body-strategy-pill` | :390 `padding: 0 12px` |
| `mapping_editor.css` | `.if-stage__body-strategy-pill` | :396 `font-size: 13px` |
| `mapping_editor.css` | `.if-predicate` | :445 `gap: 8px` |
| `mapping_editor.css` | `.if-predicate` | :446 `padding: 8px 0` |
| `mapping_editor.css` | `.if-predicate__input-row` | :452 `gap: 8px` |
| `mapping_editor.css` | `.if-predicate__range-row` | :458 `gap: 8px` |
| `mapping_editor.css` | `.if-predicate__range-label` | :463 `font-size: 12px` |
| `mapping_editor.css` | `.if-predicate__nested-list` | :472 `gap: 6px` |
| `mapping_editor.css` | `.if-predicate__nested-list` | :473 `padding-left: 16px` |
| `mapping_editor.css` | `.if-predicate__nested-empty` | :484 `font-size: 12px` |
| `mapping_editor.css` | `.if-hat-direction-grid` | :494 `gap: 4px` |
| `mapping_editor.css` | `.if-hat-direction-grid__cell` | :501 `gap: 4px` |
| `mapping_editor.css` | `.if-hat-direction-grid__cell` | :503 `font-size: 12px` |
| `mapping_editor.css` | `.if-editor__readout` | :518 `gap: 4px` |
| `mapping_editor.css` | `.if-editor__readout` | :519 `padding: 8px 16px` |
| `mapping_editor.css` | `.if-editor__readout-group` | :533 `column-gap: 12px` |
| `mapping_editor.css` | `.if-editor__readout-group` | :534 `row-gap: 8px` |
| `mapping_editor.css` | `.if-editor__readout-label` | :545 `font-size: 11px` |
| `mapping_editor.css` | `.if-editor__readout-label` | :547 `font-weight: 500` |
| `mapping_editor.css` | `.if-editor__readout-tag` | :554 `font-size: 11px` |
| `mapping_editor.css` | `.if-editor__readout-bar` | :570 `border-radius: 2px` |
| `mapping_editor.css` | `.if-editor__readout-fill` | :600 `border-radius: 2px` |
| `mapping_editor.css` | `.if-editor__readout-pct` | :606 `font-size: 12px` |
| `mapping_editor.css` | `.if-editor__readout-divider-label` | :639 `font-size: 10px` |
| `mapping_editor.css` | `.if-editor__readout-section` | :649 `gap: 4px` |
| `mapping_editor.css` | `.if-editor__readout-section-label` | :654 `font-size: 11px` |
| `mapping_editor.css` | `.if-editor__readout-section-label` | :655 `font-weight: 500` |
| `mapping_editor.css` | `.if-editor__readout-section-label` | :656 `line-height: 1.45` |
| `mapping_editor.css` | `.if-editor__readout-section-label` | :658 `letter-spacing: 0.04em` |
| `mapping_editor.css` | `.if-editor__readout-predicate-label` | :670 `font-size: 11px` |
| `mapping_editor.css` | `.if-editor__readout-predicate-label` | :672 `font-weight: 500` |
| `mapping_editor.css` | `.if-editor__readout-chips` | :680 `gap: 6px` |
| `mapping_editor.css` | `.if-editor__readout-chip` | :687 `gap: 4px` |
| `mapping_editor.css` | `.if-editor__readout-chip` | :689 `font-size: 11px` |
| `mapping_editor.css` | `.if-editor__readout-chip` | :690 `padding: 2px 8px` |
| `mapping_editor.css` | `.if-editor__readout-chip` | :691 `border-radius: 999px` |
| `mapping_editor.css` | `.if-editor__readout-chip-dot` | :706 `border-radius: 50%` |
| `mapping_editor.css` | `.if-editor__readout-row--hat .if-editor__readout-hat-glyph` | :725 `font-size: 16px` |
| `mapping_editor.css` | `.if-editor__readout-button-cell` | :738 `gap: 8px` |
| `mapping_editor.css` | `.if-editor__readout-button-dot` | :745 `border-radius: 50%` |
| `mapping_editor.css` | `.if-editor__readout-button-pill` | :761 `padding: 2px 8px` |
| `mapping_editor.css` | `.if-editor__readout-button-pill` | :762 `border-radius: 999px` |
| `mapping_editor.css` | `.if-editor__readout-button-pill` | :765 `font-size: 11px` |
| `mapping_editor.css` | `.if-editor__readout-button-pill` | :766 `line-height: 1.2` |
| `mapping_editor.css` | `.if-editor__readout-hat-compass` | :797 `column-gap: 4px` |
| `mapping_editor.css` | `.if-editor__readout-hat-compass` | :798 `row-gap: 2px` |
| `mapping_editor.css` | `.if-editor__readout-hat-spoke` | :810 `border-radius: 2px` |
| `mapping_editor.css` | `.if-editor__readout-hat-spoke` | :813 `font-size: 10px` |
| `mapping_editor.css` | `.if-editor__readout-hat-spoke` | :814 `line-height: 1` |
| `mapping_editor.css` | `.if-editor__readout-hat-state` | :829 `padding-left: 4px` |
| `mapping_editor.css` | `.if-editor__readout-hat-state` | :831 `font-size: 11px` |
| `mapping_editor.css` | `.if-editor__readout-kb-chip` | :863 `font-size: 11px` |
| `mapping_editor.css` | `.if-editor__readout-kb-chip` | :864 `padding: 2px 8px` |
| `mapping_editor.css` | `.if-editor__readout-kb-chip` | :865 `border-radius: 999px` |
| `mapping_editor.css` | `.if-editor__readout-chevron` | :909 `font-size: 12px` |
| `mapping_editor.css` | `.if-editor__readout-chevron` | :910 `padding: 0` |
| `mapping_editor.css` | `.if-editor__readout-chevron` | :911 `border-radius: 2px` |
| `mapping_editor.css` | `.if-editor__readout-chain` | :936 `column-gap: 12px` |
| `mapping_editor.css` | `.if-editor__readout-chain` | :937 `row-gap: 2px` |
| `mapping_editor.css` | `.if-editor__readout-chain` | :940 `padding: 6px 0` |
| `mapping_editor.css` | `--if-editor__readout-pct-col .if-editor__readout-chain-row` | :946 `font-size: 11px` |
| `mapping_editor.css` | `.if-editor__readout-chain-step` | :960 `padding-left: 8px` |
| `mapping_editor.css` | `.if-editor__readout-chain-step` | :962 `font-size: 10px` |
| `mapping_editor.css` | `.if-editor__readout-chain-step` | :964 `letter-spacing: 0.04em` |
| `mapping_editor.css` | `.if-editor__readout-chain-step` | :966 `font-weight: 500` |
| `mapping_editor.css` | `.if-editor__readout-chain-bar` | :988 `border-radius: 2px` |
| `mapping_editor.css` | `.if-editor__readout-chain-fill` | :1007 `border-radius: 2px` |
| `mapping_editor.css` | `.if-editor__readout-chain-pct` | :1013 `font-size: 10px` |
| `mapping_editor.css` | `.if-editor__readout-chain-outcome` | :1022 `font-size: 10px` |
| `mapping_editor.css` | `.if-editor__inactive-hint` | :1035 `padding: 8px 12px` |
| `mapping_editor.css` | `.if-editor__inactive-hint` | :1038 `border-radius: 6px` |
| `mapping_editor.css` | `.if-editor__inactive-hint` | :1040 `font-size: 12px` |
| `mapping_editor.css` | `.if-editor__inactive-hint` | :1042 `transition: opacity 150ms ease-out` |
| `mapping_editor.css` | `.if-editor__footer` | :1054 `padding: 6px 16px` |
| `mapping_editor.css` | `.if-editor__footer` | :1057 `font-size: 11px` |
| `mapping_editor.css` | `.if-editor__footer` | :1061 `gap: 4px` |
| `mapping_editor.css` | `.if-pipeline` | :1072 `padding: 0px 8px 16px` |
| `mapping_editor.css` | `.if-pipeline` | :1080 `gap: 0` |
| `mapping_editor.css` | `.if-pipeline--empty` | :1084 `padding: 0px 8px 16px` |
| `mapping_editor.css` | `.if-stage` | :1106 `border-radius: 6px` |
| `mapping_editor.css` | `.if-stage__header` | :1141 `gap: 12px` |
| `mapping_editor.css` | `.if-stage__header` | :1142 `padding: 8px 12px 8px 22px` |
| `mapping_editor.css` | `.if-stage__title` | :1152 `font-size: 12px` |
| `mapping_editor.css` | `.if-stage__title` | :1153 `font-weight: 500` |
| `mapping_editor.css` | `.if-stage__title` | :1154 `line-height: 16px` |
| `mapping_editor.css` | `.if-stage__summary` | :1166 `font-size: 12px` |
| `mapping_editor.css` | `.if-stage__chevron` | :1192 `transition: transform 180ms ease-out` |
| `mapping_editor.css` | `.if-stage__body` | :1211 `padding: 8px 12px 8px 22px` |
| `mapping_editor.css` | `.if-stage__conditional-body` | :1222 `gap: 8px` |
| `mapping_editor.css` | `.if-stage__branch-label` | :1258 `font-size: 11px` |
| `mapping_editor.css` | `.if-stage__branch-label` | :1260 `font-weight: 500` |
| `mapping_editor.css` | `.if-add-palette__trigger` | :1285 `gap: 6px` |
| `mapping_editor.css` | `.if-add-palette__trigger` | :1289 `border-radius: 6px` |
| `mapping_editor.css` | `.if-add-palette__trigger` | :1290 `padding: 6px 12px` |
| `mapping_editor.css` | `.if-add-palette__trigger` | :1293 `font-size: 12px` |
| `mapping_editor.css` | `.if-add-palette__trigger` | :1294 `font-weight: 500` |
| `mapping_editor.css` | `.if-add-palette__trigger` | :1295 `line-height: 1` |
| `mapping_editor.css` | `.if-add-palette__trigger--louder` | :1315 `padding: 10px 12px` |
| `mapping_editor.css` | `.if-add-palette__trigger--louder` | :1316 `line-height: 1.45` |
| `mapping_editor.css` | `.if-add-palette__section` | :1333 `padding: 4px` |
| `mapping_editor.css` | `.if-add-palette__section-title` | :1338 `font-size: 11px` |
| `mapping_editor.css` | `.if-add-palette__section-title` | :1340 `font-weight: 500` |
| `mapping_editor.css` | `.if-add-palette__section-title` | :1342 `padding: 2px 8px` |
| `mapping_editor.css` | `.if-add-palette__item` | :1374 `padding: 4px 8px` |
| `mapping_editor.css` | `.if-add-palette__item` | :1375 `font-size: 12px` |
| `mapping_list.css` | `.if-rail__device-chip` | :61 `padding: 0` |
| `mapping_list.css` | `.if-rail__group-header` | :140 `font-size: 10px` |
| `mapping_list.css` | `.if-rail__group-header` | :141 `font-weight: 600` |
| `mapping_list.css` | `.if-rail__group-header` | :142 `letter-spacing: 0.08em` |
| `mapping_list.css` | `.if-rail__group-header__count` | :149 `font-weight: 500` |
| `mapping_list.css` | `.if-rail__group-header__count` | :156 `letter-spacing: 0` |
| `mapping_list.css` | `.if-row:hover .if-sortable-handle` | :209 `padding: 1px 2px` |
| `mapping_list.css` | `color-mix(in srgb, var(--color-live) calc(var(--row-live-intensity, 0) * 100%), .if-row.is-active .if-row__name` | :251 `font-weight: 700` |
| `mapping_list.css` | `.if-row__source-arrow` | :267 `font-size: 11px` |
| `mapping_list.css` | `.if-row__name` | :285 `font-size: 12px` |
| `mapping_list.css` | `.if-row__name` | :286 `font-weight: 500` |
| `mapping_list.css` | `.if-row__name` | :288 `line-height: 1.3` |
| `mapping_list.css` | `.if-row__name--unnamed` | :298 `font-weight: 400` |
| `mapping_list.css` | `.if-row__source` | :310 `gap: 1px` |
| `mapping_list.css` | `.if-row__source` | :311 `font-size: 11px` |
| `mapping_list.css` | `.if-row__source` | :313 `line-height: 1.4` |
| `mapping_list.css` | `.if-row__source-qualifiers` | :363 `font-size: 10px` |
| `mapping_list.css` | `.if-row__qualifier` | :377 `font-size: 10px` |
| `mapping_list.css` | `.if-row__qualifier .if-row__qualifier-glyph` | :383 `font-weight: 600` |
| `mapping_list.css` | `.if-rail-empty__title` | :407 `font-size: 18px` |
| `mapping_list.css` | `.if-rail-empty__title` | :408 `font-weight: 600` |
| `mapping_list.css` | `.if-rail-empty__helper` | :413 `font-size: 12px` |
| `mapping_list.css` | `.if-add-inline__dashed-row` | :444 `font-size: 12px` |
| `mapping_list.css` | `.if-add-inline__dashed-row` | :445 `font-weight: 500` |
| `mapping_list.css` | `.if-add-inline--pad input` | :474 `font-size: 12px` |
| `mapping_list.css` | `.if-chip.if-add-inline__chip--listening::before` | :514 `border-radius: 50%` |
| `mapping_list.css` | `.if-chip.if-add-inline__chip--listening::before` | :516 `animation: if-add-pulse-dot 1100ms ease-in-out infinite` |
| `mapping_list.css` | `.if-add-inline__device` | :530 `font-size: 12px` |
| `mapping_list.css` | `.if-add-inline__collision-text` | :568 `font-size: 12px` |
| `mapping_list.css` | `.if-row-rename` | :573 `font-size: 12px` |
| `mapping_list.css` | `.if-row-menu__item` | :613 `font-size: 12px` |
| `bulk_map.css` | `.if-bulk-map__kind` | :103 `border-radius: 3px` |
| `bulk_map.css` | `.if-bulk-map__live--axis` | :161 `border-radius: 2px` |
| `bulk_map.css` | `.if-bulk-map__live-bar` | :169 `border-radius: 2px` |
| `bulk_map.css` | `width var(--duration-fast) var(--easing-fast), left var(--duration-fast) var(--easing-fast), .if-bulk-map__live--button` | :181 `border-radius: 50%` |
| `bulk_map.css` | `.if-bulk-map__chip` | :226 `border-radius: 3px` |
| `response_curve.css` | `.if-curve__tick-label` | :133 `font-size: 0.05` |
| `deadzone.css` | `.if-deadzone__tick-label` | :116 `font-size: 0.05` |

Prioritized command sequence: `$impeccable harden` for M1/M13/M14/M15; `$impeccable distill` for component/API/style ownership; `$impeccable colorize` for M17; `$impeccable polish` after these scoped migrations.

## Additional dead / redundant selector evidence

[P3] Confirmed no production source class occurrence for `mapping_list.css:296` `.if-row__name--unnamed` (row.rs only renders a name when Some), `mapping_list.css:584` `.if-add-inline--captured input:focus-visible` (shared `--pad` is rendered), `deadzone.css:23` `.if-deadzone__toolbar-spacer` (toolbar has no spacer), or `mapping_editor.css:719/:723/:730` `.if-editor__readout-hat-glyph` rules (current hat body uses compass/spokes/state instead). The last obsolete CSS class is still asserted by an embedded stylesheet test `live_readout/in_block.rs:427`; a removal must update that stale CSS assertion rather than preserve a dead selector to satisfy it. `.if-stage-menu` invocation at stage_actions_menu.rs:86 is explicitly a no-op hook (comment mapping_editor.css:1381); delete the unused hook if no extension is needed, retain only semantic item styling.

[P3] `mapping_editor.css:424-428` repeats exactly the selected control colors from :411-414 and disabled opacity from :417-418 for the combined pressed+disabled selector; the preceding rules already compose the same result. Consumer density/geometry rules and purposeful inherited defaults are not otherwise labeled dead solely because they are small.

Generic-handler completeness: `mapping_list/add_inline_rendering.rs:113-115` pad key bubbling and `mapping_list/rename_inline.rs:93-96` key/focus bubbling are event scopes around real fields, not separate actionable controls; no generic wrapper component is warranted. `stage.rs:163-167` sortable li contextmenu is paired with shared stage/navigation behavior. The actionable generic row/title/menu/plot surfaces are enumerated above. Axis fills, hat spokes, SVG anchors and live chips without handlers remain presentation.
