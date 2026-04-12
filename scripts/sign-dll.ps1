# sign-dll.ps1 - xmp-reader dev helper
#
# Signs a DLL with the dev self-signed code-signing certificate.
# The cert must already exist in CurrentUser\My (see docs/dev-environment.md §8).
#
# USAGE
# -----
#   .\sign-dll.ps1                          # sign the default release DLL
#   .\sign-dll.ps1 -DllPath "C:\other.dll"  # sign a specific file
#   .\sign-dll.ps1 -Verify                  # verify the signature on the DLL
#
# NOTES
# -----
#   - Does NOT require Administrator (signing uses the user cert store).
#   - Requires signtool.exe on PATH (ships with Windows SDK / Visual Studio).
#   - The certificate subject is "xmp-reader dev" - change $CertSubject if you
#     used a different CN when creating the cert.

param(
    [string]$DllPath = "$PSScriptRoot\..\target\x86_64-pc-windows-msvc\release\xmp_reader.dll",

    [switch]$Verify
)

$ErrorActionPreference = 'Stop'

$CertSubject = "xmp-reader dev"

# ---------------------------------------------------------------------------
# Find signtool.exe
# ---------------------------------------------------------------------------

function Find-SignTool {
    # Try PATH first
    $cmd = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }

    # Search Windows SDK locations
    $roots = @(
        "${env:ProgramFiles(x86)}\Windows Kits\10\bin"
        "$env:ProgramFiles\Windows Kits\10\bin"
    )
    foreach ($root in $roots) {
        if (-not (Test-Path $root)) { continue }
        $found = Get-ChildItem $root -Recurse -Filter signtool.exe |
            Where-Object { $_.FullName -match 'x64' } |
            Sort-Object FullName -Descending |
            Select-Object -First 1
        if ($found) { return $found.FullName }
    }

    Write-Error "signtool.exe not found. Install the Windows SDK or add it to PATH."
    exit 1
}

# ---------------------------------------------------------------------------
# Resolve DLL path
# ---------------------------------------------------------------------------

$dll = $null
$resolved = Resolve-Path $DllPath -ErrorAction SilentlyContinue
if ($resolved) { $dll = $resolved.ProviderPath }
if (-not $dll) {
    Write-Error "DLL not found: $DllPath"
    exit 1
}

$signtool = Find-SignTool
Write-Host "Using signtool: $signtool"

# ---------------------------------------------------------------------------
# Verify mode
# ---------------------------------------------------------------------------

if ($Verify) {
    Write-Host "Verifying signature on: $dll"
    & $signtool verify /pa "$dll"
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Signature verification failed."
        exit 1
    }
    Write-Host "Signature OK."
    exit 0
}

# ---------------------------------------------------------------------------
# Sign
# ---------------------------------------------------------------------------

# Find the cert thumbprint
$cert = Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert |
    Where-Object { $_.Subject -match $CertSubject } |
    Sort-Object NotAfter -Descending |
    Select-Object -First 1

if (-not $cert) {
    Write-Error "No code-signing certificate with subject matching '$CertSubject' found in CurrentUser\My. Run the cert setup steps in docs/dev-environment.md section 8."
    exit 1
}

Write-Host "Signing: $dll"
Write-Host "  Cert:       $($cert.Subject)"
Write-Host "  Thumbprint: $($cert.Thumbprint)"
Write-Host "  Expires:    $($cert.NotAfter.ToString('yyyy-MM-dd'))"

& $signtool sign /fd SHA256 /sha1 $cert.Thumbprint /t http://timestamp.digicert.com "$dll"
if ($LASTEXITCODE -ne 0) {
    Write-Error "Signing failed (signtool exit code $LASTEXITCODE)."
    exit $LASTEXITCODE
}

Write-Host "Signed successfully."
