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
    $lastExitCode = Get-Variable -Name LASTEXITCODE -ValueOnly -ErrorAction SilentlyContinue

    return @{
        ExitCode = if ($null -ne $lastExitCode) { $lastExitCode } else { 0 }
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

$namespacedRef = New-TestManifest -Version "0.1.0"
Assert-Throws -MessageFragment "is not a supported semver release tag" -ScriptBlock {
    Invoke-Validator -TagRef "refs/tags/release/v0.1.0" -ManifestPath $namespacedRef.ManifestPath
}

Write-Host "release tag helper tests passed"
