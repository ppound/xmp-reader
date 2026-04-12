# xmp-reader — Dev Environment Setup

Reproducible guide for getting from a fresh clone to a working M1+ dev environment.
Goal: under an hour on a good day (excluding Windows Update).

> **Status:** Sections marked `[TODO]` are placeholders — fill them in as Tasks #4–#8 complete.

---

## Prerequisites (host machine)

- Windows 11 Pro
- Hyper-V enabled (see below)
- The project cloned to `C:\Users\<you>\claude-code\xmp-reader` (or adjust paths throughout)

---

## 1. Enable Hyper-V

In an elevated PowerShell:

```powershell
Enable-WindowsOptionalFeature -Online -FeatureName Microsoft-Hyper-V-All -NoRestart
Restart-Computer
```

Or via Settings → System → Optional features → More Windows features → check
**Hyper-V** → OK → reboot.

Confirm after reboot:

```powershell
(Get-CimInstance Win32_ComputerSystem).HyperVisorPresent  # should be True
```

---

## 2. Create the Windows 11 dev VM

Microsoft does **not** offer a standalone VHDX download. Use **Hyper-V Quick Create**:

1. Open **Hyper-V Manager** (Start → search "Hyper-V Manager")
2. Click **Quick Create** in the Actions panel (right side)
3. Select **Windows 11 development environment** from the gallery
4. Click **Create Virtual Machine** — the app handles the download (~20 GB) and VM creation

The image comes with Visual Studio and the Windows 11 SDK pre-installed.

---

## 3. Fix VM networking on Wi-Fi hosts

The Default Switch is unreliable when the host is on Wi-Fi (known issue on Lenovo
and other hardware). Use a manual NAT switch instead.

**On the host** (elevated PowerShell, run once):

```powershell
# Create an internal switch
New-VMSwitch -Name "NATSwitch" -SwitchType Internal

# Assign an IP to the host side
New-NetIPAddress -IPAddress 192.168.100.1 -PrefixLength 24 -InterfaceAlias "vEthernet (NATSwitch)"

# Create the NAT rule
New-NetNat -Name "NATNetwork" -InternalIPInterfaceAddressPrefix 192.168.100.0/24
```

**In Hyper-V Manager:**

VM Settings → Network Adapter → switch from "Default Switch" to **NATSwitch** → Apply.

**Inside the VM** (elevated PowerShell, run once):

```powershell
$idx = (Get-NetAdapter | Where-Object Status -eq Up).ifIndex
New-NetIPAddress -InterfaceIndex $idx -IPAddress 192.168.100.10 -PrefixLength 24 -DefaultGateway 192.168.100.1
Set-DnsClientServerAddress -InterfaceIndex $idx -ServerAddresses 8.8.8.8
```

Verify:

```powershell
Test-NetConnection google.com -InformationLevel Quiet  # should return True
```

---

## 4. First boot and Windows Update

Boot the VM, sign in, then run Windows Update to completion:

**Start → Settings → Windows Update → Check for updates**

Expect one or two reboot cycles. The prebuilt image can be months old.

Once fully updated, take a snapshot from the **host**:

```powershell
Checkpoint-VM -Name "Windows 11 dev environment" -SnapshotName "clean-baseline"
```

Or in Hyper-V Manager: right-click VM → Checkpoint → rename to `clean-baseline`.

This is your rollback target. Any time the VM gets into a bad state, revert here.

---

## 5. Install Rust toolchain

Inside the VM, go to **rustup.rs**, download and run `rustup-init.exe`. Accept the
defaults (stable, MSVC toolchain).

Open a **new** PowerShell (to pick up the updated PATH) and confirm:

```powershell
rustc --version    # rustc 1.94.1
cargo --version    # cargo 1.94.1
rustup show active-toolchain  # stable-x86_64-pc-windows-msvc (default)
```

Install cmake (required by `xmp_toolkit` build.rs):

```powershell
winget install Kitware.CMake --source winget
```

Open a new PowerShell and confirm:

```powershell
cmake --version    # cmake 4.3.1
```

Smoke test:

```powershell
cargo new hello-xmp && cd hello-xmp && cargo build
# Expected: Finished `dev` profile in ~1s
cd .. && Remove-Item -Recurse -Force hello-xmp
```

---

## 6. XMP Toolkit SDK

The `xmp_toolkit` crate (v1.12.1 verified, Adobe-maintained) bundles the C++
Adobe XMP Toolkit SDK and builds it automatically via `build.rs` using cmake.
**No separate SDK install is needed** — the cmake + MSVC toolchain from section 5
is sufficient.

