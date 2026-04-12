# install.ps1 — xmp-reader sandbox smoke-test installer
# Registers the xmp_reader property handler DLL, then opens Explorer to the
# test-fixtures folder so you can verify XMP fields appear in the Details pane.
#
# Run automatically by smoke-test.wsb on sandbox login.
# Can also be run manually inside the sandbox for re-registration.

$ErrorActionPreference = 'Stop'

$dll      = "C:\xmp-reader\bin\xmp_reader.dll"
$fixtures = "C:\xmp-reader\sandbox\test-fixtures"

# --- Verify DLL exists -------------------------------------------------------

if (-not (Test-Path $dll)) {
    $msg = "DLL not found:`n$dll`n`nBuild the project first:`n  cargo build --release"
    [System.Windows.Forms.MessageBox]::Show($msg, "xmp-reader smoke test", 0, 16) | Out-Null
    Write-Error "DLL not found: $dll"
    exit 1
}

# --- Register the property handler -------------------------------------------

Write-Host "Registering $dll ..."
$reg = Start-Process regsvr32.exe -ArgumentList "/s `"$dll`"" -Wait -PassThru

if ($reg.ExitCode -ne 0) {
    $msg = "regsvr32 failed (exit code $($reg.ExitCode)).`nCheck that the DLL exports DllRegisterServer."
    [System.Windows.Forms.MessageBox]::Show($msg, "xmp-reader smoke test", 0, 16) | Out-Null
    Write-Error "regsvr32 failed"
    exit $reg.ExitCode
}

Write-Host "Registration succeeded."

# --- Restart Explorer so the new handler is picked up ------------------------

Write-Host "Restarting Explorer ..."
Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 800
Start-Process explorer.exe

# --- Open test-fixtures folder -----------------------------------------------

Write-Host "Opening test-fixtures in Explorer ..."
Start-Sleep -Milliseconds 500
Start-Process explorer.exe $fixtures

Write-Host "Done. Check the Details pane (View > Details pane) for XMP fields."
