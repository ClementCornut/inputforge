# F14, Mode Editing (ChangeMode action editor): Design Spec

**Status:** Design approved, ready for implementation plan
**Date:** 2026-05-08
**Parent specs:**
- [`2026-04-24-egui-to-dioxus-rewrite-design.md`](./2026-04-24-egui-to-dioxus-rewrite-design.md), master plan, feature F14
- [`2026-04-27-f5-architecture-ia-redesign-design.md`](./2026-04-27-f5-architecture-ia-redesign-design.md), IA pass that scoped F14

**Predecessors:** F1 (state bridge), F2 (design system), F3 (shell), F4 (toast/dialog), F5 (IA), F6 (snapshot + preferences core), F7 (chrome shell, owns mode-tab right-click menu and `SetDefaultMode` wiring), F9 (pipeline stage editor, hosts the ChangeMode body), F11 (deadzone editor, established the stage-body integration pattern).
**Design system:** [`/DESIGN.md`](../../DESIGN.md)
**Product brief:** [`/PRODUCT.md`](../../PRODUCT.md)

---

## Context

F14 is the last feature in the rewrite that owns "anything to do with modes". F5 already drained most of mode CRUD into F7 (mode tabs and right-click menu). What remained for F14 was the in-pipeline `ChangeMode { strategy }` action editor and any mode-tree visualization that implementation might find warranted.

The brainstorm validated the engine semantics for each strategy variant against actual source code, not against the spec, and surfaced two structural problems:

1. **`Previous` is a misleading name and a redundant feature.** Engine inspection at `crates/inputforge-core/src/mode/state.rs:89` shows `ModeState::go_previous` is a literal alias for `pop_temporary`. It only unwinds Holds; it does not "go to the previously-active mode" in any general sense. SwitchTo and Cycle both clear the temporary stack, so any Previous fired after either of those is a silent no-op. As a user-facing mental model it is broken.
2. **`Cycle` requires mode-tree inheritance to be useful, and the GUI does not expose tree authoring.** `resolve_mapping` at `crates/inputforge-core/src/mode/resolve.rs:16` walks `tree.ancestors(mode)` and returns the first matching mapping. So a Cycle bound in `Default` propagates to all descendant modes via inheritance and the wrap-around works as intended. But `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/add_inline.rs:49` shows the GUI's `+ add mode` always sends `EngineCommand::AddMode { name, parent: None }`, producing a flat tree where no user mode has another user mode as an ancestor. Inheritance never engages. With a flat tree, Cycle is functionally equivalent to N `SwitchTo` actions authored once per cycle member; nothing only Cycle can do.

The brainstorm therefore committed a **clean cut** of both variants from the engine and the UI. The app is not yet distributed, no user TOML files contain these strategies, and no migration code is required.

What is left for F14: the `ChangeMode` action editor for the two surviving strategies, `SwitchTo` (Set) and `Temporary` (Hold).

---

## Confirmed design choices

### Engine, clean cut

**1. Drop `ModeChangeStrategy::Previous` and `ModeChangeStrategy::Cycle`.** Both variants and all their type-level support (the `CycleModes` newtype with its validation invariants) are removed from `crates/inputforge-core/src/action/mode_change.rs`. The serde tag/contents shape stays identical for the surviving variants (`#[serde(tag = "strategy", rename_all = "snake_case")]`), so `SwitchTo` and `Temporary` continue to round-trip byte-identically.

**2. Drop the dependent runtime helpers.** `ModeState::go_previous` (alias for `pop_temporary`) and `ModeState::cycle` are removed from `crates/inputforge-core/src/mode/state.rs`. The two corresponding arms in `apply_mode_change` at `crates/inputforge-core/src/engine/output_handler.rs:94` are removed. Logging and error-handling shape for the surviving arms is unchanged.

**3. Keep what Hold needs.** `ModeState::pop_temporary`, the `ReleaseCallback::PopTemporaryMode` callback, and `EngineError::ModeCycleDetected` (still raised by `push_temporary`) all stay. The auto-release lifecycle for Hold is intact.

