@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0fleet-tunnel.ps1" Status
pause
