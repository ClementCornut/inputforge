# Map To Mouse Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `MapToMouse` output actions, `Hold`/`Pulse` output behavior, runtime edge handling, mouse output sinks, and Dioxus editor support.

**Architecture:** Keep the pipeline side-effect free: it emits current output intent with owner metadata, destination, behavior, and active state. The engine owns edge detection, destination ref-counting, cleanup, and sink dispatch through a small runtime output-state module shared by keyboard and mouse outputs.

**Tech Stack:** Rust workspace, `serde`, Windows `SendInput` through the existing `windows` crate, Dioxus 0.7 GUI, existing mock sinks and engine tests.

---

## File Structure

- Modify `crates/inputforge-core/src/action/mod.rs`: add `OutputBehavior`, `MouseTarget`, behavior-aware `MapToKeyboard`, `MapToMouse`, serde normalization, labels, and model tests.
- Modify `crates/inputforge-core/src/pipeline/mod.rs`: add owner path metadata and output-intent variants for keyboard and mouse while keeping `execute_pipeline` pure.
- Create `crates/inputforge-core/src/engine/output_state.rs`: own edge detection, owner tracking, destination ref-counting, cleanup, and retryable release state.
- Modify `crates/inputforge-core/src/engine/mod.rs`: store `MouseSink` and `OutputRuntimeState`.
- Modify `crates/inputforge-core/src/engine/output_handler.rs`: route keyboard/mouse intents through `OutputRuntimeState`; keep vJoy and mode handling unchanged.
- Modify `crates/inputforge-core/src/engine/run.rs`: build output owner scopes per mapping, reconcile missing owners after each mapping evaluation, and release held outputs on mapping/profile/mode/status/shutdown/disconnect transitions.
- Modify `crates/inputforge-core/src/output/traits.rs`: extend `KeyboardSink` with down/up/pulse methods and add `MouseSink`.
- Modify `crates/inputforge-core/src/output/keyboard.rs`: expose down/up through the trait while preserving pulse behavior.
- Create `crates/inputforge-core/src/output/mouse.rs`: Windows mouse `SendInput` conversion and sink implementation.
- Modify `crates/inputforge-core/src/output/mock.rs`: record keyboard down/up/pulse and mouse button/wheel calls.
- Modify `crates/inputforge-core/src/output/mod.rs`: export the mouse module.
- Modify `crates/inputforge-app/src/main.rs`: construct and pass `MouseOutput`.
- Modify `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/add_palette.rs`: add default `Map to mouse`.
- Modify `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs`: dispatch the new body and pass keyboard behavior.
- Modify `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/map_to_keyboard.rs`: add behavior selector.
- Create `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/map_to_mouse.rs`: target and behavior editor.
- Modify `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage.rs`: titles and summaries for mouse and behavior-aware keyboard output.
- Modify `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/analyzer.rs`: add mouse destinations and behavior text for keyboard/mouse-button outputs.
- Modify `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/out_block.rs`: render mouse button and wheel rows.
- Modify existing test files under `crates/inputforge-core/src/engine/tests.rs`, `crates/inputforge-core/src/pipeline/mod.rs`, `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/tests.rs`, and `crates/inputforge-gui-dx/src/frame/mapping_editor/tests.rs`.

### Task 1: Core Action Model

**Files:**
- Modify: `crates/inputforge-core/src/action/mod.rs`
- Test: `crates/inputforge-core/src/action/mod.rs`

- [ ] **Step 1: Write failing serde and label tests**

Add these tests inside `#[cfg(test)] mod tests`:

```rust
#[test]
fn action_map_to_keyboard_behavior_roundtrips() {
    let action = Action::MapToKeyboard {
        key: KeyCombo {
            key: "Space".to_owned(),
            modifiers: vec![],
        },
        behavior: OutputBehavior::Pulse,
    };

    let json = serde_json::to_string(&action).unwrap();

    assert!(json.contains("\"type\":\"map_to_keyboard\""));
    assert!(json.contains("\"behavior\":\"pulse\""));
    let back: Action = serde_json::from_str(&json).unwrap();
    assert_eq!(action, back);
}

#[test]
fn old_keyboard_action_defaults_to_hold() {
    let json = r#"{"type":"map_to_keyboard","key":{"key":"A","modifiers":[]}}"#;

    let back: Action = serde_json::from_str(json).unwrap();

    assert_eq!(
        back,
        Action::MapToKeyboard {
            key: KeyCombo {
                key: "A".to_owned(),
                modifiers: vec![],
            },
            behavior: OutputBehavior::Hold,
        }
    );
}

#[test]
fn action_map_to_mouse_button_pulse_roundtrips() {
    let action = Action::MapToMouse {
        target: MouseTarget::LeftButton,
        behavior: OutputBehavior::Pulse,
    };

    let json = serde_json::to_string(&action).unwrap();

    assert!(json.contains("\"type\":\"map_to_mouse\""));
    assert!(json.contains("\"target\":\"LeftButton\""));
    assert!(json.contains("\"behavior\":\"pulse\""));
    let back: Action = serde_json::from_str(&json).unwrap();
    assert_eq!(action, back);
}

#[test]
fn action_map_to_mouse_button_hold_roundtrips() {
    let action = Action::MapToMouse {
        target: MouseTarget::RightButton,
        behavior: OutputBehavior::Hold,
    };

    let json = serde_json::to_string(&action).unwrap();

    assert!(json.contains("\"type\":\"map_to_mouse\""));
    assert!(json.contains("\"target\":\"RightButton\""));
    assert!(json.contains("\"behavior\":\"hold\""));
    let back: Action = serde_json::from_str(&json).unwrap();
    assert_eq!(action, back);
}

#[test]
fn action_map_to_mouse_wheel_up_normalizes_hold_to_pulse() {
    let json = r#"{"type":"map_to_mouse","target":"WheelUp","behavior":"hold"}"#;

    let back: Action = serde_json::from_str(json).unwrap();
    let saved = serde_json::to_string(&back).unwrap();

    assert_eq!(
        back,
        Action::MapToMouse {
            target: MouseTarget::WheelUp,
            behavior: OutputBehavior::Pulse,
        }
    );
    assert!(saved.contains("\"behavior\":\"pulse\""));
}

#[test]
fn action_map_to_mouse_wheel_down_normalizes_hold_to_pulse() {
    let json = r#"{"type":"map_to_mouse","target":"WheelDown","behavior":"hold"}"#;

    let back: Action = serde_json::from_str(json).unwrap();
    let saved = serde_json::to_string(&back).unwrap();

    assert_eq!(
        back,
        Action::MapToMouse {
            target: MouseTarget::WheelDown,
            behavior: OutputBehavior::Pulse,
        }
    );
    assert!(saved.contains("\"behavior\":\"pulse\""));
}

#[test]
fn invalid_mouse_target_fails_to_load() {
    let json = r#"{"type":"map_to_mouse","target":"Sideways","behavior":"pulse"}"#;

    let err = serde_json::from_str::<Action>(json).unwrap_err();

    assert!(err.to_string().contains("unknown variant"));
}

#[test]
fn invalid_output_behavior_fails_to_load() {
    let json = r#"{"type":"map_to_mouse","target":"LeftButton","behavior":"repeat"}"#;

    let err = serde_json::from_str::<Action>(json).unwrap_err();

    assert!(err.to_string().contains("unknown variant"));
}

#[test]
fn mouse_target_labels_are_stable() {
    assert_eq!(MouseTarget::LeftButton.label(), "Left click");
    assert_eq!(MouseTarget::RightButton.label(), "Right click");
    assert_eq!(MouseTarget::MiddleButton.label(), "Middle click");
    assert_eq!(MouseTarget::BackButton.label(), "Back button");
    assert_eq!(MouseTarget::ForwardButton.label(), "Forward button");
    assert_eq!(MouseTarget::WheelUp.label(), "Wheel up");
    assert_eq!(MouseTarget::WheelDown.label(), "Wheel down");
}
```

- [ ] **Step 2: Run the failing model tests**

Run:

```bash
cargo test -p inputforge-core action_map_to --lib
cargo test -p inputforge-core old_keyboard_action_defaults_to_hold --lib
cargo test -p inputforge-core invalid_ --lib
cargo test -p inputforge-core mouse_target_labels_are_stable --lib
```

Expected: FAIL with missing `OutputBehavior`, `MouseTarget`, `MapToMouse`, and changed `MapToKeyboard` fields.

- [ ] **Step 3: Add the model types and serde helpers**

In `crates/inputforge-core/src/action/mod.rs`, add these definitions above `pub enum Action`:

```rust
fn default_output_behavior() -> OutputBehavior {
    OutputBehavior::Hold
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputBehavior {
    Hold,
    Pulse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MouseTarget {
    LeftButton,
    RightButton,
    MiddleButton,
    BackButton,
    ForwardButton,
    WheelUp,
    WheelDown,
}

impl MouseTarget {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::LeftButton => "Left click",
            Self::RightButton => "Right click",
            Self::MiddleButton => "Middle click",
            Self::BackButton => "Back button",
            Self::ForwardButton => "Forward button",
            Self::WheelUp => "Wheel up",
            Self::WheelDown => "Wheel down",
        }
    }

    #[must_use]
    pub const fn is_wheel(self) -> bool {
        matches!(self, Self::WheelUp | Self::WheelDown)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct MapToMouseAction {
    target: MouseTarget,
    #[serde(default = "default_output_behavior")]
    behavior: OutputBehavior,
}

impl MapToMouseAction {
    fn into_parts(self) -> (MouseTarget, OutputBehavior) {
        let behavior = if self.target.is_wheel() {
            OutputBehavior::Pulse
        } else {
            self.behavior
        };
        (self.target, behavior)
    }
}
```

