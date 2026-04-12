# uninstall.ps1 - Uninstall xmp-reader property handler
#
# Run as Administrator. Reverses what install.ps1 did:
#   1. Unregisters the DLL (restores old handlers, unregisters .propdesc schema)
#   2. Removes %ProgramFiles%\xmp-reader\
#   3. Restarts Explorer
#
# Usage:
#   .\uninstall.ps1

$ErrorActionPreference = 'Stop'

# --- Check admin ---
$identity  = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [Security.Principal.WindowsPrincipal]$identity
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Error "This script must be run as Administrator."
    exit 1
}

$installDir = "$env:ProgramFiles\xmp-reader"
$dll        = Join-Path $installDir "xmp_reader.dll"

# --- Stop processes that hold the DLL ---
Write-Host "Stopping prevhost.exe (if running) ..."
Stop-Process -Name prevhost -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 400

# --- Unregister ---
if (Test-Path $dll) {
    Write-Host "Unregistering handler ..."
    $proc = Start-Process regsvr32.exe -ArgumentList "/u /s `"$dll`"" -Wait -PassThru
    if ($proc.ExitCode -ne 0) {
        Write-Warning "regsvr32 /u returned exit code $($proc.ExitCode) - continuing anyway."
    }
} else {
    Write-Host "DLL not found at $dll - skipping unregistration."
}

# --- Stop Explorer to release file locks ---
Write-Host "Stopping Explorer ..."
Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 800

# --- Remove files ---
if (Test-Path $installDir) {
    Write-Host "Removing $installDir ..."
    Remove-Item $installDir -Recurse -Force
}

# --- Restart Explorer ---
Write-Host "Starting Explorer ..."
Start-Process explorer.exe

Write-Host ""
Write-Host "xmp-reader uninstalled successfully."