**4. Drop dependent tests.** Remove `mode_change_strategy_previous_serde_roundtrip`, `mode_change_strategy_cycle_serde_roundtrip`, the seven `cycle_modes_*` and three `with_renamed_*` tests on `CycleModes`, `process_outputs_cycle_mode`, `process_outputs_mode_change_no_op` (asserts a Previous no-op), and any other test that constructs or asserts behavior of the dropped variants. The implementation plan enumerates the precise test list during F14 work.

**5. No migration path is shipped.** No version-detection logic, no rewrite step, no deprecation warning. Any TOML with `strategy = "previous"` or `strategy = "cycle"` will fail to parse. This is acceptable because the app is pre-distribution; verifying that no committed test fixtures or sample profiles in the repo carry these strategies is part of the implementation acceptance.

### GUI, the body composition

**6. New module `change_mode.rs` replaces the placeholder.** F14 introduces `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/change_mode.rs` exporting a `ChangeModeBody` component. The dispatcher arm at `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs:115` is rewritten to mount the new body. The `placeholders` module (currently a single component, `ChangeModePlaceholder`) is deleted. Component props mirror the F11 deadzone body shape: `mapping_key: MappingKey`, `stage_id: StageId`, `strategy: ModeChangeStrategy`, `root_actions: Vec<Action>`.

**7. Body layout, top to bottom.** Two fields stacked inside the standard `if-stage__body` container:

```
Strategy        [ Set | Hold ]
Target mode     [ Combat ▾ ]
```

Both fields use the existing `.if-stage__body-field` shell with a `.if-stage__body-label` caption, so visual rhythm matches `MergeAxisBody` and the rest of F9's bodies. The two-line layout is fixed regardless of strategy: switching Set ↔ Hold preserves the picked target mode.

**8. Strategy picker, 2-pill segmented row.** A locally-scoped pill row (CSS class `if-stage__body-strategy`) with two pills, `Set` and `Hold`. Active pill carries the control-violet tint pattern (`color-mix` of the canonical hue at 14% on background, foreground at `--color-control-badge-text` for AA), inactive pills are muted-text on a 1px hairline, the rest of the segment uses `bg-sunken` to distinguish from the surrounding stage body. No new global primitive is added to DESIGN.md; the pill row is scoped to this body. If a third stage variant later needs the same shape, promotion to a shared primitive is its own ticket.

**9. Hold gating, non-button primaries.** When the mapping's primary input (the `InputAddress` in `MappingKey.1`) is not a button-shaped input, the Hold pill is rendered as disabled (DESIGN.md disabled treatment: `opacity 0.5`, `cursor: not-allowed`, no surface tint change). A native `title` tooltip carries the copy `"Hold requires a button-shaped primary input."`. Implementation may use `InputAddress::is_button()` if it exists in core; if not, F14 adds the helper to `inputforge-core` as a small additive change. The auto-release lifecycle (`ReleaseCallback::PopTemporaryMode`) does not fire for non-button inputs, so committing a Hold against an axis or hat would silently never auto-revert; gating the pill at edit time prevents that footgun.

**10. Target mode picker, Select over current modes.** The picker uses the existing F2 `Select` primitive. Options are sourced from `MetaSnapshot.modes: Vec<String>` at `crates/inputforge-gui-dx/src/context.rs:311`. Typography follows the body-text convention (Inter, 14px, regular), matching the rest of F9's stage-body fields and the existing Select pattern in `MergeAxisBody`. Mode names are labels rather than engine-owned numerics, so DESIGN.md's "Mono For Live Numerics Rule" does not apply. Width is sized to the widest current mode name plus padding; longer names truncate with ellipsis rather than reflowing the column.

