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

!macro ZUI_STOP_OWNED_WINWS
  DetailPrint "Stopping ZUI-owned winws.exe processes..."
  nsExec::ExecToLog 'cmd /D /Q /C if exist "$APPDATA\ZUI\runtime\zapret-owned-pids.txt" for /f "usebackq tokens=*" %P in ("$APPDATA\ZUI\runtime\zapret-owned-pids.txt") do taskkill /PID %P /T /F'
  nsExec::ExecToLog 'cmd /D /Q /C if exist "$INSTDIR\data\runtime\zapret-owned-pids.txt" for /f "usebackq tokens=*" %P in ("$INSTDIR\data\runtime\zapret-owned-pids.txt") do taskkill /PID %P /T /F'
  Delete "$APPDATA\ZUI\runtime\zapret-owned-pids.txt"
  Delete "$INSTDIR\data\runtime\zapret-owned-pids.txt"
!macroend

!macro ZUI_REMOVE_RUNTIME_DATA
  DetailPrint "Removing ZUI runtime data..."
  RMDir /r "$INSTDIR\data"
  RMDir /r "$APPDATA\ZUI"
  RMDir /r "$LOCALAPPDATA\ZUI"
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro ZUI_STOP_RUNNING_APP
  !insertmacro ZUI_STOP_OWNED_WINWS
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro ZUI_STOP_RUNNING_APP
  !insertmacro ZUI_STOP_OWNED_WINWS
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  MessageBox MB_YESNO|MB_ICONQUESTION "Remove ZUI settings, logs and test results?" IDYES remove_user_data IDNO keep_user_data
  remove_user_data:
    !insertmacro ZUI_REMOVE_RUNTIME_DATA
    Goto finish_user_data
  keep_user_data:
    DetailPrint "Keeping ZUI user data."
  finish_user_data:
    RMDir "$INSTDIR"
!macroend
