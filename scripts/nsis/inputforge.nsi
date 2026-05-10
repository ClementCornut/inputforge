!define APP_NAME "inputforge"
!define WEBVIEW2_CLIENT_KEY "Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
!define WEBVIEW2_CLIENT_KEY_WOW64 "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}"

!define MULTIUSER_EXECUTIONLEVEL Highest
!define MULTIUSER_MUI
!define MULTIUSER_INSTALLMODE_COMMANDLINE
!define MULTIUSER_INSTALLMODE_INSTDIR "inputforge"
!define MULTIUSER_INSTALLMODE_DEFAULT_REGISTRY_KEY "${UNINSTALL_KEY}"
!define MULTIUSER_INSTALLMODE_DEFAULT_REGISTRY_VALUENAME "InstallMode"
!define MULTIUSER_INSTALLMODE_INSTDIR_REGISTRY_KEY "${UNINSTALL_KEY}"
!define MULTIUSER_INSTALLMODE_INSTDIR_REGISTRY_VALUENAME "InstallLocation"
!define MULTIUSER_INSTALLMODEPAGE_SHOWUSERNAME
!define MULTIUSER_INSTALLMODEPAGE_TEXT_TOP "Choose whether to install inputforge for all users of this computer or only for the current user."
!define MULTIUSER_INSTALLMODEPAGE_TEXT_ALLUSERS "Install for all users"
!define MULTIUSER_INSTALLMODEPAGE_TEXT_CURRENTUSER "Install just for me"

!include "MultiUser.nsh"
!include "x64.nsh"
!include "Sections.nsh"

Var WebView2RuntimeInstalled

!define RAW_VERSION "{{version}}"
!searchparse /noerrors "${RAW_VERSION}" "" NUMERIC_VERSION "-"
!ifndef NUMERIC_VERSION
!define NUMERIC_VERSION "${RAW_VERSION}"
!endif

Name "${APP_NAME}"
OutFile "{{output_path}}"
Unicode true
InstallDir "$PROGRAMFILES\${APP_NAME}"

