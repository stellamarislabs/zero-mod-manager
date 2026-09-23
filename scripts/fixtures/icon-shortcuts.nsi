; Isolated icon migration fixture. No real shortcuts, registry or pins are used.
Unicode true
RequestExecutionLevel user
SilentInstall silent
!include LogicLib.nsh
!include Win\COM.nsh
!include Win\Propkey.nsh
!include "${SOURCE_ROOT}\src-tauri\target\release\nsis\x64\utils.nsh"
!include "${SOURCE_ROOT}\scripts\installer-hooks.nsh"
OutFile "${FIXTURE_ROOT}\verify-shortcuts.exe"
Section
  StrCpy $ZeroBrandExecutable "${FIXTURE_ROOT}\manager.exe"
  StrCpy $ZeroBrandIcon "${FIXTURE_ROOT}\brand.ico"
  StrCpy $ZeroBrandAppId "app.zeromodmanager.desktop"
  Push "${FIXTURE_ROOT}\own.lnk"
  Call ZeroRefreshShortcutIcon
  Push "${FIXTURE_ROOT}\other.lnk"
  Call ZeroRefreshShortcutIcon
  Push "${FIXTURE_ROOT}\missing.lnk"
  Call ZeroRefreshShortcutIcon
SectionEnd
