# Release Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a tag-triggered GitHub Actions release pipeline that validates the release version, bundles the Windows Dioxus desktop app as an NSIS installer, and publishes the installer plus checksum to a GitHub Release.

**Architecture:** Keep release behavior in a small set of repository-owned files: dependency versions in the workspace manifest, bundle metadata in the root `Dioxus.toml`, PowerShell helpers under `.github/scripts`, and the orchestration workflow in `.github/workflows/release.yml`. The workflow is split into `test`, matrix `build`, and `publish` jobs so tests run once, platform packaging can grow by adding matrix rows, and release publishing stays platform-neutral.

**Tech Stack:** Rust workspace, Cargo, Dioxus CLI 0.7.9, PowerShell 7 on GitHub Actions, GitHub Actions artifacts/releases, Windows `signtool.exe` for optional signing.

---

## File Structure

- Modify `Cargo.toml`: bump `[workspace.dependencies]` `dioxus` and `dioxus-ssr` from `0.7.6` to `0.7.9`.
- Modify `Cargo.lock`: refresh locked Dioxus crates after the manifest bump.
- Modify `Dioxus.toml`: add bundle metadata and Windows NSIS settings while preserving the existing app name, identifier, and icon list.
- Create `.github/scripts/validate-release-tag.ps1`: validate `github.ref_name`/`github.ref` against the workspace version and expose `release_version` and `is_prerelease` outputs.
- Create `.github/scripts/test-release-tag.ps1`: local PowerShell tests for the tag validation helper.
- Create `.github/scripts/assert-release-workflow.ps1`: static checks for the release workflow invariants that are easy to regress.
- Create `.github/workflows/release.yml`: tag-triggered release workflow with `test`, `build`, and `publish` jobs.

Notes for implementers:

- GitHub tag filters are glob patterns, not full regexes. Use a broad `v*.*.*` trigger and enforce the exact semver shape in `.github/scripts/validate-release-tag.ps1` before tests, builds, or publishing run.
- GitHub's current documentation says secrets cannot be referenced directly in `if:` conditionals. Keep signing optional at the step level by projecting secrets into job `env` and using `if: ${{ env.WINDOWS_SIGNING_CERTIFICATE_BASE64 != '' }}` on signing steps.
- The Dioxus CLI target in this plan is `0.7.9`; crates.io currently reports `dioxus = "0.7.9"` and `dioxus-ssr = "0.7.9"` as latest stable.

---

### Task 1: Bump Dioxus Workspace Dependencies

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`

- [ ] **Step 1: Verify the intended Cargo versions**

Run:

```powershell
cargo search dioxus --limit 5
cargo search dioxus-ssr --limit 5
```

Expected output includes:

```text
dioxus = "0.7.9"
dioxus-ssr = "0.7.9"
```

- [ ] **Step 2: Update the workspace GUI dependency versions**

In `Cargo.toml`, replace the existing GUI dependency lines with:

```toml
# GUI
dioxus = "0.7.9"
dioxus-ssr = "0.7.9"
scraper = "0.26.0"
arboard = { version = "3.6.1", default-features = false }
```

- [ ] **Step 3: Refresh the lockfile for the bumped crates**

Run:

```powershell
cargo update -p dioxus -p dioxus-ssr
```

Expected: `Cargo.lock` updates Dioxus-related package entries without changing unrelated workspace manifests.

- [ ] **Step 4: Build the desktop app package**

Run:

```powershell
cargo build -p inputforge-app
```

Expected: command exits `0`.

- [ ] **Step 5: Run the workspace tests**

Run:

```powershell
cargo test --workspace
```

Expected: command exits `0`.

- [ ] **Step 6: Commit the dependency bump**

Run:

```powershell
git add Cargo.toml Cargo.lock
git commit -m "build(deps): bump dioxus to 0.7.9"
```

---

### Task 2: Configure Dioxus Windows Bundle Metadata

**Files:**
- Modify: `Dioxus.toml`

- [ ] **Step 1: Replace `Dioxus.toml` with the full bundle configuration**

Use this exact file content:

```toml
[application]
name = "InputForge"

