# Plan: `InputAddress::Unbound`

## Context

Freshly-added Conditional / MergeAxis stages displayed the lying label `Btn 1` because the data model used an empty-`DeviceId` sentinel to mean "no binding selected yet". This plan converts `InputAddress` from a struct to a `Bound | Unbound` enum so "no binding" is representable at the type level, and bundles the deletion of the deprecated `inputforge-gui` (egui) crate so the new variant only has to land in the surviving Dioxus frontend.

This is the second iteration of the plan: an earlier draft was rejected by code review for three Critical issues (would have broken TOML round-trip for the new variant and left the workspace in non-compiling intermediate states). The decisions below preserve compile-state invariants at every commit boundary.

**User decisions baked in:**

| Decision | Choice |
|----------|--------|
| Wire format for `Unbound` in TOML | Explicit `unbound = true` key, plus migration walker for legacy `{device:""}` blocks |
| Field-access migration pattern | Accessor methods (`.device()` / `.input_id()`) + `.expect("invariant: ...")` |
| Drop `AppState::quit_requested` | Yes |
| Keep validator-hint task (was original Task 10) | Yes |
| Fold the palette-seed-`Unbound` change into the big sweep so no commit reproduces the bug | Yes |
| Strip residual `gui-egui` references in CLAUDE.md and docs | Yes |

**Tech stack:** Rust 2024 edition, serde 1.x with `serde_with` patterns where useful, toml 0.8, Dioxus 0.7 (`inputforge-gui-dx` is the sole frontend after Task 1). Tests via `cargo test --workspace`.

---

## Task ordering and rationale

Order is chosen so each commit leaves the workspace green and never reproduces the visible bug:

1. **Task 1: remove `inputforge-gui` (egui) crate** — clears all egui code so subsequent sweeps don't have to touch it.
2. **Task 2: drop `AppState::quit_requested`** — orphan after Task 1; do this immediately so the field doesn't go uninitialised in any future commit.
3. **Task 3: type change + custom serde** — convert `InputAddress` to `Bound | Unbound`, add accessor helpers, custom `(De)Serialize` to emit `unbound = true` for `Unbound` and the existing flat shape for `Bound`. Only `crates/inputforge-core/src/types/address.rs` changes; the workspace will not compile after this commit (intentional). Tests for the type itself pass in isolation.
4. **Task 4: workspace-wide compile-fix sweep** — single coordinated commit that gets the workspace back to green AND eliminates the `Btn 1` bug in one go: struct literals to `::Bound`, field-access migration via accessors with `.expect()`, palette / kind-switch / source-label updates to `::Unbound`. After this commit the visible bug is fixed.
5. **Task 5: `evaluate_condition` short-circuits `Unbound` to `false`** — semantic change.
6. **Task 6: `MergeAxis` evaluator treats `Unbound` secondary as identity** — semantic change.
7. **Task 7: UI polish (`--unbound` CSS modifier)** — makes the placeholder visually distinct.
8. **Task 8: profile-load migration walker** — auto-heal legacy on-disk `{device:""}` sentinels.
9. **Task 9: validator hints for `Unbound` predicate / merge inputs** — was Task 10 of the original.

---

## Critical files

