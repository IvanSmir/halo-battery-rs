; Installer hooks for the Tauri NSIS bundle. The installer ships two programs:
; the settings window (the main binary) and the tray (halo-battery.exe, an
; external binary), which is the one that has to keep running.

; Close both programs so their files can be replaced (upgrade) or removed.
!macro CLOSE_HALO
  nsExec::Exec 'taskkill /F /IM halo-battery.exe'
  nsExec::Exec 'taskkill /F /IM halo-settings.exe'
  Sleep 500
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro CLOSE_HALO
!macroend

; Start the tray right away; "start with Windows" is set from the settings window.
!macro NSIS_HOOK_POSTINSTALL
  Exec '"$INSTDIR\halo-battery.exe"'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro CLOSE_HALO
  ; the "start with Windows" entry the settings window may have written
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "HaloBatteryRs"
!macroend