[bundle]
identifier = "io.inputforge.app"
publisher = "InputForge"
copyright = "Copyright (c) 2026 InputForge"
short_description = "Remap physical joystick, pedal, and throttle inputs to virtual vJoy devices"
long_description = "Remap physical joystick, pedal, and throttle inputs to virtual vJoy devices"
icon = [
  "crates/inputforge-app/assets/icon-16.png",
  "crates/inputforge-app/assets/icon-24.png",
  "crates/inputforge-app/assets/icon-32.png",
  "crates/inputforge-app/assets/icon-64.png",
  "crates/inputforge-app/assets/icon-256.png",
  "crates/inputforge-app/assets/icon.ico",
]

[bundle.windows]
icon_path = "crates/inputforge-app/assets/icon.ico"

[bundle.windows.nsis]
install_mode = "CurrentUser"
languages = ["English"]
display_language_selector = false
start_menu_folder = "InputForge"
```

- [ ] **Step 2: Verify the Cargo workspace still parses**

Run:

```powershell
cargo metadata --no-deps --format-version 1
```

Expected: command exits `0` and prints workspace metadata JSON.

- [ ] **Step 3: Keep the app build green after bundle config changes**

Run:

```powershell
cargo build -p inputforge-app
```

Expected: command exits `0`.

- [ ] **Step 4: Validate the Dioxus bundle configuration on Windows**

Run this on Windows with Dioxus CLI 0.7.9 available:

```powershell
dx bundle --package inputforge-app --platform desktop --release --package-types nsis
```

Expected: command exits `0` and creates one NSIS installer under `target/dx/**/bundle/nsis/*.exe`.

- [ ] **Step 5: Commit the bundle configuration**

Run:

```powershell
git add Dioxus.toml
git commit -m "build(bundle): configure windows nsis metadata"
```

---

### Task 3: Add Release Tag Helper Tests

**Files:**
- Create: `.github/scripts/test-release-tag.ps1`

- [ ] **Step 1: Write the failing helper test file**

Create `.github/scripts/test-release-tag.ps1`:

```powershell
[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $PSCommandPath
$validator = Join-Path $scriptDir "validate-release-tag.ps1"

function New-TestManifest {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Version
    )

    $tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $tempDir | Out-Null

    $manifestPath = Join-Path $tempDir "Cargo.toml"
    @"
[workspace]
members = ["crates/inputforge-app"]
resolver = "2"

[workspace.package]
version = "$Version"
edition = "2024"
"@ | Set-Content -LiteralPath $manifestPath -Encoding utf8

    return @{
        TempDir = $tempDir
        ManifestPath = $manifestPath
    }
}

function Invoke-Validator {
    param(
        [Parameter(Mandatory = $true)]
        [string] $TagRef,

        [Parameter(Mandatory = $true)]
        [string] $ManifestPath
    )

    $outputPath = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString("N"))
    & $validator -TagRef $TagRef -ManifestPath $ManifestPath -GitHubOutputPath $outputPath

    return @{
        ExitCode = $LASTEXITCODE
        OutputPath = $outputPath
        Output = if (Test-Path -LiteralPath $outputPath) { Get-Content -LiteralPath $outputPath -Raw } else { "" }
    }
}

function Assert-Contains {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Haystack,

        [Parameter(Mandatory = $true)]
        [string] $Needle
    )

    if (-not $Haystack.Contains($Needle)) {
        throw "Expected output to contain '$Needle', got: $Haystack"
    }
}

function Assert-Throws {
    param(
        [Parameter(Mandatory = $true)]
        [scriptblock] $ScriptBlock,

        [Parameter(Mandatory = $true)]
        [string] $MessageFragment
    )

    try {
        & $ScriptBlock
    }
    catch {
        if ($_.Exception.Message.Contains($MessageFragment)) {
            return
        }

        throw "Expected error containing '$MessageFragment', got: $($_.Exception.Message)"
    }

    throw "Expected command to throw '$MessageFragment'"
}

$stable = New-TestManifest -Version "0.1.0"
$stableResult = Invoke-Validator -TagRef "refs/tags/v0.1.0" -ManifestPath $stable.ManifestPath
Assert-Contains -Haystack $stableResult.Output -Needle "release_version=0.1.0"
Assert-Contains -Haystack $stableResult.Output -Needle "is_prerelease=false"

$prerelease = New-TestManifest -Version "0.1.0-rc.1"
$prereleaseResult = Invoke-Validator -TagRef "v0.1.0-rc.1" -ManifestPath $prerelease.ManifestPath
Assert-Contains -Haystack $prereleaseResult.Output -Needle "release_version=0.1.0-rc.1"
Assert-Contains -Haystack $prereleaseResult.Output -Needle "is_prerelease=true"

$mismatch = New-TestManifest -Version "0.1.0"
Assert-Throws -MessageFragment "does not match workspace version" -ScriptBlock {
    Invoke-Validator -TagRef "v0.2.0" -ManifestPath $mismatch.ManifestPath
}

$invalid = New-TestManifest -Version "0.1.0"
Assert-Throws -MessageFragment "is not a supported semver release tag" -ScriptBlock {
    Invoke-Validator -TagRef "release-0.1.0" -ManifestPath $invalid.ManifestPath
}

Write-Host "release tag helper tests passed"
```

- [ ] **Step 2: Run the test to verify it fails because the helper does not exist**

Run:

```powershell
pwsh -NoLogo -NoProfile -File .github/scripts/test-release-tag.ps1
```

Expected: FAIL with a message that `.github/scripts/validate-release-tag.ps1` is not recognized or cannot be found.

---

### Task 4: Add Release Tag Validation Helper

**Files:**
- Create: `.github/scripts/validate-release-tag.ps1`
- Test: `.github/scripts/test-release-tag.ps1`

- [ ] **Step 1: Create the validation helper**

Create `.github/scripts/validate-release-tag.ps1`:

```powershell
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string] $TagRef,

    [string] $ManifestPath = "Cargo.toml",

    [string] $GitHubOutputPath = $env:GITHUB_OUTPUT
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$tagName = Split-Path -Leaf $TagRef
$tagMatch = [regex]::Match(
    $tagName,
    '^v(?<version>[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?)$'
)

if (-not $tagMatch.Success) {
    throw "Tag '$tagName' is not a supported semver release tag. Use v0.1.0 or v0.1.0-rc.1."
}

$tagVersion = $tagMatch.Groups["version"].Value

if (-not (Test-Path -LiteralPath $ManifestPath)) {
    throw "Manifest '$ManifestPath' does not exist."
}

$manifest = Get-Content -LiteralPath $ManifestPath -Raw
$workspacePackageMatch = [regex]::Match(
    $manifest,
    '(?ms)^\[workspace\.package\]\s*(?<body>.*?)(?=^\[|\z)'
)

if (-not $workspacePackageMatch.Success) {
    throw "Cargo.toml is missing [workspace.package]."
}

$versionMatch = [regex]::Match(
    $workspacePackageMatch.Groups["body"].Value,
    '(?m)^\s*version\s*=\s*"(?<version>[^"]+)"\s*$'
)

if (-not $versionMatch.Success) {
    throw "Cargo.toml is missing [workspace.package].version."
}

$workspaceVersion = $versionMatch.Groups["version"].Value

if ($tagVersion -ne $workspaceVersion) {
    throw "Tag version '$tagVersion' does not match workspace version '$workspaceVersion'."
}

$isPrerelease = if ($tagVersion.Contains("-")) { "true" } else { "false" }

if ($GitHubOutputPath) {
    Add-Content -LiteralPath $GitHubOutputPath -Encoding utf8 -Value "release_version=$tagVersion"
    Add-Content -LiteralPath $GitHubOutputPath -Encoding utf8 -Value "is_prerelease=$isPrerelease"
}

Write-Host "Release tag '$tagName' matches workspace version '$workspaceVersion'."
Write-Host "Prerelease: $isPrerelease"
```

- [ ] **Step 2: Run the helper tests**

Run:

```powershell
pwsh -NoLogo -NoProfile -File .github/scripts/test-release-tag.ps1
```

Expected: PASS with `release tag helper tests passed`.

- [ ] **Step 3: Run the helper against the current workspace version**

Run:

```powershell
pwsh -NoLogo -NoProfile -File .github/scripts/validate-release-tag.ps1 -TagRef "refs/tags/v0.1.0" -ManifestPath Cargo.toml
```

Expected:

```text
Release tag 'v0.1.0' matches workspace version '0.1.0'.
Prerelease: false
```

- [ ] **Step 4: Commit the release helper**

Run:

```powershell
git add .github/scripts/validate-release-tag.ps1 .github/scripts/test-release-tag.ps1
git commit -m "ci(release): add release tag validation"
```

---

### Task 5: Add Workflow Static Assertions

**Files:**
- Create: `.github/scripts/assert-release-workflow.ps1`

- [ ] **Step 1: Write the failing workflow assertion script**

Create `.github/scripts/assert-release-workflow.ps1`:

```powershell
[CmdletBinding()]
param(
    [string] $WorkflowPath = ".github/workflows/release.yml"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $WorkflowPath)) {
    throw "Workflow '$WorkflowPath' does not exist."
}

$workflow = Get-Content -LiteralPath $WorkflowPath -Raw

function Assert-WorkflowContains {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Text,

        [Parameter(Mandatory = $true)]
        [string] $Reason
    )

    if (-not $workflow.Contains($Text)) {
        throw "Workflow assertion failed: $Reason. Missing text: $Text"
    }
}

function Assert-WorkflowDoesNotContain {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Text,

        [Parameter(Mandatory = $true)]
        [string] $Reason
    )

    if ($workflow.Contains($Text)) {
        throw "Workflow assertion failed: $Reason. Unexpected text: $Text"
    }
}

$actionlintVersion = "v1.7.12"
go run "github.com/rhysd/actionlint/cmd/actionlint@$actionlintVersion" $WorkflowPath
if ($LASTEXITCODE -ne 0) {
    throw "actionlint $actionlintVersion failed for '$WorkflowPath'."
}

Assert-WorkflowContains -Text "on:" -Reason "workflow declares triggers"
Assert-WorkflowContains -Text "  push:" -Reason "workflow runs only from push events"
Assert-WorkflowContains -Text "    tags:" -Reason "workflow is tag-triggered"
Assert-WorkflowContains -Text '      - "v*.*.*"' -Reason "workflow accepts semver-style release tags"
Assert-WorkflowDoesNotContain -Text "workflow_dispatch:" -Reason "workflow is not manually publishable"
Assert-WorkflowDoesNotContain -Text "branches:" -Reason "workflow does not publish from branches"
Assert-WorkflowContains -Text 'release-${{ github.ref }}' -Reason "release concurrency is keyed by tag ref"
Assert-WorkflowContains -Text "cancel-in-progress: false" -Reason "release builds are not cancelled mid-flight"
Assert-WorkflowContains -Text "contents: write" -Reason "publish job can create or update releases"
Assert-WorkflowContains -Text "DIOXUS_CLI_VERSION: 0.7.9" -Reason "workflow pins the target Dioxus CLI"
Assert-WorkflowContains -Text "cargo test --workspace" -Reason "test job gates release builds"
Assert-WorkflowContains -Text "windows-latest" -Reason "initial matrix includes Windows"
Assert-WorkflowContains -Text "package_type: nsis" -Reason "initial matrix bundles NSIS"
Assert-WorkflowContains -Text "target/dx/**/bundle/nsis/*.exe" -Reason "asset glob matches Dioxus 0.7.9 NSIS output"
Assert-WorkflowContains -Text 'Get-ChildItem -Path "${{ matrix.asset_glob }}" -File' -Reason "installer discovery consumes the matrix asset glob"
Assert-WorkflowContains -Text "softprops/action-gh-release@v2" -Reason "publish job uploads release assets"
Assert-WorkflowContains -Text "env.WINDOWS_SIGNING_CERTIFICATE_BASE64 != ''" -Reason "signing is optional at step level without direct secrets in if"
Assert-WorkflowContains -Text "Get-FileHash -Algorithm SHA256" -Reason "build job writes a SHA-256 checksum"

Write-Host "release workflow assertions passed"
```

- [ ] **Step 2: Run the assertion script to verify it fails because the workflow does not exist**

Run:

```powershell
pwsh -NoLogo -NoProfile -File .github/scripts/assert-release-workflow.ps1
```

Expected: FAIL with `Workflow '.github/workflows/release.yml' does not exist.` before actionlint runs.

---

### Task 6: Add Tag-Triggered Release Workflow

**Files:**
- Create: `.github/workflows/release.yml`
- Test: `.github/scripts/assert-release-workflow.ps1`

- [ ] **Step 1: Create the release workflow**

Security note: this workflow intentionally uses maintained action version tags
such as `actions/checkout@v4` and `softprops/action-gh-release@v2`. Full
commit SHA pinning is not required for this milestone by project decision.

Create `.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags:
      - "v*.*.*"

concurrency:
  group: release-${{ github.ref }}
  cancel-in-progress: false

permissions:
  contents: read

env:
  CARGO_TERM_COLOR: always
  DIOXUS_CLI_VERSION: 0.7.9

jobs:
  test:
    name: Test release gate
    runs-on: windows-latest
    outputs:
      release_version: ${{ steps.version.outputs.release_version }}
      is_prerelease: ${{ steps.version.outputs.is_prerelease }}
    steps:
      - name: Checkout
        uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Install Rust
        uses: dtolnay/rust-toolchain@1.85.0

      - name: Validate release tag
        id: version
        shell: pwsh
        run: .github/scripts/validate-release-tag.ps1 -TagRef "${{ github.ref }}" -ManifestPath Cargo.toml

      - name: Restore Cargo cache
        uses: Swatinem/rust-cache@v2

      - name: Test release helper
        shell: pwsh
        run: pwsh -NoLogo -NoProfile -File .github/scripts/test-release-tag.ps1

      - name: Run workspace tests
        run: cargo test --workspace

  build:
    name: Bundle ${{ matrix.platform }}
    needs: test
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: windows-latest
            platform: windows
            package_type: nsis
            asset_glob: target/dx/**/bundle/nsis/*.exe
    env:
      WINDOWS_SIGNING_CERTIFICATE_BASE64: ${{ secrets.WINDOWS_SIGNING_CERTIFICATE_BASE64 }}
      WINDOWS_SIGNING_CERTIFICATE_PASSWORD: ${{ secrets.WINDOWS_SIGNING_CERTIFICATE_PASSWORD }}
      WINDOWS_SIGNING_TIMESTAMP_URL: ${{ secrets.WINDOWS_SIGNING_TIMESTAMP_URL }}
    steps:
      - name: Checkout
        uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Install Rust
        uses: dtolnay/rust-toolchain@1.85.0

      - name: Restore Cargo cache
        uses: Swatinem/rust-cache@v2

      - name: Install Dioxus CLI
        shell: pwsh
        run: |
          cargo install cargo-binstall --locked
          if ($LASTEXITCODE -eq 0) {
            cargo binstall --no-confirm --locked "dioxus-cli@${{ env.DIOXUS_CLI_VERSION }}"
          }
          if ($LASTEXITCODE -ne 0) {
            cargo install dioxus-cli --version "${{ env.DIOXUS_CLI_VERSION }}" --locked
          }

      - name: Bundle desktop app
        shell: pwsh
        run: |
          dx bundle --package inputforge-app --platform "${{ matrix.platform }}" --release --package-types "${{ matrix.package_type }}"

      - name: Locate installer
        id: installer
        shell: pwsh
        run: |
          $matches = @(Get-ChildItem -Path "${{ matrix.asset_glob }}" -File | Sort-Object FullName)

          if ($matches.Count -ne 1) {
            $found = $matches | ForEach-Object { $_.FullName } | Out-String
            throw "Expected exactly one installer matching '${{ matrix.asset_glob }}', found $($matches.Count): $found"
          }

          $installer = $matches[0]
          "installer_path=$($installer.FullName)" >> $env:GITHUB_OUTPUT
          "installer_name=$($installer.Name)" >> $env:GITHUB_OUTPUT

      - name: Import signing certificate
        if: ${{ env.WINDOWS_SIGNING_CERTIFICATE_BASE64 != '' }}
        id: signing_cert
        shell: pwsh
        run: |
          if (-not $env:WINDOWS_SIGNING_CERTIFICATE_PASSWORD) {
            throw "WINDOWS_SIGNING_CERTIFICATE_PASSWORD is required when WINDOWS_SIGNING_CERTIFICATE_BASE64 is set."
          }

          $certPath = Join-Path $env:RUNNER_TEMP "inputforge-signing.pfx"
          [System.IO.File]::WriteAllBytes($certPath, [Convert]::FromBase64String($env:WINDOWS_SIGNING_CERTIFICATE_BASE64))
          "certificate_path=$certPath" >> $env:GITHUB_OUTPUT

      - name: Sign installer
        if: ${{ env.WINDOWS_SIGNING_CERTIFICATE_BASE64 != '' }}
        shell: pwsh
        env:
          INSTALLER_PATH: ${{ steps.installer.outputs.installer_path }}
          CERTIFICATE_PATH: ${{ steps.signing_cert.outputs.certificate_path }}
        run: |
          $timestampUrl = if ($env:WINDOWS_SIGNING_TIMESTAMP_URL) {
            $env:WINDOWS_SIGNING_TIMESTAMP_URL
          } else {
            "http://timestamp.digicert.com"
          }

          $signtool = Get-ChildItem -Path "${env:ProgramFiles(x86)}\Windows Kits\10\bin" -Recurse -File -Filter signtool.exe |
            Where-Object { $_.FullName -match "\\x64\\signtool.exe$" } |
            Sort-Object FullName -Descending |
            Select-Object -First 1

          if (-not $signtool) {
            throw "signtool.exe was not found in the Windows Kits installation."
          }

          & $signtool.FullName sign /fd SHA256 /f "$env:CERTIFICATE_PATH" /p "$env:WINDOWS_SIGNING_CERTIFICATE_PASSWORD" /tr "$timestampUrl" /td SHA256 "$env:INSTALLER_PATH"

      - name: Generate checksum
        id: checksum
        shell: pwsh
        env:
          INSTALLER_PATH: ${{ steps.installer.outputs.installer_path }}
          INSTALLER_NAME: ${{ steps.installer.outputs.installer_name }}
        run: |
          $checksumPath = "$env:INSTALLER_PATH.sha256"
          $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath "$env:INSTALLER_PATH").Hash.ToLowerInvariant()
          "$hash  $env:INSTALLER_NAME" | Set-Content -LiteralPath "$checksumPath" -Encoding ascii -NoNewline
          "checksum_path=$checksumPath" >> $env:GITHUB_OUTPUT

      - name: Upload release artifact
        uses: actions/upload-artifact@v4
        with:
          name: inputforge-${{ matrix.platform }}-${{ needs.test.outputs.release_version }}
          retention-days: 7
          if-no-files-found: error
          path: |
            ${{ steps.installer.outputs.installer_path }}
            ${{ steps.checksum.outputs.checksum_path }}

  publish:
    name: Publish GitHub Release
    needs:
      - test
      - build
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - name: Download release artifacts
        uses: actions/download-artifact@v4
        with:
          path: release-assets
          merge-multiple: true

      - name: Publish release
        uses: softprops/action-gh-release@v2
        with:
          tag_name: ${{ github.ref_name }}
          name: ${{ github.ref_name }}
          prerelease: ${{ needs.test.outputs.is_prerelease == 'true' }}
          files: release-assets/**/*