**11. Stale-mode handling, malformed-hint pattern.** When the action references a mode that is not in the active profile's modes (snapshot-restore drift; mid-edit mode delete), the body:
- Preserves the orphaned name as a disabled `<option>` rendered with the error tint and italic style, so the user sees what was there.
- Surfaces a malformed-hint via `editor.malformed_hints` (existing `EditorState` Signal): `"Mode \"<name>\" no longer exists. Pick a current mode."`.
- Keeps the action data unchanged until the user explicitly picks a current mode. No silent rewrite.

This mirrors the convention `MergeAxisBody` and `MapToVJoyBody` use for similar drift conditions.

**12. Edit dispatch, shared `dispatch_stage_edit`.** Both the strategy pill change and the target-mode change call `instruments::stage_dispatch::dispatch_stage_edit` (signal-wrapping form), which sends `EngineCommand::SetMapping` and pushes a single `UndoKind::StageEdit` entry on success. Name preservation reads from `cfg.mapping_names` so user-set names are never silently cleared (Amendment 2 convention from `MergeAxisBody`).

Undo labels:
- Strategy switch: `format_undo_label(UndoKind::StageEdit, LabelArgs { stage_name: Some("Change mode"), field: Some("strategy"), before_after: Some(("Set", "Hold")), .. })`.
- Target change: `format_undo_label(UndoKind::StageEdit, LabelArgs { stage_name: Some("Change mode"), field: Some("target"), before_after: Some((<old>, <new>)), .. })`.

**13. Header, no thumbnail override.** F14 does not override `header_right_slot` at `stage_body/mod.rs:131`. The match arm for `Action::ChangeMode` reverts from "F14 will override" placeholder to a final `_ => default_chevron(expanded)` (collapses into the catch-all). The collapsed summary line, already produced by `format_mode_strategy` at `stage.rs:345` ("Set Combat" / "Hold Combat"), carries the visible information. There is no glanceable visual content beyond text, and PRODUCT.md's "Restraint over spectacle" principle argues against a decorative thumbnail.

**14. Stage title and category, unchanged.** `stage_title_for(action)` returns `"Change mode"` for both surviving variants (no change). The category-class branch at `stage.rs:124` continues to map `Action::ChangeMode { .. }` to `is-control` (control-violet tint).

### Malformed hints, priority order

The body computes hints at render time and writes a single highest-priority hint to `editor.malformed_hints` per stage. Priority order (highest first):

