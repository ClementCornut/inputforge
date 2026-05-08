# Mapping List Device Filter And vJoy Badge Design

## Summary

This pass improves the mapping list rail for large profiles with many mappings:

- Remove the visible `(unnamed)` placeholder from resting mapping rows.
- Add one-click device filtering.
- Make vJoy output assignment distinguishable when multiple vJoy devices are used.

The design stays within the existing left-rail role: fast navigation, dense scanning, and hardware-first labels. It does not redesign the mapping editor or introduce an advanced filter language.

## Goals

- Let users filter the mapping list by physical device in one click.
- Treat a device filter as "show mappings touched by this hardware", not only "show mappings whose primary input belongs to this device".
- Keep the device filter stable and compact even when many devices exist.
- Replace the ambiguous visual output indicator with a compact badge that includes both vJoy device and output.
- Stop rendering `(unnamed)` as a row title after batch-created mappings.

## Non-Goals

- No multi-select device filter in this pass.
- No query language such as `device:VKB` or `vjoy:1`.
- No redesign of grouping, drag sorting, context menus, or inline rename behavior.
- No full representation of multiple output actions in the row. The rail shows the first vJoy output only.

## Data Model

`ConfigSnapshot::from_state` already precomputes `MappingSummary` once per polling tick. This pass extends that summary rather than walking action trees from the row renderer.

`MappingSummary` should gain:

- `referenced_devices`: physical `DeviceId` values that the row touches.
- `first_vjoy_output`: the first `OutputAddress` found in a `MapToVJoy` action, if any.

`referenced_devices` includes:

- the mapping's primary input device
- `MergeAxis.second_input.device`
- any input-bearing `Conditional` predicate device, including predicates nested under `All`, `Any`, and `Not`

`first_vjoy_output` is found with deterministic pre-order traversal over the action tree. Scan actions in stored order; when a `Conditional` is encountered, visit `if_true` before `if_false`; stop at the first `MapToVJoy`. If there are multiple `MapToVJoy` actions, this first visited output is used for the compact row badge.

## Determinism

`referenced_devices` is a de-duplicated set of bound physical `DeviceId` values. `InputAddress::Unbound` contributes no device reference.

The device-chip list is derived from current-mode mappings before applying the free-text filter, so typing in the text filter does not make device chips appear or disappear.

Device chips are sorted alphabetically by display label, case-insensitive. Duplicate-name chips are first disambiguated by appending `· {DeviceId}`, so the appended suffix breaks ties deterministically.

The selected device filter stores and compares `DeviceId`, not the visible device label.

## Filtering Behavior

The rail has two independent filters:

- free-text query, as today
- optional selected physical device

A row is visible only when it matches both active filters.

The selected device filter matches when the selected `DeviceId` appears in the row's `referenced_devices`. This means a row appears when the device is the primary input, a merge secondary input, or a conditional predicate input.

Clicking an inactive device chip activates that device. Clicking another chip switches to that device. Clicking the active chip clears the device filter.

## Rail UI

The filter area contains the existing text filter and a compact device-chip row.

The device-chip strip wraps onto multiple rows when many devices are present. CSS uses `flex-wrap: wrap`, no horizontal scroll. The rail's filter area grows by one chip-row per wrapped row; the mapping list below absorbs the height delta naturally rather than scrolling chips off-screen. (Updated by the 2026-05-07 mapping-list cohesion pass; the contract test `mapping_list_css_wraps_device_filter_chips_into_multiple_rows` locks `flex-wrap: wrap;` is present and `overflow-x: auto;` is absent.)

Device chips are derived from devices referenced by mappings in the current mode. Chip identity is always `DeviceId`. The visible label uses the current connected device name when available, otherwise a cached or profile-known name when available, otherwise the raw device ID. If two chips resolve to the same visible label, append a compact ID suffix so the chips remain distinguishable.

The chip row is hidden when there are no device references in the current mode.

The `Add mapping` control is sticky at the bottom of the rail. The mapping groups scroll behind/above it when the list is long, so adding a mapping remains reachable without scrolling to the end. The sticky control uses rail chrome, not a floating card, and must not cover the final row; reserve bottom padding in the scrollable list equal to the sticky control height plus the rail gap.

