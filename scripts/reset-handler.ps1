# reset-handler.ps1 - xmp-reader dev helper
#
# Manages the property handler DLL registration and Explorer/prevhost process
# lifecycle during development. Use this constantly during M1+ to pick up DLL
# changes without rebooting the VM.
#
# MODES
# -----
#   (no args)       Kill prevhost.exe + restart Explorer. No registration change.
#                   Use after any change that doesn't affect COM registration
#                   (e.g. logic-only changes where DLL is already registered).
#
#   -Release        Kill prevhost.exe only (releases the DLL file lock).
#                   Run this BEFORE `cargo build --release` so the linker can
#                   overwrite the DLL. Does not touch Explorer.
#
#   -Install        Unregister old DLL, register new DLL, reset Explorer+prevhost.
#                   Run this AFTER `cargo build --release` when registration
#                   entries may have changed, or for a first install.
#
#   -Uninstall      Unregister the DLL and reset Explorer+prevhost.
#                   Use when removing the handler entirely (e.g. rolling back).
#
# EXAMPLES
# --------
#   # Typical dev loop:
#   .\reset-handler.ps1 -Release          # before build - release DLL lock
#   cargo build --release
#   .\reset-handler.ps1 -Install          # after build  - register + reset
#
#   # Quick Explorer refresh (no registration change):
#   .\reset-handler.ps1
#
#   # Use a non-default DLL path:
#   .\reset-handler.ps1 -Install -DllPath "C:\path\to\xmp_reader.dll"
#
# NOTES
# -----
#   - Must be run as Administrator (regsvr32 writes to HKLM).
#   - prevhost.exe is the surrogate process that hosts property handlers;
#     killing Explorer alone does not unload the DLL.
#   - After -Release, Explorer keeps running - build output is not locked by it.

[CmdletBinding(DefaultParameterSetName = 'Reset')]
param(
    [Parameter(ParameterSetName = 'Release', Mandatory)]
    [switch]$Release,

    [Parameter(ParameterSetName = 'Install', Mandatory)]
    [switch]$Install,

    [Parameter(ParameterSetName = 'Uninstall', Mandatory)]
    [switch]$Uninstall,

    [Parameter(ParameterSetName = 'Install')]
    [Parameter(ParameterSetName = 'Uninstall')]
    [string]$DllPath = "$PSScriptRoot\..\target\x86_64-pc-windows-msvc\release\xmp_reader.dll"
)

$ErrorActionPreference = 'Stop'

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

function Assert-Admin {
    $identity  = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]$identity
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        Write-Error "This script must be run as Administrator (regsvr32 requires HKLM write access)."
        exit 1
    }
}

function Stop-Prevhost {
    $procs = Get-Process prevhost -ErrorAction SilentlyContinue
    if ($procs) {
        Write-Host "Stopping prevhost.exe ($($procs.Count) instance(s)) ..."
        $procs | Stop-Process -Force
        Start-Sleep -Milliseconds 400
    } else {
        Write-Host "prevhost.exe not running - nothing to stop."
    }
}

function Reset-Explorer {
    Write-Host "Stopping Explorer ..."
    Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 800
    Write-Host "Starting Explorer ..."
    Start-Process explorer.exe
}

function Invoke-Regsvr32 ([string]$args) {
    $dll = $null
    $resolved = Resolve-Path $DllPath -ErrorAction SilentlyContinue
    if ($resolved) { $dll = $resolved.ProviderPath }
    if (-not $dll) {
        Write-Error "DLL not found: $DllPath"
        exit 1
    }
    Write-Host "regsvr32 $args `"$dll`" ..."
    $proc = Start-Process regsvr32.exe -ArgumentList "$args `"$dll`"" -Wait -PassThru
    if ($proc.ExitCode -ne 0) {
        Write-Error "regsvr32 failed (exit code $($proc.ExitCode))."
        exit $proc.ExitCode
    }
}

# ---------------------------------------------------------------------------
# Modes
# ---------------------------------------------------------------------------

switch ($PSCmdlet.ParameterSetName) {

    'Release' {
        # Release the DLL file lock so cargo can overwrite it.
        # Explorer keeps running - it does not lock the DLL directly.
        Stop-Prevhost
        Write-Host "DLL lock released. Safe to run: cargo build --release"
    }

    'Install' {
        Assert-Admin
        # Unregister first (ignore failure - DLL may not be registered yet).
        Write-Host "Unregistering existing handler (if any) ..."
        $dll = $null
        $resolved = Resolve-Path $DllPath -ErrorAction SilentlyContinue
        if ($resolved) { $dll = $resolved.ProviderPath }
        if ($dll) {
            Start-Process regsvr32.exe -ArgumentList "/u /s `"$dll`"" -Wait | Out-Null
        }
        Stop-Prevhost
        Invoke-Regsvr32 "/s"
        Reset-Explorer
        Write-Host "Done. Handler installed and Explorer restarted."
    }

    'Uninstall' {
        Assert-Admin
        Invoke-Regsvr32 "/u /s"
        Stop-Prevhost
        Reset-Explorer
        Write-Host "Done. Handler unregistered and Explorer restarted."
    }

    'Reset' {
        # Plain reset - no registration change.
        Stop-Prevhost
        Reset-Explorer
        Write-Host "Done. Explorer restarted."
    }
}
