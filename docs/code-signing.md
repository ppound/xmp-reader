# Code Signing Guide (Dev & Production)

Code signing is required to run PowerShell scripts without bypassing the system execution policy, and to distribute your Windows DLLs without triggering SmartScreen or antivirus false-positives.

This guide covers both local development (using a free, self-signed certificate) and production distribution (using a real Authenticode certificate).

---

## 1. Local Development (Self-Signed)

For rapid local testing, you can generate your own certificate. Your PC will trust it, but other computers will not.

### A. Generate the Development Certificate

Run the following in an **elevated PowerShell** window to create the certificate and add it to your Trusted Root store:

```powershell
# Create the certificate (valid for 2 years)
$cert = New-SelfSignedCertificate `
    -Subject "CN=xmp-reader dev" `
    -Type CodeSigningCert `
    -CertStoreLocation Cert:\CurrentUser\My `
    -NotAfter (Get-Date).AddYears(2)

# Export and trust the certificate locally
Export-Certificate -Cert $cert -FilePath "$env:TEMP\xmp-reader-dev.cer" | Out-Null
Import-Certificate -FilePath "$env:TEMP\xmp-reader-dev.cer" -CertStoreLocation Cert:\LocalMachine\Root
Remove-Item "$env:TEMP\xmp-reader-dev.cer"
```

### B. Sign the PowerShell Scripts

Use PowerShell's built-in `Set-AuthenticodeSignature` cmdlet:

```powershell
# Retrieve your dev cert
$cert = Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert | 
    Where-Object { $_.Subject -match "xmp-reader dev" } | 
    Sort-Object NotAfter -Descending | Select-Object -First 1

# Sign your scripts
Set-AuthenticodeSignature -FilePath .\install.ps1 -Certificate $cert
Set-AuthenticodeSignature -FilePath .\uninstall.ps1 -Certificate $cert
Set-AuthenticodeSignature -FilePath .\scripts\reset-handler.ps1 -Certificate $cert
```

### C. Sign the DLL

You can sign the DLL using `signtool.exe` (installed with the Windows SDK/Visual Studio). Your project includes a helper script for this:

```powershell
.\scripts\sign-dll.ps1
```

*(You can also pass a specific path: `.\scripts\sign-dll.ps1 -DllPath "...\xmp_reader.dll"`)*

---

## 2. Production (Real Certificate)

To distribute your software to other users, you need a Publicly Trusted Code Signing Certificate. 

> [!WARNING]
> **The 2023 Rules Change:** In 2023, the CA/B Forum mandated that *all* Code Signing Certificates (both Standard and EV) must be stored on physical hardware tokens (like a YubiKey or a secure HSM). Because of this, traditional Standard Code Signing certificates are no longer easily downloadable files; they often cost $150–$300+ per year just to cover the shipping and hardware.

### The Most Economical Way: Azure Trusted Signing

Microsoft recently released **Azure Trusted Signing** (also known as Artifact Signing). Instead of sending you a physical USB stick, Microsoft securely maps your identity to Azure-hosted Cloud HSMs.

**Why it's the best option:**
- **Cost:** Starts at **$9.99/month** (Basic Plan, suitable for individuals). You can cancel when not needed. 
- **Convenience:** No physical USB sticks to lose or manage.
- **CI/CD Ready:** Super easy to automate via GitHub Actions or Azure DevOps without passing around private keys.

### A. How to Get It

1. You need an **Azure Subscription** (pay-as-you-go works fine).
2. Go to the Azure Portal and search for **Trusted Signing Account**.
3. Create a new Trusted Signing Account resource.
4. **Identity Validation:** You will be prompted to undergo vetting. For an individual, this means providing government ID and a biometric (selfie) validation.
5. Create a **Certificate Profile** under your Trusted Signing Account. This creates your actual digital identity that gets stamped on your software.
6. Grant your user account (or your Service Principal) the **"Trusted Signing Certificate Profile Signer"** RBAC role.

### B. How to Sign with Azure Trusted Signing

Whether you are signing the `.dll` or the `.ps1` files, Azure integrates directly into the standard Windows `signtool.exe`.

**1. Install the Client Tools:**
Install the Azure Trusted Signing Client Tooling (which gives `signtool` the ability to talk to Azure).

**2. Create a metadata file (`metadata.json`):**
```json
{
  "Endpoint": "https://<your-region>.codesigning.azure.net/",
  "CodeSigningAccountName": "<your-account-name>",
  "CertificateProfileName": "<your-profile-name>"
}
```

**3. Run SignTool:**
Because the tooling includes an Azure plug-in for `signtool.exe`, you literally just point it at the metadata file:

```powershell
# Sign the DLL
signtool.exe sign /v /fd SHA256 /tr http://timestamp.acs.microsoft.com /td SHA256 /d "xmp-reader" /metadata_file metadata.json "path\to\xmp_reader.dll"

# Sign the PowerShell scripts
signtool.exe sign /v /fd SHA256 /tr http://timestamp.acs.microsoft.com /td SHA256 /d "xmp-reader script" /metadata_file metadata.json "path\to\install.ps1"
```

> [!NOTE]
> If you automate your builds using GitHub Actions, you can simply use Microsoft's official `actions/trusted-signing` task, skipping the `metadata.json` entirely.
