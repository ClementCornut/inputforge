# GUI design-system adoption

Implements the [source audit](gui-design-system-audit.md) against baseline `28a88f8`. Historical source snapshots remain in the linked audit partitions.

Status: all 46 audit findings implemented and reviewed; automated checks passed. Changes remain uncommitted.

## Audit closure

| Finding | Issue | Resolution / verification |
|---|---|---|
| Shell-S1 | Inline mode rename has no accessible name | `InlineTextEditor` names the mode rename field and associates its normal-flow error. |
| Shell-S2 | SettingsFieldRow duplicates Field and its advertised ARIA wiring never reaches controls | Removed `SettingsFieldRow`; horizontal `Field` mounts helper/error IDs and inputs reference the active message. |
| Shell-S3 | Profile name fields duplicate Field without label coupling/error association | Profile create/open forms use coupled `Field`/`TextInput` IDs; rename/filter inputs have explicit names and error metadata. |
| Shell-S4 | Profile action menus restyle a shared trigger as a separate IconButton family | Profile actions use the shared icon `MenuTrigger` presentation with a contextual profile name. |
| Shell-S5 | Menu danger state and snapshot icon tones are consumer overrides | `MenuItem.danger` and `IconButton` ghost danger/primary tones own reusable colors and interaction states. |
| Shell-S6 | Tab density/scrollbar styling needs a supported presentation variant | Shared compact/scrollable `TabsList` and truncating `TabButton` replace private tab skin overrides. |
| Shell-S7 | One absolute error class is reused across incompatible containers | `InlineFeedback` owns normal-flow errors; active mode rename sits beside the scrollable tab list so its message can grow the strip. |
| Shell-S8 | Mode deletion uses Secondary for the destructive final action | Final mode deletion uses `ButtonVariant::Danger`; cancel focus and acknowledged dispatch remain unchanged. |
| Shell-S9 | Repeated empty state/readout/separator vocabulary | Shared `EmptyState`, `ReadoutCaption` and decorative/strong `Separator` replace recurring content treatments. |
| Shell-S10 | Layout primitive opportunities, without treating all flex CSS as a violation | Shared `Cluster` owns action groups, including inline/nowrap variants; collapsed duplicate settings-section wrapper. |
| Shell-S11 | Existing typography tokens are bypassed in six CSS declarations | Profile weight literals replaced with canonical medium/semibold tokens; repeated caption tracking lives in `ReadoutCaption`. |
| Shell-S12 | Dead/redundant CSS from previous menu/layout structures | Removed obsolete profile/menu/path/value selectors, repeated snapshot rules and blanket reduced-motion override. |
| Shell-S13 | Profile text rules reference five undefined line-height tokens | Undefined profile line-height names replaced by canonical typography tokens/shared message styles. |
| Shell-S14 | Source-option label contains unrelated interactive controls | `RadioGroup`/`RadioOption` keep the library select and file-picker action in sibling details outside radio labels. |
| Mapping-M1 | Rail context menu bypasses the existing keyboard/overlay infrastructure | Shared measured `AnchoredMenu` with visible `MenuGroup` destinations replaces the hover submenu; existing bool acknowledgements keep rejected actions open. |
| Mapping-M2 | Rebind field is copied three times with six native buttons | Three consumers compose shared `RebindField`; capture lifetime and callback policy remain app-owned. |
| Mapping-M3 | Physical-key capture surface is implemented twice | Both physical-key capture surfaces use `KeyCaptureButton` and shared listening presentation. |
| Mapping-M4 | Editor title is a raw text input plus a custom interactive heading | Title uses `InlineTextEditor` title treatment with native text ownership and explicit edit action. |
| Mapping-M5 | Recovery action bypasses Button | Offline recovery uses shared `Button`. |
| Mapping-M6 | Readout expansion icon bypasses IconButton | Live-readout expansion uses named shared `IconButton`. |
| Mapping-M7 | Stage expansion header is another button family | Stage expansion uses shared disclosure `Button` presentation with expanded metadata. |
| Mapping-M8 | Eight predicate checkboxes bypass Checkbox | Eight repeated hat directions use shared labeled `Checkbox`. |
| Mapping-M9 | Add mapping and Add stage duplicate a create-slot action style | Add mapping/stage use shared create `Button`/`MenuTrigger` presentation. |
| Mapping-M10 | Device filter wraps Chip in an ad hoc toggle button | Device filter uses shared `ToggleChip` with pressed state. |
| Mapping-M11 | Batch action chips implement a separate button/toggle vocabulary | Batch actions use compact `Button`; Replace uses `ToggleChip`. |
| Mapping-M12 | Selectable mapping rows need a shared row interaction contract, not Button wrapping | Shared `SelectableRow` is a named focusable compound group, retaining nested rename/drag controls and app keyboard callbacks. |
| Mapping-M13 | Missing explicit labels and label associations | Compact fields have explicit names; visible stage labels use per-instance control IDs; batch row selects include contextual source names. |
| Mapping-M14 | Deadzone toolbar reuses IDs across independently expanded stages | Each deadzone toolbar derives all four control IDs from its unique instrument mount ID. |
| Mapping-M15 | Domain plot frames are duplicated; keep SVG geometry local | Curve/deadzone use named `PlotFrame` with instructions/value description; domain SVG, hit testing and app event interpretation stay local. |
| Mapping-M16 | Frame CSS overrides existing DS internals | Shared inline inputs, listening Chip, compact menu items and component-owned SegmentedControl styles replace feature skin overrides; domain sizing/drag placement stays local. |
| Mapping-M17 | Hard-coded semantic colors and token names | Semantic colors now derive from canonical tokens; undefined gesture typography names removed. |
| Mapping-M18 | Readout chips/status surfaces duplicate existing primitive styling | Shared `Badge`, `Chip` and `StatusDot` own repeated status chrome; axes, compass and signal geometry stay in readout renderers. |
| Mapping-M19 | Motion/typography/radius bypasses are widespread, including one wrong ownership link | Capture pulse and reduced motion are shared; remaining decorative motion/type/radius uses canonical tokens and stale CSS assertions are updated. |
| Shared-S1 | `src/frame/panel_slot/controller_session.rs:46` — routed-controller checkbox inside the `view.selections` loop | Shared `Checkbox` owns the sole native label and forwards prepared device availability/selection; disabled fieldset retained. |
| Shared-S2 | `src/frame/panel_slot/virtual_config.rs:62` — Buttons number input | Shared `NativeNumberInput` forwards exact button-count strings on native change with native driver bounds and step. |
| Shared-S3 | `src/frame/panel_slot/virtual_config.rs:66` — Hats number input | Shared `NativeNumberInput` forwards exact hat-count strings on native change with native bounds and step. |
| Shared-S4 | `src/frame/panel_slot/virtual_config.rs:71` — enabled-axis checkbox inside the `AXES` loop | Shared labeled `Checkbox` replaces each repeated axis control without nested labels; prepared disabling remains explicit. |
| Shared-S5 | `src/frame/panel_slot/device_panel/mod.rs:83` — entire selectable device ledger row is a native button | Device ledger uses native `Button` Row presentation with selected/pressed state; routing and inspection remain independent. |
| Shared-S6 | `src/frame/panel_slot/device_panel/inspector.rs:43` — display-name native text input | Inspector uses coupled `Field`/`TextInput`, contextual name, app text owner and uncontrolled initial value; workflow errors use `InlineFeedback`. |
| Shared-S7 | `src/toast/viewport.rs:81` — hand-built × dismiss button | Dismiss uses `IconButton` X, Ghost, Sm; notification host still forwards hover/focus/Escape and app dismissal observations. |
| Shared-S8 | `src/frame/panel_slot/axis_settings.rs:18-19` — native `details`/`summary` disclosure | Shared native `Disclosure` owns details/summary; browser collapsed state and axis-setting content are retained. |
| Shared-S9 | `src/toast/viewport.rs:79`; `assets/toast/toast.css:142-159` `.if-toast__count` | Repeated count uses a matching-tone `Badge` count treatment without adding a nested live region. |
| Shared-S10 | `assets/frame/panel_slot.css:304`, `:307`, `:309` `.if-controller-session input[type="number"]` and focus | Removed broad numeric descendant chrome; controller layout retains only structural spacing and sizing. |
| Shared-S11 | `assets/frame/panel_slot.css:301-307` | Controller spacing/type uses canonical tokens and shared leaf styling; broad label skin removed. |
| Shared-S12 | `assets/frame/panel_slot.css:163` `.if-device-row--selected` | Shared action-row selected surface uses `--tint-selected`; hover/focus/pressed styling is component-owned. |
| Shared-S13 | `assets/toast/toast.css:35-37` `.if-toast` | Toast composes existing `--shadow-3` and exact `--shadow-1` token instead of the duplicated literal. |

