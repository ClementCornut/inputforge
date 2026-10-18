# GUI design-system audit

Current remediation: [design-system adoption and validation](gui-design-system-adoption.md). The findings below remain the historical audit snapshot.

Source audit: 2026-10-18. Baseline: `242bd6114e555a350d564da1a940134dc54823d2`.

The GUI does not yet satisfy the rule that interactive controls come from the design system. The three audit partitions found **42 native button/input/select declaration sites outside `src/components`**, plus a native disclosure and several custom interactive surfaces. Loop declarations count once; rendered device, axis, direction, mode and batch rows can create many instances. Existing shared components also receive private visual overrides or lack attributes needed for faithful adoption.

This change reorganizes the existing design system. The consumer migrations, API extensions and new primitives below are a prioritized backlog, not implemented fixes.

## Read this first

1. **Fix the mapping rail menu.** Its duplicate-to-mode submenu opens through hover on a nonfocusable element. Reuse the shared overlay/menu mechanics and add keyboard-operable submenu support.
2. **Fix names, associations and duplicate IDs.** Several fields have unassociated visible labels; the mapping/mode rename fields are unnamed; every deadzone toolbar repeats the same four IDs. Settings helper/error attributes currently land on a wrapper instead of its control.
3. **Extend shared APIs before replacing controls.** Controller number fields intentionally deliver raw strings on native change events. Parsed/clamped numeric components are not equivalent. Disclosure, toggle, capture and menu-trigger controls also need attributes/events the current APIs omit.
4. **Move reusable visual states out of screen CSS.** Segmented controls, capture fields, create slots, selected rows, inline feedback and icon menu triggers currently rely on feature-owned chrome.
5. **Delete obsolete rules after migrations.** Keep domain layouts, plotting geometry, app callbacks, validation and draft retention in their present owners.

P1 means a source-demonstrable user/accessibility problem; P2 means a component/API/ownership problem; P3 means cleanup. These are engineering priorities, not a measured accessibility conformance score. Placeholder-only fields are listed as missing explicit, persistent labels; no computed accessibility-tree name is claimed for them.

## Complete evidence and coverage

The linked partitions include every native-control site, affected CSS selectors and lines, replacement/API requirements, behavior constraints, and per-file coverage. Their line numbers and component paths refer to the baseline snapshot; Rust imports and component locations change during the reorganization. Consumer rendering and frame/toast CSS remain the audit targets. Use the final component catalogue below to resolve moved primitive paths.

| Partition | Source scope | Stylesheets | Native control declarations | Evidence |
|---|---|---|---:|---|
| Shell and management | App shell, layout, top bar, modes, profiles, snapshots, settings | `layout`, `top_bar`, `profiles`, `settings_panel`, `empty_state` | 12 | [Shell findings and complete file inventory](gui-design-system-audit/shell.md) |
| Mapping | Mapping rail/editor, pipeline bodies, batch editor, instrument surfaces | `mapping_editor`, `mapping_list`, `bulk_map`, `response_curve`, `deadzone` | 23 | [Mapping findings and CSS declaration appendix](gui-design-system-audit/mapping.md) |
| Devices and shared surfaces | Panel slot, controller settings, patterns, notifications, examples, root contracts | `panel_slot`, toast | 7, plus one disclosure | [Shared findings and complete file inventory](gui-design-system-audit/shared.md) |
| Design system and global CSS | Component APIs, component stylesheets, theme mounts, global reset, all token files | 26 component CSS files, 7 token files, `global.css` | Native elements inside primitives are expected | Catalogue and shared gaps below |

The baseline inventory contains **354 tracked GUI files: 263 Rust, 46 CSS, 32 SVG icons, four fonts, one JS asset, five JS test fixtures, two Markdown files and one manifest**. All 46 stylesheets are assigned above. Renderer files were read; contracts/helpers and test-only fixtures were classified separately. Vendored icon paths and font binaries are not alternate controls and were not visually reviewed. Cartog stats/map supplied the code inventory; stalled outline calls were abandoned in favor of targeted reads of the mapped files.

## What to fix using existing components

