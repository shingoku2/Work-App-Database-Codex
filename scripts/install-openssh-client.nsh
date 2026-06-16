!include LogicLib.nsh

!macro NSIS_HOOK_PREINSTALL
  DetailPrint "Checking Windows OpenSSH Client..."
  ExecWait `"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -ExecutionPolicy Bypass -Command "if (-not (Get-Command ssh.exe -ErrorAction SilentlyContinue)) { $$cap = Get-WindowsCapability -Online -Name 'OpenSSH.Client~~~~0.0.1.0'; if ($$cap.State -ne 'Installed') { Add-WindowsCapability -Online -Name 'OpenSSH.Client~~~~0.0.1.0' | Out-Null } }; if (-not (Get-Command ssh.exe -ErrorAction SilentlyContinue)) { exit 42 }"` $0
  ${If} $0 != 0
    MessageBox MB_ICONSTOP|MB_OK "Windows OpenSSH Client could not be installed or found. Install OpenSSH Client from Windows Optional Features, then run this installer again. Exit code: $0"
    Abort
  ${EndIf}
!macroend