```

- [ ] **Step 2: Run the workflow assertions**

Run:

```powershell
pwsh -NoLogo -NoProfile -File .github/scripts/assert-release-workflow.ps1
```

Expected: PASS with `release workflow assertions passed`; the script also runs `actionlint` v1.7.12 against `.github/workflows/release.yml`.

- [ ] **Step 3: Run the release helper tests again**

Run:

```powershell
pwsh -NoLogo -NoProfile -File .github/scripts/test-release-tag.ps1
```

Expected: PASS with `release tag helper tests passed`.

- [ ] **Step 4: Run the repository test suite**

Run:

```powershell
cargo test --workspace
```

Expected: command exits `0`.

- [ ] **Step 5: Commit the workflow**

Run:

```powershell
git add .github/workflows/release.yml .github/scripts/assert-release-workflow.ps1
git commit -m "ci(release): add windows release workflow"
```

---

### Task 7: Verify the Bundle Command on Windows

**Files:**
- Modify: none expected unless verification exposes a real config problem.

- [ ] **Step 1: Install Dioxus CLI 0.7.9 locally if needed**

Run:

```powershell
dx --version
```

If `dx` is missing or is not `0.7.9`, run:

```powershell
cargo install dioxus-cli --version 0.7.9 --locked
```

Expected: `dx --version` reports `dioxus-cli 0.7.9` or equivalent 0.7.9 CLI output.

- [ ] **Step 2: Run the Windows NSIS bundle command**

Run on Windows:

```powershell
dx bundle --package inputforge-app --platform desktop --release --package-types nsis
```

Expected: command exits `0`.

- [ ] **Step 3: Confirm the installer path matches the workflow glob**

Run:

```powershell
Get-ChildItem -Path target/dx -Recurse -File -Filter *.exe |
  Where-Object { $_.FullName -match "\\bundle\\nsis\\" } |
  Select-Object -ExpandProperty FullName
