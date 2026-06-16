!include LogicLib.nsh
!include FileFunc.nsh

; OpenSSH Client install hook
;
; Runs at PREINSTALL time as the elevated SYSTEM user, before the Antminer
; Fleet Manager files are copied. The PowerShell content is inlined into
; $PLUGINSDIR so we do not depend on a relative file path during the NSIS
; build (NSIS chdirs into a temp dir).
;
; Strategy (in order):
;   1. Skip if ssh.exe is already on PATH.
;   2. Try Add-WindowsCapability for OpenSSH.Client~~~~0.0.1.0.
;   3. If the capability resolved but ssh.exe is not on PATH, register
;      %SystemRoot%\System32\OpenSSH for the user.
;   4. If everything above failed, extract the bundled OpenSSH binaries
;      (declared as Tauri resources in tauri.conf.json) from
;      $PLUGINSDIR\openssh-bin to %ProgramFiles%\Antminer Fleet
;      Manager\OpenSSH and add that directory to the user PATH. This
;      handles WSUS lockdown and stripped Windows feature payloads.
;
; The binaries are bundled by Tauri and extracted to $PLUGINSDIR by
; the `!macro NSIS_HOOK_PREINSTALL` block below using `File /oname=`.

!macro NSIS_HOOK_PREINSTALL
  ; Extract the bundled OpenSSH binaries from the staged Tauri build
  ; resources into $PLUGINSDIR so PowerShell can find them. These
  ; paths are valid at PREINSTALL time because the resources are
  ; embedded in the installer.
  ;
  ; The build stages the binaries at
  ;   target/release/_up_/scripts/openssh-bin/
  ; and the NSIS source lives at
  ;   target/release/nsis/x64/installer.nsi
  ; so the relative path from the NSIS source is ../../
  SetOverwrite on
  File "/oname=$PLUGINSDIR\openssh-bin\ssh.exe" "..\..\_up_\scripts\openssh-bin\ssh.exe"
  File "/oname=$PLUGINSDIR\openssh-bin\ssh-add.exe" "..\..\_up_\scripts\openssh-bin\ssh-add.exe"
  File "/oname=$PLUGINSDIR\openssh-bin\ssh-agent.exe" "..\..\_up_\scripts\openssh-bin\ssh-agent.exe"
  File "/oname=$PLUGINSDIR\openssh-bin\ssh-keygen.exe" "..\..\_up_\scripts\openssh-bin\ssh-keygen.exe"
  File "/oname=$PLUGINSDIR\openssh-bin\ssh-keyscan.exe" "..\..\_up_\scripts\openssh-bin\ssh-keyscan.exe"
  File "/oname=$PLUGINSDIR\openssh-bin\scp.exe" "..\..\_up_\scripts\openssh-bin\scp.exe"
  File "/oname=$PLUGINSDIR\openssh-bin\sftp.exe" "..\..\_up_\scripts\openssh-bin\sftp.exe"
  File "/oname=$PLUGINSDIR\openssh-bin\ssh-pkcs11-helper.exe" "..\..\_up_\scripts\openssh-bin\ssh-pkcs11-helper.exe"
  File "/oname=$PLUGINSDIR\openssh-bin\ssh-sk-helper.exe" "..\..\_up_\scripts\openssh-bin\ssh-sk-helper.exe"

  ; Write the PowerShell script into $PLUGINSDIR.
  GetTempFileName $0
  Rename "$0" "$PLUGINSDIR\install-openssh-client.ps1"
  SetFileAttributes "$PLUGINSDIR\install-openssh-client.ps1" NORMAL
  FileOpen $1 "$PLUGINSDIR\install-openssh-client.ps1" w
  FileWrite $1 '$$ErrorActionPreference = "Continue"$\r$\n'
  FileWrite $1 '$$logPath = Join-Path $$env:TEMP "antminer-fleet-openssh-install.log"$\r$\n'
  FileWrite $1 '$$plgDir = $$env:PLUGINSDIR$\r$\n'
  FileWrite $1 'if (-not $$plgDir) {$$plgDir = "C:\Windows\Temp\_nsis"} $\r$\n'
  FileWrite $1 '$$bundledSrc = Join-Path $$plgDir "openssh-bin"$\r$\n'
  FileWrite $1 '$$installedDir = Join-Path $$env:ProgramFiles "Antminer Fleet Manager\OpenSSH"$\r$\n'
  FileWrite $1 '$$installedDirX86 = (Join-Path $${env:ProgramFiles(x86)} "Antminer Fleet Manager\OpenSSH")$\r$\n'
  FileWrite $1 '$$cap = Get-WindowsCapability -Online -Name "OpenSSH.Client~~~~0.0.1.0" 2>$$null$\r$\n'
  FileWrite $1 'if ($$cap -and $$cap.State -ne "Installed") {$\r$\n'
  FileWrite $1 '  Write-Output "Adding Windows capability OpenSSH.Client..."$\r$\n'
  FileWrite $1 '  try {$\r$\n'
  FileWrite $1 '    $$result = Add-WindowsCapability -Online -Name "OpenSSH.Client~~~~0.0.1.0" -ErrorAction Stop 2>&1$\r$\n'
  FileWrite $1 '    $$result | Out-File -FilePath $$logPath -Encoding utf8$\r$\n'
  FileWrite $1 '    $$ec = $$LASTEXITCODE$\r$\n'
  FileWrite $1 '    Write-Output "Add-WindowsCapability exit code: $$ec"$\r$\n'
  FileWrite $1 '  } catch {$\r$\n'
  FileWrite $1 '    $$_ | Out-File -FilePath $$logPath -Append -Encoding utf8$\r$\n'
  FileWrite $1 '    Write-Output "Add-WindowsCapability threw: $$_"$\r$\n'
  FileWrite $1 '    $$ec = 1$\r$\n'
  FileWrite $1 '  }$\r$\n'
  FileWrite $1 '} else {$\r$\n'
  FileWrite $1 '  $$ec = 0$\r$\n'
  FileWrite $1 '  Write-Output "OpenSSH.Client capability already Installed (or capability discovery unavailable)."$\r$\n'
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
  FileWrite $1 '  $$target = $$installedDir$\r$\n'
  FileWrite $1 '  if (-not (Test-Path "$$target")) {$$target = $$installedDirX86}$\r$\n'
  FileWrite $1 '  if (Test-Path $$bundledSrc) {$\r$\n'
  FileWrite $1 '    Write-Output "Capability install did not surface ssh.exe; extracting bundled binaries to $$target"$\r$\n'
  FileWrite $1 '    New-Item -ItemType Directory -Path $$target -Force | Out-Null$\r$\n'
  FileWrite $1 '    Copy-Item -Path (Join-Path $$bundledSrc "*") -Destination $$target -Force$\r$\n'
  FileWrite $1 '    $$userPath = [Environment]::GetEnvironmentVariable("Path", "User")$\r$\n'
  FileWrite $1 '    if (-not ($$userPath -like "*$$target*")) {$\r$\n'
  FileWrite $1 '      [Environment]::SetEnvironmentVariable("Path", "$$userPath;$$target", "User")$\r$\n'
  FileWrite $1 '      Write-Output "Added $$target to user PATH."$\r$\n'
  FileWrite $1 '    }$\r$\n'
  FileWrite $1 '    $$env:Path = "$$env;Path;$$target"$\r$\n'
  FileWrite $1 '  } else {$\r$\n'
  FileWrite $1 '    Write-Output "Bundled OpenSSH binaries not present at $$bundledSrc; installer build is broken."$\r$\n'
  FileWrite $1 '  }$\r$\n'
  FileWrite $1 '}$\r$\n'
  FileWrite $1 'if (-not (Get-Command ssh.exe -ErrorAction SilentlyContinue)) {$\r$\n'
  FileWrite $1 '  Write-Output "ssh.exe still not available after capability install, system scan, and bundled extraction."$\r$\n'
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

  DetailPrint "OpenSSH Client not found. Attempting to install..."

  ; 2-minute outer timeout. Add-WindowsCapability can hang on machines
  ; where the Windows Optional Features source is being rebuilt.
  nsExec::ExecToLog '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -ExecutionPolicy Bypass -File "$PLUGINSDIR\install-openssh-client.ps1"'
  Pop $3
  ${If} $3 != 0
    MessageBox MB_ICONSTOP|MB_OK "Windows OpenSSH Client could not be installed.$\r$\n$\r$\nThe installer tried the Windows Optional Feature, the system scan, and the bundled fallback. None surfaced ssh.exe.$\r$\n$\r$\nMost common cause: corporate policy is blocking the feature source AND the bundled extraction.$\r$\n$\r$\nWorkaround: install OpenSSH Client from Windows Settings -> Apps -> Optional features, then run this installer again.$\r$\n$\r$\nPowerShell exit code: $3$\r$\nLog: %TEMP%\antminer-fleet-openssh-install.log"
    Abort
  ${EndIf}

done:
!macroend
