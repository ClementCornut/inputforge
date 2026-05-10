[CmdletBinding()]
param(
    [string] $DioxusPath = "Dioxus.toml",
    [string] $AppManifestPath = "crates/inputforge-app/Cargo.toml",
    [string] $BuildScriptPath = "crates/inputforge-app/build.rs",
    [string] $WorkflowPath = ".github/workflows/release.yml",
    [string] $TemplatePath = "scripts/nsis/inputforge.nsi",
    [string] $BundleRoot = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Read-RequiredFile {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Path
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        throw "Required file '$Path' does not exist."
    }

    return Get-Content -LiteralPath $Path -Raw
}

function Assert-Contains {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Content,

        [Parameter(Mandatory = $true)]
        [string] $Text,

        [Parameter(Mandatory = $true)]
        [string] $Reason
    )

    if (-not $Content.Contains($Text)) {
        throw "Windows installer contract failed: $Reason. Missing text: $Text"
    }
}

function Assert-DoesNotContain {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Content,

        [Parameter(Mandatory = $true)]
        [string] $Text,

        [Parameter(Mandatory = $true)]
        [string] $Reason
    )

    if ($Content.Contains($Text)) {
        throw "Windows installer contract failed: $Reason. Unexpected text: $Text"
    }
}

$dioxus = Read-RequiredFile -Path $DioxusPath
$manifest = Read-RequiredFile -Path $AppManifestPath
$buildScript = Read-RequiredFile -Path $BuildScriptPath
$workflow = Read-RequiredFile -Path $WorkflowPath
$template = Read-RequiredFile -Path $TemplatePath

Assert-Contains -Content $dioxus -Text 'name = "inputforge"' -Reason "Dioxus application name is exact lowercase"
Assert-Contains -Content $dioxus -Text 'resources = [' -Reason "bundle resources are declared"
Assert-Contains -Content $dioxus -Text '"SDL/SDL3.dll"' -Reason "SDL3 runtime is included as a Dioxus bundle resource"
Assert-Contains -Content $dioxus -Text 'template = "../../scripts/nsis/inputforge.nsi"' -Reason "Windows NSIS uses the project-owned template resolved from the app crate"
Assert-Contains -Content $dioxus -Text 'install_mode = "Both"' -Reason "installer supports all-users and current-user modes"
Assert-Contains -Content $dioxus -Text 'start_menu_folder = "inputforge"' -Reason "Start Menu folder is exact lowercase"

Assert-Contains -Content $manifest -Text 'autobins = false' -Reason "implicit inputforge-app executable is disabled"
Assert-Contains -Content $manifest -Text '[[bin]]' -Reason "explicit app binary target is declared"
Assert-Contains -Content $manifest -Text 'name = "inputforge"' -Reason "app binary target is exact lowercase"
Assert-Contains -Content $manifest -Text 'path = "src/main.rs"' -Reason "explicit app binary uses the existing entrypoint"

Assert-Contains -Content $buildScript -Text 'const DX_APP_TARGET: &str = "inputforge";' -Reason "dx runtime DLL copy targets the lowercase app output"
Assert-DoesNotContain -Content $buildScript -Text '.join("inputforge-app")' -Reason "build script no longer copies SDL3.dll into the old dx app directory"

Assert-Contains -Content $workflow -Text '--bin inputforge' -Reason "release bundling selects the lowercase binary target"
Assert-Contains -Content $workflow -Text 'Assert Windows installer source contract' -Reason "release workflow checks installer source contract"
Assert-Contains -Content $workflow -Text 'Assert Windows bundle payload' -Reason "release workflow checks generated bundle payload"
Assert-Contains -Content $workflow -Text 'Test-Path -LiteralPath "SDL/SDL3.dll"' -Reason "release workflow fails fast when SDL3.dll is absent"
Assert-Contains -Content $workflow -Text '$targetName = "inputforge_{0}_x64-setup.exe" -f $env:RELEASE_VERSION' -Reason "release asset name is exact lowercase"
Assert-DoesNotContain -Content $workflow -Text '$targetName = $installer.Name.Replace($env:NSIS_BUNDLE_VERSION, $env:RELEASE_VERSION)' -Reason "release asset name is not inherited from Dioxus product casing"

