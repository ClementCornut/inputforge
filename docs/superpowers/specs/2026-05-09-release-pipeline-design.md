# Release Pipeline Design

## Context

InputForge is a Rust workspace with a Dioxus desktop app entry point in
`inputforge-app`. The repository already has a workspace-root `Dioxus.toml`
(`./Dioxus.toml`, not per-crate) with bundle metadata for the app name,
identifier, and Windows icon assets. There is no existing `.github` workflow in
this worktree.

The first release milestone is a tag-only GitHub Actions pipeline that bundles
a Windows installer users can download from a GitHub Release. Runtime
auto-update checks are intentionally out of scope for this milestone, but the
release asset layout should stay compatible with adding updater assets later.

Relevant Dioxus documentation:

- https://dioxuslabs.com/learn/0.7/tutorial/bundle/
- https://dioxuslabs.com/learn/0.7/guides/deploy/config/

This spec targets Dioxus CLI v0.7.9. Path and flag claims below are verified
against the `v0.7.9` tag on github.com/DioxusLabs/dioxus; specific file:line
citations appear inline at the relevant sections.

## Goals

- Publish releases only from semver tags such as `v0.1.0` or `v0.1.0-rc.1`.
- Verify the tag version matches the workspace package version before bundling.
- Bundle the Windows desktop app with Dioxus using the NSIS `.exe` package type.
- Upload the installer and a SHA-256 checksum to the GitHub Release.
- Keep signing optional so unsigned releases work immediately, while future
  signing can be enabled by adding CI secrets.
- Structure the workflow as a platform matrix so macOS and Linux packaging can
  be added later without rewriting the job.

## Non-Goals

- Do not implement runtime auto-update checks or update UI.
- Do not add Dioxus updater package output yet.
- Do not add macOS or Linux packaging rows yet.
- Do not acquire or require a Windows signing certificate.
- Do not implement macOS notarization or Linux repository publishing.

## Workspace Prerequisite

Before the first release tag is pushed, `[workspace.dependencies]` in the root
`Cargo.toml` must bump `dioxus` and `dioxus-ssr` from `0.7.6` to `0.7.9`. CI
pins the matching CLI version (see CI Flow). Bumping these together keeps the
runtime crate and the bundling CLI on the same patch release, which avoids
config-shape drift in `Dioxus.toml`.

This bump is sequenced before the workflow file lands. After the bump, the
implementer runs `cargo build -p inputforge-app` and `cargo test --workspace`
locally to confirm the patch upgrade is clean.

## Release Trigger

The workflow runs on pushed semver tags matching `v[0-9]+.[0-9]+.[0-9]+` with
an optional pre-release suffix (e.g., `v0.1.0`, `v0.1.0-rc.1`,
`v0.2.0-beta.3`). The tag is the source of truth for release publication.

Before building, CI strips the leading `v` and compares the full remainder
(including any pre-release suffix) to `[workspace.package].version` in the root
`Cargo.toml`. A mismatch fails the workflow before any release assets are
published. Pre-release tags require the workspace version to carry the matching
suffix; a `v0.1.0-rc.1` tag requires `version = "0.1.0-rc.1"` in `Cargo.toml`
for that release cycle.

GitHub Release creation happens after successful matrix builds. The release
name uses the tag name. If the resolved version contains a `-` (pre-release
suffix), the GitHub Release is created with `prerelease: true`; otherwise
`prerelease: false`. Release assets are attached to that tag's release.

## Build Matrix

The workflow is matrix-shaped from the first implementation, with only Windows
enabled initially:

```yaml
include:
  - os: windows-latest
    platform: windows
    package_type: nsis
    asset_glob: target/dx/**/bundle/nsis/*.exe
```

The job body should read runner, package type, and artifact glob values from
the matrix. Future platform rows can add macOS package types such as `dmg` or
Linux package types such as `appimage` and `deb`, plus any platform-specific
setup steps.

The asset glob path comes from Dioxus 0.7.9 NSIS output: `dx bundle` writes the
installer to
`target/dx/<crate>/bundle/nsis/<ProductName>_<Version>_<Arch>-setup.exe`. There
is a single `bundle/` segment, not two. Sources:

