[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string] $TagRef,

    [string] $ManifestPath = "Cargo.toml",

    [string] $GitHubOutputPath = $env:GITHUB_OUTPUT
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$tagName = if ($TagRef.StartsWith("refs/tags/")) {
    $TagRef.Substring("refs/tags/".Length)
} else {
    $TagRef
}

$tagMatch = [regex]::Match(
    $TagRef,
    '^(?:refs/tags/)?v(?<version>[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?)$'
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
