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

**4. Drop dependent tests and references.** Every test, fixture, and call site that constructs or asserts behavior of `ModeChangeStrategy::Previous`, `ModeChangeStrategy::Cycle`, `CycleModes`, `ModeState::go_previous`, or `ModeState::cycle` is removed. The implementation plan enumerates the exact removal list against the current tree at plan-time; acceptance #1's grep gate is authoritative. Note: `process_outputs_mode_change_no_op` at `crates/inputforge-core/src/engine/tests.rs` asserts a `SwitchTo`-to-current-mode no-op and stays.

**5. No migration path is shipped.** No version-detection logic, no rewrite step, no deprecation warning. Any TOML with `strategy = "previous"` or `strategy = "cycle"` will fail to parse. This is acceptable because the app is pre-distribution; verifying that no committed test fixtures or sample profiles in the repo carry these strategies is part of the implementation acceptance.

**5a. Add `is_button_shaped` helper to `inputforge-core`.** F14 adds, in `crates/inputforge-core/src/types/address.rs`:

```rust
impl InputId {
    pub fn is_button_shaped(&self) -> bool {
        matches!(self, InputId::Button { .. })
    }
}

impl InputAddress {
    pub fn is_button_shaped(&self) -> bool {
        matches!(self, InputAddress::Bound { input, .. } if input.is_button_shaped())
    }
}
```

`Hat` returns `false` because hats encode 1D continuous angle in this codebase (`crates/inputforge-core/src/output/vjoy_output.rs:326-336` converts hat directions to a single angle value), not discrete press/release. `Axis` returns `false` for the same press/release reason. `InputAddress::Unbound` returns `false`. The `ReleaseCallback::PopTemporaryMode` auto-release lifecycle only fires on real `Button` releases, so gating Hold to `is_button_shaped() == true` is exactly correct.

### GUI, the body composition

**6. New module `change_mode.rs` replaces the placeholder.** F14 introduces `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/change_mode.rs` exporting a `ChangeModeBody` component. The dispatcher arm at `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs:115` is rewritten to mount the new body. The `placeholders` module (currently a single component, `ChangeModePlaceholder`) is deleted. Component props follow the shape established by all three sibling stage bodies (`MergeAxisBody` at `merge_axis.rs:105`, `DeadzoneBody` at `deadzone/mod.rs:175`, `MapToVJoyBody` at `map_to_vjoy.rs:103`): `(mapping_key: MappingKey, stage_id: StageId, <payload>, root_actions: Vec<Action>)`. F14's `<payload>` is `strategy: ModeChangeStrategy`, destructured from the dispatcher arm.

**7. Body layout, top to bottom.** Two fields stacked inside the standard `if-stage__body` container:

```
Strategy        [ Set | Hold ]
Target mode     [ Combat ▾ ]
```

Both fields use the existing `.if-stage__body-field` shell with a `.if-stage__body-label` caption, so visual rhythm matches `MergeAxisBody` and the rest of F9's bodies. The two-line layout is fixed regardless of strategy: switching Set ↔ Hold preserves the picked target mode.

**8. Strategy picker, 2-pill segmented row.** A locally-scoped pill row (CSS class `if-stage__body-strategy`) with two pills, `Set` and `Hold`. Active pill carries the control-violet tint pattern (`color-mix` of the canonical hue at 14% on `bg-elevated`, foreground at `--color-control-badge-text` for AA, inheriting the verified contrast math from DESIGN.md §2). Inactive pills are muted-text on a 1px hairline. The pill row sits on `bg-elevated` to keep the active-pill contrast claim load-bearing. No new global primitive is added to DESIGN.md; the pill row is scoped to this body. If a third stage variant later needs the same shape, promotion to a shared primitive is its own ticket.

Why segmented pills over Select or radios: a 2-state choice would waste a click in a Select (the user always needs the other state); two radios cost more vertical space in a body that is already two stacked fields; segmented pills surface both states simultaneously in a single horizontal control. F9's `MergeAxisBody` operation picker uses Select because its choice is 3-way (Bidirectional / Average / Maximum) and the per-option side effects are uniform.

