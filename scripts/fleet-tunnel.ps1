[CmdletBinding()]
param(
    [ValidateSet("Start", "Stop", "Status")]
    [string]$Action = "Start",
    [string]$Config = (Join-Path $PSScriptRoot "fleet-tunnel.local.json")
)

$ErrorActionPreference = "Stop"
$stateDirectory = Join-Path $env:LOCALAPPDATA "AntminerFleetManager"

function Read-TunnelConfig {
    if (-not (Test-Path -LiteralPath $Config)) {
        throw "Tunnel config not found: $Config. Copy fleet-tunnel.example.json to fleet-tunnel.local.json and set ssh_destination."
    }

    $settings = Get-Content -LiteralPath $Config -Raw | ConvertFrom-Json
    if (-not $settings.ssh_destination -or $settings.ssh_destination -match "CHANGE_ME") {
        throw "Set ssh_destination in $Config to USER@REMOTE_HOST or an SSH config host alias."
    }

    return $settings
}

function Get-ManagedProcess {
    if (-not (Test-Path -LiteralPath $pidPath)) {
        return $null
    }

    $savedPid = (Get-Content -LiteralPath $pidPath -Raw).Trim()
    if ($savedPid -notmatch "^\d+$") {
        Remove-Item -LiteralPath $pidPath -Force
        return $null
    }

    $process = Get-Process -Id ([int]$savedPid) -ErrorAction SilentlyContinue
    if (-not $process -or $process.ProcessName -ne "ssh") {
        Remove-Item -LiteralPath $pidPath -Force
        return $null
    }

    return $process
}

function Test-LocalPort {
    param([int]$Port)

    $client = [System.Net.Sockets.TcpClient]::new()
    try {
        $result = $client.BeginConnect("127.0.0.1", $Port, $null, $null)
        if (-not $result.AsyncWaitHandle.WaitOne(3000)) {
            return $false
        }
        $client.EndConnect($result)
        return $true
    }
    catch {
        return $false
    }
    finally {
        $client.Dispose()
    }
}

function Show-Status {
    param($Settings)

    $process = Get-ManagedProcess
    $portOpen = Test-LocalPort -Port ([int]$Settings.local_port)
    [pscustomobject]@{
        Running = [bool]$process
        ProcessId = if ($process) { $process.Id } else { $null }
        LocalUrl = "https://localhost:$($Settings.local_port)"
        LocalPortOpen = $portOpen
        RemoteTarget = "$($Settings.remote_host):$($Settings.remote_port)"
    }
}

$settings = Read-TunnelConfig
$pidPath = Join-Path $stateDirectory "ssh-tunnel-$($settings.local_port).pid"

switch ($Action) {
    "Status" {
        Show-Status -Settings $settings
    }
    "Stop" {
        $process = Get-ManagedProcess
        if ($process) {
            Stop-Process -Id $process.Id
            $process.WaitForExit(5000)
        }
        if (Test-Path -LiteralPath $pidPath) {
            Remove-Item -LiteralPath $pidPath -Force
        }
        Show-Status -Settings $settings
    }
    "Start" {
        $existing = Get-ManagedProcess
        if ($existing -and (Test-LocalPort -Port ([int]$settings.local_port))) {
            Show-Status -Settings $settings
            break
        }

        if (Test-LocalPort -Port ([int]$settings.local_port)) {
            throw "Local port $($settings.local_port) is already in use by another process."
        }

        New-Item -ItemType Directory -Path $stateDirectory -Force | Out-Null
        $arguments = @(
            "-N",
            "-T",
            "-o", "BatchMode=yes",
            "-o", "ExitOnForwardFailure=yes",
            "-o", "ServerAliveInterval=30",
            "-o", "ServerAliveCountMax=3",
            "-L", "$($settings.local_port):$($settings.remote_host):$($settings.remote_port)",
            [string]$settings.ssh_destination
        )
        if ($settings.ssh_port) {
            $arguments = @("-p", [string]$settings.ssh_port) + $arguments
        }
        if ($settings.identity_file) {
            $identityPath = [Environment]::ExpandEnvironmentVariables([string]$settings.identity_file)
            if (-not (Test-Path -LiteralPath $identityPath -PathType Leaf)) {
                throw "SSH identity file not found: $identityPath"
            }
            $arguments = @("-i", $identityPath, "-o", "IdentitiesOnly=yes") + $arguments
        }

        $process = Start-Process -FilePath "ssh.exe" -ArgumentList $arguments -WindowStyle Hidden -PassThru
        Set-Content -LiteralPath $pidPath -Value $process.Id

        $ready = $false
        for ($attempt = 0; $attempt -lt 20; $attempt++) {
            Start-Sleep -Milliseconds 500
            if ($process.HasExited) {
                Remove-Item -LiteralPath $pidPath -Force -ErrorAction SilentlyContinue
                throw "SSH exited before opening the tunnel. Verify key-based login and the SSH destination."
            }
            if (Test-LocalPort -Port ([int]$settings.local_port)) {
                $ready = $true
                break
            }
        }
        if (-not $ready) {
            Stop-Process -Id $process.Id -ErrorAction SilentlyContinue
            Remove-Item -LiteralPath $pidPath -Force -ErrorAction SilentlyContinue
            throw "SSH started but local port $($settings.local_port) did not open."
        }

        Show-Status -Settings $settings
    }
}

exit 0