- `packages/cli/src/bundler/windows.rs:374` joins `"nsis"` to
  `project_out_directory()`.
- `packages/cli/src/build/request.rs:2625-2628` resolves
  `bundle_dir(Windows) = internal_out_dir().join(main_target).join("bundle")`.

## Dioxus Bundle Configuration

`Dioxus.toml` (workspace-root file `./Dioxus.toml`, not per-crate) remains the
central bundle configuration file. The existing application name, bundle
identifier, and icon list stay in place. The implementation adds Windows
settings that are useful for the NSIS installer without changing runtime
behavior:

- `publisher = "InputForge"` at the bundle level.
- `copyright = "Copyright (c) 2026 InputForge"` at the bundle level. Avoids the
  "Unknown copyright" placeholder in the NSIS installer header.
- `short_description` and `long_description` copied from the workspace package
  description.
- Windows `icon_path` pointing at the existing `.ico` asset. Path is
  workspace-relative, matching the existing `icon = [...]` entries.
- NSIS `install_mode = "CurrentUser"` so the first installer does not require
  administrator rights.
- NSIS `languages = ["English"]` and `display_language_selector = false`.
- NSIS `start_menu_folder = "InputForge"`.

All of these keys are valid `Dioxus.toml` fields in 0.7.9
(`packages/cli/src/config/bundle.rs`: `BundleConfig` for `publisher`,
`copyright`, `short_description`, `long_description`; `NsisSettings` for
`install_mode`, `languages`, `display_language_selector`,
`start_menu_folder`).

The Windows bundling command is:

```powershell
dx bundle --package inputforge-app --platform desktop --release --package-types nsis
```

Flag notes for 0.7.9:

- `--platform`: there is no `--desktop` shorthand in 0.7.9. `desktop` resolves
  to the host's native desktop platform; on a Windows runner this is the
  Windows target. `--platform windows` is also valid and explicit. Source:
  `packages/cli/src/platform.rs:43-61` (`Platform::from_identifier`).
- `--package`: selects the workspace member crate to bundle (`inputforge-app`).
  Source: `packages/cli/src/cli/target.rs:36-38`.
- `--package-types`: accepts the lowercase value `nsis`. Source:
  `packages/cli/src/config/bundle.rs` (`PackageType::Nsis`
  `#[clap(name = "nsis")]`).
- `--release`: inherited from `BuildArgs`.
- Asset discovery: the workflow locates produced assets via the matrix
  `asset_glob` exclusively. Dioxus 0.7.9 has no `--json-output` flag;
  structured output is emitted by internal tracing and is not a stable CI
  parsing target.

## CI Flow

The workflow is split into three layers: a single `test` gate job, a build
matrix that depends on the gate, and a final `publish` job that depends on the
matrix.

Workflow-level configuration:

- `concurrency: { group: release-${{ github.ref }}, cancel-in-progress: false }`
  prevents two pushes of the same tag from racing to create duplicate
  releases. Release builds complete rather than abort mid-flight.
- Top-level `permissions: { contents: read }` is the default; the `publish`
  job overrides to `permissions: { contents: write }`. Other permissions stay
  default-deny.

All jobs check out the repository with `actions/checkout@v4` and
`fetch-depth: 0` so the version-match step can resolve annotated tags and the
publish step has full history available.

External actions use maintained version tags such as `actions/checkout@v4`,
`Swatinem/rust-cache@v2`, and `softprops/action-gh-release@v2`. Full commit
SHA pinning is intentionally out of scope for the first milestone.

**`test` job** (runs once, gates the matrix):

1. Check out the repository.
2. Verify the tag version matches the workspace package version (strip leading
   `v`, compare full remainder to `[workspace.package].version`).
3. Install the Rust toolchain required by the workspace.
4. Restore Cargo cache via `Swatinem/rust-cache@v2`.
5. Run `cargo test --workspace`.

**`build` matrix job** (one row per platform, `needs: test`):

1. Check out the repository.
2. Install the Rust toolchain.
3. Install Dioxus CLI 0.7.9:
   `cargo binstall --no-confirm --locked dioxus-cli@0.7.9`.
   Fallback when binstall is unavailable:
   `cargo install dioxus-cli --version 0.7.9 --locked`.