**9. Hold gating, non-button primaries.** When the mapping's primary input (the `InputAddress` in `MappingKey.1`) is not a button input (`InputAddress::is_button_shaped()` returns false; see Engine clean cut 5a), the Hold pill is rendered as a focusable `<button>` with `aria-pressed` reflecting the persisted strategy and `aria-disabled="true"` (NOT HTML `disabled`, which removes the element from the tab order and from most screen-reader trees). A click handler intercepts and short-circuits the strategy switch when `aria-disabled` is set. Visual treatment follows DESIGN.md disabled rules: `opacity: 0.5; cursor: not-allowed`; no surface tint change.

The pill is wrapped in the F2 `Tooltip` primitive (`crates/inputforge-gui-dx/src/components/tooltip.rs:14-33`) with `content: "Hold requires a button input."` and `placement: TooltipPlacement::Top`. The Tooltip surfaces on hover AND on keyboard focus (via the existing `if-tooltip` focus-within style), so the explanation is keyboard-discoverable. This replaces native `title`, which is invisible to screen readers on disabled controls and not keyboard-triggerable.

The "selected-but-disabled" case (Hold persisted, primary later rebound to a non-button) carries `aria-pressed="true"` and `aria-disabled="true"` simultaneously. The Set pill stays enabled in this state; clicking Set dispatches a one-step migration to `SwitchTo { mode }` preserving the target. CSS treatment for selected-but-disabled follows DESIGN.md §7 Tabs disabled-active rule (control-violet tint at the desaturated value).

Why prevent at edit time rather than commit-and-warn: the auto-release lifecycle (`ReleaseCallback::PopTemporaryMode`) fires only on `Button` releases. Committing a Hold against a non-button would silently never auto-revert. Both reviewers agreed that a footgun this load-bearing should not be reachable through the strategy picker; the post-rebind path (priority 3 hint) handles the edge case where state arrives at a non-button Hold by other means.

**10. Target mode picker, Select over current modes.** The picker uses the F2 `Select` primitive (extended per Choice 11 below). Options are sourced from `MetaSnapshot.modes: Vec<String>` at `crates/inputforge-gui-dx/src/context.rs:311`. Typography follows the body-text convention (Inter, 14px, regular), matching the rest of F9's stage-body fields and the existing Select pattern in `MergeAxisBody`. Mode names are labels rather than engine-owned numerics, so DESIGN.md's "Mono For Live Numerics Rule" does not apply.

Width: `max-width: 240px`. The closed-state value truncates with ellipsis when it overflows; option labels in the dropdown also truncate with ellipsis. The Select width never expands beyond `max-width` regardless of the longest mode name, so a single very long mode name does not bulge the stage body wider than its siblings. Empty-modes case (a profile with no user-defined modes): `default_change_mode` already produces `mode: ""`, so malformed-hint priority 1 ("Choose a mode to switch to") fires unconditionally and the user has the recovery copy. No special "(no modes available)" placeholder is rendered; the hint already tells the user what to do.

**11. Stale-mode handling, malformed-hint pattern.** When the action references a mode that is not in the active profile's modes (snapshot-restore drift; mid-edit mode delete), the body:
- Preserves the orphaned name as a disabled `<option>` rendered with the error tint and italic style, so the user sees what was there.
- Surfaces a malformed-hint via `editor.malformed_hints` (existing `EditorState` Signal): `"Mode \"<name>\" no longer exists. Pick a current mode."`.
- Keeps the action data unchanged until the user explicitly picks a current mode. No silent rewrite.
- Once the user picks a current mode, the orphaned name is no longer persisted in the action and the disabled option does not appear on subsequent renders.

This mirrors the convention `MergeAxisBody` and `MapToVJoyBody` use for similar drift conditions.

**F2 coordination, Select primitive extension.** The current `Select` at `crates/inputforge-gui-dx/src/components/select.rs:9-19` accepts `options: Vec<(String, String)>` only; there is no per-option `disabled` and no per-option `class`. Rendering an orphaned mode name as a disabled error-tinted `<option>` requires extending the primitive. F14 coordinates this with F2 in the same way F11 coordinated `NumberInput.oncommit` and `Field.error`:

- New shape: `options: Vec<SelectOption>` where `SelectOption { value: String, label: String, disabled: bool, class: Option<String> }` (additive struct in `select.rs`; existing call sites migrate to `SelectOption { value, label, disabled: false, class: None }`).
- HTML emission updates so each `<option>` carries its own `disabled` and `class` attributes.
- Migration: every existing `Select { options: Vec<(String, String)>, .. }` call site in the gui crate is reshaped in the same change. The implementation plan enumerates call sites against the current tree.