| File | Change |
|------|--------|
| `crates/inputforge-gui/` | **Deleted entirely** (Task 1) |
| `Cargo.toml` (workspace root) | Drop `inputforge-gui` member; drop `eframe` / `egui*` workspace deps (Task 1) |
| `crates/inputforge-app/Cargo.toml` | Drop `gui-egui` feature; default = `gui-dioxus` (Task 1) |
| `crates/inputforge-app/src/main.rs` | Drop both `compile_error!` lines, all `#[cfg(feature = "gui-egui")]` items (`EngineStatus` use, `TrayAction` use, `launch_gui` use, the egui block in `main()` lines 146-168, `launch_gui_blocking` lines 249-282, `drain_stale_gui_events` lines 290-299, `run_tray_loop` lines 309-383). Update doc comment lines 5-7 to drop "optional GUI window" framing (Task 1) |
| `crates/inputforge-app/src/tray.rs` | Drop the `MenuEvent` use (line 12-13), the `EngineStatus` use (line 18-19), the `TrayAction` enum (lines 21-35), `poll_event` (lines 88-108), `refresh_toggle_label` (lines 121-134) (Task 1) |
| `crates/inputforge-core/src/state/mod.rs` | Drop `quit_requested: bool` field at line 73 and both `quit_requested: false` initialisers at lines 94, 133 (Task 2) |
| `crates/inputforge-gui-dx/src/lifecycle/mod.rs` | Update doc comment at line 35 (`quit_requested` no longer exists, so the comment is stale) (Task 2) |
| `CLAUDE.md` | Drop `--no-default-features --features gui-dioxus` from launch command (Task 1). Verify with ripgrep no other `gui-egui` reference remains (Task 1 step 8) |
| `crates/inputforge-core/src/types/address.rs` | Struct → enum with `Bound { device, input } | Unbound`, accessor helpers, custom `Serialize` / `Deserialize` (Task 3) |
| Workspace-wide (~25 sites) | Field accesses `addr.device` / `addr.input` → accessor calls (Task 4). Full enumeration in Task 4 |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/add_palette.rs` | `default_conditional` (line 100), `default_merge_axis` (line 90) → `InputAddress::Unbound` (Task 4) |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/predicate.rs` | `default_condition_for_kind` (line 89) seeds `Unbound` when `prev_input` is `None`; `condition_input` (line 124) returns `None` for `Unbound` (Task 4) |
| `crates/inputforge-gui-dx/src/frame/mapping_list/source_label.rs` | `format` and `split_label` handle `Unbound` (Task 4 — must be in the same commit because they currently field-access `addr.device` / `addr.input` and would break the build otherwise) |
| `crates/inputforge-core/src/pipeline/condition.rs` | `evaluate_condition` short-circuits `Unbound` to `false` (Task 5) |
| `crates/inputforge-core/src/pipeline/mod.rs` | `Action::MergeAxis` arm passes primary through when `second_input.is_unbound()` (Task 6) |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/predicate.rs` (`PredicateInputRow` ~line 234) | Compute `composite_class` with `--unbound` modifier (Task 7) |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/merge_axis.rs` (`MergeAxisBody` ~line 85) | Same `composite_class` treatment (Task 7) |
| `crates/inputforge-gui-dx/assets/frame/mapping_editor.css` | New `.if-rebind-composite--unbound` rule (Task 7) |
| `crates/inputforge-core/src/profile/mod.rs` | New `with_legacy_addresses_unbound` walker, called from `from_raw` (Task 8) |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/predicate.rs` + `merge_axis.rs` | Add `Unbound`-priority malformed hints (Task 9) |

---

## Task 1: Remove the deprecated `inputforge-gui` (egui) crate

**Files:** delete `crates/inputforge-gui/`; modify `Cargo.toml` (root), `crates/inputforge-app/Cargo.toml`, `crates/inputforge-app/src/main.rs`, `crates/inputforge-app/src/tray.rs`, `CLAUDE.md`.

- [ ] **Step 1.** Delete the egui crate directory:
  ```powershell
  Remove-Item -Recurse -Force "crates\inputforge-gui"
  ```
- [ ] **Step 2.** In root `Cargo.toml`, replace the `[workspace] members` line with:
  ```toml
  members = ["crates/inputforge-core", "crates/inputforge-gui-dx", "crates/inputforge-app"]
  ```
  In `[workspace.dependencies]`, delete: `eframe`, `egui`, `egui_extras`, `egui_plot`, `egui_kittest`, `inputforge-gui`.
- [ ] **Step 3.** In `crates/inputforge-app/Cargo.toml`, replace `[features]` block with:
  ```toml
  [features]
  default    = ["gui-dioxus"]
  gui-dioxus = ["dep:inputforge-gui-dx"]
  ```
  In `[dependencies]`, delete the `inputforge-gui` line. Keep `inputforge-gui-dx = { workspace = true, optional = true }`.
- [ ] **Step 4.** In `crates/inputforge-app/src/main.rs`, delete:
  - Lines 9-13: both `compile_error!` blocks.
  - Lines 34-35, 39-40, 42-43: `EngineStatus`, `TrayAction`, `inputforge_gui::launch_gui` use lines.
  - Lines 146-168: the entire `#[cfg(feature = "gui-egui")] { ... }` block in `main()`.
  - Lines 249-282: `launch_gui_blocking` (including doc comment).
  - Lines 290-299: `drain_stale_gui_events`.
  - Lines 309-383: `run_tray_loop` (including the `#[expect(unsafe_code, ...)]` attribute and the inner Win32 imports).
  - Section header comments at lines 240-242 and 301-303 if their sections are now empty.
  - Drop the `if let Err(e) = launch_gui(...)` `cfg(feature = "gui-dioxus")` wrapping; the dioxus path becomes unconditional.
  - Update the file-level doc comment at lines 5-7 to drop the "optional GUI window" / `eframe` framing.
- [ ] **Step 5.** In `crates/inputforge-app/src/tray.rs`, delete:
  - Lines 12-13 (`MenuEvent` use).
  - Lines 18-19 (`EngineStatus` use).
  - Lines 21-35 (`TrayAction` enum and its doc comment).
  - Lines 88-108 (`poll_event`, including doc comment).
  - Lines 121-134 (`refresh_toggle_label`, including doc comment).
- [ ] **Step 6.** In `CLAUDE.md`, change the launch command to `dx run -p inputforge-app` and remove the parenthetical sentence explaining `--no-default-features`.
- [ ] **Step 7.** Build and run:
  ```powershell
  cargo build --workspace
  cargo test --workspace
  ```
- [ ] **Step 8.** Final ripgrep sweep for stale `gui-egui` references:
  ```powershell
  rg "gui-egui" --type rust
  rg "gui-egui" CLAUDE.md
  rg "inputforge_gui[^_]" --type rust
  ```
  All hits should be in test fixtures or historical doc files only. Patch any production hit on sight.
- [ ] **Step 9.** Confirm dioxus boots without flags:
  ```powershell
  dx run -p inputforge-app
  ```
- [ ] **Step 10.** Commit:
  ```
  chore(workspace): remove deprecated inputforge-gui (egui) crate
  ```

---

## Task 2: Drop the orphaned `AppState::quit_requested` field

**Files:** `crates/inputforge-core/src/state/mod.rs`, `crates/inputforge-gui-dx/src/lifecycle/mod.rs`.

After Task 1, the field has no remaining writer. The Dioxus path uses `WindowCloseBehaviour::WindowHides` and never touches it.

- [ ] **Step 1.** In `crates/inputforge-core/src/state/mod.rs`:
  - Delete `pub quit_requested: bool` at line 73.
  - Delete `quit_requested: false,` at lines 94 and 133 (in both `AppState::new()` and `AppState::with_profile`).