| Work | Consumers | Replacement and caveat |
|---|---|---|
| Back/recovery/add actions | New/Open profile back buttons; offline recovery; add-mode plus; toast dismiss | `Button` / `IconButton`; preserve existing labels, focus and dispatch. Choose shared size instead of restoring private button chrome. |
| Profile library source select | `frame/profiles/new_profile_submode.rs` | `Select` / `SelectOption`; preserve empty option and selected key. It currently applies text-input classes to a native select. |
| Controller and predicate checkboxes | Controller selection, virtual axes, eight hat directions | `Checkbox` plus stable label association. Its existing outer label means callers must not nest labels. |
| Device display name and mapping title | Device inspector and editor header | `TextInput` plus `Field` / `Label` or an explicit name; retain app text ownership and `initial_value`, keyboard behavior and failed-save drafts. |
| Profile/settings forms | New/Open/rename forms; six settings rows | `Field` / `Label` with stable IDs and a horizontal layout variant; extend description/error metadata on the actual input first. |
| Rail context menu | Rename, Duplicate, duplicate-to-mode, Move up/down, Delete | `AnchoredMenu` / `MenuItem`; submenu support is missing. Preserve app acknowledgement so rejected actions leave the menu open. |
| Notification count and live chips | Toast count; repeated readout pills | `Badge` / `Chip` with explicit count/live treatment where necessary. Keep signal geometry and notification queue policy outside primitives. |
| Destructive mode confirmation | `frame/top_bar/mode_tabs/delete_dialog.rs` | Existing `ButtonVariant::Danger`; current Secondary differs from other destructive confirmations. |

## What to extend in the design system

These are concrete requirements from current consumers, not a request for unrestricted attribute bags or a new abstraction layer.

| Existing family | Smallest required capability | Why |
|---|---|---|
| `Button`, `IconButton` | Explicit pressed/expanded/controls/title/id support where used; contextual owned icon labels; agreed strip/row/disclosure presentation | Tools, engine state, profile selector, snapshot drawer, readout and stage expansion currently recreate native buttons. Preserve native semantics and avoid nested buttons. |
| `TextInput` | Accessible name, invalid/error metadata, inline/title treatment and required data/focus hooks | Header/inspector manual text-owner composition and compact editors cannot all migrate faithfully today. `invalid` currently only adds a class, not `aria-invalid`. |
| Numeric inputs | A native raw-change numeric presentation; naming/description/error attributes; intentional size behavior | Controller fields require raw native-change payloads; existing numeric commits parse/clamp on Enter/blur. `NumberInput` emits small/medium/large classes with no matching size rules in its stylesheet. |
| `Checkbox`, `Switch`, `Select`, `Slider` | Consistent explicit label/description/error forwarding; Checkbox label content or documented external-label composition | Current consumer labels and dynamic names cannot always be carried through. Keep Checkbox's indeterminate behavior and disabled fieldset semantics. |
| `Field` | Horizontal/wrapping settings layout; actual helper/error IDs and a documented control metadata contract | Replacing SettingsFieldRow's wrapper alone will not repair its ARIA wiring. Workflow errors are not automatically invalid input. |
| `MenuTrigger`, `MenuItem` | Icon/create-slot presentation; density; danger tone; nested-menu semantics | Existing trigger/item CSS is overridden in profiles and the pipeline; the rail separately implements its own menu. |
| `TabsList`, `TabButton` | Compact strip density, scroll/truncate presentation | Mode-strip CSS overrides shared tab geometry. Retain roving focus, running-state indication and keyboard/context-menu handlers. |
| `SegmentedControl` | Component-owned CSS/BEM names | Its reusable Rust code currently hard-codes `if-stage__body-strategy` classes styled in `assets/frame/mapping_editor.css:372-428`. Folder moves alone do not fix that ownership. |
| `Chip`, `Badge`, `Separator` | Interactive chip as a separate explicit control; live/count presentation where repeated; decorative/strong separator | Avoid turning presentational spans into actions or every layout seam into a semantic separator. |
| `Tooltip` | Stable description ID and a supported trigger association | Its current `role=tooltip` bubble has no ID/description linkage. Hover/focus appearance alone does not establish that association. |

## What to create or extract

Prefer composing the existing primitives. Each entry has real current consumers; several can be implemented as variants of the same small primitive rather than separate component families.

