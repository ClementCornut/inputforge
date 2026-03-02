# InputForge - Design Document

**Date**: 2026-03-02
**Status**: Approved
**Author**: Claude + User (collaborative brainstorming)

---

## 1. Overview

**InputForge** is a Windows-first Rust application for remapping physical joystick, pedal, and throttle inputs to virtual vJoy devices. It targets flight simulation (DCS, MSFS, IL-2) and space simulation (Star Citizen, Elite Dangerous) users who need flexible input remapping with response curves, deadzones, modes/layers, and conditional logic.

### Goals
- Replace JoystickGremlin with a faster, more reliable Rust-based tool
- Clean architecture with engine/GUI separation
- Pipeline-based action model (composable, multi-output)
- Professional-grade GUI for configuration
- Headless engine operation (GUI is optional, system tray when not configuring)

### Non-Goals for v1
- User scripting (Lua/Rhai) -- future
- Force feedback passthrough -- future
- Macro system (key sequences) -- future
- Cross-platform (Linux) -- future (but architected for it)
- JoystickGremlin profile import -- future

---

## 2. Technology Stack

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| Language | Rust | Performance, safety, single binary distribution |
| Physical input | SDL3 (`sdl3` crate v0.16+) | Gold standard device compatibility, cross-platform |
| Virtual output | vJoy (`vjoy` crate v0.7+) | Kernel-level virtual HID devices visible to all games |
| Device hiding | HidHide (via `windows` crate IOCTL) | Prevent double input from physical + virtual devices |
| GUI framework | egui (`eframe` 0.33+) | Immediate mode, perfect for real-time input viz, pure Rust |
| Plotting | `egui_plot` | Response curve visualization and interaction |
| Theming | `catppuccin-egui` | Professional dark theme (Mocha variant) |
| System tray | `tray-icon` | Show/hide GUI, engine status |
| Profile format | TOML (`toml` + `serde`) | Human-readable, Rust ecosystem standard |
| Keyboard output | Win32 SendInput (`windows` crate) | Simulate keyboard key presses |

---

## 3. Architecture

### Workspace Structure

```
inputforge/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── inputforge-core/    # Engine library (no GUI dependency)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── device/         # Device enumeration & reading (SDL3)
│   │   │   ├── output/         # vJoy output + HidHide
│   │   │   ├── mapping/        # Axis/button/hat mapping logic
│   │   │   ├── processing/     # Response curves, deadzone, calibration
│   │   │   ├── mode/           # Mode/layer system with inheritance
│   │   │   ├── condition/      # Condition evaluation
│   │   │   ├── profile/        # TOML profile load/save
│   │   │   └── engine.rs       # Main event loop & orchestration
│   │
│   ├── inputforge-gui/     # egui GUI (depends on core)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── app.rs          # eframe App implementation
│   │   │   ├── panels/         # Device list, input config, monitor
│   │   │   ├── widgets/        # Response curve editor, axis display
│   │   │   └── theme.rs        # Catppuccin Mocha theme setup
│   │
│   └── inputforge-app/     # Binary entry point
│       └── src/
│           └── main.rs         # System tray + spawn GUI + start engine
```

### Key Design Principles

1. **`inputforge-core` has zero GUI dependencies** -- usable as a library
2. **`inputforge-gui` depends on `inputforge-core`** but not vice versa
3. Communication via shared `AppState` behind `Arc<RwLock<>>` + command channels
4. Engine runs on dedicated thread, GUI on main thread
5. Clean trait boundaries for future platform abstraction

### Platform Abstraction Traits