Replace the enum derive and variants with manual serde support:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    ResponseCurve { curve: ResponseCurve },
    Deadzone { config: DeadzoneConfig },
    Invert,
    MapToVJoy { output: OutputAddress },
    MapToKeyboard {
        key: KeyCombo,
        behavior: OutputBehavior,
    },
    MapToMouse {
        target: MouseTarget,
        behavior: OutputBehavior,
    },
    MergeAxis {
        second_input: InputAddress,
        operation: MergeOp,
    },
    ChangeMode { strategy: ModeChangeStrategy },
    Conditional {
        condition: Condition,
        if_true: Vec<Action>,
        if_false: Vec<Action>,
    },
}
```

Use an internal tagged enum for serde:

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ActionSerde {
    ResponseCurve { curve: ResponseCurve },
    Deadzone { config: DeadzoneConfig },
    Invert,
    #[serde(rename = "map_to_vjoy")]
    MapToVJoy { output: OutputAddress },
    MapToKeyboard {
        key: KeyCombo,
        #[serde(default = "default_output_behavior")]
        behavior: OutputBehavior,
    },
    #[serde(rename = "map_to_mouse")]
    MapToMouse {
        target: MouseTarget,
        #[serde(default = "default_output_behavior")]
        behavior: OutputBehavior,
    },
    MergeAxis {
        second_input: InputAddress,
        operation: MergeOp,
    },
    ChangeMode { strategy: ModeChangeStrategy },
    Conditional {
        condition: Condition,
        #[serde(default)]
        if_true: Vec<Action>,
        #[serde(default)]
        if_false: Vec<Action>,
    },
}

impl From<Action> for ActionSerde {
    fn from(action: Action) -> Self {
        match action {
            Action::ResponseCurve { curve } => Self::ResponseCurve { curve },
            Action::Deadzone { config } => Self::Deadzone { config },
            Action::Invert => Self::Invert,
            Action::MapToVJoy { output } => Self::MapToVJoy { output },
            Action::MapToKeyboard { key, behavior } => Self::MapToKeyboard { key, behavior },
            Action::MapToMouse { target, behavior } => {
                let behavior = if target.is_wheel() {
                    OutputBehavior::Pulse
                } else {
                    behavior
                };
                Self::MapToMouse { target, behavior }
            }
            Action::MergeAxis { second_input, operation } => Self::MergeAxis {
                second_input,
                operation,
            },
            Action::ChangeMode { strategy } => Self::ChangeMode { strategy },
            Action::Conditional { condition, if_true, if_false } => Self::Conditional {
                condition,
                if_true,
                if_false,
            },
        }
    }
}

impl From<ActionSerde> for Action {
    fn from(action: ActionSerde) -> Self {
        match action {
            ActionSerde::ResponseCurve { curve } => Self::ResponseCurve { curve },
            ActionSerde::Deadzone { config } => Self::Deadzone { config },
            ActionSerde::Invert => Self::Invert,
            ActionSerde::MapToVJoy { output } => Self::MapToVJoy { output },
            ActionSerde::MapToKeyboard { key, behavior } => Self::MapToKeyboard {
                key,
                behavior,
            },
            ActionSerde::MapToMouse { target, behavior } => {
                let (target, behavior) = MapToMouseAction { target, behavior }.into_parts();
                Self::MapToMouse { target, behavior }
            }
            ActionSerde::MergeAxis { second_input, operation } => Self::MergeAxis {
                second_input,
                operation,
            },
            ActionSerde::ChangeMode { strategy } => Self::ChangeMode { strategy },
            ActionSerde::Conditional { condition, if_true, if_false } => Self::Conditional {
                condition,
                if_true,
                if_false,
            },
        }
    }
}

impl Serialize for Action {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ActionSerde::from(self.clone()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Action {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        ActionSerde::deserialize(deserializer).map(Self::from)
    }
}
```

- [ ] **Step 4: Update existing `Action::MapToKeyboard` construction sites**

For every existing `Action::MapToKeyboard { key }`, add `behavior: OutputBehavior::Hold` unless a test explicitly needs `Pulse`.

Example replacement:

```rust
Action::MapToKeyboard {
    key: combo,
    behavior: OutputBehavior::Hold,
}
```

- [ ] **Step 5: Run the model tests again**

Run:

```bash
cargo test -p inputforge-core action_map_to --lib
cargo test -p inputforge-core old_keyboard_action_defaults_to_hold --lib
cargo test -p inputforge-core invalid_ --lib
cargo test -p inputforge-core mouse_target_labels_are_stable --lib
cargo check -p inputforge-core
```

Expected: PASS for action serde tests and compile check.

- [ ] **Step 6: Commit**

Run:

```bash
git add crates/inputforge-core/src/action/mod.rs
git commit -m "feat(core): add mouse action model"
```

### Task 2: Pipeline Output Intents

**Files:**
- Modify: `crates/inputforge-core/src/pipeline/mod.rs`
- Modify: `crates/inputforge-core/src/engine/dependencies.rs`
- Test: `crates/inputforge-core/src/pipeline/mod.rs`

- [ ] **Step 1: Write failing pipeline tests**

Add tests near the existing `map_to_keyboard_with_button` tests:

```rust
fn button(index: u32) -> InputAddress {
    InputAddress::Bound {
        device: DeviceId("stick-1".to_owned()),
        input: InputId::Button { index },
    }
}

fn output_owner_path(output: &PipelineOutput) -> Vec<ActionPathSegment> {
    match output {
        PipelineOutput::Keyboard { owner, .. } | PipelineOutput::Mouse { owner, .. } => {
            owner.action_path.clone()
        }
        _ => Vec::new(),
    }
}

#[test]
fn map_to_keyboard_outputs_behavior_and_owner() {
    let actions = vec![Action::MapToKeyboard {
        key: key_combo("Space"),
        behavior: OutputBehavior::Hold,
    }];
    let cache = MockCache::new();
    let mut ctx = button_ctx(&cache, true);

    execute_pipeline_with_scope(
        &actions,
        &mut ctx,
        OutputOwnerScope::new("profile-a", "Default", button(0)),
    );

    assert_eq!(
        ctx.outputs,
        vec![PipelineOutput::Keyboard {
            owner: OutputOwner {
                profile: "profile-a".to_owned(),
                mode: "Default".to_owned(),
                input: button(0),
                action_path: vec![ActionPathSegment::Index(0)],
                destination: OutputDestination::Keyboard(key_combo("Space")),
                behavior: OutputBehavior::Hold,
            },
            key: key_combo("Space"),
            behavior: OutputBehavior::Hold,
            active: true,
        }]
    );
}

#[test]
fn map_to_mouse_outputs_button_intent() {
    let actions = vec![Action::MapToMouse {
        target: MouseTarget::LeftButton,
        behavior: OutputBehavior::Pulse,
    }];
    let cache = MockCache::new();
    let mut ctx = button_ctx(&cache, true);

    execute_pipeline_with_scope(
        &actions,
        &mut ctx,
        OutputOwnerScope::new("profile-a", "Default", button(0)),
    );

    assert_eq!(
        ctx.outputs,
        vec![PipelineOutput::Mouse {
            owner: OutputOwner {
                profile: "profile-a".to_owned(),
                mode: "Default".to_owned(),
                input: button(0),
                action_path: vec![ActionPathSegment::Index(0)],
                destination: OutputDestination::Mouse(MouseTarget::LeftButton),
                behavior: OutputBehavior::Pulse,
            },
            target: MouseTarget::LeftButton,
            behavior: OutputBehavior::Pulse,
            active: true,
        }]
    );
}

#[test]
fn map_to_mouse_wheel_is_effective_pulse() {
    let actions = vec![Action::MapToMouse {
        target: MouseTarget::WheelDown,
        behavior: OutputBehavior::Hold,
    }];
    let cache = MockCache::new();
    let mut ctx = button_ctx(&cache, true);

    execute_pipeline_with_scope(
        &actions,
        &mut ctx,
        OutputOwnerScope::new("profile-a", "Default", button(0)),
    );

    assert!(matches!(
        &ctx.outputs[0],
        PipelineOutput::Mouse {
            behavior: OutputBehavior::Pulse,
            target: MouseTarget::WheelDown,
            active: true,
            ..
        }
    ));
}

#[test]
fn conditional_action_paths_distinguish_branches() {
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed { input: button(1) },
        if_true: vec![Action::MapToMouse {
            target: MouseTarget::LeftButton,
            behavior: OutputBehavior::Hold,
        }],
        if_false: vec![Action::MapToMouse {
            target: MouseTarget::LeftButton,
            behavior: OutputBehavior::Hold,
        }],
    }];
    let mut cache = MockCache::new();
    cache.buttons.insert(button(1), true);
    let mut ctx = button_ctx(&cache, true);

    execute_pipeline_with_scope(
        &actions,
        &mut ctx,
        OutputOwnerScope::new("profile-a", "Default", button(0)),
    );

    assert_eq!(
        output_owner_path(&ctx.outputs[0]),
        vec![
            ActionPathSegment::Index(0),
            ActionPathSegment::IfTrue,
            ActionPathSegment::Index(0),
        ]
    );
}
```

- [ ] **Step 2: Run the failing pipeline tests**

Run:

```bash
cargo test -p inputforge-core map_to_ --lib
cargo test -p inputforge-core conditional_action_paths_distinguish_branches --lib
```

Expected: FAIL with missing pipeline owner and mouse output types.

- [ ] **Step 3: Add pipeline owner and output destination types**

