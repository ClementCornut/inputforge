# F15, Settings UI (preferences editor surface): Design Spec

**Status:** Design approved, ready for implementation plan
**Date:** 2026-05-09
**Parent specs:**
- [`2026-04-24-egui-to-dioxus-rewrite-design.md`](./2026-04-24-egui-to-dioxus-rewrite-design.md), master plan, feature F15
- [`2026-04-27-f5-architecture-ia-redesign-design.md`](./2026-04-27-f5-architecture-ia-redesign-design.md), IA pass that scoped F15
- [`2026-04-28-f6-snapshot-preferences-core-design.md`](./2026-04-28-f6-snapshot-preferences-core-design.md), data layer F15 sits on top of

**Predecessors:** F1 (state bridge), F2 (design system), F3 (shell), F4 (toast/dialog), F5 (IA), F6 (`AppSettings` schema, `SnapshotConfig`, `EngineCommand::ReloadSettings`, snapshot `list` / `prune`), F7 (chrome shell + tools cluster + Replace discipline), F12 (Devices panel reference shape), F13 (Profiles panel reference shape).
**Design system:** [`/DESIGN.md`](../../DESIGN.md)
**Product brief:** [`/PRODUCT.md`](../../PRODUCT.md)
**Brainstorm artefacts:** wireframes at `.superpowers/brainstorm/1220-1778318545/content/` (`surface-form-v2.html`).

---

## Context

F15 is the editor surface that sits on top of `AppSettings` (F6). F6 already ships the schema, the on-disk TOML at `%APPDATA%/inputforge/settings.toml`, the `[snapshot]` sub-table with `max_count` and `skip_if_unchanged`, and `EngineCommand::ReloadSettings` for re-reading the file. F15 adds the GUI by which a user changes these values without hand-editing TOML.

Two pieces of upstream guidance had to be reconciled before the design was approved:

1. **DESIGN.md §6 forbids settings-in-a-dialog.** The forbidden-dialog list at DESIGN.md:336 includes "Settings opened from a settings panel". The F5 line at `2026-04-27-f5-architecture-ia-redesign-design.md:644` ("the settings surface itself, panel sub-section or dialog, F15 brainstorm decides") predates the DESIGN.md hardening; F15 narrows the choice to the legal subset.
2. **F6 ships no file watcher.** The F15 spec line in F5 ("after commit, the surface reflects the post-`ReloadPreferences` state... by surfacing a non-destructive warning + reload action") assumed file-change detection that F6 did not deliver. F15 narrows the contract to match what F6 actually built: hand-edits during a session are silently overwritten by the next GUI commit; we accept that limitation rather than bolt on a watcher.

Per PRODUCT.md "live data is the contract", this spec accepts that the GUI cannot detect concurrent hand-edits to `settings.toml` during a session. Hand-edits made while the panel is open are out-of-band writes the engine does not observe; the next GUI commit silently overwrites them. This is a known limitation, not a defect, and is not addressed by F15.

The brainstorm also confirmed that F15's editable scope today is exactly the two `SnapshotConfig` fields. Other `AppSettings` fields are owned elsewhere (`device_aliases` by F12 Devices panel; `last_profile` and `device_registry` are implicit, not user-facing). The panel structure is built additive-first so future settings sections drop in without rework.

---

## Confirmed design choices

The decisions below are recorded in dependency order, each surfaced and approved during brainstorming.

### Surface form

**1. Settings is the third right-side panel**, parallel to Devices and Profiles. It plugs into F7's existing shared panel slot via the established Replace discipline: opening Settings closes whichever of Devices/Profiles was open, and vice versa. Tools-cluster ordering is `Devices · Profiles · Settings`, with Settings at the end so existing muscle memory is undisturbed. Label is text "Settings" matching the cluster's convention; no gear icon.

DESIGN.md §6 mandates side panels for multi-field edits. Even though the F15 schema is two fields today, the side panel is the surface that grows naturally as the schema does, and the dialog forbidden-uses list ruled out the only non-panel alternative.

**2. Settings is reachable when no profile is loaded.** Settings is app-global, not profile-scoped. The button stays enabled regardless of `meta.profile_name`. Compare Devices and Calibration in F7 which disable when no profile is loaded; Settings does not share that constraint. The `would_prune` check (Choice 8) returns 0 when no profile is loaded, so the prune-confirm step skips silently in that state.

**3. No first-time onboarding hint.** PRODUCT.md L55 ("Onboarding is minimal; assume the user knows what a deadzone is") and the anti-reference list (PRODUCT.md L44, Generic SaaS dashboard) both rule out a first-launch toast pointing at Settings. The button is in the chrome; users who want it find it.

### Panel layout