- [ ] **Step 2.** In `crates/inputforge-gui-dx/src/lifecycle/mod.rs:35`, delete or rewrite the doc comment that explains "`quit_requested` is not read on the Dioxus path" — it now refers to a field that does not exist.
- [ ] **Step 3.** Verify no remaining references:
  ```powershell
  rg "quit_requested" --type rust
  ```
  Hits in `docs/` are historical and may be ignored; hits under `crates/` must be zero.
- [ ] **Step 4.** Build and test:
  ```powershell
  cargo build --workspace
  cargo test --workspace
  ```
- [ ] **Step 5.** Commit:
  ```
  refactor(state): drop orphan AppState::quit_requested after egui removal
  ```

---

## Task 3: Convert `InputAddress` to enum with custom serde

**Files:** `crates/inputforge-core/src/types/address.rs`.

After this commit only `address.rs` changes; the rest of the workspace stops compiling. Task 4 fixes that. The type's own tests pass in isolation.

The wire format chosen by the user:

| Variant | TOML / JSON shape |
|---------|------------------|
| `Bound { device, input }` | `{ device = "guid-001", input = { type = "button", index = 0 } }` (existing flat shape, untouched) |
| `Unbound` | `{ unbound = true }` |

This requires custom `Serialize` and `Deserialize` impls because the two shapes are mutually exclusive at the same nesting level. `#[serde(untagged)]` cannot distinguish them reliably in TOML (untagged enums work poorly when both variants are tables).