4. Restore Cargo cache via `Swatinem/rust-cache@v2`. The Dioxus bundle output
   cache (`target/dx`) is left as a future optimization once cold-build cost
   data is collected.
5. Run `dx bundle --package inputforge-app --platform ${{ matrix.platform }} --release --package-types ${{ matrix.package_type }}`.
6. Locate produced installer assets with the matrix `asset_glob` value.
7. Sign the installer when signing secrets are configured (see Optional
   Signing).
8. Generate a SHA-256 checksum file for the installer, named
   `<installer-filename>.sha256`.
9. Upload the installer and checksum as workflow artifacts with
   `retention-days: 7`. Artifacts only need to live until the publish job
   consumes them.

**`publish` job** (runs once after the matrix succeeds):

- Downloads all matrix artifacts.
- Creates or updates the GitHub Release for the tag with `prerelease` set
  according to the Release Trigger rules.
- Uploads the assets.

Splitting tests from the build matrix means `cargo test` runs once per release
rather than once per platform, which keeps wall-clock cost down when macOS and
Linux rows are added. Splitting publish from build keeps release publishing
free of platform-specific logic.

## Optional Signing

The first workflow must not require signing secrets. Gating happens at the
**step level inside the `build` job**, using
`if: ${{ env.WINDOWS_SIGNING_CERTIFICATE_BASE64 != '' }}` on each signing
step after projecting the secret into the build job environment. Direct
`secrets.*` references in `if:` conditionals and cross-job gating on secret
presence must not be used.

If the expected signing secrets are absent, the gating expression evaluates
false, the signing steps are skipped, and the installer ships unsigned. If all
required signing secrets are present, the steps run and sign the installer
before checksum generation so the published checksum matches the distributed
asset.

Use these secret names for the optional path:

- `WINDOWS_SIGNING_CERTIFICATE_BASE64`: base64-encoded `.pfx` certificate.
- `WINDOWS_SIGNING_CERTIFICATE_PASSWORD`: password for the `.pfx`.
- `WINDOWS_SIGNING_TIMESTAMP_URL`: optional timestamp server URL, defaulting to
  `http://timestamp.digicert.com` when omitted.

The implementation signs the generated `.exe` after `dx bundle` with Windows
`signtool.exe`. Keep Dioxus's `sign_command` unset for the first
implementation so unsigned builds and signed builds share the same bundle
configuration; CI owns the optional signing branch.

## Extensibility For Updates

The workflow deliberately does not create updater assets yet. It prepares for
that work by:

- Publishing deterministic, versioned release assets. The checksum is named
  `<installer-filename>.sha256` so the asset URL pattern is stable for a
  future updater client.
- Keeping package type and platform details in matrix data.
- Separating platform build jobs from release publication.
- Leaving room to add Dioxus's `updater` package type as a future matrix row or
  companion artifact once the app-side updater behavior is designed.

## Testing And Verification

Implementation verifies local scripts and workflow helpers where possible,
then runs the repository test suite. The actual release upload requires
pushing a matching RC semver tag for the first publish-style rehearsal. The
Windows bundle can also be verified locally
with:

```powershell
dx bundle --package inputforge-app --platform desktop --release --package-types nsis
```

Success criteria for the first milestone:

- Workspace `Cargo.toml` lists `dioxus = "0.7.9"` and `dioxus-ssr = "0.7.9"`,
  and `cargo build -p inputforge-app` and `cargo test --workspace` succeed.
- A tag like `v0.1.0` builds only if it matches the workspace version.
- Pre-release tags like `v0.1.0-rc.1` produce a GitHub Release marked
  `prerelease: true`.
- The first publish-style rehearsal uses `v0.1.0-rc.1` and documents cleanup
  for the rehearsal release and tag.
- GitHub Actions produces a Windows NSIS `.exe` installer at
  `target/dx/inputforge-app/bundle/nsis/InputForge_<version>_x64-setup.exe`.
- The installer and matching `.sha256` are attached to the GitHub Release.
- Workflow validation includes PowerShell assertions and `actionlint` v1.7.12.
- After a test tag publishes, the installer runs on a clean Windows machine
  (or VM/sandbox) and the InputForge app launches successfully.
- The workflow remains easy to extend by adding new matrix rows.
