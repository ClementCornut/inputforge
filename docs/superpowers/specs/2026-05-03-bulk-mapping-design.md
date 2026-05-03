# Bulk Mapping Wizard Design Spec

**Status:** Design approved, ready for implementation plan
**Date:** 2026-05-03
**Parent specs:** [`2026-04-30-f8-mapping-list-design.md`](./2026-04-30-f8-mapping-list-design.md) (mapping list, `+ Add mapping` row, F8's per-row conflict pattern is extended across modes for the wizard's per-(row, mode) scope), [`2026-04-30-f9-mapping-editor-design.md`](./2026-04-30-f9-mapping-editor-design.md) (live data-source convention: `AppState.input_cache` reads), [`2026-04-28-f6-snapshot-preferences-core-design.md`](./2026-04-28-f6-snapshot-preferences-core-design.md) (snapshot infrastructure).
**Brainstorm artefacts:** `.superpowers/brainstorm/1197-1777800140/content/panel-layout.html`
**Design system:** [`/DESIGN.md`](../../DESIGN.md)
**Product brief:** [`/PRODUCT.md`](../../PRODUCT.md)
**Engine command:** `crates/inputforge-core/src/engine/command.rs:33` (existing `EngineCommand::SetMapping`).
**Profile mutation:** `crates/inputforge-core/src/profile/mod.rs:219` (existing `Profile::set_mapping`).
**Snapshot kinds:** `crates/inputforge-core/src/snapshot/types.rs:27` (existing `SnapshotKind`).
**Live virtual-device state:** `crates/inputforge-core/src/state/mod.rs:62` (existing `AppState.virtual_devices`).
**vJoy device enumeration:** `crates/inputforge-core/src/output/vjoy_output.rs:215` (existing `VJoyOutput::list_devices`).

---

## Context

Today the user creates input-to-vJoy mappings one at a time. The `+ Add mapping` row in the mapping list rail captures one physical input, optionally lets the user name it, and dispatches one `EngineCommand::SetMapping` per row. The mapping editor then layers on the per-mapping pipeline (deadzone, response curve, output, conditionals). For a fresh stick with eight axes, thirty buttons, and a hat, building the baseline "axis X to vJoy axis X, axis Y to vJoy axis Y, every button to its same-numbered vJoy button" set takes thirty-nine separate add-and-confirm cycles before the user can start adding any per-mapping flavour.

This spec introduces a side-panel wizard that creates the baseline pass-through mapping set in one shot, leaving every per-mapping pipeline empty save for a single `MapToVJoy` action. The user then opens each mapping in the existing editor to add their deadzones, curves, and conditionals at their own pace. The wizard is additive: it does not replace, alter, or wrap the existing `+ Add mapping` flow.

The shape of the bulk operation is intentionally narrow:

- One physical source device per session of the wizard.
- One target vJoy device per session.
- Per-row override of the auto-suggested target output, with a `(do not map)` choice that excludes a row.
- Pre-flight tally of what Apply will create, replace, skip, and exclude.
- Per-group bulk affordances (skip-all-conflicts, replace-all-conflicts) that surface only when relevant, so the group headers stay clean for the typical case.
- Atomic apply: pre-save the in-memory profile (so disk and memory match), one auto-snapshot taken from disk, one bulk command to the engine upserting in-memory, one second profile save, one toast.

The user can re-run the wizard whenever a new device is plugged in, or when starting from a fresh profile.

---

## Confirmed design choices

The decisions below were validated in brainstorm one question at a time.

**Q1. Source / target relationship: B refined.** Pick one source device, pick one target vJoy. Each source input gets its own row with a select containing the auto-mapped target pre-filled and a `(do not map)` choice to exclude. Per-row select rather than checkboxes, so the user can repoint a row when the auto-mapping guesses wrong instead of binary including/excluding it.

**Q2. Mode scope: B with all-modes checkbox.** A Mode picker defaults to the active editing mode, with an `Apply to all modes` checkbox alongside. Checkbox carries the live mode count (`Apply to all modes (5)`); when checked, the Mode picker enters its disabled state and the summary chip below the Apply button updates to read `... across N modes`.

**Q3. Conflict handling: A.** Skip on conflict by default. A row whose `(input, mode)` already exists in the active profile renders dimmed with an `already mapped: "<existing name>"` subtext and is excluded from the apply tally. A per-row `replace` chip promotes the row to the replace tally and tints the row amber, swapping the subtext to `replacing "<existing name>"`. Bias is toward not destroying user work.

**Q4. Auto-mapping convention: A.** Positional, by index. Source axis index `i` maps to the `i`-th vJoy axis in enum order (`X, Y, Z, Rx, Ry, Rz, Slider0, Slider1`). Source button `i` (0-indexed) maps to vJoy button `i + 1` (1-indexed at the SDK layer; this is intentional, not a typo). Source hat `i` (0-indexed) maps to vJoy hat `i + 1`. If the source has more inputs of a kind than the target vJoy exposes, the overflow rows default to `(do not map)` and the per-row select hides the unavailable slots. Concretely: an axis row whose `i >= len(VirtualDeviceConfig.axes)`, a button row whose `i >= VirtualDeviceConfig.button_count`, or a hat row whose `i >= VirtualDeviceConfig.hat_count` is overflow. Persisted per-(source, target) override replay is deferred (see Deferred follow-ups).

**Q5. Action set: A.** Bare passthrough. Each generated mapping carries a single `Action::MapToVJoy { output }` and nothing else. No deadzone, no response curve, no inversion. Flavour is added afterward in the regular per-mapping editor.