**4. Additive `SettingsSection` stack from day one.** The panel body is a vertical stack of `SettingsSection` components. Each section has a heading and a body of stacked field rows. F15 ships exactly one section, `Snapshots`, but the section component and its heading ship from day one so future additions are pure-add. Inter-section spacing `space-6` (24px); inter-row spacing `space-3` (12px). The panel body scrolls when content exceeds viewport height (`overflow-y: auto` + `min-height: 0`), matching the Profiles panel library's pattern.

**5. Two new local primitives, panel-scoped.**
- `SettingsSection { heading: &'static str, children: Element }`, owns heading typography and inter-section spacing.
- `SettingsFieldRow { label: &'static str, helper: &'static str, control: Element }`, owns label/helper/control rhythm so every field row in every section reads the same. The row also owns the ARIA wiring for its child control: it generates the `<label for="...">` link, the helper text id, and the `aria-describedby` / `aria-invalid` / `aria-errormessage` attributes. Wrapped controls (`IntegerInput`, `Switch`) inherit those linkages without each component needing its own `aria_describedby` or `aria_label` props.

These do not graduate to F2 atomics; they are F15 internals. If a second feature later needs the same shapes, promotion is its own ticket.

**6. Section heading typography.** Label (12px/500), uppercase, letter-spacing 0.06em, `text-muted` color, full-width `border-bottom: 1px solid var(--color-border)`. Neutral hue: DESIGN.md categories (processing teal, output gold, control violet) are pipeline taxonomy markers and do not apply to settings groupings. The same heading style is used for every section regardless of subject.

### Field set (Snapshots section)

**7. Two field rows, in this order:**

| Order | Field | Label | Control | Helper text |
|---|---|---|---|---|
| 1 | `snapshot.max_count` | Snapshot buffer size | `IntegerInput` (`Signal<usize>`, `min: 1`, `max: 100`, mono) | "Maximum number of unpinned snapshots kept per profile. The oldest are auto-evicted. Pinned snapshots are kept regardless." |
| 2 | `snapshot.skip_if_unchanged` | Skip startup snapshot if unchanged | `Switch` | "Don't take a snapshot at app start when the active profile is identical to the most recent snapshot." |

Label uses Body (14/400). Helper uses Caption (11/400) with `text-muted`. The `IntegerInput` ships with the dense-row inset focus-ring variant by default for settings (class `.if-integer-input--inset` applied via the `class` consumer prop). `IntegerInput` is a new F2 component introduced by F15 (see "Component additions to F2" below); it operates on `usize` natively, so no f64 conversion is needed in the F15 path. `Switch` is the F2 primitive; track flips to primary on, muted on off.

Field label "Snapshot buffer size" matches the dialog copy in Choice 10 ("Reduce snapshot buffer to *N*?") so the term is consistent across the surface.

(Note on the DESIGN.md drift: DESIGN.md §7 names the inset variant `.if-input--inset`, but the implemented `NumberInput` uses `.if-number-input--inset` at `assets/components/number-input.css:15`. F15 sidesteps this drift by introducing its own `IntegerInput` with `.if-integer-input` and `.if-integer-input--inset` classes; the underlying NumberInput is not consumed in F15.)

### Commit model

**8. Auto-commit on blur/Enter.** Each field commits independently when focus leaves it. No Apply or Cancel buttons. The panel does not maintain an in-memory clone of `AppSettings`; it binds directly to a polled `Signal<SettingsSnapshot>` (Choice 11-bis) and dispatches engine commands when local edits commit.

The `IntegerInput` and `Switch` each own a small local Signal for the in-flight value during interaction. Focus-state semantics:

| Focus state | Polled snapshot changes | Behavior |
|---|---|---|
| Unfocused | yes | Local Signal := polled value (mirror through). |
| Focused, value pristine (== polled) | yes | Local Signal := polled value (mirror through). |
| Focused, value dirty (!= polled) | yes | Local Signal unchanged; on Escape, snap to polled value; on commit (Enter/blur with valid value), dispatch the user's value. |

The "pristine while focused" row covers the case where a user clicks into the field but does not type before an external `ReloadSettings` or peer GUI commit lands; in that case the polled value flows through immediately. Once the user types and the in-flight value diverges from the polled value, the field becomes dirty and external changes are deferred until the next commit, blur, or Escape.

This diverges from the F5/F15-line proposal of "in-memory clone + Apply/Cancel" on three grounds:

- F5 §8 already commits the GUI to "auto-commit + session undo + on-disk snapshots". Calibration is the documented sole exception ("the only explicit-save surface"). An Apply/Cancel settings panel would be a second exception with weaker justification.
- DESIGN.md §6 inline-edit primitive: "A numeric field accepts a click, a focus ring appears, the value can be typed or scrubbed, and the change commits on blur or Enter." Auto-commit is the project's declared default for inline editing.
- The "atomic settings change" use case Apply/Cancel solves does not match settings reality: skip_if_unchanged is a single boolean toggle, max_count is a single integer, the two fields do not have any joint invariant that requires committing them together.

