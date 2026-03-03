# InputForge Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.
> **Required skills during execution:** `ms-rust` (before any .rs file), `latest-packages` (verify crate versions before adding dependencies), `conventional-commits` (before any commit).
> **Design document:** `E:\Git\Perso\docs\plans\2026-03-02-inputforge-design.md` -- read this for all data model, architecture, and GUI design details.

**Goal:** Build InputForge, a Rust application for remapping physical joystick/pedal/throttle inputs to virtual vJoy devices, with 32 v1 features.

**Architecture:** Rust workspace with 3 crates: `inputforge-core` (engine library, no GUI deps), `inputforge-gui` (egui configuration UI), `inputforge-app` (binary entry point with system tray). Engine runs on dedicated thread, GUI is optional. Communication via `Arc<RwLock<AppState>>` + mpsc channels.

**Tech Stack:** Rust, SDL3 (input), vJoy (output), HidHide (device hiding), egui/eframe (GUI), egui_plot (curves), catppuccin-egui (theme), tray-icon (system tray), TOML+serde (profiles), Win32 SendInput (keyboard), tracing (logging), anyhow/thiserror (errors), mimalloc (allocator), clap (CLI).

**Test coverage:** Aim for maximum test coverage across the entire workspace. Use `cargo-llvm-cov` to measure and report coverage. Every task that adds logic MUST include corresponding tests. Target >95% line, branches, functions coverage for `inputforge-core`, measure after each phase. Add `cargo-llvm-cov` as a project tool and run `rtk cargo llvm-cov --workspace --html` to generate HTML reports. Include coverage checks in the post-implementation checklist.

---

## Package Versions (verify with `latest-packages` skill at execution time)

| Package         | Version | Notes                                             |
| --------------- | ------- | ------------------------------------------------- |
| sdl3            | 0.16+   | Builds SDL3 from source via sdl3-sys, needs CMake |
| vjoy            | 0.7+    | Wraps vjoy-sys, needs vJoy driver installed       |
| egui            | 0.33+   | Lockstep with eframe                              |
| eframe          | 0.33+   | Lockstep with egui                                |
| egui_plot       | 0.34+   | Independent versioning; 0.34 targets egui 0.33    |
| catppuccin-egui | 5.7+    | Use `egui33` feature flag for egui 0.33           |
| tray-icon       | 0.21+   | By Tauri team                                     |
| toml            | 1.0+    | TOML 1.1 spec (0.9 is deprecated)                 |
| serde           | 1.0     | Use `features = ["derive"]`                       |
| thiserror       | 2.0+    | For core crate errors                             |
| anyhow          | 1.0     | For app crate errors                              |
| windows         | 0.62+   | For SendInput + HidHide IOCTL                     |
| clap            | 4.5+    | Use `features = ["derive"]`                       |
| parking_lot     | 0.12+   | Faster RwLock for shared state                    |
| tracing         | 0.1     | With tracing-subscriber 0.3                       |
| mimalloc        | 0.1     | Global allocator for app crate                    |
| uuid            | 1.0+    | Use `features = ["v4"]` for profile IDs           |

---

## Prerequisites

1. **Rust toolchain**: `rustup` with stable channel (1.85+, edition 2024)
2. **vJoy driver**: https://github.com/BrunnerInnovation/vJoy/releases (v2.2.2+)
3. **HidHide**: https://github.com/nefarius/HidHide/releases
4. **CMake**: Required for SDL3 build from source
5. **Visual Studio Build Tools**: C/C++ compiler for Windows
6. **cargo-llvm-cov**: `cargo install cargo-llvm-cov` -- code coverage measurement
7. **llvm-tools-preview**: `rustup component add llvm-tools-preview` -- required by cargo-llvm-cov

---

## Task Overview

### Phase 0: Prerequisite Tooling Check
- Verify all required tools are installed and working before writing any code

### Phase 1: Foundation (Tasks 1-4)
- **Task 1**: Workspace scaffolding & dependencies
- **Task 2**: Claude hook for automatic cargo fmt
- **Task 3**: Core types
- **Task 4**: Error types

### Phase 2: Processing Pipeline (Tasks 5-8)
- **Task 5**: Deadzone & calibration
- **Task 6**: Axis/button inversion
- **Task 7**: Response curves (all 3 types + symmetry)
- **Task 8**: Pipeline executor & action types

**Phase 2 coverage checkpoint:** Run `rtk cargo llvm-cov --workspace` after completing Phase 2. Processing modules should have >95% coverage. ✅ 138 tests, 99.38% line coverage after code review fix.

### Phase 3: Profile & Mode System (Tasks 9-11)
- **Task 9**: Profile TOML serialization
- **Task 10**: Mode tree & inheritance
- **Task 11**: Mode switching, temporary modes, cycle detection, axis refresh

### Phase 4: Conditions & Advanced Logic (Tasks 12-14)
- **Task 12**: Condition types & evaluation
- **Task 13**: Axis merging
- **Task 14**: Button release callback system

**Phase 4 coverage checkpoint:** Run `rtk cargo llvm-cov --workspace` after completing Phase 4. Core logic (processing, mode, conditions, pipeline) should have >95% coverage overall. ✅ 235 tests, 98.58% region / 99.02% line coverage.

### Phase 5: Hardware I/O (Tasks 15-18) -- parallelizable
- **Task 15**: SDL3 input source + hotplug
- **Task 16**: vJoy output sink
- **Task 17**: Keyboard output (Win32 SendInput)
- **Task 18**: HidHide integration

### Phase 6: Engine (Task 19)
- **Task 19**: Engine event loop & AppState

### Phase 7: GUI (Tasks 20-25) -- parallelizable after Task 20
- **Task 20**: GUI foundation (theme, fonts, layout) -> run frontend skill and challenge theme crate usage
- **Task 21**: Device panel (tree, axis bars, buttons)
- **Task 22**: Mapping editor (action pipeline cards)
- **Task 23**: Response curve editor (egui_plot interactive)
- **Task 24**: Input monitor
- **Task 25**: Mode, calibration & deadzone editors

### Phase 8: Integration (Task 26)
- **Task 26**: System tray, CLI arguments, app entry point

**Parallelism opportunities:**
- Tasks 5, 6, 7 are independent processing modules
- Tasks 15, 16, 17, 18 are independent I/O backends
- Tasks 21-25 are independent GUI panels (after Task 20)
- Tasks 12, 13, 14 are independent logic modules

---

## Phase 0: Prerequisite Tooling Check [COMPLETED]

**Goal:** Verify that all required tools are installed and accessible before writing any code. Fail fast if anything is missing.

**Checks to run (all must pass):**

1. `rustc --version` -- must be 1.85+ (edition 2024 support)
2. `cargo --version` -- must be present
3. `cmake --version` -- required for SDL3 build from source (not blocking for Phase 1)
4. `cl` or `cl.exe` -- MSVC compiler must be on PATH (not blocking for Phase 1)
5. `cargo install cargo-llvm-cov` -- install if not present, then `cargo llvm-cov --version`
6. `rustup component add llvm-tools-preview` -- required by cargo-llvm-cov
7. Check vJoy driver: look for `C:\Program Files\vJoy\` or run `vJoyConf.exe` to verify driver is installed
8. Check HidHide: look for `C:\Program Files\Nefarius Software Solutions\HidHide\` or check Device Manager for HidHide device

**If any check fails:** Stop and inform the user. Do not proceed to Phase 1 until all prerequisites are met. CMake and MSVC cl are only needed from Phase 3 (SDL3), so their absence does not block Phase 1-2.

**Results:** Rust 1.93.0, cargo-llvm-cov 0.8.4, llvm-tools-preview, vJoy, HidHide all present. CMake and MSVC `cl` not on PATH (needed later for SDL3).

**No commit for this phase** -- it's a validation step only.

---

## Phase 1: Foundation

### Task 1: Workspace Scaffolding [COMPLETED]

**Goal:** Create the Rust workspace with 3 crates and all dependencies configured.

**Files to create:**
- `Cargo.toml` -- workspace root with members, workspace dependencies, workspace package metadata, and workspace lints
- `crates/inputforge-core/Cargo.toml` -- depends on serde, toml, thiserror, tracing, parking_lot; dev-depends on serde_json
- `crates/inputforge-core/src/lib.rs` -- empty stub with guideline compliance comment
- `crates/inputforge-gui/Cargo.toml` -- depends on inputforge-core, serde, tracing (GUI deps added later in Task 20)
- `crates/inputforge-gui/src/lib.rs` -- empty stub with guideline compliance comment
- `crates/inputforge-app/Cargo.toml` -- depends on inputforge-core, inputforge-gui, anyhow, tracing, tracing-subscriber, mimalloc, clap (with derive)
- `crates/inputforge-app/src/main.rs` -- minimal main that sets mimalloc as global allocator and prints version using `#[expect(clippy::print_stdout)]`
- `.gitignore` -- /target, *.swp, .DS_Store, *.pdb, *.log, lcov.info, .env, .vscode/, .idea/
- `rustfmt.toml` -- edition 2024, max_width 100

