---
title: "Minimized Window Coordinates + PixPin Filtering + DXGI Monitor Index Mapping"
date: 2026-05-11
category: build-issues
module: src-tauri/src/capture/platform/windows.rs, src-tauri/src/pipeline/gst_pipeline.rs, src-tauri/src/pipeline/manager.rs
problem_type: runtime_error
component: Window enumeration, DXGI+crop pipeline, monitor index resolution
severity: high
tags: [windows, minimized-window, getwindowrect, getwindowplacement, pixpin, ws-ex-toolwindow, dxgi, monitor-index, crop-coordinates]
---

## Problem

Three interrelated bugs prevented window capture from working correctly:

1. **Minimized window preview shows wrong/no picture** — DXGI+crop pipeline uses stale coordinates from `GetWindowRect`, which returns (-32000,-32000, 160x28) for minimized windows (the taskbar icon area)
2. **PixPin window missing from source list** — Window enumeration filters out `WS_EX_TOOLWINDOW` windows, which includes PixPin's overlay
3. **DXGI+crop captures wrong monitor area** — `monitor-index` derived from Windows `\\.\DISPLAY` numbering (e.g., DISPLAY1=0, DISPLAY2=1) doesn't match `d3d11screencapturesrc`'s internal DXGI output enumeration order (primary=0, then sorted by position)

## Symptoms

- Starting a stream on a **minimized window** shows black or the wrong screen area (coordinates near -32000 are off-screen)
- **PixPin** does not appear in the window source list
- On multi-monitor setups with non-default primary display ordering, **DXGI+crop captures the wrong monitor** for a window
- All three bugs are **silent**: no errors, no panics, pipelines start successfully but capture the wrong region

## What Didn't Work

### 1. Using `source.x`/`source.y` from enumeration in pipeline construction

The `CaptureSource` struct stores `x`, `y`, `width`, `height` from enumeration time. But:

- For minimized windows, `GetWindowRect` returns the taskbar icon area, not the restored position
- By the time `build_capture_element` runs, the window may have moved or been resized
- The coordinates are stale — they don't reflect the window's current position

### 2. Keeping `WS_EX_TOOLWINDOW` filter for window enumeration

`WS_EX_TOOLWINDOW` (0x80) was originally filtered to remove floating toolbars and tooltips. However:

- PixPin's overlay window has `WS_EX_TOOLWINDOW` set
- Some legitimate user-visible windows (floating toolbars, overlay apps) also set this style
- The filter is too broad — it removes user-visible windows that should be capturable

### 3. Deriving `monitor-index` from `\\.\DISPLAY` device name

The previous code extracted the display number from `\\.\DISPLAY1` → index 0, `\\.\DISPLAY2` → index 1. But `d3d11screencapturesrc` uses DXGI output enumeration order:

- Primary monitor (origin 0,0) = index 0
- Then remaining monitors sorted by position (left, top)
- If `\\.\DISPLAY2` is the primary monitor at (0,0), it should be index 0, not index 1

This caused DXGI+crop to capture the wrong monitor when display numbering didn't match DXGI order.

### 4. Using `monitor-handle` (HMONITOR) for window DXGI+crop

Previous attempts used `monitor-handle` (HMONITOR) for window capture pipelines. However:

- `d3d11screencapturesrc` with `monitor-handle` + `crop-x/y` is ambiguous — the handle identifies the monitor, but crop coordinates must be relative to that monitor's origin
- The monitor handle belongs to the `CaptureSource.handle` for monitors, but for windows the handle is the HWND
- This was a conceptual confusion: window sources should use `monitor-index` (derived from window position), not `monitor-handle`

## Solution

### Fix 1: Use `IsIconic()` + `GetWindowPlacement()` for minimized windows

In both `windows.rs` (enumeration) and `gst_pipeline.rs` (pipeline construction), detect minimized windows and use restored coordinates:

```rust
let (win_x, win_y, win_w, win_h) = if IsIconic(hwnd_ptr).as_bool() {
    // Minimized: GetWindowRect returns (-32000,-32000, 160x28)
    // Use GetWindowPlacement to get the restored position instead
    let mut wp: WINDOWPLACEMENT = std::mem::zeroed();
    wp.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
    let _ = GetWindowPlacement(hwnd_ptr, &mut wp);
    let r = wp.rcNormalPosition;
    ((r.right - r.left).max(0) as u32, (r.bottom - r.top).max(0) as u32, r.left, r.top)
} else {
    // Normal: GetWindowRect returns actual window position
    let mut rect = std::mem::zeroed();
    let _ = GetWindowRect(hwnd_ptr, &mut rect);
    (rect.left, rect.top, (rect.right - rect.left).max(0) as u32, (rect.bottom - rect.top).max(0) as u32)
};
```

Applied in three places:
- `windows.rs::enum_windows_callback` — enumeration stores correct coordinates
- `gst_pipeline.rs::build_capture_element` — RTSP pipeline uses live window rect
- `gst_pipeline.rs::build_dxgi_crop_preview_capture_string` — preview fallback pipeline uses live rect

