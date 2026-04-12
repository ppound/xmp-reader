# xmp-reader — Project Plan

> Handoff doc. Written 2026-04-11 at end of a WSL-hosted Claude Code session, before migrating to a Windows-native Claude Code session. A fresh session should read this + `docs/tasks.md` and resume at the indicated milestone.

## Goal

Ship a Windows shell extension — an in-process COM DLL registered as a **property handler** for selected image extensions — that reads a matching `.xmp` sidecar file and exposes its fields via `IPropertyStore`, so XMP sidecar metadata appears in Explorer's Details pane, tooltips, column views, and the Windows Search index.

Read-only for v1. No write-back.

## Why this project exists

Windows Explorer and the Windows 11 Photos app do not read XMP sidecar files. Existing tools that do are standalone DAMs (Bridge, Lightroom, digiKam, XnView, etc.) with no shell integration. The last shell-integration option (FastPictureViewer Codec Pack) is frozen on Windows 7/8-era code. There is a real gap for a modern, maintained XMP-sidecar-aware property handler.

## Scope decisions (locked in 2026-04-11)

| Decision | Value |
|---|---|
| Implementation language | **Rust** (`windows-rs` crate for COM) |
| XMP parser | **Adobe XMP Toolkit C++ SDK**, ideally via the `xmp-toolkit-rs` binding; fall back to a hand-rolled C-ABI wrapper if the binding is dead |
| Target OS | **Windows 11 only** |
| v1 extension set | **`.jpg` only** for M1–M4; add `.cr2 .nef .arw .dng .tif .tiff` at M5 |
| Sidecar precedence on conflict | **Sidecar wins** (matches ExifTool default) |
| Sidecar matching | For `image.ext`, look for both `image.xmp` and `image.ext.xmp` (ExifTool convention) |
| Code signing for dev | **Self-signed** during M1–M7 |
| Code signing for release | **Azure Trusted Signing** (decision deferred to M8, identity verification can start in parallel) |
| Read vs write | **Read-only** for v1; writes are a separate follow-up project |
| Test environment | **Hyper-V Windows 11 Dev VM** (primary) + **Windows Sandbox** (smoke tests) |

## Explicitly out of scope for v1

- Writing XMP sidecars back.
- `IThumbnailProvider` / `IPreviewHandler` — leave the system ones in place.
- Windows Photos / new Image Viewer plugin work (no plugin surface exists; we only reach it indirectly via the shell property system).
- Video sidecars.
- Windows 10 support.

## Architecture notes

### Property handler replaces, doesn't chain
When we register our CLSID for `.jpg`, we **evict** the system's built-in property handler. Therefore we must also surface *embedded* EXIF/XMP ourselves (via WIC → `IWICMetadataQueryReader`) and merge it with sidecar data. Otherwise users would lose metadata they had before installing us.

Merge policy: **embedded is the base layer, sidecar overrides on field conflicts.**

### Interfaces we implement
- `IInitializeWithFile` — Explorer hands us the path.
- `IPropertyStore` — `GetCount`, `GetAt`, `GetValue` for read. `SetValue`/`Commit` are stubs that return `STG_E_ACCESSDENIED` for v1.
- `IPropertyStoreCapabilities` — report all properties as read-only.

### Hosting process
Explorer loads property handlers in the `prevhost.exe` surrogate process, not in `explorer.exe` itself. This means a crash is isolated, but also that restarting Explorer alone doesn't reliably reload the DLL — our reset helper must also kill `prevhost.exe`.

### Why native code (not C#)
Managed shell extensions are officially discouraged by Microsoft because the .NET runtime version loaded into `prevhost.exe` is shared across all managed handlers, causing version conflicts. Rust produces a native DLL with no runtime dependency and is a safe choice.

## Milestones

| ID | Title | Exit criterion |
|---|---|---|
| M0 | Scope decisions | This doc exists; scope table is filled |
| **M0.5** | **Windows dev VM + toolchain** | **`ready-for-m1` Hyper-V checkpoint exists; no-op DLL builds, signs, installs cleanly from doc** |
| M1 | Minimal end-to-end skeleton | COM DLL returns one hardcoded `System.Comment` field; visible in Explorer Details pane on a real JPEG in the VM |
| M2 | Sidecar discovery + XMP parse | `parse_sidecar(path)` returns a stable field map over fixture corpus (unit tests green on host) |
| M3 | XMP → PKEY mapping | Sidecar values reach Explorer Details pane on real JPEG+sidecar pair |
| M4 | Embedded metadata fallback + merge | Installing our handler does not regress Details pane on sidecar-less JPEGs |
| M5 | Multi-format + per-extension opt-in + clean uninstall | Can register/unregister across extension set without clobbering others; uninstall restores original handler mapping |
| M6 | Test suite | Automated COM-level tests + manual Explorer checklist |
| M7 | (Optional) Custom `.propdesc` schema | Non-PKEY XMP fields addable as Explorer columns |
| M8 | Packaging, README, release | Installer built via GitHub releases workflow; version bumped in README |

## M3 — XMP → Windows PKEY mapping (initial)

| XMP field | Windows PKEY |
|---|---|
| `xmp:Rating` | `System.Rating`, `System.SimpleRating` |
| `dc:title` | `System.Title` |
| `dc:description` | `System.Comment` |
| `dc:subject` (bag) | `System.Keywords` |
| `dc:creator` (seq) | `System.Author` |
| `photoshop:DateCreated` / `xmp:CreateDate` | `System.Photo.DateTaken` |
| `photoshop:Headline` | `System.Photo.Event` |
| `Iptc4xmpCore:Location` | `System.Photo.LocationName` |
| `exif:GPS*` | `System.GPS.*` |
| `photoshop:City` / `State` / `Country` | `System.GPS.*` or `System.Photo.*` (TBD) |

## Open questions / flags for future-self

- **xmp-toolkit-rs liveness** — Task #1. If dead, add ~1 day for C-ABI wrapper.
- **Azure Trusted Signing identity verification** — can take days; user should start verification early (during M3 or so) so it's ready for M8.
- **Build on host vs in VM** — default plan is build-inside-VM. Revisit if iteration speed hurts.
- **Custom `.propdesc` schema (M7)** — skip if the user doesn't care about fields without native PKEYs.

## Prior art consulted

- **Dijji/FileMeta** — reference for shell property handler plumbing; does not do sidecars (stores in NTFS alt streams).
- **FastPictureViewer Codec Pack** — historical precedent; unmaintained, Windows 7/8 era.
- **ExifTool sidecar docs** — authoritative reference for matching rules and precedence.
- Adobe XMP Toolkit C++ SDK — parser.
- Microsoft Learn: shell extension handlers, property handlers, `IPropertyStore`.

## Conventions

- Per `CLAUDE.md`: present plans and wait for user approval before implementation. Always confirm target branch before git operations. After implementing a feature, do a full build and test before committing.
- Installer build happens on GitHub via the releases workflow, not locally.
