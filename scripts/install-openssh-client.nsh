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
;   4. If everything above failed, query the GitHub API for the latest
;      Win32-OpenSSH release and download the Win64 zip. This handles
;      WSUS lockdown and stripped Windows feature payloads on machines
;      that DO have internet access to GitHub.
;   5. If even GitHub access fails, show the manual remediation.

!macro NSIS_HOOK_PREINSTALL
  ; Write the PowerShell script to the installer's plugin dir.
  GetTempFileName $0
  Rename "$0" "$PLUGINSDIR\install-openssh-client.ps1"
  SetFileAttributes "$PLUGINSDIR\install-openssh-client.ps1" NORMAL
  FileOpen $1 "$PLUGINSDIR\install-openssh-client.ps1" w
  FileWrite $1 '$$ErrorActionPreference = "Continue"$\r$\n'
  FileWrite $1 '$$logPath = Join-Path $$env:TEMP "antminer-fleet-openssh-install.log"$\r$\n'
  FileWrite $1 'try { Remove-Item -Path $$logPath -ErrorAction SilentlyContinue } catch {}$\r$\n'
  FileWrite $1 'function Write-Log($$msg) { Write-Output $$msg; Add-Content -Path $$logPath -Value $$msg }$\r$\n'
  FileWrite $1 '$$installedDir = Join-Path $$env:ProgramFiles "Antminer Fleet Manager\OpenSSH"$\r$\n'
  FileWrite $1 '$$installedDirX86 = (Join-Path $${env:ProgramFiles(x86)} "Antminer Fleet Manager\OpenSSH")$\r$\n'
  FileWrite $1 '$$cap = Get-WindowsCapability -Online -Name "OpenSSH.Client~~~~0.0.1.0" 2>$$null$\r$\n'
  FileWrite $1 'if ($$cap -and $$cap.State -ne "Installed") {$\r$\n'
  FileWrite $1 '  Write-Log "Adding Windows capability OpenSSH.Client..."$\r$\n'
  FileWrite $1 '  try {$\r$\n'
  FileWrite $1 '    $$result = Add-WindowsCapability -Online -Name "OpenSSH.Client~~~~0.0.1.0" -ErrorAction Stop 2>&1$\r$\n'
  FileWrite $1 '    Write-Log ($$result | Out-String)$\r$\n'
  FileWrite $1 '    $$ec = $$LASTEXITCODE$\r$\n'
  FileWrite $1 '    Write-Log "Add-WindowsCapability exit code: $$ec"$\r$\n'
  FileWrite $1 '  } catch {$\r$\n'
  FileWrite $1 '    Write-Log "Add-WindowsCapability threw: $$_"$\r$\n'
  FileWrite $1 '    $$ec = 1$\r$\n'
  FileWrite $1 '  }$\r$\n'
  FileWrite $1 '} else {$\r$\n'
  FileWrite $1 '  $$ec = 0$\r$\n'
  FileWrite $1 '  Write-Log "OpenSSH.Client capability already Installed (or capability discovery unavailable)."$\r$\n'
  FileWrite $1 '}$\r$\n'
  FileWrite $1 'if (-not (Get-Command ssh.exe -ErrorAction SilentlyContinue)) {$\r$\n'
  FileWrite $1 '  $$candidates = @("$$env:SystemRoot\System32\OpenSSH\ssh.exe", "$$env:SystemRoot\SysWOW64\OpenSSH\ssh.exe")$\r$\n'
  FileWrite $1 '  foreach ($$candidate in $$candidates) {$\r$\n'
  FileWrite $1 '    if (Test-Path $$candidate) {$\r$\n'
  FileWrite $1 '      $$openSshDir = Split-Path $$candidate -Parent$\r$\n'
  FileWrite $1 '      $$userPath = [Environment]::GetEnvironmentVariable("Path", "User")$\r$\n'
  FileWrite $1 '      if (-not ($$userPath -like "*$$openSshDir*")) {$\r$\n'
  FileWrite $1 '        [Environment]::SetEnvironmentVariable("Path", "$$userPath;$$openSshDir", "User")$\r$\n'
  FileWrite $1 '        Write-Log "Added $$openSshDir to user PATH."$\r$\n'
  FileWrite $1 '      }$\r$\n'
  FileWrite $1 '      break$\r$\n'
  FileWrite $1 '    }$\r$\n'
  FileWrite $1 '  }$\r$\n'
  FileWrite $1 '}$\r$\n'
  FileWrite $1 'if (-not (Get-Command ssh.exe -ErrorAction SilentlyContinue)) {$\r$\n'
  FileWrite $1 '  $$target = $$installedDir$\r$\n'
  FileWrite $1 '  if (-not (Test-Path "$$target")) {$$target = $$installedDirX86}$\r$\n'
  FileWrite $1 '  Write-Log "Capability install did not surface ssh.exe. Querying GitHub for latest Win32-OpenSSH release..."$\r$\n'
  FileWrite $1 '  try {$\r$\n'
  FileWrite $1 '    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12$\r$\n'
  FileWrite $1 '    $$releases = Invoke-RestMethod -Uri "https://api.github.com/repos/PowerShell/Win32-OpenSSH/releases" -UseBasicParsing -ErrorAction Stop$\r$\n'
  FileWrite $1 '    Write-Log "Got $$($$releases.Count) releases from GitHub"$\r$\n'
  FileWrite $1 '  } catch {$\r$\n'
  FileWrite $1 '    Write-Log "Could not query GitHub API: $$_"$\r$\n'
  FileWrite $1 '    $$releases = @()$\r$\n'
  FileWrite $1 '  }$\r$\n'
  FileWrite $1 '  $$asset = $$null$\r$\n'
  FileWrite $1 '  foreach ($$rel in $$releases) {$\r$\n'
  FileWrite $1 '    foreach ($$a in $$rel.assets) {$\r$\n'
  FileWrite $1 '      if ($$a.name -eq "OpenSSH-Win64.zip") {$$asset = $$a; break}\r$\n'
  FileWrite $1 '    }$\r$\n'
  FileWrite $1 '    if ($$asset) {break}\r$\n'
  FileWrite $1 '  }$\r$\n'
  FileWrite $1 '  if ($$asset) {$\r$\n'
  FileWrite $1 '    Write-Log "Downloading $$($asset.browser_download_url)"$\r$\n'
  FileWrite $1 '    $$zipPath = Join-Path $$env:TEMP "antminer-fleet-openssh.zip"$\r$\n'
  FileWrite $1 '    try {$\r$\n'
  FileWrite $1 '      Invoke-WebRequest -Uri $$asset.browser_download_url -OutFile $$zipPath -UseBasicParsing -ErrorAction Stop$\r$\n'
  FileWrite $1 '      Write-Log "Extracting to $$target"$\r$\n'
  FileWrite $1 '      New-Item -ItemType Directory -Path $$target -Force | Out-Null$\r$\n'
  FileWrite $1 '      Add-Type -AssemblyName System.IO.Compression.FileSystem$\r$\n'
  FileWrite $1 '      [System.IO.Compression.ZipFile]::ExtractToDirectory($$zipPath, $$target)$\r$\n'
  FileWrite $1 '      $$userPath = [Environment]::GetEnvironmentVariable("Path", "User")$\r$\n'
  FileWrite $1 '      if (-not ($$userPath -like "*$$target*")) {$\r$\n'
  FileWrite $1 '        [Environment]::SetEnvironmentVariable("Path", "$$userPath;$$target", "User")$\r$\n'
  FileWrite $1 '        Write-Log "Added $$target to user PATH."$\r$\n'
  FileWrite $1 '      }$\r$\n'
  FileWrite $1 '      $$env:Path = "$$env:Path;$$target"$\r$\n'
  FileWrite $1 '      Remove-Item $$zipPath -ErrorAction SilentlyContinue$\r$\n'
  FileWrite $1 '    } catch {$\r$\n'
  FileWrite $1 '      Write-Log "Download or extract failed: $$_"$\r$\n'
  FileWrite $1 '    }$\r$\n'
  FileWrite $1 '  } else {$\r$\n'
  FileWrite $1 '    Write-Log "Could not find OpenSSH-Win64.zip in any GitHub release."$\r$\n'
  FileWrite $1 '  }$\r$\n'
  FileWrite $1 '}$\r$\n'
  FileWrite $1 'if (-not (Get-Command ssh.exe -ErrorAction SilentlyContinue)) {$\r$\n'
  FileWrite $1 '  Write-Log "ssh.exe still not available after capability install, system scan, and GitHub download."$\r$\n'
  FileWrite $1 '  exit 42$\r$\n'
  FileWrite $1 '}$\r$\n'
  FileWrite $1 'Write-Log "OpenSSH Client ready."$\r$\n'
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

  ; 5-minute outer timeout (allows for GitHub download).
  nsExec::ExecToLog '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -ExecutionPolicy Bypass -File "$PLUGINSDIR\install-openssh-client.ps1"'
  Pop $3
  ${If} $3 != 0
    MessageBox MB_ICONSTOP|MB_OK "Windows OpenSSH Client could not be installed.$\r$\n$\r$\nThe installer tried the Windows Optional Feature, the system scan, and downloading the official Microsoft OpenSSH release from GitHub. None surfaced ssh.exe.$\r$\n$\r$\nMost common cause: corporate policy is blocking the feature source AND GitHub access.$\r$\n$\r$\nWorkaround: install OpenSSH Client from Windows Settings -> Apps -> Optional features, then run this installer again.$\r$\n$\r$\nPowerShell exit code: $3$\r$\nLog: %TEMP%\antminer-fleet-openssh-install.log"
    Abort
  ${EndIf}

done:
!macroend
