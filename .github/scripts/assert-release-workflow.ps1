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
