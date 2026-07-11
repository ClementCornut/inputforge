# Linux Support Design

**Date:** 2026-07-11

**Status:** Approved for implementation

## Goal

Run InputForge natively on CachyOS/Arch Linux while preserving Windows behavior and existing profile files. Linux must support physical controller input, exclusive capture, configurable virtual controllers, keyboard output, mouse output, hotplug, diagnostics, and both native-Linux and Proton consumers.

## Architectural Direction

Keep the platform-neutral engine and profile model in `inputforge-core`. Move backend selection into an application-level `PlatformBackends` factory with target-specific implementations. The core crate has a hardware-free default feature set so its model and tests compile on every supported host.

Windows continues to compose SDL3, vJoy, and Win32 output backends. Linux composes direct evdev input with uinput controller, keyboard, and mouse output backends. Backend construction errors are copied into shared application state before the engine thread exits so the GUI can explain failures.

## Portable Vocabulary and Compatibility

Public UI and new Rust APIs use the generic term “virtual device.” Existing profile serialization remains compatible:

- The `map_to_vjoy` action key continues to deserialize and serialize unchanged.
- Existing axis values and profile structures retain their wire representation.
- Windows profiles load on Linux without rewriting the profile file.

Internal legacy Rust names may be migrated behind serde aliases or compatibility aliases where necessary. Compatibility tests must load representative v0.2.0 profiles.

## Input Backend

Linux uses evdev directly rather than SDL so the same file descriptors can read events and acquire `EVIOCGRAB` exclusive access. Device discovery includes controller-class devices that either have the udev joystick classification or a joydev handler. This intentionally includes axis-only pedals while excluding keyboards, mice, genuine accelerometers, and InputForge-created virtual devices.

Each local device receives a stable identifier using, in order:

1. serial or evdev unique identifier;
2. vendor ID, product ID, name, and stable device path fallback.

The backend monitors udev for hotplug changes. Starting the engine attempts to grab every selected physical controller atomically. If any grab fails, all grabs acquired during that attempt are released and the engine stays stopped. Stopping or encountering a runtime backend error releases all grabs and tears down virtual outputs.

`InputSource::poll` returns `Result<()>` so disconnects and permission failures can propagate. Exclusive-capture lifecycle methods live on the input abstraction. The unused `DeviceHider` ownership is removed from `Engine`.

## Cross-Platform Input Bindings

SDL GUIDs, evdev identities, and control indices are platform-dependent. Linux settings therefore store a local binding overlay from each complete foreign `InputAddress` used by the active profile to a local Linux input address. The overlay is local configuration; the imported profile is never rewritten.

Activation is blocked until every foreign input address referenced by the active profile has a valid local mapping. The settings UI shows unresolved addresses and provides live input capture to create mappings.

## Output Backends

Linux exposes independent uinput sinks for virtual controllers, keyboard events, and relative mouse events including wheel axes. The user can configure 1 through 16 virtual controllers while the engine is stopped. A fresh configuration contains one virtual controller with eight axes, 128 buttons, and four hats.

Controller identity and capabilities are deterministic:

- bus: `BUS_VIRTUAL`;
- name: `InputForge Virtual Controller N`;
- physical path prefix: `inputforge/`;
- axes X, Y, Z, Rx, Ry, Rz, Slider0, Slider1 map to ABS_X, ABS_Y, ABS_Z, ABS_RX, ABS_RY, ABS_RZ, ABS_THROTTLE, ABS_RUDDER;
- axis range is normalized to -32767 through 32767;
- four hats use ABS_HAT0X/Y through ABS_HAT3X/Y with values -1, 0, or 1;
- buttons 1 through 128 use consecutive key codes 0x120 through 0x19f.

Output reconfiguration is rejected while running. Output state is flushed on teardown to avoid stuck keys, buttons, or axes.

## State and Commands

Engine commands cover virtual-device reconfiguration and Linux input-overlay updates. Shared app state includes backend health, actionable error text, unresolved bindings, and detected devices. Startup failures remain visible after the engine thread terminates. Runtime failures transition to stopped, clean up resources, and keep the application usable for diagnosis or configuration.

## Diagnostics and Distribution

`inputforge --diagnose-linux` is read-only. It reports uinput availability and permissions, detected controller candidates and classification evidence, stable IDs, grab feasibility without retaining a grab, and relevant remediation guidance. It must not create persistent virtual devices or modify host configuration.

The CachyOS/Arch package installs narrowly scoped udev rules:

- grant active-seat access to `/dev/uinput`;
- correct the Thrustmaster Sim Pedals `044f:b371` classification and access;
- do not require root runtime or membership in the broad `input` group.

Packaging documents reloading rules and reconnecting hardware after installation.

## Verification

Verification is layered:

- portable unit and serialization tests run on Linux and Windows CI;
- evdev classification and address translation use fixture-driven tests;
- uinput event encoding is unit-tested without hardware;
- privileged integration tests are opt-in and exercise discovery, grabs, hotplug, controller output, keyboard, mouse, and cleanup;
- manual acceptance validates a generic DirectInput harness under Proton and a native Linux input-event consumer;
- the three controllers on the target CachyOS system are used for final discovery, exclusive-capture, and mapping validation.

## Delivery Slices

1. Portable build baseline, target-specific backend factory, generic terminology, and profile compatibility tests.
2. Read-only evdev discovery and diagnostics, followed by hotplug and atomic exclusive grabs.
3. Configurable uinput virtual controllers.
4. Keyboard and mouse output, Linux binding/configuration UI, and backend health states.
5. Arch packaging, CI, and native plus Proton acceptance documentation.

Each slice must leave the workspace buildable and independently testable.
