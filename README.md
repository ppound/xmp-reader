# xmp-reader

A Windows shell extension that reads XMP sidecar files and surfaces their metadata
in Explorer's Details pane, column views, tooltips, and the Windows Search index —
without needing Bridge, Lightroom, or any other DAM.

Implemented as a native Rust COM DLL registered as a Windows **property handler**
and **context menu extension**. Read-only. No write-back.

> **Status:** v0.3.1 released.

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

1. Download the latest release zip from
   [Releases](https://github.com/ppound/xmp-reader/releases)
2. Extract the zip to a temporary folder
3. Open an **elevated PowerShell** (Run as Administrator)
4. Run the installer:
   *(Note: Since the scripts are not currently signed, we use `-ExecutionPolicy Bypass` to allow them to run)*

```powershell
cd "C:\path\to\extracted\folder"
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

This copies the DLL and property schema to `%ProgramFiles%\xmp-reader\`,
registers the handler for all supported formats, and restarts Explorer.

### Uninstall

```powershell
powershell -ExecutionPolicy Bypass -File .\uninstall.ps1
```

Or run from the install directory:

```powershell
powershell -ExecutionPolicy Bypass -File "$env:ProgramFiles\xmp-reader\uninstall.ps1"
```

This restores the original system handlers and removes the install directory.

### Development installation

For building from source see [docs/dev-environment.md](docs/dev-environment.md).

---

## Copy / Move with sidecar

Right-clicking any supported image file in Explorer shows two additional items
under **Show more options** (Windows 11's extended context menu):

- **Copy with sidecar** — copies the selected files to a chosen folder, including
  any matching XMP sidecars
- **Move with sidecar** — moves the selected files and their sidecars

Multi-select is supported: all selected images are copied/moved, with sidecars
included for those that have them. If no selected file has a sidecar, the menu
items are hidden. Operations are undo-able via Ctrl+Z.

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

## Searching with AQS

Once files are indexed by Windows Search, you can search by XMP sidecar metadata
directly in Explorer's search box or the Start menu using
[Advanced Query Syntax (AQS)](https://learn.microsoft.com/en-us/windows/win32/search/advanced-query-syntax).

> **Note:** Files must be in an indexed location. If searches return no results after
> installing, go to **Settings → Privacy & Security → Searching Windows → Advanced
> indexing options → Advanced → Rebuild** and wait for the index to complete.

### Standard property queries

| What you want | AQS query |
|---|---|
| Files with a specific title | `title:"golden hour"` |
| Files with a keyword | `keywords:vacation` |
| Files by author | `author:"Alice"` |
| Files with a description containing a word | `comment:sunset` |
| 4-star rated files | `System.SimpleRating:=4` |
| 3 stars or higher | `System.SimpleRating:>=3` |
| Unrated files | `System.SimpleRating:=0` |
| Files taken in a date range | `datetaken:2024` |

**Rating note:** Windows Search's built-in `rating:` keyword resolves to the
`System.Rating` property which uses a 0–99 scale (1 star = 1, 2 = 25, 3 = 50,
4 = 75, 5 = 99). For intuitive 1–5 star comparisons use `System.SimpleRating:`
instead, which xmp-reader also exposes.

### Custom property queries

Custom XMP fields use their full canonical name:

| What you want | AQS query |
|---|---|
| Files with a specific headline | `XmpSidecar.Headline:"product launch"` |
| Files shot at a location | `XmpSidecar.Location:Paris` |
| Files featuring a person | `XmpSidecar.PersonInImage:"Bob"` |
| Files tagged with a place | `XmpSidecar.Place:"Central Park"` |
| Files with a specific cloud upload tag | `XmpSidecar.CloudUploads:instagram` |

### Combining queries

AQS supports `AND`, `OR`, `NOT` and parentheses:

```
keywords:vacation AND System.SimpleRating:>=4
author:"Alice" AND datetaken:2024
XmpSidecar.PersonInImage:"Bob" AND keywords:wedding
```

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
| `src/context_menu.rs` | Context menu extension: `IShellExtInit`, `IContextMenu` — "Copy/Move with sidecar" |
| `src/registry.rs` | `DllRegisterServer` / `DllUnregisterServer`: saves and restores per-extension old handlers, registers `.propdesc` schema and context menu |

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
| M8 — Packaging + release | Done |
| M9 — Sidecar copy/move context menu | Done |
| M10 — AQS search property format fixes | Done |

---

## License

MIT — see [LICENSE](LICENSE)