**Instructions:**
1. Create the directory structure: `crates/inputforge-{core,gui,app}/src/`
2. Write workspace `Cargo.toml` with:
   - `resolver = "2"`, all three members
   - `[workspace.package]`: version 0.1.0, edition 2024, license MIT, rust-version 1.85, description, repository (https://github.com/ClementCornut/inputforge), readme, keywords (joystick, vjoy, input-remapping, flight-simulation), categories (hardware-support)
   - `[workspace.dependencies]` for all shared deps
   - `[workspace.lints]` sections following full ms-rust M-STATIC-VERIFICATION guidelines: compiler lints (unsafe_code deny, 9 warn lints) + clippy groups (cargo, complexity, correctness, pedantic, perf, style, suspicious at priority -1) + 30 restriction lints. Project-specific allowances: only `cast_precision_loss` and `module_name_repetitions`. Do NOT globally allow `cast_possible_truncation`, `cast_sign_loss`, `cast_possible_wrap`, `missing_errors_doc`, or `missing_panics_doc` -- use `#[expect]` locally where needed.
3. Write each crate's `Cargo.toml` using `workspace = true` references for all package metadata fields and `[lints] workspace = true`
4. Write stub lib.rs / main.rs files with `// Rust guideline compliant {date}` comments. main.rs uses `#[expect(clippy::print_stdout, reason = "...")]` per M-LINT-OVERRIDE-EXPECT.
5. Run `rtk cargo build` and `rtk cargo clippy --workspace -- -D warnings` to verify everything compiles clean
6. `git init`, add all files, commit: `feat(workspace): scaffold inputforge workspace with 3 crates`

**Notes from execution:**
- `toml` crate version: plan said `0.9+` but latest stable is `1.0` (verified via `cargo search`)
- `string_to_string` clippy lint was removed in Rust 1.93, now covered by `implicit_clone` -- do not include it
- Workspace metadata (description, repository, readme, keywords, categories) required by `cargo_common_metadata` lint

---

### Task 2: Claude Hook for Automatic cargo fmt [COMPLETED]

**Goal:** Create a Claude Code hook that automatically runs `cargo fmt --all` after every Rust file change, ensuring consistent formatting without manual intervention.

**Files to create:**
- `.claude/settings.json` -- project-level Claude Code settings with hooks

**Hook configuration:**
- `PostToolUse` hook with matcher `"Edit|Write"`
- `fileEditExtensions: [".rs"]` to match only Rust files
- Command: `cargo fmt --all 2>/dev/null || true` (suppresses errors, 10s timeout)

**Steps:**
1. Create `.claude/` directory
2. Write `settings.json` with the hook configuration
3. Commit: `chore(workspace): add claude hook for automatic cargo fmt`

---

### Task 3: Core Types

**Goal:** Define all foundational types used across the codebase. PUT EXTRA CARE IN THIS STEP, REVIEW EACH TYPES BEFORE IMPLEMENTING IT.

**Files to create:**
- `crates/inputforge-core/src/types.rs`

**Files to modify:**
- `crates/inputforge-core/src/lib.rs` -- add `pub mod types;`

**Types to define (see design doc Section 4 for exact definitions):**
- `DeviceId(String)` -- stable SDL3 GUID, derive Debug/Clone/PartialEq/Eq/Hash/Serialize/Deserialize
- `InputAddress { device: DeviceId, input: InputId }` -- same derives
- `InputId` -- tagged enum with Axis{index}, Button{index}, Hat{index}. Use `#[serde(tag = "type", rename_all = "snake_case")]`
- `OutputAddress { device: u8, output: OutputId }`
- `OutputId` -- tagged enum with Axis{id: VJoyAxis}, Button{id: u8}, Hat{id: u8}
- `VJoyAxis` -- enum: X, Y, Z, Rx, Ry, Rz, Slider0, Slider1
- `AxisValue(f64)` -- normalized [-1.0, 1.0] wrapper with `new()` (clamping), `raw()` (no clamp), `value()`, `clamped()` methods
- `HatDirection` -- enum: Center, N, NE, E, SE, S, SW, W, NW
- `InputEvent { source: InputAddress, value: InputValue, timestamp: Instant }`
- `InputValue` -- enum: Axis(AxisValue), Button(bool), Hat(HatDirection)
- `DeviceInfo { id, name, axes, buttons, hats }`
- `VirtualDeviceConfig { device_id, axes, button_count, hat_count }`
- `KeyCombo { key: String, modifiers: Vec<KeyModifier> }`
- `KeyModifier` -- enum: Ctrl, Shift, Alt, Win
- `MergeOp` -- enum: Bidirectional, Average, Maximum

**Tests to write:**
- `AxisValue::new()` clamps values outside [-1, 1]
- `AxisValue::raw()` does not clamp
- `InputId` serializes with correct serde tags
- Add `serde_json` as dev-dependency for serialization tests

**Steps:**
1. Write types module following the design doc data model
2. Add `pub mod types;` to lib.rs
3. Add `serde_json = "1.0"` as dev-dependency for testing
4. Run `rtk cargo test -p inputforge-core`
5. Commit: `feat(core): add core types for devices, inputs, outputs, and events`

---

### Task 4: Error Types

**Goal:** Define typed errors for the core crate.

**Files to create:**
- `crates/inputforge-core/src/error.rs`

**Files to modify:**
- `crates/inputforge-core/src/lib.rs` -- add `pub mod error;`

**Error variants to define (using thiserror):**
- `ProfileNotFound { path }`
- `ProfileParse` (from `toml::de::Error`)
- `ProfileWrite` (from `toml::ser::Error`)
- `VJoyDeviceUnavailable { device_id }`
- `VJoyDriverMissing`
- `Sdl(String)`
- `HidHide(String)`
- `DeviceNotFound { device_id }`
- `InvalidMapping { reason }`
- `ModeNotFound { name }`
- `ModeCycleDetected { path }`
- `Io` (from `std::io::Error`)

Also define `pub type Result<T> = std::result::Result<T, EngineError>;`

**Steps:**
1. Write error module with thiserror derives
2. Export from lib.rs
3. Run `rtk cargo check`
4. Commit: `feat(core): add engine error types with thiserror`

---

## Phase 2: Processing Pipeline [COMPLETED]

### Task 5: Deadzone & Calibration [COMPLETED]

**Goal:** Implement deadzone and calibration processing as pure functions.

**Files created:**
- `crates/inputforge-core/src/processing/mod.rs` -- `pub(crate) fn lerp_range()` shared helper + re-exports
- `crates/inputforge-core/src/processing/deadzone.rs` -- `DeadzoneConfig` with Default and `apply()`
- `crates/inputforge-core/src/processing/calibration.rs` -- `Calibration` (no Default, device-specific) with `apply()`

**Files modified:**
- `crates/inputforge-core/src/lib.rs` -- added `pub mod processing;`

**Tests:** 10 deadzone tests (center, boundaries, lerp midpoints, serde roundtrip), 9 calibration tests (min/max mapping, center band, disabled passthrough, midpoints, serde roundtrip), 4 lerp_range tests.

**Commit:** `feat(core): add deadzone and calibration processing`

---

### Task 6: Axis/Button Inversion [COMPLETED]

**Goal:** Simple value inversion functions.

**Files created:**
- `crates/inputforge-core/src/processing/inversion.rs` -- `invert_axis(f64) -> f64`, `invert_button(bool) -> bool`, both `#[must_use]`

**Tests:** 6 tests (positive/negative/zero axis negation, 1→-1, button toggle both ways).

**Commit:** `feat(core): add axis and button inversion`

---

### Task 7: Response Curves [COMPLETED]

**Goal:** Implement all 3 response curve types with symmetry support.

**Files created:**
- `crates/inputforge-core/src/processing/curves.rs`

**Types defined:**
- `ResponseCurve` enum (serde tagged by `"kind"`, `rename_all = "snake_case"`): `PiecewiseLinear`, `CubicSpline`, `CubicBezier`
- `BezierSegment { start, control1, control2, end }` (each `(f64, f64)`)

**Implementation:**
- Piecewise linear: segment search + lerp, clamp outside range
- Cubic spline: Thomas algorithm (`compute_spline_coefficients`) for natural cubic spline coefficients (`SplineCoeffs` with `poly_a/b/c/d/x_start` fields -- renamed from single-char names to satisfy clippy `many_single_char_names`)
- Cubic bezier: `bezier_x/y/dx` helpers, `find_t_for_x` with Newton's method (8 iter) + bisection fallback (50 iter, uses `f64::midpoint()`)
- Symmetry: `maybe_mirror_points()` (antisymmetric: f(-x) = -f(x)), `mirror_bezier_segments()`

**Tests:** 24 tests including identity, S-curve, clamping, spline interpolation, bezier endpoints, linear control points, all 3 symmetry types, 3 serde roundtrips verifying tag values, single-point fallbacks, NaN input edge cases, bisection fallback with pathological S-shaped x curve.

**Commit:** `feat(core): add response curves (linear, spline, bezier) with symmetry`

---

### Task 8: Pipeline Executor & Action Types [COMPLETED]

**Goal:** Define action types and implement the pipeline executor that processes input through ordered action lists.

**Files created:**
- `crates/inputforge-core/src/action.rs` -- `Action`, `ModeChangeStrategy`, `Condition`, `Mapping`
- `crates/inputforge-core/src/pipeline.rs` -- `PipelineOutput`, `InputCache` trait, `PipelineContext`, `execute_pipeline()`, `evaluate_condition()`, `merge_axes()`

**Files modified:**
- `crates/inputforge-core/src/lib.rs` -- added `pub mod action; pub mod pipeline;`

**Notes from execution:**
- `Action::MapToVJoy` uses `#[serde(rename = "map_to_vjoy")]` to avoid ugly `map_to_v_joy` from automatic snake_case
- `PipelineOutput` is NOT Serialize/Deserialize (transient output, not persisted)
- `PipelineContext` has manual `Debug` impl (cannot derive due to `&dyn InputCache`)
- `Mapping` placed in `action.rs` (not `types/mapping.rs`) to avoid circular dependency with `processing/`
- Button inversion in pipeline uses `invert_button()` from processing module; axes use `invert_axis()`
- `merge_axes` Maximum returns the axis with greater absolute value, preserving sign
- `MergeOp::Average` uses `f64::midpoint()` per clippy

**Tests:** 12 action serde roundtrip tests + 25 pipeline tests (empty pipeline, axis/button passthrough, invert axis + button both directions, deadzone, calibrate, response curve, conditional true/false/no-else, merge axis 3 operations + first-wins, nested conditions All/Any/Not, ButtonReleased, AxisInRange true/false, MapToKeyboard, ChangeMode, multiple outputs, hat input no-op, Debug impl, full processing chain).

**Commits:**
- `feat(core): add action types and pipeline executor`
- `test(core): cover all production code paths in pipeline and curves`

**Phase 2 coverage results:** 118 tests, 99.11% total line coverage. All production code in `inputforge-core` is 100% covered. Only uncovered lines are `main.rs` (no integration tests) and test-only `panic!` branches.

---

### Phase 2 Code Review Fix [COMPLETED]

**Goal:** Address 5 issues found in code review — 2 high-confidence bugs (score 85) sharing the same root cause (public struct fields with `Deserialize` but no validation, allowing division-by-zero in `lerp_range`), and 3 behavioral bugs in the pipeline executor.

**Design decision:** Validation at construction time (constructors return `Result`). Once constructed, `apply()` and `evaluate()` remain infallible (`-> f64`), keeping the hot pipeline path allocation-free and error-free.

**Files modified:**
- `crates/inputforge-core/src/error.rs` -- added `InvalidConfig { reason: String }` variant to `EngineError`
- `crates/inputforge-core/src/processing/deadzone.rs` -- private fields, `new() -> Result<Self>`, getters, `DeadzoneConfigRaw` + custom `Deserialize` impl via `TryFrom`
- `crates/inputforge-core/src/processing/calibration.rs` -- same pattern: private fields, `new() -> Result<Self>`, getters, `CalibrationRaw` + `TryFrom`
- `crates/inputforge-core/src/processing/curves.rs` -- `ResponseCurveRaw` enum for deserialization, factory methods (`piecewise_linear()`, `cubic_spline()`, `cubic_bezier()`) returning `Result`, `validate_points()` helper
- `crates/inputforge-core/src/processing/mod.rs` -- added `debug_assert!(in_min < in_max)` in `lerp_range`
- `crates/inputforge-core/src/pipeline.rs` -- `MapToKeyboard` now skips Hat inputs (mirrors `MapToVJoy`), `Invert` arm uses `invert_axis()`/`invert_button()` from processing module instead of inline reimplementation
- `crates/inputforge-core/src/action.rs` -- reformatted only (struct literals updated by rustfmt)

**Invariants enforced:**

| Type | Invariant |
|------|-----------|
| DeadzoneConfig | `low < center_low <= center_high < high` |
| Calibration | `physical_min < physical_center_low <= physical_center_high < physical_max` |
| ResponseCurve points | `len >= 2`, x strictly increasing, x >= 0 when symmetric |
| BezierSegment | `segments.len() >= 1` |

**Serde pattern:** Each validated type uses `FooRaw` (Deserialize only) → `TryFrom<FooRaw> for Foo` → calls `Foo::new()` → validates → `Ok(Foo)` or `Err(EngineError)`. Custom `Deserialize` impl delegates to raw + `try_from` with `serde::de::Error::custom`. `Serialize` stays on the validated type.

**Tests added:** 20 new tests — validation rejection tests for each invariant boundary, serde rejection tests for invalid input, zero-width center band acceptance tests, getter tests, `hat_input_map_to_keyboard_no_output` test. All existing tests updated to use constructors.

**Post-fix coverage:** 138 tests, 99.38% line coverage, 100% function coverage, 98.75% region coverage.

**Commit:** `fix(core): validate configs at construction to prevent division-by-zero`

---

## Phase 3: Profile & Mode System [COMPLETE]

> **Completed 2026-03-02.** All 3 tasks implemented, refactored into module directories. 211 tests, 98.60% region / 99.11% line coverage.
> - Task 9: Profile TOML serialization with validation (profile/)
> - Task 10: Mode tree with inheritance-based mapping resolution (mode/)
> - Task 11: Mode state machine with CycleModes validation, temporary mode stack, cycle detection (mode/state.rs, action.rs)
> - Refactored mode.rs, profile.rs, pipeline.rs into module directories following existing patterns

### Task 9: Profile TOML Serialization

**Goal:** Define the Profile struct and implement save/load from TOML files.

**Files to create:**
- `crates/inputforge-core/src/profile.rs`

**Files to modify:**
- `crates/inputforge-core/src/lib.rs` -- add `pub mod profile;`
- `Cargo.toml` (workspace root) -- add `uuid = { version = "1.0", features = ["v4"] }` to `[workspace.dependencies]`
- `crates/inputforge-core/Cargo.toml` -- add `uuid = { workspace = true }` to `[dependencies]`

**Types to define:**
- `ProfileId(String)` -- UUID v4, auto-generated on creation, stable across renames. Derive Debug/Clone/PartialEq/Eq/Hash/Serialize/Deserialize
- `Profile { id: ProfileId, name, devices: Vec<DeviceEntry>, modes: ModeTree, mappings: Vec<Mapping>, settings: ProfileSettings }`
- `DeviceEntry { id: DeviceId, name: String }`
- `ProfileSettings { startup_mode: String }` (extensible later)

**Functions:**
- `Profile::load(path: &Path) -> Result<Profile>` -- read file, parse TOML
- `Profile::save(&self, path: &Path) -> Result<()>` -- serialize to TOML, write file
- `Profile::from_toml(s: &str) -> Result<Profile>` -- parse from string (for testing)
- `Profile::to_toml(&self) -> Result<String>` -- serialize to string (for testing)

**TOML format:** Follow the design doc Section 4 "TOML Profile Format" example exactly. Use `#[serde(tag = "type")]` on Action enum variants so they serialize as shown.

**Tests to write:**
- Round-trip: create a Profile, serialize to TOML string, deserialize back, compare
- Load from example TOML string matching the design doc format
- Verify all action types serialize/deserialize correctly
- Verify mode tree serializes correctly
- Test error handling for invalid TOML

**Steps:**
1. Define Profile and related structs with serde derives
2. Implement load/save functions using `std::fs` and `toml` crate
3. Write round-trip and format tests
4. Run `rtk cargo test -p inputforge-core -- profile`
5. Commit: `feat(core): add TOML profile serialization`

---

### Task 10: Mode Tree & Inheritance

**Goal:** Implement the mode tree data structure with parent-child inheritance for unmapped inputs.

**Files to create:**
- `crates/inputforge-core/src/mode.rs`

**Files to modify:**
- `crates/inputforge-core/src/lib.rs` -- add `pub mod mode;`

**Types to define:**
- `ModeTree { root: ModeNode }` -- custom Serialize/Deserialize using flat adjacency map
- `ModeNode { name: String, children: Vec<ModeNode> }` -- private fields, getters only

**TOML serialization format:** Flat adjacency map where keys are parent mode names and values are child name lists. Root is auto-detected as the key that never appears in any value list. Example:
```toml
[modes]
Default = ["Combat", "Landing"]
Combat = ["Missiles", "Guns"]
```
> **Note:** Design doc showed `Default = []` which doesn't capture the parent-child hierarchy. The adjacency map format properly encodes the full tree.

**Constructor:** `ModeTree::from_adjacency(&HashMap<String, Vec<String>>) -> Result<Self>` -- validates: non-empty, exactly one root, no duplicate mode names, all children reachable

**Methods to implement:**
- `ModeTree::root(&self) -> &ModeNode`
- `ModeTree::find_mode(&self, name: &str) -> Option<&ModeNode>` -- recursive search
- `ModeTree::ancestors(&self, name: &str) -> Vec<&str>` -- return path from mode to root (for inheritance chain)
- `ModeTree::contains(&self, name: &str) -> bool`
- `ModeTree::all_modes(&self) -> Vec<&str>` -- flat list of all mode names

**Inheritance logic (used later by engine):**
- Given a `(device, input, mode)` lookup, if no mapping found for current mode, walk up to parent mode, repeat until root
- Implement `resolve_mapping(mappings: &[Mapping], input: &InputAddress, mode: &str, tree: &ModeTree) -> Option<&Mapping>` -- walks inheritance chain

**Tests to write:**
- Build a tree: Default -> [Combat -> [Missiles, Guns], Landing]
- `find_mode("Missiles")` finds the node
- `ancestors("Missiles")` returns ["Missiles", "Combat", "Default"]
- `resolve_mapping` finds direct mapping
- `resolve_mapping` falls through to parent when no mapping in child mode
- `resolve_mapping` returns None when no mapping in entire chain
- Validation rejections (empty, multiple roots, duplicates)
- Serde roundtrip via TOML

**Steps:**
1. Define ModeTree and ModeNode with serde derives
2. Implement tree traversal methods
3. Implement `resolve_mapping` function
4. Write tests with example mode tree from design doc
5. Run `rtk cargo test -p inputforge-core -- mode`
6. Commit: `feat(core): add mode tree with inheritance-based mapping resolution`

---

### Task 11: Mode Switching, Temporary Modes, Cycle Detection, Axis Refresh

**Goal:** Implement mode state machine with all switching strategies.

**Files created:**
- `crates/inputforge-core/src/mode/state.rs` -- ModeState struct (placed inside mode/ module directory)

**Files modified:**
- `crates/inputforge-core/src/mode/mod.rs` -- add `mod state; pub use state::ModeState;`
- `crates/inputforge-core/src/action.rs` -- add `CycleModes` validated newtype, change `Cycle { modes: Vec<String> }` to `Cycle { modes: CycleModes }`

**Types defined:**
- `ModeState` struct: holds current mode name, mode stack (for temporary modes)
- `CycleModes(Vec<String>)`: validated newtype, rejects <2 modes or duplicates, custom Serialize/Deserialize

**Methods to implement:**
- `ModeState::new(initial: String) -> Self`
- `ModeState::current(&self) -> &str`
- `ModeState::switch_to(&mut self, name: &str, tree: &ModeTree) -> Result<()>` -- validate mode exists
- `ModeState::push_temporary(&mut self, name: &str, tree: &ModeTree) -> Result<()>` -- push onto stack
- `ModeState::pop_temporary(&mut self)` -- return to previous mode
- `ModeState::go_previous(&mut self)` -- pop stack or go to parent
- `ModeState::cycle(&mut self, modes: &[String], tree: &ModeTree) -> Result<()>` -- advance cycle index

**Cycle detection:**
- Before any mode switch, validate that the target mode exists in the tree
- For cycle: validate no duplicate modes in cycle list
- Detect if a temporary mode push would create a loop (same mode already on stack)

> **Lesson from Phase 2 code review:** The `Cycle { modes: Vec<String> }` variant in `ModeChangeStrategy` has public fields. Apply the same validated-constructor pattern: make fields private, add a constructor that rejects empty or duplicate mode lists, add a `CycleRaw` serde bridge. Validate at construction time so runtime `cycle()` stays infallible.

**Axis refresh:**
- `ModeState` should track when a mode change occurs by returning a `ModeChanged` flag/event
- The engine (Task 19) will use this flag to re-emit all cached axis values through the new mode's pipelines

**Tests to write:**
- Basic switch_to changes current mode
- Temporary mode: push, verify current, pop, verify reverted
- Nested temporary modes work (push A, push B, pop B -> A, pop A -> original)
- Previous goes back
- Cycle advances through modes in order, wraps around
- Error on switch to nonexistent mode
- Error on cycle with duplicate modes
- Temporary push of already-stacked mode is detected

**Steps:**
1. Define ModeState with stack and cycle tracking
2. Implement all switching strategies
3. Implement cycle detection and validation
4. Write comprehensive tests
5. Run `rtk cargo test -p inputforge-core -- mode_state`
6. Commit: `feat(core): add mode state machine with temporary modes and cycle detection`

---

## Phase 4: Conditions & Advanced Logic [COMPLETED]

> **Completed 2026-03-02.** All 3 tasks implemented. 235 tests, 98.58% region / 99.02% line coverage.
> - Task 12: HatDirection condition variant + get_hat() on InputCache + deeply nested condition tests
> - Task 13: Bidirectional merge integration tests for rudder pedal use case + full pipeline test
> - Task 14: CallbackRegistry with register/fire/cancel, PopTemporaryMode and Custom(FnOnce) variants

### Task 12: Condition Types & Evaluation

**Goal:** Already defined in Task 8 (Condition enum + evaluate_condition). This task adds condition evaluation tests and the HatDirection condition.

**Note:** If condition evaluation was already implemented and tested in Task 8, this task only adds `HatDirection` condition support and additional edge case tests.

**Files to modify:**
- `crates/inputforge-core/src/action.rs` -- add `HatDirection` variant to Condition if not already present
- `crates/inputforge-core/src/pipeline.rs` -- update `evaluate_condition` for HatDirection

**Additional condition variant:**
- `HatDirection { input: InputAddress, directions: Vec<HatDirection> }` -- true if hat is in any of the listed directions

**Additional tests:**
- HatDirection condition matches single direction
- HatDirection condition matches any of multiple directions
- Deeply nested All/Any/Not combinations

> **Lesson from Phase 2 code review:** When adding the `HatDirection` condition variant, audit ALL match arms in `execute_pipeline` and `evaluate_condition` for completeness. The `MapToKeyboard` bug (missing Hat guard, fixed in Phase 2 review) was caused by a new `InputValue` variant not being handled in all match sites. After adding `get_hat()` to `InputCache`, verify every `match &ctx.input_value` block handles all variants correctly.

**Steps:**
1. Add HatDirection condition variant if missing
2. Add corresponding evaluation logic
3. Add InputCache method: `get_hat(&InputAddress) -> HatDirection`
4. Audit all `match &ctx.input_value` and `match action` arms in pipeline.rs for completeness
5. Write additional tests
6. Run `rtk cargo test -p inputforge-core`
7. Commit: `feat(core): add hat direction condition support`

---

### Task 13: Axis Merging

**Goal:** Already implemented in pipeline executor (Task 8). This task adds integration tests and verifies bidirectional merge works for rudder pedals use case.

**Files to modify:**
- `crates/inputforge-core/src/pipeline.rs` -- add integration-level tests

**Tests to write:**
- Bidirectional merge: left pedal at -1 + right pedal at -1 = -1.0 (full left)
- Bidirectional merge: left pedal at -1 + right pedal at 1 = 1.0 (full right)
- Bidirectional merge: both centered = 0.0
- Full pipeline: two pedal axes with calibration + deadzone + merge + map to vJoy

**Steps:**
1. Add integration tests verifying the pedal merge use case end-to-end through the pipeline
2. Fix any issues with the bidirectional formula
3. Run `rtk cargo test -p inputforge-core`
4. Commit: `test(core): add axis merging integration tests for pedal use case`

---

### Task 14: Button Release Callback System

**Goal:** Engine infrastructure to schedule callbacks when buttons are released. Required for temporary modes (auto-pop on release) and future features.

**Files to create:**
- `crates/inputforge-core/src/callbacks.rs`

**Files to modify:**
- `crates/inputforge-core/src/lib.rs` -- add `pub mod callbacks;`

**Types to define:**
- `CallbackId(u64)` -- unique callback identifier
- `ReleaseCallback` enum: `PopTemporaryMode`, `Custom(Box<dyn FnOnce() + Send>)`
- `CallbackRegistry` struct: maps `InputAddress` -> list of `(CallbackId, ReleaseCallback)`

**Methods:**
- `CallbackRegistry::register(&mut self, input: InputAddress, callback: ReleaseCallback) -> CallbackId`
- `CallbackRegistry::fire(&mut self, input: &InputAddress) -> Vec<ReleaseCallback>` -- removes and returns all callbacks for that input
- `CallbackRegistry::cancel(&mut self, id: CallbackId)` -- remove specific callback

**Tests:**
- Register callback, fire it, verify it's returned and removed
- Multiple callbacks on same input all fire
- Cancel removes specific callback without affecting others
- Fire on unregistered input returns empty vec

**Steps:**
1. Define types and implement CallbackRegistry
2. Write tests
3. Run `rtk cargo test -p inputforge-core -- callbacks`
4. Commit: `feat(core): add button release callback registry`

---

## Phase 5: Hardware I/O [COMPLETED]

> **Completed 2026-03-02.** All 4 tasks implemented (parallelizable). 292 tests total after code review fixes.
> - Task 15: SDL3 input with hotplug, GUID device IDs, hat conversion, `!Send` by design (device/)
> - Task 16: vJoy output with cached dirty-flag flush, axis/button/hat mapping (output/)
> - Task 17: Keyboard output via Win32 SendInput, full key/modifier mapping (output/)
> - Task 18: HidHide IOCTL with blacklist/whitelist management, cleanup on Drop (device/)
> - Scaffolding commit separated hardware deps into feature flags (`sdl3-input`, `vjoy-output`, `win32-io`) + `test-util` for mocks
> - Two code review rounds (10 + 41 findings) hardened all I/O subsystems

> **Note:** Tasks 15-18 are independent and can be implemented in parallel.
> Each task implements a platform abstraction trait defined in the design doc Section 3.

### Task 15: SDL3 Input Source + Hotplug [COMPLETED]

**Goal:** Implement the `InputSource` trait using SDL3 for physical device input.

**Files created:**
- `crates/inputforge-core/src/device/mod.rs` -- re-exports, feature-gated submodules
- `crates/inputforge-core/src/device/traits.rs` -- `InputSource` + `DeviceHider` traits
- `crates/inputforge-core/src/device/sdl3.rs` -- `Sdl3Input` implementation
- `crates/inputforge-core/src/device/mock.rs` -- `MockInputSource` + `MockDeviceHider` (behind `test-util` feature)

**Files modified:**
- `crates/inputforge-core/src/lib.rs` -- added `pub mod device;`
- `crates/inputforge-core/Cargo.toml` -- added `sdl3` dependency (feature-gated `sdl3-input`)

**Trait defined in `traits.rs`:**
- `InputSource` trait:
  - `fn enumerate_devices(&self) -> Vec<DeviceInfo>`
  - `fn poll(&mut self) -> Vec<InputEvent>`
  - `fn is_device_connected(&self, id: &DeviceId) -> bool`
  - `fn hotplug_events(&self) -> Vec<HotplugEvent>`

**Notes from execution:**
- `Sdl3Input` is `!Send` by design — enforced via `PhantomData<*mut ()>` because SDL3 is not thread-safe
- Device IDs are SDL GUID strings (stable across reconnects)
- Instance path retrieved via FFI `SDL_GetJoystickPathForID` for HidHide matching
- Opens all connected joysticks on creation, caches `DeviceInfo` per device
- `poll()` translates `JoyAxisMotion`/`JoyButtonDown`/`JoyButtonUp`/`JoyHatMotion`/`JoyDeviceAdded`/`JoyDeviceRemoved` into `InputEvent`/`HotplugEvent`
- Hat conversion (`sdl_hat_to_direction`) maps all 9 SDL hat states to `HatDirection`
- `MockInputSource` pre-loadable with devices/events/hotplug events, drains on poll; `MockDeviceHider` records hide/unhide calls — both behind `test-util` feature

**Tests:** 1 SDL test (hardware-dependent), 6 mock tests.

**Commit:** `feat(core): add SDL3 input source with hotplug detection`

---

### Task 16: vJoy Output Sink [COMPLETED]

**Goal:** Implement the `OutputSink` trait using vJoy for virtual device output.

**Files created:**
- `crates/inputforge-core/src/output/mod.rs` -- re-exports, feature-gated submodules
- `crates/inputforge-core/src/output/traits.rs` -- `OutputSink` trait (with `Send` bound) + `KeyboardSink` trait
- `crates/inputforge-core/src/output/vjoy_output.rs` -- `VJoyOutput` implementation
- `crates/inputforge-core/src/output/mock.rs` -- `MockOutputSink` (behind `test-util` feature)

**Files modified:**
- `crates/inputforge-core/src/lib.rs` -- added `pub mod output;`
- `crates/inputforge-core/Cargo.toml` -- added `vjoy` dependency (feature-gated `vjoy-output`)

**Trait defined in `traits.rs`:**
- `OutputSink: Send` trait:
  - `fn create_device(&mut self, config: VirtualDeviceConfig) -> Result<()>`
  - `fn set_axis(&mut self, device: u8, axis: VJoyAxis, value: f64) -> Result<()>`
  - `fn set_button(&mut self, device: u8, button: u8, pressed: bool) -> Result<()>`
  - `fn set_hat(&mut self, device: u8, hat: u8, direction: HatDirection) -> Result<()>`
  - `fn release_device(&mut self, device: u8) -> Result<()>`
  - `fn flush(&mut self) -> Result<()>` (default no-op)

**Notes from execution:**
- Dirty-flag flush optimization: state changes cached in `HashMap<u8, Device>`, dirty devices tracked in `HashSet<u8>`, `flush()` calls `vjoy.update_device_state()` only for dirty devices — reduces IPC calls
- Reuses `lerp_range` from `processing::mod` for axis conversion `[-1.0, 1.0] → [0x0001, 0x8000]` (per Phase 2 lesson)
- Hat mapping: `HatState::Continuous` in hundredths-of-degrees (`u32::MAX` = center, `0` = N, `4500` = NE, ..., `31500` = NW)
- Axis ID mapping: X=1 through Slider1=8
- `Drop` impl flushes remaining dirty devices; `flush()` called before `release_device()`
- NaN guard on axis input values
- `MockOutputSink` records all calls as `OutputCall` enum variants (CreateDevice/SetAxis/SetButton/SetHat/ReleaseDevice/Flush)

**Tests:** 9 vJoy tests (axis conversion, hat mapping, flush behavior), 8 mock tests.

**Commit:** `feat(core): add vJoy output sink with axis/button/hat conversion`

---

### Task 17: Keyboard Output (Win32 SendInput) [COMPLETED]

**Goal:** Implement keyboard key press/release simulation using Win32 SendInput.

**Files created:**
- `crates/inputforge-core/src/output/keyboard.rs`

**Files modified:**
- `crates/inputforge-core/src/output/mod.rs` -- added `pub mod keyboard;`
- `crates/inputforge-core/Cargo.toml` -- added `windows` dependency with `Win32_UI_Input_KeyboardAndMouse` (feature-gated `win32-io`)

**Notes from execution:**
- `KeyboardOutput` is a zero-size struct implementing `KeyboardSink` trait (separate from `OutputSink` — keyboard is not a vJoy-style device)
- `KeyboardSink::send_key(combo)` does press-then-release in one call
- Internal `send_key(combo, pressed: bool)` builds up to 5 `INPUT` structs on the stack (4 modifiers + 1 main key) — stack array replaces heap Vec (per code review)
- `MapVirtualKeyW(MAPVK_VK_TO_VSC)` for scan code lookup, `KEYEVENTF_EXTENDEDKEY` set automatically for navigation keys (arrows, Home, End, PgUp, PgDn, Insert, Delete) and right-side modifiers
- Key mapping coverage: A-Z (ASCII), 0-9 (ASCII), F1-F24, Space, Enter/Return, Tab, Escape/Esc, Backspace, Delete/Del, Insert/Ins, Up/Down/Left/Right, Home, End, PageUp/PgUp, PageDown/PgDn
- Modifiers use left-side VKs (LControl, LShift, LAlt, LWin) by convention
- `OutputFailed` error variant with `GetLastError` context on `SendInput` failure

**Tests:** 16 tests (key string parsing, modifier combinations, extended key detection, full combo builds).

**Commit:** `feat(core): add keyboard output via Win32 SendInput`

---

### Task 18: HidHide Integration [COMPLETED]

**Goal:** Implement the `DeviceHider` trait using HidHide IOCTL.

**Files created:**
- `crates/inputforge-core/src/device/hidhide.rs`

**Files modified:**
- `crates/inputforge-core/src/device/mod.rs` -- added `pub mod hidhide;`
- `crates/inputforge-core/src/device/traits.rs` -- added `DeviceHider` trait

**Trait defined:**
- `DeviceHider` trait:
  - `fn hide_device(&mut self, device: &DeviceInfo) -> Result<()>`
  - `fn unhide_device(&mut self, device: &DeviceInfo) -> Result<()>`
  - `fn is_active(&self) -> bool`
  - `fn list_hidden_devices(&self) -> Result<Vec<String>>` (added in code review)

**Notes from execution:**
- Opens `\\.\HidHide` via `CreateFileW` with correct file flags
- IOCTL codes defined for GET/SET_BLACKLIST, GET/SET_WHITELIST, GET/SET_ACTIVE
- Full blacklist read/modify/write cycle for hide/unhide operations
- `whitelist_self()` adds current exe path to HidHide whitelist
- `set_active()` / `refresh_active()` to toggle and refresh HidHide state
- `Drop` impl cleans up: removes only devices *this instance* added to the blacklist, restores original active state, closes handle — logs `CloseHandle` failures
- Exponential-backoff retry on `ERROR_INSUFFICIENT_BUFFER` / `ERROR_MORE_DATA` in `read_multi_string`
- Double-null-terminated UTF-16LE encoding/decoding (`decode_multi_string` / `encode_multi_string`)
- Path canonicalization and odd byte count validation
- Replaced lossy UTF-16 decode with proper error handling
- Device matching uses `instance_path` field from `DeviceInfo` (added to `DeviceInfo` in code review for this purpose)

**Tests:** 13 tests (multi-string encoding/decoding, blacklist add/remove, active state toggle, path canonicalization, edge cases).

**Commit:** `feat(core): add HidHide device hiding integration`

---

### Phase 5 Code Review Fixes [COMPLETED]

**Goal:** Address findings from two code review rounds covering all I/O infrastructure and cross-cutting concerns.

**Round 1** (`f0cbf87`, 4 important + 6 minor):
- Added `instance_path: Option<String>` to `DeviceInfo` for HidHide device matching (I1)
- Tracked SDL3 binaries with Git LFS via `.gitattributes` (I2)
- Added vJoy dirty-flag flush optimization — cache state, `flush()` only sends changed devices (I3)
- Added HidHide retry loop with exponential backoff on `ERROR_INSUFFICIENT_BUFFER` (I4)
- Populated `wScan` via `MapVirtualKeyW` in keyboard output (M1)
- Replaced hardcoded linker path with `build.rs` (M2)
- Fixed `DeviceId` doc comment to be platform-agnostic (M3)
- Documented `Sdl3Input` thread safety (`!Send`) (M4)
- Documented vJoy acquire/relinquish lifecycle (M5)

**Round 2** (`c932301`, 41 findings across all subsystems):

| Category | Key fixes |
|----------|-----------|
| **Safety** | HidHide handle leak fix in constructor, device cleanup in `Drop`, `CloseHandle` failure logging, odd byte count validation, empty multi-string encoding fix, path canonicalization, lossy UTF-16 decode replaced with proper error |
| **Performance** | Keyboard stack array replaces heap `Vec`, key lookup allocation eliminated, VK arithmetic simplified, `swap_remove` in callbacks |
| **Quality** | `Send` bound on `OutputSink`, `KeyboardSink` trait extracted, `OutputFailed` error variant with `GetLastError` context, condition depth validation (`MAX_CONDITION_DEPTH = 32`), `VirtualDeviceConfig::validate()`, `BUTTON_PRESS_THRESHOLD` constant, `pub(crate) AxisValue::raw()`, NaN guard on axis input, `flush()` before vJoy release, explicit hat no-op arms in pipeline |

**Commits:**
- `fix(core): address code review findings across I/O infrastructure`
- `fix(core): resolve 41 code review findings across all subsystems`

**Lessons for future phases:**
1. **Drop impls are critical for I/O** — every resource-holding struct needs cleanup (vJoy flush, HidHide blacklist restore, handle close). Phase 6 Engine must ensure graceful shutdown order.
2. **Feature-gate platform code** — `#[cfg(feature = "...")]` keeps the build working on CI/platforms without hardware drivers. Apply to engine integration.
3. **`Send` bounds matter** — `OutputSink: Send` was missing initially. Phase 6 Engine will move sinks across threads; verify all trait bounds compile early.
4. **Stack over heap for fixed-size buffers** — keyboard `INPUT` array was heap-allocated unnecessarily. Apply this in engine hot path (avoid per-frame allocations).
5. **Validate at boundaries, trust internally** — `VirtualDeviceConfig::validate()`, condition depth limits. Engine should validate profile on load, then trust data in the processing loop.
6. **Guard NaN/infinity from hardware** — floating-point values from physical devices can be surprising. Axis values are guarded now; keep this pattern when wiring the engine loop.

**Phase 5 coverage checkpoint:** 292 tests total after code review fixes.

---

## Phase 6: Engine [IN PROGRESS]

> **Implemented 2026-03-03.** Engine event loop, shared state, and two code review rounds complete. 362 tests total. Engine integration tests deferred.
> - Engine decomposed into `engine/` module (mod.rs, command.rs, output_handler.rs, run.rs) instead of single file
> - State decomposed into `state/` module (mod.rs, cache.rs, device.rs, status.rs) instead of single file
> - `MockKeyboardSink` added to output/mock.rs for engine testing
> - Code review round 1 (8 findings: 2 critical, 5 important, 1 suggestion) applied
> - Code review round 2 (5 findings at 75/100 confidence: axis refresh, error handling, state cleanup, cache eviction, doc accuracy) applied

### Task 19: Engine Event Loop & AppState [COMPLETED]

**Goal:** Implement the main engine that ties everything together: polls input, routes through modes, executes pipelines, writes output.

**Files created:**
- `crates/inputforge-core/src/engine/mod.rs` -- `Engine` struct, constructor, custom Debug, Drop impl
- `crates/inputforge-core/src/engine/command.rs` -- `EngineCommand` enum (6 variants)
- `crates/inputforge-core/src/engine/output_handler.rs` -- `process_pipeline_outputs()`, `apply_mode_change()`, `refresh_axes_for_mode_change()`
- `crates/inputforge-core/src/engine/run.rs` -- `Engine::run()` loop, `Engine::tick()`, command processing, hotplug handling
- `crates/inputforge-core/src/state/mod.rs` -- `AppState` struct with `new()`, `with_profile()`, `Default`
- `crates/inputforge-core/src/state/cache.rs` -- `InputCacheStore` implementing `InputCache` trait
- `crates/inputforge-core/src/state/device.rs` -- `DeviceState` (info + connected)
- `crates/inputforge-core/src/state/status.rs` -- `EngineStatus` enum (Running, Paused, Stopped)

**Files modified:**
- `crates/inputforge-core/src/lib.rs` -- added `pub mod engine; pub mod state;`
- `crates/inputforge-core/src/output/mock.rs` -- added `MockKeyboardSink` + `KeyboardCall` (behind `test-util`)
- `crates/inputforge-core/src/output/mod.rs` -- re-export `MockKeyboardSink`, `KeyboardCall`
- `crates/inputforge-core/src/device/traits.rs` -- added thread safety docs to `InputSource` and `DeviceHider`

**Notes from execution:**
- Engine is `!Send` by design -- `InputSource` and `DeviceHider` are `!Send` (SDL3 same-thread constraint). Engine must be constructed and run on the same thread where InputSource was created. Documented on traits and `Engine::new`.
- `Engine::tick()` separates single-frame logic from `run()` loop for testability
- `POLL_INTERVAL = 1ms` (~1000 Hz) with `std::thread::sleep`
- Mappings and mode tree cloned out of `RwLock` at frame start to avoid holding lock during processing
- Event buffer reused via `std::mem::take` + restore pattern (avoids per-frame allocation while satisfying borrow checker -- original `drain().collect()` allocated every frame)
- Release callbacks fire BEFORE mapping resolution (intentional deviation from plan step ordering): when user releases a "shift" button, the pop happens first so the release event maps in the restored mode, not the temporary one
- Axis refresh on mode change re-processes all cached axes through new mode's pipelines; skips `SendKey` (avoids spurious key events) and `ChangeMode` (avoids recursion)
- `OutputId` type mismatches (e.g., `SetAxis` with `Button` OutputId) logged via `tracing::warn!` instead of silent skip
- `Engine::Drop` flushes output sink with `tracing::error!` on failure
- `Deactivate` command flushes pending output but does not release virtual devices (no device tracking yet)
- `EngineCommand` derives `Debug + PartialEq`
- Channel disconnect (`TryRecvError::Disconnected`) treated as shutdown signal
- `clone_into` optimization for per-frame mode string writes to shared state

**Tests:** 20 tests in state/ modules (14 cache + 1 evict_device, 3 AppState, 3 status, 3 device). 2 tests for MockKeyboardSink. 34 engine tests. Engine integration tests (6 planned scenarios) deferred.

**Commits:** (pending)
- `feat(core): add engine event loop with mode routing and pipeline execution`

### Phase 6 Code Review Fixes [COMPLETED]

#### Round 1: Implementation Review (8 findings)

**Goal:** Address 8 findings from code review (2 critical, 5 important, 1 suggestion).

| ID | Severity | Fix |
|----|----------|-----|
| C2 | Critical | Replaced per-frame `drain().collect()` with `std::mem::take` + restore pattern in event buffer |
| C3 | Critical | Documented `!Send` contract on `InputSource`, `DeviceHider` traits and `Engine::new` |
| I1 | Important | Documented intentional callback-before-mapping ordering deviation |
| I2 | Important | Added `tracing::warn!` on `OutputId` type mismatches (4 locations) |
| I3 | Important | Axis refresh skips `SendKey` outputs; removed unused `keyboard` param from `refresh_axes_for_mode_change` |
| I4 | Important | Updated `Deactivate` doc to match behavior ("flush pending output" not "release devices") |
| I5 | Important | Added `Engine::Drop` impl that flushes output sink |
| S1 | Suggestion | Added `PartialEq` derive to `EngineCommand` |

#### Round 2: Deep Review with 5-Agent Scoring (5 findings, all 75/100)

**Goal:** Address 5 findings from a second, deeper code review using 5 parallel review agents with confidence scoring (threshold 80, all scored 75).

| ID | Score | Fix |
|----|-------|-----|
| R2-1 | 75 | Pop-temporary on release now triggers axis refresh -- track mode before/after callbacks, OR flag into refresh condition (`engine/run.rs`) |
| R2-2 | 75 | Mode change errors (`ModeNotFound`, `ModeCycleDetected`) logged and skipped instead of terminating engine -- `apply_mode_change` returns `()` (`engine/output_handler.rs`) |
| R2-3 | 75 | `CallbackRegistry` cleared on `LoadProfile` -- added `clear()` method to `callbacks.rs`, called in handler (`engine/run.rs`) |
| R2-4 | 75 | `InputCacheStore` evicts entries on device disconnect -- added `evict_device()` method to `state/cache.rs`, called in hotplug handler (`engine/run.rs`) |
| R2-5 | 75 | `DeviceHider` thread-safety doc rewritten -- removed misleading "for consistency with InputSource" claim (`device/traits.rs`) |

3 additional findings scored below threshold and were not actioned:
- `SendKey { pressed: false }` silently dropped (50 -- intentional by design, confirmed by test)
- RwLock read guard held during pipeline execution (25 -- no actual contention, engine is sole writer)
- `assert_eq!(x, false)` style (15 -- Clippy will catch)

**Lessons for future phases:**
1. **Callback ordering matters for temporary modes** -- release callbacks must fire before mapping resolution so the release event resolves in the restored mode. This is a semantic invariant, not just an optimization.
2. **`std::mem::take` solves borrow checker conflicts in event loops** -- when iterating a buffer while needing `&mut self` for other fields, move the buffer out temporarily and restore it after.
3. **Axis refresh must be conservative** -- only apply axis and button outputs. Key presses and mode changes during refresh cause spurious side effects.
4. **`!Send` traits need explicit documentation** -- when a trait intentionally omits `Send`, document why and what construction constraints it implies for consumers.
5. **Callback-triggered mode changes need axis refresh too** -- not just pipeline-output mode changes. The pop-on-release path was invisible to the pipeline's `mode_changed` flag.
6. **Recoverable user-config errors must not crash the engine loop** -- bad mode names, cycle detection, etc. should be logged and skipped, not propagated as fatal errors.
7. **State cleanup on profile load must include ALL engine-owned state** -- callbacks, cache, and mode must all be reset together.
8. **Cache entries tied to physical devices should have a lifecycle matching device connection** -- evict on disconnect to prevent unbounded growth.

**Phase 6 coverage checkpoint:** 362 tests total (20 state + 2 mock keyboard + 34 engine + existing).

---

## Phase 7: GUI

### Task 20: GUI Foundation

**Goal:** Set up eframe app with Catppuccin Mocha theme, custom fonts, and main layout structure.

**Files to create:**
- `crates/inputforge-gui/src/app.rs`
- `crates/inputforge-gui/src/theme.rs`
- `crates/inputforge-gui/src/panels/mod.rs`
- `crates/inputforge-gui/src/widgets/mod.rs`

**Files to modify:**
- `crates/inputforge-gui/Cargo.toml` -- add eframe, egui, catppuccin-egui (with egui33 feature), egui_plot
- `crates/inputforge-gui/src/lib.rs` -- export modules and public launch function

**Assets to include:**
- Download Lexend font (Regular, Medium, SemiBold weights) and JetBrains Mono (Regular) -- embed as bytes using `include_bytes!`
- Place font files in `crates/inputforge-gui/assets/fonts/`

**Theme setup (theme.rs):**
- Apply Catppuccin Mocha using `catppuccin_egui::set_theme()` with the Mocha variant
- Override specific style properties for our design (see design doc Section 7):
  - Window rounding, widget rounding
  - Spacing adjustments
  - Selection color (Teal)
- Define semantic color constants: PRIMARY (Teal), SECONDARY (Peach), INFO (Blue), SUCCESS (Green), ERROR (Red), SPECIAL (Mauve)

**App struct (app.rs):**
- `InputForgeApp` implementing `eframe::App`
- Hold reference to `Arc<RwLock<AppState>>` and `mpsc::Sender<EngineCommand>`
- `update()` method: render main layout:
  - Top panel: logo text, profile dropdown, activate/deactivate button, settings gear
  - Left panel (280px fixed): device tree area + mode tree area (implemented in Task 21, 25)
  - Central panel (flex): context-dependent content area (stub for now)
  - Bottom panel: status bar with engine status, device count, vJoy status, mode badge

**Public function:**
- `pub fn launch_gui(state: Arc<RwLock<AppState>>, commands: mpsc::Sender<EngineCommand>) -> Result<()>` -- create and run eframe

**Steps:**
1. Add GUI dependencies to Cargo.toml
2. Download and embed font files
3. Implement theme.rs with Catppuccin Mocha + semantic colors
4. Implement app.rs with main layout structure (panels are stubs)
5. Implement launch function in lib.rs
6. Run `rtk cargo build -p inputforge-gui` (verify it compiles)
7. Commit: `feat(gui): add foundation with Catppuccin Mocha theme, fonts, and layout`

---

### Task 21: Device Panel

**Goal:** Left panel showing connected devices with live axis bars and button indicators.

**Files to create:**
- `crates/inputforge-gui/src/panels/device_panel.rs`

**Files to modify:**
- `crates/inputforge-gui/src/panels/mod.rs` -- add `pub mod device_panel;`

**Implementation:**
- Read device list and live values from `AppState`
- Render each device as a collapsible tree node:
  - Device name + connection status indicator (green/red dot)
  - **Axis bars**: horizontal bars, 6px height, Teal for positive values, Peach for negative, center tick mark, value in JetBrains Mono next to bar
  - **Button grid**: 12px circles in a grid layout, filled Teal when pressed, outlined Surface1 when released
  - **Hat indicators**: show current direction as arrow or "C" for center
- Clicking a device or input selects it for editing in the central panel

**Steps:**
1. Implement device panel widget reading from AppState
2. Implement axis bar custom painting using egui Painter API
3. Implement button grid layout
4. Implement hat direction display
5. Wire up selection to update central panel context
6. Run `rtk cargo build -p inputforge-gui`
7. Commit: `feat(gui): add device panel with live axis bars and button indicators`

---

### Task 22: Mapping Editor

**Goal:** Central panel for editing action pipelines on selected inputs.

**Files to create:**
- `crates/inputforge-gui/src/panels/mapping_panel.rs`
- `crates/inputforge-gui/src/widgets/action_card.rs`

**Files to modify:**
- `crates/inputforge-gui/src/panels/mod.rs`
- `crates/inputforge-gui/src/widgets/mod.rs`

**Implementation:**
- Show mapping list for the selected device input + current mode
- **Action cards** (design doc Section 7 "Key Widgets"):
  - Stacked cards connected by vertical flow lines
  - Each card has: left accent bar (colored by action type), action name, configuration fields, drag handle for reorder
  - Colors: Processing = Blue, Output = Green, Control = Mauve
- Add action button at bottom of pipeline (dropdown to pick action type)
- Delete action button on each card (X icon, confirmation)
- Drag-and-drop reorder of actions within the pipeline
- Each action type shows its specific configuration:
  - ResponseCurve: curve type dropdown, link to curve editor (Task 23)
  - Deadzone: four slider values
  - Calibrate: five values + enable toggle
  - Invert: no config needed
  - MapToVJoy: device + output type dropdowns
  - MapToKeyboard: key combo input field
  - MergeAxis: second input selector + operation dropdown
  - ChangeMode: strategy dropdown + mode selector
  - Conditional: condition editor (inline) + nested action lists for if_true/if_false

**Steps:**
1. Implement action_card widget with accent bar and drag handle
2. Implement mapping panel showing pipeline of action cards
3. Implement add/delete/reorder actions
4. Implement configuration UI for each action type
5. Wire up changes to update the Profile in AppState
6. Run `rtk cargo build -p inputforge-gui`
7. Commit: `feat(gui): add mapping editor with action pipeline cards`

---

### Task 23: Response Curve Editor

**Goal:** Interactive egui_plot-based response curve editor with draggable control points.

**Files to create:**
- `crates/inputforge-gui/src/widgets/curve_editor.rs`

**Files to modify:**
- `crates/inputforge-gui/src/widgets/mod.rs`

**Implementation (design doc Section 7 "Key Widgets" - Response curve editor):**
- Use `egui_plot::Plot` with equal aspect ratio, -1 to 1 on both axes
- Grid overlay: major grid at 0.5 intervals, minor at 0.1
- Identity line (diagonal dashed line) for reference
- **Curve rendering**: compute and draw the current response curve as a polyline
- **Control points**: draggable circles at each defined point
  - Piecewise linear: drag points along the curve
  - Cubic spline: drag interpolation points
  - Cubic bezier: drag endpoints + control point handles (lines connecting to control points)
- **Live input/output lines**:
  - Vertical line at current input value (Green)
  - Horizontal line at current output value (Green)
  - Intersection dot
- **Deadzone shading**: semi-transparent Red overlay on deadzone regions
- **Symmetry toggle**: checkbox, when enabled, editing positive side auto-mirrors to negative
- **Curve type selector**: tabs or dropdown to switch between Linear/Spline/Bezier
- Add/remove control points: right-click to add, delete key to remove selected

> **Lesson from Phase 2 code review:** Every point edit (drag, add, remove) must go through `ResponseCurve::piecewise_linear()` / `cubic_spline()` / `cubic_bezier()` factory methods, which validate invariants (>= 2 points, strictly increasing x, x >= 0 when symmetric, >= 1 bezier segment). Constrain drag handles to prevent crossing adjacent points (preserves x-monotonicity) and use the constructor as a safety net. On validation failure, revert the drag or show a user-facing error.

**Steps:**
1. Set up egui_plot widget with correct axis ranges and grid
2. Implement curve polyline rendering (evaluate curve at many points, draw as line)
3. Implement draggable control points using plot interaction
4. Implement bezier control point handles (lines from endpoint to control point)
5. Implement live input/output indicator lines
6. Implement deadzone shading overlay
7. Implement symmetry toggle
8. Implement add/remove control points
9. Run `rtk cargo build -p inputforge-gui`
10. Commit: `feat(gui): add interactive response curve editor with egui_plot`

---

### Task 24: Input Monitor

**Goal:** Real-time display of all input values for debugging and verification.

**Files to create:**
- `crates/inputforge-gui/src/panels/monitor_panel.rs`

**Files to modify:**
- `crates/inputforge-gui/src/panels/mod.rs`

**Implementation:**
- Central panel mode showing all active device inputs in real-time
- Table layout: Device | Input | Raw Value | Processed Value | Output
- Values update every frame from AppState
- Axis values shown as number + small inline bar
- Button values shown as filled/unfilled circle
- Hat values shown as direction arrow
- Optional: filter by device, highlight active (recently changed) inputs

**Steps:**
1. Implement monitor panel reading from AppState input cache
2. Layout as scrollable table
3. Add inline visualizations (bars, circles, arrows)
4. Add filtering and highlight for active inputs
5. Run `rtk cargo build -p inputforge-gui`
6. Commit: `feat(gui): add real-time input monitor panel`

---

### Task 25: Mode, Calibration & Deadzone Editors

**Goal:** GUI editors for mode tree management, per-axis calibration wizard, and deadzone configuration.

**Files to create:**
- `crates/inputforge-gui/src/panels/mode_panel.rs`
- `crates/inputforge-gui/src/widgets/deadzone_editor.rs`
- `crates/inputforge-gui/src/widgets/calibration_editor.rs`

**Files to modify:**
- `crates/inputforge-gui/src/panels/mod.rs`
- `crates/inputforge-gui/src/widgets/mod.rs`

**Mode panel:**
- Tree view of all modes with indent showing parent-child relationships
- Add/remove/rename modes
- Drag to reparent modes
- Current active mode highlighted (Mauve badge)
- Click mode to filter mapping editor to that mode

**Deadzone editor widget (design doc Section 7):**
- Horizontal bar representing [-1, 1] range
- 4 draggable boundary handles for low, center_low, center_high, high
- Center deadzone region shaded Red
- Live input marker (Green vertical line at current axis value)
- Numeric value labels on each handle

**Calibration editor widget:**
- Step-by-step wizard:
  1. "Move axis to minimum" -> record physical_min
  2. "Release axis to center" -> record physical_center_low and physical_center_high (wait for stabilization)
  3. "Move axis to maximum" -> record physical_max
- Live axis value display during calibration
- Manual override: edit 5 values directly
- Enable/disable toggle

> **Lesson from Phase 2 code review:** Both editors MUST go through validated constructors (`DeadzoneConfig::new()`, `Calibration::new()`) — the private fields enforce this at compile time. On every drag/edit, call the constructor and handle `Err` by showing a user-facing message (e.g., "center_low must be less than center_high") or by constraining drag handles to prevent invalid states (e.g., don't let center_low pass center_high). Same applies to numeric field edits: validate via constructor before applying.

**Steps:**
1. Implement mode panel with tree view and add/remove/rename
2. Implement deadzone editor with draggable handles using egui Painter API
3. Implement calibration wizard with step-by-step flow
4. Wire up to AppState and profile
5. Run `rtk cargo build -p inputforge-gui`
6. Commit: `feat(gui): add mode tree, deadzone, and calibration editors`

---

## Phase 8: Integration

### Task 26: System Tray, CLI Arguments, App Entry Point

**Goal:** Tie everything together: system tray icon, CLI parsing, engine thread, optional GUI.

**Files to modify:**
- `crates/inputforge-app/src/main.rs` -- full rewrite
- `crates/inputforge-app/Cargo.toml` -- add tray-icon dependency

**Files to create:**
- `crates/inputforge-app/src/tray.rs`
- `crates/inputforge-app/src/cli.rs`

**CLI arguments (clap derive):**
- `--profile <path>` -- load profile on startup
- `--enable` -- activate profile immediately
- `--start-minimized` -- start with GUI hidden (tray only)

**System tray (tray.rs):**
- Create tray icon using `tray-icon` crate
- Menu items: "Show/Hide GUI", separator, profile submenu (list .toml files from profiles directory), separator, "Activate/Deactivate", "Quit"
- Tray icon: use a simple embedded PNG icon (create a basic icon or use a placeholder)
- Double-click tray icon: toggle GUI visibility

**Main entry point (main.rs):**
1. Set mimalloc as global allocator
2. Parse CLI args with clap
3. Initialize tracing subscriber (with env filter)
4. Create shared `AppState` wrapped in `Arc<RwLock<>>`
5. Create mpsc channel for GUI->Engine commands
6. If `--profile` specified, load profile
7. Spawn engine thread: create SDL3 input, vJoy output, HidHide, keyboard output, engine. Run engine loop.
8. If not `--start-minimized`, launch GUI on main thread (eframe requires main thread)
9. If `--start-minimized`, show tray icon and enter tray event loop
10. If `--enable`, send Activate command to engine
11. On quit: send Shutdown command to engine, join engine thread, exit

**Steps:**
1. Implement CLI parsing with clap derive in cli.rs
2. Implement tray icon setup and menu in tray.rs
3. Rewrite main.rs to wire everything together
4. Handle graceful shutdown (engine thread join, vJoy release, HidHide unhide)
5. Test: run with `--help` to verify CLI
6. Test: run without arguments to verify GUI launches
7. Test: run with `--start-minimized` to verify tray-only mode
8. Commit: `feat(app): add system tray, CLI arguments, and app entry point`

---

## Post-Implementation Checklist

After all 26 tasks are complete:

1. Run full test suite: `rtk cargo test --workspace`
2. Run clippy: `rtk cargo clippy --workspace`
3. Run fmt check: `cargo fmt --check`
4. Run coverage report: `rtk cargo llvm-cov --workspace --html` -- open `target/llvm-cov/html/index.html` and verify:
   - `inputforge-core` has >95% line coverage
   - All processing modules (deadzone, calibration, curves, pipeline) have >95% coverage
   - Identify uncovered code paths and add tests if feasible
5. Run coverage summary: `rtk cargo llvm-cov --workspace` -- print text summary to verify thresholds
6. Manual integration test: connect a joystick, create a simple profile, verify axis mapping works
7. Create initial git tag: `v0.1.0`