Cargo dependency:

```toml
[dependencies]
xmp_toolkit = { version = "1", features = ["crt_static"] }
```

The `crt_static` feature statically links the MSVC CRT — important for a DLL loaded
into `prevhost.exe` to avoid runtime version conflicts with other handlers.

### Gotchas when reading sidecars

Two things will bite you in M2:

**1. `from_file()` does NOT read `.xmp` sidecars.** It reads embedded XMP out of
image file containers (JPEG, TIFF, etc.). For standalone `.xmp` sidecar files,
read the file as a string and parse it:

```rust
let xml = fs::read_to_string("photo.xmp")?;
let xmp: XmpMeta = xml.parse()?;   // via FromStr — use .parse(), not XmpMeta::from_str
```

**2. `dc:title` and `dc:description` are Lang Alt arrays, not strings.** Reading
them via `property()` returns an empty string (the array container value). Use
`localized_text()` with `"x-default"` as the locale:

```rust
let title = xmp.localized_text(xmp_ns::DC, "title", None, "x-default")
    .map(|(v, _)| v.value);
```

This applies to any Dublin Core alt-text field. Simple properties like `xmp:Rating`
use `property()` directly.

---

## 7. File sharing (host ↔ VM)

**Strategy:** Build inside the VM. Source lives on the host, exposed to the VM via
an SMB share over the NAT switch. The Rust `target/` directory stays VM-local
(set via `CARGO_TARGET_DIR`) — building into an SMB share is slow and causes file
lock conflicts with incremental compilation.

### Host side (one-time setup, elevated PowerShell)

```powershell
# 1. Create the SMB share
New-SmbShare -Name "xmp-reader" `
    -Path "C:\Users\paulp\claude-code\xmp-reader" `
    -FullAccess "lenovo-pdp\paulp"

# 2. Firewall rule scoped to the NAT subnet
New-NetFirewallRule -DisplayName "SMB from Hyper-V NAT" `
    -Direction Inbound `
    -Protocol TCP `
    -LocalPort 445 `
    -RemoteAddress 192.168.100.0/24 `
    -Profile Private `
    -Action Allow `
    -Enabled True

# 3. Set the NAT switch interface to the Private network profile
Set-NetConnectionProfile -InterfaceAlias "vEthernet (NATSwitch)" -NetworkCategory Private
```

### The silent-firewall gotcha (critical)

Windows Firewall has a per-profile `AllowInboundRules` setting. When it is `False`,
**every inbound allow rule on that profile is silently ignored** — rules still
appear as `Enabled: True` but never match traffic. On this host the Private profile
shipped with `AllowInboundRules: False`. Fix:

```powershell
# Check the setting
Get-NetFirewallProfile -Name Private |
    Select-Object Name, Enabled, AllowInboundRules

# If False, flip it
Set-NetFirewallProfile -Profile Private -AllowInboundRules True
```

Verify from the VM:

```powershell
Test-NetConnection 192.168.100.1 -Port 445    # should return TcpTestSucceeded : True
```

### VM side (enable network sharing, then mount)

Before mounting, ensure network sharing is turned on in the VM:

**Settings → Network & internet → Advanced network settings → Advanced sharing settings**
→ expand **Private networks** → enable **Network discovery** and **File and printer sharing**.

```powershell
# Mount \\host\xmp-reader as drive Z: persistently
New-PSDrive -Name "Z" -PSProvider FileSystem `
    -Root "\\192.168.100.1\xmp-reader" `
    -Credential lenovo-pdp\paulp -Persist
# Prompt appears for the host account password.

# Keep target/ off the share — VM-local disk only
[System.Environment]::SetEnvironmentVariable("CARGO_TARGET_DIR", "C:\cargo-target", "User")
```

Verify:

```powershell
cd Z:\
ls                 # should show CLAUDE.md, docs, sandbox, scripts
cargo build        # writes to C:\cargo-target\, not Z:\target\
```

---

## 8. Code-signing certificate (dev)

Self-signed cert for M1–M7. Production signing (Azure Trusted Signing) is deferred
to M8.

### Create the cert (VM, elevated PowerShell, run once)

```powershell
# 1. Create a self-signed code-signing certificate (valid 2 years)
$cert = New-SelfSignedCertificate `
    -Subject "CN=xmp-reader dev" `
    -Type CodeSigningCert `
    -CertStoreLocation Cert:\CurrentUser\My `
    -NotAfter (Get-Date).AddYears(2)

