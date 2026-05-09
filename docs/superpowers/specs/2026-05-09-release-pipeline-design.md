# Release Pipeline Design

## Context

InputForge is a Rust workspace with a Dioxus desktop app entry point in
`inputforge-app`. The repository already has `Dioxus.toml` bundle metadata for
the app name, identifier, and Windows icon assets. There is no existing
`.github` workflow in this worktree.

The first release milestone is a tag-only GitHub Actions pipeline that bundles a
Windows installer users can download from a GitHub Release. Runtime auto-update
checks are intentionally out of scope for this milestone, but the release asset
layout should stay compatible with adding updater assets later.

Relevant Dioxus documentation:

- https://dioxuslabs.com/learn/0.7/tutorial/bundle/
- https://dioxuslabs.com/learn/0.7/guides/deploy/config/

## Goals

- Publish releases only from semver tags such as `v0.1.0`.
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

## Release Trigger

The workflow runs on pushed semver tags matching `v*.*.*`. The tag is the source
of truth for release publication. Before building, CI strips the leading `v` and
compares the resulting version with `[workspace.package].version` in the root
`Cargo.toml`. A mismatch fails the workflow before any release assets are
published.

GitHub Release creation happens after successful matrix builds. The release name
uses the tag name, and release assets are attached to that tag's release.

## Build Matrix

The workflow is matrix-shaped from the first implementation, with only Windows
enabled initially:

```yaml
include:
  - os: windows-latest
    platform: windows
    package_type: nsis
    asset_glob: target/dx/**/bundle/windows/bundle/nsis/*.exe
```

The job body should read runner, package type, and artifact glob values from the
matrix. Future platform rows can add macOS package types such as `dmg` or Linux
package types such as `appimage` and `deb`, plus any platform-specific setup
steps.

## Dioxus Bundle Configuration

`Dioxus.toml` remains the central bundle configuration file. The existing
application name, bundle identifier, and icon list stay in place. The
implementation should add Windows settings that are useful for the NSIS
installer without changing runtime behavior:

- `publisher = "InputForge"` at the bundle level.
- `short_description` and `long_description` copied from the workspace package
  description.
- Windows `icon_path` pointing at the existing `.ico` asset.
- NSIS `install_mode = "CurrentUser"` so the first installer does not require
  administrator rights.
- NSIS `languages = ["English"]` and `display_language_selector = false`.
- NSIS `start_menu_folder = "InputForge"`.

The Windows bundling command is:

```powershell
dx bundle --desktop --release --package-types nsis --json-output
```

The workflow should prefer the Dioxus JSON output for locating generated assets.
If that output is not convenient or stable enough, the matrix `asset_glob` is the
fallback source of truth for upload.

## CI Flow

Each matrix build job should:

1. Check out the repository with full tag history.
2. Verify that the tag version matches the workspace package version.
3. Install the Rust toolchain required by the workspace.
4. Install a Dioxus CLI version compatible with Dioxus 0.7.
5. Restore Cargo and Dioxus build caches where practical.
6. Run `cargo test --workspace`.
7. Run `dx bundle --desktop --release --package-types nsis --json-output`.
8. Sign the installer only when signing secrets are configured.
9. Generate a SHA-256 checksum file for the installer.
10. Upload the installer and checksum as workflow artifacts.

A final publish job should depend on all matrix build jobs, download their
artifacts, create or update the GitHub Release for the tag, and upload the
assets. This keeps release publishing separate from platform-specific build
logic and will scale better when macOS and Linux rows are added.

## Optional Signing

The first workflow must not require signing secrets. If the expected signing
secrets are absent, the job leaves the installer unsigned and continues. If all
required signing secrets are present, the job signs the installer before
checksum generation so the checksum matches the distributed asset.

Use these secret names for the optional path:

- `WINDOWS_SIGNING_CERTIFICATE_BASE64`: base64-encoded `.pfx` certificate.
- `WINDOWS_SIGNING_CERTIFICATE_PASSWORD`: password for the `.pfx`.
- `WINDOWS_SIGNING_TIMESTAMP_URL`: optional timestamp server URL, defaulting to
  `http://timestamp.digicert.com` when omitted.

The implementation should sign the generated `.exe` after `dx bundle` with
Windows `signtool.exe`. Keep Dioxus's `sign_command` unset for the first
implementation so unsigned builds and signed builds share the same bundle
configuration; CI owns the optional signing branch.

## Extensibility For Updates

The workflow deliberately does not create updater assets yet. It prepares for
that work by:

- Publishing deterministic, versioned release assets.
- Keeping package type and platform details in matrix data.
- Separating platform build jobs from release publication.
- Leaving room to add Dioxus's `updater` package type as a future matrix row or
  companion artifact once the app-side updater behavior is designed.

## Testing And Verification

Implementation should verify the local scripts and workflow helpers where
possible, then run the repository test suite. The actual release upload requires
pushing a test semver tag. The Windows bundle can also be verified locally with:

```powershell
dx bundle --desktop --release --package-types nsis
```

Success criteria for the first milestone:

- A tag like `v0.1.0` builds only if it matches the workspace version.
- GitHub Actions produces a Windows NSIS `.exe` installer.
- The installer and `.sha256` file are attached to the GitHub Release.
- The workflow remains easy to extend by adding new matrix rows.
