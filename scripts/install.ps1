# install.ps1 - Install xmp-reader property handler
#
# Run as Administrator from the directory containing xmp_reader.dll and
# xmp-sidecar.propdesc (e.g. the release zip after extraction).
#
# What it does:
#   1. Copies DLL + .propdesc to %ProgramFiles%\xmp-reader\
#   2. Registers the DLL via regsvr32 (writes CLSID, property handler keys,
#      saves old handler CLSIDs, registers .propdesc schema)
#   3. Restarts Explorer so the new handler takes effect
#
# Usage:
#   .\install.ps1
#   .\install.ps1 -SourceDir "C:\path\to\extracted\release"

[CmdletBinding()]
param(
    [string]$SourceDir = $PSScriptRoot
)

$ErrorActionPreference = 'Stop'

# --- Check admin ---
$identity  = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [Security.Principal.WindowsPrincipal]$identity
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Error "This script must be run as Administrator."
    exit 1
}

$installDir = "$env:ProgramFiles\xmp-reader"
$dll        = "xmp_reader.dll"
$propdesc   = "xmp-sidecar.propdesc"

# --- Validate source files ---
$srcDll      = Join-Path $SourceDir $dll
$srcPropdesc = Join-Path $SourceDir $propdesc

if (-not (Test-Path $srcDll)) {
    Write-Error "DLL not found: $srcDll"
    exit 1
}
if (-not (Test-Path $srcPropdesc)) {
    Write-Error "Property schema not found: $srcPropdesc"
    exit 1
}

# --- Stop processes that hold the DLL ---
Write-Host "Stopping prevhost.exe (if running) ..."
Stop-Process -Name prevhost -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 400

# --- Unregister old copy if present ---
$existingDll = Join-Path $installDir $dll
if (Test-Path $existingDll) {
    Write-Host "Unregistering previous installation ..."
    Start-Process regsvr32.exe -ArgumentList "/u /s `"$existingDll`"" -Wait | Out-Null
}

# --- Copy files ---
Write-Host "Installing to $installDir ..."
if (-not (Test-Path $installDir)) {
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
}
Copy-Item $srcDll      -Destination $installDir -Force
Copy-Item $srcPropdesc -Destination $installDir -Force

# Copy install/uninstall scripts for later use.
$srcInstall   = Join-Path $SourceDir "install.ps1"
$srcUninstall = Join-Path $SourceDir "uninstall.ps1"
if (Test-Path $srcInstall)   { Copy-Item $srcInstall   -Destination $installDir -Force }
if (Test-Path $srcUninstall) { Copy-Item $srcUninstall -Destination $installDir -Force }

# --- Register ---
$installedDll = Join-Path $installDir $dll
Write-Host "Registering handler ..."
$proc = Start-Process regsvr32.exe -ArgumentList "/s `"$installedDll`"" -Wait -PassThru
if ($proc.ExitCode -ne 0) {
    Write-Error "regsvr32 failed (exit code $($proc.ExitCode))."
    exit $proc.ExitCode
}

# --- Restart Explorer ---
Write-Host "Restarting Explorer ..."
Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 800
Start-Process explorer.exe

Write-Host ""
Write-Host "xmp-reader installed successfully."
Write-Host "  Location: $installDir"
Write-Host "  To uninstall: run uninstall.ps1 as Administrator"