Write-Host "Thumbprint: $($cert.Thumbprint)"

# 2. Trust it: export the cert and import into Trusted Root (requires elevation)
Export-Certificate -Cert $cert -FilePath "$env:TEMP\xmp-reader-dev.cer" | Out-Null
Import-Certificate -FilePath "$env:TEMP\xmp-reader-dev.cer" `
    -CertStoreLocation Cert:\LocalMachine\Root
Remove-Item "$env:TEMP\xmp-reader-dev.cer"
```

Verify the cert exists:

```powershell
Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert |
    Where-Object { $_.Subject -match "xmp-reader dev" } |
    Format-Table Subject, Thumbprint, NotAfter
```

### Sign a DLL

Use the helper script (no elevation required):

```powershell
.\scripts\sign-dll.ps1                          # sign the default release DLL
.\scripts\sign-dll.ps1 -DllPath "C:\other.dll"  # sign a specific file
.\scripts\sign-dll.ps1 -Verify                  # verify the signature
```

The script finds the cert by subject name (`xmp-reader dev`) and calls `signtool`
with SHA-256 and a DigiCert timestamp. `signtool.exe` ships with Visual Studio /
Windows SDK (both pre-installed in the dev VM image).

### Typical dev loop (updated)

```powershell
.\scripts\reset-handler.ps1 -Release       # release DLL lock
cargo build --release
.\scripts\sign-dll.ps1                      # sign the new DLL
.\scripts\reset-handler.ps1 -Install        # register + restart Explorer
```

---

## 9. Explorer / prevhost reset helper

`scripts/reset-handler.ps1` — run from an **elevated PowerShell** inside the VM.

### Typical dev loop

```powershell
# 1. Before cargo build — release the DLL file lock held by prevhost.exe
.\scripts\reset-handler.ps1 -Release

# 2. Build
cargo build --release

# 3. After build — register the new DLL and reload Explorer
.\scripts\reset-handler.ps1 -Install
```

### Other modes

```powershell
# Quick Explorer refresh, no registration change
.\scripts\reset-handler.ps1

# Remove the handler entirely
.\scripts\reset-handler.ps1 -Uninstall

# Override the DLL path
.\scripts\reset-handler.ps1 -Install -DllPath "C:\path\to\xmp_reader.dll"
```

**Why kill prevhost.exe?**
Explorer loads property handlers into `prevhost.exe` (a surrogate process), not into
`explorer.exe` itself. Restarting Explorer alone does not unload the DLL — you must
kill `prevhost.exe` too. `-Release` kills it before the build so the linker can
overwrite the file; `-Install` kills it after so the fresh copy is loaded.

---

## 10. Windows Sandbox smoke-test

`sandbox/smoke-test.wsb` — a clean-install smoke test environment independent of
the persistent Hyper-V VM.

**Prerequisites:** build the project first (`cargo build --release`).

**To run:**

Double-click `sandbox\smoke-test.wsb` on the host. The sandbox will:

1. Map the release build output and sandbox scripts as read-only folders
2. Run `sandbox\install.ps1` automatically on login
3. Register the DLL, restart Explorer, and open `sandbox\test-fixtures\` in a window

Select a file in the Explorer window → **View → Details pane** to verify XMP fields
are visible.

Add JPEG + `.xmp` sidecar pairs to `sandbox\test-fixtures\` (done in M2).

---

## Checkpoints

| Name | When to create | Purpose |
|---|---|---|
| `clean-baseline` | After Windows Update, before any dev tools | Rollback to a clean OS |
| `ready-for-m1` | After completing all M0.5 tasks (#4–#11) | Rollback to a known-good toolchain state |

---

## Troubleshooting

**`cargo build` fails with "file in use" / access denied on the DLL**
Run `.\scripts\reset-handler.ps1 -Release` first — prevhost.exe holds a lock on the
loaded DLL.

**VM has no internet (169.254.x.x address)**
The Default Switch failed to assign an IP. Follow section 3 to set up the manual
NAT switch.

**regsvr32 fails**
Ensure you are running from an elevated PowerShell. On first run the DLL must export
`DllRegisterServer` — this is implemented in M1.

**Explorer Details pane shows no XMP fields**
Check that the handler is registered for the correct extension:
```powershell
Get-ItemProperty "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\PropertySystem\PropertyHandlers\.jpg"
```
The default value should be our handler's CLSID.