**Q6. Naming: A.** Blank. Each generated mapping has `name: None`. The mapping list renders the input identifier as a fallback label for unnamed mappings, so the rail stays scannable. Names get added later when the user knows what each input means in the target game. Note that the existing `+ Add mapping` flow already supports unnamed creation; only rename is currently gated against empty values, which is a separate, out-of-scope inconsistency.

**Q7. Trigger location: tools cluster only.** A small icon button in the top bar's `tools_cluster`. No empty-state CTA in the rail, no right-click affordance on group headers, no split disclosure on the `+ Add mapping` row. The tools cluster is the idiomatic home for profile-wide tools; surfacing a "Get started" wizard in the empty state imports SaaS empty-state grammar that `DESIGN.md` explicitly bans.

**Q8. Source device picker scope: A.** Currently-connected devices only. The picker reads from the engine's connected-device list (the source of truth for input enumeration). Disconnected devices in the profile are not eligible. This sidesteps the offline-enumeration problem (SDL3 cannot report axis or button counts for an unplugged device) and matches the `Live data is the contract` principle. Offline authoring is deferred (see Deferred follow-ups).

**Q9. Target vJoy picker scope: C.** Live-detected vJoys only, with an honest empty state. The picker reads from `AppState.virtual_devices`, populated by the engine's vJoy probe at startup. The select surfaces both device id and capability summary (`vJoy 1: 8 axes, 32 buttons, 1 hat`). Per-row target selects filter their option lists to slots that the chosen vJoy actually exposes (no offering `Slider1` if vJoy is configured with five axes). When `AppState.virtual_devices` is empty, the panel shows a no-signal empty state pointing the user at vJoyConf and disables the Apply button.

**Q10. Live readout per row: A.** Inline, via a new sibling component `crate::frame::bulk_map::row_readout` that mirrors the data sources used by `LiveReadout` (`AppState.input_cache` reads) but renders axes, buttons, and hats per the wizard grid template. F9's existing `LiveReadout` is layout-coupled to the editor and only handles axes; the wizard needs all three kinds, so a sibling component is the cleaner boundary. Each axis row shows a bipolar bar; each button row shows a filled-or-stamped dot; each hat row shows a mono cardinal letter. Wiggle the stick, watch the corresponding row pulse, confirm the auto-mapping pointed at the right slot.

**Q11. Apply atomicity and undo: C, embedded snapshot.** New `EngineCommand::SetMappingsBulk { entries, snapshot_label }` variant. The engine handler runs four steps in order: (1) save the current in-memory profile to disk so the on-disk body matches the user's pre-bulk state; (2) create an `AutoBeforeBulkMap` snapshot via `crate::snapshot::create`, then `crate::snapshot::prune`; (3) process all entries in one in-memory pass; (4) save the post-bulk profile to disk. The success toast is GUI-side and dispatched optimistically by the wizard before the engine even processes the command (see Post-apply behaviour below); it is not an engine handler step. The snapshot creation is **inside** the bulk handler (not a separate dispatched command) so the bulk apply cannot proceed without a recovery snapshot. If the snapshot creation fails, the handler aborts the apply and logs a warning; the user sees an error toast and the profile is unchanged. The pre-save in step 1 guarantees the recovery snapshot captures the user's authored state at the moment of bulk apply, not whatever happened to be on disk last. This handler shape borrows from the `RestoreSnapshot` pattern (`engine/run.rs:686-727`) for the snapshot-then-mutate discipline, but bulk-map does not reload from disk after the bulk save: the in-memory mutation is the source of truth, and the recovery path on save failure is user-driven Restore via the snapshot index UI rather than auto-rollback.

**Q12. Summary chip placement: B.** Co-located with the Apply button. The summary chip sits directly above (or beside) the primary action, so plan and execute are visually coupled at the moment the user is about to commit. No top-of-panel summary; the metadata strip at the top stays focused on inputs (Source, Target, Mode, all-modes).

**Q13. Per-group bulk operations: A.** Conditional inline action chips on group headers. When the group contains conflicts, the group header surfaces `skip all conflicts` and `replace all conflicts` chips; when the group has many excluded rows, `include all` appears; when the group has many included rows, `exclude all` appears. Empty-handed when no action applies. No panel-level toolbar, no global "skip everything" affordance; both were considered footguns for the rare case where the user means them.

**Q14. Apply-to-all-modes semantics: A.** Inline-count checkbox with picker dimming. The `Apply to all modes (N)` checkbox carries the active profile's mode count. When checked, the Mode picker enters its disabled state, and the summary chip updates to include the cross-mode tally (`across N modes`). Segmented control replacing the picker plus checkbox combo was considered and rejected for v1: InputForge has no segmented-control primitive yet, and adding one is its own design decision out of scope here.

---

## Non-goals (out of scope)

- **Replacing or wrapping the per-input `+ Add mapping` flow.** The wizard is additive; manual single-input add continues unchanged.
- **Authoring sessions for unplugged devices.** Source picker is connected-only.
- **Default action templates beyond bare passthrough.** No auto-deadzone for axes, no curve presets, no per-kind defaults. The `Action template` knob is deferred.
- **Auto-naming of generated mappings.** Names are blank; the user names each later if they want.
- **Panel-level bulk operations.** All bulk affordances are scoped to a group (Axes / Buttons / Hats); a panel-wide "skip everything" or "replace everything" is intentionally absent.
- **Segmented mode-scope control.** The picker plus checkbox grammar is kept; introducing a segmented control is its own design decision.
- **Reusable `SidePanel` primitive.** The wizard's panel surface is one-off CSS; extraction is deferred until a second consumer appears.
- **Live keyboard output preview in row live readouts.** The wizard never generates `MapToKeyboard` actions; the live readout column carries vJoy-shaped feedback only.