## Native controls and shared gaps

- All 42 audited native button/input/select declaration sites (Shell 12 / Mapping 23 / Shared 7), plus native disclosure, now compose shared components. Test fixtures and domain SVG geometry remain deliberate exceptions to the production-control inventory.
- Input metadata reaches native leaves; raw numeric change remains distinct from parsed numeric commit. Labels, helper/error IDs and invalid flags are explicit.
- Shared additions: native radio choices, raw number input, inline text editor, rebind/key-capture fields, toggle/compound row surfaces, inline feedback, status dot, readout caption, disclosure, empty state and plot frame.
- Existing actions/menu/tabs/Chip/Badge/Tooltip/Field/Separator/Cluster APIs now cover the audited presentations. SegmentedControl owns its CSS and the disabled Hold option references its mounted explanation tooltip.
- Gallery examples are split into cohesive modules, label every control and verify unique IDs and mounted ID references; demos cover the newly shared APIs.
- The rail uses visible grouped destination actions, avoiding a hidden hover submenu. Custom menu closure returns an explicit acknowledgement before focus restoration; failed activation or dismissal keeps its menu and focus.
- Shared action variants use low-specificity selectors so Row, Create, Navigation, Status and pressed Compact presentations retain their surface during hover/press. Frozen held readouts retain their text with neutral status styling.
- Newly touched rendering modules are split by responsibility rather than minified. App workflow/capture/validation/history policy and domain geometry remain in their existing layers.

