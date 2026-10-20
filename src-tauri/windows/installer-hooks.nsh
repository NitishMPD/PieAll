; Replaces the pinned Tauri CLI 2.12.3 process guard. Never force-close a job.
; Review against upstream utils.nsh/installer.nsi when upgrading the CLI.
!ifmacrondef CheckIfAppIsRunning
  !error "Tauri process-check contract changed; review PieAll installer hooks."
!endif
!macroundef CheckIfAppIsRunning
!macro CheckIfAppIsRunning executablePath productName
  !define PieAllGuardID ${__LINE__}
  Push $R0
  Push $R7
  Push $R8
  StrCpy $R8 0
  pieall_check_${PieAllGuardID}:
    !insertmacro RestartManager_StartSession $R0
    ${If} $R0 == ""
      Abort "Cannot check running applications. Close PieAll and retry."
    ${EndIf}
    !insertmacro RestartManager_RegisterFile $R0 "${executablePath}"
    ${If} $0 != 0
      !insertmacro RestartManager_EndSession $R0
      Abort "Cannot check the PieAll executable. Close PieAll and retry."
    ${EndIf}
    System::Call 'RSTRTMGR::RmGetList(p R0, *i .r1, *i .r2, p 0, *i .r3) i .r0'
    StrCpy $R7 $0
    !insertmacro RestartManager_EndSession $R0
    ${If} $R7 == ${ERROR_MORE_DATA}
      ${If} $UpdateMode == 1
      ${AndIf} $R8 < 60
        Sleep 1000
        IntOp $R8 $R8 + 1
        Goto pieall_check_${PieAllGuardID}
      ${EndIf}
      IfSilent pieall_abort_${PieAllGuardID}
      MessageBox MB_OK|MB_ICONEXCLAMATION "Finish the current action, then exit PieAll from its tray menu and retry. No processes will be stopped."
      pieall_abort_${PieAllGuardID}:
      SetErrorLevel 2
      Abort "PieAll is running. Installation/uninstallation cancelled."
    ${ElseIf} $R7 != 0
      SetErrorLevel 2
      Abort "Could not determine whether PieAll is running. Retry after closing it."
    ${EndIf}
  Pop $R8
  Pop $R7
  Pop $R0
  !undef PieAllGuardID
!macroend