1. **Empty target mode.** The default action created by `default_change_mode` at `add_palette.rs:112` is `SwitchTo { mode: "" }`. Until the user picks a mode, the hint is `"Choose a mode to switch to"`. Mirrors `HINT_MERGE_UNBOUND` in `MergeAxisBody`. First because it is the always-on hint for newly-added stages and the most universal authoring guidance.
2. **Target mode not in active profile's modes.** When the persisted target is non-empty but not present in `MetaSnapshot.modes` (snapshot-restore drift, mid-edit mode delete, manual TOML edit). Hint copy: `"Mode \"<name>\" no longer exists. Pick a current mode."`. Per choice 11, the orphaned name is preserved as a disabled error-tinted option in the Select.
3. **Hold strategy with non-button primary.** When the persisted action is `Temporary` but `MappingKey.1.is_button()` is false (typically because the user changed the mapping's primary input from a button to a non-button after authoring the Hold). The Hold pill renders as visually-selected-but-disabled so the user sees current state. Hint copy: `"Hold requires a button-shaped input. Switch to Set or pick a button input."` Clicking the Set pill commits a one-step migration to `SwitchTo { mode }` preserving the target.

When any hint is present, `stage_summary_for` is preempted in the collapsed header (existing pattern at `stage.rs:159`): the hint text replaces the normal `"Set X"` / `"Hold X"` summary, and the title gets the error-tint class.

### Format, summary, and undo helpers

**15. `format_mode_strategy` reduces to two arms.** After the engine cut, the function at `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage.rs:345` becomes:

```rust
fn format_mode_strategy(strategy: &ModeChangeStrategy) -> String {
    match strategy {
        ModeChangeStrategy::SwitchTo { mode } => format!("Set {mode}"),
        ModeChangeStrategy::Temporary { mode } => format!("Hold {mode}"),
    }
}
```

The Pop and Cycle arms are removed.

**16. `default_change_mode` stays.** The factory at `add_palette.rs:112` continues to produce `Action::ChangeMode { strategy: SwitchTo { mode: "" } }` as the initial state when the user adds a Change mode stage. The empty mode triggers hint priority 2 immediately, guiding the user to pick a target before commit.

### Default-mode selector

**17. F7 owns the wiring.** The default-mode selector lives in `ModeTabContextMenu` at `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/context_menu.rs:49` and dispatches `EngineCommand::SetDefaultMode { name }` on the "Set as default" item click. F14 adds nothing here. Acceptance for F14 includes a verification pass that the wiring is intact and behaves as documented; no new code.

### Mode-tree visualization

**18. Out of scope.** F5's default plan was "do not". Cycle's removal makes the question moot for F14: there is no in-pipeline action that benefits from a tree picker. If mode-tree authoring is later exposed in the chrome (an open follow-up below), Cycle's reintroduction would be its own ticket and that ticket may want a tree-picker primitive at that point.

---

## Non-goals

- **Mode CRUD on the chrome.** Activate / Rename / Delete / Set as default are F7's responsibility and are already shipped in `top_bar/mode_tabs/`. F14 does not touch them.
- **Mode-tree parenting authoring.** Adding a "make this a child of X" affordance to the chrome's mode tabs is unscoped here. F7 sends `parent: None` for every new mode; that does not change in F14.
- **Inheritance visibility in the mapping list.** A mapping authored in an ancestor mode is currently invisible while editing a descendant. F14 cannot fix this; F8's left-rail mapping list is the right surface to revisit. Documented as a follow-up.
- **Cycle reintroduction.** If the GUI later exposes mode parenting and mapping inheritance, Cycle becomes a useful action again. Reviving it is its own brainstorm cycle and its own engine reintroduction; not F14's call.
- **Dialog-based "many-mode" management surface.** Old F11's "Mode Editor" provisional shape was a dialog browsing the mode tree. F5 absorbed everything actionable about it into F7 chrome and F14 stage editor. There is no mode-management dialog and no Profiles-panel "Modes" sub-section.
- **`InputAddress::is_button()` engine-side.** If the helper does not already exist, F14 adds it as a tiny additive change in `inputforge-core`. Anything more elaborate (a richer "input shape" taxonomy) is out of scope.

---

## Acceptance

### Engine

1. `cargo build -p inputforge-core` and `cargo build -p inputforge-gui-dx` both succeed after the cut. No surviving references to `ModeChangeStrategy::Previous` or `::Cycle`, no surviving references to `CycleModes`, no dead `pop_temporary` callsite via `go_previous`.
2. `cargo test --workspace` succeeds. The test list documented in choice 4 is removed; surviving tests for `SwitchTo` and `Temporary` round-trip behavior continue to pass.
3. `cargo deny`, `cargo clippy --all-targets --all-features -- -D warnings`, and the project's existing CI checks succeed. Dead-code lints from the removal are resolved (every remaining symbol still has a callsite).
4. Profile fixtures and sample TOML in the repo do not reference the removed strategies. Verified by a grep.

### GUI

5. Adding a "Change mode" stage from the add palette commits an action with `SwitchTo { mode: "" }` (unchanged). The stage immediately surfaces hint priority 2 ("Choose a mode to switch to"); the collapsed header shows the hint copy in place of the normal summary.
6. Picking a target mode commits `EngineCommand::SetMapping` once and pushes one undo entry. Label format follows `format_undo_label` for `UndoKind::StageEdit` with `field: Some("target")` and the before/after pair (empty-before is rendered per the helper's existing convention).
7. Toggling the Set ↔ Hold pill commits `EngineCommand::SetMapping` once and pushes one undo entry labeled `Change mode · strategy: Set → Hold`. The target-mode value is preserved across the toggle.
8. Hold pill is disabled when `MappingKey.1` is not button-shaped. The pill carries the documented tooltip and does not respond to clicks.
9. Stale-mode reference (the persisted target is not in `MetaSnapshot.modes`) renders the orphaned name as a disabled error-tinted option in the Select; hint priority 3 surfaces in the stage header.
10. Hold-on-non-button reference (e.g., user changes the mapping primary input from a button to an axis after authoring Hold) renders the Hold pill as visually-selected-but-disabled and surfaces hint priority 1; clicking Set commits a one-step migration to `SwitchTo { mode }` preserving the target.
11. Collapsed-stage header shows `Set <mode>` or `Hold <mode>` from `format_mode_strategy`, except when a malformed hint preempts the summary.
12. The `header_right_slot` for `Action::ChangeMode` renders the default chevron; no thumbnail override.
13. F7's `ModeTabContextMenu` "Set as default" item still dispatches `EngineCommand::SetDefaultMode`. Verified by an existing-test grep, no new test required.

### Cross-cutting

14. Keyboard reachability: Tab cycles through the strategy pills (each is a `<button>`), the target-mode `<select>`, and out. Enter activates the focused strategy pill. Visible focus rings on dark background per DESIGN.md focus-ring conventions.
15. `prefers-reduced-motion`: no spatial motion in the body; pill activation is an instant color/border swap. Already conformant with the global rule (DESIGN.md §5).

---

## Open follow-ups beyond F14

- **Mode-tree parenting in the chrome.** F7's mode-tab `+ add mode` always sends `parent: None`. Exposing a "child of X" choice in the add-mode flow (and potentially a re-parent affordance in the right-click menu) would unlock real mode inheritance for GUI-only users. If shipped, Cycle becomes meaningfully useful again.
- **Mapping list inheritance visibility.** F8's left rail today shows only the active editing mode's directly-authored mappings. Mappings active in the mode via ancestor inheritance are invisible. A "show inherited" affordance (greyed rows, separate group, badge on the row) is a candidate for a future F8 iteration.
- **Reintroducing Cycle.** Predicated on both follow-ups above. The engine variant and `CycleModes` newtype would need to come back, the strategy picker would gain a third pill, and the body would gain a list editor (the brainstorm's pill-chain mockup at `.superpowers/brainstorm/1577-1778229310/content/cycle-editor.html` is the starting point).
- **Cross-mode Hold validation hardening.** `push_temporary` already errors `ModeCycleDetected` when the mode is current or already on the stack. Surfacing this at edit time would require knowing the runtime stack, which the GUI does not have. Accepted as runtime-only for now.
- **Mode-rename rewrite of `ChangeMode` actions.** Already covered by the existing engine cascade and tested by `rename_mode_refs_rewrites_change_mode_actions` at `crates/inputforge-core/src/profile/mod.rs:1523`. F14 verifies the test still passes after the variant cut; no new code.

---

## Impeccable commands

Recommended invocations during the focused implementation plan:

- `impeccable:shape`, structural pass on the body composition (pill row + select + hint slot interaction).
- `impeccable:frontend-design`, control-violet pill row aesthetic; coherence with F2's existing Select treatment.
- `impeccable:clarify`, copy for the three malformed hints, the Hold tooltip, and the undo labels.
- `impeccable:harden`, the three malformed-hint conditions and the Hold-on-non-button migration path; the engine-cut audit (no surviving references, no test gaps).
- `impeccable:polish`, final pass.

`impeccable:audit`, `impeccable:layout`, `impeccable:typeset`, and `impeccable:onboard` are not invoked: the body is two fields and a hint, layout is dictated by the existing `.if-stage__body-field` shell, typography follows the established Inter body-text convention used by every other F9 stage body, and there is no first-run flow.

---

## Next steps

1. Commit this spec to git.
2. Invoke `superpowers:writing-plans` to produce the focused implementation plan for F14, sequenced as: engine cut first (smaller, testable in isolation), then GUI body, then the verification-and-acceptance pass.
3. After F14 lands, the master plan's F14 entry can be marked resolved.
