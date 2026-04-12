# xmp-reader

A Windows shell extension that reads XMP sidecar files and surfaces their metadata
in Explorer's Details pane, column views, tooltips, and the Windows Search index —
without needing Bridge, Lightroom, or any other DAM.

Implemented as a native Rust COM DLL registered as a Windows **property handler**.
Read-only. No write-back.

> **Status:** M7 complete. M8 (installer + release) is next.

---

## Why

Windows Explorer does not read XMP sidecar files. Every tool that does (Bridge,
Lightroom, digiKam, XnView) is a standalone application with no shell integration.
The last shell-integration option (FastPictureViewer Codec Pack) is frozen on
Windows 7/8-era code.

xmp-reader fills that gap with a modern, maintained property handler.

---

## How it works

When Explorer needs metadata for a file (e.g. `photo.jpg`), it activates the
registered property handler for that extension. xmp-reader:

1. Receives the file path via `IInitializeWithFile`
2. Delegates to the previous system handler (via `CoCreateInstance`) to load
   embedded EXIF/XMP as the base layer
3. Looks for a matching sidecar (`photo.xmp` or `photo.jpg.xmp`)
4. Parses the sidecar with the Adobe XMP Toolkit SDK (via the `xmp_toolkit` crate)
5. Merges sidecar fields on top — sidecar wins on conflict, embedded-only fields
   are preserved
6. Exposes the merged result via `IPropertyStore`

Because registering our handler for an extension evicts the system's built-in
handler, we chain to the old handler for embedded metadata — users don't lose
anything by installing us.

---

## Requirements

- Windows 11 (x64)
- No runtime dependencies (statically linked CRT)

---

## Installation

> Installer not yet available — packaging is planned for M8.

For development installation see [docs/dev-environment.md](docs/dev-environment.md).

---

## Supported formats

| Extension | Status |
|---|---|
| `.jpg` / `.jpeg` | Supported |
| `.cr2` `.nef` `.arw` `.dng` `.tif` `.tiff` | Supported |
| `.raf` | Supported (Fujifilm RAW) |

---

## XMP fields exposed

### Standard Windows properties

| XMP field | Windows property |
|---|---|
| `dc:title` | `System.Title` |
| `dc:description` | `System.Comment` |
| `dc:subject` | `System.Keywords` |
| `dc:creator` | `System.Author` |
| `xmp:Rating` | `System.Rating` / `System.SimpleRating` |
| `xmp:CreateDate` / `photoshop:DateCreated` | `System.Photo.DateTaken` |
| `Iptc4xmpCore:Location` | `System.Photo.LocationName` |
| `exif:GPS*` | `System.GPS.*` |

### Custom properties (via `.propdesc` schema)

These appear as addable columns in Explorer's "Choose columns" dialog under the
`XmpSidecar` group.

| XMP field | Custom property |
|---|---|
| `photoshop:Headline` | `XmpSidecar.Headline` |
| `Iptc4xmpCore:Location` | `XmpSidecar.Location` |
| `Iptc4xmpExt:PersonInImage` | `XmpSidecar.PersonInImage` |
| `photostat:place` | `XmpSidecar.Place` |
| `photostat:cloudUploads` | `XmpSidecar.CloudUploads` |

---

## Development

See [docs/dev-environment.md](docs/dev-environment.md) for the full setup guide
(Hyper-V VM, Rust toolchain, code-signing cert, dev loop).

**Quick dev loop (elevated PowerShell in the VM):**

```powershell
.\scripts\reset-handler.ps1 -Release    # release DLL lock held by prevhost.exe
cargo build --release
.\scripts\sign-dll.ps1                  # sign with dev cert
.\scripts\reset-handler.ps1 -Install   # register + restart Explorer
```

**Architecture:**

| File | Role |
|---|---|
| `src/lib.rs` | DLL entry points (`DllGetClassObject`, `DllRegisterServer`, etc.) |
| `src/handler.rs` | COM object: `IClassFactory`, `IInitializeWithFile`, `IPropertyStore`, `IPropertyStoreCapabilities` |
| `src/sidecar.rs` | Sidecar discovery (`find_sidecar`) and XMP parsing (`parse_xmp`) |
| `src/embedded.rs` | Reads embedded metadata by delegating to the previous system handler |
| `src/pkeys.rs` | `PROPERTYKEY` constants and rating scale conversion |
| `src/registry.rs` | `DllRegisterServer` / `DllUnregisterServer`: saves and restores per-extension old handlers, registers `.propdesc` schema |

**Tests:** 20 unit tests across all modules. Manual Explorer checklist in
[docs/test-checklist.md](docs/test-checklist.md).

---

## Roadmap

| Milestone | Status |
|---|---|
| M0.5 — Dev VM + toolchain | Done |
| M1 — COM skeleton visible in Explorer | Done |
| M2 — Sidecar discovery + XMP parse | Done |
| M3 — XMP → PKEY mapping | Done |
| M4 — Embedded metadata fallback + merge | Done |
| M5 — Multi-format + clean uninstall | Done |
| M6 — Test suite | Done |
| M7 — Custom `.propdesc` schema | Done |
| M8 — Packaging + release | Next |

---

## License

MIT — see [LICENSE](LICENSE)
