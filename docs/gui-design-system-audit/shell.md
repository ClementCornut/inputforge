# Shell, profiles, settings: design-system consumer audit

Read-only source snapshot, 2026-10-18. Paths below are relative to `crates/inputforge-gui-dx/`. No source edits, tests, GUI launch, or Git operations. Cartog stats confirmed an index (747 files); Cartog map identified the exact production and test files. Per-file outline requests stalled and were terminated; actual render bodies and corresponding CSS were inspected directly. Findings refer to consumer paths, before forthcoming component moves.

Severity: P1 = source-proven accessibility/user issue; P2 = component consistency or behavior risk; P3 = cleanup. Browser appearance, platform accessibility trees, and live interaction are unverified.

## Raw control inventory: every occurrence in this scope

12 raw control declarations (some expand into multiple controls). There are no raw textarea elements or generic div/span click activation surfaces in the audited production bodies. Keyboard/focus listeners on inline editor wrappers are coordination around a real TextInput, not fake buttons.

| Consumer occurrence | CSS source | Recommended reusable component / precise API gap | Behavior to preserve | Severity |
|---|---|---|---|---|
| `src/frame/top_bar/primary_nav.rs:57` (`PrimaryNavButton`, used for Mappings at :21 and Batch map at :28) | `assets/frame/top_bar.css:63` `.if-primary-nav__button`; :79 underline; :90/:94 hover; :98/:102 active; :106 disabled; :112 focus | Reusable `NavigationButton` / shared tab-style presentation with `active`, `disabled`, `disabled_reason/title`, `aria_current`, `onclick`. Do not blindly substitute `TabButton`: it requires TabsRoot and emits role=tab/aria-selected, while this is navigation with aria-current=page. Reuse the tab presentation, retain navigation semantics. | App navigation callback, disabled Batch map with no loaded profile, active-page indicator, tooltip reason, native button keyboard behavior. | P2 |
| `src/frame/top_bar/tools_cluster/mod.rs:78` (`ToolButton`, Devices at :37, Profiles :44, Settings :53) | `assets/frame/top_bar.css:307` `.if-tools-cluster__button`; :328 hover; :333 pressed; :340 disabled; :346 focus | Extend `Button` with `aria_pressed` and `title` or create a shared compact `ToggleButton` (ghost/pressed appearance). Current `Button` has neither property; SegmentedControlOption has aria-pressed but models exclusive options, whereas each tool can toggle off. | Native disabled, pressed state, app TogglePanel route, calibration-derived Devices active state, title disabled reason. | P2 |
| `src/frame/top_bar/profile_name.rs:24` loaded button | `assets/frame/top_bar.css:220` `.if-profile-name`; :236 hover; :244 focus; :273 role caption | `Button` Ghost/Sm plus shared readout content; `Button.aria_label` already exists. Move presentation to a named readout/tone variant if it must stay distinct from normal Ghost. | Accessible `Profile: {name}` label, same app OpenPanel(Profiles) route, mono readout role/value and loaded state. | P2 |
| `src/frame/top_bar/profile_name.rs:42` empty button | Same base/focus; `top_bar.css:250` empty typography; :263 empty hover | Same Button as loaded branch; one wrapper/branch can vary label/value/state without repeating native button creation. | “Choose profile” accessible name and same Profiles panel route even without a profile. | P2 |
| `src/frame/top_bar/engine_pill/mod.rs:37` status/action button | `assets/frame/top_bar.css:125` `.if-engine-pill`; :151 disabled; :161 focus; :171 LED; :189/:194/:207 tones | Precise missing primitive: `StatusButton` (tone=live/warning/error, leading status dot, aria_pressed, aria_label, title, disabled, children/label, onclick). Ordinary Badge is noninteractive and Button lacks title/aria_pressed/status tones. Keep EnginePill's action/status decision in the domain; move only status-control rendering/chrome. | Prepared Start/Stop/Retry/Review/Refresh action, disabled when absent, aria-pressed, accessible action verb, separate polite live status text. | P2 |
| `src/frame/top_bar/mode_tabs/mod.rs:88` tail “+” add button | `assets/frame/top_bar.css:408` `.if-mode-tab--add`; :418 hover; :420 focus | `IconButton` Ghost/Sm with Plus + label “Add mode”; if strip-height appearance is required, add a compact toolbar/strip size instead of reimplementing button chrome. Existing IconButton accepts disabled/onclick/label. | Outside tablist/roving registry, BeginAdd app callback, disabled `!model.can_add`, accessible name. | P2 |
| `src/frame/profiles/library.rs:55` create-profile row button | `assets/frame/profiles.css:229` `.profile-row`; :252 `.profile-row--create`; :265 hover; :270 focus; :275 label | Existing Button Ghost with Plus children and shared `row/create` variant, or precise missing `CreateRow` action primitive owning dashed row chrome. A plain Card is not sufficient because this is an action. | Full row click target, native keyboard activation, app Intent::New, plus icon + label. | P2 |
| `src/frame/profiles/new_profile_submode.rs:39` back button | `assets/frame/profiles.css:84` `.profiles-panel__submode-back`; :95 hover; :100 focus | `Button` Ghost/Sm; retain only align-self/layout class. | Intent::Back and current text. | P2 |
| `src/frame/profiles/open_choice_submode.rs:24` back button | Same three rules as above | Same `Button` Ghost/Sm. These are exact copies of the same action chrome. | Intent::Back, cancellation/draft behavior owned by app. | P2 |
| `src/frame/profiles/new_profile_submode.rs:95` native radio, four options from loop at :77 | `assets/frame/profiles.css:177` `.profiles-panel__source-option`; :188 hover; :192 selected; :197 native radio accent/spacing; :203/:210/:216 text | Precise missing `RadioGroup`/`RadioOption` (or `ChoiceCard`) with native radio name/value/checked/onchange, label, description, and an optional detail slot outside the radio label. Existing Checkbox and SegmentedControl are not equivalent to radio semantics. | Browser mutual exclusion/arrow behavior from name `new-profile-source`; selected Source discriminant; source labels/descriptions; selected Library/File details; app Source callback. | P2 |
| `src/frame/profiles/new_profile_submode.rs:113` library select | `class: "if-text-input if-text-input--sm"` at :114; no dedicated screen selector | Existing `Select`/`SelectOption`, `InputSize::Sm`, reactive selected key; prepend empty “Pick a profile...” option. This is a direct design-system bypass by manually applying text-input classes to a different control. | Empty sentinel, prepared library keys/names, app `Intent::LibrarySource { key, choices }`, selected key. | P2 |
| `src/frame/profiles/snapshot_drawer.rs:49` disclosure button | Hand-authored `if-button if-button--ghost if-button--md` at :50; `assets/frame/profiles.css:444` `.snapshot-drawer__toggle`; :450 chevron; :457 expanded rotation; :461 title | Extend Button with `id`, `aria_expanded`, `aria_controls`, or add a small `DisclosureButton` that composes shared button rendering. Existing Button cannot express these attributes, so this is a real API blocker, not a drop-in replacement. | Stable `snapshot-drawer-bar-title` id used by Drawer label, expanded/control relationship to body, app toggle, count Badge, active profile text, rotating chevron, left-aligned shrinkable content. | P2 |