In `crates/inputforge-core/src/pipeline/mod.rs`, add imports for `MouseTarget` and `OutputBehavior`, then define:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionPathSegment {
    Index(usize),
    IfTrue,
    IfFalse,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OutputDestination {
    Keyboard(KeyCombo),
    Mouse(MouseTarget),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OutputOwner {
    pub profile: String,
    pub mode: String,
    pub input: InputAddress,
    pub action_path: Vec<ActionPathSegment>,
    pub destination: OutputDestination,
    pub behavior: OutputBehavior,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputOwnerScope {
    profile: String,
    mode: String,
    input: InputAddress,
}

impl OutputOwnerScope {
    #[must_use]
    pub fn new(profile: impl Into<String>, mode: impl Into<String>, input: InputAddress) -> Self {
        Self {
            profile: profile.into(),
            mode: mode.into(),
            input,
        }
    }

    fn owner(
        &self,
        action_path: &[ActionPathSegment],
        destination: OutputDestination,
        behavior: OutputBehavior,
    ) -> OutputOwner {
        OutputOwner {
            profile: self.profile.clone(),
            mode: self.mode.clone(),
            input: self.input.clone(),
            action_path: action_path.to_vec(),
            destination,
            behavior,
        }
    }
}
```

Replace `PipelineOutput::SendKey` with:

```rust
Keyboard {
    owner: OutputOwner,
    key: KeyCombo,
    behavior: OutputBehavior,
    active: bool,
},
Mouse {
    owner: OutputOwner,
    target: MouseTarget,
    behavior: OutputBehavior,
    active: bool,
},
```

- [ ] **Step 4: Add scoped pipeline execution**

Keep the current public API for GUI projections:

```rust
pub fn execute_pipeline(actions: &[Action], ctx: &mut PipelineContext<'_>) {
    execute_pipeline_with_scope(
        actions,
        ctx,
        OutputOwnerScope::new("anonymous", "anonymous", InputAddress::Unbound),
    );
}

pub fn execute_pipeline_with_scope(
    actions: &[Action],
    ctx: &mut PipelineContext<'_>,
    scope: OutputOwnerScope,
) {
    let mut path = Vec::new();
    execute_pipeline_inner(actions, ctx, &scope, &mut path);
}
```

Move the existing loop into `execute_pipeline_inner`. For each enumerated action, push `ActionPathSegment::Index(i)` before handling output variants, then pop it after the action. For conditional branches, push `IfTrue` or `IfFalse` around recursive calls.

For keyboard and mouse variants, emit:

```rust
Action::MapToKeyboard { key, behavior } => match &ctx.input_value {
    InputValue::Hat { .. } => {
        tracing::debug!("hat-to-keyboard mapping not yet implemented");
    }
    _ => {
        let active = button_pressed_from_value(ctx.current_value);
        ctx.outputs.push(PipelineOutput::Keyboard {
            owner: scope.owner(
                path,
                OutputDestination::Keyboard(key.clone()),
                *behavior,
            ),
            key: key.clone(),
            behavior: *behavior,
            active,
        });
    }
},
Action::MapToMouse { target, behavior } => match &ctx.input_value {
    InputValue::Hat { .. } => {
        tracing::debug!("hat-to-mouse mapping not yet implemented");
    }
    _ => {
        let active = button_pressed_from_value(ctx.current_value);
        let behavior = if target.is_wheel() {
            OutputBehavior::Pulse
        } else {
            *behavior
        };
        ctx.outputs.push(PipelineOutput::Mouse {
            owner: scope.owner(path, OutputDestination::Mouse(*target), behavior),
            target: *target,
            behavior,
            active,
        });
    }
},
```

- [ ] **Step 5: Update pipeline consumers for renamed output variants**

In `record_outputs_to_cache` and `refresh_axes_for_mode_change`, replace `PipelineOutput::SendKey { .. }` matches with:

```rust
PipelineOutput::Keyboard { .. }
| PipelineOutput::Mouse { .. }
| PipelineOutput::ChangeMode { .. } => {}
```

In `crates/inputforge-core/src/engine/dependencies.rs`, add `Action::MapToMouse { .. }` to the output action no-op arm.

- [ ] **Step 6: Run pipeline tests**

Run:

```bash
cargo test -p inputforge-core map_to_ --lib
cargo test -p inputforge-core conditional_action_paths_distinguish_branches --lib
cargo check -p inputforge-core
```

Expected: PASS for pipeline tests and compile check.

- [ ] **Step 7: Commit**

Run:

```bash
git add crates/inputforge-core/src/pipeline/mod.rs crates/inputforge-core/src/engine/dependencies.rs
git commit -m "feat(core): emit behavior-aware output intents"
```

### Task 3: Sink Traits And Mocks

**Files:**
- Modify: `crates/inputforge-core/src/output/traits.rs`
- Modify: `crates/inputforge-core/src/output/keyboard.rs`
- Modify: `crates/inputforge-core/src/output/mock.rs`
- Modify: `crates/inputforge-core/src/output/mod.rs`
- Test: `crates/inputforge-core/src/output/mock.rs`

- [ ] **Step 1: Write failing mock sink tests**

Add tests to `crates/inputforge-core/src/output/mock.rs`:

```rust
#[test]
fn mock_keyboard_records_down_up_and_pulse() {
    let mut mock = MockKeyboardSink::new();
    let combo = KeyCombo {
        key: "A".to_owned(),
        modifiers: vec![],
    };

    mock.key_down(&combo).unwrap();
    mock.key_up(&combo).unwrap();
    mock.pulse_key(&combo).unwrap();

    assert_eq!(
        mock.calls(),
        &[
            KeyboardCall::KeyDown(combo.clone()),
            KeyboardCall::KeyUp(combo.clone()),
            KeyboardCall::PulseKey(combo),
        ]
    );
}

#[test]
fn mock_mouse_records_button_and_wheel_calls() {
    let mut mock = MockMouseSink::new();

    mock.button_down(MouseTarget::LeftButton).unwrap();
    mock.button_up(MouseTarget::LeftButton).unwrap();
    mock.pulse_button(MouseTarget::RightButton).unwrap();
    mock.wheel(MouseTarget::WheelUp).unwrap();

    assert_eq!(
        mock.calls(),
        &[
            MouseCall::ButtonDown(MouseTarget::LeftButton),
            MouseCall::ButtonUp(MouseTarget::LeftButton),
            MouseCall::PulseButton(MouseTarget::RightButton),
            MouseCall::Wheel(MouseTarget::WheelUp),
        ]
    );
}
```

- [ ] **Step 2: Run failing mock tests**

Run: `cargo test -p inputforge-core mock_ --lib`

Expected: FAIL with missing trait methods and mock mouse sink.

- [ ] **Step 3: Update sink traits**

In `crates/inputforge-core/src/output/traits.rs`, import `MouseTarget` and replace `KeyboardSink` with:

```rust
pub trait KeyboardSink: Send {
    fn key_down(&mut self, combo: &KeyCombo) -> Result<()>;
    fn key_up(&mut self, combo: &KeyCombo) -> Result<()>;

    fn pulse_key(&mut self, combo: &KeyCombo) -> Result<()> {
        self.key_down(combo)?;
        self.key_up(combo)
    }
}

pub trait MouseSink: Send {
    fn button_down(&mut self, target: MouseTarget) -> Result<()>;
    fn button_up(&mut self, target: MouseTarget) -> Result<()>;

    fn pulse_button(&mut self, target: MouseTarget) -> Result<()> {
        self.button_down(target)?;
        self.button_up(target)
    }

    fn wheel(&mut self, target: MouseTarget) -> Result<()>;
}
```

- [ ] **Step 4: Update `KeyboardOutput` trait impl**

In `crates/inputforge-core/src/output/keyboard.rs`, replace the trait impl with:

```rust
impl KeyboardSink for KeyboardOutput {
    fn key_down(&mut self, combo: &KeyCombo) -> Result<()> {
        Self::send_key(&*self, combo, true)
    }

    fn key_up(&mut self, combo: &KeyCombo) -> Result<()> {
        Self::send_key(&*self, combo, false)
    }
}
```

- [ ] **Step 5: Update mocks**

In `crates/inputforge-core/src/output/mock.rs`, change `KeyboardCall` and add mouse types:

```rust
pub enum KeyboardCall {
    KeyDown(KeyCombo),
    KeyUp(KeyCombo),
    PulseKey(KeyCombo),
}

impl KeyboardSink for MockKeyboardSink {
    fn key_down(&mut self, combo: &KeyCombo) -> Result<()> {
        self.calls.push(KeyboardCall::KeyDown(combo.clone()));
        Ok(())
    }

    fn key_up(&mut self, combo: &KeyCombo) -> Result<()> {
        self.calls.push(KeyboardCall::KeyUp(combo.clone()));
        Ok(())
    }

    fn pulse_key(&mut self, combo: &KeyCombo) -> Result<()> {
        self.calls.push(KeyboardCall::PulseKey(combo.clone()));
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MouseCall {
    ButtonDown(MouseTarget),
    ButtonUp(MouseTarget),
    PulseButton(MouseTarget),
    Wheel(MouseTarget),
}

#[derive(Debug, Default)]
pub struct MockMouseSink {
    calls: Vec<MouseCall>,
}

impl MockMouseSink {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn calls(&self) -> &[MouseCall] {
        &self.calls
    }

    pub fn clear(&mut self) {
        self.calls.clear();
    }
}

impl MouseSink for MockMouseSink {
    fn button_down(&mut self, target: MouseTarget) -> Result<()> {
        self.calls.push(MouseCall::ButtonDown(target));
        Ok(())
    }

    fn button_up(&mut self, target: MouseTarget) -> Result<()> {
        self.calls.push(MouseCall::ButtonUp(target));
        Ok(())
    }

    fn pulse_button(&mut self, target: MouseTarget) -> Result<()> {
        self.calls.push(MouseCall::PulseButton(target));
        Ok(())
    }

    fn wheel(&mut self, target: MouseTarget) -> Result<()> {
        self.calls.push(MouseCall::Wheel(target));
        Ok(())
    }
}
```

- [ ] **Step 6: Run mock tests**

Run:

```bash
cargo test -p inputforge-core mock_ --lib
cargo check -p inputforge-core
```

Expected: PASS for mock tests and compile check.

- [ ] **Step 7: Commit**

Run:

```bash
git add crates/inputforge-core/src/output/traits.rs crates/inputforge-core/src/output/keyboard.rs crates/inputforge-core/src/output/mock.rs crates/inputforge-core/src/output/mod.rs
git commit -m "feat(output): add behavior-aware sink methods"
```

### Task 4: Windows Mouse Output

**Files:**
- Create: `crates/inputforge-core/src/output/mouse.rs`
- Modify: `crates/inputforge-core/src/output/mod.rs`
- Test: `crates/inputforge-core/src/output/mouse.rs`

- [ ] **Step 1: Write failing Windows conversion tests**

Create `crates/inputforge-core/src/output/mouse.rs` with these tests at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_WHEEL, MOUSEEVENTF_XDOWN,
        MOUSEEVENTF_XUP, XBUTTON1, WHEEL_DELTA,
    };

    #[test]
    fn left_button_maps_to_down_and_up_flags() {
        assert_eq!(button_flags(MouseTarget::LeftButton, true).unwrap().0, MOUSEEVENTF_LEFTDOWN);
        assert_eq!(button_flags(MouseTarget::LeftButton, false).unwrap().0, MOUSEEVENTF_LEFTUP);
    }

    #[test]
    fn back_button_sets_xbutton_data() {
        let down = button_flags(MouseTarget::BackButton, true).unwrap();
        let up = button_flags(MouseTarget::BackButton, false).unwrap();

        assert_eq!(down.0, MOUSEEVENTF_XDOWN);
        assert_eq!(down.1, XBUTTON1.0);
        assert_eq!(up.0, MOUSEEVENTF_XUP);
        assert_eq!(up.1, XBUTTON1.0);
    }

    #[test]
    fn wheel_targets_map_to_standard_notches() {
        assert_eq!(wheel_data(MouseTarget::WheelUp).unwrap(), WHEEL_DELTA);
        assert_eq!(wheel_data(MouseTarget::WheelDown).unwrap(), -WHEEL_DELTA);
        assert!(wheel_data(MouseTarget::LeftButton).is_none());
    }

    #[test]
    #[expect(unsafe_code, reason = "accessing INPUT union field in test")]
    fn wheel_input_uses_mouse_wheel_flag() {
        let input = make_mouse_input(MOUSEEVENTF_WHEEL, WHEEL_DELTA);

        assert_eq!(input.r#type, INPUT_MOUSE);
        assert_eq!(unsafe { input.Anonymous.mi.dwFlags }, MOUSEEVENTF_WHEEL);
        assert_eq!(unsafe { input.Anonymous.mi.mouseData }, WHEEL_DELTA as u32);
    }
}
```

- [ ] **Step 2: Run failing conversion tests**

Run:

```bash
cargo test -p inputforge-core button_ --lib
cargo test -p inputforge-core wheel_ --lib
```

Expected: FAIL until the module is implemented and exported.

- [ ] **Step 3: Implement conversion and sink**

Add:

```rust
use windows::Win32::Foundation::GetLastError;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_MOUSE, MOUSEINPUT, MOUSE_EVENT_FLAGS, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP,
    MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEEVENTF_XDOWN,
    MOUSEEVENTF_XUP, SendInput, WHEEL_DELTA, XBUTTON1, XBUTTON2,
};

use crate::action::MouseTarget;
use crate::error::{EngineError, Result};

use super::traits::MouseSink;

#[derive(Debug, Default)]
pub struct MouseOutput;

impl MouseOutput {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl MouseSink for MouseOutput {
    fn button_down(&mut self, target: MouseTarget) -> Result<()> {
        let (flags, data) = button_flags(target, true).ok_or_else(|| {
            EngineError::InvalidConfig {
                reason: format!("mouse wheel target cannot be held: {target:?}"),
            }
        })?;
        send_mouse_input(make_mouse_input(flags, data))
    }

    fn button_up(&mut self, target: MouseTarget) -> Result<()> {
        let (flags, data) = button_flags(target, false).ok_or_else(|| {
            EngineError::InvalidConfig {
                reason: format!("mouse wheel target cannot be released: {target:?}"),
            }
        })?;
        send_mouse_input(make_mouse_input(flags, data))
    }

    fn wheel(&mut self, target: MouseTarget) -> Result<()> {
        let data = wheel_data(target).ok_or_else(|| EngineError::InvalidConfig {
            reason: format!("mouse button target cannot scroll: {target:?}"),
        })?;
        send_mouse_input(make_mouse_input(MOUSEEVENTF_WHEEL, data))
    }
}

fn button_flags(target: MouseTarget, down: bool) -> Option<(MOUSE_EVENT_FLAGS, u32)> {
    match (target, down) {
        (MouseTarget::LeftButton, true) => Some((MOUSEEVENTF_LEFTDOWN, 0)),
        (MouseTarget::LeftButton, false) => Some((MOUSEEVENTF_LEFTUP, 0)),
        (MouseTarget::RightButton, true) => Some((MOUSEEVENTF_RIGHTDOWN, 0)),
        (MouseTarget::RightButton, false) => Some((MOUSEEVENTF_RIGHTUP, 0)),
        (MouseTarget::MiddleButton, true) => Some((MOUSEEVENTF_MIDDLEDOWN, 0)),
        (MouseTarget::MiddleButton, false) => Some((MOUSEEVENTF_MIDDLEUP, 0)),
        (MouseTarget::BackButton, true) => Some((MOUSEEVENTF_XDOWN, XBUTTON1.0)),
        (MouseTarget::BackButton, false) => Some((MOUSEEVENTF_XUP, XBUTTON1.0)),
        (MouseTarget::ForwardButton, true) => Some((MOUSEEVENTF_XDOWN, XBUTTON2.0)),
        (MouseTarget::ForwardButton, false) => Some((MOUSEEVENTF_XUP, XBUTTON2.0)),
        (MouseTarget::WheelUp | MouseTarget::WheelDown, _) => None,
    }
}

fn wheel_data(target: MouseTarget) -> Option<i32> {
    match target {
        MouseTarget::WheelUp => Some(WHEEL_DELTA),
        MouseTarget::WheelDown => Some(-WHEEL_DELTA),
        _ => None,
    }
}

fn make_mouse_input(flags: MOUSE_EVENT_FLAGS, mouse_data: impl Into<i64>) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: mouse_data.into() as u32,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

#[expect(unsafe_code, reason = "SendInput and GetLastError are Win32 FFI calls")]
fn send_mouse_input(input: INPUT) -> Result<()> {
    let sent = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    if sent == 1 {
        return Ok(());
    }

    let last_error = unsafe { GetLastError() };
    Err(EngineError::OutputFailed {
        reason: format!("SendInput mouse event failed: sent {sent}/1, GetLastError={last_error:?}"),
    })
}
```

- [ ] **Step 4: Export the mouse module**

In `crates/inputforge-core/src/output/mod.rs`, add:

```rust
pub mod mouse;
```

- [ ] **Step 5: Run conversion tests**

Run:

```bash
cargo test -p inputforge-core button_ --lib
cargo test -p inputforge-core wheel_ --lib
cargo check -p inputforge-core
```

Expected: PASS for conversion tests and compile check.

- [ ] **Step 6: Commit**

Run:

```bash
git add crates/inputforge-core/src/output/mouse.rs crates/inputforge-core/src/output/mod.rs
git commit -m "feat(output): add windows mouse sink"
```

### Task 5: Runtime Output State

**Files:**
- Create: `crates/inputforge-core/src/engine/output_state.rs`
- Modify: `crates/inputforge-core/src/engine/mod.rs`
- Test: `crates/inputforge-core/src/engine/output_state.rs`

- [ ] **Step 1: Write failing runtime-state tests**

Create `crates/inputforge-core/src/engine/output_state.rs` with tests for the pure reconciler:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{MouseTarget, OutputBehavior};
    use crate::pipeline::{ActionPathSegment, OutputDestination, OutputOwner};
    use crate::types::{InputAddress, KeyCombo};

    fn combo() -> KeyCombo {
        KeyCombo {
            key: "A".to_owned(),
            modifiers: vec![],
        }
    }

    fn owner(destination: OutputDestination, index: usize, behavior: OutputBehavior) -> OutputOwner {
        OutputOwner {
            profile: "profile-a".to_owned(),
            mode: "Default".to_owned(),
            input: InputAddress::Unbound,
            action_path: vec![ActionPathSegment::Index(index)],
            destination,
            behavior,
        }
    }

    fn events(actions: Vec<OutputAction>) -> Vec<OutputEvent> {
        actions.into_iter().map(OutputAction::into_event).collect()
    }

    #[test]
    fn hold_sends_down_once_and_up_once() {
        let mut state = OutputRuntimeState::default();
        let key = combo();
        let owner = owner(OutputDestination::Keyboard(key.clone()), 0, OutputBehavior::Hold);

        assert_eq!(
            events(state.reconcile_keyboard(owner.clone(), key.clone(), OutputBehavior::Hold, true)),
            vec![OutputEvent::KeyDown(key.clone())]
        );
        assert_eq!(
            events(state.reconcile_keyboard(owner.clone(), key.clone(), OutputBehavior::Hold, true)),
            Vec::new()
        );
        assert_eq!(
            events(state.reconcile_keyboard(owner, key.clone(), OutputBehavior::Hold, false)),
            vec![OutputEvent::KeyUp(key)]
        );
    }

    #[test]
    fn two_hold_owners_ref_count_destination() {
        let mut state = OutputRuntimeState::default();
        let key = combo();
        let owner_a = owner(OutputDestination::Keyboard(key.clone()), 0, OutputBehavior::Hold);
        let owner_b = owner(OutputDestination::Keyboard(key.clone()), 1, OutputBehavior::Hold);

        assert_eq!(
            events(state.reconcile_keyboard(owner_a.clone(), key.clone(), OutputBehavior::Hold, true)),
            vec![OutputEvent::KeyDown(key.clone())]
        );
        assert_eq!(
            events(state.reconcile_keyboard(owner_b.clone(), key.clone(), OutputBehavior::Hold, true)),
            Vec::new()
        );
        assert_eq!(
            events(state.reconcile_keyboard(owner_a, key.clone(), OutputBehavior::Hold, false)),
            Vec::new()
        );
        assert_eq!(
            events(state.reconcile_keyboard(owner_b, key.clone(), OutputBehavior::Hold, false)),
            vec![OutputEvent::KeyUp(key)]
        );
    }

    #[test]
    fn pulse_fires_once_per_rising_edge() {
        let mut state = OutputRuntimeState::default();
        let target = MouseTarget::LeftButton;
        let owner = owner(OutputDestination::Mouse(target), 0, OutputBehavior::Pulse);

        assert_eq!(
            events(state.reconcile_mouse(owner.clone(), target, OutputBehavior::Pulse, true)),
            vec![OutputEvent::MousePulse(target)]
        );
        assert_eq!(
            events(state.reconcile_mouse(owner.clone(), target, OutputBehavior::Pulse, true)),
            Vec::new()
        );
        assert_eq!(
            events(state.reconcile_mouse(owner.clone(), target, OutputBehavior::Pulse, false)),
            Vec::new()
        );
        assert_eq!(
            events(state.reconcile_mouse(owner, target, OutputBehavior::Pulse, true)),
            vec![OutputEvent::MousePulse(target)]
        );
    }

    #[test]
    fn missing_owner_releases_hold_destination() {
        let mut state = OutputRuntimeState::default();
        let target = MouseTarget::LeftButton;
        let owner = owner(OutputDestination::Mouse(target), 0, OutputBehavior::Hold);
        let scope = OwnerScopeKey::from_owner(&owner);

        state.reconcile_mouse(owner, target, OutputBehavior::Hold, true);

        assert_eq!(
            events(state.reconcile_absent_owners_for_scope(&scope, &[])),
            vec![OutputEvent::MouseUp(target)]
        );
    }
}
```

- [ ] **Step 2: Run failing runtime-state tests**

Run: `cargo test -p inputforge-core output_state --lib`

Expected: FAIL because `OutputRuntimeState` does not exist.

- [ ] **Step 3: Implement runtime state types**

Add:

```rust
use std::collections::{HashMap, HashSet};

use crate::action::{MouseTarget, OutputBehavior};
use crate::pipeline::{OutputDestination, OutputOwner};
use crate::types::KeyCombo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputEvent {
    KeyDown(KeyCombo),
    KeyUp(KeyCombo),
    KeyPulse(KeyCombo),
    MouseDown(MouseTarget),
    MouseUp(MouseTarget),
    MousePulse(MouseTarget),
    Wheel(MouseTarget),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputAction {
    Immediate(OutputEvent),
    Release {
        owner: OutputOwner,
        event: OutputEvent,
    },
}

impl OutputAction {
    #[must_use]
    pub fn into_event(self) -> OutputEvent {
        match self {
            Self::Immediate(event) | Self::Release { event, .. } => event,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OwnerScopeKey {
    profile: String,
    mode: String,
    input: crate::types::InputAddress,
}

impl OwnerScopeKey {
    #[must_use]
    pub fn from_owner(owner: &OutputOwner) -> Self {
        Self {
            profile: owner.profile.clone(),
            mode: owner.mode.clone(),
            input: owner.input.clone(),
        }
    }
}

#[derive(Debug, Default)]
pub struct OutputRuntimeState {
    active_owners: HashSet<OutputOwner>,
    hold_counts: HashMap<OutputDestination, usize>,
}
```

Implement:

```rust
impl OutputRuntimeState {
    pub fn reconcile_keyboard(
        &mut self,
        owner: OutputOwner,
        key: KeyCombo,
        behavior: OutputBehavior,
        active: bool,
    ) -> Vec<OutputAction> {
        match behavior {
            OutputBehavior::Hold => self.reconcile_hold(owner, OutputDestination::Keyboard(key.clone()), active, || {
                OutputEvent::KeyDown(key.clone())
            }, || OutputEvent::KeyUp(key.clone())),
            OutputBehavior::Pulse => self.reconcile_pulse(owner, active, || OutputEvent::KeyPulse(key)),
        }
    }

    pub fn reconcile_mouse(
        &mut self,
        owner: OutputOwner,
        target: MouseTarget,
        behavior: OutputBehavior,
        active: bool,
    ) -> Vec<OutputAction> {
        if target.is_wheel() {
            return self.reconcile_pulse(owner, active, || OutputEvent::Wheel(target));
        }

        match behavior {
            OutputBehavior::Hold => self.reconcile_hold(owner, OutputDestination::Mouse(target), active, || {
                OutputEvent::MouseDown(target)
            }, || OutputEvent::MouseUp(target)),
            OutputBehavior::Pulse => self.reconcile_pulse(owner, active, || OutputEvent::MousePulse(target)),
        }
    }

    pub fn reconcile_absent_owners_for_scope(
        &mut self,
        scope: &OwnerScopeKey,
        current: &[OutputOwner],
    ) -> Vec<OutputAction> {
        let current: HashSet<&OutputOwner> = current.iter().collect();
        let missing: Vec<OutputOwner> = self
            .active_owners
            .iter()
            .filter(|owner| OwnerScopeKey::from_owner(owner) == *scope)
            .filter(|owner| !current.contains(owner))
            .cloned()
            .collect();

        missing
            .into_iter()
            .flat_map(|owner| self.stage_release_owner(owner))
            .collect()
    }

    pub fn release_all(&mut self) -> Vec<OutputAction> {
        let owners: Vec<OutputOwner> = self.active_owners.iter().cloned().collect();
        owners
            .into_iter()
            .flat_map(|owner| self.stage_release_owner(owner))
            .collect()
    }

    pub fn commit_release(&mut self, owner: &OutputOwner) {
        if !self.active_owners.remove(owner) {
            return;
        }
        if owner.behavior == OutputBehavior::Hold {
            if let Some(count) = self.hold_counts.get_mut(&owner.destination) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    self.hold_counts.remove(&owner.destination);
                }
            }
        }
    }

    fn reconcile_hold(
        &mut self,
        owner: OutputOwner,
        destination: OutputDestination,
        active: bool,
        down: impl FnOnce() -> OutputEvent,
        up: impl FnOnce() -> OutputEvent,
    ) -> Vec<OutputAction> {
        if active {
            if !self.active_owners.insert(owner) {
                return Vec::new();
            }
            let count = self.hold_counts.entry(destination).or_insert(0);
            *count += 1;
            if *count == 1 {
                vec![OutputAction::Immediate(down())]
            } else {
                Vec::new()
            }
        } else {
            self.stage_release_owner(owner)
        }
    }

    fn reconcile_pulse(
        &mut self,
        owner: OutputOwner,
        active: bool,
        pulse: impl FnOnce() -> OutputEvent,
    ) -> Vec<OutputAction> {
        if active {
            if self.active_owners.insert(owner) {
                vec![OutputAction::Immediate(pulse())]
            } else {
                Vec::new()
            }
        } else {
            self.active_owners.remove(&owner);
            Vec::new()
        }
    }

    fn stage_release_owner(&mut self, owner: OutputOwner) -> Vec<OutputAction> {
        if !self.active_owners.contains(&owner) {
            return Vec::new();
        }

        match owner.behavior {
            OutputBehavior::Pulse => {
                self.active_owners.remove(&owner);
                Vec::new()
            }
            OutputBehavior::Hold => {
                let Some(count) = self.hold_counts.get(&owner.destination).copied() else {
                    self.active_owners.remove(&owner);
                    return Vec::new();
                };
                if count > 1 {
                    self.commit_release(&owner);
                    return Vec::new();
                }
                match owner.destination.clone() {
                    OutputDestination::Keyboard(key) => vec![OutputAction::Release {
                        owner,
                        event: OutputEvent::KeyUp(key),
                    }],
                    OutputDestination::Mouse(target) if !target.is_wheel() => {
                        vec![OutputAction::Release {
                            owner,
                            event: OutputEvent::MouseUp(target),
                        }]
                    }
                    OutputDestination::Mouse(_) => Vec::new(),
                }
            }
        }
    }
}
```

This staging/commit split is intentional. `stage_release_owner` must not remove the last active hold owner until the sink dispatch succeeds; `dispatch_output_action` in Task 6 calls `commit_release` only after successfully sending `KeyUp`/`MouseUp`.

- If releasing one of several owners for the same destination, no OS event is needed, so `stage_release_owner` may call `commit_release` immediately.
- If releasing the final owner, return `OutputAction::Release { owner, event }` and leave state untouched until `commit_release`.
- Pulse owners can be removed on inactive transitions because no release sink call is needed.

- [ ] **Step 4: Register the module**

In `crates/inputforge-core/src/engine/mod.rs`, add:

```rust
mod output_state;
```

Add fields:

```rust
mouse: Box<dyn MouseSink>,
output_state: output_state::OutputRuntimeState,
```

Import `MouseSink`, update `Engine::new` to accept `mouse: Box<dyn MouseSink>` after `keyboard`, and initialize `output_state: output_state::OutputRuntimeState::default()`.

- [ ] **Step 5: Run runtime-state tests**

Run:

```bash
cargo test -p inputforge-core output_state --lib
cargo check -p inputforge-core
```

Expected: PASS for runtime-state tests and compile check.

- [ ] **Step 6: Commit**

Run:

```bash
git add crates/inputforge-core/src/engine/output_state.rs crates/inputforge-core/src/engine/mod.rs
git commit -m "feat(engine): add output edge state"
```

### Task 6: Engine Output Dispatch And Constructor Wiring

**Files:**
- Modify: `crates/inputforge-core/src/engine/output_handler.rs`
- Modify: `crates/inputforge-core/src/engine/run.rs`
- Modify: `crates/inputforge-core/src/engine/mod.rs`
- Modify: `crates/inputforge-core/src/engine/tests.rs`
- Modify: `crates/inputforge-app/src/main.rs`

- [ ] **Step 1: Write failing engine dispatch tests**

In `crates/inputforge-core/src/engine/tests.rs`, replace the old key pulse test with:

```rust
#[test]
fn process_outputs_keyboard_hold_sends_down_and_up_edges() {
    let combo = key_combo("Space");
    let owner = keyboard_owner(combo.clone(), 0, OutputBehavior::Hold);
    let mut output_state = OutputRuntimeState::default();
    let mut sink = MockOutputSink::new();
    let mut keyboard = MockKeyboardSink::new();
    let mut mouse = MockMouseSink::new();
    let tree = simple_mode_tree();
    let mut mode_state = ModeState::new("Default".to_owned());
    let mut callbacks = CallbackRegistry::new();

    process_pipeline_outputs(
        &[PipelineOutput::Keyboard {
            owner: owner.clone(),
            key: combo.clone(),
            behavior: OutputBehavior::Hold,
            active: true,
        }],
        &mut sink,
        &mut keyboard,
        &mut mouse,
        &mut output_state,
        &mut mode_state,
        &tree,
        &mut callbacks,
        &button(0),
    )
    .unwrap();

    process_pipeline_outputs(
        &[PipelineOutput::Keyboard {
            owner,
            key: combo.clone(),
            behavior: OutputBehavior::Hold,
            active: false,
        }],
        &mut sink,
        &mut keyboard,
        &mut mouse,
        &mut output_state,
        &mut mode_state,
        &tree,
        &mut callbacks,
        &button(0),
    )
    .unwrap();

    assert_eq!(
        keyboard.calls(),
        &[KeyboardCall::KeyDown(combo.clone()), KeyboardCall::KeyUp(combo)]
    );
}

#[test]
fn process_outputs_mouse_wheel_pulses_once() {
    let owner = mouse_owner(MouseTarget::WheelUp, 0, OutputBehavior::Pulse);
    let mut output_state = OutputRuntimeState::default();
    let mut sink = MockOutputSink::new();
    let mut keyboard = MockKeyboardSink::new();
    let mut mouse = MockMouseSink::new();
    let tree = simple_mode_tree();
    let mut mode_state = ModeState::new("Default".to_owned());
    let mut callbacks = CallbackRegistry::new();

    for _ in 0..2 {
        process_pipeline_outputs(
            &[PipelineOutput::Mouse {
                owner: owner.clone(),
                target: MouseTarget::WheelUp,
                behavior: OutputBehavior::Pulse,
                active: true,
            }],
            &mut sink,
            &mut keyboard,
            &mut mouse,
            &mut output_state,
            &mut mode_state,
            &tree,
            &mut callbacks,
            &button(0),
        )
        .unwrap();
    }

    assert_eq!(mouse.calls(), &[MouseCall::Wheel(MouseTarget::WheelUp)]);
}
```

- [ ] **Step 2: Run failing engine dispatch tests**

Run:

```bash
cargo test -p inputforge-core process_outputs_ --lib
```

Expected: FAIL until `process_pipeline_outputs` accepts mouse and runtime state.

- [ ] **Step 3: Route output events through sinks**

In `crates/inputforge-core/src/engine/output_handler.rs`, import `MouseSink`, `OutputRuntimeState`, `OutputAction`, and `OutputEvent`.

Update `process_pipeline_outputs` signature to include:

```rust
mouse: &mut dyn MouseSink,
output_state: &mut OutputRuntimeState,
```

For keyboard and mouse output variants, run:

```rust
PipelineOutput::Keyboard {
    owner,
    key,
    behavior,
    active,
} => {
    for action in output_state.reconcile_keyboard(owner.clone(), key.clone(), *behavior, *active) {
        dispatch_output_action(action, output_state, keyboard, mouse)?;
    }
}
PipelineOutput::Mouse {
    owner,
    target,
    behavior,
    active,
} => {
    for action in output_state.reconcile_mouse(owner.clone(), *target, *behavior, *active) {
        dispatch_output_action(action, output_state, keyboard, mouse)?;
    }
}
```

Add:

```rust
pub(super) fn dispatch_output_action(
    action: OutputAction,
    output_state: &mut OutputRuntimeState,
    keyboard: &mut dyn KeyboardSink,
    mouse: &mut dyn MouseSink,
) -> Result<()> {
    let (event, release_owner) = match action {
        OutputAction::Immediate(event) => (event, None),
        OutputAction::Release { owner, event } => (event, Some(owner)),
    };
    match event {
        OutputEvent::KeyDown(key) => keyboard.key_down(&key)?,
        OutputEvent::KeyUp(key) => keyboard.key_up(&key)?,
        OutputEvent::KeyPulse(key) => keyboard.pulse_key(&key)?,
        OutputEvent::MouseDown(target) => mouse.button_down(target)?,
        OutputEvent::MouseUp(target) => mouse.button_up(target)?,
        OutputEvent::MousePulse(target) => mouse.pulse_button(target)?,
        OutputEvent::Wheel(target) => mouse.wheel(target)?,
    };
    if let Some(owner) = release_owner {
        output_state.commit_release(&owner);
    }
    Ok(())
}
```

- [ ] **Step 4: Update engine tick to use owner scopes**

In `crates/inputforge-core/src/engine/run.rs`, clone a stable profile identity with mappings:

```rust
let (profile_id, mappings, mode_tree) = {
    let state = self.state.read();
    match &state.active_profile {
        Some(profile) => (
            state.profile_path
                .as_ref()
                .map_or_else(|| "memory-profile".to_owned(), |path| path.display().to_string()),
            profile.mappings().to_vec(),
            profile.modes().clone(),
        ),
        None => return Ok(()),
    }
};
```

Before running a mapping, build:

```rust
let owner_scope = pipeline::OutputOwnerScope::new(
    profile_id.clone(),
    mapping.mode.clone(),
    mapping.input.clone(),
);
pipeline::execute_pipeline_with_scope(&mapping.actions, &mut ctx, owner_scope);
```

Pass `self.mouse.as_mut()` and `&mut self.output_state` into `process_pipeline_outputs`.

- [ ] **Step 5: Reconcile absent owners for each mapping scope**

After taking `outputs`, collect current owners:

```rust
let current_owners: Vec<_> = outputs
    .iter()
    .filter_map(|output| match output {
        PipelineOutput::Keyboard { owner, .. } | PipelineOutput::Mouse { owner, .. } => {
            Some(owner.clone())
        }
        _ => None,
    })
    .collect();

if let Some(first_owner) = current_owners.first() {
    let scope = OwnerScopeKey::from_owner(first_owner);
    for action in self
        .output_state
        .reconcile_absent_owners_for_scope(&scope, &current_owners)
    {
        dispatch_output_action(action, &mut self.output_state, self.keyboard.as_mut(), self.mouse.as_mut())?;
    }
}
```

If `current_owners` is empty, still reconcile with a scope built from `profile_id`, `mapping.mode`, and `mapping.input` by adding `OwnerScopeKey::new(profile_id.clone(), mapping.mode.clone(), mapping.input.clone())` in `output_state.rs`.

- [ ] **Step 6: Update every `Engine::new` call**

Add `Box::new(MockMouseSink::new())` after `Box::new(MockKeyboardSink::new())` in all engine tests.

In `crates/inputforge-app/src/main.rs`, import `MouseOutput`, construct it beside `KeyboardOutput`, and pass it into `Engine::new` after the keyboard sink:

```rust
use inputforge_core::output::mouse::MouseOutput;

let keyboard = Box::new(KeyboardOutput::new());
let mouse = Box::new(MouseOutput::new());
```

- [ ] **Step 7: Run engine dispatch tests**

Run:

```bash
cargo test -p inputforge-core process_outputs_ --lib
cargo check -p inputforge-core
cargo check -p inputforge-app
```

Expected: PASS for engine dispatch tests and compile checks.

- [ ] **Step 8: Commit**

Run:

```bash
git add crates/inputforge-core/src/engine/output_handler.rs crates/inputforge-core/src/engine/run.rs crates/inputforge-core/src/engine/mod.rs crates/inputforge-core/src/engine/tests.rs crates/inputforge-app/src/main.rs
git commit -m "feat(engine): dispatch mouse and held outputs"
```

### Task 7: Cleanup Paths

**Files:**
- Modify: `crates/inputforge-core/src/engine/output_state.rs`
- Modify: `crates/inputforge-core/src/engine/run.rs`
- Test: `crates/inputforge-core/src/engine/tests.rs`

- [ ] **Step 1: Write failing cleanup tests**

Add focused tests:

```rust
#[test]
fn held_outputs_release_on_deactivate() {
    let (mut engine, state, tx) = running_engine_with_mapping(Action::MapToMouse {
        target: MouseTarget::LeftButton,
        behavior: OutputBehavior::Hold,
    });

    press_button_and_tick(&mut engine, button(0));
    tx.send(EngineCommand::Deactivate).unwrap();
    engine.tick().unwrap();

    assert_mouse_calls_contain(&engine, &[MouseCall::ButtonDown(MouseTarget::LeftButton), MouseCall::ButtonUp(MouseTarget::LeftButton)]);
    assert_eq!(state.read().engine_status, EngineStatus::Stopped);
}

#[test]
fn held_outputs_release_on_mapping_removal() {
    let (mut engine, _state, tx) = running_engine_with_mapping(Action::MapToKeyboard {
        key: key_combo("Space"),
        behavior: OutputBehavior::Hold,
    });

    press_button_and_tick(&mut engine, button(0));
    tx.send(EngineCommand::RemoveMapping {
        input: button(0),
        mode: "Default".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    assert_keyboard_calls_contain(&engine, &[
        KeyboardCall::KeyDown(key_combo("Space")),
        KeyboardCall::KeyUp(key_combo("Space")),
    ]);
}

#[test]
fn held_outputs_release_on_bulk_mapping_replacement() {
    let (mut engine, _state, tx) = running_engine_with_mapping(Action::MapToMouse {
        target: MouseTarget::RightButton,
        behavior: OutputBehavior::Hold,
    });

    press_button_and_tick(&mut engine, button(0));
    tx.send(EngineCommand::SetMappingsBulk {
        entries: Vec::new(),
        snapshot_label: "Before clearing mappings".to_owned(),
    }).unwrap();
    engine.tick().unwrap();

    assert_mouse_calls_contain(&engine, &[
        MouseCall::ButtonDown(MouseTarget::RightButton),
        MouseCall::ButtonUp(MouseTarget::RightButton),
    ]);
}

#[test]
fn held_outputs_release_on_profile_transition() {
    let (mut engine, _state, tx) = running_engine_with_mapping(Action::MapToKeyboard {
        key: key_combo("Space"),
        behavior: OutputBehavior::Hold,
    });

    press_button_and_tick(&mut engine, button(0));
    tx.send(EngineCommand::LoadProfile(alternate_profile_path())).unwrap();
    engine.tick().unwrap();

    assert_keyboard_calls_contain(&engine, &[
        KeyboardCall::KeyDown(key_combo("Space")),
        KeyboardCall::KeyUp(key_combo("Space")),
    ]);
}

#[test]
fn held_outputs_release_on_mode_transition() {
    let (mut engine, _state, tx) = running_engine_with_mapping(Action::MapToMouse {
        target: MouseTarget::MiddleButton,
        behavior: OutputBehavior::Hold,
    });

    press_button_and_tick(&mut engine, button(0));
    tx.send(EngineCommand::SwitchMode { mode: "Alternate".to_owned() }).unwrap();
    engine.tick().unwrap();

    assert_mouse_calls_contain(&engine, &[
        MouseCall::ButtonDown(MouseTarget::MiddleButton),
        MouseCall::ButtonUp(MouseTarget::MiddleButton),
    ]);
}

#[test]
fn held_outputs_release_on_shutdown_and_disconnect() {
    let (mut engine, _state, tx) = running_engine_with_mapping(Action::MapToKeyboard {
        key: key_combo("Escape"),
        behavior: OutputBehavior::Hold,
    });

    press_button_and_tick(&mut engine, button(0));
    tx.send(EngineCommand::Shutdown).unwrap();
    engine.tick().unwrap();

    assert_keyboard_calls_contain(&engine, &[
        KeyboardCall::KeyDown(key_combo("Escape")),
        KeyboardCall::KeyUp(key_combo("Escape")),
    ]);
}

#[test]
fn failed_release_is_retryable() {
    let (mut engine, _state, tx) = running_engine_with_mapping(Action::MapToMouse {
        target: MouseTarget::LeftButton,
        behavior: OutputBehavior::Hold,
    });
    engine.mouse_mut_for_test().fail_next_button_up();

    press_button_and_tick(&mut engine, button(0));
    tx.send(EngineCommand::Deactivate).unwrap();
    assert!(engine.tick().is_err());
    engine.mouse_mut_for_test().clear_failures();
    tx.send(EngineCommand::Deactivate).unwrap();
    engine.tick().unwrap();

    assert_mouse_calls_contain(&engine, &[
        MouseCall::ButtonDown(MouseTarget::LeftButton),
        MouseCall::ButtonUp(MouseTarget::LeftButton),
    ]);
}
```

If the harness does not already expose `alternate_profile_path`, `mouse_mut_for_test`, or the call assertion helpers, add them in `engine/tests.rs` next to the existing `EngineHarness` utilities. They must use the same mock sinks created for the engine so the assertions inspect real dispatched calls.

- [ ] **Step 2: Run failing cleanup tests**

Run:

```bash
cargo test -p inputforge-core held_outputs_release --lib
cargo test -p inputforge-core failed_release_is_retryable --lib
```

Expected: FAIL because cleanup is not wired.

- [ ] **Step 3: Add a shared release helper**

In `crates/inputforge-core/src/engine/run.rs`, add:

```rust
fn release_all_held_outputs(&mut self) -> Result<()> {
    for action in self.output_state.release_all() {
        dispatch_output_action(
            action,
            &mut self.output_state,
            self.keyboard.as_mut(),
            self.mouse.as_mut(),
        )?;
    }
    Ok(())
}
```

Call it before profile reload/delete/restore, mapping edit/removal, bulk mapping replacement, mode switch/deletion, pause/deactivate, and shutdown/disconnect transitions.

For command-channel disconnect in `process_commands`, replace the disconnect arm with:

```rust
Err(mpsc::TryRecvError::Disconnected) => {
    if let Err(e) = self.release_all_held_outputs() {
        tracing::warn!(target: "engine", error = %e, "engine.output.release_on_disconnect_failed");
    }
    self.shutdown = true;
    break;
}
```

For `EngineCommand::Shutdown` and `EngineCommand::Deactivate`, run the same helper before status changes.

- [ ] **Step 4: Keep live cleanup transactional and retryable**

Do not add a `restore_failed_events` method. Release cleanup is retryable because `OutputRuntimeState::release_all` and `reconcile_absent_owners_for_scope` return staged `OutputAction::Release` values without removing the final active owner. `dispatch_output_action` commits the release only after `KeyUp` or `MouseUp` succeeds.

```rust
fn dispatch_cleanup_actions(&mut self, actions: Vec<OutputAction>) -> Result<()> {
    for action in actions {
        dispatch_output_action(
            action,
            &mut self.output_state,
            self.keyboard.as_mut(),
            self.mouse.as_mut(),
        )?;
    }
    Ok(())
}
```

Use this helper for live cleanup paths. If dispatch fails, return/log the error and leave `OutputRuntimeState` untouched for the failed final-owner release; a later cleanup can call `release_all` again and receive the same release action.

- [ ] **Step 5: Run cleanup tests**

Run:

```bash
cargo test -p inputforge-core held_outputs_release --lib
cargo test -p inputforge-core failed_release_is_retryable --lib
cargo check -p inputforge-core
```

Expected: PASS for cleanup tests and compile check.

- [ ] **Step 6: Commit**

Run:

```bash
git add crates/inputforge-core/src/engine/output_state.rs crates/inputforge-core/src/engine/run.rs crates/inputforge-core/src/engine/tests.rs
git commit -m "feat(engine): release held outputs on cleanup"
```

### Task 8: GUI Palette And Stage Bodies

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/add_palette.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/map_to_keyboard.rs`
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/map_to_mouse.rs`
- Test: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/tests.rs`

- [ ] **Step 1: Write failing GUI pipeline tests**

Add tests:

```rust
#[test]
fn add_palette_includes_map_to_mouse() {
    let html = render_add_palette();

    assert!(html.contains("Map to mouse"));
}

#[test]
fn map_to_keyboard_body_renders_behavior_selector() {
    let html = render_stage_body(Action::MapToKeyboard {
        key: key_combo("A"),
        behavior: OutputBehavior::Hold,
    });

    assert!(html.contains("Hold"));
    assert!(html.contains("Pulse"));
}

#[test]
fn map_to_mouse_body_renders_targets_and_button_behavior() {
    let html = render_stage_body(Action::MapToMouse {
        target: MouseTarget::LeftButton,
        behavior: OutputBehavior::Hold,
    });

    assert!(html.contains("Left click"));
    assert!(html.contains("Right click"));
    assert!(html.contains("Wheel up"));
    assert!(html.contains("Hold"));
    assert!(html.contains("Pulse"));
}

#[test]
fn map_to_mouse_wheel_hides_behavior_selector() {
    let html = render_stage_body(Action::MapToMouse {
        target: MouseTarget::WheelUp,
        behavior: OutputBehavior::Pulse,
    });

    assert!(html.contains("Wheel up"));
    assert!(!html.contains("Hold"));
}
```

- [ ] **Step 2: Run failing GUI tests**

Run:

```bash
cargo test -p inputforge-gui-dx add_palette_includes_map_to_mouse --lib
cargo test -p inputforge-gui-dx map_to_ --lib
```

Expected: FAIL because the GUI has no mouse stage.

- [ ] **Step 3: Add palette defaults**

In `add_palette.rs`, import `MouseTarget` and `OutputBehavior`. Change the keyboard default:

```rust
fn default_map_to_keyboard() -> Action {
    Action::MapToKeyboard {
        key: KeyCombo {
            key: "A".to_owned(),
            modifiers: vec![],
        },
        behavior: OutputBehavior::Hold,
    }
}

fn default_map_to_mouse() -> Action {
    Action::MapToMouse {
        target: MouseTarget::LeftButton,
        behavior: OutputBehavior::Hold,
    }
}
```

Add to `OUTPUT_ITEMS` between keyboard and merge:

```rust
PaletteItem {
    label: "Map to mouse",
    make: default_map_to_mouse,
},
```

- [ ] **Step 4: Dispatch `MapToMouseBody`**

In `stage_body/mod.rs`, add `mod map_to_mouse;`, update keyboard destructuring to include behavior, and add:

```rust
Action::MapToMouse { target, behavior } => rsx! {
    map_to_mouse::MapToMouseBody {
        mapping_key: mapping_key.clone(),
        stage_id: stage_id.clone(),
        target: *target,
        behavior: *behavior,
        root_actions: root_actions.clone(),
    }
},
```

- [ ] **Step 5: Add the mouse body**

Create `map_to_mouse.rs`:

```rust
use dioxus::prelude::*;

use inputforge_core::action::{Action, Mapping, MouseTarget, OutputBehavior};
use inputforge_core::engine::EngineCommand;

use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::replace_at_path;
use crate::frame::mapping_editor::undo_log::{LabelArgs, StageId, UndoKind, format_undo_label};

const MOUSE_TARGETS: &[MouseTarget] = &[
    MouseTarget::LeftButton,
    MouseTarget::RightButton,
    MouseTarget::MiddleButton,
    MouseTarget::BackButton,
    MouseTarget::ForwardButton,
    MouseTarget::WheelUp,
    MouseTarget::WheelDown,
];

#[component]
pub(crate) fn MapToMouseBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    target: MouseTarget,
    behavior: OutputBehavior,
    root_actions: Vec<Action>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();
    let cfg = ctx.config.read();
    let current_name = cfg.mapping_names.get(&mapping_key.1).cloned();
    let before_mapping = Mapping {
        input: mapping_key.1.clone(),
        mode: mapping_key.0.clone(),
        name: current_name.clone(),
        actions: root_actions.clone(),
    };
    drop(cfg);

    let cmd_tx = ctx.commands.clone();
    let undo_log = editor.undo_log;

    rsx! {
        div { class: "if-stage__body-mouse",
            div { class: "if-stage__body-field",
                label { class: "if-stage__body-label", "Target" }
                div { class: "if-stage__body-segmented",
                    for candidate in MOUSE_TARGETS {
                        let candidate = *candidate;
                        let mapping_key_target = mapping_key.clone();
                        let stage_id_target = stage_id.clone();
                        let root_actions_target = root_actions.clone();
                        let before_target = before_mapping.clone();
                        let current_name_target = current_name.clone();
                        let cmd_tx_target = cmd_tx.clone();
                        let mut undo_log_target = undo_log;
                        button {
                            class: if candidate == target { "if-stage__body-segment is-active" } else { "if-stage__body-segment" },
                            onclick: move |_| dispatch_mouse_change(
                                candidate,
                                behavior,
                                "target",
                                &mapping_key_target,
                                &stage_id_target,
                                &root_actions_target,
                                &before_target,
                                current_name_target.clone(),
                                &cmd_tx_target,
                                &mut undo_log_target,
                            ),
                            "{candidate.label()}"
                        }
                    }
                }
            }
            if !target.is_wheel() {
                div { class: "if-stage__body-field",
                    label { class: "if-stage__body-label", "Behavior" }
                    div { class: "if-stage__body-segmented",
                        {
                            let mapping_key_hold = mapping_key.clone();
                            let stage_id_hold = stage_id.clone();
                            let root_actions_hold = root_actions.clone();
                            let before_hold = before_mapping.clone();
                            let current_name_hold = current_name.clone();
                            let cmd_tx_hold = cmd_tx.clone();
                            let mut undo_log_hold = undo_log;
                            rsx! {
                        button {
                            class: if behavior == OutputBehavior::Hold { "if-stage__body-segment is-active" } else { "if-stage__body-segment" },
                            onclick: move |_| dispatch_mouse_change(
                                target,
                                OutputBehavior::Hold,
                                "behavior",
                                &mapping_key_hold,
                                &stage_id_hold,
                                &root_actions_hold,
                                &before_hold,
                                current_name_hold.clone(),
                                &cmd_tx_hold,
                                &mut undo_log_hold,
                            ),
                            "Hold"
                        }
                            }
                        }
                        {
                            let mapping_key_pulse = mapping_key.clone();
                            let stage_id_pulse = stage_id.clone();
                            let root_actions_pulse = root_actions.clone();
                            let before_pulse = before_mapping.clone();
                            let current_name_pulse = current_name.clone();
                            let cmd_tx_pulse = cmd_tx.clone();
                            let mut undo_log_pulse = undo_log;
                            rsx! {
                        button {
                            class: if behavior == OutputBehavior::Pulse { "if-stage__body-segment is-active" } else { "if-stage__body-segment" },
                            onclick: move |_| dispatch_mouse_change(
                                target,
                                OutputBehavior::Pulse,
                                "behavior",
                                &mapping_key_pulse,
                                &stage_id_pulse,
                                &root_actions_pulse,
                                &before_pulse,
                                current_name_pulse.clone(),
                                &cmd_tx_pulse,
                                &mut undo_log_pulse,
                            ),
                            "Pulse"
                        }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "Matches the existing map_to_keyboard dispatch helper shape."
)]
fn dispatch_mouse_change(
    new_target: MouseTarget,
    new_behavior: OutputBehavior,
    field: &'static str,
    mapping_key: &MappingKey,
    stage_id: &StageId,
    root_actions: &[Action],
    before: &Mapping,
    current_name: Option<String>,
    cmd_tx: &std::sync::mpsc::Sender<EngineCommand>,
    undo_log: &mut Signal<crate::frame::mapping_editor::undo_log::UndoLog>,
) {
    let effective_behavior = if new_target.is_wheel() {
        OutputBehavior::Pulse
    } else {
        new_behavior
    };
    let new_action = Action::MapToMouse {
        target: new_target,
        behavior: effective_behavior,
    };
    let Some(new_actions) = replace_at_path(root_actions, stage_id, new_action) else {
        return;
    };
    if cmd_tx
        .send(EngineCommand::SetMapping {
            input: mapping_key.1.clone(),
            mode: mapping_key.0.clone(),
            name: current_name,
            actions: new_actions,
        })
        .is_err()
    {
        tracing::warn!(target: "f9::mapping_editor", field, "mouse output change dropped: engine channel disconnected");
        return;
    }
    let label = format_undo_label(
        UndoKind::StageEdit,
        LabelArgs {
            stage_name: Some("Map to mouse"),
            field: Some(field),
            ..LabelArgs::default()
        },
    );
    undo_log.write().push_edit(
        mapping_key.clone(),
        before.clone(),
        UndoKind::StageEdit,
        label,
    );
}
```

- [ ] **Step 6: Add keyboard behavior selector**

In `map_to_keyboard.rs`, add a `behavior: OutputBehavior` prop, render the same Hold/Pulse segmented control, and change `dispatch_keyboard` to preserve or update behavior:

```rust
let new_action = Action::MapToKeyboard {
    key: new_combo,
    behavior: new_behavior,
};
```

Use `"behavior"` as the undo field label when clicking the behavior selector.

- [ ] **Step 7: Run GUI body tests**

Run:

```bash
cargo test -p inputforge-gui-dx add_palette_includes_map_to_mouse --lib
cargo test -p inputforge-gui-dx map_to_ --lib
cargo check -p inputforge-gui-dx
```

Expected: PASS for GUI body tests and compile check.

- [ ] **Step 8: Commit**

Run:

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/add_palette.rs crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/map_to_keyboard.rs crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/map_to_mouse.rs crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/tests.rs
git commit -m "feat(gui): add mouse output editor"
```

### Task 9: Stage Summaries And Live Readout

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/analyzer.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/out_block.rs`
- Test: `crates/inputforge-gui-dx/src/frame/mapping_editor/tests.rs`

- [ ] **Step 1: Write failing summary and readout tests**

Add tests:

```rust
#[test]
fn stage_summary_keyboard_includes_behavior() {
    let summary = stage_summary_for(
        &Action::MapToKeyboard {
            key: key_combo("A"),
            behavior: OutputBehavior::Hold,
        },
        &ConfigSnapshot::default(),
    );

    assert_eq!(summary, "A - Hold");
}

#[test]
fn stage_summary_mouse_button_includes_behavior() {
    let summary = stage_summary_for(
        &Action::MapToMouse {
            target: MouseTarget::LeftButton,
            behavior: OutputBehavior::Pulse,
        },
        &ConfigSnapshot::default(),
    );

    assert_eq!(summary, "Left click - Pulse");
}

#[test]
fn stage_summary_mouse_wheel_omits_behavior() {
    let summary = stage_summary_for(
        &Action::MapToMouse {
            target: MouseTarget::WheelUp,
            behavior: OutputBehavior::Pulse,
        },
        &ConfigSnapshot::default(),
    );

    assert_eq!(summary, "Wheel up");
}

#[test]
fn live_readout_exposes_mouse_destination() {
    let actions = vec![Action::MapToMouse {
        target: MouseTarget::RightButton,
        behavior: OutputBehavior::Hold,
    }];
    let model = analyze_actions(&actions, &button(0));

    assert!(matches!(
        model.outputs[0].destination,
        OutputDestination::Mouse {
            target: MouseTarget::RightButton,
            behavior: OutputBehavior::Hold,
            active: false,
        }
    ));
}
```

- [ ] **Step 2: Run failing GUI readout tests**

Run:

```bash
cargo test -p inputforge-gui-dx stage_summary_ --lib
cargo test -p inputforge-gui-dx live_readout_exposes_mouse_destination --lib
```

Expected: FAIL until summaries and analyzer model include mouse behavior.

- [ ] **Step 3: Update stage title and summary**

In `stage.rs`, add:

```rust
Action::MapToMouse { .. } => "Map to mouse",
```

Update keyboard and mouse summary arms:

```rust
Action::MapToKeyboard { key, behavior } => {
    format!("{} - {}", format_key_combo(key), format_behavior(*behavior))
}

Action::MapToMouse { target, behavior } => {
    if target.is_wheel() {
        target.label().to_owned()
    } else {
        format!("{} - {}", target.label(), format_behavior(*behavior))
    }
}
```

Add:

```rust
fn format_behavior(behavior: OutputBehavior) -> &'static str {
    match behavior {
        OutputBehavior::Hold => "Hold",
        OutputBehavior::Pulse => "Pulse",
    }
}
```

- [ ] **Step 4: Extend live readout model**

In `analyzer.rs`, change `OutputDestination`:

```rust
Keyboard {
    key: KeyCombo,
    behavior: OutputBehavior,
    pressed: bool,
},
Mouse {
    target: MouseTarget,
    behavior: OutputBehavior,
    active: bool,
},
```

In the walker:

```rust
Action::MapToMouse { target, behavior } => {
    let active = context.keyboard_pressed(branch_path, i);
    let behavior = if target.is_wheel() {
        OutputBehavior::Pulse
    } else {
        *behavior
    };
    model.outputs.push(OutputDescriptor {
        destination: OutputDestination::Mouse {
            target: *target,
            behavior,
            active,
        },
        chain: chain_stack.clone(),
        is_active: compute_is_active(chain_stack),
        polarity: AxisPolarity::Bipolar,
    });
}
```

- [ ] **Step 5: Render mouse rows**

In `out_block.rs`, add a mouse destination arm:

```rust
OutputDestination::Mouse {
    target,
    behavior,
    active,
} => {
    let is_live = engine_running && descriptor.is_active && *active;
    let tag = if target.is_wheel() {
        target.label().to_owned()
    } else {
        format!("{} - {}", target.label(), format_behavior(*behavior))
    };
    rsx! {
        ButtonReadoutRow {
            label: row_label.clone(),
            tag,
            pressed: if target.is_wheel() { false } else { is_live },
            frozen,
        }
    }
}
```

Wheel rows use `pressed: false` and the wheel label only. The row is intentionally momentary because wheel output is pulse-only.

- [ ] **Step 6: Run readout tests**

Run:

```bash
cargo test -p inputforge-gui-dx stage_summary_ --lib
cargo test -p inputforge-gui-dx live_readout_exposes_mouse_destination --lib
cargo check -p inputforge-gui-dx
```

Expected: PASS for readout tests and compile check.

- [ ] **Step 7: Commit**

Run:

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage.rs crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/analyzer.rs crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/out_block.rs crates/inputforge-gui-dx/src/frame/mapping_editor/tests.rs
git commit -m "feat(gui): show mouse output readouts"
```

### Task 10: End-To-End Verification

**Files:**
- Modify as needed only for compile errors found by verification.
- Test: workspace test suite.

- [ ] **Step 1: Run formatting**

Run: `cargo fmt --all --check`

Expected: PASS. If it fails, run `cargo fmt --all`, then rerun the check.

- [ ] **Step 2: Run core tests**

Run: `cargo test -p inputforge-core`

Expected: PASS.

- [ ] **Step 3: Run GUI tests**

Run: `cargo test -p inputforge-gui-dx`

Expected: PASS.

- [ ] **Step 4: Run app build**

Run: `cargo check -p inputforge-app`

Expected: PASS.

- [ ] **Step 5: Launch Dioxus GUI for manual smoke**

Run: `dx run -p inputforge-app`

Expected: the app opens. In the mapping editor:

- Add a stage from the output section.
- Confirm `Map to mouse` appears.
- Add `Map to mouse`.
- Confirm the target selector lists `Left click`, `Right click`, `Middle click`, `Back button`, `Forward button`, `Wheel up`, and `Wheel down`.
- Select `Wheel up`.
- Confirm the behavior selector disappears.
- Select `Left click`.
- Confirm `Hold` and `Pulse` appear.

- [ ] **Step 6: Use Chrome DevTools Protocol smoke check**

With `dx run -p inputforge-app` still running, open `http://127.0.0.1:9222/json` or `http://localhost:9222/json`.

Expected: a WebView target exists. Use the configured Chrome DevTools MCP server to capture a screenshot of the mapping editor with the `Map to mouse` body open.

- [ ] **Step 7: Commit verification fixes**

If verification required edits, commit them:

```bash
git add crates/inputforge-core crates/inputforge-gui-dx crates/inputforge-app
git commit -m "fix(mouse-output): resolve integration issues"
```

If no edits were needed, skip this commit.

## Self-Review

Spec coverage:

- `MapToMouse` action, `MouseTarget`, behavior serde, default `Hold`, and wheel normalization are covered in Task 1.
- Pipeline current-value intent, owner metadata, and side-effect-free evaluation are covered in Task 2.
- Separate mouse sink and keyboard down/up/pulse support are covered in Tasks 3 and 4.
- Runtime edge detection, ref-counting, pulse gating, conditional owner disappearance, and cleanup are covered in Tasks 5 through 7.
- GUI palette, target selector, behavior selector, wheel behavior hiding, summaries, undo labels, and live readout are covered in Tasks 8 and 9.
- Mock output and Windows conversion tests are covered in Tasks 3 and 4.
- Full verification and manual Dioxus smoke are covered in Task 10.

Placeholder scan:

- No step uses vague recovery language in place of code.
- No step uses undefined model names without defining them in the same or an earlier task.
- No task asks for generic tests without concrete test names and assertions.

Type consistency:

- `OutputBehavior` and `MouseTarget` are defined in `inputforge_core::action` and reused consistently by pipeline, engine, output sinks, and GUI.
- `PipelineOutput::Keyboard` and `PipelineOutput::Mouse` carry `owner`, destination data, `behavior`, and `active`.
- `OutputRuntimeState` consumes pipeline owner metadata and emits `OutputAction` values; immediate events dispatch directly, and release actions commit owner state only after successful sink dispatch.
