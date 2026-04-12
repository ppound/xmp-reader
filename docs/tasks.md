# Task state — handoff snapshot

> Updated 2026-04-12.
>
> **Current milestone: M3 — XMP-to-PKEY mapping.**
> **Next action: wire sidecar parsing into the property handler so XMP fields appear in Explorer.**

## M0.5 tasks

### #1 — Verify xmp-toolkit-rs is viable ✅ DONE
`xmp_toolkit` v1.12.1 (Adobe-maintained) works. Windows MSVC supported.
All namespaces we need (`dc`, `xmp`, `photoshop`, `iptc_core`, `exif`) present.
Dependency line:
```toml
xmp_toolkit = { version = "1", features = ["crt_static"] }
```
**Status:** completed.

### #2 — Verify Windows host prerequisites for Hyper-V ✅ DONE
Windows 11 Pro, Hyper-V feature installed.
**Status:** completed.

### #3 — Set up Windows 11 Dev VM via Hyper-V Quick Create ✅ DONE
VM created via Hyper-V Manager → Quick Create → "Windows 11 development environment"
(note: Microsoft does NOT offer a standalone VHDX download — this is the right path).
NAT switch (`NATSwitch`) set up manually because Default Switch doesn't work reliably
on Wi-Fi. VM at 192.168.100.10, host at 192.168.100.1. Windows Update complete.
`clean-baseline` checkpoint taken.
**Status:** completed.

### #4 — Rust toolchain in VM ✅ DONE
Rust 1.94.1 + `stable-x86_64-pc-windows-msvc` + cmake 4.3.1 (via winget).
Smoke test `cargo build` of an empty project succeeded.
**Status:** completed.

### #5 — XMP Toolkit SDK in VM ✅ DONE
`xmp_toolkit` crate builds the bundled C++ SDK automatically via cmake. No separate
SDK install needed. End-to-end read test validated: parse raw XMP XML via
`xml.parse::<XmpMeta>()` and read `dc:title` / `dc:description` via `localized_text()`
with `"x-default"`. (Note: `XmpMeta::from_file()` does NOT read `.xmp` sidecars —
that path is for embedded XMP in image containers.)
**Status:** completed.

### #6 — File sharing host ↔ VM ✅ DONE
**Strategy decided:** Build-in-VM. Source lives on host, shared into VM via SMB over
the NATSwitch. `target/` directory is VM-local (not on the share) via `CARGO_TARGET_DIR`
to avoid slow builds and file lock issues.

**Host side — DONE:**
- SMB share `xmp-reader` → `C:\Users\paulp\claude-code\xmp-reader`
  - Created with: `New-SmbShare -Name "xmp-reader" -Path "C:\Users\paulp\claude-code\xmp-reader" -FullAccess "lenovo-pdp\paulp"`
- NATSwitch profile changed from Public to Private
- Firewall rule "SMB from Hyper-V NAT" (Private profile, TCP 445, RemoteAddress 192.168.100.0/24)
- **Critical fix:** `Set-NetFirewallProfile -Profile Private -AllowInboundRules True` was required —
  the profile shipped with `AllowInboundRules: False`, which silently ignored all
  inbound allow rules. See `memory/reference_win_firewall_allow_inbound.md`.
- VM→host `Test-NetConnection 192.168.100.1 -Port 445` returns `True`.

**VM side — TODO:**
1. Mount the share:
   ```powershell
   New-PSDrive -Name "Z" -PSProvider FileSystem `
       -Root "\\192.168.100.1\xmp-reader" `
       -Credential lenovo-pdp\paulp -Persist
   ```
   (will prompt for host account password interactively)
2. Verify: `cd Z:\; ls` should show CLAUDE.md, docs, sandbox, scripts.
3. Set VM-local target dir as a user env var:
   ```powershell
   [System.Environment]::SetEnvironmentVariable("CARGO_TARGET_DIR", "C:\cargo-target", "User")
   ```
4. Round-trip test: edit a file on host, see change in VM. `cargo build` inside
   `Z:\` writes to `C:\cargo-target\`, not the share.

**Additional requirement discovered:** Network sharing (Network Discovery + File and Printer Sharing) must be enabled on the VM for SMB to work.

**Status:** completed 2026-04-12.

### #7 — Windows Sandbox .wsb config ✅ DONE
`sandbox/smoke-test.wsb`, `sandbox/install.ps1`, `sandbox/test-fixtures/`.
Ready to use once a DLL exists to smoke-test.
**Status:** completed.

### #8 — Self-signed code-signing cert in VM ✅ DONE
Cert `CN=xmp-reader dev` created in `Cert:\CurrentUser\My` and root installed in
`Cert:\LocalMachine\Root`. Thumbprint: `676AD06A054968DFD61551576E724CD394BDD1E5`.
`scripts/sign-dll.ps1` and `docs/dev-environment.md` §8 already written.
**Status:** completed 2026-04-12.

### #9 — Explorer/prevhost reset helper script ✅ DONE
`scripts/reset-handler.ps1` — handles `-Release` (kill prevhost before build),
`-Install` (unregister/register/reset), `-Uninstall`, and plain reset.
**Status:** completed.

### #10 — Document dev environment in docs/dev-environment.md ✅ DONE
Drafted with all sections filled in except #7 (file sharing — waiting on Task #6 completion)
and #8 (cert — waiting on Task #8).
**Status:** completed.

### #11 — Smoke test M0.5 end-to-end and checkpoint as `ready-for-m1` ✅ DONE
Built stub cdylib, signed with dev cert, installed/uninstalled via reset-handler.ps1,
Explorer restarted cleanly. `ready-for-m1` Hyper-V checkpoint taken.
**Status:** completed 2026-04-12.

## M1 tasks

### #12 — COM property handler skeleton ✅ DONE
Implemented full COM DLL with IClassFactory, IInitializeWithFile, IPropertyStore,
IPropertyStoreCapabilities. Returns hardcoded `System.Comment` = "XMP sidecar handler active".
DllRegisterServer writes CLSID + InprocServer32 + PropertyHandler\.jpg entries;
DllUnregisterServer restores old handler. Verified in Explorer Details pane on real JPEG.
**Status:** completed 2026-04-12.

## M2 tasks

### #13 — Sidecar discovery + XMP parse ✅ DONE
`src/sidecar.rs`: `find_sidecar(path)` checks `<stem>.xmp` then `<name>.xmp`,
`parse_sidecar(path)` / `parse_xmp(xml)` extracts rating, title, description,
keywords, creators, date_taken, headline, location into `XmpFields` struct.
6 unit tests (parse_full, parse_minimal, find_stem, find_ext, find_prefers_stem, find_none).
All pass. `xmp_toolkit` v1.12.1 with `crt_static` feature.
**Status:** completed 2026-04-12.

## Resume here

Next: M3 — XMP-to-PKEY mapping. Wire `sidecar.rs` into `handler.rs` so parsed XMP
fields reach Explorer Details pane on real JPEG+sidecar pairs.

## Context notes

- Host: `lenovo-pdp`, Windows 11 Pro, Build 10.0.26200, Lenovo, Wi-Fi.
- Host user: `paulp`.
- VM: "Windows 11 dev environment" on Hyper-V, default user `User`, 192.168.100.10.
- Networking: NATSwitch (manual `New-NetNat`), Default Switch does NOT work on Wi-Fi.
- `xmp_toolkit` API gotchas captured in `memory/reference_xmp_toolkit_api.md`.
- Firewall `AllowInboundRules=False` gotcha captured in `memory/reference_win_firewall_allow_inbound.md`.
- Debugging tone feedback captured in `memory/feedback_debugging_tone.md`.
