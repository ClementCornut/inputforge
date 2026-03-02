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
| toml            | 0.9+    | TOML 1.1 spec                                     |
| serde           | 1.0     | Use `features = ["derive"]`                       |
| thiserror       | 2.0+    | For core crate errors                             |
| anyhow          | 1.0     | For app crate errors                              |
| windows         | 0.62+   | For SendInput + HidHide IOCTL                     |
| clap            | 4.5+    | Use `features = ["derive"]`                       |
| parking_lot     | 0.12+   | Faster RwLock for shared state                    |
| tracing         | 0.1     | With tracing-subscriber 0.3                       |
| mimalloc        | 0.1     | Global allocator for app crate                    |

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

**Phase 2 coverage checkpoint:** Run `rtk cargo llvm-cov --workspace` after completing Phase 2. Processing modules should have >95% coverage.

### Phase 3: Profile & Mode System (Tasks 9-11)
- **Task 9**: Profile TOML serialization
- **Task 10**: Mode tree & inheritance
- **Task 11**: Mode switching, temporary modes, cycle detection, axis refresh

### Phase 4: Conditions & Advanced Logic (Tasks 12-14)
- **Task 12**: Condition types & evaluation
- **Task 13**: Axis merging
- **Task 14**: Button release callback system

**Phase 4 coverage checkpoint:** Run `rtk cargo llvm-cov --workspace` after completing Phase 4. Core logic (processing, mode, conditions, pipeline) should have >95% coverage overall.

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

## Phase 0: Prerequisite Tooling Check

**Goal:** Verify that all required tools are installed and accessible before writing any code. Fail fast if anything is missing.

**Checks to run (all must pass):**