The F5 acceptance bullet "Cancel after edits leaves the on-disk TOML byte-identical to its pre-edit state" is dropped: there is no Cancel under auto-commit. Reverting a value means typing the previous value back. Settings are not on the per-mapping session-undo stack; their change frequency is too low to justify a bespoke undo path.

**9. Validation: `max_count` accepts `1..=100`; switch is always valid.**

- Out-of-range input or input that does not parse as `usize` shows an inline error: error-red border, helper text replaced with "Must be between 1 and 100", commit blocked.
- Escape during editing reverts the displayed value to the last-committed value.
- Enter or blur with a valid value triggers the commit pipeline.
- Lower bound 1 disallows 0 (which would auto-evict all unpinned snapshots, surprising). Upper bound 100 caps filesystem noise (each snapshot is a few KB; 100 is plenty for any realistic tuning history).

**10. Destructive prune confirmation.** When committing a new `max_count` that would evict at least one currently-existing unpinned snapshot in the active profile, the GUI opens the shared `DestructiveConfirmDialog` (new pattern under `patterns/destructive_confirm.rs`, see "Component additions to F2" below) before dispatching the command:

> **Reduce snapshot buffer to *N*?**
>
> *K* unpinned snapshots will be deleted from *<active-profile>*. Pinned snapshots are kept.

Cancel button (default focus) reverts the displayed value to the last-committed value and dispatches nothing. Reduce button (danger variant) dispatches `EngineCommand::SetSnapshotConfig`. The shared dialog is F4's destructive-shape primitive in concrete form, matching F5's destructive-confirm trigger table.

**11. `would_prune` is computed locally from the polled `SettingsSnapshot`.** The new `Signal<SettingsSnapshot>` (Choice 11-bis) carries `unpinned_snapshot_count: usize`, computed each polling tick by the engine using its existing `resolve_snapshot_namespace` helper at `crates/inputforge-core/src/snapshot/pending_delete.rs:44` (which already routes library vs external profiles correctly) plus `snapshot::list_in(&namespace_dir)`. On commit, the snapshots section computes `would_prune = unpinned_snapshot_count.saturating_sub(candidate_max)`. No GUI fs read; no new query channel; correct for both library and external profiles automatically. When no profile is loaded or namespace resolution fails, `unpinned_snapshot_count` is 0, so the prune-confirm step skips silently. The computation is invoked once at commit time (after blur/Enter, before dispatch); not during keystrokes; not on the polling tick itself.

**11-bis. `Signal<SettingsSnapshot>` projection backed by an engine-mirrored `AppState` field.** Add a new public `snapshot_config: SnapshotConfig` field to `AppState` (in `crates/inputforge-core/src/state/mod.rs`), written by the engine on every path that mutates `self.settings.snapshot` (startup, `ReloadSettings`, the new `SetSnapshotConfig`). This matches the existing `device_aliases` mirror at `engine/run.rs:554-560`, so no architectural precedent is created.

Add a new `settings: Signal<SettingsSnapshot>` field to `AppContext` (in `crates/inputforge-gui-dx/src/context.rs`), populated by the existing 16ms polling task in `bridge.rs`. The struct shape:

```rust
pub struct SettingsSnapshot {
    pub snapshot: SnapshotConfig,
    pub unpinned_snapshot_count: usize,
}
```

A new `SettingsSnapshot::from_state(state: &AppState) -> Self` reads `state.snapshot_config` directly and computes `unpinned_snapshot_count` via the existing `resolve_snapshot_namespace(state)` (promoted from `pub(crate)` to `pub`) plus `snapshot::list_in(&namespace_dir)`. `resolve_snapshot_namespace` only reads `state.profile_path` and `state.active_profile_origin`, so it is independent of the new mirror field. Falls back to 0 when no profile is loaded or namespace resolution fails. The projection is gated on `PartialEq` inequality like the existing `MetaSnapshot` / `ConfigSnapshot` / `LiveSnapshot` projections. The legacy `AppContext.settings: Arc<AppSettings>` field is dropped (was dead-code, marked `#[expect(dead_code)]`).

### Engine command and write path

**12. New `EngineCommand::SetSnapshotConfig { config: SnapshotConfig }`** in `crates/inputforge-core/src/engine/command.rs`. Surgical: it replaces `settings.snapshot` only, not other `AppSettings` fields. Future settings sections add their own per-section commands following this shape. No generic `SetSettings(AppSettings)` blob command is introduced; that would race with `SetDeviceAlias` and friends and would force the GUI to send a full settings copy on every per-field commit.