; Version information
VIProductVersion "${NUMERIC_VERSION}.0"
VIAddVersionKey "ProductName" "${APP_NAME}"
VIAddVersionKey "FileVersion" "{{version}}"
VIAddVersionKey "ProductVersion" "{{version}}"
VIAddVersionKey "FileDescription" "{{short_description}}"
{{#if publisher}}
VIAddVersionKey "CompanyName" "{{publisher}}"
{{/if}}
{{#if copyright}}
VIAddVersionKey "LegalCopyright" "{{copyright}}"
{{/if}}

; MUI settings
!define MUI_ABORTWARNING
{{#if installer_icon}}
!define MUI_ICON "{{installer_icon}}"
{{/if}}
{{#if header_image}}
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_BITMAP "{{header_image}}"
{{/if}}
{{#if sidebar_image}}
!define MUI_WELCOMEFINISHPAGE_BITMAP "{{sidebar_image}}"
{{/if}}

Function WebView2MarkInstalledIfValid
    StrCmp $0 "" done
    StrCmp $0 "0.0.0.0" done
    StrCpy $WebView2RuntimeInstalled "true"
done:
FunctionEnd

Function DetectWebView2Runtime
    StrCpy $WebView2RuntimeInstalled "false"

    ClearErrors
    ReadRegStr $0 HKLM "${WEBVIEW2_CLIENT_KEY_WOW64}" "pv"
    IfErrors +2
    Call WebView2MarkInstalledIfValid

    ClearErrors
    ReadRegStr $0 HKLM "${WEBVIEW2_CLIENT_KEY}" "pv"
    IfErrors +2
    Call WebView2MarkInstalledIfValid

    ClearErrors
    ReadRegStr $0 HKCU "${WEBVIEW2_CLIENT_KEY}" "pv"
    IfErrors +2
    Call WebView2MarkInstalledIfValid
FunctionEnd

; Pages
{{#if license}}
!insertmacro MUI_PAGE_LICENSE "{{license}}"
{{/if}}
!insertmacro MULTIUSER_PAGE_INSTALLMODE
{{#if install_webview}}
!define MUI_PAGE_CUSTOMFUNCTION_PRE WebView2ComponentsPre
!insertmacro MUI_PAGE_COMPONENTS
!ifdef MUI_PAGE_CUSTOMFUNCTION_PRE
!undef MUI_PAGE_CUSTOMFUNCTION_PRE
!endif
{{/if}}
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\{{main_binary_name}}"
!define MUI_FINISHPAGE_RUN_TEXT "Open inputforge"
!insertmacro MUI_PAGE_FINISH

; Uninstaller pages
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

; Language
!insertmacro MUI_LANGUAGE "English"
{{#each additional_languages}}
!insertmacro MUI_LANGUAGE "{{this}}"
{{/each}}

Section "inputforge" SEC_APP
    SectionIn RO
    SetOutPath $INSTDIR

    ; Install main binary
    File "{{main_binary_path}}"

    ; Install resources
    {{#each staged_files}}
    SetOutPath "$INSTDIR{{#if this.target_dir}}\{{this.target_dir}}{{/if}}"
    File "{{this.source}}"
    {{/each}}

    SetOutPath $INSTDIR

    ; Create uninstaller
    WriteUninstaller "$INSTDIR\uninstall.exe"

    ; Create Start Menu shortcuts
    CreateDirectory "$SMPROGRAMS\${APP_NAME}"
    CreateShortcut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\{{main_binary_name}}"
    CreateShortcut "$SMPROGRAMS\${APP_NAME}\Uninstall ${APP_NAME}.lnk" "$INSTDIR\uninstall.exe"

    ; Create Desktop shortcut
    CreateShortcut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\{{main_binary_name}}"

    ; Write registry keys for Add/Remove Programs
    WriteRegStr SHCTX "${UNINSTALL_KEY}" "DisplayName" "${APP_NAME}"
    WriteRegStr SHCTX "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
    WriteRegStr SHCTX "${UNINSTALL_KEY}" "DisplayVersion" "{{version}}"
    {{#if publisher}}
    WriteRegStr SHCTX "${UNINSTALL_KEY}" "Publisher" "{{publisher}}"
    {{/if}}
    WriteRegStr SHCTX "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
    WriteRegStr SHCTX "${UNINSTALL_KEY}" "InstallMode" "$MultiUser.InstallMode"

    ; Get installed size
    ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
    IntFmt $0 "0x%08X" $0
    WriteRegDWORD SHCTX "${UNINSTALL_KEY}" "EstimatedSize" "$0"
SectionEnd

{{#if install_webview}}
Section "Microsoft Edge WebView2 Runtime" SEC_WEBVIEW2
    ; Install WebView2 only when the runtime is absent and the user keeps this
    ; optional dependency selected.
{{webview_install_code}}
SectionEnd
{{/if}}

Function .onInit
    !insertmacro MULTIUSER_INIT
{{#if install_webview}}
    Call DetectWebView2Runtime
    StrCmp $WebView2RuntimeInstalled "true" 0 WebView2OnInitMissing
    SectionSetFlags ${SEC_WEBVIEW2} 0
    Goto WebView2OnInitDone

WebView2OnInitMissing:
    SectionSetFlags ${SEC_WEBVIEW2} ${SF_SELECTED}

WebView2OnInitDone:
{{/if}}
FunctionEnd

Function un.onInit
    !insertmacro MULTIUSER_UNINIT
FunctionEnd

{{#if install_webview}}
Function WebView2ComponentsPre
    Call DetectWebView2Runtime
    StrCmp $WebView2RuntimeInstalled "true" 0 WebView2Missing
    SectionSetFlags ${SEC_WEBVIEW2} 0
    Abort

WebView2Missing:
    SectionSetFlags ${SEC_WEBVIEW2} ${SF_SELECTED}
FunctionEnd
{{/if}}

{{#if installer_hooks}}
!include "{{installer_hooks}}"
{{/if}}

Section "Uninstall"
    ; Remove files
    RMDir /r "$INSTDIR"

    ; Remove Start Menu items
    RMDir /r "$SMPROGRAMS\${APP_NAME}"

    ; Remove Desktop shortcut
    Delete "$DESKTOP\${APP_NAME}.lnk"

    ; Remove registry keys
    DeleteRegKey SHCTX "${UNINSTALL_KEY}"
SectionEnd
