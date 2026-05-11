---
title: "GStreamer gst-plugin-scanner.exe Missing from Windows Build Package"
date: 2026-05-11
category: build-issues
module: scripts/prepare-gstreamer-runtime.ps1, src-tauri/tauri.conf.json
problem_type: build_error
component: GStreamer runtime packaging, Tauri bundle resources
severity: medium
tags: [gstreamer, windows, packaging, nsis, msi, gst-plugin-scanner, tauri]
---

## Problem

Building the Windows installer (NSIS/MSI) fails with:

```
resource path `gstreamer-runtime\lib\gstreamer-1.0\gst-plugin-scanner.exe` doesn't exist
failed to build app: failed to build app
```

## Symptoms

- `npx tauri build` fails with "resource path doesn't exist"
- The `gstreamer-runtime/lib/gstreamer-1.0/` directory exists but is empty (no plugins, no scanner)
- The `prepare-gstreamer-runtime.ps1` script appears to run successfully but doesn't actually copy `gst-plugin-scanner.exe`

## Root Cause

Two issues:

1. **Script looks in wrong directory**: The `prepare-gstreamer-runtime.ps1` script searches for `gst-plugin-scanner.exe` in `bin/` of the GStreamer SDK. On Windows with the MSVC x86_64 SDK, the file is actually located at `libexec/gstreamer-1.0/gst-plugin-scanner.exe`.

2. **Empty runtime directory**: The `gstreamer-runtime/` directory exists with the correct subdirectory structure but contains no files — the script was either never run, or the `gst-plugin-scanner.exe` copy silently failed because the source path was wrong.

## Solution

### 1. Fix the script to look in `libexec/gstreamer-1.0/`

In `prepare-gstreamer-runtime.ps1`, the scanner copy logic should check `libexec/gstreamer-1.0/` first:

```powershell
# gst-plugin-scanner.exe is in libexec/gstreamer-1.0/, NOT in bin/
$scanner = Join-Path $GstRoot "libexec\gstreamer-1.0\gst-plugin-scanner.exe"
if (Test-Path $scanner) {
    Copy-Item $scanner $pluginOutDir -Force
    Write-Host "Copied gst-plugin-scanner.exe to lib/gstreamer-1.0/"
} else {
    # Fallback: check bin/ as well
    $scannerBin = Join-Path $binDir "gst-plugin-scanner.exe"
    if (Test-Path $scannerBin) {
        Copy-Item $scannerBin $pluginOutDir -Force
    } else {
        Write-Warning "gst-plugin-scanner.exe not found"
    }
}
```

### 2. Verify after running the script

After running the script, confirm the file exists:

```powershell
Test-Path "src-tauri/gstreamer-runtime/lib/gstreamer-1.0/gst-plugin-scanner.exe"
```

### 3. tauri.conf.json resource path is correct

The `tauri.conf.json` already has the correct resource path:

```json
"resources": [
  "gstreamer-runtime/bin/*.dll",
  "gstreamer-runtime/lib/gstreamer-1.0/*.dll",
  "gstreamer-runtime/lib/gstreamer-1.0/gst-plugin-scanner.exe"
]
```

## Prevention

- **Always verify the GStreamer SDK directory structure** before writing copy scripts — the layout differs between Linux and Windows, and between MinGW and MSVC builds
- **Add explicit Test-Path assertions** after critical file copies in the script to fail fast if something is missing
- **Run the prepare script before every `tauri build`** — consider adding it to `beforeBuildCommand` in `tauri.conf.json` or documenting it as a required pre-build step

## Related Issues

- `gstreamer-rtsp-runtime-environment-2026-05-06.md` — GST_PLUGIN_SCANNER runtime environment setup
- `tauri-gstreamer-windows-build-2026-04-30.md` — Initial GStreamer + Tauri Windows build setup