The F14 implementation plan must include this extension as its own task block, sequenced before the `ChangeModeBody` work that depends on it.

**12. Edit dispatch, shared `dispatch_stage_edit`.** Both the strategy pill change and the target-mode change call `instruments::stage_dispatch::dispatch_stage_edit` (signal-wrapping form, signature at `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/stage_dispatch.rs:73`), which sends `EngineCommand::SetMapping` and pushes a single `UndoKind::StageEdit` entry on success. Canonical call-site pattern is `MergeAxisBody` at `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/merge_axis.rs:230-237` (and the symmetric block at `:339-346`). Name preservation reads from `cfg.mapping_names` (`crates/inputforge-gui-dx/src/context.rs:117`, `HashMap<InputAddress, String>`) so user-set names are never silently cleared (Amendment 2 convention from `MergeAxisBody`).

Undo labels (note: `format_undo_label` for `UndoKind::StageEdit` produces the literal format `"{name}: {field} {b} -> {a}"` per `crates/inputforge-gui-dx/src/frame/mapping_editor/undo_log.rs:218-223`; ASCII arrow, no middle-dot):
- Strategy switch: `format_undo_label(UndoKind::StageEdit, LabelArgs { stage_name: Some("Change mode"), field: Some("strategy"), before_after: Some(("Set", "Hold")), .. })` produces `"Change mode: strategy Set -> Hold"`.
- Target change: `format_undo_label(UndoKind::StageEdit, LabelArgs { stage_name: Some("Change mode"), field: Some("target"), before_after: Some((<old_or_unset>, <new>)), .. })`.

Empty-before substitution: when the persisted target is the empty string (initial state from `default_change_mode`), the body passes `"<unset>"` (literal seven-character string) as the before label to `format_undo_label`, producing `"Change mode: target <unset> -> Combat"`. The body owns this substitution; `format_undo_label` is unchanged.

**13. Header, no thumbnail override.** F14 does not override `header_right_slot` at `stage_body/mod.rs:131`. The match arm for `Action::ChangeMode` reverts from "F14 will override" placeholder to a final `_ => default_chevron(expanded)` (collapses into the catch-all). The collapsed summary line, already produced by `format_mode_strategy` at `pipeline/stage.rs:345` ("Set Combat" / "Hold Combat"), carries the visible information.

Explicit divergence from F10/F11: those features ship glanceable previews (mini-curve, mini-zone-bar) because the geometry IS the user-facing value (a curve shape, a zone radius). F14's strategy is a 2-state choice the label suffices to convey; a thumbnail here would add chrome without adding information density, which PRODUCT.md's "Restraint over spectacle" rules against.

**14. Stage title and category, unchanged.** `stage_title_for(action)` returns `"Change mode"` for both surviving variants (no change). The category-class branch at `pipeline/stage.rs:124` continues to map `Action::ChangeMode { .. }` to `is-control` (control-violet tint). The collapsed summary uses sentence-start capitalization (`Set Combat` / `Hold Combat`) because the strategy heads the line; this is a deliberate change from F9's earlier internal `set Combat` / `hold Combat` lowercase form, since the strategy is now the leading word rather than a mid-sentence verb.

### Malformed hints, priority order

The body computes hints at render time and writes a single highest-priority hint to `editor.malformed_hints` per stage, alongside a compact summary tag in `editor.malformed_summary_tags`. Priority order (highest first):

