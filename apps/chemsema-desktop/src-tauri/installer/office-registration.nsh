!include WinMessages.nsh

!macro CHEMSEMA_WRITE_PATH_HELPER
  InitPluginsDir
  FileOpen $4 "$PLUGINSDIR\chemsema-path.ps1" w
  FileWrite $4 "param([string]$$Action, [string]$$Dir, [string]$$TargetName)$\r$\n"
  FileWrite $4 "$$target = [EnvironmentVariableTarget]::$$TargetName$\r$\n"
  FileWrite $4 "$$current = [Environment]::GetEnvironmentVariable('Path', $$target)$\r$\n"
  FileWrite $4 "$$parts = @()$\r$\n"
  FileWrite $4 "if (-not [string]::IsNullOrEmpty($$current)) { $$parts = @($$current -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($$_) }) }$\r$\n"
  FileWrite $4 "$$normalizedDir = $$Dir.TrimEnd('\')$\r$\n"
  FileWrite $4 "$$kept = @()$\r$\n"
  FileWrite $4 "$$found = $$false$\r$\n"
  FileWrite $4 "foreach ($$part in $$parts) {$\r$\n"
  FileWrite $4 "  if ([string]::Equals($$part.TrimEnd('\'), $$normalizedDir, [StringComparison]::OrdinalIgnoreCase)) { $$found = $$true } else { $$kept += $$part }$\r$\n"
  FileWrite $4 "}$\r$\n"
  FileWrite $4 "if ($$Action -eq 'Add') {$\r$\n"
  FileWrite $4 "  if ($$found) { $$next = $$parts -join ';' } else { $$next = @($$parts + $$Dir) -join ';' }$\r$\n"
  FileWrite $4 "} elseif ($$Action -eq 'Remove') {$\r$\n"
  FileWrite $4 "  $$next = $$kept -join ';'$\r$\n"
  FileWrite $4 "} else {$\r$\n"
  FileWrite $4 "  throw 'Unknown PATH action: ' + $$Action$\r$\n"
  FileWrite $4 "}$\r$\n"
  FileWrite $4 "[Environment]::SetEnvironmentVariable('Path', $$next, $$target)$\r$\n"
  FileClose $4
!macroend