```

Expected output contains one installer path shaped like:

```text
target\dx\inputforge-app\bundle\nsis\InputForge_0.1.0_x64-setup.exe
```

- [ ] **Step 4: Generate and inspect a local checksum**

Run:

```powershell
$installer = Get-ChildItem -Path target/dx -Recurse -File -Filter *.exe |
  Where-Object { $_.FullName -match "\\bundle\\nsis\\" } |
  Select-Object -First 1
$hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $installer.FullName).Hash.ToLowerInvariant()
"$hash  $($installer.Name)" | Set-Content -LiteralPath "$($installer.FullName).sha256" -Encoding ascii -NoNewline
Get-Content -LiteralPath "$($installer.FullName).sha256"
```

Expected: output is one lowercase SHA-256 hash followed by two spaces and the installer filename.

- [ ] **Step 5: Commit fixes only if verification required edits**

If Task 7 required no file changes, do not create an empty commit.

If Task 7 required file changes, run:

```powershell
git add Dioxus.toml .github/workflows/release.yml .github/scripts
git commit -m "fix(release): correct windows bundle verification"
```

---

### Task 8: Verify a Test Release Tag

**Files:**
- Modify: `Cargo.toml`, `Cargo.lock`
- Modify if needed: `.github/workflows/release.yml`, `.github/scripts/*`

- [ ] **Step 1: Push the branch with all release-pipeline commits**

Run:

```powershell
git status --short
git push
```

Expected: branch push succeeds and `git status --short` is clean before creating the tag.

- [ ] **Step 2: Prepare the first RC version**

Run only when the branch is ready to publish a rehearsal release:

First set `[workspace.package].version` to the RC version:

```toml
[workspace.package]
version = "0.1.0-rc.1"
```

Then run:

```powershell
git add Cargo.toml Cargo.lock
git commit -m "chore(release): prepare 0.1.0-rc.1"
```

Expected: workspace version is committed as `0.1.0-rc.1`.

- [ ] **Step 3: Create and push the RC rehearsal tag**

```powershell
git tag v0.1.0-rc.1
git push origin HEAD
git push origin v0.1.0-rc.1
```

Expected: GitHub Actions starts the `Release` workflow for `refs/tags/v0.1.0-rc.1`.

- [ ] **Step 4: Confirm release outputs in GitHub**

Check the completed workflow run:

```text
test: passes after validating v0.1.0-rc.1 against workspace version 0.1.0-rc.1
build / Bundle windows: uploads inputforge-windows-0.1.0-rc.1 artifact
publish: creates or updates the v0.1.0-rc.1 GitHub Release as a prerelease
```

Expected release assets:

```text
InputForge_0.1.0-rc.1_x64-setup.exe
InputForge_0.1.0-rc.1_x64-setup.exe.sha256
```

- [ ] **Step 5: Smoke-test the installer**

Download the installer from the GitHub Release onto a clean Windows machine, VM, or sandbox. Run the installer as a normal user.

Expected:

```text
The installer does not require administrator elevation.
The Start Menu folder is InputForge.
InputForge launches successfully after installation.
```

- [ ] **Step 6: Clean up the rehearsal release when it is not the release candidate to keep**

Run only when `v0.1.0-rc.1` was a disposable rehearsal and should not remain published:

```powershell
gh release delete v0.1.0-rc.1 --yes
git push origin :refs/tags/v0.1.0-rc.1
git tag -d v0.1.0-rc.1
```

Expected: the GitHub Release, remote tag, and local tag for `v0.1.0-rc.1` are removed.

---

## Self-Review

Spec coverage:

- Tag-only release trigger: covered by Task 6 and exact semver enforcement in Tasks 3-4.
- Version/tag match before bundling: covered by Tasks 3-4 and the `test` job in Task 6.
- Dioxus dependency/CLI pin to 0.7.9: covered by Tasks 1 and 6.
- Windows NSIS bundle: covered by Tasks 2, 6, and 7.
- Installer plus SHA-256 release assets: covered by Tasks 6, 7, and 8.
- Optional signing without requiring secrets: covered by Task 6 using step-level `env.*` gating.
- Matrix-shaped workflow: covered by Task 6.
- No runtime updater implementation: preserved; only deterministic assets and matrix structure are added.

Placeholder scan:

- No placeholder red-flag phrases or unspecified validation steps remain.
- Every created file includes full content.
- Every verification step has an exact command and expected result.

Type and name consistency:

- Workflow outputs use `release_version` and `is_prerelease` consistently from `.github/scripts/validate-release-tag.ps1` through `test.outputs`, artifact names, and release prerelease handling.
- Signing uses `WINDOWS_SIGNING_CERTIFICATE_BASE64`, `WINDOWS_SIGNING_CERTIFICATE_PASSWORD`, and `WINDOWS_SIGNING_TIMESTAMP_URL` consistently.
- The NSIS asset discovery uses `matrix.asset_glob`, matching the matrix and assertions.
