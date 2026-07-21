# InputForge Linux Portable Baseline Design

## Goal

Deliver Slice 1 of the Linux port: a portable build baseline, target-specific
application composition, generic virtual-device terminology, and explicit
v0.2.0 profile compatibility. The slice must compile and test on Linux without
claiming that Linux input or output backends exist, and it must preserve the
current Windows backend behavior.

The broader Linux architecture remains defined by
`2026-07-11-linux-support-design.md`. This document narrows only the first
delivery slice.

## Verified Baseline

- `inputforge-core --no-default-features` compiles on the CachyOS/Arch host.
- `inputforge-core --no-default-features --features sdl3-input` also compiles.
- The core default and full workspace builds fail on Linux because default
  features activate the Win32 and vJoy dependency graphs. The observed failures
  come from `windows-future` and `vjoy-sys`.
- `inputforge-app` directly constructs `Sdl3Input`, `VJoyOutput`, Win32 keyboard
  output, Win32 mouse output, and a no-op device hider in the engine thread.
- `Engine` stores its `DeviceHider`, but no engine code reads or calls it.
- `InputSource` already provides the portable abstraction needed by this slice;
  its current polling contract does not block the factory boundary.
- The v0.2.0 tag contains no committed profile fixtures. Two full Windows-created
  profiles were supplied separately for compatibility-fixture provenance.

## Scope Boundary

Slice 1 includes:

- hardware-free core defaults;
- target-specific Cargo dependency activation;
- an application-level `PlatformBackends` factory;
- deterministic Linux startup failure while hardware backends are unavailable;
- removal of unused `DeviceHider` ownership from `Engine`;
- generic user-facing virtual-device terminology;
- frozen compatibility tests for existing v0.2.0 profiles.

Slice 1 does not include evdev, uinput, device discovery, hotplug, exclusive
grabs, Linux input-address overlays, placeholder production sinks, backend
health state, Linux settings UI, packaging, or Linux CI. Fallible input polling
and exclusive-capture lifecycle methods are deferred until Slice 2, where an
actual Linux input backend needs them.

## Application-Level Backend Factory

`inputforge-app` owns a crate-private `PlatformBackends` bundle. Its exact
ownership fields are:

- `input`: boxed `InputSource`;
- `controller`: boxed `OutputSink`;
- `keyboard`: boxed `KeyboardSink`;
- `mouse`: boxed `MouseSink`.

“Factory” is an architectural description of this composition boundary, not a
public Rust type name. Slice 1 does not introduce a public `*Factory` API.

The bundle exposes two crate-private operations:

- `preflight`, returning an application-level `anyhow::Result<()>` without
  constructing hardware;
- `create`, returning `anyhow::Result<PlatformBackends>` and transferring
  exclusive backend ownership to the caller.

The module is split into a platform-neutral facade and cfg-selected Windows and
Linux implementations. Unsupported targets produce a compile-time error rather
than inheriting Linux behavior.

`main` parses the CLI and initializes logging before calling `preflight`, so
normal Linux launches fail before shared state, profile, tray, or GUI side
effects. Clap still handles `--help` before preflight. The stable Linux error is:

> Linux input and output backends are unavailable in Slice 1; evdev and uinput
> arrive in later slices.

Linux `create` returns the same error defensively and constructs no placeholder
objects. Windows `preflight` succeeds without probing hardware. Windows `create`
is called inside the engine thread and constructs the same SDL3, vJoy, Win32
keyboard, and Win32 mouse backends used by v0.2.0. This preserves SDL3's
same-thread construction and use requirement.

`run_engine_inner` returns an application-level `anyhow::Result<()>`, allowing
existing core construction and runtime errors to propagate without adding a
platform-unavailable variant to `EngineError`. Windows construction failures
continue through the existing engine-thread logging path. Shared backend-health
state and GUI presentation remain Slice 4 work.

## Core Ownership Cleanup

Remove the `DeviceHider` field and constructor argument from `Engine`, together
with the obsolete documentation claiming that the engine owns it. Update all
engine construction sites and tests accordingly.

Keep the `DeviceHider` trait, `HidHideManager`, `NoOpDeviceHider`,
`MockDeviceHider`, exports, and direct tests. Removing that public compatibility
surface is unrelated to establishing the portable boundary and could create an
unnecessary Windows/API regression.