---

## Trigger and panel surface

The wizard launches from a single icon button in the top bar's `tools_cluster`. The button's hover tooltip reads `Bulk-map device to vJoy ...`. No other entry point exists.

When the user clicks, the panel slides in from the right of the workspace at 460px wide (clamped to `min(460px, calc(100vw - 240px - 320px))` on narrow windows, so the rail and a minimum mapping-editor footprint stay visible). The wizard mounts inside the existing `<aside class="if-panel-slot">` scaffolding owned by `crates/inputforge-gui-dx/src/frame/panel_slot/mod.rs:51`, reached via a new `PanelSlot::BulkMap` variant on the existing enum at `crates/inputforge-gui-dx/src/frame/view_state.rs:28`. This makes the wizard mutually exclusive with `PanelSlot::Devices` and `PanelSlot::Profiles` (only one right-side tool open at a time), reuses the entrance-keyframe discipline already in `panel_slot/mod.rs`, and avoids a parallel right-aside element. The panel surface uses `--bg-elevated` with a 1px `--border` left edge (not `--strong`; the spirit of the side-stripe ban applies) and the standard chamfer highlight (`inset 0 1px 0 rgba(255,255,255,0.04)`) on the upper edge. If the existing aside CSS in `panel_slot.css` enforces a width that conflicts with 460px, the implementation may add a per-variant width override in `panel_slot/mod.rs`; this is a tactical follow-up, not a blocker.

The panel is **modeless**, on the same terms as the existing `PanelSlot::Devices` and `PanelSlot::Profiles` variants. It does not dim the canvas, does not trap focus from the underlying mapping list rail, and does not block keyboard navigation outside its boundary. Esc dismisses the panel and discards all per-row edits silently with no confirm dialog. This matches `DESIGN.md` Surfaces section 6: side panel for multi-field flows, dialog only for data-loss or destructive confirmations.

The mapping editor (centre column) is occluded while the panel is open. The mapping list rail (left column) stays visible so the user can see the mode they are editing and the ambient list of existing mappings while the wizard reasons about conflicts against that mode.

---

## Visual treatment

### Panel header

Title `Bulk-map device` rendered in the system's `title` typography (20px / weight 600 / leading 1.45 / letter-spacing -0.01em). On the right, a real `<button>` close affordance with `aria-label="Close panel"` and a tooltip showing `Esc`.

### Metadata strip

Two-row, two-column grid:

- Row 1: Source picker, Target picker.
- Row 2: Mode picker, `Apply to all modes (N)` checkbox.

Both pickers use the existing `Select` primitive. The all-modes toggle uses the existing `Checkbox` primitive (real `<input type="checkbox">`, never a span-as-checkbox). When the all-modes checkbox is checked, the Mode picker enters its `disabled` state. If the existing `Checkbox` primitive does not support arbitrary label content (the `(N)` count needs to render inline with the label), treat this as a primitive enhancement; the fallback is to render the count as a sibling span. Tactical decision at implementation time, not blocking.

### Rows table

A grid-rendered table grouped by Axes / Buttons / Hats, mirroring the rail's group taxonomy. Column template, shared between the header row and every body row:

```
28px | 1fr    | 70px | 1fr    | auto
kind | source | live | target | action
```

The arrow column from the brainstorm mockup is **dropped**; the column order already implies direction (source on the left, target on the right). The action column is **auto-sized** so it does not reserve width on rows with no per-row affordance.

Row anatomy:

- **Kind chip.** 18px square with a single letter (`A`, `B`, `H`), tinted with the row's category hue at 14% (processing teal for axis, control violet for button, output gold for hat). The hat chip's gold is desaturated one step from the canonical `--output` to widen the gap from `--warning` amber and avoid visual collision when the row also carries a replace badge.
- **Source cell.** Mono `Axis 0` / `Btn 5` / `Hat 0` label in the body. When the row collides with an existing mapping in any mode the wizard is targeting, a caption-size subtext appears below: `replacing "Throttle"` (amber) when set to replace, or `already mapped: "Throttle"` (muted) when set to skip.
- **Live cell.** `crate::frame::bulk_map::row_readout` (new sibling component, see Cross-cutting reuses below). Bipolar bar for axes (showing the cached input value from `AppState.input_cache`), filled-or-stamped dot for buttons, mono cardinal letter for hats. Live values use `--live` at 60-70% fill rather than full saturation, so a row of pressed buttons does not become the loudest pixel cluster on the panel.
- **Target cell.** The existing `Select` primitive listing only the chosen vJoy's available outputs of the matching kind, plus a `(do not map)` option at the top. Mono font for the actual identifiers (`Axis X`, `Button 5`, `Hat 1`); Inter italic for the `(do not map)` placeholder. If the existing `Select` primitive does not support per-option font variants, treat this as a tactical follow-up; the fallback is a non-italic placeholder. Not blocking.
- **Action cell.** A single affordance with two visual states: outlined chip `replace` when the row is currently set to skip (action available), filled chip `replacing` when set to replace (state active). Same shape, same column, same x-position; weight changes encode state. This collapses the two distinct affordances flagged in design critique into one consistent column.

### Group headers

Group headers render between body rows, sticky to the top of the table on scroll. Format: `Axes (4)` left-aligned in `label` typography. Right-aligned: the conditional bulk-action chip set, surfaced only when at least one applies:

