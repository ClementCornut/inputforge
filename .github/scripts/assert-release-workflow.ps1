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

function Assert-WorkflowDoesNotMatch {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Pattern,

        [Parameter(Mandatory = $true)]
        [string] $Reason
    )

    if ($workflow -match $Pattern) {
        throw "Workflow assertion failed: $Reason. Unexpected pattern: $Pattern"
    }
}

function Assert-WorkflowMatchCount {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Pattern,

        [Parameter(Mandatory = $true)]
        [int] $ExpectedCount,

        [Parameter(Mandatory = $true)]
        [string] $Reason
    )

    $count = [regex]::Matches($workflow, $Pattern).Count
    if ($count -ne $ExpectedCount) {
        throw "Workflow assertion failed: $Reason. Expected $ExpectedCount matches for pattern '$Pattern', found $count."
    }
}

function Get-WorkflowStepBlock {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Name
    )

    $escapedName = [regex]::Escape($Name)
    $match = [regex]::Match($workflow, "(?ms)^      - name: $escapedName\r?\n.*?(?=^      - name: |\z)")
    if (-not $match.Success) {
        throw "Workflow assertion failed: step '$Name' exists."
    }

    return $match.Value
}

function Assert-BlockContains {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Block,

        [Parameter(Mandatory = $true)]
        [string] $Text,

        [Parameter(Mandatory = $true)]
        [string] $Reason
    )

    if (-not $Block.Contains($Text)) {
        throw "Workflow assertion failed: $Reason. Missing text: $Text"
    }
}

function Assert-BlockDoesNotContain {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Block,

        [Parameter(Mandatory = $true)]
        [string] $Text,

        [Parameter(Mandatory = $true)]
        [string] $Reason
    )

    if ($Block.Contains($Text)) {
        throw "Workflow assertion failed: $Reason. Unexpected text: $Text"
    }
}

function Assert-StepBefore {
    param(
        [Parameter(Mandatory = $true)]
        [string] $FirstName,

        [Parameter(Mandatory = $true)]
        [string] $SecondName,

        [Parameter(Mandatory = $true)]
        [string] $Reason
    )

    $firstText = "      - name: $FirstName"
    $secondText = "      - name: $SecondName"
    $firstIndex = $workflow.IndexOf($firstText, [StringComparison]::Ordinal)
    $secondIndex = $workflow.IndexOf($secondText, [StringComparison]::Ordinal)

    if ($firstIndex -lt 0) {
        throw "Workflow assertion failed: $Reason. Missing step: $FirstName"
    }

    if ($secondIndex -lt 0) {
        throw "Workflow assertion failed: $Reason. Missing step: $SecondName"
    }

    if ($firstIndex -ge $secondIndex) {
        throw "Workflow assertion failed: $Reason. Step '$FirstName' must appear before '$SecondName'."
    }
}

function Invoke-PinnedActionlint {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Version,

        [Parameter(Mandatory = $true)]
        [string] $Path
    )

    if (Get-Command go -ErrorAction SilentlyContinue) {
        go run "github.com/rhysd/actionlint/cmd/actionlint@$Version" $Path
        if ($LASTEXITCODE -ne 0) {
            throw "actionlint $Version failed for '$Path'."
        }

        return
    }

    if (-not ($IsWindows -or $env:OS -eq "Windows_NT")) {
        throw "Go is required to run actionlint $Version on this platform."
    }

    $versionNumber = $Version.TrimStart("v")
    $tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $tempDir | Out-Null

    try {
        $archivePath = Join-Path $tempDir "actionlint.zip"
        $downloadUrl = "https://github.com/rhysd/actionlint/releases/download/$Version/actionlint_${versionNumber}_windows_amd64.zip"
        Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath -UseBasicParsing
        Expand-Archive -LiteralPath $archivePath -DestinationPath $tempDir

        $actionlint = Get-ChildItem -Path $tempDir -Recurse -File -Filter actionlint.exe |
            Select-Object -First 1

        if (-not $actionlint) {
            throw "Downloaded actionlint $Version archive did not contain actionlint.exe."
        }

        & $actionlint.FullName $Path
        if ($LASTEXITCODE -ne 0) {
            throw "actionlint $Version failed for '$Path'."
        }
    } finally {
        Remove-Item -LiteralPath $tempDir -Recurse -Force
    }
}

