# Map To Mouse Output And Behavior Design Spec

## Context

InputForge already supports output actions such as `MapToKeyboard` and `MapToVJoy`. The new feature adds a mouse output action so a mapping can synthesize OS mouse input. This is output support only: it does not make physical mouse buttons available as InputForge input sources.

The intended mental model is parallel to keyboard mapping at the product level:

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

Add a shared `OutputBehavior` enum for keyboard and mouse output actions:

- `Hold`
- `Pulse`

`OutputBehavior` serializes in profiles as `behavior: "hold"` or `behavior: "pulse"`. Missing `behavior` fields deserialize as `Hold`, including old `MapToKeyboard` profile data. This intentionally changes legacy keyboard mappings from the current pulse-like behavior to held-key behavior.

Targets:

- `LeftButton`
- `RightButton`
- `MiddleButton`
- `BackButton`
- `ForwardButton`
- `WheelUp`
- `WheelDown`

Keyboard targets and mouse button targets support both behaviors. `Hold` presses on inactive-to-active and releases on active-to-inactive. `Pulse` sends one press-and-release pulse on inactive-to-active and does not repeat while the source remains active.

Wheel targets are always pulse outputs. On an inactive-to-active transition, InputForge emits one wheel step. It does not repeat while the source remains active. After the source returns inactive, a later active transition may emit another wheel step.

Analog inputs use the existing button press threshold behavior used by keyboard mapping. Hat input behavior should match `MapToKeyboard` unless the implementation plan explicitly expands hat support for both action types.

## Architecture

### Core action model

Update keyboard and mouse output actions to carry behavior:

- `Action::MapToKeyboard { key: KeyCombo, behavior: OutputBehavior }`
- `Action::MapToMouse { target: MouseTarget, behavior: OutputBehavior }`

`MapToMouse` serializes with profile JSON tag `type: "map_to_mouse"`. `MouseTarget` serializes as PascalCase strings matching the variant names, for example `"LeftButton"` and `"WheelUp"`.

The enums should derive the same traits as related action data. Unknown target or behavior strings in hand-edited profiles should fail profile loading through the existing serde/profile error path.

Human-readable labels should be stable and concise:

- `Left click`
- `Right click`
- `Middle click`
- `Back button`
- `Forward button`
- `Wheel up`
- `Wheel down`

### Pipeline

The pipeline should translate the current input value into current output intent. Keyboard and mouse output should produce pipeline outputs that include destination, behavior, and `active` state. `active` uses the existing button press threshold for button and analog input. Hat input behavior should match existing `MapToKeyboard` behavior unless the implementation plan explicitly expands hat support.

The pipeline must not own previous activation state. It remains a current-value evaluator so GUI projections and recursive conditional evaluation stay side-effect free.

### State and lifecycle

The engine/output handling layer owns keyboard and mouse edge detection in runtime output behavior state.

State is keyed by output owner `(mode, input, action path/output identity)`. The owner key must distinguish multiple keyboard or mouse actions in the same mapping, including actions nested inside conditionals.

For `Hold` keyboard keys and mouse buttons:

```text
owner inactive -> active: register owner; send key/button down if this is the first active owner for the destination
owner active -> inactive: unregister owner; send key/button up if this was the last active owner for the destination
owner active -> active: no duplicate down event
owner inactive -> inactive: no event
```

For `Pulse` keyboard keys, mouse buttons, and wheel targets:

```text
owner inactive -> active: emit one press-and-release or wheel event and mark owner active
owner active -> active: no event
owner active -> inactive: mark owner inactive
owner inactive -> inactive: no event
```

Wheel targets are always effective `Pulse`, regardless of any deserialized behavior value. The GUI should write `Pulse` for wheel targets, and the runtime should treat wheel targets as pulse-only to avoid impossible held-wheel semantics.

Cleanup should release held keyboard keys and mouse buttons on mapping edit/removal, bulk mapping replacement, profile load/unload/delete/restore, mode switch, mode deletion, pause, deactivate, shutdown, and command-channel disconnect. Cleanup is best-effort through the keyboard and mouse sinks: failures use the existing output error mechanism and should leave the affected state retryable until release is confirmed.

### Output backend

Add a separate `MouseSink` beside `KeyboardSink` and the vJoy `OutputSink`. Do not add OS mouse methods to the vJoy output trait.

Update the runtime keyboard sink path so it can send key down/up events as well as a press-and-release pulse. The current low-level Windows keyboard implementation already has press/release building blocks; the trait used by the engine should expose them so held keyboard output does not need to bypass the sink abstraction.

On Windows, implement `MouseSink` with `SendInput`. Normal button targets map to mouse button down/up flags. Back and forward use `MOUSEEVENTF_XDOWN`/`MOUSEEVENTF_XUP` plus `XBUTTON1`/`XBUTTON2` in `mouseData`. Wheel targets use `MOUSEEVENTF_WHEEL` with one standard notch per pulse:

```text
WheelUp: +WHEEL_DELTA
WheelDown: -WHEEL_DELTA
```

The low-level Windows mapping should be factored so flag/data conversion can be tested without issuing live OS mouse events. `SendInput` failures should check the returned input count and include `GetLastError` details through the existing output error path.