- `skip all conflicts` chip when the group contains rows currently in replace state and at least one is conflict-driven.
- `replace all conflicts` chip when the group contains rows currently in skip state with conflicts.
- `include all` chip when the group has at least one row in `(do not map)` state.
- `exclude all` chip when the group has at least one row in any non-`(do not map)` state.

Empty-handed when no chips apply; the header stays clean for the common no-conflict case.

### Pre-apply summary chip

Sits directly above the panel footer, rendering in mono with `font-variant-numeric: tabular-nums` so the counts do not jitter as the user toggles per-row state. Format:

```
+12 create across 5 modes  ·  0 replace  ·  2 skip  ·  1 excluded
```

The `across N modes` clause appears only when `Apply to all modes` is checked. Numeric counts render in `--text`; the label words and bullet separators render in `--subtle`. The chip is the contractual statement of what the Apply button is about to do; co-locating it with the button removes the "scroll back up to check" loop.

### Footer

Right-aligned action row. Ghost `Cancel` plus primary `Apply 12 mappings` (the count is `create + replace`, updated live as the user toggles per-row state). Below the footer, a single muted-text caption: `Snapshot taken before apply.`. No confirm dialog precedes the apply.

### Post-apply behaviour

On Apply click, the GUI dispatches the single `SetMappingsBulk` command, **immediately closes the panel** (sets `view.panel_slot` to `PanelSlot::None`), and **immediately enqueues a success toast** `Created X mappings` (where X is `create + replace`) via the existing toast primitive. This is optimistic: the engine handler is presumed to succeed. If the snapshot creation fails inside the handler, the engine pushes a warning to the existing warnings channel, which already drives a separate error toast (`Bulk-map aborted: could not create recovery snapshot`) via the existing warnings-bridge wiring. Both toasts can co-exist; the user reads them top-to-bottom and the failure toast supersedes in meaning.

### Empty state (no vJoy detected)

When `AppState.virtual_devices` is empty at panel open, the metadata strip and rows table are replaced by an instrument-style "no signal" framing: a neutral icon, the title `No vJoy devices configured`, and the caption `Configure outputs in vJoyConf, then reopen.`. No SVG illustration of people or devices, no "Get started!" CTA. The footer renders with Apply disabled; Cancel and Esc still work.

### Motion

- Panel enter: 240ms `easing-standard` (`cubic-bezier(0.32, 0.08, 0.24, 1)`) on `transform: translateX(40px → 0)` plus `opacity 0 → 1`.
- Panel exit: 180ms `easing-fast` opacity-only.
- Per-row live cell value updates: 100ms `easing-fast` width transition on the bipolar bar fill.
- No bounce, no elastic, no overshoot.
- `prefers-reduced-motion` collapses the slide transform to zero distance, keeps the opacity fade.

### Accessibility commitments (decided at design time, not deferred)

- Esc-close uses a real `<button>` with `aria-label="Close panel"`.
- All-modes toggle uses a real `<input type="checkbox">`.
- Rows table uses an ARIA grid pattern: `role="grid"` on the container, `role="row"` per row, `role="gridcell"` per cell, `role="rowgroup"` per group header.
- Per-row replace chip is a real `<button>` with state encoded via `aria-pressed`.
- Group bulk-action chips are real `<button>` elements, focusable in document order between the group header and the first row.
- Focus rings follow the system 2px focus-cyan outline at 2px offset.

---

## Engine command and data model

### New types in `inputforge-core`

```rust
// crates/inputforge-core/src/action/bulk.rs (new file)
#[derive(Debug, Clone, PartialEq)]
pub struct BulkMapEntry {
    /// `input` MUST be `InputAddress::Bound { device, input }`.
    /// The wizard always knows the source device, so all entries it
    /// dispatches are bound. The engine handler treats `Unbound`
    /// entries as user error and silently skips them (covered by the
    /// `engine_set_mappings_bulk_skips_entries_with_unbound_input`
    /// test in layer 3).
    pub input: InputAddress,
    pub mode: String,
    pub output: OutputAddress,
}
```

Re-exported via `crate::action::BulkMapEntry`.

```rust
// crates/inputforge-core/src/engine/command.rs (modified)
pub enum EngineCommand {
    // ... existing variants
    SetMappingsBulk {
        entries: Vec<BulkMapEntry>,
        /// Label for the AutoBeforeBulkMap snapshot the handler takes
        /// before applying the upserts. Format: "Before bulk-map: <source> to vJoy <id>".
        snapshot_label: String,
    },
}
```

The entry shape is `(input, mode, output)`. Each row by mode pair the user committed to creating or replacing becomes one entry. The engine treats every entry as an upsert that writes a mapping with `name: None` and `actions: vec![Action::MapToVJoy { output }]`. There is no conflict flag in the entry; the GUI is responsible for excluding skip-on-conflict rows before dispatch. The `snapshot_label` is the user-visible label for the recovery snapshot the handler creates before the upserts; the handler is responsible for taking the snapshot, not the GUI.

### New `SnapshotKind` variant

```rust
// crates/inputforge-core/src/snapshot/types.rs (modified)
pub enum SnapshotKind {
    AutoSessionStart,
    AutoBeforeRestore,
    AutoBeforeBulkMap, // NEW
    Manual,
}
```