```rust
/// Reads physical devices
trait InputSource {
    fn enumerate_devices(&self) -> Vec<DeviceInfo>;
    fn poll(&mut self) -> Vec<InputEvent>;
}

/// Writes to virtual devices
trait OutputSink {
    fn create_device(&mut self, config: VirtualDeviceConfig) -> DeviceId;
    fn set_axis(&mut self, device: DeviceId, axis: AxisId, value: f64);
    fn set_button(&mut self, device: DeviceId, button: ButtonId, pressed: bool);
    fn set_hat(&mut self, device: DeviceId, hat: HatId, direction: HatDirection);
}

/// Hides physical devices from other applications
trait DeviceHider {
    fn hide_device(&mut self, device: &DeviceInfo) -> Result<()>;
    fn unhide_device(&mut self, device: &DeviceInfo) -> Result<()>;
}
```

**Platform implementations:**

| Trait | Windows | Linux (future) |
|-------|---------|----------------|
| `InputSource` | `Sdl3Input` | `Sdl3Input` (same) |
| `OutputSink` | `VJoyOutput` | `UinputOutput` |
| `DeviceHider` | `HidHideManager` | `EvdevGrabber` (EVIOCGRAB) |

### Thread Architecture

```
ENGINE THREAD:
  SDL3 Poller (~1000Hz) → Event Queue → Mode Router → Action Pipeline Executor
                                                         ├→ vJoy Output
                                                         ├→ Keyboard Output
                                                         └→ Input Cache update

GUI THREAD (optional):
  Reads shared AppState (Arc<RwLock<AppState>>)
  Sends commands via mpsc channel: LoadProfile, Activate, Deactivate, Pause, Resume

SHARED STATE:
  AppState {
      devices: Vec<DeviceState>,        // Live device list with values
      current_mode: String,             // Active mode
      engine_status: EngineStatus,      // Running/Paused/Stopped
      active_profile: Option<Profile>,  // Loaded profile
      input_cache: InputCache,          // Last known value of every input
  }
```

---

## 4. Data Model

### Core Types

```rust
struct DeviceId(String);          // SDL3 GUID, stable across reconnects

struct InputAddress {
    device: DeviceId,
    input: InputId,
}

enum InputId {
    Axis(u8),
    Button(u8),
    Hat(u8),
}

struct OutputAddress {
    device: u8,                   // vJoy device ID (1-16)
    output: OutputId,
}

enum OutputId {
    Axis(VJoyAxis),               // X, Y, Z, Rx, Ry, Rz, Slider0, Slider1
    Button(u8),                   // 1-128
    Hat(u8),                      // 1-4
}

struct AxisValue(f64);            // -1.0 to 1.0

enum HatDirection {
    Center, N, NE, E, SE, S, SW, W, NW,
}
```

### Profile Model

```rust
struct Profile {
    name: String,
    devices: Vec<DeviceEntry>,
    modes: ModeTree,
    mappings: Vec<Mapping>,
    settings: ProfileSettings,
}

struct Mapping {
    input: InputAddress,
    mode: String,
    actions: Vec<Action>,         // Pipeline: ordered list of actions
}
```

### Action Pipeline Model

Actions execute in order. Processing actions transform the value. Output actions send to destinations. Multiple outputs per input supported.

```rust
enum Action {
    // Processing (transforms value, passes to next)
    ResponseCurve { curve: ResponseCurve },
    Deadzone { config: DeadzoneConfig },
    Calibrate { config: Calibration },
    Invert,

    // Output (produces side effects)
    MapToVJoy { output: OutputAddress },
    MapToKeyboard { key: KeyCombo },
    MergeAxis { second_input: InputAddress, operation: MergeOp },

    // Control flow
    ChangeMode { strategy: ModeChangeStrategy },
    Conditional {
        condition: Condition,
        if_true: Vec<Action>,
        if_false: Option<Vec<Action>>,
    },
}

struct Calibration {
    physical_min: f64,
    physical_center_low: f64,   // Center is a band, not a point (from JG scan)
    physical_center_high: f64,
    physical_max: f64,
    enabled: bool,
}

struct DeadzoneConfig { low: f64, center_low: f64, center_high: f64, high: f64 }

enum ResponseCurve {
    PiecewiseLinear { points: Vec<(f64, f64)>, symmetric: bool },
    CubicSpline { points: Vec<(f64, f64)>, symmetric: bool },
    CubicBezier { segments: Vec<BezierSegment>, symmetric: bool },
}
// symmetric: auto-mirrors control points across center (from JG scan)

struct BezierSegment {
    start: (f64, f64), control1: (f64, f64), control2: (f64, f64), end: (f64, f64),
}
```