Assert-Contains -Content $template -Text '!include "MultiUser.nsh"' -Reason "NSIS template uses the multi-user chooser"
Assert-Contains -Content $template -Text '!include "Sections.nsh"' -Reason "NSIS template can preselect or skip optional WebView2 section"
Assert-Contains -Content $template -Text '!define APP_NAME "inputforge"' -Reason "NSIS app name is exact lowercase"
Assert-Contains -Content $template -Text '!searchparse /noerrors "${RAW_VERSION}" "" NUMERIC_VERSION "-"' -Reason "NSIS file version strips prerelease suffixes for local RC bundles"
Assert-Contains -Content $template -Text 'VIProductVersion "${NUMERIC_VERSION}.0"' -Reason "NSIS file version is always numeric"
Assert-Contains -Content $template -Text '!define MULTIUSER_INSTALLMODE_INSTDIR "inputforge"' -Reason "Program Files and user install roots end in inputforge"
Assert-Contains -Content $template -Text '!insertmacro MULTIUSER_PAGE_INSTALLMODE' -Reason "installer exposes all-users/current-user selection"
Assert-Contains -Content $template -Text '!insertmacro MULTIUSER_INIT' -Reason "installer initializes multi-user install mode"
Assert-Contains -Content $template -Text 'Section "inputforge" SEC_APP' -Reason "main app section is exact lowercase"
Assert-Contains -Content $template -Text 'SectionIn RO' -Reason "main app section cannot be deselected"
Assert-Contains -Content $template -Text 'Function DetectWebView2Runtime' -Reason "installer detects existing WebView2 before installing it"
Assert-Contains -Content $template -Text 'SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' -Reason "installer checks the documented HKLM WebView2 runtime key"
Assert-Contains -Content $template -Text 'Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' -Reason "installer checks the documented HKCU WebView2 runtime key"
Assert-Contains -Content $template -Text 'Section "Microsoft Edge WebView2 Runtime" SEC_WEBVIEW2' -Reason "WebView2 remains available when absent"
Assert-Contains -Content $template -Text '!insertmacro MUI_PAGE_COMPONENTS' -Reason "optional WebView2 dependency is exposed in the installer UI"
Assert-Contains -Content $template -Text 'SectionSetFlags ${SEC_WEBVIEW2} 0' -Reason "WebView2 section is skipped when already installed"
Assert-Contains -Content $template -Text '!define MUI_FINISHPAGE_RUN "$INSTDIR\{{main_binary_name}}"' -Reason "finish page offers to launch the installed app"
Assert-Contains -Content $template -Text '!define MUI_FINISHPAGE_RUN_TEXT "Open inputforge"' -Reason "finish page launch checkbox uses exact lowercase app name"
Assert-Contains -Content $template -Text 'CreateShortcut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"' -Reason "Start Menu shortcut uses exact lowercase name"
Assert-Contains -Content $template -Text 'CreateShortcut "$DESKTOP\${APP_NAME}.lnk"' -Reason "Desktop shortcut uses exact lowercase name"
Assert-Contains -Content $template -Text '"DisplayName" "${APP_NAME}"' -Reason "Add/Remove Programs display name is exact lowercase"

if ($BundleRoot) {
    if (-not (Test-Path -LiteralPath $BundleRoot)) {
        throw "Bundle root '$BundleRoot' does not exist."
    }

    $staging = Join-Path $BundleRoot "_staging"
    if (-not (Test-Path -LiteralPath $staging)) {
        throw "Bundle staging directory '$staging' does not exist."
    }

    $expectedExe = Join-Path $staging "inputforge.exe"
    $expectedDll = Join-Path $staging "SDL3.dll"
    $oldExe = Join-Path $staging "inputforge-app.exe"

    if (-not (Test-Path -LiteralPath $expectedExe -PathType Leaf)) {
        throw "Expected bundled executable '$expectedExe'."
    }

    if (-not (Test-Path -LiteralPath $expectedDll -PathType Leaf)) {
        throw "Expected bundled SDL runtime '$expectedDll'."
    }

    if ((Get-Item -LiteralPath $expectedDll).Length -le 0) {
        throw "Bundled SDL runtime '$expectedDll' is empty."
    }

    if (Test-Path -LiteralPath $oldExe) {
        throw "Old bundled executable '$oldExe' must not exist."
    }
}

Write-Host "Windows installer contract assertions passed"