### Fix 2: Replace `WS_EX_TOOLWINDOW` filter with `WS_EX_NOREDIRECTIONBITMAP`

```rust
// Old: skips PixPin and other legitimate overlay windows
// let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
// if ex_style & 0x80 != 0 { return BOOL(1); }

// New: skip only NOREDIRECTIONBITMAP (UWP background windows)
let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
if ex_style & 0x00200000 != 0 { return BOOL(1); }
```

`WS_EX_NOREDIRECTIONBITMAP` (0x200000) filters UWP background windows (Microsoft Store, Microsoft Text Input Application) that have no visible surface. These are the actual hidden windows that should be excluded.

### Fix 3: Sort monitors by DXGI enumeration order for `monitor-index`

```rust
// Collect monitors as (device_name, left, top, right, bottom)
let monitors: Vec<(String, i32, i32, i32, i32)> = ...;

// d3d11screencapturesrc monitor-index follows DXGI output enumeration order:
// primary monitor (origin 0,0) = index 0, then sorted by (left, top)
let mut sorted: Vec<_> = monitors.iter().collect();
sorted.sort_by(|a, b| {
    let a_primary = a.1 == 0 && a.2 == 0;
    let b_primary = b.1 == 0 && b.2 == 0;
    b_primary.cmp(&a_primary) // primary=true sorts first
        .then(a.1.cmp(&b.1))  // then by left
        .then(a.2.cmp(&b.2))  // then by top
});

for (idx, m) in sorted.iter().enumerate() {
    if x >= m.1 && x < m.3 && y >= m.2 && y < m.4 {
        return (idx as i32, m.1, m.2);
    }
}
```

This ensures `monitor-index` matches what `d3d11screencapturesrc` expects, regardless of Windows display numbering.

## Why This Works

1. **`GetWindowPlacement` returns the restored position** — When a window is minimized, `WINDOWPLACEMENT.rcNormalPosition` contains the position and size the window would have if restored. This is exactly what DXGI+crop needs to capture the right screen area.

2. **`WS_EX_NOREDIRECTIONBITMAP` is the correct filter** — UWP background windows (Microsoft Store, Text Input) set this flag because they redirect to a virtual surface that DXGI cannot capture. Regular overlay windows like PixPin do NOT set this flag.

3. **DXGI output enumeration is deterministic** — Primary monitor (origin 0,0) is always index 0, then remaining monitors sort by position. This matches how `d3d11screencapturesrc` assigns `monitor-index`, regardless of the `\\.\DISPLAY` naming convention.

4. **Querying live window rect at pipeline construction time** ensures coordinates are current, not stale from enumeration time.

## Prevention

### Rule: Never use `GetWindowRect` for minimized windows

Always check `IsIconic()` before calling `GetWindowRect`. For minimized windows, use `GetWindowPlacement` to get `rcNormalPosition`. This applies to:

- Window enumeration (`windows.rs`)
- Pipeline construction (`gst_pipeline.rs`)
- Thumbnail capture (`thumbnail.rs` — already uses `PrintWindow` which handles this)
- Any code that needs window coordinates

### Rule: Don't filter `WS_EX_TOOLWINDOW` in window enumeration

`WS_EX_TOOLWINDOW` is not a reliable indicator of "non-capturable" windows. Use `WS_EX_NOREDIRECTIONBITMAP` (0x200000) instead — it specifically marks windows without a redirectable surface (UWP backgrounds).

### Rule: Use DXGI enumeration order for `monitor-index`, not Windows display numbering

`d3d11screencapturesrc`'s `monitor-index` follows DXGI output enumeration order (primary=0, then sorted by position). Do NOT derive the index from `\\.\DISPLAY` device names. The `get_monitor_info_for_window` function must enumerate monitors and sort them to match DXGI's ordering.

### Known limitation: minimized windows cannot produce live video

Neither WGC nor DXGI+crop can capture the actual content of a minimized window in real-time:
- **WGC**: Stops producing frames when the window is minimized (OS limitation)
- **DXGI+crop**: Captures screen pixels at the restored position — shows whatever is there, not the minimized window's content
- **PrintWindow with PW_RENDERFULLCONTENT**: CAN capture minimized window content, but is too slow for real-time video (used only for thumbnails)

For minimized windows, consider showing an error message or placeholder in the UI rather than displaying incorrect screen content.

## Related Issues

- `.feature/solutions/integration-issues/cross-layer-field-omission-tauri-commands-2026-05-08.md` — Same monitor-index mismatch from a different angle (handle field not passed through Tauri commands)
- `.feature/solutions/build-issues/gstreamer-element-factory-panic-remote-resolution-2026-05-09.md` — WGC play() failure for PixPin (related: WGC cannot capture PixPin's overlay)
- `src-tauri/src/capture/platform/windows.rs` — Window enumeration with filtering
- `src-tauri/src/pipeline/gst_pipeline.rs` — Pipeline construction with DXGI+crop
- `src-tauri/src/capture/thumbnail.rs` — Thumbnail capture using PrintWindow (works for minimized windows)