## Existing components whose consumers still fork the system

### S1 [P1] Inline mode rename has no accessible name

`src/frame/top_bar/mode_tabs/rename_inline.rs:33` renders TextInput with value, small size, invalid and describedby; no id/label/placeholder/aria-label exists. Its sibling at :37 only describes errors, not the field's purpose. A keyboard/screen-reader user opening rename reaches an unnamed textbox. This is source-visible absent accessible-name input, not a claim from a browser run.

Use a shared inline editing field with an explicit accessible name (e.g. “Mode name”), or add `aria_label` to TextInput and supply it. Preserve mounted focus (:34), commit-on-focusout (:32), key coordination (:31), app draft/error retention. The AddInline counterpart at `src/frame/top_bar/mode_tabs/add_inline.rs:32` has a placeholder only; give it an explicit name in the same pass. Relevant CSS: `assets/frame/top_bar.css:448`/`:449` editor geometry and :471 error. WCAG 4.1.2.

### S2 [P2] SettingsFieldRow duplicates Field and its advertised ARIA wiring never reaches controls

`src/frame/settings_panel/field_row.rs:39` implements its own label, :44 control wrapper, :51 helper. The ARIA description/invalid/error attributes are on the generic div at :46–:48, while focus lands on the IntegerInput/Switch descendant. `error_id` is generated at :26 and referenced at :48, but nothing renders that id (the message at :52 always uses helper_id). This contradicts the module's claim that it threads ids through the slotted element. It does not connect helper/error context to focused controls.