| Presentation contract | Current consumers | Boundary |
|---|---|---|
| Native `RadioGroup` / `RadioOption` | Four New Profile source options | Keep native radio semantics. Selected source details must be outside the radio label; the current label also contains a select/file-picker button. |
| Selectable row surface | Device ledger and mapping rail | Share selected/hover/focus chrome. A mapping row contains editing/drag controls, so it cannot become a native button wrapping those controls. Keep roving-focus and domain callbacks explicit. |
| Create-slot action | Add mapping, Add stage, New profile row | Share dashed/compact/emphasized action presentation and compatible menu-trigger styling. |
| Toggle/filter chip | Device filter and batch Replace toggle | Own pressed state and shared chip/button styling; ordinary batch group actions can simply use Button. |
| Rebind/capture field and physical-key capture button | Header, merge-axis, predicate; key-mapping and add-palette capture | Share display/listening/unbound chrome, pulse and announcements. Capture lifetimes, filters and command validation stay in existing app bindings. |
| Inline editor/feedback | Mapping/mode/profile renames and inline additions; repeated alerts | Own field names and visible feedback. Do not merge distinct app workflows into a generic editor controller. |
| Navigation/toggle/status action presentation | Primary navigation, tool toggles, engine pill | A few explicit variants/compositions can share chrome while preserving navigation, pressed and status semantics. Do not substitute role=tab for navigation. |
| Native `Disclosure` / shared disclosure trigger | Advanced axis settings; stage and snapshot expansion | A minimal details/summary wrapper suffices for advanced settings. Other consumers retain their app-controlled state and full-width hit targets. |
| Empty-state/readout caption | Workspace empty state, profile filter empty state, empty snapshots | Share recurring typography/content slots. Screen sizing and domain copy remain local. |
| Optional plot interaction frame | Curve and deadzone instruments | Only common focus/interaction surface, name/instructions/value-description contract. SVG geometry and domain hit testing stay local; no generic Button wrappers around graph handles. |

The toast renderer is already one shared notification pattern. Reuse IconButton/Badge there first; if moving its surface into `components/feedback`, extract presentation only and leave queue/timer/interaction policy in its current owner.

## CSS to simplify

The evidence partitions enumerate all affected selectors, including raw scale declarations. Literal dimensions are not automatically violations: viewport sizing, hairlines, plot coordinates and domain geometry can be intentional.

- **Remove cloned control chrome during migration:** top-bar button families; rebind/key capture; row menu; create-slot and filter/batch chips; title/inspector inputs; settings fields; toast dismiss/count. Keep structural width, grid placement, overflow and domain layout in feature CSS.
- **Stop reaching into component internals:** profile/menu/icon action overrides; mode-tab padding; pipeline MenuItem density; mapping input resets; broad `.if-controller-session input[type="number"]` and `label` rules. The broad controller rules would otherwise restyle nested DS controls after migration.
- **Fix seven undefined token uses:** `profiles.css:122` uses `--leading-normal`; `:145`, `:152`, `:219`, `:516` use `--leading-snug`; `mapping_editor.css:1252-1253` use `--font-size-label` / `--font-weight-medium`. Use canonical `--leading-base` / `--leading-tight`, `--text-sm`, `--weight-medium` as appropriate.
- **Centralize semantic tints and motion:** mapping editor offline/live/inactive/create treatments and batch category tint bypass colors; editor capture pulse depends on keyframes declared in mapping-list CSS. Shared capture styling should own that animation. Preserve reduced-motion handling.
- **Use existing scale values where equivalent:** panel-session 4/8px spacing and 12px type; profile literal 500/600 weights; Chip's 4px gap, 11px size, 500 weight and 1.15 leading. Keep deliberate 1px borders/checkmark geometry. Shared icon/button geometry can gain semantic size tokens when consolidating repeated sizes, not aliases for every literal.
- **Reuse elevation and stacking vocabulary:** toast repeats `--shadow-1` exactly; tooltip/menu still hard-code 1000/1001 while only toast has a named z token. Introduce a small semantic overlay scale when consolidating those surfaces; no runtime stacking failure is asserted here.
- **Delete confirmed obsolete selectors and their obsolete CSS-only assertions:** old profile new-form/path/menu-wrapper hooks; old mapping unnamed/captured modifiers; deadzone toolbar spacer; obsolete hat-glyph rules; empty profile-name-value rule; repeated snapshot-time/action definitions and redundant pressed+disabled segmented rules. Mapping evidence names the stale hat-glyph stylesheet assertion that must change with removal.
- **Split normal and floating errors:** `.if-mode-tab__error` is used in inline editors, menus, dialogs and shell context but always gets absolute positioning. A shared normal-flow feedback surface plus a deliberate inline-editor variant is simpler and avoids inheriting the wrong placement.

