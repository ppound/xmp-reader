# Task state — handoff snapshot

> Updated 2026-04-15.
>
> **All milestones M0–M10 complete.**

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
- SMB share `xmp-reader` → `C:\path\to\xmp-reader`
  - Created with: `New-SmbShare -Name "xmp-reader" -Path "C:\path\to\xmp-reader" -FullAccess "[COMPUTER-NAME]\[USERNAME]"`
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
       -Credential [COMPUTER-NAME]\[USERNAME] -Persist
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

## M3 tasks

### #14 — XMP-to-PKEY mapping ✅ DONE
`src/pkeys.rs`: PROPERTYKEY constants + rating conversion (1-5 stars -> 0-99 scale).
`src/handler.rs`: rewritten to call `find_sidecar` + `parse_sidecar` in Initialize,
build dynamic property list, serve real values via GetCount/GetAt/GetValue.
Mapped: Title, Comment, Keywords, Author, Rating, DateTaken, Photo.Event.
Verified in Explorer Details pane on Desktop JPEG+sidecar pair: all fields visible
except Photo.Event (not displayed by default in Details pane - acceptable).
**Status:** completed 2026-04-12.

## M4 tasks

### #15 — Embedded metadata fallback + merge ✅ DONE
`src/embedded.rs`: delegates to old system property handler via `CoCreateInstance`,
reads all properties via `IPropertyStore`. `src/handler.rs` updated: `Initialize`
loads embedded properties first as base, then overlays sidecar properties on top
(sidecar wins on PKEY conflict, embedded-only properties preserved).
`src/registry.rs`: added `get_old_handler_clsid()` and `parse_guid()`.
Verified: sidecar-less JPEG shows embedded EXIF/XMP, JPEG with sidecar shows
merged result (sidecar overrides + embedded preserved).
**Status:** completed 2026-04-12.

## M5 tasks

### #16 — Multi-format registration ✅ DONE
`src/registry.rs`: replaced single `JPG_HANDLER_PATH` with `EXTENSIONS` array
(`.jpg`, `.cr2`, `.nef`, `.arw`, `.dng`, `.tif`, `.tiff`). `register()` and
`unregister()` loop over all extensions, saving/restoring per-extension old handler.
`src/embedded.rs` and `src/handler.rs`: `load_embedded` now takes the file extension
so it delegates to the correct old handler per format.
Bug fix: `reset-handler.ps1` `Invoke-Regsvr32` parameter renamed from `$args`
(reserved PS automatic variable) to `$RegArgs` — `/u` flag was silently dropped.
Verified: all 7 extensions registered on install, all restored on uninstall.
**Status:** completed 2026-04-12.

## M6 tasks

### #17 — Test suite ✅ DONE
20 automated unit tests across 4 modules:
- `sidecar.rs` (6): XMP parsing (full + minimal) and sidecar discovery
- `registry.rs` (7): GUID parse/format/roundtrip, invalid input, extension list
- `pkeys.rs` (4): rating conversion (valid + out-of-range), PKEY fmtid grouping
- `handler.rs` (3): build_properties (full + empty), merge logic (override + append)
Manual Explorer checklist: `docs/test-checklist.md` (8 scenarios).
**Status:** completed 2026-04-12.

## M7 tasks

### #18 — Custom .propdesc schema for non-standard XMP fields ✅ DONE
Created `xmp-sidecar.propdesc` with 5 custom properties under format ID
`{B2A7E62A-1D9C-4F5E-8A3B-7C6D5E4F3A2B}`:
- `XmpSidecar.Headline` (pid 2) - photoshop:Headline
- `XmpSidecar.Location` (pid 3) - Iptc4xmpCore:Location
- `XmpSidecar.PersonInImage` (pid 4) - Iptc4xmpExt:PersonInImage
- `XmpSidecar.Place` (pid 5) - photostat:place
- `XmpSidecar.CloudUploads` (pid 6) - photostat:cloudUploads

`src/sidecar.rs`: added parsing for PersonInImage, photostat:place, photostat:cloudUploads.
`src/pkeys.rs`: added 5 custom PKEY constants.
`src/handler.rs`: maps all new fields to custom PKEYs.
`src/registry.rs`: calls PSRegisterPropertySchema/PSUnregisterPropertySchema.
Schema registered successfully. Custom columns available in Explorer "Choose columns".
**Status:** completed 2026-04-12.

## M8 tasks

