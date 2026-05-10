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
Assert-WorkflowContains -Text "DIOXUS_CLI_VERSION: 0.7.9" -Reason "workflow pins the target Dioxus CLI"
Assert-WorkflowContains -Text "cargo test --workspace" -Reason "test job gates release builds"
Assert-WorkflowContains -Text "windows-latest" -Reason "initial matrix includes Windows"
Assert-WorkflowContains -Text "package_type: nsis" -Reason "initial matrix bundles NSIS"
Assert-WorkflowContains -Text "target/dx/*/bundle/windows/nsis/*.exe" -Reason "asset glob uses a PowerShell-compatible Dioxus Windows NSIS output pattern"
Assert-WorkflowDoesNotContain -Text "target/dx/**/bundle/windows/nsis/*.exe" -Reason "asset glob avoids recursive glob syntax unsupported by Get-ChildItem -Path"
Assert-WorkflowDoesNotContain -Text "target/dx/*/*/bundle/windows/nsis/*.exe" -Reason "asset glob matches the verified Dioxus output depth"
Assert-WorkflowContains -Text 'Get-ChildItem -Path "${{ matrix.asset_glob }}" -File' -Reason "installer discovery consumes the matrix asset glob"
Assert-WorkflowContains -Text "softprops/action-gh-release@v2" -Reason "publish job uploads release assets"
Assert-WorkflowContains -Text 'HAS_WINDOWS_SIGNING_CERTIFICATE: ${{ secrets.WINDOWS_SIGNING_CERTIFICATE_BASE64 != '''' }}' -Reason "build job exposes a non-secret signing availability flag"
Assert-WorkflowContains -Text "env.HAS_WINDOWS_SIGNING_CERTIFICATE == 'true'" -Reason "signing is optional at step level without direct secrets in if"
Assert-WorkflowDoesNotContain -Text "env.WINDOWS_SIGNING_CERTIFICATE_BASE64 != ''" -Reason "signing conditions do not rely on secret-valued env vars"
Assert-WorkflowDoesNotMatch -Pattern "(?m)^      WINDOWS_SIGNING_CERTIFICATE_BASE64: \$\{\{ secrets\.WINDOWS_SIGNING_CERTIFICATE_BASE64 \}\}$" -Reason "certificate secret is not exposed at build job scope"
Assert-WorkflowDoesNotMatch -Pattern "(?m)^      WINDOWS_SIGNING_CERTIFICATE_PASSWORD: \$\{\{ secrets\.WINDOWS_SIGNING_CERTIFICATE_PASSWORD \}\}$" -Reason "certificate password secret is not exposed at build job scope"
Assert-WorkflowDoesNotMatch -Pattern "(?m)^      WINDOWS_SIGNING_TIMESTAMP_URL: \$\{\{ secrets\.WINDOWS_SIGNING_TIMESTAMP_URL \}\}$" -Reason "timestamp secret is not exposed at build job scope"
Assert-WorkflowContains -Text "Get-FileHash -Algorithm SHA256" -Reason "build job writes a SHA-256 checksum"

$importSigningStep = Get-WorkflowStepBlock -Name "Import signing certificate"
$signInstallerStep = Get-WorkflowStepBlock -Name "Sign installer"

Assert-BlockContains -Block $importSigningStep -Text 'WINDOWS_SIGNING_CERTIFICATE_BASE64: ${{ secrets.WINDOWS_SIGNING_CERTIFICATE_BASE64 }}' -Reason "certificate secret is scoped to certificate import"
Assert-BlockContains -Block $importSigningStep -Text 'WINDOWS_SIGNING_CERTIFICATE_PASSWORD: ${{ secrets.WINDOWS_SIGNING_CERTIFICATE_PASSWORD }}' -Reason "password secret is available while importing the certificate"
Assert-BlockDoesNotContain -Block $importSigningStep -Text 'WINDOWS_SIGNING_TIMESTAMP_URL: ${{ secrets.WINDOWS_SIGNING_TIMESTAMP_URL }}' -Reason "timestamp secret is not exposed during certificate import"
Assert-BlockDoesNotContain -Block $signInstallerStep -Text 'WINDOWS_SIGNING_CERTIFICATE_BASE64: ${{ secrets.WINDOWS_SIGNING_CERTIFICATE_BASE64 }}' -Reason "certificate secret is not exposed during signing after import"
Assert-BlockContains -Block $signInstallerStep -Text 'WINDOWS_SIGNING_CERTIFICATE_PASSWORD: ${{ secrets.WINDOWS_SIGNING_CERTIFICATE_PASSWORD }}' -Reason "password secret is scoped to installer signing"
Assert-BlockContains -Block $signInstallerStep -Text 'WINDOWS_SIGNING_TIMESTAMP_URL: ${{ secrets.WINDOWS_SIGNING_TIMESTAMP_URL }}' -Reason "timestamp secret is scoped to installer signing"

Write-Host "release workflow assertions passed"