### Mode System

```rust
struct ModeTree { root: ModeNode }
struct ModeNode { name: String, children: Vec<ModeNode> }

enum ModeChangeStrategy {
    SwitchTo(String),
    Temporary(String),      // Active while held, auto-pops on release
    Previous,
    Cycle(Vec<String>),
}
```

### Conditions

```rust
enum Condition {
    ButtonPressed { input: InputAddress },
    ButtonReleased { input: InputAddress },
    AxisInRange { input: InputAddress, min: f64, max: f64 },
    HatDirection { input: InputAddress, directions: Vec<HatDirection> },
    All(Vec<Condition>),
    Any(Vec<Condition>),
    Not(Box<Condition>),
}
```

### TOML Profile Format

```toml
[profile]
name = "Star Citizen HOSAS"
startup_mode = "Default"

[[devices]]
id = "030000005e040000ea02000000007801"
name = "VKB Gladiator NXT Left"

[modes]
Default = []
Combat = ["Missiles", "Guns"]
Landing = []

# Axis with full pipeline
[[mappings]]
input = { device = "03000000...", type = "axis", index = 0 }
mode = "Default"

[[mappings.actions]]
type = "calibrate"
min = -32768
center = 0
max = 32767

[[mappings.actions]]
type = "deadzone"
center_low = -0.03
center_high = 0.03

[[mappings.actions]]
type = "response_curve"
kind = "cubic_spline"
points = [[-1.0, -1.0], [-0.5, -0.2], [0.0, 0.0], [0.5, 0.2], [1.0, 1.0]]

[[mappings.actions]]
type = "map_to_vjoy"
output = { device = 1, type = "axis", id = "X" }

# Button with dual output
[[mappings]]
input = { device = "03000000...", type = "button", index = 0 }
mode = "Default"

[[mappings.actions]]
type = "map_to_vjoy"
output = { device = 1, type = "button", id = 1 }

[[mappings.actions]]
type = "conditional"
condition = { type = "button_pressed", device = "03000000...", index = 5 }
if_true = [{ type = "map_to_keyboard", key = "Ctrl+F1" }]
if_false = [{ type = "map_to_keyboard", key = "F1" }]
```

---

## 5. v1 Feature List (26 features)

### Core (Tier 1)
1. Physical device enumeration & reading (SDL3)
2. vJoy virtual device output (axes, buttons, hats)
3. Button-to-vJoy-button mapping
4. Axis-to-vJoy-axis mapping (absolute)
5. Hat-to-vJoy-hat mapping
6. Response curves (piecewise linear)
7. Deadzone (4-parameter: low, center_low, center_high, high)
8. TOML profile save/load
9. egui configuration GUI
10. System tray with headless engine
11. HidHide integration
12. Device hotplug detection
13. Profile activate/deactivate

### Important (Tier 2)
14. Response curves - cubic spline
15. Axis inversion
16. Button inversion
17. Mode/layer system (tree hierarchy)
18. Temporary/shift mode (hold button = temporary mode)
19. Mode inheritance (unmapped inputs fall through to parent)
20. Axis merging (bidirectional -- ideal for rudder pedals)
21. Button-to-keyboard mapping
22. Conditions (button state: pressed/released)
23. Real-time input monitor in GUI
24. Per-axis calibration
25. Dark mode GUI (Catppuccin Mocha)

### Selected from Tier 3
26. Response curves - cubic bezier (with interactive control point editor)