**13. Engine handler logic for `SetSnapshotConfig`:**
1. Capture `old_config = self.settings.snapshot.clone()`.
2. Replace `self.settings.snapshot = config`. Call `AppSettings::save()` to persist `settings.toml`. If `save()` returns Err, restore `self.settings.snapshot = old_config` so the in-memory state matches the on-disk truth, push a warning to the warnings channel, emit a tracing event recording the failure, and return without attempting step 3.
3. If `config.max_count < old_config.max_count` and a profile is loaded, compute the namespace dir via the existing `resolved_snapshot_target` helper at `engine/run.rs:1098-1114` (which wraps `resolve_snapshot_namespace` and is the canonical accessor for `(profile_path, namespace_dir)`), then call `snapshot::prune_in(&namespace_dir, &config)`. Pinned snapshots are exempt per existing `prune_in` semantics. This matches every other prune call site in `engine/run.rs`.
4. Emit a `tracing::info!(target: "settings", old_max_count, new_max_count, pruned)` event with structured fields for observability.

The on-disk write is independent of the prune step: a prune failure (rare; fs error) does not roll back the settings save. The handler emits a separate tracing event for the prune failure and propagates a warning to the toast channel; the in-memory `settings.snapshot` keeps the new value (matching the on-disk truth).

**14. No file watcher; concurrent-edit overwrite is silent.** F6 reads `settings.toml` once at startup and on `ReloadSettings`. If a user hand-edits `settings.toml` while the GUI panel is open and then commits a field in the GUI, the GUI's commit overwrites their hand-edit. This is documented as a known limitation: the spec does not bolt a watcher on for an edge case the F6 spec did not commit to detecting. External changes are propagated to the panel only on the next polling tick after a manually triggered `ReloadSettings`; in practice, since no UI affordance triggers `ReloadSettings` today, hand-edits during a session are picked up only on next app launch. The Context-level acknowledgment above frames this as an accepted live-data-contract trade-off.

### Toasts and feedback

**15. Toasts only when results aren't visible on the surface.**

| Event | Toast |
|---|---|
| Switch toggled | None. The switch state on the surface is the answer. |
| `max_count` committed, no prune | None. The new value is visible in the field. |
| `max_count` committed, prune ran | Success toast: "Snapshot buffer set to *N*. *K* removed from *<profile>*." |
| Inline validation error | None. Inline helper text is the answer. |
| Engine command failure | Error toast surfaces the underlying error via the existing warnings channel. |
| Settings file write failure inside the engine handler | Error toast: "Could not save settings: *<reason>*." In-memory `settings.snapshot` rolls back to the previous value on the next polled snapshot. |

### Accessibility and keyboard

**16. ARIA shape:**
- Panel container: `<aside aria-label="Settings" role="region">`. No panel-level header; matches the existing Devices and Profiles panels which `panel_slot/mod.rs:144-150` explicitly tests render no `<h2>`, no `if-panel-slot__header`, no `if-panel-slot__title`.
- Section heading: `<h3>` is the topmost panel heading. AT navigates the panel by section.
- Field row: `SettingsFieldRow` provides the `<label for="...">` linked to its control plus `aria-describedby` pointing at the helper text id. Invalid state uses `aria-invalid="true"` and `aria-errormessage` pointing at the error helper. Wrapped controls (`IntegerInput`, `Switch`) do not need their own `aria_describedby` props; the field row owns the wiring.
- The shared `DestructiveConfirmDialog` (Choice 10) reuses `DialogRoot`'s focus-trap and ESC dismissal. Cancel button receives default focus via the existing `onmounted` pattern in `DialogRoot`.

**17. Keyboard navigation:**
- Tab order: Settings tools-cluster button → panel container → first field control → second field control. The Settings button reaches focus from the rest of the chrome via F7's existing tab order, after Profiles.
- Enter on the Settings button toggles the panel open/closed (matches Devices/Profiles).
- Escape blurs the focused control inside the panel; the panel itself is non-focus-trapping per DESIGN.md §6.
- NumberInput: type to edit, Enter to commit, Escape to revert.
- Switch: Space toggles, Enter toggles.

### Motion

**18. Reuse F7's existing right-side panel motion.** Slide-in/out with the Replace discipline animation already in place. Under `prefers-reduced-motion` the existing fallback applies (slide drops, opacity remains). Nothing F15-specific.

---

## Non-goals (out of scope for this spec)

- **Pixel-level visual treatment.** F15 commits structure and behaviour, not aesthetics. Visual passes happen via `impeccable:frontend-design` and `impeccable:polish` invocations during the implementation plan.
- **A second AppSettings field set.** Only `snapshot.*` is editable in F15. Adding more fields (theme, window size memory, default profile path) is a follow-up that drops in as a new `SettingsSection`.
- **File watcher / external-change detection.** Out of scope per Choice 14.
- **Settings undo.** Out of scope per Choice 8.
- **Light theme.** Out of scope for the whole rewrite per parent plan.
- **Localization / i18n.** Not present today; out of scope.