Serializes as `auto_before_bulk_map`. Not pinned (existing production `pinned: matches!(kind, SnapshotKind::Manual)` logic in `crates/inputforge-core/src/snapshot/mod.rs:80` already handles the new variant correctly; the matching expression in `snapshot/index.rs:169` is a test-fixture builder, not the runtime invariant). Always fires; never deduped, mirroring `AutoBeforeRestore` semantics. Recovery via the existing snapshot index UI.

### New `Profile` method

```rust
// crates/inputforge-core/src/profile/mod.rs (modified)
impl Profile {
    pub fn set_mappings_bulk(&mut self, entries: &[BulkMapEntry]) {
        for entry in entries {
            let actions = vec![Action::MapToVJoy { output: entry.output.clone() }];
            self.set_mapping(&entry.input, &entry.mode, None, actions);
        }
    }
}
```

Single in-memory pass that delegates to the existing `set_mapping` upsert. **No file save inside this method.** The engine handler owns the two profile saves (pre-snapshot and post-bulk); see the next section.

### Engine handler

```rust
// crates/inputforge-core/src/engine/run.rs (modified)
EngineCommand::SetMappingsBulk { entries, snapshot_label } => {
    self.set_mappings_bulk(entries, snapshot_label);
    self.pending_output_refresh = true;
}

// Returns `()`, matching the shape of the existing `set_mapping` handler
// (`crates/inputforge-core/src/engine/run.rs:783`). Snapshot and save
// errors surface to the user via the warnings channel rather than `?`,
// because the parent command-drain loop swallows arm errors anyway.
fn set_mappings_bulk(&self, entries: Vec<BulkMapEntry>, snapshot_label: String) {
    // Step 0: clone the profile path. The read guard drops at the end of
    // this statement. Do NOT hold any state lock during
    // `crate::snapshot::create` and `crate::snapshot::prune` (step 2),
    // because those perform disk I/O that must run lock-free (mirrors
    // `engine/run.rs:687-700`). Step 1's pre-save takes a short-lived
    // read guard that drops before step 2.
    let Some(path) = self.state.read().profile_path.clone() else {
        tracing::warn!("SetMappingsBulk: no profile loaded, ignoring");
        return;
    };

    // Step 1: pre-save the in-memory profile so the on-disk body matches
    // the user's pre-bulk authored state. Without this, the snapshot in
    // step 2 captures whatever happened to be on disk last (which may be
    // older than the in-memory state if any caller deferred a save).
    {
        let state = self.state.read();
        if let Some(profile) = state.active_profile.as_ref() {
            if let Err(e) = profile.save(&path) {
                tracing::warn!(path = %path.display(), error = ?e, "SetMappingsBulk: pre-snapshot save failed; aborting");
                drop(state);
                self.state.write().warnings.push(
                    "Bulk-map aborted: could not save profile before snapshot".to_owned()
                );
                return;
            }
        } else {
            return;
        }
    } // read guard drops

    // Step 2: take the recovery snapshot. Abort if it fails so the user
    // never ends up with bulk-applied mappings and no snapshot to roll
    // back to. Note: `snapshot::create` and `snapshot::prune` are free
    // functions in `crates/inputforge-core/src/snapshot/mod.rs`, NOT
    // methods on `Engine`. They are called with `&path` and the existing
    // `self.settings.snapshot` config.
    match crate::snapshot::create(
        &path,
        crate::snapshot::SnapshotKind::AutoBeforeBulkMap,
        Some(snapshot_label),
        &self.settings.snapshot,
    ) {
        Ok(_) => {
            // Prune retention; failure is non-fatal (matches RestoreSnapshot).
            let _ = crate::snapshot::prune(&path, &self.settings.snapshot);
        }
        Err(e) => {
            tracing::warn!(error = ?e, "SetMappingsBulk: AutoBeforeBulkMap snapshot failed; aborting apply");
            self.state.write().warnings.push(
                "Bulk-map aborted: could not create recovery snapshot".to_owned()
            );
            return;
        }
    }

    // Step 3: apply upserts and persist (second save).
    let mut state = self.state.write();
    let Some(profile) = state.active_profile.as_mut() else {
        return;
    };
    profile.set_mappings_bulk(&entries);
    if let Err(e) = profile.save(&path) {
        tracing::warn!(path = %path.display(), error = ?e, "SetMappingsBulk: post-bulk save failed; in-memory state holds bulk; recovery via Restore");
    }
}
```

Two profile saves per bulk apply (one before the snapshot, one after the upserts), regardless of entry count. The snapshot captures the pre-bulk authored state; the upserts run lock-correctly with the read guard dropped before snapshot I/O and the write guard taken only for the in-memory mutation plus post-bulk save. Same `fn -> ()` shape as the existing `set_mapping` handler at `crates/inputforge-core/src/engine/run.rs:783`.

### GUI dispatch

The wizard dispatches **one command** when Apply is clicked:

```rust
EngineCommand::SetMappingsBulk {
    entries,
    snapshot_label: format!("Before bulk-map: {} to vJoy {}", source_name, target_id),
}
```

The engine handler is responsible for creating the recovery snapshot and the upserts as a single atomic step. The GUI does not separately dispatch `CreateSnapshot`; that path is reserved for the user's manual snapshot button.

### Per-row by mode entry generation

The wizard computes its `Vec<BulkMapEntry>` by walking the cross-product of committed rows and selected modes:

```text
selected_modes: Vec<String> = if apply_to_all_modes {
    // ModeTree::all_modes returns Vec<&str>; entries own their mode strings.
    profile.modes().all_modes().iter().map(|s| (*s).to_owned()).collect()
} else {
    vec![mode_picker_value]
}

for each row in user-committed rows:                # excludes "(do not map)" and skip-on-conflict
    for each mode in selected_modes:
        existing = profile.find_mapping(row.input, mode)
        if existing.is_some() and not row.replace_in_this_mode:
            continue                                # skip-on-conflict default
        entries.push(BulkMapEntry { row.input, mode, row.target })
```