Affected consumers, exhaustively:

- `src/frame/settings_panel/startup_section.rs:21` launch setting; Switch :26, id :27.
- `src/frame/settings_panel/startup_section.rs:33` minimized setting; Switch :38, id :39.
- `src/frame/settings_panel/gestures_section.rs:28` double-tap threshold; IntegerInput :33, id :34.
- `src/frame/settings_panel/gestures_section.rs:44` long-press threshold; IntegerInput :49, id :50.
- `src/frame/settings_panel/snapshots_section.rs:30` buffer size; IntegerInput :37, id :38; the sole caller using row `error` (:35).
- `src/frame/settings_panel/snapshots_section.rs:49` skip unchanged; Switch :55, id :56.

CSS duplication: `assets/frame/settings_panel.css:35` row, :43 label, :49 control, :53 helper, :60 error text, :64 descendant IntegerInput border override. Shared `Field` exists with label/for_id/helper/error and matching wrappers; it needs a horizontal/wrapping layout variant to replace this row without losing settings density. Neither current Field nor IntegerInput/Switch automatically forwards helper/error metadata. Add explicit IDs on rendered helper/error and aria_describedby/aria_invalid/aria_errormessage props to the actual controls. Do not claim replacing the wrapper alone fixes accessibility.

Preserve correct label-for/control-id pairing, min/max/commit validation, app-owned failed-draft retention, helper replacement by buffer error, and wrapping row layout. P2 because static code proves miswiring, but degree of resulting assistive-technology impact has not been measured.

### S3 [P2] Profile name fields duplicate Field without label coupling/error association

- `src/frame/profiles/new_profile_submode.rs:49` wrapper, raw label :51, TextInput :55, error :63.
- `src/frame/profiles/open_choice_submode.rs:39` wrapper, raw label :41, TextInput :45, error :53, helper :60.
- `src/frame/profiles/row.rs:74` inline rename wrapper, TextInput :75, error :89.
- `src/frame/profiles/library.rs:24` filter TextInput relies only on placeholder “Filter profiles”.

The first two visible label elements lack for= and input ids; clicking the label cannot target the textbox. The error spans are not associated via TextInput.aria_describedby and the inputs do not set invalid. Shared Field already supplies label/helper/error presentation and for_id; use it with matching input ids, and extend its error/helper-id support rather than reproducing wiring in each form. Add a visually hidden Field/label or TextInput aria_label API for compact filter/rename fields.

CSS: `assets/frame/profiles.css:127` field flex-column/gap, :133 label, :141 helper, :148/:149 errors; :340 rename input flex. Move reusable field styling into Field variants; keep input sizing/layout in screen CSS. Preserve Name intents, rename key/blur/mounted focus callbacks and failed-delivery draft/error state.

### S4 [P2] Profile action menus restyle a shared trigger as a separate IconButton family