---

## Architecture

### File-by-file change list

**`crates/inputforge-core/`:**

| Path | Change |
|---|---|
| `src/state/mod.rs` | Add `pub snapshot_config: SnapshotConfig` field to `AppState`. Initialise from `AppSettings.snapshot` at engine construction time. This is the truth-source the GUI projection reads; the engine mirrors `self.settings.snapshot` into it on every mutation path. |
| `src/engine/command.rs` | Add `EngineCommand::SetSnapshotConfig { config: SnapshotConfig }` variant. Extend the existing `tests::debug_format_contains_variant_name` and PartialEq tests to cover the new variant per the established pattern. |
| `src/engine/run.rs` | Add the `SetSnapshotConfig` arm in `process_commands` per Choice 13. Reuses the existing `resolved_snapshot_target` helper at `run.rs:1098-1114` for namespace dispatch and calls `snapshot::prune_in(&namespace_dir, &config)`, matching every existing prune call site. Engine startup, `ReloadSettings`, and the new `SetSnapshotConfig` arm all write `self.state.write().snapshot_config = self.settings.snapshot.clone()` after `self.settings` mutations, mirroring the `device_aliases` pattern at `run.rs:554-560`. |
| `src/engine/tests.rs` | Add the engine-handler tests listed in Acceptance (including `set_snapshot_config_save_failure_does_not_persist`). |
| `src/snapshot/pending_delete.rs` | Promote `resolve_snapshot_namespace` from `pub(crate)` to `pub` so the GUI projection (`SettingsSnapshot::from_state`) can call it. The function reads only `state.profile_path` and `state.active_profile_origin`; it does not depend on the new `snapshot_config` mirror. |

No changes to `crates/inputforge-core/src/settings.rs` (schema, `save`, `save_to` already in place from F6). No changes to `crates/inputforge-core/src/snapshot/mod.rs` (`list_in` and `prune_in` already public from F6).

**`crates/inputforge-gui-dx/`:**

| Path | Change |
|---|---|
| `src/components/integer_input.rs` | New: `IntegerInput` component per "Component additions to F2" below. Operates on `usize` natively; reuses NumberInput's stepper/focus/validation CSS via shared layout class plus a new `.if-integer-input` modifier. |
| `src/components/integer_input/tests.rs` (or co-located `#[cfg(test)] mod tests`) | Component tests: parse-valid, parse-invalid, clamp-min, clamp-max, oncommit dispatches on Enter, oncommit dispatches on blur, Escape reverts. |
| `src/components/mod.rs` | Re-export `IntegerInput`. |
| `src/patterns/destructive_confirm.rs` | New: shared 2-button (Cancel + Danger) confirm dialog. Props per "Component additions to F2" below. Cancel receives default focus. Uses `DialogRoot { dismissible: true, close_on_backdrop_click: false }`. |
| `src/patterns/mod.rs` | Re-export `DestructiveConfirmDialog`. |
| `src/context.rs` | Remove `settings: Arc<AppSettings>` field. Add `settings: Signal<SettingsSnapshot>` (initialised in `app.rs` mirror to existing pattern). Add `pub struct SettingsSnapshot { snapshot: SnapshotConfig, unpinned_snapshot_count: usize }` and `impl SettingsSnapshot { pub(crate) fn from_state(state: &AppState) -> Self }`. |
| `src/bridge.rs` | Extend the polling task to project `SettingsSnapshot::from_state(&state)` into the new Signal each tick, gated on `PartialEq` inequality. Mirror the existing `MetaSnapshot` / `ConfigSnapshot` / `LiveSnapshot` projection pattern at `bridge.rs:16-54`. |
| `src/app.rs` | Update `AppContext` construction (`app.rs:27-34`) to initialize the new `settings: Signal<SettingsSnapshot>` field. |
| `src/frame/view_state.rs` | Extend `PanelSlot` enum with `Settings` variant. Update `Default` impl, serde derives, and any exhaustive match sites. |
| `src/frame/top_bar/tools_cluster/logic.rs` | Add `Tool::Settings` variant; add `(PanelSlot::Settings, _, Tool::Settings)` arm to `tool_active`; extend `logic::tests` with positive and negative cases per the established convention at `logic.rs:36-62`. |
| `src/frame/top_bar/tools_cluster/mod.rs` | Add a third `ToolButton` for Settings after Profiles. Click handler toggles `panel_slot` between `None` and `Settings`. `disabled: false`, `disabled_reason: String::new()` regardless of profile-load state, matching the existing always-enabled Profiles button. |
| `src/frame/panel_slot/mod.rs` | Add a render arm for `PanelSlot::Settings` mounting the new `SettingsPanel`. Update the existing "no header" assertion test (`panel_slot/mod.rs:144-150`) to also cover the Settings variant (assert no `<h2>`, no `if-panel-slot__header`, no title text). |
| `src/frame/settings_panel/mod.rs` | New: panel root component. Renders `<aside aria-label="Settings" role="region">` with no panel-level header. Composes `SettingsSection` children. Reads `ctx.settings`. Lives at `frame/settings_panel/`, sibling to `frame/profiles/`, mirroring the Profiles panel module structure (not nested under `panel_slot/`). |
| `src/frame/settings_panel/section.rs` | New: `SettingsSection` primitive per Choice 5. |
| `src/frame/settings_panel/field_row.rs` | New: `SettingsFieldRow` atom per Choice 5; owns the `<label for=>`, helper text id, `aria-describedby`, `aria-invalid`, and `aria-errormessage` wiring on behalf of its child control. |
| `src/frame/settings_panel/snapshots_section.rs` | New: composes the two snapshot fields. Owns the in-flight Signal for the `IntegerInput`, blur/Enter commit handlers, the local `would_prune = unpinned_snapshot_count.saturating_sub(candidate_max)` computation, and dispatch to `DestructiveConfirmDialog` (via `prune_confirm.rs`) or directly to `EngineCommand::SetSnapshotConfig`. |
| `src/frame/settings_panel/validation.rs` | New: pure-fn `validate_max_count(input: &str) -> Result<usize, ValidationError>`. Tested independently. |
| `src/frame/settings_panel/prune_confirm.rs` | New: thin wrapper around `DestructiveConfirmDialog` carrying the prune-specific copy ("Reduce snapshot buffer to N? K unpinned snapshots will be deleted from <profile>. Pinned snapshots are kept."). Keeps the dialog primitive content-agnostic. |
| `src/frame/settings_panel/tests.rs` | New: component-level tests per Acceptance (including the new pristine-focus and external-change tests). |
| `src/frame/settings_panel/test_helpers.rs` | New (only if a generic helper is needed; defer to plan-time). |
| `src/frame/mod.rs` | Wire the new `settings_panel` module. |
| `src/lib.rs` | Re-export touch points if any (likely none beyond the existing surface). |