### Mock output

Extend the mock output implementation to record mouse events through `MouseSink`. Tests should be able to assert both held button events and wheel pulses without interacting with the operating system.

### GUI

Add a `Map to mouse` item to the action palette in the output section.

Add a stage body for `MapToMouse` with one compact target selector containing the seven targets. The stage title, header, summary, and live readout should use the stable labels listed above.

Add a compact `Hold` / `Pulse` behavior selector to the `MapToKeyboard` editor and to the `MapToMouse` editor for button targets. Hide the selector for `WheelUp` and `WheelDown` because wheel targets are always pulse-only.

New keyboard and mouse button actions should default to `Hold`. New wheel actions should default to effective `Pulse`.

Target and behavior changes should dispatch normal stage edits and produce undo labels consistent with existing output action editors.

The live-readout analyzer and output destination model should represent `MapToMouse` explicitly so the GUI can show mouse button and wheel targets without treating them as vJoy or keyboard output. Stage summaries and live readout should include behavior where it disambiguates output, for example `Ctrl+A - Hold` or `Left click - Pulse`.

## Error Handling

`MouseTarget` is a closed enum, so invalid targets should be impossible after successful deserialization.

If a profile contains an unknown target value, loading should fail through the existing profile error path. The feature does not need a custom recovery path for malformed hand-edited JSON.

The Windows backend should report OS-level `SendInput` failures through the existing output error mechanism. The implementation plan should identify whether that mechanism already covers keyboard output adequately or needs a small shared helper.

## Testing

Core/model tests:

- `Action::MapToKeyboard` serde round-trip for `Hold` and `Pulse`.
- `Action::MapToMouse` serde round-trip for each target.
- Missing `behavior` deserializes as `Hold`, including old keyboard action JSON.
- Invalid `MouseTarget` strings fail profile loading through the existing profile error path.
- Invalid `OutputBehavior` strings fail profile loading through the existing profile error path.
- Human-readable labels remain stable.

Pipeline/engine state tests:

- Keyboard and mouse `Hold` emit down/up behavior from digital input.
- Keyboard and mouse `Hold` do not emit duplicate down events while held.
- Keyboard and mouse `Pulse` emit one press-and-release pulse per inactive-to-active transition.
- Keyboard and mouse `Pulse` do not repeat while held.
- Keyboard and mouse `Pulse` can fire again after release and re-press.
- Analog input uses the same active threshold as keyboard output.
- Wheel targets emit once on inactive-to-active transition.
- Wheel targets do not repeat while held.
- Wheel targets can emit again after release and re-press.
- Wheel state resets after inactive transitions.
- Hat behavior matches `MapToKeyboard`.

Lifecycle cleanup tests:

- Held keyboard keys release on mapping edit/removal.
- Held mouse buttons release on mapping edit/removal.
- Held keyboard keys release on bulk mapping replacement.
- Held mouse buttons release on bulk mapping replacement.
- Held keyboard keys release on profile switch, delete, and restore.
- Held mouse buttons release on profile switch, delete, and restore.
- Held keyboard keys release on mode switch and mode deletion.
- Held mouse buttons release on mode switch and mode deletion.
- Held keyboard keys release on pause, deactivate, shutdown, and command-channel disconnect.
- Held mouse buttons release on pause, deactivate, shutdown, and command-channel disconnect.

Output tests:

- Mock keyboard output records key down/up and pulse events.
- Mock output records button down/up events.
- Mock output records wheel pulses.
- Windows conversion maps each target to the expected `SendInput` flags and mouse data without requiring a live click test.

GUI tests:

- Add palette includes `Map to mouse`.
- `MapToKeyboard` body renders `Hold` / `Pulse`.
- Stage body renders all seven targets.
- `MapToMouse` body renders `Hold` / `Pulse` for button targets.
- `MapToMouse` body hides the behavior selector for wheel targets.
- Changing targets dispatches the expected stage edit and undo label.
- Changing behavior dispatches the expected stage edit and undo label.
- Stage title, header, summary, and live readout display stable labels.
- Live-readout analyzer exposes mouse output destinations.

## Out Of Scope

- Physical mouse buttons as InputForge input sources.
- Mouse movement output.
- Horizontal wheel output.
- Continuous auto-repeat scrolling while a source is held.
- Per-action custom scroll amount.

These can be designed later if needed, but they should not be folded into this first mouse output feature.

## Definition Of Done

- Profiles can serialize and deserialize `MapToMouse` actions.
- Profiles can serialize and deserialize keyboard and mouse `OutputBehavior`.
- Users can add and edit a `Map to mouse` action in the Dioxus GUI.
- Users can choose `Hold` or `Pulse` behavior for keyboard mappings and mouse button mappings.
- Mouse buttons left, right, middle, back, and forward can be held, released, or pulsed from mappings.
- Wheel up and wheel down emit one standard scroll step per activation.
- Held keyboard keys and mouse buttons are released during mapping, profile, mode, status, shutdown, and disconnect cleanup paths.
- Automated tests cover core model, pipeline behavior, mock output, Windows conversion, and GUI editor behavior.
