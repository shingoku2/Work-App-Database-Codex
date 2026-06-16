!include LogicLib.nsh
!include FileFunc.nsh

; OpenSSH Client install hook
;
; Runs at PREINSTALL time as the elevated SYSTEM user, before the Antminer
; Fleet Manager files are copied. The PowerShell content is inlined into
; $PLUGINSDIR so we do not depend on a relative file path during the NSIS
; build (NSIS chdirs into a temp dir).
;
; The script:
;   1. Skips if ssh.exe is already on PATH.
;   2. Runs Get-WindowsCapability + Add-WindowsCapability for
;      OpenSSH.Client~~~~0.0.1.0.
;   3. If the capability install did not surface ssh.exe (common when
;      WSUS strips the payload), adds %SystemRoot%\System32\OpenSSH to
;      the user PATH as a fallback.
;   4. Returns 0 only if ssh.exe is reachable; 42 if it is not.
;
; Common failure modes:
;   - Corporate WSUS lockdown (payload not on the source server).
;   - No internet to the Windows Update feature source.
;   - Stripped Windows feature payload.
;
; The installer shows a clear message and aborts, asking the user to
; install "OpenSSH Client" from Windows Settings -> Apps -> Optional
; features manually.

!macro NSIS_HOOK_PREINSTALL
  ; Write the PowerShell script into $PLUGINSDIR.
  GetTempFileName $0
  Rename "$0" "$PLUGINSDIR\install-openssh-client.ps1"
  SetFileAttributes "$PLUGINSDIR\install-openssh-client.ps1" NORMAL
  FileOpen $1 "$PLUGINSDIR\install-openssh-client.ps1" w
  FileWrite $1 '$$ErrorActionPreference = "Continue"$\r$\n'
  FileWrite $1 '$$logPath = Join-Path $$env:TEMP "antminer-fleet-openssh-install.log"$\r$\n'
  FileWrite $1 '$$cap = Get-WindowsCapability -Online -Name "OpenSSH.Client~~~~0.0.1.0" 2>$$null$\r$\n'
  FileWrite $1 'if ($$cap -and $$cap.State -ne "Installed") {$\r$\n'
  FileWrite $1 '  Write-Output "Adding Windows capability OpenSSH.Client..."$\r$\n'
  FileWrite $1 '  try {$\r$\n'
  FileWrite $1 '    $$result = Add-WindowsCapability -Online -Name "OpenSSH.Client~~~~0.0.1.0" -ErrorAction Stop 2>&1$\r$\n'
  FileWrite $1 '    $$result | Out-File -FilePath $$logPath -Encoding utf8$\r$\n'
  FileWrite $1 '    $$ec = $$LASTEXITCODE$\r$\n'
  FileWrite $1 '    Write-Output "Add-WindowsCapability exit code: $$ec"$\r$\n'
  FileWrite $1 '    if ($$ec -ne 0) { exit $$ec }$\r$\n'
  FileWrite $1 '  } catch {$\r$\n'
  FileWrite $1 '    $$_ | Out-File -FilePath $$logPath -Append -Encoding utf8$\r$\n'
  FileWrite $1 '    Write-Output "Add-WindowsCapability threw: $$_"$\r$\n'
  FileWrite $1 '    exit 1$\r$\n'
  FileWrite $1 '  }$\r$\n'
  FileWrite $1 '} else {$\r$\n'
  FileWrite $1 '  Write-Output "OpenSSH.Client capability already Installed (or not found by Get-WindowsCapability)."$\r$\n'
  FileWrite $1 '}$\r$\n'
  FileWrite $1 'if (-not (Get-Command ssh.exe -ErrorAction SilentlyContinue)) {$\r$\n'
  FileWrite $1 '  $$candidates = @("$$env:SystemRoot\System32\OpenSSH\ssh.exe", "$$env:SystemRoot\SysWOW64\OpenSSH\ssh.exe")$\r$\n'
  FileWrite $1 '  foreach ($$candidate in $$candidates) {$\r$\n'
  FileWrite $1 '    if (Test-Path $$candidate) {$\r$\n'
  FileWrite $1 '      $$openSshDir = Split-Path $$candidate -Parent$\r$\n'
  FileWrite $1 '      $$userPath = [Environment]::GetEnvironmentVariable("Path", "User")$\r$\n'
  FileWrite $1 '      if (-not ($$userPath -like "*$$openSshDir*")) {$\r$\n'
  FileWrite $1 '        [Environment]::SetEnvironmentVariable("Path", "$$userPath;$$openSshDir", "User")$\r$\n'
  FileWrite $1 '        Write-Output "Added $$openSshDir to user PATH."$\r$\n'
  FileWrite $1 '      }$\r$\n'
  FileWrite $1 '      break$\r$\n'
  FileWrite $1 '    }$\r$\n'
  FileWrite $1 '  }$\r$\n'
  FileWrite $1 '}$\r$\n'
  FileWrite $1 'if (-not (Get-Command ssh.exe -ErrorAction SilentlyContinue)) {$\r$\n'
  FileWrite $1 '  Write-Output "ssh.exe still not available after capability install and fallback scan."$\r$\n'
  FileWrite $1 '  exit 42$\r$\n'
  FileWrite $1 '}$\r$\n'
  FileWrite $1 'Write-Output "OpenSSH Client ready."$\r$\n'
  FileWrite $1 'exit 0$\r$\n'
  FileClose $1

  ; Quick check: is ssh.exe already on PATH?
  nsExec::ExecToStack 'cmd.exe /c where ssh.exe'
  Pop $2
  ${If} $2 == 0
    DetailPrint "OpenSSH Client already on PATH."
    Goto done
  ${EndIf}

  DetailPrint "OpenSSH Client not found. Attempting to add Windows capability..."

  ; 2-minute outer timeout. Add-WindowsCapability can hang on machines
  ; where the Windows Optional Features source is being rebuilt.
  nsExec::ExecToLog '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -ExecutionPolicy Bypass -File "$PLUGINSDIR\install-openssh-client.ps1"'
  Pop $3
  ${If} $3 != 0
    MessageBox MB_ICONSTOP|MB_OK "Windows OpenSSH Client could not be installed automatically.$\r$\n$\r$\nCommon causes: corporate WSUS lockdown, no internet, or stripped Windows feature payload.$\r$\n$\r$\nWorkaround: install OpenSSH Client from Windows Settings -> Apps -> Optional features, then run this installer again.$\r$\n$\r$\nPowerShell exit code: $3"
    Abort
  ${EndIf}

done:
!macroend