!macro NSIS_HOOK_POSTINSTALL
  DetailPrint "Registering ChemSema Office/OLE integration..."

  IfFileExists "$INSTDIR\chemsema-office.exe" chemsema_office_found_root
  IfFileExists "$INSTDIR\resources\chemsema-office.exe" chemsema_office_found_resources
  DetailPrint "ChemSema Office/OLE registration skipped: chemsema-office.exe was not found."
  MessageBox MB_ICONSTOP "ChemSema Office/OLE registration failed because chemsema-office.exe was not found."
  Abort

  chemsema_office_found_root:
    StrCpy $1 "$INSTDIR\chemsema-office.exe"
    Goto chemsema_office_register_machine

  chemsema_office_found_resources:
    StrCpy $1 "$INSTDIR\resources\chemsema-office.exe"

  chemsema_office_register_machine:
  ClearErrors
  ExecWait '"$1" --register-machine' $0
  IfErrors chemsema_office_register_machine_exec_failed
  StrCmp $0 0 chemsema_office_register_machine_done
  DetailPrint "ChemSema Office/OLE machine registration failed with exit code: $0"
  Goto chemsema_office_register_user

  chemsema_office_register_machine_exec_failed:
  DetailPrint "ChemSema Office/OLE machine registration could not launch."

  chemsema_office_register_user:
  DetailPrint "Registering ChemSema Office/OLE integration for the current user..."
  ClearErrors
  ExecWait '"$1" --register-user' $0
  IfErrors chemsema_office_register_user_exec_failed
  StrCmp $0 0 chemsema_office_register_user_done
  DetailPrint "ChemSema Office/OLE current-user registration failed with exit code: $0"
  MessageBox MB_ICONSTOP "ChemSema Office/OLE registration failed with exit code $0."
  Abort

  chemsema_office_register_user_exec_failed:
  DetailPrint "ChemSema Office/OLE current-user registration could not launch."
  MessageBox MB_ICONSTOP "ChemSema Office/OLE registration failed because chemsema-office.exe could not be launched."
  Abort

  chemsema_office_register_machine_done:
  DetailPrint "ChemSema Office/OLE machine registration succeeded."
  Goto chemsema_office_register_done

  chemsema_office_register_user_done:
  DetailPrint "ChemSema Office/OLE current-user registration succeeded."

  chemsema_office_register_done:

  DetailPrint "Registering ChemSema CLI app path..."
  IfFileExists "$INSTDIR\chemsema-cli.exe" chemsema_cli_found_root
  IfFileExists "$INSTDIR\resources\chemsema-cli.exe" chemsema_cli_found_resources
  DetailPrint "ChemSema CLI app path registration skipped: chemsema-cli.exe was not found."
  Goto chemsema_cli_register_done

  chemsema_cli_found_root:
    StrCpy $2 "$INSTDIR\chemsema-cli.exe"
    StrCpy $3 "$INSTDIR"
    Goto chemsema_cli_register_machine

  chemsema_cli_found_resources:
    DetailPrint "Copying ChemSema CLI from resources to the install root..."
    CopyFiles /SILENT "$INSTDIR\resources\chemsema-cli.exe" "$INSTDIR\chemsema-cli.exe"
    IfFileExists "$INSTDIR\chemsema-cli.exe" chemsema_cli_found_root
    StrCpy $2 "$INSTDIR\resources\chemsema-cli.exe"
    StrCpy $3 "$INSTDIR\resources"

  chemsema_cli_register_machine:
  ClearErrors
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\chemsema-cli.exe" "" "$2"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\chemsema-cli.exe" "Path" "$3"
  IfErrors chemsema_cli_register_user
  DetailPrint "ChemSema CLI machine app path registration succeeded."
  Goto chemsema_cli_register_done

  chemsema_cli_register_user:
  ClearErrors
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\App Paths\chemsema-cli.exe" "" "$2"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\App Paths\chemsema-cli.exe" "Path" "$3"
  IfErrors chemsema_cli_register_failed
  DetailPrint "ChemSema CLI current-user app path registration succeeded."
  Goto chemsema_cli_register_done

  chemsema_cli_register_failed:
  DetailPrint "ChemSema CLI app path registration failed."

  chemsema_cli_register_done:
  IfFileExists "$3\chemsema-cli.exe" chemsema_cli_path_begin
  DetailPrint "ChemSema CLI PATH registration skipped: chemsema-cli.exe was not found in the selected CLI directory."
  Goto chemsema_cli_path_done

  chemsema_cli_path_begin:
  DetailPrint "Adding ChemSema CLI directory to PATH..."
  !insertmacro CHEMSEMA_WRITE_PATH_HELPER
  ClearErrors
  ExecWait 'powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$PLUGINSDIR\chemsema-path.ps1" Add "$3" Machine' $0
  IfErrors chemsema_cli_path_machine_failed
  StrCmp $0 0 chemsema_cli_path_machine_done chemsema_cli_path_machine_failed

  chemsema_cli_path_machine_done:
  DetailPrint "ChemSema CLI machine PATH registration succeeded."
  Goto chemsema_cli_path_notify

  chemsema_cli_path_machine_failed:
  DetailPrint "ChemSema CLI machine PATH registration failed; trying current-user PATH."
  ClearErrors
  ExecWait 'powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$PLUGINSDIR\chemsema-path.ps1" Add "$3" User' $0
  IfErrors chemsema_cli_path_user_failed
  StrCmp $0 0 chemsema_cli_path_user_done chemsema_cli_path_user_failed

  chemsema_cli_path_user_done:
  DetailPrint "ChemSema CLI current-user PATH registration succeeded."
  Goto chemsema_cli_path_notify

  chemsema_cli_path_user_failed:
  DetailPrint "ChemSema CLI PATH registration failed. App Paths registration may still allow ShellExecute launchers to find chemsema-cli.exe."
  Goto chemsema_cli_path_done

  chemsema_cli_path_notify:
  SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000

  chemsema_cli_path_done:
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DetailPrint "Unregistering ChemSema Office/OLE integration..."
  IfFileExists "$INSTDIR\chemsema-office.exe" chemsema_office_uninstall_found_root
  IfFileExists "$INSTDIR\resources\chemsema-office.exe" chemsema_office_uninstall_found_resources
  DetailPrint "ChemSema Office/OLE unregistration skipped: chemsema-office.exe was not found."
  Goto chemsema_office_uninstall_done

  chemsema_office_uninstall_found_root:
    StrCpy $1 "$INSTDIR\chemsema-office.exe"
    Goto chemsema_office_unregister

  chemsema_office_uninstall_found_resources:
    StrCpy $1 "$INSTDIR\resources\chemsema-office.exe"

  chemsema_office_unregister:
  ClearErrors
  ExecWait '"$1" --unregister-machine' $0
  IfErrors 0 chemsema_office_unregister_user
  DetailPrint "ChemSema Office/OLE machine unregistration could not launch."

  chemsema_office_unregister_user:
  ClearErrors
  ExecWait '"$1" --unregister-user' $0
  IfErrors 0 chemsema_office_uninstall_done
  DetailPrint "ChemSema Office/OLE current-user unregistration could not launch."

  chemsema_office_uninstall_done:

  DetailPrint "Unregistering ChemSema CLI app path..."
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\chemsema-cli.exe"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\App Paths\chemsema-cli.exe"

  DetailPrint "Removing ChemSema CLI directories from PATH..."
  !insertmacro CHEMSEMA_WRITE_PATH_HELPER
  ClearErrors
  ExecWait 'powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$PLUGINSDIR\chemsema-path.ps1" Remove "$INSTDIR" Machine' $0
  ClearErrors
  ExecWait 'powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$PLUGINSDIR\chemsema-path.ps1" Remove "$INSTDIR\resources" Machine' $0
  ClearErrors
  ExecWait 'powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$PLUGINSDIR\chemsema-path.ps1" Remove "$INSTDIR" User' $0
  ClearErrors
  ExecWait 'powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$PLUGINSDIR\chemsema-path.ps1" Remove "$INSTDIR\resources" User' $0
  SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000

  Delete "$INSTDIR\chemsema-cli.exe"
!macroend