$actionlintVersion = "v1.7.12"
Invoke-PinnedActionlint -Version $actionlintVersion -Path $WorkflowPath

Assert-WorkflowContains -Text "on:" -Reason "workflow declares triggers"
Assert-WorkflowContains -Text "  push:" -Reason "workflow runs only from push events"
Assert-WorkflowContains -Text "    tags:" -Reason "workflow is tag-triggered"
Assert-WorkflowContains -Text '      - "v*.*.*"' -Reason "workflow accepts semver-style release tags"
Assert-WorkflowDoesNotContain -Text "workflow_dispatch:" -Reason "workflow is not manually publishable"
Assert-WorkflowDoesNotContain -Text "branches:" -Reason "workflow does not publish from branches"
Assert-WorkflowContains -Text 'release-${{ github.ref }}' -Reason "release concurrency is keyed by tag ref"
Assert-WorkflowContains -Text "cancel-in-progress: false" -Reason "release builds are not cancelled mid-flight"
Assert-WorkflowContains -Text "contents: write" -Reason "publish job can create or update releases"
Assert-WorkflowContains -Text "          lfs: true" -Reason "checkout downloads Git LFS binary dependencies"
Assert-WorkflowContains -Text "actions/checkout@v6.0.2" -Reason "checkout uses the latest verified Node 24 release"
Assert-WorkflowDoesNotContain -Text "actions/checkout@v4" -Reason "checkout avoids the deprecated Node 20 action runtime"
Assert-WorkflowContains -Text "Swatinem/rust-cache@v2.9.1" -Reason "Cargo cache action uses the latest verified release"
Assert-WorkflowMatchCount -Pattern "(?m)^      - name: Add Cargo bin to PATH$" -ExpectedCount 2 -Reason "Windows jobs explicitly add Cargo-installed tools to PATH"
Assert-WorkflowMatchCount -Pattern '(?m)^          Join-Path \$env:CARGO_HOME "bin" >> \$env:GITHUB_PATH$' -ExpectedCount 2 -Reason "Cargo bin PATH update uses CARGO_HOME in both Windows jobs"
Assert-WorkflowContains -Text "DIOXUS_CLI_VERSION: 0.7.9" -Reason "workflow pins the target Dioxus CLI"
Assert-WorkflowContains -Text "cargo test --workspace" -Reason "test job gates release builds"
Assert-WorkflowContains -Text "windows-2025-vs2026" -Reason "Windows jobs use the latest verified hosted runner image"
Assert-WorkflowContains -Text "ubuntu-24.04" -Reason "publish job uses the latest stable Ubuntu hosted runner image"
Assert-WorkflowDoesNotContain -Text "windows-latest" -Reason "Windows jobs avoid drifting runner aliases"
Assert-WorkflowDoesNotContain -Text "ubuntu-latest" -Reason "publish job avoids drifting runner aliases"
Assert-WorkflowContains -Text "package_type: nsis" -Reason "initial matrix bundles NSIS"
Assert-WorkflowContains -Text "target/dx/*/bundle/windows/nsis/*.exe" -Reason "asset glob uses a PowerShell-compatible Dioxus Windows NSIS output pattern"
Assert-WorkflowDoesNotContain -Text "target/dx/**/bundle/windows/nsis/*.exe" -Reason "asset glob avoids recursive glob syntax unsupported by Get-ChildItem -Path"
Assert-WorkflowDoesNotContain -Text "target/dx/*/*/bundle/windows/nsis/*.exe" -Reason "asset glob matches the verified Dioxus output depth"
Assert-WorkflowContains -Text 'Get-ChildItem -Path "${{ matrix.asset_glob }}" -File' -Reason "installer discovery consumes the matrix asset glob"
Assert-WorkflowContains -Text "Normalize NSIS bundle version" -Reason "build job normalizes prerelease versions before NSIS bundling"
Assert-WorkflowContains -Text '$nsisBundleVersion = $env:RELEASE_VERSION -replace "-.*$", ""' -Reason "NSIS bundle version drops prerelease suffix"
Assert-WorkflowContains -Text 'nsis_bundle_version=$nsisBundleVersion' -Reason "NSIS numeric bundle version is exposed for asset renaming"
Assert-WorkflowContains -Text "Rename installer asset" -Reason "installer is renamed back to the release version before upload"
Assert-WorkflowContains -Text '$targetName = $installer.Name.Replace($env:NSIS_BUNDLE_VERSION, $env:RELEASE_VERSION)' -Reason "final installer basename uses release version"
Assert-WorkflowContains -Text "steps.release_installer.outputs.installer_path" -Reason "signing, checksum, and upload use the final release installer path"
Assert-WorkflowContains -Text "steps.release_installer.outputs.installer_name" -Reason "checksum line uses the final release installer basename"
Assert-WorkflowContains -Text "actions/upload-artifact@v7.0.1" -Reason "artifact upload uses the latest verified Node 24 release"
Assert-WorkflowContains -Text "actions/download-artifact@v8.0.1" -Reason "artifact download uses the latest verified Node 24 release"
Assert-WorkflowContains -Text "softprops/action-gh-release@v3.0.0" -Reason "publish job uploads release assets with the latest verified action release"
Assert-WorkflowDoesNotContain -Text "actions/upload-artifact@v4" -Reason "artifact upload avoids the deprecated Node 20 action runtime"
Assert-WorkflowDoesNotContain -Text "actions/download-artifact@v4" -Reason "artifact download avoids the deprecated Node 20 action runtime"
Assert-WorkflowDoesNotContain -Text "softprops/action-gh-release@v2" -Reason "release publishing uses the latest action major"
Assert-WorkflowContains -Text 'HAS_WINDOWS_SIGNING_CERTIFICATE: ${{ secrets.WINDOWS_SIGNING_CERTIFICATE_BASE64 != '''' }}' -Reason "build job exposes a non-secret signing availability flag"
Assert-WorkflowContains -Text "env.HAS_WINDOWS_SIGNING_CERTIFICATE == 'true'" -Reason "signing is optional at step level without direct secrets in if"
Assert-WorkflowDoesNotContain -Text "env.WINDOWS_SIGNING_CERTIFICATE_BASE64 != ''" -Reason "signing conditions do not rely on secret-valued env vars"
Assert-WorkflowDoesNotMatch -Pattern "(?m)^      WINDOWS_SIGNING_CERTIFICATE_BASE64: \$\{\{ secrets\.WINDOWS_SIGNING_CERTIFICATE_BASE64 \}\}$" -Reason "certificate secret is not exposed at build job scope"
Assert-WorkflowDoesNotMatch -Pattern "(?m)^      WINDOWS_SIGNING_CERTIFICATE_PASSWORD: \$\{\{ secrets\.WINDOWS_SIGNING_CERTIFICATE_PASSWORD \}\}$" -Reason "certificate password secret is not exposed at build job scope"
Assert-WorkflowDoesNotMatch -Pattern "(?m)^      WINDOWS_SIGNING_TIMESTAMP_URL: \$\{\{ secrets\.WINDOWS_SIGNING_TIMESTAMP_URL \}\}$" -Reason "timestamp secret is not exposed at build job scope"
Assert-WorkflowContains -Text "Get-FileHash -Algorithm SHA256" -Reason "build job writes a SHA-256 checksum"

$normalizeNsisStep = Get-WorkflowStepBlock -Name "Normalize NSIS bundle version"
$renameInstallerStep = Get-WorkflowStepBlock -Name "Rename installer asset"
$importSigningStep = Get-WorkflowStepBlock -Name "Import signing certificate"
$signInstallerStep = Get-WorkflowStepBlock -Name "Sign installer"
$generateChecksumStep = Get-WorkflowStepBlock -Name "Generate checksum"

Assert-StepBefore -FirstName "Normalize NSIS bundle version" -SecondName "Bundle desktop app" -Reason "Cargo files are patched before Dioxus renders the NSIS template"
Assert-StepBefore -FirstName "Locate installer" -SecondName "Rename installer asset" -Reason "the numeric Dioxus installer is found before it is renamed"
Assert-StepBefore -FirstName "Rename installer asset" -SecondName "Sign installer" -Reason "optional signing targets the final release asset"
Assert-StepBefore -FirstName "Rename installer asset" -SecondName "Generate checksum" -Reason "checksum is generated after the final release asset name is set"
Assert-StepBefore -FirstName "Generate checksum" -SecondName "Upload release artifact" -Reason "checksum exists before artifact upload"

Assert-BlockContains -Block $normalizeNsisStep -Text '$updated = $content.Replace($env:RELEASE_VERSION, $nsisBundleVersion)' -Reason "CI checkout Cargo files are patched from release version to numeric NSIS version"
Assert-BlockContains -Block $renameInstallerStep -Text 'INSTALLER_PATH: ${{ steps.installer.outputs.installer_path }}' -Reason "installer rename starts from the located Dioxus artifact"
Assert-BlockContains -Block $renameInstallerStep -Text 'NSIS_BUNDLE_VERSION: ${{ steps.nsis_version.outputs.nsis_bundle_version }}' -Reason "installer rename knows the numeric NSIS version"
Assert-BlockContains -Block $renameInstallerStep -Text 'RELEASE_VERSION: ${{ needs.test.outputs.release_version }}' -Reason "installer rename restores the actual release version"
Assert-BlockContains -Block $importSigningStep -Text 'WINDOWS_SIGNING_CERTIFICATE_BASE64: ${{ secrets.WINDOWS_SIGNING_CERTIFICATE_BASE64 }}' -Reason "certificate secret is scoped to certificate import"
Assert-BlockContains -Block $importSigningStep -Text 'WINDOWS_SIGNING_CERTIFICATE_PASSWORD: ${{ secrets.WINDOWS_SIGNING_CERTIFICATE_PASSWORD }}' -Reason "password secret is available while importing the certificate"
Assert-BlockDoesNotContain -Block $importSigningStep -Text 'WINDOWS_SIGNING_TIMESTAMP_URL: ${{ secrets.WINDOWS_SIGNING_TIMESTAMP_URL }}' -Reason "timestamp secret is not exposed during certificate import"
Assert-BlockDoesNotContain -Block $signInstallerStep -Text 'WINDOWS_SIGNING_CERTIFICATE_BASE64: ${{ secrets.WINDOWS_SIGNING_CERTIFICATE_BASE64 }}' -Reason "certificate secret is not exposed during signing after import"
Assert-BlockContains -Block $signInstallerStep -Text 'INSTALLER_PATH: ${{ steps.release_installer.outputs.installer_path }}' -Reason "signing uses the final release installer path"
Assert-BlockContains -Block $signInstallerStep -Text 'WINDOWS_SIGNING_CERTIFICATE_PASSWORD: ${{ secrets.WINDOWS_SIGNING_CERTIFICATE_PASSWORD }}' -Reason "password secret is scoped to installer signing"
Assert-BlockContains -Block $signInstallerStep -Text 'WINDOWS_SIGNING_TIMESTAMP_URL: ${{ secrets.WINDOWS_SIGNING_TIMESTAMP_URL }}' -Reason "timestamp secret is scoped to installer signing"
Assert-BlockContains -Block $generateChecksumStep -Text 'INSTALLER_PATH: ${{ steps.release_installer.outputs.installer_path }}' -Reason "checksum uses the final release installer path"
Assert-BlockContains -Block $generateChecksumStep -Text 'INSTALLER_NAME: ${{ steps.release_installer.outputs.installer_name }}' -Reason "checksum line uses the final release installer basename"

Write-Host "release workflow assertions passed"