**CSS** (`crates/inputforge-gui-dx/assets/`):

| Path | Change |
|---|---|
| New: `assets/frame/settings_panel.css` | Panel layout (no header, scrollable body), `SettingsSection` rhythm, section heading style, `SettingsFieldRow` two-column grid with helper text below the label. Reuses tokens from `tokens.css`; reuses `Switch` and (new) `IntegerInput` styles. Flat snake_case under `assets/frame/`, matching every existing `assets/frame/*.css`. |
| New: `assets/components/integer_input.css` | Base `.if-integer-input` styles plus `.if-integer-input--inset` modifier. May share rules with `number_input.css` via a common dense-row class; decide at plan-time. |

**No new workspace dependencies.** All required crates are already in the workspace.

### Data flow summary

```
Engine state ── polling tick (16ms) ──▶ AppContext.settings: Signal<SettingsSnapshot>
                                          │   { snapshot, unpinned_snapshot_count }
                                          │
                                          ▼
GUI panel reads → IntegerInput.value, Switch.checked
                  unpinned_snapshot_count fed into local would_prune at commit time

GUI field commit (blur/Enter)
       │
       ├─ valid?         no  ──▶ inline error, no dispatch
       │     yes
       │
       ├─ candidate_max < unpinned_snapshot_count?
       │     yes ──▶ DestructiveConfirmDialog (Cancel default | Reduce danger)
       │              │
       │              ├─ Cancel: revert displayed value, no dispatch
       │              └─ Reduce: dispatch
       │     no
       │
       ▼
EngineCommand::SetSnapshotConfig { config }
       │
       ▼
Engine handler:
  1. old_config = self.settings.snapshot.clone()
  2. self.settings.snapshot = config; AppSettings::save()?
       on Err: restore old_config, push warning, return
  3. if config.max_count < old_config.max_count and active profile:
       resolve_snapshot_namespace(&state) -> namespace_dir
       snapshot::prune_in(&namespace_dir, &config)
  4. tracing::info!(target: "settings", old_max_count, new_max_count, pruned)
```

---

## Acceptance

### Engine-layer (`crates/inputforge-core/src/engine/tests.rs`)