Rows keep their current group structure: axes, buttons, hats. Named rows show the mapping name as the primary line. Unnamed rows do not render a fake title; the source line becomes the visible identity.

Rows with `first_vjoy_output` echo the output inline on the source line, after an arrow glyph separator (`→`), as a `Chip Output` primitive instance:

- axes: `Stick · X → vJoy 1 · X`
- buttons: `Stick · Btn 4 → vJoy 2 · Btn 4`
- hats: `Stick · Hat 1 → vJoy 1 · Hat 1`

(Updated by the 2026-05-07 mapping-list cohesion pass; the legacy right-floating `.if-row__output-badge` was replaced by the inline `Chip Output` rendered immediately after the trigger label, separated by a `→` glyph.)

Output formatting reuses the existing editor/live-readout output-label convention. Axis labels are compact in the chip (`X`, `Y`, `Z`, `Rx`, etc.). Button and hat labels use the stored vJoy output IDs consistently with the current `format_output_label` behavior unless a later implementation centralizes a different convention across the GUI.

Rows without a vJoy output do not render the chip and the arrow glyph is not painted.

## Visual Direction

This is product UI for a precision sim-input tool. The design remains sharp, calm, and technical:

- compact neutral device chips
- action-blue only for the active device chip and focus affordances
- output-gold taxonomy hue for vJoy badges
- stable row heights
- no decorative cards, side stripes, gradients, or modal interactions

The output identifier follows the selected visual direction: one compact inline `Chip Output` echo on the source line after the trigger, separated by a `→` glyph, combining vJoy device and output into a single scannable token in the same scan path as the trigger itself.

## Empty And Edge States

If text and device filters combine to produce zero rows, reuse the existing zero-filter empty state. Its copy should acknowledge active filters when needed, for example "No mappings match the current filters." The empty state must expose inline clear actions for each active filter: clear text query when query text exists, clear device filter when a device chip is active, or both when both filters are active.

If a selected device no longer appears in the current mode's referenced-device set, clear the selected device filter rather than leaving a dead chip selected.

If the selected device disconnects but mappings still reference its ID, keep the chip because the filter is derived from mapping references, not only live device presence.

## Accessibility And Keyboard

Device chips are toggle buttons in tab order. Each chip has `aria-pressed="true"` when active and `aria-pressed="false"` when inactive, a stable accessible name, and a visible focus ring.

The chip strip wraps onto multiple rows when many devices are present, so all chips remain reachable in tab order without horizontal scrolling. Keyboard users tab through chips in document order across rows. When a visible chip label truncates, provide a `title` or equivalent accessible description with the full label.

Color is not the only state channel: the active chip should carry a class, pressed state, or text/state attribute in addition to color.

## Testing

Add focused coverage for:

- unnamed rows no longer render `(unnamed)` in resting row titles
- device reference extraction includes primary input devices
- device reference extraction includes `MergeAxis.second_input`
- device reference extraction includes nested conditional predicate inputs
- device reference extraction de-duplicates devices and ignores `InputAddress::Unbound`
- deterministic first vJoy output extraction covers top-level outputs, `Conditional.if_true`, and `Conditional.if_false`
- device filter combines with free-text search
- chip derivation is current-mode-only and independent of free-text query
- active device chip toggles and clears
- active device filter clears on mode change when the selected device no longer appears
- disconnected mapped-device chips persist because they are derived from mapping references
- duplicate device display names are disambiguated
- zero-result empty state exposes clear actions for text and device filters
- chip strip renders as a single-row overflow container with stable height in constrained width
- sticky `Add mapping` control remains visible while long mapping lists scroll and does not occlude the last row
- vJoy output badge includes both vJoy device and output label

## Implementation Notes

Prefer keeping action-tree extraction in `context.rs` near the existing glyph derivation helpers. The row renderer should consume `MappingSummary` fields and format labels; it should not inspect action trees.

The existing `mapping_list::filter::matches_filter` can remain responsible for free-text matching. Device filtering can either wrap it in `MappingList` or become a second predicate in `filter.rs`, whichever fits the current module boundaries cleanly.
