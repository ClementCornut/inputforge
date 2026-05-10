# Release Pipeline Implementation Notes

> **Status:** Final release-branch shape after review fixes. The earlier
> temporary PowerShell test/assertion scripts were useful while developing the
> workflow, but they are not part of the final release pipeline.

**Goal:** Publish tag-triggered Windows releases for InputForge with a Dioxus
NSIS installer and SHA-256 checksum.

**Architecture:** The release workflow validates the pushed tag against
`[workspace.package].version`, runs locked workspace tests, builds a locked
Dioxus NSIS bundle, optionally signs the final installer, generates a checksum,
and publishes the artifacts to the matching GitHub Release. The NSIS template
keeps prerelease strings visible in installer metadata while deriving a numeric
`VIProductVersion` for Windows.

**Tech Stack:** Rust workspace, Cargo, Dioxus CLI 0.7.9, PowerShell 7,
GitHub Actions artifacts/releases, NSIS, optional Windows `signtool.exe`.

---

## Final File Structure

- `Cargo.toml` / `Cargo.lock`: workspace version and locked dependency graph.
- `Dioxus.toml`: bundle metadata, SDL runtime resource, Windows NSIS settings,
  and `install_mode = "Both"`.
- `.github/scripts/validate-release-tag.ps1`: the only retained PowerShell
  release helper; validates stable SemVer and matching SemVer prerelease tags.
- `.github/workflows/release.yml`: tag-triggered release workflow.
- `scripts/nsis/inputforge.nsi`: project-owned installer template with numeric
  `VIProductVersion` handling.

Deleted dev-only scripts:

- `.github/scripts/test-release-tag.ps1`
- `.github/scripts/assert-release-workflow.ps1`
- `.github/scripts/assert-windows-installer-contract.ps1`

## Current Release Flow

1. A pushed tag matching the broad GitHub filter `v*.*.*` starts the workflow.
2. `validate-release-tag.ps1` enforces the SemVer shape and exact
   `[workspace.package].version` match.
3. The test job runs `cargo test --workspace --locked`.
4. The Windows build job installs Dioxus CLI 0.7.9, verifies `SDL/SDL3.dll`,
   and runs:

   ```powershell
   dx bundle --package inputforge-app --bin inputforge --platform windows --release --package-types nsis --locked
   ```

5. The workflow locates the installer under
   `target/dx/inputforge/bundle/windows/nsis/*.exe`.
6. The installer is renamed to `inputforge_<version>_x64-setup.exe`.
7. Optional signing runs only when signing secrets are configured.
8. The workflow writes `<installer-filename>.sha256`, uploads both files as
   workflow artifacts, then publishes them to the GitHub Release.

## Versioning Contract

- CI must not rewrite `Cargo.toml` or `Cargo.lock` to strip prerelease suffixes.
- `{{version}}` remains the real release version for visible installer
  metadata and asset naming, such as `0.1.0-rc.1`.
- `scripts/nsis/inputforge.nsi` derives `NUMERIC_VERSION` from `RAW_VERSION`
  and uses `VIProductVersion "${NUMERIC_VERSION}.0"` so NSIS/Windows receive a
  numeric product version such as `0.1.0.0`.

## Verification

Use these checks after release workflow edits:

```powershell
pwsh -NoLogo -NoProfile -File .github/scripts/validate-release-tag.ps1 -TagRef "refs/tags/v0.1.0-rc.1" -ManifestPath Cargo.toml
cargo test --workspace --locked
dx bundle --package inputforge-app --bin inputforge --platform windows --release --package-types nsis --locked
git diff --check
```

For bundle inspection, confirm the generated
`target/dx/inputforge/bundle/windows/nsis/Inputforge.nsi` keeps:

- `RAW_VERSION "0.1.0-rc.1"` or the current prerelease version.
- `VIProductVersion "${NUMERIC_VERSION}.0"`.
- `DisplayVersion "0.1.0-rc.1"` or the current prerelease version.