### #19 — Installer + uninstaller scripts ✅ DONE
`scripts/install.ps1`: copies DLL + .propdesc to `%ProgramFiles%\xmp-reader\`,
registers via regsvr32, restarts Explorer. Also copies itself and uninstall.ps1
to install dir for later use.
`scripts/uninstall.ps1`: unregisters, removes install dir, restarts Explorer.
Both require Administrator elevation.
**Status:** completed 2026-04-12.

### #20 — GitHub Actions release workflow ✅ DONE
`.github/workflows/release.yml`: triggered on `v*` tag push. Builds on
`windows-latest`, runs tests, packages DLL + propdesc + scripts + README + LICENSE
into a zip, creates GitHub Release with auto-generated release notes.
**Status:** completed 2026-04-12.

### #21 — README update ✅ DONE
Updated installation section with download + install/uninstall instructions.
Status bumped to v0.1.0. Roadmap table all marked Done.
**Status:** completed 2026-04-12.

## M9 tasks

### #22 — Context menu shell extension ✅ DONE
`src/context_menu.rs`: `IShellExtInit` + `IContextMenu` implementation.
`IShellExtInit::Initialize` receives selected file, calls `find_sidecar()`.
`QueryContextMenu` adds "Copy with sidecar" / "Move with sidecar" items
only when a sidecar exists. `InvokeCommand` opens `IFileOpenDialog` folder
picker, then uses `IFileOperation` to copy/move both image and sidecar.
Supports undo via `FOF_ALLOWUNDO`.

New CLSID `{A1C2D3E4-5F60-4718-B9CA-0D1E2F3A4B5C}` registered under
`HKCR\SystemFileAssociations\.<ext>\shellex\ContextMenuHandlers\XmpSidecar`
for all supported extensions. `DllGetClassObject` dispatches on CLSID.
`src/registry.rs` updated for register/unregister.

Debug + release builds pass. All 20 existing tests pass.
**Status:** completed 2026-04-13.

### #23 — Manual Explorer testing of context menu ✅ DONE
Verified in VM on test-images:
- Right-click DSCF1004.RAF (has sidecar): "Copy with sidecar" and
  "Move with sidecar" appear under "Show more options" (Win11 modern
  context menu puts classic IContextMenu items there by default).
- Right-click DSCF1001.RAF (no sidecar): items do not appear.
- Copy and Move both work correctly, transferring image + sidecar.
**Status:** completed 2026-04-13.

## M10 tasks

### #24 — AQS search property format fixes ✅ DONE

**Problem diagnosed 2026-04-15.**

Explorer columns show all XMP sidecar data correctly. Windows Search AQS queries mostly
do not work because we return multi-valued properties as a single joined VT_BSTR instead
of VT_VECTOR|VT_LPWSTR, and we are missing System.SimpleRating.

**Confirmed working before this fix:**
- `title:"golden hour"` — System.Title is VT_BSTR (single-value), indexes correctly. ✅

**Broken before this fix:**
- `keywords:vacation` — Keywords returned as `"vacation; mountains"` VT_BSTR; indexer
  stores as one blob, individual element matching fails.
- `System.Author:"Alice"` — same issue.
- `XmpSidecar.PersonInImage:"Alice"` — same issue.
- `XmpSidecar.CloudUploads:"someservice"` — same issue.
- `rating:<5` — AQS `rating:` maps to System.Rating (0–99 scale). A 4-star image is
  stored as 75; `75 < 5` is false. `rating:>3` works only by coincidence (75 > 3).
  `rating:<99` also works (75 < 99). Fix: expose System.SimpleRating (1–5) so users can
  write `System.SimpleRating:<5`.

**Changes required:**

#### 1. `src/pkeys.rs` — add PKEY_SIMPLE_RATING

```rust
// System.SimpleRating  {A09F084E-AD41-489F-8076-AA5BE3082BCA} pid 100
pub const PKEY_SIMPLE_RATING: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID {
        data1: 0xA09F084E,
        data2: 0xAD41,
        data3: 0x489F,
        data4: [0x80, 0x76, 0xAA, 0x5B, 0xE3, 0x08, 0x2B, 0xCA],
    },
    pid: 100,
};
```

#### 2. `src/handler.rs` — two changes

**A. Add a string-vector helper** (above `build_properties`):

```rust
use windows::Win32::UI::Shell::PropertiesSystem::InitPropVariantFromStringAsVector;

/// Build a VT_VECTOR|VT_LPWSTR PROPVARIANT from a slice of strings.
/// Uses InitPropVariantFromStringAsVector which splits on ";".
/// Caller must ensure strings do not contain ";".
fn string_vec_propvar(strings: &[String]) -> PROPVARIANT {
    debug_assert!(!strings.is_empty());
    let joined: Vec<u16> = strings
        .join(";")
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut pv = PROPVARIANT::default();
    unsafe {
        let _ = InitPropVariantFromStringAsVector(PCWSTR(joined.as_ptr()), &mut pv);
    }
    pv
}
```

**B. Update `build_properties`** — replace the four joined-string entries and add
SimpleRating:

```rust
// Rating — keep System.Rating (0-99), also emit System.SimpleRating (1-5)
if let Some(stars) = fields.rating {
    let win_rating = xmp_rating_to_windows(stars);
    props.push(PropEntry { key: PKEY_RATING,        value: PROPVARIANT::from(win_rating) });
    props.push(PropEntry { key: PKEY_SIMPLE_RATING,  value: PROPVARIANT::from(stars as u32) });
}