### Added after second codebase scan
27. Button release callback system (engine infrastructure required for temporary modes, auto-release)
28. Response curve symmetry mode (auto-mirror control points across center)
29. Mode cycle detection with resolution strategies (Oldest/Newest)
30. Axis refresh on mode change (re-emit current axis values so new curves apply immediately)
31. CLI arguments: `--profile <path>`, `--enable`, `--start-minimized`
32. Calibration as 5-value band: [min, center_low, center_high, max, enabled]

---

## 6. Event Processing Pipeline

```
SDL3 Poller (~1000Hz)
    │
    ▼
InputEvent { device, input_id, value, timestamp }
    │
    ▼
Mode Router
    │  Lookup: (device, input_id, current_mode)
    │  Inheritance: walks up mode tree if no mapping found
    │
    ▼
Action Pipeline Executor
    │  Maintains PipelineContext { current_value }
    │  Walks actions[] in order:
    │    - Processing actions: transform current_value
    │    - Output actions: send current_value to sink
    │    - Conditional: evaluate, recurse into sub-pipeline
    │
    ├──▶ VJoyOutput.set_axis/button/hat()
    ├──▶ KeyboardOutput.send_key()
    └──▶ InputCache.update()
```

---

## 7. GUI Design

### Framework
- egui 0.33+ via eframe
- Catppuccin Mocha color scheme
- Lexend font (display/body) + JetBrains Mono (data/values)
- `egui_plot` for response curve editor
- `tray-icon` for system tray integration

### Layout
- **Title bar**: Logo, profile dropdown, activate/status, settings
- **Left panel** (280px): Device tree with live axis bars/button indicators + Mode tree
- **Center panel** (flex): Context-dependent -- mapping editor, mode overview, or input monitor
- **Status bar**: Engine status, device count, vJoy status, current mode badge

### Key Widgets
- **Axis bars**: 6px height, Teal (positive) / Peach (negative), center tick, value in mono
- **Button grid**: 12px circles, Teal filled when pressed, grid layout
- **Response curve editor**: Interactive canvas with draggable bezier control points, live input/output lines, deadzone shading, grid overlay
- **Deadzone editor**: Horizontal bar with draggable boundary handles, live input marker
- **Action cards**: Stacked cards connected by vertical flow lines, drag handles for reorder, left accent bar
- **Condition editor**: Nested card with condition list, if-true/if-false sub-pipelines

### Color System
- Base backgrounds: Crust #11111b, Mantle #181825, Base #1e1e2e
- Widget backgrounds: Surface0 #313244
- Borders: Surface1 #45475a
- Primary accent: Teal #94e2d5 (active, selected, positive axis)
- Secondary accent: Peach #fab387 (control points, negative axis, warnings)
- Info: Blue #89b4fa
- Success: Green #a6e3a1 (connected, running, live input)
- Error: Red #f38ba8 (disconnected, deadzone shading)
- Special: Mauve #cba6f7 (modes, conditions)

---

## 8. JoystickGremlin Comparison

What InputForge does differently:
- **Rust** instead of Python -- faster, safer, single binary
- **Pipeline actions** instead of action trees -- more intuitive composition
- **TOML** instead of XML -- human-editable profiles
- **egui** instead of Qt/QML -- lighter, no runtime dependency
- **SDL3** instead of custom DILL DLL -- standard, maintained, cross-platform ready
- **Headless engine** -- GUI is optional, not required for operation
- **Clean separation** -- engine is a library, GUI is a separate crate

What InputForge defers to later versions:
- User scripting (JG has Python scripting)
- Macro system (JG has full macro sequences)
- Force feedback
- Logical device abstraction
- Action library (reusable actions by UUID)
- Auto-profile loading by process

---

## 9. Deferred Features (Post-v1 Roadmap)

The following features were identified during analysis but intentionally excluded from v1 to keep scope manageable. They are organized by priority for future versions.

