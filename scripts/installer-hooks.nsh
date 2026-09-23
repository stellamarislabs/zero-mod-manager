; Keep existing shortcuts and pin preferences. Only this installation's exact
; EXE target is eligible; arguments, working directory and other link data stay.
!include "${__FILEDIR__}\..\src-tauri\icons\windows-icon.nsh"

Var ZeroBrandReady
Var ZeroBrandExecutable
Var ZeroBrandIcon
Var ZeroBrandAppId
Var ZeroBrandDesktop
Var ZeroBrandStartMenu
Var ZeroBrandStartMenuFolder

Function ZeroRefreshShortcutIcon
  Exch $R9
  Push $0
  Push $1
  Push $2
  Push $3
  Push $4
  Push $5
  IfFileExists "$R9" 0 zero_icon_done
  !insertmacro IsShortcutTarget "$R9" "$ZeroBrandExecutable"
  Pop $0
  ${If} $0 = 1
    !insertmacro ComHlpr_CreateInProcInstance ${CLSID_ShellLink} ${IID_IShellLink} r0 ""
    ${If} $0 P<> 0
      ${IUnknown::QueryInterface} $0 '("${IID_IPersistFile}",.r1)'
      ${If} $1 P<> 0
        ${IPersistFile::Load} $1 '("$R9", ${STGM_READWRITE})i.r3'
        ${If} $3 >= 0
          ${IShellLink::SetIconLocation} $0 '(w "$ZeroBrandIcon", 0)i.r3'
          ${If} $3 >= 0
            ${IUnknown::QueryInterface} $0 '("${IID_IPropertyStore}",.r2)'
            ${If} $2 P<> 0
              System::Call 'Oleaut32::SysAllocString(w "$ZeroBrandAppId") p.r3'
              System::Call '*${SYSSTRUCT_PROPERTYKEY}(${PKEY_AppUserModel_ID})p.r4'
              System::Call '*${SYSSTRUCT_PROPVARIANT}(${VT_BSTR},,&i4 $3)p.r5'
              ${IPropertyStore::SetValue} $2 '($4,$5)'
              ${IPropertyStore::Commit} $2 ''
              System::Call 'Oleaut32::SysFreeString(p r3)'
              System::Free $4
              System::Free $5
              ${IUnknown::Release} $2 ''
            ${EndIf}
            ${IPersistFile::Save} $1 '("$R9",1)i.r3'
            ${If} $3 >= 0
              ; Notify only the changed item. No Explorer restart or global purge.
              System::Call 'Shell32::SHChangeNotify(i 0x2000, i 0x1005, w "$R9", p 0)'
            ${EndIf}
          ${EndIf}
        ${EndIf}
        ${IUnknown::Release} $1 ''
      ${EndIf}
      ${IUnknown::Release} $0 ''
    ${EndIf}
  ${EndIf}
  zero_icon_done:
  Pop $5
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
  Pop $R9
FunctionEnd

Function ZeroRefreshBrandShortcuts
  ${If} $ZeroBrandReady != 1
    Return
  ${EndIf}
  IfFileExists "$ZeroBrandIcon" 0 zero_refresh_done
  Push "$ZeroBrandDesktop"
  Call ZeroRefreshShortcutIcon
  Push "$ZeroBrandStartMenu"
  Call ZeroRefreshShortcutIcon
  Push "$ZeroBrandStartMenuFolder"
  Call ZeroRefreshShortcutIcon
  ; A pin is a separate .lnk. Refresh a matching pin in place, never create,
  ; remove or reorder one. The exact EXE-target check also excludes other installs.
  Push $R7
  Push $R8
  FindFirst $R8 $R7 "$APPDATA\Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar\*.lnk"
  ${DoWhile} $R7 != ""
    Push "$APPDATA\Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar\$R7"
    Call ZeroRefreshShortcutIcon
    FindNext $R8 $R7
  ${Loop}
  FindClose $R8
  Pop $R8
  Pop $R7
  zero_refresh_done:
FunctionEnd

!macro NSIS_HOOK_POSTINSTALL
  StrCpy $ZeroBrandExecutable "$INSTDIR\${MAINBINARYNAME}.exe"
  StrCpy $ZeroBrandIcon "$INSTDIR\${ZERO_BRAND_ICON}"
  StrCpy $ZeroBrandAppId "${BUNDLEID}"
  StrCpy $ZeroBrandDesktop "$DESKTOP\${PRODUCTNAME}.lnk"
  StrCpy $ZeroBrandStartMenu "$SMPROGRAMS\${PRODUCTNAME}.lnk"
  StrCpy $ZeroBrandStartMenuFolder "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk"
  StrCpy $ZeroBrandReady 1
  WriteRegStr SHCTX "${UNINSTKEY}" "DisplayIcon" "$ZeroBrandIcon,0"
  Call ZeroRefreshBrandShortcuts
!macroend

; Tauri's GUI finish checkbox creates the desktop link AFTER POSTINSTALL.
; The final GUI callback catches that new link; POSTINSTALL handles /S and /P.
; The guard above also makes a cancelled or failed installation a no-op.
Function .onGUIEnd
  Call ZeroRefreshBrandShortcuts
FunctionEnd
