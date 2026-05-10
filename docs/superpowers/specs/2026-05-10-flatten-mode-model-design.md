# Flatten Mode Model Design Spec

## Context

Profiles should store modes as a flat, ordered list. The first list entry is the first tab in the GUI. `startup_mode` remains an independent profile setting validated against the mode list.

The feature is pre-distribution and does not need profile conversion support. The supported on-disk shape is root-level:

```toml
modes = ["Default", "Combat", "Landing"]

[profile]
id = "01J00000000000000000000000"
name = "Flight"
startup_mode = "Default"
```

Unsupported non-list `modes` input fails with neutral flat-list wording.

## Confirmed Design Choices

- `inputforge-core` exposes `Modes(Vec<String>)` with private storage.
- `Modes::new` enforces a non-empty list and ASCII-case-insensitive uniqueness.
- `Modes` provides `as_slice`, `first`, `len`, `contains`, `with_appended`, `with_renamed`, and `with_removed`.
- `Profile` stores `Modes`, and `ProfileRaw` expects root-level `modes = [...]`.
- Mapping resolution is direct `(input, mode)` lookup.
- `EngineCommand::AddMode` has only `name: String`; new modes append to the list.
- `DeleteMode` rejects the first mode and the startup mode, then removes only the named mode and directly scoped mappings.
- GUI duplicate-name validation uses the same ASCII-case-insensitive comparison as the engine.
- `MetaSnapshot.modes` remains `Vec<String>`.

## Touchpoints

- Core mode model: `crates/inputforge-core/src/mode/mod.rs` and `crates/inputforge-core/src/mode/state.rs`.
- Profile loading and saving: `crates/inputforge-core/src/profile/mod.rs` and `crates/inputforge-core/src/profile/manager.rs`.
- Engine routing and commands: `crates/inputforge-core/src/engine/dependencies.rs`, `crates/inputforge-core/src/engine/output_handler.rs`, `crates/inputforge-core/src/engine/run.rs`, `crates/inputforge-core/src/engine/command.rs`, and `crates/inputforge-core/src/engine/tests.rs`.
- Snapshot fixtures: `crates/inputforge-core/src/snapshot/tests.rs`.
- GUI mode tabs: files under `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/`.

## Tests

- `Modes::new` accepts non-empty unique names and rejects empty or duplicate names.
- `Modes` serializes and deserializes as `modes = ["Default", ...]`.
- Non-list `modes` values fail with `modes must be a flat list of strings`.
- Non-string mode entries fail with `mode names must be strings`.
- Profile round-trip tests keep `modes = [...]` at the TOML root.
- Engine command tests cover append, rename cascade, delete-first rejection, delete-startup rejection, and direct mapping cleanup.
- Snapshot tests use root-level `modes = [...]` fixtures.
- GUI tests cover case-insensitive duplicate rejection and delete disabling for first/startup modes.

## Acceptance Gates

- `cargo test -p inputforge-core -p inputforge-gui-dx` passes.
- `cargo fmt --all --check` passes.
- Mode-related code, tests, and the touched plan/spec docs contain no removed mode-model identifiers or unsupported profile-shape examples.
- `EngineCommand::AddMode` has only `name: String`.
- Root-level `modes = [...]` is the only documented supported profile mode shape.