### v2 Candidates (High Value)

| # | Feature | Description | Complexity | JG Equivalent |
|---|---------|-------------|------------|---------------|
| 3.4 | Tempo (short/long press) | Different actions for tap vs hold. Configurable threshold timer. | Medium | `tempo` plugin |
| 3.5 | Smart toggle | Short press = toggle on/off, long press = momentary. FSM-based. | Medium | `smart_toggle` plugin |
| 3.6 | Macro system (basic) | Sequences of key/button/pause actions. Repeat modes: single, count, toggle, hold. | High | `macro` plugin (7 action types) |
| 3.7 | Hat-to-buttons conversion | Convert each hat direction into individual virtual buttons. 4-way and 8-way. | Low | `hat_buttons` plugin |
| 3.8 | Axis-to-button (virtual axis button) | Trigger button action when axis crosses a threshold. Configurable range. | Low | `VirtualAxisButton` in profile |
| 3.9 | Conditions (axis range, hat direction) | Extend condition system: axis within range, hat in specific direction(s). | Low | `RangeComparator`, `DirectionComparator` |
| 3.13 | Auto-profile loading | Monitor active Windows process, auto-switch profiles based on regex matching. | Medium | `process_monitor.py` |
| 3.14 | Relative axis mode | Incremental axis output (background thread adds delta each tick). For axis-to-mouse or slow axis control. | Medium | `map_to_vjoy` relative mode |
| 3.15 | Initial axis values | Set vJoy axes to specific positions on profile activation. Per-device, per-axis. | Low | `vjoy_initial_values` in settings |
| NEW | Auto-mapper | One-click automatic mapping of all physical inputs to vJoy outputs. Scans devices, finds available vJoy slots, creates 1:1 mappings. Great for first-time setup. | Medium | `auto_mapper.py` |
| NEW | Device swap tool | When hardware changes, remap all profile references from old device GUID to new GUID atomically. Swaps inputs, actions, and script references. | Medium | `swap_devices.py` |
| NEW | Device database (input naming) | JSON database mapping vendor_id/product_id to human-readable axis/button names. "Axis 0" becomes "Stick X". Community-maintainable. | Low | `DeviceDatabase` in input_cache.py |
| NEW | Exclusive macros | Prevent concurrent execution of the same macro instance. Flag-based locking. | Low | `is_exclusive` in macro.py |
| NEW | Macro auto-pause insertion | Automatically insert configurable default delays between macro actions during preprocessing. | Low | `MacroManager` preprocessing |

### v3 Candidates (Advanced Features)

| # | Feature | Description | Complexity | JG Equivalent |
|---|---------|-------------|------------|---------------|
| 3.2 | Axis splitting | Split one axis into two at configurable point. Each half linearly interpolated to full range. | Low | `split_axis` plugin |
| 3.3 | Dual axis circular deadzone | Circular deadzone for paired axes (joystick X/Y). Trigonometric calculation. | Medium | `dual_axis_deadzone` plugin |
| 3.10 | Button-to-mouse mapping | Map joystick buttons to mouse buttons (left/right/middle/fwd/back). | Low | `map_to_mouse` plugin (buttons) |
| 3.11 | Axis-to-mouse motion | Control mouse cursor with axis. Fixed delta or accelerated mode with Vector2 math. | Medium | `map_to_mouse` plugin (motion) + `MouseController` |
| 3.12 | Chain/cycle action | Cycle through different action sets on each successive press. Optional timeout reset. | Medium | `chain` plugin |
| 4.4 | Keyboard/mouse input capture | Use keyboard keys and mouse as input sources (via Win32 low-level hooks). | High | `KeyboardHook`, `MouseHook` |
| 4.5 | Action library (reusable actions) | Central repository of action definitions referenced by UUID. Reuse across inputs. | Medium | `Library` in profile.py |

### Future Vision (v4+)