## Design-system organization implemented

Forty Rust files and 26 CSS files were moved into seven groups. The root facade stays in place, and seven small private group modules describe the source folders. The Rust source and matching CSS are grouped by responsibility. Existing component behavior, visual declarations, stylesheet order and app ownership remain the same.

| Folder | Existing components |
|---|---|
| [`actions`](../crates/inputforge-gui-dx/src/components/actions/) | Button, IconButton |
| [`inputs`](../crates/inputforge-gui-dx/src/components/inputs/) | TextInput, NumberInput, IntegerInput, Select, Slider, Checkbox, Switch, SegmentedControl |
| [`layout`](../crates/inputforge-gui-dx/src/components/layout/) | Card, Drawer, Field, Label, Stack, Cluster, Inset, Separator |
| [`navigation`](../crates/inputforge-gui-dx/src/components/navigation/) | Tabs, TabsRoot, TabsList, TabButton, menu family |
| [`feedback`](../crates/inputforge-gui-dx/src/components/feedback/) | Badge, Chip, Icon, Spinner, StatusBar, Tooltip |
| [`overlays`](../crates/inputforge-gui-dx/src/components/overlays/) | Dialog family, Portal, ClickAwayListener |
| [`interaction`](../crates/inputforge-gui-dx/src/components/interaction/) | Sortable family |

Consumers use the flat [`components::{Button, ...}` facade](../crates/inputforge-gui-dx/src/components/mod.rs); sortable retains its public namespace. Internal group/leaf modules do not introduce duplicate public item paths. Assets follow the same groups; sortable JS behavior is unchanged. No new component behavior or dependencies are introduced. The largest production component source is 165 physical lines including comments, excluding its test module; the facade is 118 production lines. All files meet the global 200-line baseline without counting tests. Tests moved with their components and retain their existing behavior.

## Recommended migration order

1. Add the missing naming/metadata and native numeric-change contracts, plus meaningful regression tests. Repair fixed deadzone IDs, settings associations, missing field names and radio-label composition.
2. Adopt shared menus for the mapping rail, including keyboard-accessible duplicate-to-mode behavior and rejected-action retention.
3. Add shared row/create/toggle/disclosure/capture presentations; migrate every native site in the three occurrence inventories.
4. Move shared visual rules into their component CSS, consolidate variants/tokens, then remove dead/duplicate feature CSS and update obsolete assertions.
5. Expand the component gallery to cover new variants and label its currently unnamed/placeholder-only input demos. Validate keyboard/focus, contrast, sizing and actual desktop rendering after each cohesive migration.

Do not change the engine, profile persistence, command history, capture cadence, validation, failure retention or source/target identity checks as part of these presentation migrations.

## Verification and manual acceptance

- Baseline `cargo test -p inputforge-gui-dx --all-targets`: **607 passed**, no failures.
- Post-reorganization `cargo test -p inputforge-gui-dx --all-targets`: **607 passed**, no failures.
- Final unchanged `bash .githooks/pre-commit`: **passed**, including `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings`. The gate caught and was rerun after correcting the new module-documentation markup and renaming `layout::layout` to `layout::spacing`; no lint suppressions were added.
- After the final private module rename, `cargo test -p inputforge-gui-dx --lib components::`: **56 passed**, no failures.
- All 26 moved CSS files retain identical declarations; all 37 stylesheet mounts retain their order. Asset/include references resolve, no files are staged, and an independent source review found no issues.
- Source/CSS audit: complete across the assigned inventories. No GUI was launched and no visual, assistive-technology, hardware, Windows or native Wayland acceptance is claimed.

Manual acceptance for the reorganization: open the component gallery with `make gallery` and the normal editor with `dx run -p inputforge-app`; confirm assets load, widgets retain styling, menus/dialogs/portals open and reopen, tab focus works and sortable rows retain pointer/keyboard behavior.

For subsequent migrations, additionally exercise invalid raw controller number changes, disabled driver layouts, alias Unicode typing with rejected Save/Enter and Escape cancel, rename failure retention, keyboard-only rail submenus near viewport edges, simultaneous deadzone stages and label focus, radio source details, toast hover/focus pause and dismiss, and reduced motion.