1. **Empty target mode.** The default action created by `default_change_mode` at `add_palette.rs:112` is `SwitchTo { mode: "" }`. Until the user picks a mode, the hint is `"Choose a mode to switch to"` (tag: `Set target mode`). Mirrors `HINT_MERGE_UNBOUND` in `MergeAxisBody`. First because it is the always-on hint for newly-added stages and the most universal authoring guidance.
2. **Target mode not in active profile's modes.** When the persisted target is non-empty but not present in `MetaSnapshot.modes` (snapshot-restore drift, mid-edit mode delete, manual TOML edit). Hint copy: `"Mode \"<name>\" no longer exists. Pick a current mode."` (tag: `Mode missing`). Per choice 11, the orphaned name is preserved as a disabled error-tinted option in the Select; the option label is prefixed with `(removed) ` so the closed-trigger display carries the orphan signal even on Chromium where native `<option>` styling is muted on the trigger.
3. **Hold strategy with non-button primary.** When the persisted action is `Temporary` but `MappingKey.1.is_button_shaped()` is false (typically because the user changed the mapping's primary input from a button to a non-button after authoring the Hold). The Hold pill renders as visually-selected-but-disabled (`aria-pressed="true"` + `aria-disabled="true"`, plus `tabindex="-1"` so the disabled pill is skipped in sequential focus order) so the user sees current state. Hint copy: `"Hold requires a button input. Pick a button on your device, or switch to Set."` (tag: `Needs button`). Clicking the Set pill commits a one-step migration to `SwitchTo { mode }` preserving the target.

**Combined priority 2 + 3.** When both an orphan target and a non-button primary are simultaneously persisted, the body emits a single combined banner so the user can recover both errors in one edit pass: `"Mode \"<name>\" no longer exists. Hold also requires a button input. Pick a button on your device, then a current mode."` (tag: `Mode missing`). Strictly, priorities 2 and 3 are mutually exclusive in the priority order, but emitting two banners would reflow the header twice as the user fixes one then bounces to the other.

When any hint is present, the collapsed-header summary slot renders the compact tag in place of `stage_summary_for`, and the body banner carries the full prose. The split keeps the right-aligned 12px mono summary slot glanceable (the slot was sized for `Hold Combat`-shaped fragments, not multi-sentence prose). Stage bodies that have not yet adopted the tag fall back to the prose hint, with CSS ellipsis at `.if-stage__summary` clipping any overflow as defense-in-depth. The title also receives the error-tint class.

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

**16. `default_change_mode` stays.** The factory at `add_palette.rs:112` continues to produce `Action::ChangeMode { strategy: SwitchTo { mode: "" } }` as the initial state when the user adds a Change mode stage. The empty mode triggers hint priority 1 immediately, guiding the user to pick a target before commit.

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
- **A richer "input shape" taxonomy.** F14 adds `is_button_shaped()` on `InputId` and `InputAddress` (Engine clean cut 5a). Anything beyond that single predicate (per-axis-direction shapes, pressure-curve classification, a polymorphic `InputShape` enum) is out of scope.

---

## Acceptance

### Engine

1. **Grep gate (authoritative removal verification).** After F14, `rg "ModeChangeStrategy::Previous"`, `rg "ModeChangeStrategy::Cycle"`, `rg "CycleModes"`, `rg "go_previous"`, and `rg "fn cycle\b"` (within `crates/inputforge-core/src/mode/state.rs`) all return zero matches across `crates/`. The implementation plan enumerates the specific tests, fixtures, and call sites to remove against the current tree at plan time.
2. `cargo build -p inputforge-core` and `cargo build -p inputforge-gui-dx` both succeed after the cut.
3. `cargo test --workspace` succeeds. Surviving tests for `SwitchTo` and `Temporary` round-trip behavior continue to pass; `process_outputs_mode_change_no_op` (a `SwitchTo`-to-current-mode no-op test) is preserved.
4. `cargo deny`, `cargo clippy --all-targets --all-features -- -D warnings`, and the project's existing CI checks succeed. Dead-code lints from the removal are resolved (every remaining symbol still has a callsite).
5. Profile fixtures and sample TOML in the repo do not reference the removed strategies. Verified by `rg 'strategy = "previous"'` and `rg 'strategy = "cycle"'`.

### GUI

6. Adding a "Change mode" stage from the add palette commits an action with `SwitchTo { mode: "" }` (unchanged). The stage immediately surfaces hint priority 1 ("Choose a mode to switch to"); the collapsed header shows the hint copy in place of the normal summary.
7. Picking a target mode commits `EngineCommand::SetMapping` once and pushes one undo entry. Empty-before renders as `<unset>` (e.g., `"Change mode: target <unset> -> Combat"`); non-empty before/after pairs render as `"Change mode: target <old> -> <new>"`.
8. Toggling the Set ↔ Hold pill commits `EngineCommand::SetMapping` once and pushes one undo entry. The label produced by `format_undo_label` is `"Change mode: strategy Set -> Hold"` (or the reverse). The target-mode value is preserved across the toggle.
9. Hold pill is gated when `MappingKey.1` is not a button input (`InputAddress::is_button_shaped() == false`): the pill renders as a focusable `<button>` with `aria-disabled="true"`, carries the F2 `Tooltip` with content `"Hold requires a button input."`, and does not commit on click. The Tooltip surfaces on both hover and keyboard focus.
10. Stale-mode reference (the persisted target is not in `MetaSnapshot.modes`) renders the orphaned name as a disabled error-tinted option in the Select; hint priority 2 surfaces in the stage header. Once the user picks a current mode, the orphaned name is no longer persisted and does not reappear on subsequent renders.
11. Hold-on-non-button reference (e.g., user changes the mapping primary input from a button to an axis after authoring Hold) renders the Hold pill with `aria-pressed="true"` AND `aria-disabled="true"` (selected-but-disabled) and surfaces hint priority 3 ("Hold requires a button input. Pick a button on your device, or switch to Set."). Clicking the (still-enabled) Set pill commits a one-step migration to `SwitchTo { mode }` preserving the target.
12. Collapsed-stage header shows `Set <mode>` or `Hold <mode>` from `format_mode_strategy`, except when a malformed hint preempts the summary.
13. The `header_right_slot` for `Action::ChangeMode` renders the default chevron; no thumbnail override.
14. F7's `ModeTabContextMenu` "Set as default" item dispatches `EngineCommand::SetDefaultMode`. Verified by citing an existing F7 test by file:fn name (default expectation; the implementation plan locates it). If no such test exists, F14 adds one SSR test asserting the menu item renders and clicking it sends `EngineCommand::SetDefaultMode`.

### Cross-cutting

15. Keyboard reachability: Tab visits the strategy pills (each a `<button>`, including any `aria-disabled="true"` pill so the Tooltip is keyboard-discoverable) and the target-mode Select, then leaves the body. Enter activates the focused strategy pill; on an `aria-disabled` pill, Enter is a no-op. Visible focus rings on dark background per DESIGN.md focus-ring conventions.
16. `prefers-reduced-motion`: no spatial motion in the body; pill activation is an instant color/border swap. Already conformant with the global rule (DESIGN.md §5).

---

## Open follow-ups beyond F14

- **Mode-tree parenting in the chrome.** F7's mode-tab `+ add mode` always sends `parent: None`. Exposing a "child of X" choice in the add-mode flow (and potentially a re-parent affordance in the right-click menu) would unlock real mode inheritance for GUI-only users. If shipped, Cycle becomes meaningfully useful again.
- **Mapping list inheritance visibility.** F8's left rail today shows only the active editing mode's directly-authored mappings. Mappings active in the mode via ancestor inheritance are invisible. A "show inherited" affordance (greyed rows, separate group, badge on the row) is a candidate for a future F8 iteration.
- **Reintroducing Cycle.** Predicated on both follow-ups above. The engine variant and `CycleModes` newtype would need to come back, the strategy picker would gain a third pill, and the body would gain a list editor (the brainstorm's pill-chain mockup at `.superpowers/brainstorm/1577-1778229310/content/cycle-editor.html` is the starting point).
- **Cross-mode Hold validation hardening.** `push_temporary` already errors `ModeCycleDetected` when the mode is current or already on the stack. Surfacing this at edit time would require knowing the runtime stack, which the GUI does not have. Accepted as runtime-only for now.
- **Mode-rename rewrite of `ChangeMode` actions.** Already covered by the existing engine cascade and tested by `rename_mode_refs_rewrites_change_mode_actions` at `crates/inputforge-core/src/profile/mod.rs:1523`. F14 verifies the test still passes after the variant cut; no new code.
- **User-visible failure mode for legacy TOML.** A profile TOML with `strategy = "previous"` or `strategy = "cycle"` (e.g., a contributor's local stash predating the cut) will fail to parse after F14. The exact surface (toast, profile-load error dialog, silent fall-back to a fresh profile) is left to the implementation plan; pre-distribution status makes this low-stakes but worth committing one consistent path.
- **Live-readout / analyzer audit.** `live_readout/predicate.rs` and `live_readout/analyzer.rs` may contain code paths that switch on `ModeChangeStrategy::Previous` or `::Cycle` (e.g., axis-driven Cycle preview). The implementation plan greps these paths and removes any dead arms in the same change.

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