Feature/toast stylesheets shrink from 4,626 to 2,813 physical lines (1,813 removed, including the follow-up device layout), with reusable styling moved into component-owned sheets. Source re-audit reports no production native action/input/select/disclosure/label bypass; remaining native elements in fixtures and domain SVG are intentional. The seven CSS variables not declared in token sheets are explicitly supplied runtime layout/plot/live-intensity properties. Touched production Rust modules stay within the 200 nonblank/noncomment-line baseline, excluding test modules.

## Native feedback follow-up

The six issues reported after the source audit are addressed:

| Reported issue | Correction |
|---|---|
| Mapping grip touches text | The name/source gutter leaves a full spacing token beyond the hovered grip's border and padding. A native WebKit fixture measured about 4.7px clearance. |
| Mapping right-click menu does not work | The shared cursor menu waits for its native `mounted` event and one animation frame before measuring, then explicitly sets `visibility: visible`. Dioxus retains omitted CSS properties, so removing the hidden declaration alone left the real menu invisible despite correct coordinates. Initial focus waits one frame for the visibility patch; directional navigation and existing activation/closure acknowledgement remain immediate. |
| Hold/Pulse looks like mismatched nested pills | The shared segmented control uses one rectangular group with equal grid columns; selected options fill the inner height. Tooltip-wrapped disabled options use the same geometry. |
| Virtual controller inputs are scattered | Each controller has a header/removal action, equal Buttons/Hats columns and a wrapping Axes group. Shared native controls keep raw change strings, driver limits and disabled behavior. Advanced axis settings follow the primary controls. |
| Display-name Save is smaller and top aligned | The input and Save use matching Md sizes in a full-width, stretching grid. Native WebKit measured both at 36px with identical top coordinates. |
| Profiles/Settings lack CSS on first visit | Both sheets register from the always-mounted theme before navigation. Removed panel-local registration and the raw Profiles `@import` that bypassed bundled asset URLs. |

Native WebKitGTK fixture checks passed for controller and alias layouts at 280px and 320px, with no control overflow. Direct and tooltip-wrapped segmented options have matching width/height. The actual cursor-measurement script returned nonzero native geometry and made the fixture menu visible. These checks use current production CSS and SSR device fragments; they do not establish full-app pointer delivery, bundled cold-start rendering or Windows acceptance. Temporary HTML dump hooks were removed after capture.

### Full native-app follow-up

The subsequent no-menu report was reproduced in the actual Wayland/WebKitGTK app launched by `dx run`, with routing stopped and profile/settings/data/cache isolated under `/tmp`. The menu had correct measured coordinates but retained inline `visibility: hidden`; changing visibility in the live DOM revealed it. The explicit ready-state visibility regression failed before the one-line fix and passed afterward. A separate initial-focus regression failed before deferring First focus by one frame.

After rebuilding, real Dioxus IPC/DOM checks verified visible right-click opening, initial Rename focus, ArrowDown to Duplicate, Escape dismissal/focus return to the row, and Rename activation with input focus and Escape cancellation. No runtime style override was used for the final visibility/focus checks. The compact Add action trigger is now centered; its native icon-center offset is effectively zero.

Opening Devices left the document at **854×506**, matching its client dimensions, with `window.scrollY` staying zero. The Devices pane had a **465px** viewport over **1,049px** of content and scrolled internally. The user confirmed that internal panel scrolling should remain visible, so no document clipping or scrollbar-hiding rule was added. These checks injected events into the actual native DOM; physical device/compositor delivery and Windows remain outside this run. The diagnostic app and inspector were closed.

### Viewport ownership correction

