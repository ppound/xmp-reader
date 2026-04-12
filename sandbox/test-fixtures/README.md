# test-fixtures

Place JPEG + XMP sidecar pairs here for smoke-test verification.

Expected layout (ExifTool sidecar conventions — both forms):

```
photo.jpg
photo.xmp          ← preferred form
photo.jpg.xmp      ← alternate form (also checked by xmp-reader)
```

This folder is populated in M2 once the sidecar parser is implemented.
When the sandbox is running, Explorer opens to this folder automatically
after DLL registration — select a file and open the Details pane
(View → Details pane) to verify XMP fields are visible.
