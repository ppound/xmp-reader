# Manual Explorer Test Checklist

Run after each milestone or significant change. All tests performed in
the Hyper-V dev VM with the handler installed via `reset-handler.ps1 -Install`.

## Prerequisites

- [ ] Handler built: `cargo build --release`
- [ ] Handler signed: `scripts/sign-dll.ps1`
- [ ] Handler installed: `scripts/reset-handler.ps1 -Install`

## 1. JPEG without sidecar (embedded metadata preserved)

- [ ] Open a folder containing a JPEG with embedded EXIF/XMP (e.g. a camera photo)
- [ ] Select the file, open Details pane (Alt+Shift+P)
- [ ] Verify embedded fields appear: Date taken, Dimensions, Camera model, etc.
- [ ] Verify no regression vs. the system handler (same fields visible)

## 2. JPEG with XMP sidecar (sidecar fields visible)

- [ ] Place a `.xmp` sidecar next to a JPEG (matching stem, e.g. `photo.xmp` for `photo.jpg`)
- [ ] Select the JPEG, check Details pane
- [ ] Verify sidecar fields appear: Title, Tags/Keywords, Rating, Authors, Comment
- [ ] Verify embedded-only fields (Dimensions, Camera model) are still present

## 3. Sidecar overrides embedded on conflict

- [ ] Use a JPEG that has embedded title/rating AND a sidecar with different title/rating
- [ ] Verify the sidecar values win (Details pane shows sidecar title/rating)

## 4. ExifTool-style sidecar naming

- [ ] Place sidecar as `photo.jpg.xmp` (not `photo.xmp`)
- [ ] Verify it is still discovered and fields appear

## 5. Stem sidecar preferred over ext sidecar

- [ ] Place both `photo.xmp` and `photo.jpg.xmp` in the same folder
- [ ] Verify `photo.xmp` values are used (stem match wins)

## 6. Multi-format registration

- [ ] Check registry for all extensions:
  ```powershell
  foreach ($ext in @(".jpg",".cr2",".nef",".arw",".dng",".raf",".tif",".tiff")) {
      $path = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\PropertySystem\PropertyHandlers\$ext"
      if (Test-Path $path) { (Get-ItemProperty $path)."(default)" }
  }
  ```
- [ ] All should show `{D4B5A6C7-8E9F-4A1B-BC2D-3E4F5A6B7C8D}`

## 7. Clean uninstall

- [ ] Run `scripts/reset-handler.ps1 -Uninstall`
- [ ] Verify all extensions restored to original system handler
- [ ] Verify `OldHandler` values are removed
- [ ] Open a JPEG in Explorer -- embedded metadata still appears (system handler restored)

## 8. RAW format (if test files available)

- [ ] Open a folder with `.cr2`, `.nef`, `.arw`, `.dng`, `.raf`, or `.tif` files
- [ ] Verify embedded metadata appears in Details pane
- [ ] Place a sidecar next to a RAW file, verify sidecar fields appear