Consumer `src/frame/profiles/row.rs:96` MenuTrigger with Icon at :98 and hand-rolled hidden label :101. `assets/frame/profiles.css:362` reimplements width/height (28px), inline-flex centering, padding, background, border, color and transitions; :378–:383 reimplements row-hover/hover/expanded state. Shared IconButton already has 28px Sm geometry and Ghost behavior. MenuTrigger does not expose icon/size/variant, but does expose aria_label/unstyled; use a shared icon-trigger presentation/API (avoid nesting IconButton's native button inside MenuTrigger). Provide contextual `aria_label: format!("Profile actions for {}", name)`; remove custom hidden span if doing so.

Preserve shared menu opening/focus/close/rejected-onactivate behavior; row-hover reveal, aria-expanded latched style, and contextual name. `assets/frame/profiles.css:386` custom screen-reader-only clipping duplicates `.if-sr-only` if a hidden span remains.

### S5 [P2] Menu danger state and snapshot icon tones are consumer overrides

- `src/frame/profiles/row.rs:146` MenuItem class `.profile-row__danger-item`; `assets/frame/profiles.css:402` forces error text. Add semantic `MenuItem` tone/danger API; preserve app onactivate return value so failed action does not close menu.
- `src/frame/profiles/snapshot_drawer.rs:159` restore IconButton class; `assets/frame/profiles.css:630`/`:631` changes color+border to primary.
- `src/frame/profiles/snapshot_drawer.rs:167` delete IconButton class; `profiles.css:636` initially opacity zero, :640–:643 reveal, :646 disabled opacity/cursor, :651/:652 hover/focus changes color+border to error.
- Both buttons at `profiles.css:621` have custom transition policy duplicating IconButton's.

Keep the legitimate row-dependent visibility layout policy in the consumer (restore always visible; delete on hover/focus; disabled recovery anchor still visible). Move reusable ghost action/danger tone, transition and disabled opacity policy to IconButton rather than using the existing filled Danger variant, which would change the visual meaning. Preserve labels, enabled/disabled `can_delete`, restore confirmation, delete callback and focus reveal.

### S6 [P2] Tab density/scrollbar styling needs a supported presentation variant

The actual mode selection uses `TabsRoot`/`TabsList`/`TabButton` correctly (`src/frame/top_bar/mode_tabs/mod.rs:56`, :60, :74). But `assets/frame/top_bar.css:372` overrides TabsList gap to 0, padding to 0 and scroll behavior; :394 targets `.if-mode-tabs-wrap > .if-tab` to replace primitive padding with `0 0.875rem`, add max-width/truncation, and gap `0.375rem`. Shared `tabs.css` otherwise defines padding/gap/state chrome. Add a compact/toolbar density + scrollable-list option and per-tab max-width/truncate support; retain structural height and flex constraints in mode-strip CSS.

Do not move selection, app callbacks, right-click/Shift+F10/Delete coordination, mode-running dot, title, focus request/registry or editing-mode state out of their current owners. This is a styling contract improvement.

### S7 [P2] One absolute error class is reused across incompatible containers

Occurrences: `src/frame/top_bar/mode_tabs/add_inline.rs:36`, `rename_inline.rs:37`, `context_menu.rs:48`, `delete_dialog.rs:40`, `mod.rs:101`. All use `.if-mode-tab__error`, but `assets/frame/top_bar.css:471` unconditionally positions it absolute/top:100%, single-line/clipped with pointer-events:none. Only add/rename have the relative wrapper at :448/:449. Menu, dialog body and outer shell errors therefore inherit positioning designed for inline editors. The rename lives inside `.if-mode-tabs-wrap` with overflow-y:hidden (:380), so its below-input error may be clipped.

Create/reuse a normal-flow FieldError / InlineFeedback primitive, with an explicit floating inline-editor presentation only where needed. Keep role=alert and describedby linkage, retain app error content/draft behavior. Browser visual clipping is a verification target, not claimed tested.

### S8 [P2] Mode deletion uses Secondary for the destructive final action

`src/frame/top_bar/mode_tabs/delete_dialog.rs:47` renders Confirm as `ButtonVariant::Secondary`, while profile deletion uses Danger (`src/frame/profiles/delete_dialog.rs:53`), snapshot restore uses Danger (`snapshot_confirm.rs:27`), and snapshot pruning uses Danger (`settings_panel/prune_confirm.rs:30`). Use existing Danger for the destructive commit (and name the action “Delete” if copy scope is later authorized). Preserve cancel initial focus, counts, app confirmation/stale-target checks and failure retention. This is a variant-use inconsistency, not a missing component.

## Lower-priority duplicated presentation / token bypass

### S9 [P3] Repeated empty state/readout/separator vocabulary

- `src/frame/layout/empty_state.rs:16–:21`: root, eyebrow, heading, decorative divider, hint; actions :21.
- `src/frame/profiles/snapshot_drawer.rs:79–:83`: empty snapshot eyebrow/hint.
- `src/frame/profiles/library.rs:50`: filtered-empty state.

CSS `assets/frame/empty_state.css:23` and `profiles.css:503` duplicate mono/caption/regular/subtle/uppercase/0.18em tracking; `top_bar.css:273` role caption and :496 counts repeat the same readout vocabulary. A small shared `EmptyState` content component (optional eyebrow/title/description/actions, size) or shared `ReadoutCaption` typography variant is justified by actual recurrence. Retain the domain wording and structural fill/centering in consumers; do not replace entire screen shells with a generic Card.

`src/frame/top_bar/mod.rs:28` `.if-top-bar__divider` and `layout/empty_state.rs:19` `.if-empty-state__rule` duplicate existing Separator rendering. CSS `top_bar.css:42` and `empty_state.css:42` define 1px rules. Separator needs `decorative`/aria-hidden and `strong` tone (current Separator always emits role=separator and border-color, not border-strong) before a faithful substitution. Top-bar full-height seam and fixed 48px decorative rule remain legitimate layout distinctions.

### S10 [P3] Layout primitive opportunities, without treating all flex CSS as a violation

Exact duplicated generic recipes already served by Stack/Cluster:

- `src/frame/settings_panel/section.rs:10–:11` section + body wrappers; CSS `settings_panel.css:23` and :29 are identical column/gap=space-3; collapse one wrapper if no semantic/layout need, or use Stack inside semantic section.
- `src/frame/profiles/new_profile_submode.rs:152` and `open_choice_submode.rs:65` action rows; `profiles.css:155` is flex/wrap/gap=space-2 plus justification/margin; use Cluster plus layout-only class.
- `src/frame/layout/empty_state.rs:21`; `empty_state.css:55` is Cluster gap=space-2 plus center justification.
- `src/frame/profiles/snapshot_drawer.rs:61`, :153; `profiles.css:468`, :601 repeat inline-flex/center/gap=space-1 action groups; Cluster is display:flex/wrapping by default, so standardizing needs deliberate nowrap/inline API rather than automatic substitution.

Not violations: `layout/mod.rs` workspace branches, rail/center sizing, TopBar composition, profile/snapshot domain articles and metadata, source fieldset/legend, panel scroll/sticky behavior and Drawer region structure. These are screen/domain structure and should remain so.

### S11 [P3] Existing typography tokens are bypassed in six CSS declarations

All in `assets/frame/profiles.css`:

- :108 `.profiles-panel__submode-title` font-weight:600 → var(--weight-semibold).
- :136 `.profiles-panel__submode-field-label` font-weight:500 → var(--weight-medium).
- :213 `.profiles-panel__source-option-text strong` font-weight:600 → semibold.
- :277 `.profile-row--create__label` font-weight:500 → medium.
- :296 `.profile-row__title strong` font-weight:600 → semibold.
- :576 `.snapshot-row__label` font-weight:600 → semibold.

The repeated 0.18em readout tracking (`top_bar.css:256`, :279, :512; `empty_state.css:29`; `profiles.css:509`) is a named shared type treatment with no current tracking token. Add a shared typography class/token only when consolidating that treatment. No hard-coded color palette bypass was found in these five CSS files; colors use tokens/color-mix/currentColor/transparent. Do not label geometry like 40px chrome height, 320px rail, 40vh drawer limit, 1px seams, or documented custom capsule proportions as unconditionally wrong merely for being literal.

### S12 [P3] Dead/redundant CSS from previous menu/layout structures

- `assets/frame/profiles.css:395` `.profile-row__menu .if-menu__items:not([hidden]) .if-menu__list` and :399 z-index:1001 target a wrapper the current MenuItems no longer renders. MenuItems portals `.if-menu__list` directly and owns measured fixed positioning, so screen CSS must not own old menu positioning/stacking. Remove after confirming no hidden legacy host in other consumers.
- `profiles.css:14`, :23, :27 `.profiles-panel__new-profile` has no occurrence in any current assigned profile render body (creation is now ProfileLibrary's create row plus submodes).
- `profiles.css:301` `.profile-row__path` has no occurrence in current ProfileRow body.
- `profiles.css:302` `.snapshot-row__time` duplicates color/font/size/white-space already redeclared at :592–:599; keep one owner.
- `.snapshot-row__actions` appears in grouped rule at :17 and its own rule :601; use one rule (and deliberately retain the intended gap, which later rule changes to space-1).
- `top_bar.css:287` `.if-profile-name__value` is empty; delete empty selector.
- `profiles.css:657–:664` overrides all descendant animation/transition durations with hard-coded 0.01ms!important even though system `tokens/motion.css:47` sets all three duration tokens to 0ms for reduced motion. Let shared motion policy own it; check any non-token descendants before removing. This is cleanup, not an accessibility regression assertion.

### S13 [P2] Profile text rules reference five undefined line-height tokens

`assets/frame/profiles.css:122` `.profiles-panel__submode-path` uses `--leading-normal`. `:145` `.profiles-panel__submode-hint`, `:152` `.profiles-panel__submode-error/.profile-row__rename-error`, `:219` `.profiles-panel__source-option-desc`, and `:516` `.snapshot-drawer__empty-hint` use `--leading-snug`. The typography token file defines only `--leading-tight` (1.15) and `--leading-base` (1.45), at `assets/tokens/typography.css:31–:32`. Undefined var() with no fallback invalidates the declaration, so these surfaces inherit line-height instead of following the authored type rhythm. Use the canonical token appropriate to the prose/caption, normally base for descriptive text; do not invent aliases merely to preserve old names. Parent independently checked all-CSS definitions and reported the same missing tokens; the canonical token source was inspected directly here.

### S14 [P2] Source-option label contains unrelated interactive controls

`src/frame/profiles/new_profile_submode.rs:93` opens a label containing its radio (:95), plus a select (:113) or Button (:136) when selected. A label may contain only its labeled control as a labelable descendant. The nested library select/file-picker button means the source-choice component currently mixes choosing a radio with operating its details. A shared RadioOption should label only the radio and title/description; render details as a sibling slot associated by field/group labeling. Preserve Source selection and the select/file-picker callbacks independently. This is a component boundary/HTML-semantics issue; no platform click behavior was tested.

## Behavior-sensitive shared patterns worth consolidating

1. Back-to-library action and submode form scaffold: exact duplicated back action, title, Field, inline error and action Cluster across New/Open. The minimal change is use existing Button/Field/Cluster, not introduce a domain mega-component.
2. AddInline/RenameInline: same focused TextInput + error feedback + key/blur shell. A reusable `InlineTextEditor` can own label/description/error markup/focus styling, while app-owned Draft/Request/commit logic remains in callers. Profile inline rename has related markup but different commit wiring; do not force one workflow.
3. Compact icon menu trigger: shared menu mechanics already exist; share icon-button presentation/API rather than wrapper buttons.
4. Status action/pill and navigation/toggle chrome are genuine missing presentation variants. Prefer a few explicit shared APIs over exposing arbitrary CSS or converting all controls to a headless generic div.

## File-by-file coverage inventory

Production render bodies checked in full (test modules within files were not treated as production consumers):

- `src/app.rs` — checked app_root/app_root_view; context composition, ThemeProvider/ToastViewport/Layout only; no raw controls.
- `src/frame/layout/mod.rs` — checked Layout, MappingWorkspace, BulkMapWorkspace; legitimate structure, no raw controls.
- `src/frame/layout/empty_state.rs` — checked EmptyState; existing Buttons, repeated empty-state/separator layout noted.
- `src/frame/top_bar/mod.rs` — checked TopBar; legitimate structure + divider noted.
- `src/frame/top_bar/primary_nav.rs` — checked PrimaryNav + PrimaryNavButton; 1 raw declaration/2 uses.
- `src/frame/top_bar/profile_name.rs` — checked both render branches; 2 raw declarations.
- `src/frame/top_bar/engine_pill/mod.rs` — checked EnginePill production body; 1 raw declaration.
- `src/frame/top_bar/tools_cluster/mod.rs` — checked ToolsCluster + ToolButton; 1 raw declaration/3 uses.
- `src/frame/top_bar/mode_tabs/mod.rs` — checked ModeTabs including all branch bodies; 1 raw declaration.
- `src/frame/top_bar/mode_tabs/add_inline.rs` — checked; TextInput, no raw control.
- `src/frame/top_bar/mode_tabs/rename_inline.rs` — checked; TextInput, missing accessible name.
- `src/frame/top_bar/mode_tabs/context_menu.rs` — checked; AnchoredMenu/MenuItem only, error class mismatch.
- `src/frame/top_bar/mode_tabs/delete_dialog.rs` — checked; shared Dialog/Button, destructive variant mismatch.
- `src/frame/profiles/mod.rs` — checked ProfilesPanel; legitimate structure/shared children.
- `src/frame/profiles/library.rs` — checked; 1 raw create-row declaration, shared filter/icon button.
- `src/frame/profiles/row.rs` — checked; TextInput/Menu/Badge/Icon, trigger styling/field issues noted.
- `src/frame/profiles/new_profile_submode.rs` — checked; raw back/radio/select declarations (radio expands into four options).
- `src/frame/profiles/open_choice_submode.rs` — checked; 1 raw back declaration.
- `src/frame/profiles/delete_dialog.rs` — checked; Dialog/Button only.
- `src/frame/profiles/snapshot_confirm.rs` — checked; Dialog/Button only.
- `src/frame/profiles/snapshot_drawer.rs` — checked; 1 raw disclosure declaration, existing Drawer/IconButtons/Badges.
- `src/frame/settings_panel/mod.rs` — checked SettingsPanel/context mounting; no raw controls.
- `src/frame/settings_panel/field_row.rs` — checked; reusable Field duplicated, ARIA wrapper issue.
- `src/frame/settings_panel/section.rs` — checked; generic identical wrappers.
- `src/frame/settings_panel/gestures_section.rs` — checked; 2 shared IntegerInputs.
- `src/frame/settings_panel/startup_section.rs` — checked; 2 shared Switches.
- `src/frame/settings_panel/snapshots_section.rs` — checked; shared IntegerInput/Switch.
- `src/frame/settings_panel/prune_confirm.rs` — checked; shared Dialog/Button only.

Non-rendering/support/test-only assigned files, excluded from production-control findings (no tests run):

- `src/frame/top_bar/engine_pill/logic.rs` — logic module, no production render body.
- `src/frame/top_bar/tools_cluster/logic.rs` — logic module, no production render body.
- `src/frame/layout/acceptance_tests.rs`, `library_tests.rs` — test-only harnesses/assertions.
- `src/frame/top_bar/mode_tabs/tests.rs`, `events_tests/mod.rs`, `events_tests/fixture.rs` — test-only harnesses/assertions.
- `src/frame/profiles/tests.rs`, `event_tests.rs`, `snapshot_events.rs` — test-only harnesses/assertions; snapshot_events imports event_tests and is not a runtime consumer.
- `src/frame/settings_panel/event_tests.rs`, `test_support.rs` — test-only harnesses/support.

All five assigned stylesheets checked in full: `assets/frame/layout.css` (117 lines), `top_bar.css` (523), `profiles.css` (665), `settings_panel.css` (66), `empty_state.css` (60). Supporting component APIs checked: Button, IconButton, TextInput, IntegerInput, Select/SelectOption, Field, Switch, Card, TabButton, TabsRoot/List use contract, MenuTrigger/MenuItems/MenuItem, SegmentedControl/Option, Separator, Stack/Cluster/Inset; corresponding relevant component CSS and motion/typography/spacing/radii tokens inspected.

## Positive evidence and practical boundary

Most profile/settings operations already use shared Dialog/Button/TextInput/IntegerInput/Switch/Menu/Drawer/Badge components. Mode tabs already use shared roving-tab/focus behavior. This audit does not recommend extracting app workflow decisions into rendering primitives or replacing legitimate domain layout with Cards.

Skipped: visual/AT runtime checks, tests, hardware/platform checks, source edits. Main risk for follow-up: a mechanical Button/Field substitution would drop required ARIA, focus, rejected-delivery, or native-radio behavior unless the explicit API gaps above are addressed first.
