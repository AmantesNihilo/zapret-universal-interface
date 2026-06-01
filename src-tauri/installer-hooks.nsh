Icon "${__FILEDIR__}\icons\icon.ico"
UninstallIcon "${__FILEDIR__}\icons\icon.ico"

!macro ZUI_STOP_RUNNING_APP
  DetailPrint "Stopping running ZUI instance..."
  nsExec::ExecToLog 'taskkill /IM zui.exe /T'
  nsExec::ExecToLog 'taskkill /IM ZUI.exe /T'
  Sleep 2500
  nsExec::ExecToLog 'taskkill /IM zui.exe /T /F'
  nsExec::ExecToLog 'taskkill /IM ZUI.exe /T /F'
  Sleep 1000
!macroend

!macro ZUI_REMOVE_RUNTIME_DATA
  DetailPrint "Removing ZUI runtime data..."
  RMDir /r "$INSTDIR\data"
  RMDir /r "$APPDATA\ZUI"
  RMDir /r "$LOCALAPPDATA\ZUI"
  RMDir "$INSTDIR"
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro ZUI_STOP_RUNNING_APP
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro ZUI_STOP_RUNNING_APP
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  !insertmacro ZUI_REMOVE_RUNTIME_DATA
!macroend
