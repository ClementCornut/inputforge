# Map To Mouse Output Design Spec

## Context

InputForge already supports output actions such as `MapToKeyboard` and `MapToVJoy`. The new feature adds a mouse output action so a mapping can synthesize OS mouse input. This is output support only: it does not make physical mouse buttons available as InputForge input sources.

The intended mental model is parallel to keyboard mapping:

```text
InputForge input -> pipeline -> synthetic OS output
```

For example:

```text
Joystick Button 3 -> MapToKeyboard(Space)
Joystick Button 4 -> MapToMouse(LeftButton)
Joystick Button 5 -> MapToMouse(WheelUp)
```

## Confirmed Design Choices

Add one output action named `MapToMouse`, carrying a closed target enum named `MouseTarget`.

Targets:

- `LeftButton`
- `RightButton`
- `MiddleButton`
- `BackButton`
- `ForwardButton`
- `WheelUp`
- `WheelDown`

Mouse button targets behave like `MapToKeyboard`: when the mapping source is active, InputForge emits and holds a synthetic mouse button down state; when inactive, it releases that button.

Wheel targets are pulse outputs. On an inactive-to-active transition, InputForge emits one wheel step. It does not repeat while the source remains active. After the source returns inactive, a later active transition may emit another wheel step.

Analog inputs use the existing button press threshold behavior used by keyboard mapping. Hat input behavior should match `MapToKeyboard` unless the implementation plan explicitly expands hat support for both action types.

## Architecture

### Core action model

Add `Action::MapToMouse { target: MouseTarget }` beside the existing output actions. The enum should derive the same traits and serde behavior as related action data. Unknown target strings in hand-edited profiles should fail profile loading through the existing serde/profile error path.

Human-readable labels should be stable and concise:

- `Left click`
- `Right click`
- `Middle click`
- `Back button`
- `Forward button`
- `Wheel up`
- `Wheel down`

### Pipeline

The pipeline should translate the current input value into mouse output commands.

For button targets:

```text
inactive -> active: emit button down
active -> inactive: emit button up
active -> active: no duplicate down event
inactive -> inactive: no event
```

For wheel targets:

```text
inactive -> active: emit one wheel event
active -> active: no event
active -> inactive: no event
inactive -> inactive: no event
```

The implementation should release any held mouse buttons on shutdown, profile switch, or mapping removal using the same lifecycle principles as held keyboard keys.

### Output backend

Add a mouse output path beside the keyboard output path. On Windows, implement it with `SendInput`.

Button targets map to mouse button down/up flags. Back and forward use the extended mouse button data path. Wheel targets use the wheel event with one standard notch per pulse:

```text
WheelUp: +WHEEL_DELTA
WheelDown: -WHEEL_DELTA
```

The low-level Windows mapping should be factored so most flag/data conversion can be tested without issuing live OS mouse events.

### Mock output

Extend the mock output implementation to record mouse events. Tests should be able to assert both held button events and wheel pulses without interacting with the operating system.

### GUI

Add a `Map to mouse` item to the action palette in the output section.

Add a stage body for `MapToMouse` with one compact target selector containing the seven targets. The stage summary and live readout should use the stable labels listed above.

Target changes should dispatch normal stage edits and produce undo labels consistent with existing output action editors.

## Error Handling

`MouseTarget` is a closed enum, so invalid targets should be impossible after successful deserialization.

If a profile contains an unknown target value, loading should fail through the existing profile error path. The feature does not need a custom recovery path for malformed hand-edited JSON.

The Windows backend should report OS-level `SendInput` failures through the existing output error mechanism. The implementation plan should identify whether that mechanism already covers keyboard output adequately or needs a small shared helper.

## Testing

Core/model tests:

- `Action::MapToMouse` serde round-trip for each target.
- Human-readable labels remain stable.

Pipeline tests:

- Button targets emit down/up behavior from digital input.
- Analog input uses the same active threshold as keyboard output.
- Wheel targets emit once on inactive-to-active transition.
- Wheel targets do not repeat while held.
- Wheel targets can emit again after release and re-press.
- Hat behavior matches `MapToKeyboard`.

Output tests:

- Mock output records button down/up events.
- Mock output records wheel pulses.
- Windows conversion maps each target to the expected `SendInput` flags and mouse data without requiring a live click test.

GUI tests:

- Add palette includes `Map to mouse`.
- Stage body renders all seven targets.
- Changing targets dispatches the expected stage edit and undo label.
- Stage summary and live readout display stable labels.

## Out Of Scope

- Physical mouse buttons as InputForge input sources.
- Mouse movement output.
- Horizontal wheel output.
- Continuous auto-repeat scrolling while a source is held.
- Per-action custom scroll amount.

These can be designed later if needed, but they should not be folded into this first mouse output feature.

## Definition Of Done

- Profiles can serialize and deserialize `MapToMouse` actions.
- Users can add and edit a `Map to mouse` action in the Dioxus GUI.
- Mouse buttons left, right, middle, back, and forward can be held and released from mappings.
- Wheel up and wheel down emit one standard scroll step per activation.
- Held mouse buttons are released during normal output cleanup paths.
- Automated tests cover core model, pipeline behavior, mock output, Windows conversion, and GUI editor behavior.