| # | Feature | Description | Complexity | JG Equivalent |
|---|---------|-------------|------------|---------------|
| 4.1 | User scripting (Lua or Rhai) | Custom logic via embedded scripting language. Decorators for input callbacks, periodic execution, access to vJoy/input cache. | Very High | Full Python scripting system with 10 variable types |
| 4.2 | Double tap detection | Differentiate single tap vs double tap. Exclusive mode (waits threshold) or combined mode. | Medium | `double_tap` plugin |
| 4.3 | Force feedback passthrough | Forward FFB effects from game to physical device. Requires vJoy FFB API + SDL3 haptic. | Very High | Not implemented in JG (attempted, crashes) |
| 4.6 | JoystickGremlin profile import | Parse JG's XML v14 profiles and convert to InputForge TOML. Migration path for existing users. | High | N/A |
| 4.7 | Play sound action | Audio feedback on button press. Configurable file and volume. | Low | `play_sound` plugin + `audio_player.py` |
| 4.8 | Cheatsheet generation | Export a summary of all mappings as PDF/image/text for reference. | Medium | `cheatsheet.py` |
| 4.9 | Multi-profile active | Multiple profiles running simultaneously for different device groups. | High | Not in JG (single profile only) |
| 4.10 | Logical device abstraction | Merge multiple physical devices into one logical device. Scripts/actions reference by logical name. | High | `logical_device.py` |

### Features Intentionally NOT Planned

These JoystickGremlin features are excluded by design:

| Feature | Reason |
|---------|--------|
| Text-to-speech | Niche, users can use OS TTS or external tools |
| Load profile action (runtime) | Adds complexity; prefer system tray profile switching |
| Description/annotation action | No-op action, adds UI clutter without runtime value |
| Reference/library indirection | The pipeline model with TOML makes copy-paste sufficient; library pattern adds complexity for marginal benefit |
| vJoy-as-input | Circular dependency risk; if needed, use a second InputForge instance |
| Pause/resume action | System tray provides this; no need for an in-pipeline action |
| Repeater (legacy) | JG's repeater module is legacy PyQt5 code with timeout-based re-emission. Not needed with our clean event loop. |
| FileWatcher | JG watches files for external changes. We use TOML with explicit save -- no need for external file watching. |
| Hint system (CSV tooltips) | JG loads hints from CSV. We'll use inline tooltip strings in the code -- no need for external hint files. |
| QML/Qt dependency | JG's entire UI is Qt Quick/QML. We use egui -- no Qt dependency at all. |

---

## 10. Codebase Scan Confidence

Two complete scans were performed against the JoystickGremlin source:
1. **First scan**: GitHub API-based analysis of repository structure, all 22 action plugins, core modules, dependencies, and 50+ GitHub issues.
2. **Second scan**: Local filesystem deep read of every `.py` file in `gremlin/` (21 core modules, ~15,000 lines), all action plugins (25 modules), types, config, profiles, tests, QML, and entry point.

**Confidence level**: Every public class, function, configuration option, and feature in JoystickGremlin has been documented and either included in InputForge's roadmap or explicitly excluded with rationale.

---

## 11. Future Considerations

### Linux Support
- `InputSource`: SDL3 works identically on Linux
- `OutputSink`: Replace vJoy with `uinput` (via `evdev` crate)
- `DeviceHider`: Replace HidHide with `EVIOCGRAB` (exclusive grab)
- Gate platform-specific code with `#[cfg(target_os)]` and Cargo features

### Process Separation
- Engine and GUI communicate via `Arc<RwLock<AppState>>` + channels
- This boundary is designed to be replaceable with IPC (named pipes, Unix sockets)
- Future: engine as Windows service, GUI as separate process

### Performance Budget
- Engine thread: <1ms per event processing cycle
- GUI: 60 FPS target (egui handles this naturally)
- SDL3 polling: configurable rate, default 1000Hz
- vJoy updates: batched per frame, not per-event