// Keywords — VT_VECTOR|VT_LPWSTR (was joined VT_BSTR)
if !fields.keywords.is_empty() {
    props.push(PropEntry { key: PKEY_KEYWORDS, value: string_vec_propvar(&fields.keywords) });
}

// Authors — VT_VECTOR|VT_LPWSTR (was joined VT_BSTR)
if !fields.creators.is_empty() {
    props.push(PropEntry { key: PKEY_AUTHOR, value: string_vec_propvar(&fields.creators) });
}

// PersonInImage — VT_VECTOR|VT_LPWSTR (was joined VT_BSTR)
if !fields.person_in_image.is_empty() {
    props.push(PropEntry {
        key: PKEY_XMP_PERSON_IN_IMAGE,
        value: string_vec_propvar(&fields.person_in_image),
    });
}

// CloudUploads — VT_VECTOR|VT_LPWSTR (was joined VT_BSTR)
if !fields.photostat_cloud_uploads.is_empty() {
    props.push(PropEntry {
        key: PKEY_XMP_CLOUD_UPLOADS,
        value: string_vec_propvar(&fields.photostat_cloud_uploads),
    });
}
```

Also update the use line at the top of handler.rs to include `PKEY_SIMPLE_RATING` from
`crate::pkeys`.

The `build_properties_full` unit test currently asserts `props.len() == 8`. After adding
SimpleRating it will be 9. Update that assert.

#### 3. `xmp-sidecar.propdesc` — fix multipleValues for PersonInImage and CloudUploads

Change:
```xml
<typeInfo type="String" multipleValues="false" isViewable="true" />
```
to:
```xml
<typeInfo type="String" multipleValues="true" isViewable="true" />
```
for `XmpSidecar.PersonInImage` (pid 4) and `XmpSidecar.CloudUploads` (pid 6).

**After the fix, AQS that should work:**

| Query | Explanation |
|---|---|
| `keywords:vacation` | individual keyword match |
| `author:"Alice"` | individual author match |
| `rating:>50` | System.Rating 0-99 scale (3+ stars = >50) |
| `System.SimpleRating:>3` | 4 or 5 star images |
| `System.SimpleRating:<5` | 1–4 star images |
| `XmpSidecar.PersonInImage:"Alice"` | person in image match |
| `XmpSidecar.Headline:"sunrise"` | headline match (already worked) |
| `XmpSidecar.Location:"Paris"` | location match (already worked) |

**Verification steps after implementing:**
1. `cargo test` — all tests pass (update `build_properties_full` count to 9).
2. Build release DLL, sign, reinstall in VM.
3. Trigger reindex: Settings → Search → Windows Search → Advanced → Rebuild index.
   Wait for completion.
4. Test each AQS query above against known test images.

**Status:** completed 2026-04-15. Verified in VM after full index rebuild:
- Multi-valued AQS queries (keywords, author, PersonInImage, CloudUploads) now
  match individual elements.
- `System.SimpleRating:<5` / `:>3` / `:=4` work as expected.
- AQS `rating:` remains tied to System.Rating (0–99) — a Windows AQS keyword
  resolution behavior we can't override from a property handler, so users
  needing star-based numeric comparisons should use `System.SimpleRating`.

Code summary:
- `src/pkeys.rs`: added `PKEY_SIMPLE_RATING`.
- `src/handler.rs`: added `string_vec_propvar` helper using
  `InitPropVariantFromStringAsVector` (imported from
  `windows::Win32::System::Com::StructuredStorage`; the function returns
  `Result<PROPVARIANT>` in windows 0.58, not the out-parameter form shown in the
  plan). Keywords / Author / PersonInImage / CloudUploads now emitted as
  VT_VECTOR|VT_LPWSTR. Rating path also emits `System.SimpleRating` (1–5).
- `xmp-sidecar.propdesc`: `multipleValues="true"` for PersonInImage (pid 4) and
  CloudUploads (pid 6).
- `build_properties_full` test updated 8 → 9. `cargo test --lib` 20/20 pass.

## Resume here

All milestones M0–M10 complete. No active work queued.

All milestones M0–M9 complete.

## Context notes

- Host: `[COMPUTER-NAME]`, Windows 11 Pro, Build 10.0.26200, Lenovo, Wi-Fi.
- Host user: `[USERNAME]`.
- VM: "Windows 11 dev environment" on Hyper-V, default user `User`, 192.168.100.10.
- Networking: NATSwitch (manual `New-NetNat`), Default Switch does NOT work on Wi-Fi.
- `xmp_toolkit` API gotchas captured in `memory/reference_xmp_toolkit_api.md`.
- Firewall `AllowInboundRules=False` gotcha captured in `memory/reference_win_firewall_allow_inbound.md`.
- Debugging tone feedback captured in `memory/feedback_debugging_tone.md`.