Do not change `InputSource` in Slice 1. In particular, `poll` remains infallible
and no grab or release lifecycle methods are added speculatively.

## Cargo Features and Target Dependencies

The portable default is enforced in manifests rather than left to each caller:

- The workspace dependency on `inputforge-core` disables default features.
- `inputforge-core` sets `default = []` and retains the existing
  `sdl3-input`, `vjoy-output`, `win32-io`, and `test-util` feature names.
- SDL3 remains an optional cross-platform dependency.
- `vjoy` and `windows` become optional dependencies under a Windows target
  dependency table.
- vJoy output, Win32 keyboard/mouse output, and HidHide modules and re-exports
  require both their existing feature and a Windows target.
- `inputforge-app` uses the hardware-free core dependency normally and enables
  `sdl3-input`, `vjoy-output`, and `win32-io` from its Windows target dependency
  entry.
- The app's unused direct dependency on `windows` is removed.
- `inputforge-gui-dx` continues to consume hardware-free core and gains no
  platform dependencies.

The existing feature names remain for compatibility. On Linux, enabling one of
the Windows-only feature names does not compile its target-specific dependency
or expose its Windows module. No dependency versions change, so `Cargo.lock` is
not expected to change.

When core defaults become hardware-free, `AxisValue::raw` is used only by tests
or the SDL3 backend. Gate it with `cfg(any(test, feature = "sdl3-input"))` so the
portable default remains warning-free without changing runtime behavior.

## Terminology and Serialization Compatibility

New APIs and generic UI concepts use “virtual device.” Existing compatibility
identifiers and wire representations remain stable:

- Keep the public Rust type name `VJoyAxis`.
- Keep the public Rust variant `Action::MapToVJoy`.
- Keep the canonical serialized action tag `map_to_vjoy` for both serialization
  and deserialization. Do not add or emit `map_to_virtual_device` in Slice 1.
- Add explicit serde spellings for all `VJoyAxis` variants: `X`, `Y`, `Z`, `Rx`,
  `Ry`, `Rz`, `Slider0`, and `Slider1`.

User-facing generic copy changes consistently:

- “Map to vJoy” becomes “Map to virtual device.”
- “vJoy device N” and “vJoy N” become “Virtual device N.”
- Empty states, output chips, selectors, snapshot labels, undo labels, headers,
  and live readouts use the generic term.

`vJoy` and `vJoyConf` remain visible only when naming the concrete Windows
driver or configuration tool. For example, the Windows-specific remediation
copy may say, “Configure virtual devices in vJoyConf, then reopen.” Internal
legacy filenames, helper names, enum matches, CSS classes, and telemetry keys do
not need mechanical renaming.

## v0.2.0 Compatibility Fixtures

Create full tracked fixtures from the two externally supplied Windows profiles.
Their source provenance is:

- `Default.toml`: SHA-256
  `fbfbc4bf8e9a3c3c9d121090275baac6295a2155221a7ebc07eed3e0cd4bd368`;
- `Star Citizen.toml`: SHA-256
  `d51ac212d93c6bed514243309ff27ebeabd82cbb0b0a42e41dc3048128e0bce8`.

Store the sanitized fixtures at:

- `crates/inputforge-core/tests/fixtures/v0_2_0/default.toml`;
- `crates/inputforge-core/tests/fixtures/v0_2_0/star-citizen.toml`.

The integration test lives at
`crates/inputforge-core/tests/profile_v0_2_0_compat.rs`.

Preserve all 77 mappings in each profile, action ordering, profile names, modes,
curve data, output addresses, and TOML structure. Sanitize identifiers only:

- replace the two profile UUIDs with
  `00000000-0000-0000-0000-000000000001` and
  `00000000-0000-0000-0000-000000000002` respectively;
- sort the four distinct source device GUID strings lexicographically and
  replace them consistently across both fixtures with these exact 32-character
  test identifiers, in order:
  - `00000000000000000000000000000001`;
  - `00000000000000000000000000000002`;
  - `00000000000000000000000000000003`;
  - `00000000000000000000000000000004`.

Do not commit the original identifiers or modify the source profile folder.

The hardware-free integration suite must:

- load both files through `Profile::load`;
- assert the expected profile names, mode counts, and 77 mappings per file;
- find 77 deserialized `Action::MapToVJoy` actions per file;
- assert the action ordering and exact curve data and output addresses from the
  source profiles;
- parse each sanitized fixture as `toml::Value`, serialize the loaded `Profile`,
  parse the emitted TOML as `toml::Value`, and assert equality; this preserves
  every represented value while allowing formatting and table-order changes;
- assert each fixture's exact approved profile UUID;
- recursively collect every string-valued TOML field named `device`, assert
  that no value falls outside the four approved test identifiers, and assert
  that the combined observed set equals all four identifiers;
- assert that serialization emits `map_to_vjoy` and never
  `map_to_virtual_device`;
- assert the exact serde spelling of every `VJoyAxis` variant.

These are characterization tests for pre-existing behavior. They must pass
before production refactoring; they are not forced into an artificial failing
test stage.

## Verification

Linux verification on the CachyOS/Arch host:

```text
cargo fmt --all -- --check
cargo check -p inputforge-core --locked
cargo check -p inputforge-core --no-default-features --locked
cargo check -p inputforge-core --no-default-features --features sdl3-input --locked
cargo check -p inputforge-core --no-default-features --features vjoy-output --locked
cargo check -p inputforge-core --no-default-features --features win32-io --locked
cargo check -p inputforge-core --no-default-features --features test-util --locked
cargo check -p inputforge-core --all-features --all-targets --locked
cargo check --workspace --all-targets --locked
cargo check --workspace --all-targets --all-features --locked
cargo test -p inputforge-core --test profile_v0_2_0_compat --locked
cargo test -p inputforge-app --test linux_startup --locked
cargo test --workspace --locked
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

Windows verification on a Windows environment with the repository's existing
SDL and vJoy prerequisites:

```text
cargo check -p inputforge-core --locked
cargo check -p inputforge-core --no-default-features --locked
cargo check -p inputforge-core --no-default-features --features sdl3-input --locked
cargo check -p inputforge-core --no-default-features --features vjoy-output --locked
cargo check -p inputforge-core --no-default-features --features win32-io --locked
cargo check -p inputforge-core --no-default-features --features test-util --locked
cargo check -p inputforge-core --all-features --all-targets --locked
cargo check --workspace --all-targets --locked
cargo check --workspace --all-targets --all-features --locked
cargo test --workspace --locked
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
dx build -p inputforge-app --locked
```

The Linux startup integration test launches the application along three process
paths:

- no arguments exits non-zero and matches the stable preflight error exactly;
- `--help` exits successfully with Clap help and no preflight error;
- `--version` exits successfully with the package version and no preflight
  error.

None of these paths may require a display, tray, SDL3 initialization, evdev,
uinput, or other hardware.

Existing engine tests protect the `DeviceHider` removal as a green-to-green
refactor. UI terminology changes use red-green order: update render expectations
first, observe failure against the old copy, then update production strings.
Cargo checks provide the red-green proof for feature gating: the verified
baseline fails through Windows-only dependencies, while the completed slice
passes the Linux matrix above.

## Acceptance Criteria

Slice 1 is complete when all of the following are true:

- Linux default workspace builds do not activate vJoy or Win32 dependencies.
- Hardware-free core defaults and explicit SDL3 input both compile on Linux.
- The normal Linux binary fails deterministically before tray or GUI startup
  with the specified message.
- Windows selects SDL3, vJoy, and Win32 keyboard/mouse output through
  `PlatformBackends` and passes the Windows verification matrix.
- `Engine` no longer owns or accepts a `DeviceHider`, while the hider API itself
  remains available.
- `InputSource` has no premature Slice 2 API changes.
- Both sanitized v0.2.0 profiles load and preserve the legacy action and axis
  wire formats.
- Generic UI terminology uses “virtual device,” with vJoy names limited to
  concrete Windows product references.
- The Linux and Windows feature matrices prove the hardware-free default,
  individual target-gated features, and the combined all-features build.
- Linux no-argument startup fails with the stable preflight error, while
  `--help` and `--version` retain their normal successful Clap behavior.
- No evdev, uinput, grabs, Linux overlay/configuration UI, placeholder sinks,
  backend-health state, packaging, or CI expansion is present.