Conflict resolution is per `(row, mode)`. With `Apply to all modes` checked, a single row that is already mapped in three of five modes correctly creates two mappings and skips three, unless the user has toggled the row's replace chip (in which case all five are committed as replacements).

### Failure semantics

- **Pre-snapshot save failure** (step 1): the handler aborts before the snapshot. In-memory state is unchanged. A warning is pushed to the GUI's warnings channel: "Bulk-map aborted: could not save profile before snapshot." Disk and memory remain in sync. Typically caused by permissions or disk-full on the profile path.
- **Snapshot creation failure** (step 2): the handler aborts before any upsert. Profile is unchanged on disk and in memory (the pre-snapshot save in step 1 succeeded but is a no-op against the previous on-disk body). A warning is pushed: "Bulk-map aborted: could not create recovery snapshot." The user can retry once the underlying issue (typically disk space or permissions on the snapshot directory) is resolved.
- **Per-entry validation failures** (e.g. an entry references a mode that disappears between dispatch and handle): GUI filters before dispatch. Engine treats entries as authoritative; a stale mode reference produces a mapping into a now-orphan mode, which is benign in the data model. Existing `Profile::from_raw` validation flags orphan mappings on next reload.
- **Post-bulk save failure** (step 3): the in-memory upserts are committed; the disk write logs a warning. Disk now holds the pre-bulk state (the post-bulk save did not land); memory holds the post-bulk state. The recovery snapshot, taken in step 2, captures the same pre-bulk state as disk, so a Restore via the snapshot index UI is a visual no-op but is correct (and resyncs memory to disk on reload). On the next normal `LoadProfile` or app restart the user loses the bulk apply silently; the warnings-channel toast is the only signal.
- **Bulk command sent without a loaded profile**: warn-and-ignore before any save or snapshot. Identical to the existing `SetMapping` handler.

### What this design avoids

- Per-entry disk writes: a 40-row apply does two saves total (pre-snapshot, post-bulk), not 40.
- Touching the existing `SetMapping` command: per-mapping editing flows continue unchanged.
- A parallel upsert code path: `set_mappings_bulk` is a thin loop over the existing `set_mapping`.
- Holding write locks across IPC: same discipline as existing handlers.

---

## Testing strategy

Tests at every layer. Test names below are the concrete sentences the implementor writes into `#[test] fn ...()`.

### Layer 1: `BulkMapEntry`

Trivial struct with derives. No dedicated tests beyond what the compiler enforces.

### Layer 2: `Profile::set_mappings_bulk` (in `profile/mod.rs`)

```text
profile_set_mappings_bulk_with_empty_entries_is_noop
profile_set_mappings_bulk_creates_single_mapping_with_unnamed_passthrough
profile_set_mappings_bulk_creates_one_mapping_per_entry_across_modes
profile_set_mappings_bulk_replaces_existing_mapping_overwriting_name_and_actions
profile_set_mappings_bulk_each_generated_mapping_has_action_vec_of_exactly_one_map_to_vjoy
profile_set_mappings_bulk_each_generated_mapping_has_name_none
profile_set_mappings_bulk_mixed_create_and_replace_in_one_call
profile_set_mappings_bulk_into_unknown_mode_still_upserts_silently
```

Reuses fixtures from existing `set_mapping_*` tests in the same module.

### Layer 3: engine command handler (in `engine/tests.rs`)

```text
engine_set_mappings_bulk_writes_profile_to_disk_exactly_twice_per_apply
engine_set_mappings_bulk_with_no_profile_loaded_is_noop_and_logs_warning
engine_set_mappings_bulk_sets_pending_output_refresh_true
engine_set_mappings_bulk_n_entries_completes_in_single_handler_pass
engine_set_mappings_bulk_creates_auto_before_bulk_map_snapshot_between_pre_save_and_upserts
engine_set_mappings_bulk_pre_snapshot_save_failure_aborts_and_logs_warning
engine_set_mappings_bulk_aborts_apply_when_snapshot_creation_fails
engine_set_mappings_bulk_abort_path_does_not_leak_state_write_lock
engine_set_mappings_bulk_post_bulk_save_failure_keeps_in_memory_state_and_logs_warning
engine_set_mappings_bulk_skips_entries_with_unbound_input
```

The "writes exactly twice" test counts profile saves on the happy path via tempfile mtime or a wrapped save call: one before the snapshot, one after the upserts. The abort-path tests (`pre_snapshot_save_failure_aborts_and_logs_warning`, `aborts_apply_when_snapshot_creation_fails`) cover the lower save counts when the handler bails before step 3. The "creates snapshot between pre-save and upserts" test asserts that the snapshot's `taken_at` falls strictly between the two save mtimes, capturing the pre-bulk authored state. The "aborts when snapshot fails" test injects a snapshot-creation failure (e.g. read-only snapshot dir) and asserts the profile mappings are unchanged plus a warning was pushed to the warnings channel. The "abort path does not leak write lock" test asserts that after a snapshot-creation failure, a subsequent `state.read()` succeeds without timeout. The "skips entries with unbound input" test passes a `BulkMapEntry { input: InputAddress::Unbound, ... }` and asserts no mapping is created (defensive: GUI never dispatches Unbound, but the engine treats the variant as user error).