1. `rustc --version` -- must be 1.85+ (edition 2024 support)
2. `cargo --version` -- must be present
3. `cmake --version` -- required for SDL3 build from source
4. `cl` or `cl.exe` -- MSVC compiler must be on PATH (run from Developer Command Prompt or verify VS Build Tools)
5. `cargo install cargo-llvm-cov` -- install if not present, then `cargo llvm-cov --version`
6. `rustup component add llvm-tools-preview` -- required by cargo-llvm-cov
7. Check vJoy driver: look for `C:\Program Files\vJoy\` or run `vJoyConf.exe` to verify driver is installed
8. Check HidHide: look for `C:\Program Files\Nefarius Software Solutions\HidHide\` or check Device Manager for HidHide device

**If any check fails:** Stop and inform the user. Do not proceed to Phase 1 until all prerequisites are met.

**No commit for this phase** -- it's a validation step only.

---

## Phase 1: Foundation

### Task 1: Workspace Scaffolding

**Goal:** Create the Rust workspace with 3 crates and all dependencies configured.

**Files to create:**
- `inputforge/Cargo.toml` -- workspace root with members, workspace dependencies, and workspace lints
- `inputforge/crates/inputforge-core/Cargo.toml` -- depends on serde, toml, thiserror, tracing, parking_lot
- `inputforge/crates/inputforge-core/src/lib.rs` -- empty stub
- `inputforge/crates/inputforge-gui/Cargo.toml` -- depends on inputforge-core, serde, tracing (GUI deps added later in Task 20)
- `inputforge/crates/inputforge-gui/src/lib.rs` -- empty stub
- `inputforge/crates/inputforge-app/Cargo.toml` -- depends on inputforge-core, inputforge-gui, anyhow, tracing, tracing-subscriber, mimalloc, clap (with derive)
- `inputforge/crates/inputforge-app/src/main.rs` -- minimal main that sets mimalloc as global allocator and prints version
- `inputforge/.gitignore` -- /target, *.swp, .DS_Store
- `inputforge/rustfmt.toml` -- edition 2024, max_width 100

**Instructions:**
1. Create the directory structure: `inputforge/crates/inputforge-{core,gui,app}/src/`
2. Write workspace `Cargo.toml` with `resolver = "2"`, all three members, `[workspace.package]` (version 0.1.0, edition 2024), `[workspace.dependencies]` for all shared deps, and `[workspace.lints]` sections following ms-rust M-STATIC-VERIFICATION guidelines (compiler lints + clippy lints including pedantic, cargo, restriction subset). Also allow `cast_precision_loss`, `cast_possible_truncation`, `cast_sign_loss`, `cast_possible_wrap`, and `module_name_repetitions` since we do float math.
3. Write each crate's `Cargo.toml` using `workspace = true` references and `[lints] workspace = true`
4. Write stub lib.rs / main.rs files
5. Run `rtk cargo build` to verify everything compiles
6. `git init`, add all files, commit: `feat(workspace): scaffold InputForge workspace with 3 crates`

---

### Task 2: Claude Hook for Automatic cargo fmt

**Goal:** Create a Claude Code hook that automatically runs `cargo fmt --all` after every Rust file change, ensuring consistent formatting without manual intervention.

**Files to create:**
- `inputforge/.claude/settings.json` -- project-level Claude Code settings with hooks

**Hook configuration:**
- Create a `PostToolUse` hook that triggers after `Edit` and `Write` tool uses
- The hook should match `.rs` files only (use `fileEditExtensions` or similar matcher)
- Command: `cargo fmt --all` (run from workspace root)
- This ensures every Rust file is automatically formatted after any modification

**Steps:**
1. Create `inputforge/.claude/` directory
2. Write `settings.json` with a `hooks` section containing a `PostToolUse` hook for `Edit` and `Write` tools that runs `cargo fmt --all` on `.rs` file changes
3. Verify the hook works by making a small change to any `.rs` file and confirming it gets formatted
4. Commit: `chore(workspace): add Claude hook for automatic cargo fmt`

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

## Phase 2: Processing Pipeline

### Task 5: Deadzone & Calibration

**Goal:** Implement deadzone and calibration processing as pure functions.

**Files to create:**
- `crates/inputforge-core/src/processing/mod.rs`
- `crates/inputforge-core/src/processing/deadzone.rs`
- `crates/inputforge-core/src/processing/calibration.rs`

**Files to modify:**
- `crates/inputforge-core/src/lib.rs` -- add `pub mod processing;`

**Deadzone module:**
- Define `DeadzoneConfig { low, center_low, center_high, high }` with serde derives and Default impl (low=-1, center_low=-0.05, center_high=0.05, high=1)
- Implement `apply(&self, value: f64) -> f64` method:
  - Below `low`: return -1.0
  - Between `low` and `center_low`: linearly interpolate to [-1, 0]
  - Between `center_low` and `center_high`: return 0.0
  - Between `center_high` and `high`: linearly interpolate to [0, 1]
  - Above `high`: return 1.0
- Write a helper `lerp_range(value, in_min, in_max, out_min, out_max) -> f64`

**Calibration module:**
- Define `Calibration { physical_min, physical_center_low, physical_center_high, physical_max, enabled }` (5-value band from design doc)
- Implement `apply(&self, value: f64) -> f64` method:
  - If disabled, pass through
  - Map [min, center_low] to [-1, 0], center band to 0, [center_high, max] to [0, 1]
  - Clamp beyond extremes

**Tests to write:**
- Deadzone: center returns 0, below low returns -1, above high returns 1, linear interpolation between zones, default config full range
- Calibration: min maps to -1, max maps to +1, center band maps to 0, disabled passes through, midpoints map correctly

**Steps:**
1. Create processing directory with mod.rs exporting both submodules
2. Write deadzone module with `DeadzoneConfig` and `apply()`
3. Write calibration module with `Calibration` and `apply()`
4. Run `rtk cargo test -p inputforge-core`
5. Commit: `feat(core): add deadzone and calibration processing`

---

### Task 6: Axis/Button Inversion

**Goal:** Simple value inversion functions.

**Files to create:**
- `crates/inputforge-core/src/processing/inversion.rs`

**Files to modify:**
- `crates/inputforge-core/src/processing/mod.rs` -- add `pub mod inversion;`

**Functions:**
- `invert_axis(value: f64) -> f64` -- negate the value
- `invert_button(pressed: bool) -> bool` -- toggle the state

**Tests:** Verify axis inversion negates, button inversion toggles, zero stays zero.

**Steps:**
1. Write module with both functions and tests
2. Export from processing/mod.rs
3. Run `rtk cargo test -p inputforge-core`
4. Commit: `feat(core): add axis and button inversion`

---

### Task 7: Response Curves

**Goal:** Implement all 3 response curve types with symmetry support.

**Files to create:**
- `crates/inputforge-core/src/processing/curves.rs`

**Files to modify:**
- `crates/inputforge-core/src/processing/mod.rs` -- add `pub mod curves;`

**Types to define:**
- `ResponseCurve` enum (serde tagged by "kind"):
  - `PiecewiseLinear { points: Vec<(f64, f64)>, symmetric: bool }`
  - `CubicSpline { points: Vec<(f64, f64)>, symmetric: bool }`
  - `CubicBezier { segments: Vec<BezierSegment>, symmetric: bool }`
- `BezierSegment { start, cp1, cp2, end }` (each is `(f64, f64)`)

**Implement `evaluate(&self, input: f64) -> f64` on `ResponseCurve`:**

1. **Piecewise linear**: Find the segment containing input x, linearly interpolate y. Clamp outside range.
2. **Cubic spline**: Natural cubic spline interpolation using the Thomas algorithm to compute coefficients, then evaluate the correct segment.
3. **Cubic bezier**: For each segment, find parameter t such that bezier_x(t) = x (Newton's method with bisection fallback), then compute bezier_y(t).

**Symmetry support:**
- `maybe_mirror(points, symmetric)`: When symmetric=true, mirror positive-side points to negative side (negate both x and y)
- `mirror_bezier_segments(segments)`: Reverse and negate segments for negative side

**Helper functions:**
- `bezier_x(seg, t)`, `bezier_y(seg, t)` -- standard cubic bezier evaluation
- `bezier_dx(seg, t)` -- derivative for Newton's method
- `find_t_for_x(seg, x)` -- Newton's method (8 iterations) with bisection fallback (50 iterations)
- `compute_spline_coefficients(points)` -- Thomas algorithm for natural cubic spline

**Tests to write:**
- Piecewise linear: identity curve, custom S-curve midpoints, clamping outside range
- Cubic spline: passes through defined points, endpoint values correct
- Cubic bezier: endpoints correct, linear control points produce linear output
- Symmetry: mirrored curve produces antisymmetric values (f(-x) = -f(x))

**Steps:**
1. Write the curve types with serde derives
2. Implement piecewise linear evaluation (simplest)
3. Implement cubic spline with Thomas algorithm
4. Implement cubic bezier with Newton's method
5. Implement symmetry mirroring for all types
6. Write comprehensive tests
7. Run `rtk cargo test -p inputforge-core -- curves`
8. Commit: `feat(core): add response curves (linear, spline, bezier) with symmetry`

---

### Task 8: Pipeline Executor & Action Types

**Goal:** Define action types and implement the pipeline executor that processes input through ordered action lists.

**Files to create:**
- `crates/inputforge-core/src/action.rs`
- `crates/inputforge-core/src/pipeline.rs`

**Files to modify:**
- `crates/inputforge-core/src/lib.rs` -- add `pub mod action; pub mod pipeline;`

**Action types to define (see design doc Section 4):**
- `Action` enum (serde tagged by "type"):
  - Processing: `ResponseCurve`, `Deadzone`, `Calibrate`, `Invert`
  - Output: `MapToVJoy { output }`, `MapToKeyboard { key }`, `MergeAxis { second_input, operation }`
  - Control flow: `ChangeMode { strategy }`, `Conditional { condition, if_true, if_false }`
- `ModeChangeStrategy` enum: `SwitchTo`, `Temporary`, `Previous`, `Cycle`
- `Condition` enum: `ButtonPressed`, `ButtonReleased`, `AxisInRange`, `All`, `Any`, `Not`
- `Mapping { input: InputAddress, mode: String, actions: Vec<Action> }`

**Pipeline executor:**
- Define `PipelineOutput` enum: `SetAxis`, `SetButton`, `SendKey`, `ChangeMode`
- Define `InputCache` trait with `get_button(&InputAddress) -> bool` and `get_axis(&InputAddress) -> f64`
- Define `PipelineContext` struct: holds `current_value: f64`, `input_value: InputValue`, `outputs: Vec<PipelineOutput>`, `input_cache: &dyn InputCache`
- Implement `execute_pipeline(actions: &[Action], ctx: &mut PipelineContext)`:
  - Walk actions in order
  - Processing actions: transform `ctx.current_value` (call the corresponding processing module)
  - Output actions: push to `ctx.outputs`
  - `Conditional`: evaluate condition against input cache, recurse into if_true or if_false sub-pipeline
  - `MergeAxis`: read second axis from cache, apply merge operation, update current_value
- Implement `evaluate_condition(condition, cache) -> bool` -- recursive evaluation
- Implement `merge_axes(a, b, op) -> f64` for each MergeOp variant

**Tests to write (use a MockCache struct implementing InputCache):**
- Empty pipeline produces no output
- Axis value passes through MapToVJoy correctly
- Invert then MapToVJoy negates the value
- Conditional branches correctly based on button state in cache
- MergeAxis: bidirectional, average, maximum operations
- Nested conditions (All, Any, Not) evaluate correctly

**Steps:**
1. Write action types in action.rs
2. Write pipeline executor in pipeline.rs with PipelineContext and InputCache trait
3. Write MockCache for tests
4. Write all unit tests
5. Run `rtk cargo test -p inputforge-core`
6. Commit: `feat(core): add action types and pipeline executor`

---

## Phase 3: Profile & Mode System

### Task 9: Profile TOML Serialization

**Goal:** Define the Profile struct and implement save/load from TOML files.

**Files to create:**
- `crates/inputforge-core/src/profile.rs`

**Files to modify:**
- `crates/inputforge-core/src/lib.rs` -- add `pub mod profile;`

**Types to define:**
- `Profile { name, devices: Vec<DeviceEntry>, modes: ModeTree, mappings: Vec<Mapping>, settings: ProfileSettings }`
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
- `ModeTree { root: ModeNode }` (serializable)
- `ModeNode { name: String, children: Vec<ModeNode> }`

**Methods to implement:**
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

**Files to create:**
- `crates/inputforge-core/src/mode_state.rs`

**Files to modify:**
- `crates/inputforge-core/src/lib.rs` -- add `pub mod mode_state;`

**Types to define:**
- `ModeState` struct: holds current mode name, mode stack (for temporary modes), cycle index (for Cycle strategy)

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

## Phase 4: Conditions & Advanced Logic

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

**Steps:**
1. Add HatDirection condition variant if missing
2. Add corresponding evaluation logic
3. Add InputCache method: `get_hat(&InputAddress) -> HatDirection`
4. Write additional tests
5. Run `rtk cargo test -p inputforge-core`
6. Commit: `feat(core): add hat direction condition support`

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

## Phase 5: Hardware I/O

> **Note:** Tasks 14-17 are independent and can be implemented in parallel.
> Each task implements a platform abstraction trait defined in the design doc Section 3.

### Task 15: SDL3 Input Source + Hotplug

**Goal:** Implement the `InputSource` trait using SDL3 for physical device input.

**Files to create:**
- `crates/inputforge-core/src/device/mod.rs`
- `crates/inputforge-core/src/device/traits.rs`
- `crates/inputforge-core/src/device/sdl3.rs`

**Files to modify:**
- `crates/inputforge-core/src/lib.rs` -- add `pub mod device;`
- `crates/inputforge-core/Cargo.toml` -- add `sdl3` dependency

**Trait to define in `traits.rs`:**
- `InputSource` trait (from design doc Section 3):
  - `fn enumerate_devices(&self) -> Vec<DeviceInfo>`
  - `fn poll(&mut self) -> Vec<InputEvent>`
  - `fn is_device_connected(&self, id: &DeviceId) -> bool`

**SDL3 implementation in `sdl3.rs`:**
- `Sdl3Input` struct: wraps SDL3 joystick/gamepad subsystem
- `Sdl3Input::new() -> Result<Self>` -- initialize SDL3 with joystick+gamepad subsystems
- Implement `InputSource` for `Sdl3Input`:
  - `enumerate_devices`: use SDL3 joystick enumeration, map to DeviceInfo
  - `poll`: pump SDL events, convert joystick axis/button/hat events to InputEvent
  - Map SDL3 instance IDs to our DeviceId (using GUID)
- **Hotplug**: SDL3 fires `JoyDeviceAdded` / `JoyDeviceRemoved` events during polling. Capture these and expose via a method `hotplug_events(&self) -> Vec<HotplugEvent>` where `HotplugEvent` is `Connected(DeviceInfo)` or `Disconnected(DeviceId)`.

**Tests:**
- Unit tests are limited since SDL3 requires real hardware
- Create a `MockInputSource` implementing `InputSource` for use in other tests
- Test DeviceId GUID formatting
- Integration test (manual): connect a joystick and verify enumeration output

**Steps:**
1. Define `InputSource` trait in traits.rs
2. Add `sdl3` dependency to Cargo.toml
3. Implement `Sdl3Input` struct
4. Create `MockInputSource` for testing
5. Run `rtk cargo build -p inputforge-core` (verify SDL3 compiles)
6. Commit: `feat(core): add SDL3 input source with hotplug detection`

---

### Task 16: vJoy Output Sink

**Goal:** Implement the `OutputSink` trait using vJoy for virtual device output.

**Files to create:**
- `crates/inputforge-core/src/output/mod.rs`
- `crates/inputforge-core/src/output/traits.rs`
- `crates/inputforge-core/src/output/vjoy_output.rs`

**Files to modify:**
- `crates/inputforge-core/src/lib.rs` -- add `pub mod output;`
- `crates/inputforge-core/Cargo.toml` -- add `vjoy` dependency

**Trait to define in `traits.rs`:**
- `OutputSink` trait (from design doc Section 3):
  - `fn create_device(&mut self, config: VirtualDeviceConfig) -> Result<()>`
  - `fn set_axis(&mut self, device: u8, axis: VJoyAxis, value: f64) -> Result<()>`
  - `fn set_button(&mut self, device: u8, button: u8, pressed: bool) -> Result<()>`
  - `fn set_hat(&mut self, device: u8, hat: u8, direction: HatDirection) -> Result<()>`
  - `fn release_device(&mut self, device: u8) -> Result<()>`

**vJoy implementation in `vjoy_output.rs`:**
- `VJoyOutput` struct: wraps the vjoy crate
- Map our `AxisValue` (-1.0 to 1.0) to vJoy range (typically 0x0000-0x7FFF)
- Map `VJoyAxis` enum to vJoy axis constants
- Map `HatDirection` to vJoy hat values (degrees * 100 or -1 for center)
- Acquire/relinquish vJoy devices

**Tests:**
- Create `MockOutputSink` implementing `OutputSink` that records calls
- Test axis value conversion (float to vJoy integer range)
- Test hat direction to degree conversion
- Integration test (manual): create vJoy device and set axes

**Steps:**
1. Define `OutputSink` trait in traits.rs
2. Add `vjoy` dependency to Cargo.toml
3. Implement `VJoyOutput` struct with value conversions
4. Create `MockOutputSink` for testing
5. Run `rtk cargo build -p inputforge-core`
6. Commit: `feat(core): add vJoy output sink`

---

### Task 17: Keyboard Output (Win32 SendInput)

**Goal:** Implement keyboard key press/release simulation using Win32 SendInput.

**Files to create:**
- `crates/inputforge-core/src/output/keyboard.rs`

**Files to modify:**
- `crates/inputforge-core/src/output/mod.rs` -- add `pub mod keyboard;`
- `crates/inputforge-core/Cargo.toml` -- add `windows` dependency with required features (Win32_UI_Input_KeyboardAndMouse)

**Implementation:**
- `KeyboardOutput` struct
- `send_key(&self, combo: &KeyCombo, pressed: bool) -> Result<()>`:
  - Parse key string to virtual key code (VK_*)
  - Build `INPUT` struct array with modifier keys + main key
  - Call `SendInput` from the `windows` crate
  - For release: send in reverse order (main key up, then modifiers up)
- Key string to VK mapping: support common names ("F1"-"F24", "A"-"Z", "0"-"9", "Space", "Enter", "Tab", "Escape", arrow keys, etc.)

**Tests:**
- Key string parsing: "F1" -> VK_F1, "A" -> VK_A, etc.
- Modifier parsing: KeyCombo with Ctrl+Shift produces correct INPUT array
- Create `MockKeyboardOutput` for testing in pipeline

**Steps:**
1. Add `windows` dependency with correct features
2. Implement key string to VK code mapping
3. Implement SendInput wrapper
4. Write key parsing tests
5. Run `rtk cargo build -p inputforge-core`
6. Commit: `feat(core): add keyboard output via Win32 SendInput`

---

### Task 18: HidHide Integration

**Goal:** Implement the `DeviceHider` trait using HidHide IOCTL.

**Files to create:**
- `crates/inputforge-core/src/device/hidhide.rs`

**Files to modify:**
- `crates/inputforge-core/src/device/mod.rs` -- add `pub mod hidhide;`
- `crates/inputforge-core/src/device/traits.rs` -- add `DeviceHider` trait

**Trait to define:**
- `DeviceHider` trait (from design doc Section 3):
  - `fn hide_device(&mut self, device: &DeviceInfo) -> Result<()>`
  - `fn unhide_device(&mut self, device: &DeviceInfo) -> Result<()>`
  - `fn is_active(&self) -> bool`

**HidHide implementation:**
- `HidHideManager` struct
- Communicate with HidHide driver via IOCTL:
  - Open device handle to `\\.\HidHide`
  - Use `DeviceIoControl` to add/remove device instance paths to the blacklist
  - Toggle HidHide active/inactive state
- Map SDL3 device info to Windows device instance path (may need to use SetupAPI to find the device path from vendor/product ID)

**Tests:**
- Create `MockDeviceHider` for testing
- Test that hide/unhide calls are tracked
- Integration test (manual): verify HidHide driver communication

**Steps:**
1. Add `DeviceHider` trait to traits.rs
2. Research HidHide IOCTL interface (check HidHide GitHub docs)
3. Implement `HidHideManager` with IOCTL calls via `windows` crate
4. Create `MockDeviceHider`
5. Run `rtk cargo build -p inputforge-core`
6. Commit: `feat(core): add HidHide device hiding integration`

---

## Phase 6: Engine

### Task 19: Engine Event Loop & AppState

**Goal:** Implement the main engine that ties everything together: polls input, routes through modes, executes pipelines, writes output.

**Files to create:**
- `crates/inputforge-core/src/engine.rs`
- `crates/inputforge-core/src/state.rs`

**Files to modify:**
- `crates/inputforge-core/src/lib.rs` -- add `pub mod engine; pub mod state;`

**AppState (shared between engine and GUI):**
- Define `AppState` struct (design doc Section 3 "Thread Architecture"):
  - `devices: Vec<DeviceState>` -- live device list with current values
  - `current_mode: String`
  - `engine_status: EngineStatus` (enum: Running, Paused, Stopped)
  - `active_profile: Option<Profile>`
  - `input_cache: InputCacheStore` -- HashMap<InputAddress, InputValue> implementing InputCache trait
- Wrap in `Arc<RwLock<AppState>>`

**Engine commands (sent from GUI to engine via mpsc):**
- `EngineCommand` enum: `LoadProfile(PathBuf)`, `Activate`, `Deactivate`, `Pause`, `Resume`, `Shutdown`

**Engine struct:**
- `Engine` struct: holds InputSource, OutputSink, DeviceHider, KeyboardOutput, AppState, command receiver, CallbackRegistry, ModeState
- `Engine::new(input: impl InputSource, output: impl OutputSink, ...)` -- constructor with trait objects
- `Engine::run(&mut self)` -- main loop:
  1. Check for commands from GUI (non-blocking recv)
  2. Poll input source for events
  3. Handle hotplug events (update device list in AppState)
  4. For each input event:
     a. Update input cache
     b. Resolve mapping via mode tree (using `resolve_mapping` from Task 10)
     c. Execute action pipeline (using `execute_pipeline` from Task 8)
     d. Process pipeline outputs: write to vJoy, send keyboard, handle mode changes
     e. Check for button release -> fire callbacks
  5. Sleep to maintain target poll rate (~1ms)
- On mode change: re-emit all cached axis values through new mode's pipelines (axis refresh, feature 30)

**Tests (using mock implementations for all I/O):**
- Engine processes an axis event through a mapping and produces vJoy output
- Engine handles mode switch command
- Engine handles profile load command
- Engine pauses and resumes
- Axis refresh: changing mode re-processes cached axis values
- Button release fires registered callbacks (temporary mode auto-pop)

**Steps:**
1. Define AppState and InputCacheStore in state.rs
2. Define EngineCommand enum
3. Implement Engine struct with main loop
4. Wire up all components: input polling, mode routing, pipeline execution, output writing
5. Implement axis refresh on mode change
6. Write tests with mocks
7. Run `rtk cargo test -p inputforge-core -- engine`
8. Commit: `feat(core): add engine event loop with mode routing and pipeline execution`

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