- [ ] **Step 1.** Replace the struct definition + tests at `crates/inputforge-core/src/types/address.rs:7-12, 87-97` with the enum and helpers.

  Pseudocode of the new type and serde impls (the implementer should write the full Rust):

  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Hash)]
  pub enum InputAddress {
      Bound { device: DeviceId, input: InputId },
      Unbound,
  }

  impl InputAddress {
      pub const fn is_unbound(&self) -> bool { matches!(self, Self::Unbound) }
      pub const fn is_bound(&self)   -> bool { matches!(self, Self::Bound { .. }) }
      pub const fn device(&self)     -> Option<&DeviceId> { ... }
      pub const fn input_id(&self)   -> Option<&InputId>  { ... }
  }

  // Helper structs used only inside the serde impls.
  #[derive(Serialize)]
  struct BoundOnTheWire<'a> { device: &'a DeviceId, input: &'a InputId }
  #[derive(Serialize)]
  struct UnboundOnTheWire { unbound: bool }  // always emits `unbound = true`

  impl Serialize for InputAddress {
      fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
          match self {
              Self::Bound { device, input } => BoundOnTheWire { device, input }.serialize(s),
              Self::Unbound                 => UnboundOnTheWire { unbound: true }.serialize(s),
          }
      }
  }

  impl<'de> Deserialize<'de> for InputAddress {
      fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
          // Buffer to a serde_json::Value-equivalent (use `toml::Value` since the
          // primary on-disk format is TOML; JSON tests still work because `serde_json`
          // round-trips through the same `Deserialize` impl). Inspect the buffered
          // map: if `unbound` key is present and true, return Unbound. Otherwise
          // require `device` and `input` keys and return Bound.
          //
          // Implementer note: a cleaner pattern is `#[derive(Deserialize)]` for an
          // intermediate enum with `#[serde(untagged)]` over { Unbound { unbound: bool }, Bound { device, input } }
          // and then convert. The intermediate-enum approach works because the two
          // shapes share zero fields, so untagged disambiguation is unambiguous.
      }
  }
  ```

  **Recommended implementation:** use the intermediate-enum-with-untagged pattern. It's the simplest serde code that satisfies the requirement:

  ```rust
  #[derive(Deserialize)]
  #[serde(untagged)]
  enum InputAddressOnTheWire {
      Unbound { unbound: bool },               // matches { unbound = true }
      Bound { device: DeviceId, input: InputId }, // matches the existing shape
  }
  ```

  The serializer uses two separate helper structs (cannot share an enum because `unbound: false` would silently match the unbound shape on round-trip).

- [ ] **Step 2.** Add tests, all in `mod tests` of `address.rs`:

  ```rust
  #[test]
  fn input_address_bound_toml_roundtrip() {
      let addr = InputAddress::Bound {
          device: DeviceId("guid-001".to_owned()),
          input: InputId::Axis { index: 2 },
      };
      let toml_str = toml::to_string(&addr).unwrap();
      assert!(toml_str.contains("device = \"guid-001\""));
      assert!(!toml_str.contains("unbound"));
      let back: InputAddress = toml::from_str(&toml_str).unwrap();
      assert_eq!(addr, back);
  }

  #[test]
  fn input_address_unbound_toml_roundtrip() {
      let addr = InputAddress::Unbound;
      let toml_str = toml::to_string(&addr).unwrap();
      assert_eq!(toml_str.trim(), "unbound = true");
      let back: InputAddress = toml::from_str(&toml_str).unwrap();
      assert_eq!(back, InputAddress::Unbound);
  }

  #[test]
  fn input_address_unbound_json_roundtrip() {
      // JSON path used by some debug logs / snapshot tests.
      let addr = InputAddress::Unbound;
      let j = serde_json::to_string(&addr).unwrap();
      assert_eq!(j, r#"{"unbound":true}"#);
      let back: InputAddress = serde_json::from_str(&j).unwrap();
      assert_eq!(back, InputAddress::Unbound);
  }

  #[test]
  fn input_address_legacy_bound_format_still_parses() {
      // A profile saved before this refactor.
      let legacy = r#"{"device":"guid-001","input":{"type":"button","index":3}}"#;
      let addr: InputAddress = serde_json::from_str(legacy).unwrap();
      assert!(matches!(addr, InputAddress::Bound { .. }));
  }

  #[test]
  fn input_address_legacy_empty_device_still_parses_as_bound() {
      // The pre-migration profile shape. Task 8's walker turns this into Unbound;
      // here we lock in that the deserializer alone produces Bound (with empty device).
      let legacy = r#"{"device":"","input":{"type":"button","index":0}}"#;
      let addr: InputAddress = serde_json::from_str(legacy).unwrap();
      let InputAddress::Bound { device, .. } = addr else { panic!("expected Bound") };
      assert!(device.0.is_empty());
  }

  #[test]
  fn input_address_helpers() {
      let bound = InputAddress::Bound {
          device: DeviceId("d".to_owned()),
          input: InputId::Button { index: 0 },
      };
      assert!(bound.is_bound() && !bound.is_unbound());
      assert!(bound.device().is_some() && bound.input_id().is_some());

      let unbound = InputAddress::Unbound;
      assert!(unbound.is_unbound() && !unbound.is_bound());
      assert!(unbound.device().is_none() && unbound.input_id().is_none());
  }
  ```

- [ ] **Step 3.** Run tests for the type only (the rest of the workspace will not compile yet):
  ```powershell
  cargo test -p inputforge-core --lib types::address::tests
  ```
- [ ] **Step 4.** Commit:
  ```
  feat(types): introduce InputAddress::Bound | Unbound with custom serde
  ```

---

## Task 4: Workspace-wide compile-fix sweep (single commit)

**Files:** every Rust file in `crates/inputforge-core/`, `crates/inputforge-gui-dx/`, `crates/inputforge-app/` that constructs an `InputAddress` literal OR field-accesses one. Plus the palette / kind-switch / source-label files.

This is the largest commit in the plan. It accomplishes three things at once so that no commit on the trunk reproduces the original `Btn 1` bug:

1. Convert every struct-literal `InputAddress { device, input }` → `InputAddress::Bound { device, input }`.
2. Convert every field access `addr.device` / `addr.input` (where `addr` is `InputAddress` or `&InputAddress`) → `addr.device().expect("invariant: ...")` / `addr.input_id().expect("invariant: ...")`.
3. Change the three sentinel-construction sites to `InputAddress::Unbound`, and update `source_label::format`, `source_label::split_label`, and `condition_input` to handle `Unbound`. The latter must be in this commit because they currently field-access `addr.device` / `addr.input` and would otherwise break the build.

After this commit the workspace builds, all existing tests pass, and the dioxus app no longer shows `Btn 1` for newly-added Conditional / MergeAxis stages — it shows the literal text `Unbound` (or whatever the new `format` returns).

### Step 1: Sweep struct literals to `::Bound`

```powershell
Get-ChildItem -Path crates -Recurse -Include *.rs | ForEach-Object {
    $content = Get-Content $_.FullName -Raw
    $new = $content -replace 'InputAddress\s*\{', 'InputAddress::Bound {'
    if ($new -ne $content) { Set-Content -Path $_.FullName -Value $new -NoNewline }
}
```

The regex matches whitespace between `InputAddress` and `{` to handle fully-qualified usages (`inputforge_core::types::InputAddress { ... }`).

### Step 2: Migrate field accesses to accessor calls

The sites enumerated below break with E0609 ("no field `device` on type `&InputAddress`") after Task 3. For each site, choose the smallest equivalent rewrite using the `device()` / `input_id()` helpers added in Task 3. Examples below.

**`crates/inputforge-core/src/engine/output_handler.rs` (lines 159, 167, 216, 217, 230, 231):**
- `cache.set_axis(addr.device, *id, *value)` → use a `let-else` that destructures `Bound` once at function entry, OR convert to `cache.set_axis(addr.device().expect("invariant: output cache addr always Bound").clone(), *id, *value)`. The `let-else` is cleaner because the same `addr` is used across multiple statements; pick whichever fits the surrounding control flow. Add a single `let InputAddress::Bound { device, .. } = addr else { unreachable!("invariant: output handler addr always bound"); };` early in the relevant `match` arm so subsequent uses can reference `device` directly.

**`crates/inputforge-core/src/profile/mod.rs:28` (`group_of_input`):**
- `match addr.input` → `match addr.input_id().expect("invariant: group_of_input is only called on Bound mappings")`. Lift the dereference: the helper returns `Option<&InputId>`, so the match arms become `Some(InputId::Axis { .. })` etc., or pre-unwrap and match on `*input_id`.

**`crates/inputforge-core/src/profile/mod.rs` test sites (lines 1606, 1624, 1639, 1645, 1656, 1662, 1671, 1686):**
- `m.input.input` is accessing the `Mapping.input.input` (the `InputId` of the mapping's `InputAddress`). After enum conversion this is `m.input.input_id().expect("invariant: mapping input is always Bound")`. Or, if there is a `Mapping::input_kind()` helper to add, prefer that.
- The corresponding `.map(|m| m.input.clone())` lines need no change — they clone the whole `InputAddress`, not a sub-field.

**`crates/inputforge-core/src/state/cache.rs:68`:**
- `self.values.retain(|addr, _| addr.device != *device)` → `self.values.retain(|addr, _| addr.device().is_none_or(|d| d != device))`. Unbound addresses are kept (they have no device to compare); Bound addresses with non-matching devices are kept.

  *(`is_none_or` lands in stable Rust 1.82+. If the workspace MSRV is older, write it as `.map_or(true, |d| d != device)`.)*

**`crates/inputforge-core/src/pipeline/mod.rs:209`:**
- `let input_value = match &primary.input` → `let input_value = match primary.input_id().expect("invariant: pipeline primary is always Bound")`. The pipeline's primary input comes from the mapping's primary, which is always Bound (only secondary in MergeAxis and predicate inputs in Conditional can be Unbound — not the mapping primary itself).

**`crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout.rs:276, 282`:**
- `let InputId::Axis { index } = addr.input else { ... }` → match on `addr.input_id()`:
  ```rust
  let Some(InputId::Axis { index }) = addr.input_id() else { return; };
  let dev_idx = cfg.devices.iter()
      .position(|d| Some(&d.info.id) == addr.device());
  ```

**`crates/inputforge-gui-dx/src/frame/mapping_list/source_label.rs` (lines 41-45 — combined with rendering update below):**
- See dedicated Step 4 below.

**`crates/inputforge-gui-dx/src/frame/mapping_list/row.rs:63`:**
- `let kind_class = match summary.input.input` → `match summary.input.input_id().expect("invariant: mapping list row addr always Bound")`. Mapping-list rows are always built from Bound mapping primaries, never from Unbound predicate inputs.

**`crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage.rs:275, 276, 347, 351, 355, 363`:**
- These render stage labels for `second_input` / `input` in pipeline stages. Some receivers are Bound (mapping primary) and some can be Unbound (predicate input, MergeAxis secondary). For sites where the address can be Unbound, switch to `addr.device()` returning `Option<&DeviceId>` and render a placeholder when `None`. Specifically:
  - Lines 275-276 (`second_input`, MergeAxis label): `addr.device()` returning `None` should render `"Unbound"` similarly to source_label.
  - Lines 347, 351, 355, 363 (predicate input labels): same — render `"Unbound"` when `device()` is `None`.

**`crates/inputforge-gui-dx/src/frame/mapping_list/group.rs:32`:**
- `match addr.input` → `match addr.input_id().expect("invariant: mapping list group addr always Bound")`. Mapping list groups bucket mapping primaries by axis/button/hat — primaries are always Bound.

**`crates/inputforge-gui-dx/src/frame/mapping_list/add_inline.rs:427`:**
- `let kind = match addr.input` → `match addr.input_id().expect("invariant: add_inline addr is always Bound after capture")`. Add-inline only constructs an address from F8 capture, which is always Bound.

### Step 3: Update palette / kind-switch sites to seed `Unbound`

In `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/add_palette.rs`:

- Lines 90-98 (`default_merge_axis`): replace the body with `Action::MergeAxis { second_input: InputAddress::Unbound, operation: MergeOp::Average }`.
- Lines 100-110 (`default_conditional`): replace the body with `Action::Conditional { condition: Condition::ButtonPressed { input: InputAddress::Unbound }, if_true: vec![], if_false: Vec::new() }`.
- Drop the now-unused `DeviceId` and `InputId` imports if `cargo build` warns about them (they may still be used by other defaults in the same file — check before deleting).

In `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/predicate.rs`:

- Lines 89-115 (`default_condition_for_kind`): the function takes `prev_input: Option<InputAddress>`. Replace the inner sentinel construction with:
  ```rust
  let addr = prev_input.unwrap_or(InputAddress::Unbound);
  ```
  and pass `addr` (cloning where needed) into each leaf. The `Not` branch wraps `Condition::ButtonPressed { input: addr }`. The wildcard fallback also uses `addr`.

- Lines 124+ (`condition_input`): return `None` for `Unbound`:
  ```rust
  fn condition_input(c: &Condition) -> Option<InputAddress> {
      match c {
          Condition::ButtonPressed { input }
          | Condition::ButtonReleased { input }
          | Condition::AxisInRange { input, .. }
          | Condition::HatDirection { input, .. } => match input {
              InputAddress::Bound { .. } => Some(input.clone()),
              InputAddress::Unbound      => None,
          },
          Condition::All { .. } | Condition::Any { .. } | Condition::Not { .. } => None,
      }
  }
  ```

### Step 4: Update `source_label::format` and `split_label`

In `crates/inputforge-gui-dx/src/frame/mapping_list/source_label.rs`:

```rust
pub(crate) fn format(addr: &InputAddress, cfg: &ConfigSnapshot) -> String {
    match addr {
        InputAddress::Unbound => "Unbound".to_owned(),
        InputAddress::Bound { .. } => {
            let (device_label, input_label) = split_label(addr, cfg);
            format!("{device_label} \u{00b7} {input_label}")
        }
    }
}

pub(crate) fn split_label(addr: &InputAddress, cfg: &ConfigSnapshot) -> (String, String) {
    let (device, input) = match addr {
        InputAddress::Bound { device, input } => (device, input),
        InputAddress::Unbound => return (String::new(), "Unbound".to_owned()),
    };
    let device_label = match cfg.devices.iter().find(|d| &d.info.id == device) {
        Some(dev) => dev.info.name.clone(),
        None => device.0.clone(),
    };
    let input_label = match input {
        InputId::Axis { index } => axis_label(*index).into_owned(),
        InputId::Button { index } => format!("Btn {}", index + 1),
        InputId::Hat { index } => format!("Hat {index}"),
    };
    (device_label, input_label)
}
```

Drop the stale rustdoc reference at line 12 to `inputforge-gui::panels::device_view::HID_AXIS_LABELS` (the source crate is gone after Task 1).

Add tests to the `mod tests` block:

```rust
#[test]
fn format_unbound_renders_placeholder() {
    let cfg = ConfigSnapshot::default();
    assert_eq!(format(&InputAddress::Unbound, &cfg), "Unbound");
}

#[test]
fn split_label_unbound_returns_empty_device_and_placeholder_input() {
    let cfg = ConfigSnapshot::default();
    let (device, input) = split_label(&InputAddress::Unbound, &cfg);
    assert_eq!(device, "");
    assert_eq!(input, "Unbound");
}
```

### Step 5: Verify the workspace compiles and all tests pass

```powershell
cargo build --workspace
cargo test --workspace
```

If a site was missed, it will surface as E0609 ("no field on enum") or E0599 ("no method on `InputAddress`"). Patch on sight using the same accessor pattern.

### Step 6: Smoke-test the dioxus app

```powershell
dx run -p inputforge-app
```

Add a Conditional from the palette: the predicate row should display `Unbound` (no styling yet — Task 7 adds the `--unbound` modifier). Add a MergeAxis: the secondary slot should also show `Unbound`. The `Btn 1` regression should be gone.

### Step 7: Commit

```
refactor(workspace): migrate InputAddress to Bound|Unbound and seed Unbound in palette
```

---

## Task 5: `evaluate_condition` short-circuits on `Unbound`

**Files:** `crates/inputforge-core/src/pipeline/condition.rs`.

Each leaf condition must return `false` when its input is `Unbound`. `Not(Unbound) == true` follows naturally and is acceptable behaviour (the validator in Task 9 surfaces unbound leaves regardless).

- [ ] **Step 1.** Add four failing tests to `mod tests` (`button_pressed_unbound_is_false`, `button_released_unbound_is_false`, `axis_in_range_unbound_is_false`, `hat_direction_unbound_is_false`) and a `not_unbound_is_true` regression-lock test.
- [ ] **Step 2.** Replace the leaf branches in `evaluate_condition` with explicit `match input { Bound { .. } => cache.get_*(input), Unbound => false }`. The `cache.get_button(input)` calls keep `&InputAddress` as the parameter (the `InputCache` trait does not need to change).
- [ ] **Step 3.** Run `cargo test -p inputforge-core --lib pipeline::condition::`.
- [ ] **Step 4.** Commit:
  ```
  feat(pipeline): evaluate Condition leaves on Unbound as false
  ```

---

## Task 6: `MergeAxis` evaluator passes primary through on `Unbound` secondary

**Files:** `crates/inputforge-core/src/pipeline/mod.rs`.

The existing `merge_axis_*` tests in this file (lines 580-666) use the pattern `let actions = [MergeAxis {...}, MapToVJoy { output: test_output() }]; execute_pipeline(&actions, &mut ctx); if let PipelineOutput::SetAxis { value, .. } = &ctx.outputs[0] { ... }`. The new test should match this pattern exactly.

- [ ] **Step 1.** Add a failing test to `mod tests`:
  ```rust
  #[test]
  fn merge_axis_unbound_secondary_passes_primary_through() {
      let cache = MockCache::new();
      let mut ctx = axis_ctx(&cache, 0.42);
      let actions = [
          Action::MergeAxis {
              second_input: InputAddress::Unbound,
              operation: MergeOp::Average,
          },
          Action::MapToVJoy { output: test_output() },
      ];
      execute_pipeline(&actions, &mut ctx);
      if let PipelineOutput::SetAxis { value, .. } = &ctx.outputs[0] {
          assert!((*value - 0.42).abs() < TOLERANCE, "expected 0.42, got {value}");
      } else {
          panic!("expected SetAxis");
      }
  }
  ```
- [ ] **Step 2.** Locate the `Action::MergeAxis` arm in the pipeline evaluator (search for the call to `cache.get_axis(second_input)`) and add an early-out:
  ```rust
  Action::MergeAxis { second_input, operation } => {
      if second_input.is_unbound() {
          // Unbound secondary: merge is a no-op; primary passes through unchanged.
          // Do not mutate the in-flight axis value carried in `ctx`.
          continue; // or whatever idiom keeps the primary value flowing
      }
      let (secondary_value, _) = cache.get_axis(second_input);
      // ... existing merge math ...
  }
  ```
  *(The exact local-variable plumbing depends on how the evaluator carries the running axis value across pipeline steps; the implementer should pattern-match the existing `merge_axis_*` arms.)*
- [ ] **Step 3.** Run `cargo test -p inputforge-core --lib pipeline::tests::merge_axis`.
- [ ] **Step 4.** Commit:
  ```
  feat(pipeline): MergeAxis with Unbound secondary passes primary through
  ```

---

## Task 7: UI styling — `--unbound` modifier on the rebind composite

**Files:** `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/predicate.rs`, `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/merge_axis.rs`, `crates/inputforge-gui-dx/assets/frame/mapping_editor.css`.

Today the rebind composite renders as:

```html
<div class="if-rebind-composite">
  <span class="if-rebind-composite__label">{source}</span>
  <button class="if-rebind-composite__action">rebind</button>
</div>
```

For `Unbound`, add an `if-rebind-composite--unbound` modifier so the label can be styled muted/italic.

- [ ] **Step 1.** Add render tests in the appropriate `pipeline/tests.rs` test module (pattern-match existing `render_predicate_*` helpers; if none exist, create one that mounts `PredicateInputRow` with a fixture address and snapshots the rendered HTML):
  ```rust
  #[test]
  fn predicate_input_row_unbound_renders_unbound_modifier() {
      let html = render_predicate_with_input(InputAddress::Unbound);
      assert!(html.contains("if-rebind-composite--unbound"), "modifier missing: {html}");
      assert!(html.contains(">Unbound<"));
      assert!(!html.contains("Btn 1"));
  }

  #[test]
  fn merge_axis_body_unbound_secondary_renders_unbound_modifier() {
      let html = render_merge_axis_with_secondary(InputAddress::Unbound);
      assert!(html.contains("if-rebind-composite--unbound"));
      assert!(html.contains(">Unbound<"));
  }
  ```
- [ ] **Step 2.** In `predicate.rs::PredicateInputRow` (~line 234), compute the class string at the top of the render function:
  ```rust
  let composite_class = if input.is_unbound() {
      "if-rebind-composite if-rebind-composite--unbound"
  } else {
      "if-rebind-composite"
  };
  ```
  Apply `class: "{composite_class}"` to BOTH branches' outer `<div>` (idle and listening) so the class doesn't flicker if the user opens then cancels rebind on an Unbound row.
- [ ] **Step 3.** Same change in `merge_axis.rs::MergeAxisBody` (~line 85) using `second_input.is_unbound()`.
- [ ] **Step 4.** Append to `crates/inputforge-gui-dx/assets/frame/mapping_editor.css`:
  ```css
  .if-rebind-composite--unbound .if-rebind-composite__label {
      color: var(--color-text-muted);
      font-style: italic;
  }
  ```
  If `--color-text-muted` is not defined, copy whichever muted-text token the `source_label` "unknown device" path uses.
- [ ] **Step 5.** Test:
  ```powershell
  cargo test -p inputforge-gui-dx --lib frame::mapping_editor::pipeline::stage_body
  ```
- [ ] **Step 6.** Visual smoke test:
  ```powershell
  dx run -p inputforge-app
  ```
  Add a Conditional from the palette; the predicate row should show `Unbound` italic/muted with the rebind button intact. Same for MergeAxis.
- [ ] **Step 7.** Commit:
  ```
  feat(stage-body): render Unbound rebind composite with muted modifier
  ```

---

## Task 8: Profile-load migration — legacy empty-device addresses → `Unbound`

**Files:** `crates/inputforge-core/src/profile/mod.rs`.

After Task 3's custom serde, an existing on-disk profile with `{device = "", input = { type = "button", index = 0 }}` deserializes as `InputAddress::Bound { device: "", input: Button{0} }` — the broken sentinel. Coerce these to `Unbound` post-deserialise so old profiles auto-heal. New profiles serialise `Unbound` as `unbound = true` and never produce empty-device sentinels going forward.

- [ ] **Step 1.** Add a failing test using the **real** TOML load path:
  ```rust
  #[test]
  fn legacy_sentinel_address_migrates_to_unbound() {
      use crate::action::{Action, Condition};

      // A pre-migration profile encoded as TOML.
      let toml_str = r#"
  [profile]
  id = "550e8400-e29b-41d4-a716-446655440000"
  name = "test"
  startup_mode = "Default"

  [modes]
  Default = []

  [[mappings]]
  mode = "Default"
  [mappings.input]
  device = "dev-1"
  [mappings.input.input]
  type = "button"
  index = 0

  [[mappings.actions]]
  type = "conditional"
  if_true = []
  if_false = []
  [mappings.actions.condition]
  type = "button_pressed"
  [mappings.actions.condition.input]
  device = ""
  [mappings.actions.condition.input.input]
  type = "button"
  index = 0
  "#;

      let profile = Profile::from_toml(toml_str).expect("must parse");
      let action = &profile.mappings()[0].actions[0];
      let Action::Conditional { condition, .. } = action else { panic!("not conditional"); };
      let Condition::ButtonPressed { input } = condition else { panic!("not button_pressed"); };
      assert_eq!(*input, InputAddress::Unbound, "legacy empty-device must migrate to Unbound");
  }
  ```
- [ ] **Step 2.** Implement the walker as a private function called from `from_raw`:
  ```rust
  fn migrate_legacy_addresses(mappings: &mut [Mapping]) {
      for mapping in mappings {
          walk_actions(&mut mapping.actions);
      }
  }

  fn walk_actions(actions: &mut [Action]) {
      for action in actions {
          match action {
              Action::MergeAxis { second_input, .. } => migrate_addr(second_input),
              Action::Conditional { condition, if_true, if_false } => {
                  walk_condition(condition);
                  walk_actions(if_true);
                  walk_actions(if_false);
              }
              _ => {}
          }
      }
  }

  fn walk_condition(c: &mut Condition) {
      match c {
          Condition::ButtonPressed { input }
          | Condition::ButtonReleased { input } => migrate_addr(input),
          Condition::AxisInRange { input, .. } => migrate_addr(input),
          Condition::HatDirection { input, .. } => migrate_addr(input),
          Condition::All { conditions } | Condition::Any { conditions } => {
              for c in conditions { walk_condition(c); }
          }
          Condition::Not { condition } => walk_condition(condition),
      }
  }

  fn migrate_addr(addr: &mut InputAddress) {
      if let InputAddress::Bound { device, .. } = addr {
          if device.0.is_empty() {
              *addr = InputAddress::Unbound;
          }
      }
  }
  ```
- [ ] **Step 3.** Wire `migrate_legacy_addresses(&mut raw.mappings)` into `Profile::from_raw` BEFORE the validation step (so validation sees the migrated state, not the legacy state).
- [ ] **Step 4.** Run profile tests:
  ```powershell
  cargo test -p inputforge-core --lib profile::
  ```
- [ ] **Step 5.** Commit:
  ```
  feat(profile): migrate legacy empty-device addresses to Unbound on load
  ```

---

## Task 9: Validator surfaces `Unbound` leaves as a malformed-hint

**Files:** `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/predicate.rs`, `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/merge_axis.rs`.

The leaf-condition editors already write malformed hints for empty hat-direction sets, inverted axis ranges, and empty `All`/`Any`. With `Unbound` now a real state, an unbound input gets the highest-priority hint ("Bind an input to complete this condition") because no other validation matters when there's no input yet.

Each `stage_id` in the malformed-hint map can hold only one hint string. Resolve by combining the per-kind effects into a single effect per leaf that picks the priority hint:

1. If `input.is_unbound()` → `"Bind an input to complete this condition"` (or for MergeAxis, `"Bind a secondary input to complete this merge"`).
2. Else if the existing per-kind condition holds (empty hat directions, inverted axis range, secondary equals primary) → that hint.
3. Else → remove the entry.

- [ ] **Step 1.** In `predicate.rs`, refactor each leaf branch's `use_effect` to incorporate the unbound check at top priority. Pattern:
  ```rust
  use_effect(move || {
      let mut map = malformed_hints.write();
      if input_for_hint.is_unbound() {
          map.insert(stage_id_for_hint.clone(), "Bind an input to complete this condition".to_owned());
      } else if /* existing per-kind check */ {
          map.insert(stage_id_for_hint.clone(), /* existing hint */);
      } else {
          map.remove(&stage_id_for_hint);
      }
  });
  ```
- [ ] **Step 2.** Same in `merge_axis.rs::MergeAxisBody`. The existing "Secondary input must differ from primary" effect (~line 119) becomes:
  ```rust
  use_effect(move || {
      let mut map = malformed.write();
      if secondary_for_hint.is_unbound() {
          map.insert(stage_id_for_hint.clone(), "Bind a secondary input to complete this merge".to_owned());
      } else if secondary_for_hint == primary_addr {
          map.insert(stage_id_for_hint.clone(), "Secondary input must differ from primary".to_owned());
      } else {
          map.remove(&stage_id_for_hint);
      }
  });
  ```
- [ ] **Step 3.** Build and test:
  ```powershell
  cargo build --workspace
  cargo test --workspace
  ```
- [ ] **Step 4.** Smoke test: open a fresh Conditional / MergeAxis stage; the inline hint should appear with the unbound text. Bind an input; the hint should clear.
- [ ] **Step 5.** Commit:
  ```
  feat(stage-body): malformed hint for Unbound predicate / merge inputs
  ```

---

## Verification (end-to-end)

After all tasks land:

1. **Workspace builds clean:**
   ```powershell
   cargo build --workspace
   cargo test --workspace
   cargo clippy --workspace --all-targets -- -D warnings
   ```
2. **No stale egui references:**
   ```powershell
   rg "gui-egui|inputforge_gui[^_]|eframe|egui_kittest" --type rust crates/
   rg "quit_requested" crates/
   ```
   Both should return zero hits under `crates/`.
3. **Dioxus boots without flags:**
   ```powershell
   dx run -p inputforge-app
   ```
4. **Bug-fix check (the original motivating bug):**
   - Open the Dioxus app on a fresh profile.
   - Add a Conditional stage from the palette: predicate row should show `Unbound` (italic/muted), NOT `Btn 1`. The malformed hint "Bind an input to complete this condition" should appear inline.
   - Add a MergeAxis stage: secondary slot should show `Unbound` with the same styling and the hint "Bind a secondary input to complete this merge".
   - Click rebind on either; LiveCapture should arm as before. Capture an input; the placeholder should disappear and the real input label should render.
5. **Round-trip check (the wire format):**
   - In the running app, save the profile.
   - Open the profile TOML on disk: the Unbound conditional should appear as `unbound = true`, not as a `device = ""` block.
   - Reload the profile; the Unbound state should persist.
6. **Legacy-profile auto-heal check:**
   - Manually craft a TOML profile with a `device = ""` sentinel block (or use one from before this refactor).
   - Load it via the app. The corresponding stage should now display `Unbound`. Save the profile back; the on-disk shape should now be `unbound = true` (the migration ran on load and the new serializer emits the new shape on save).
7. **MCP-assisted spot-check:**
   ```
   list_repos                              # confirm local/inputforge present
   search_symbols "InputAddress"           # confirm the enum exists
   find_references "InputAddress"          # confirm no orphan struct-literal sites remain
   check_references "AppState::quit_requested"  # confirm zero references
   ```

## Risks and notes

- **Task 4 is the largest commit.** Field-access migration covers ~25 sites across both `inputforge-core` and `inputforge-gui-dx`. The compiler is the safety net: every missed site is an `E0609` and patches the same way. If a site emerges that isn't safely Bound (i.e. the `.expect()` would panic), that's a real bug that the type system has just exposed — investigate, do not paper over with `unwrap_or`.
- **Custom serde** is intentionally chosen over `#[serde(untagged)]` because TOML untagged enums silently misroute when both variants are tables. The `BoundOnTheWire` / `UnboundOnTheWire` split prevents the symmetric ambiguity (`unbound: false` would otherwise round-trip as `Unbound`).
- **Task 8's migration walker is best-effort.** It only converts strictly-empty `DeviceId` strings. Profiles that referenced a device that was later detached keep their non-empty `DeviceId` (correct behaviour: the device may reattach).
- **Doc/spec drift in `docs/`.** Several historical plan files (`docs/superpowers/specs/2026-04-24-f1-...`, `docs/superpowers/plans/2026-04-25-f1-...`) reference `quit_requested` and `gui-egui`. These are historical artefacts of completed/abandoned work — leave them alone unless the user asks for a docs sweep separately.