A later Devices report exposed a gap in the earlier single-size check: the shell was a normal-flow `100vh` box and the document remained a possible scroll target. A native layout probe reproduced document growth before the asynchronous global body reset (667×467 viewport, 675×483 document). The shell now anchors to the viewport with `position: fixed; inset: 0`, and Devices uses `overscroll-behavior-y: contain` so reaching its scroll boundary does not chain to the page. Internal panel scrolling remains visible and usable.

The same probe passed afterward at small, medium and large window sizes, including fractional scaling, with zero page scroll and 1,800px of content still scrolling inside Devices. The actual rebuilt native app also kept document/client dimensions at 854×506, allowed Devices to reach its internal scroll bottom, and retained visible, focused context menus. The 658 GUI all-target tests and unchanged formatting/strict Clippy gate passed. This correction is uncommitted against `6a28ff3`; Windows and physical wheel delivery remain unverified.

## Verification

- Fresh baseline: 607 GUI tests passed.
- Test-first shared regressions reproduced missing APIs (compile failure) and five behavioral/style contracts (57 passed, 5 failed as expected).
- App all-target suite: **563** passed, 4 ignored. Localhost UDP single-instance tests returned sandbox EPERM initially, then passed on the authorized outside-sandbox rerun using isolated test directories.
- Notifications 4, overlays 14 and component-gallery accessibility 1: all passed.
- One-off source CSS selector/specificity matrix: 108 presentation/variant/state combinations passed after fixing shared state precedence. This is source/cascade verification, not computed-style or native visual acceptance; an attempted headless Firefox run crashed before yielding results.
- Independent shared/shell, mapping-move and whole-change reviews completed. Findings on frozen held-status color, shared action state precedence, rejected-close focus and the production Hold tooltip were repaired and rechecked.
- Complete GUI all-target suite: **658 passed**, 0 ignored, across 10 targets (including 603 library tests and 23 reusable-control event tests). Updated fixtures target native IDs/names rather than listener order, while retaining dispatch, undo, Unicode/no-echo, stale/failure retention and latched-output assertions.
- Final full-workspace all-target rerun on the final source: **2,336 passed, 0 failed, 5 ignored**, across 41 targets.
- Workspace documentation tests: **1 passed, 0 failed, 5 existing ignored examples**. Combined Rust checks: **2,337 passed, 0 failed, 10 ignored**.
- Native-feedback regressions reproduced the eager cursor measurement before mount and the missing startup panel styles before their fixes. The exact production measurement script also passes its one-frame timing check (`node crates/inputforge-gui-dx/tests/menu_scripts/measurement.mjs`). The follow-up review found no remaining actionable source issues.
- Unchanged pre-commit hook passed: `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings`, with no lint suppressions added.
- `git diff --check` passed; the index is empty and HEAD remains `28a88f8`. No dependency or hook changes; all implementation, regression tests and documentation remain uncommitted.

Reproduce the checks from the repository root (build parallelism was capped during this run):

```sh
env CARGO_BUILD_JOBS=2 cargo test --workspace --locked --offline --all-targets
env CARGO_BUILD_JOBS=2 cargo test --workspace --locked --offline --doc
node crates/inputforge-gui-dx/tests/menu_scripts/measurement.mjs
env CARGO_BUILD_JOBS=2 bash .githooks/pre-commit
```

The app's single-instance tests need localhost UDP socket access; sandbox EPERM is an execution restriction, not a test pass. The four default ignored app cases and the default ignored autostart integration case remain ignored; none were enabled to claim native/hardware acceptance.

## Manual acceptance

Source/VDOM checks and isolated native layout fixtures do not establish full-app visual, accessibility-tree or hardware acceptance. Check these on native Wayland and separately on Windows:

- Navigate compact modes and their menus by keyboard; reject an action/dismissal and verify the menu keeps focus. Test successful activation, Escape, Tab and outside-click dismissal independently.
- Add/rename/delete modes and profiles, including failed dispatch, Unicode drafts, source radios/library selection/file picker, and error wrapping in the growing mode strip.
- Check actual field names/helper/error announcements and disabled Hold explanation; expand multiple deadzone/curve stages and exercise handles, arrows, Shift adjustment and focus.
- Inspect a device independently of routing selection; edit invalid/raw numeric controller values, disabled driver layouts, alias Save/Enter/Escape failures, and reconnect/apply behavior.
- Exercise capture/rebind/cancel/listening and reduced motion; compare live and frozen held readouts.
- Verify toast chronology/count/status roles, hover/focus pause, Escape and dismiss; check snapshot action reveal, destructive confirmations and native disclosure toggling.

The standard small toast action is now 28px, and active mode rename appears beside the scrollable tabs so feedback can remain in normal flow. These intentional shared-component presentation changes need visual acceptance.
