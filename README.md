# InputForge

Remap physical joystick, pedal, and throttle inputs to virtual devices.

## Development prerequisites

- GNU Make, available as `make` on your PATH on Linux or Windows.
- A current stable Rust toolchain with Cargo, rustfmt, and Clippy. The workspace
  declares Rust 1.88 as its minimum version; dependencies may require newer Rust.
- Dioxus CLI (`dx`) for app launch, GUI examples, and packaging. The release
  workflow in [release.yml](.github/workflows/release.yml) records the CLI version
  used for packaging. It is not needed for Cargo-only checks and tests.
- Dioxus Desktop native prerequisites: WebView2 on Windows; GTK and WebKitGTK
  development libraries on Linux when building the GUI crate.
- Windows app builds need `SDL/SDL3.dll` from the repository's Git LFS files.
  Installer packaging also requires the NSIS tooling used by Dioxus.

The full application currently runs on Windows. On Linux it deliberately exits
with an unavailable-backends message before starting the GUI: evdev and uinput
support are not implemented yet. The GUI demo and gallery are separate development
harnesses; they do not provide hardware input/output support.

## Common tasks

Run commands from the repository root. Bare `make` shows help.

| Command | Task |
| --- | --- |
| `make run` | Launch the full app through Dioxus, including asset handling |
| `make dev` | Launch the full app with hot reload |
| `make demo` | Hot reload the GUI with seeded mock data and no engine |
| `make gallery` | Hot reload the component gallery |
| `make build` | Build the workspace |
| `make test` | Run workspace tests, including documentation tests |
| `make check` | Type-check all workspace targets |
| `make fmt` | Format Rust source files |
| `make fmt-check` | Check formatting without changing files |
| `make lint` | Run Clippy on all workspace targets, denying warnings |
| `make verify` | Check formatting, then Clippy, then tests; stop on failure |
| `make bundle` | Build a local Windows NSIS release installer |
| `make clean` | Remove Cargo build artifacts |

`make verify` keeps its checks sequential even with `make -j`. Build, check, test,
lint, and bundle use the existing lockfile through `--locked`.

Pass additional arguments to build, check, test, run, dev, demo, gallery, or bundle
with `ARGS`:

```sh
make test ARGS="profile -- --nocapture"
make build ARGS="--release"
```

`CARGO` and `DX` override executable paths, including paths containing spaces:

```sh
make test CARGO="/path/to/cargo"
make demo DX="/path/to/dx"
```

These overrides name executables, not shell commands with embedded flags.

## Local Windows installer

On Windows, `make bundle` runs the same Dioxus bundle command as the release
workflow, producing an installer under `target/dx/*/bundle/windows/nsis/`.
It does not run tests, sign the installer, or publish a release; run `make verify`
first when preparing a release. Other hosts receive a clear error.

For GUI development details, see the [GUI guide](crates/inputforge-gui-dx/README.md).