1. `set_snapshot_config_writes_settings_toml`: dispatch, assert `settings.toml` on disk has the new `[snapshot]` table.
2. `set_snapshot_config_replaces_in_memory_snapshot`: dispatch, assert subsequent `AppState` reads expose the new config.
3. `set_snapshot_config_prunes_when_max_count_decreased`: seed an active profile with N unpinned and M pinned snapshots, dispatch with `max_count = N/2`, assert FIFO eviction respects pinned flags and the on-disk count drops to `N/2 + M`.
4. `set_snapshot_config_does_not_prune_when_max_count_increased`: seed snapshots, dispatch larger `max_count`, assert no eviction.
5. `set_snapshot_config_no_prune_when_no_profile_loaded`: dispatch with no active profile, assert engine does not error and no prune is attempted.
6. `set_snapshot_config_save_failure_does_not_persist`: inject a write-failing `settings_path` (e.g. parent dir is read-only or path collision), dispatch with a different `max_count`, assert `engine.settings.snapshot` reverts to the pre-command value, no prune is attempted, and the warnings channel received an error message. Verifies Choice 13 step 2 rollback.
7. `set_snapshot_config_prune_failure_does_not_corrupt_settings`: inject a snapshot module fs error, assert `settings.toml` was still written and the in-memory config matches the on-disk file.

### Pure GUI logic

8. `tools_cluster::logic::tests::settings_panel_lights_settings_regardless_of_via_calibration`: matcher returns true for `(PanelSlot::Settings, _, Tool::Settings)` across both `via_calibration = true` and `false`. Mirrors the existing `profiles_panel_lights_profiles_regardless_of_via_calibration` shape at `logic.rs:36-62`.
9. `tools_cluster::logic::tests::settings_panel_does_not_light_other_tools`: mutual exclusion against `Tool::Devices`, `Tool::Calibration`, `Tool::Profiles`.
10. `settings_panel::validation::tests`: `validate_max_count(input: &str) -> Result<usize, ValidationError>` covers in-range, zero, above 100, input that does not parse as `usize`, empty string. Result type matches `SnapshotConfig.max_count: usize`.
11. `context::settings_snapshot::tests::unpinned_snapshot_count_projection_uses_active_namespace`: with a library-loaded profile, `SettingsSnapshot::from_state` resolves the namespace to the `<profile>.snapshots/` sibling and counts unpinned snapshots there; with an external profile, it resolves to `<config_dir>/external_snapshots/<hash>/` and counts there. Both paths return 0 when no profile is loaded.

### Component-level (`settings_panel/tests.rs`)

12. `panel_renders_when_settings_slot_active`: set `panel_slot = Settings`, assert `aria-label="Settings"` region exists with the Snapshots `<h3>` heading and no `<h2>` panel header.
13. `tools_cluster_button_toggles_panel`: click the Settings button, assert `panel_slot` flips to Settings; click again, flips to None.
14. `opening_settings_closes_devices`: set slot to Devices, click Settings, assert slot is now Settings (Replace discipline).
15. `opening_settings_closes_profiles`: symmetric.
16. `max_count_commit_on_blur_dispatches_command`: type a new in-range value, blur the input, assert `EngineCommand::SetSnapshotConfig { config: { max_count: N, .. } }` was sent.
17. `max_count_commit_on_enter_dispatches_command`: same with Enter key.
18. `max_count_invalid_value_does_not_dispatch`: type 0, blur, assert no command sent and helper text shows the validation message. (Verifies the IntegerInput `oninvalid` path: out-of-range input fires `oninvalid` with an `IntegerInputError`, the consumer renders the helper as the error string, and no `EngineCommand::SetSnapshotConfig` is dispatched.)
19. `max_count_invalid_value_reverts_on_escape`: type something, press Escape, assert displayed value reverts to the persisted value.
20. `switch_toggle_dispatches_command_immediately`: click switch, assert `SetSnapshotConfig` with the new bool value (no `DestructiveConfirmDialog` step).
21. `prune_confirm_appears_when_decreasing_max_count_below_unpinned_count`: seed `unpinned_snapshot_count = N` in the polled `SettingsSnapshot`, type `max_count` below `N`, blur, assert `DestructiveConfirmDialog` visible with the right copy. Confirm dispatches; cancel reverts the displayed value.
22. `prune_confirm_does_not_appear_when_no_profile_loaded`: set `unpinned_snapshot_count = 0` (the no-profile branch of `SettingsSnapshot::from_state`), type `max_count` arbitrary, blur, assert no dialog and command dispatched directly.
23. `panel_reflects_external_changes_via_polling_signal`: with the `IntegerInput` unfocused, update mocked `AppState.snapshot_config.max_count` from 10 to 25, advance the polling tick, assert the displayed value updates to 25.
24. `focused_dirty_field_does_not_clobber_in_flight_value`: focus the `IntegerInput`, type 15 without blurring (in-flight value `15` differs from polled `10`), advance a polled-snapshot tick that updates `max_count` to 25 externally via `AppState.snapshot_config`, assert the displayed value remains 15 (user typing wins while focused). On subsequent Escape, the displayed value reverts to 25 (the latest polled value).
25. `focused_pristine_field_mirrors_external_change`: focus the `IntegerInput` WITHOUT typing (in-flight value matches polled `10`), advance a polled-snapshot tick that updates `max_count` to 25, assert the displayed value updates to 25 even while focused. Verifies the "pristine while focused" row of the Choice 8 focus-state table.
26. `panel_reachable_when_no_profile_loaded`: set `meta.profile_name = None`, assert Settings tools-cluster button is enabled and clicking it opens the panel.

