; Ryuuji Windows Installer Script
; Built with NSIS - https://nsis.sourceforge.io
;
; Required defines (passed via -D from CI):
;   VERSION      - e.g. "0.1.0"
;   BINARY_PATH  - absolute path to ryuuji.exe
;   LICENSE_PATH - absolute path to LICENSE
;   ICON_PATH    - absolute path to icon.ico

!include "MUI2.nsh"

; General
Name "Ryuuji ${VERSION}"
OutFile "ryuuji-${VERSION}-windows-x64-setup.exe"
InstallDir "$PROGRAMFILES64\Ryuuji"
InstallDirRegKey HKLM "Software\Ryuuji" "InstallDir"
RequestExecutionLevel admin
Unicode True

; Version info
VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "Ryuuji"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "FileDescription" "Ryuuji Installer"
VIAddVersionKey "LegalCopyright" "MIT License"

; Interface settings
!define MUI_ABORTWARNING
!define MUI_ICON "${ICON_PATH}"
!define MUI_UNICON "${ICON_PATH}"

; Pages
!insertmacro MUI_PAGE_LICENSE "${LICENSE_PATH}"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

; Language
!insertmacro MUI_LANGUAGE "English"

; Installer section
Section "Ryuuji" SecMain
    SectionIn RO

    ; Install files
    SetOutPath "$INSTDIR"
    File "${BINARY_PATH}"

    ; Create uninstaller
    WriteUninstaller "$INSTDIR\uninstall.exe"

    ; Start Menu shortcuts
    CreateDirectory "$SMPROGRAMS\Ryuuji"
    CreateShortcut "$SMPROGRAMS\Ryuuji\Ryuuji.lnk" "$INSTDIR\ryuuji.exe"
    CreateShortcut "$SMPROGRAMS\Ryuuji\Uninstall.lnk" "$INSTDIR\uninstall.exe"

    ; Desktop shortcut
    CreateShortcut "$DESKTOP\Ryuuji.lnk" "$INSTDIR\ryuuji.exe"

    ; Registry - install dir
    WriteRegStr HKLM "Software\Ryuuji" "InstallDir" "$INSTDIR"

    ; Registry - Add/Remove Programs
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Ryuuji" \
        "DisplayName" "Ryuuji ${VERSION}"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Ryuuji" \
        "UninstallString" '"$INSTDIR\uninstall.exe"'
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Ryuuji" \
        "DisplayVersion" "${VERSION}"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Ryuuji" \
        "Publisher" "Ryuuji"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Ryuuji" \
        "URLInfoAbout" "https://github.com/umarudotdev/ryuuji"
    WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Ryuuji" \
        "NoModify" 1
    WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Ryuuji" \
        "NoRepair" 1
SectionEnd

; Uninstaller section
Section "Uninstall"
    ; Remove files
    Delete "$INSTDIR\ryuuji.exe"
    Delete "$INSTDIR\uninstall.exe"
    RMDir "$INSTDIR"

    ; Remove shortcuts
    Delete "$SMPROGRAMS\Ryuuji\Ryuuji.lnk"
    Delete "$SMPROGRAMS\Ryuuji\Uninstall.lnk"
    RMDir "$SMPROGRAMS\Ryuuji"
    Delete "$DESKTOP\Ryuuji.lnk"

    ; Remove registry keys
    DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Ryuuji"
    DeleteRegKey HKLM "Software\Ryuuji"
SectionEnd