### Layer 4: `SnapshotKind::AutoBeforeBulkMap` (in `snapshot/tests.rs`)

```text
snapshot_kind_auto_before_bulk_map_serializes_to_snake_case
snapshot_kind_auto_before_bulk_map_round_trips_through_toml
snapshot_kind_auto_before_bulk_map_creates_unpinned_snapshot
snapshot_kind_auto_before_bulk_map_always_fires_never_deduped
```

Mirrors the existing tests for `AutoSessionStart` and `AutoBeforeRestore`.

### Layer 5: GUI panel (in `frame/bulk_map/tests.rs`, new module)

Dioxus SSR pattern from `frame/mapping_list/tests.rs` and `frame/mapping_editor/tests.rs`. Per-test fixtures construct an `AppContext` plus `AppState` snapshot, render the panel via `dioxus_ssr::render`, and assert against the rendered DOM string. Command-dispatch tests inspect the mock `Sender<EngineCommand>` queue via `commands.try_iter()`.

```text
panel_renders_no_signal_when_virtual_devices_empty
panel_disables_apply_button_when_virtual_devices_empty
panel_source_picker_lists_only_connected_devices
panel_target_picker_lists_only_live_detected_vjoys
panel_target_select_filters_options_to_chosen_vjoys_capacity
panel_auto_maps_axis_index_to_vjoy_axis_in_enum_order
panel_auto_maps_button_index_to_vjoy_button_id_plus_one
panel_auto_maps_hat_index_to_vjoy_hat_id_plus_one
panel_overflow_axis_rows_default_to_do_not_map
panel_conflict_row_renders_dimmed_and_in_skip_state_by_default
panel_per_row_replace_chip_promotes_row_to_replace_state
panel_summary_chip_counts_match_row_states
panel_summary_chip_includes_across_n_modes_when_apply_to_all_modes_checked
panel_apply_to_all_modes_dims_mode_picker_when_checked
panel_group_header_skip_all_conflicts_skips_only_that_groups_conflicts
panel_group_header_replace_all_conflicts_replaces_only_that_groups_conflicts
panel_group_header_bulk_actions_hidden_when_no_conflicts_in_group
panel_apply_dispatches_single_set_mappings_bulk_command_with_snapshot_label
panel_apply_excludes_skip_rows_and_do_not_map_rows_from_dispatch
panel_apply_to_all_modes_emits_one_entry_per_committed_row_per_mode
panel_apply_to_all_modes_with_per_mode_conflict_skips_only_conflicting_modes
panel_apply_immediately_closes_panel_setting_panel_slot_to_none
panel_apply_enqueues_success_toast_with_create_plus_replace_count
panel_esc_dismisses_panel_silently_with_no_confirm
panel_apply_button_label_renders_create_plus_replace_count
panel_axis_row_renders_compact_bipolar_bar
panel_button_row_renders_filled_or_stamped_dot
panel_hat_row_renders_cardinal_letter
```

### Layer 6: workspace-level smoke (cargo-runnable)

One end-to-end smoke test, runnable via `cargo test`:

```text
smoke_bulk_map_full_round_trip_creates_correct_profile_state
```

Constructs a profile with one mode, no mappings, one device with four axes, eight buttons, one hat. Constructs an engine with a mock vJoy output reporting one vJoy with eight axes, thirty-two buttons, one hat. Pushes a single `SetMappingsBulk` command through the engine command channel. Asserts the resulting profile has thirteen mappings (4 + 8 + 1), each with `name: None` and `actions == vec![MapToVJoy { output: ... }]`. Verifies one snapshot was created with kind `AutoBeforeBulkMap` as a side effect of the bulk handler. Pure `cargo test`, no `dx run`.

### What the test suite explicitly does not test

- SDL3 device enumeration (mock at the `AppState.devices` boundary; trust the SDL3 layer).
- The vJoy driver acquire path (mock at `AppState.virtual_devices`).
- Live readout pixel rendering for `LiveReadout` (covered by existing F9 tests). The new `row_readout` sibling component is covered by layer-5 wizard tests below; it does not share rendering code with `LiveReadout`.
- The `Select`, `Checkbox`, `Button` primitives (covered by existing component tests).
- Trivial standard library behaviour (`Vec::push`, `String::clone`).

### Manual verification (interactive, `dx run`-only)

Listed for the implementation plan, not the automated test suite:

- Open the wizard from the tools cluster on a profile with one connected stick and one configured vJoy. Verify auto-mapping points axis 0 to X, axis 1 to Y, etc. Verify the per-row live readout updates as the stick moves.
- Trigger a conflict by pre-mapping one button manually, then opening the wizard. Verify the row dims and shows `already mapped: ...`. Verify the per-row replace chip toggles correctly and the summary chip counts update.
- Apply with `Apply to all modes` checked on a profile with three modes; verify the snapshot was taken; verify the apply count matches expectations across modes.
- Disable vJoy in vJoyConf, restart, open the wizard; verify the no-signal empty state renders and Apply is disabled.

---

## File layout and scope of code touched

### `inputforge-core` (modifications)

- `src/engine/command.rs`: add `EngineCommand::SetMappingsBulk { entries: Vec<BulkMapEntry> }`. Update existing `Debug` and `PartialEq` test for the new variant.
- `src/engine/run.rs`: add the `SetMappingsBulk` arm in the command-drain match, plus the `set_mappings_bulk` private method.
- `src/engine/tests.rs`: add layer-3 tests.
- `src/snapshot/types.rs`: add `SnapshotKind::AutoBeforeBulkMap`.
- `src/snapshot/tests.rs`: add layer-4 tests.
- `src/profile/mod.rs`: add `Profile::set_mappings_bulk` plus layer-2 tests.

