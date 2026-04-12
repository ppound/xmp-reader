# xmp-reader

A Windows shell extension that reads XMP sidecar files and surfaces their metadata
in Explorer's Details pane, column views, tooltips, and the Windows Search index —
without needing Bridge, Lightroom, or any other DAM.

Implemented as a native Rust COM DLL registered as a Windows **property handler**.
Read-only. No write-back.

> **Status:** M1 complete — COM skeleton verified end-to-end in Explorer.
> M2 (XMP sidecar parsing) in progress.

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
2. Looks for a matching sidecar (`photo.xmp` or `photo.jpg.xmp`)
3. Parses the sidecar with the Adobe XMP Toolkit SDK (via the `xmp_toolkit` crate)
4. Also reads embedded EXIF/XMP from the image via WIC (fallback / merge base)
5. Exposes the merged result via `IPropertyStore`

Sidecar fields take precedence over embedded fields on conflict.

Because registering our handler for `.jpg` evicts the system's built-in handler,
we also surface embedded metadata ourselves — users don't lose anything by
installing us.

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
| `.jpg` / `.jpeg` | M1–M4 |
| `.cr2` `.nef` `.arw` `.dng` `.tif` `.tiff` | Planned (M5) |

---

## XMP fields exposed

| XMP field | Windows property |
|---|---|
| `dc:title` | `System.Title` |
| `dc:description` | `System.Comment` |
| `dc:subject` | `System.Keywords` |
| `dc:creator` | `System.Author` |
| `xmp:Rating` | `System.Rating` / `System.SimpleRating` |
| `xmp:CreateDate` / `photoshop:DateCreated` | `System.Photo.DateTaken` |
| `photoshop:Headline` | `System.Photo.Event` |
| `Iptc4xmpCore:Location` | `System.Photo.LocationName` |
| `exif:GPS*` | `System.GPS.*` |

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

- `src/lib.rs` — DLL entry points (`DllGetClassObject`, `DllRegisterServer`, etc.)
- `src/handler.rs` — `IClassFactory` + `IPropertyStore` / `IInitializeWithFile` / `IPropertyStoreCapabilities` impls
- `src/registry.rs` — `DllRegisterServer` / `DllUnregisterServer` logic (saves and restores the previous `.jpg` handler)

---

## Roadmap

| Milestone | Status |
|---|---|
| M0.5 — Dev VM + toolchain | Done |
| M1 — COM skeleton visible in Explorer | Done |
| M2 — Sidecar discovery + XMP parse | In progress |
| M3 — XMP → PKEY mapping | Planned |
| M4 — Embedded metadata fallback + merge | Planned |
| M5 — Multi-format + clean uninstall | Planned |
| M6 — Test suite | Planned |
| M7 — Custom `.propdesc` schema (optional) | Planned |
| M8 — Packaging + release | Planned |

---

## License

MIT — see [LICENSE](LICENSE)