### Integration smoke (manual)

27. `dx run -p inputforge-app` → open Settings → toggle each field → close app → reopen → Settings reflect the persisted values. Documented as a verification step in the plan, not as an automated test.

---

## Open questions

- **Section ordering when more sections ship.** F15 ships exactly one section (`Snapshots`). When future sections arrive (theme, default profile path, etc.), the ordering rule is owned by that future feature's spec. F15 does not commit a precedence rule.
- **Per-field reset to default affordance.** Whether each field gets a small "reset to default" button (returning `max_count` to 10, `skip_if_unchanged` to true) is deferred to F16 polish if a user pain point emerges. Default plan: do not ship.
- **Tools-cluster button gear icon.** The button is text-labelled "Settings" today, matching `Devices` and `Profiles`. If F16's icon-strategy review adopts icons across the cluster, Settings inherits the convention. Default plan: text label.
- **`IntegerInput` graduation to F2-proper.** F15 ships `IntegerInput` as a component addition (see "Component additions to F2" below). If a third feature later needs the same shape, promotion to a generic `NumberInput<T>` sibling (full F2 atomic, formal Props parity) is a follow-up ticket. Default plan: keep panel-scoped consumption pattern.

---

## Component additions to F2

F15 introduces two new shared components into the F2 component library:

1. **`IntegerInput`** (`crates/inputforge-gui-dx/src/components/integer_input.rs`).
   Props: `value: ReadSignal<usize>`, `min: usize`, `max: usize`, `oncommit: Option<EventHandler<usize>>`, `oninvalid: Option<EventHandler<IntegerInputError>>`, `oninput: Option<EventHandler<FormEvent>>`, `disabled: bool`, `id: Option<String>`, `class: Option<String>`, `size: InputSize`. Emits `oncommit` on Enter or blur with the parsed value when it is in `[min, max]`. Out-of-range or unparseable input fires `oninvalid` with an `IntegerInputError` (`Empty`, `NotANumber`, or `OutOfRange { min, max }`); `oncommit` does not fire and no value is dispatched. Escape reverts the in-flight buffer to the last-committed value and suppresses the next blur's commit, so reverting does not produce a redundant `oncommit`. Inset variant via `class: "if-integer-input--inset"`. Operates on `usize` natively, so no f64 conversion is needed in the consumer. F15 is the first consumer; promotion to a generic `NumberInput<T>` sibling is a follow-up if a second consumer materializes.
2. **`DestructiveConfirmDialog`** (`crates/inputforge-gui-dx/src/patterns/destructive_confirm.rs`). Shared 2-button confirm for destructive actions; F4's destructive-shape primitive in concrete form, parallel to the existing `DirtyConfirmDialog` at `patterns/dirty_confirm.rs`. Props: `open: Signal<bool>`, `title: Option<String>`, `description: Element` (rich body for emphasis), `cancel_label: Option<String>` (default "Cancel"), `confirm_label: String` (no default; caller specifies action verb), `oncancel: EventHandler<()>`, `onconfirm: EventHandler<()>`, `class: Option<String>`. Cancel receives default focus via `DialogRoot`'s `onmounted` hook. Uses `DialogRoot { dismissible: true, close_on_backdrop_click: false }`. F15's prune-confirm consumes it via the thin `prune_confirm.rs` wrapper. Future destructive flows (profile delete, snapshot delete, mapping bulk-delete) MAY adopt it; that adoption is each feature's own scope, not F15's.

---

## Impeccable commands

Recommended invocations during the F15 implementation plan:

- `impeccable:shape`: confirm panel structure and field rhythm before implementation.
- `impeccable:frontend-design`: primary visual treatment for the panel (section heading, field rows; no panel-level header per Choice 16).
- `impeccable:layout`: vertical rhythm between sections and field rows.
- `impeccable:typeset`: section heading vs label vs helper hierarchy (no panel title under Choice 16).
- `impeccable:clarify`: field labels, helper text, validation error copy, prune-confirm copy.
- `impeccable:harden`: error states (engine offline, file write failure, prune failure), keyboard reachability.
- `impeccable:polish`: final pass.