### `inputforge-core` (new files)

- `src/action/bulk.rs`: `BulkMapEntry` definition. Re-exported from `action/mod.rs`.

### `inputforge-gui-dx` (new files)

A new module `src/frame/bulk_map/`:

```text
mod.rs              entry: BulkMapPanel component, props, mount logic
state.rs            wizard state machine: rows, per-row target overrides, replace flags
auto_map.rs         positional auto-mapping logic
conflicts.rs        per-(row, mode) conflict detection
group_actions.rs    skip-all / replace-all / include-all / exclude-all chip logic
summary.rs          summary chip count computation
apply.rs            entry generation: rows by modes to Vec<BulkMapEntry>; dispatches the SetMappingsBulk command with snapshot label
row_readout.rs      sibling compact live readout: bipolar bar (axis), filled-or-stamped dot (button), cardinal letter (hat); reads AppState.input_cache like LiveReadout
empty_state.rs      no-signal framing when virtual_devices is empty
tests.rs            layer-5 tests (Dioxus SSR)
```

A new asset:

- `assets/frame/bulk_map.css`: panel-scoped CSS following the existing `.if-bulk-map__*` BEM pattern.

### `inputforge-gui-dx` (modifications)

- `src/frame/top_bar/tools_cluster/logic.rs`: add the bulk-map toggle handler that sets `view.panel_slot` to `PanelSlot::BulkMap` (or back to `PanelSlot::None` on second click of the same button).
- `src/frame/top_bar/tools_cluster/mod.rs`: render the new icon button.
- `src/frame/view_state.rs`: extend `PanelSlot` enum at `view_state.rs:28` with a `BulkMap` variant. `ViewState.panel_slot` already carries this; no new signal added.
- `src/frame/panel_slot/mod.rs`: add a `PanelSlot::BulkMap` arm to the `match` at `panel_slot/mod.rs:27` that mounts `BulkMapPanel` inside the existing `<aside>` scaffolding. If 460px conflicts with the aside's CSS width, add a per-variant width override here.
- `src/frame/mod.rs`: `pub mod bulk_map;` so `panel_slot/mod.rs` can import the `BulkMapPanel` component.
- `src/icons/mod.rs`: add the tools-cluster icon glyph (specific SVG drawn at implementation time).

### Cross-cutting reuses (no modification)

- `crate::toast::queue`: enqueue the post-apply success toast (`X mappings created`).
- `crate::frame::mapping_editor::live_readout`: not reused; F9 is unmodified. The wizard ships its own sibling `row_readout` component (see new files above) that draws from the same `AppState.input_cache` data source but renders axes, buttons, and hats per the wizard grid template (LiveReadout handles axes only and is layout-coupled to the editor).
- `crate::components::*`: `Select`, `Checkbox`, `Button`, `IconButton`, badge.

### No new component primitives

The wizard introduces no new reusable primitives (no `SidePanel`, no segmented control, no new chip shape). All affordances are built from existing primitives or one-off CSS in `bulk_map.css`. Extraction is deferred until a second consumer of any candidate primitive appears.

### Dependencies on existing features

- F6 snapshot/preferences: extended (new `SnapshotKind` variant). Strictly additive.
- F8 mapping list: borrows the conflict-detection pattern from `add_inline.rs` for per-(row, mode) computation. F8 itself is not modified.
- F9 mapping editor: shares the live data-source convention (`AppState.input_cache` reads via the same `AppContext` patterns). The wizard's `row_readout` is a new sibling component, not a borrow of `LiveReadout`. F9 itself is not modified.
- Toast system: reused. Not modified.
- Tools cluster: extended with one new button. Strictly additive.

No blocking dependencies on unbuilt features. The wizard ships in isolation.

---

## Deferred follow-ups

Locked into the spec as **explicitly deferred**, not omissions:

- **Persisted per-(source, target) override replay.** Adds a small key-value store keyed on `(DeviceId, vjoy_id)` storing the user's last per-row override choices, applied on next open. Single follow-up spec when the v1 wizard has been used enough to know which overrides repeat.
- **Offline authoring of disconnected devices.** Requires per-device input-count metadata in the profile (axes, buttons, hats counts that SDL3 cannot report when the device is unplugged). Touches the profile schema, not just this feature.
- **Default-action templates.** "Every axis gets a default deadzone", "every axis gets a linear curve", etc. Surfaces as an `Action template` knob in the wizard; affects every generated mapping uniformly.
- **Auto-naming of generated mappings.** "Axis 0", "Btn 5", or device-name-aware variants ("Roll", "Pitch") if the source device exposes named axes.
- **Panel-level bulk operations.** "Skip all conflicts" / "replace all" across the whole table, in addition to per-group. Build only if per-group affordances prove insufficient in practice.
- **Segmented mode-scope control.** A `[ This mode | All modes ]` segmented control replacing the picker plus checkbox combo, with the picker collapsing when "All modes" is selected. Requires a new component primitive and is its own design decision.
- **Reusable `SidePanel` primitive.** Extract when the snapshot manager, profile properties, or any second side-panel consumer ships.
- **Live keyboard output preview.** The wizard never generates `MapToKeyboard` actions; if it ever grows that capability, the per-row live readout column would extend to handle key-combo chips.
- **Mapping-list rename to allow blank.** Out of scope here; the wizard's blank-name design follows the existing `+ Add mapping` contract. The rename gate against empty values is its own bug, filed separately.
