---
title: "Cross-Layer Field Omission in Tauri Command Data Flow"
date: 2026-05-08
category: integration-issues
module: src-tauri/src/lib.rs, src/composables, src-tauri/src/capture/thumbnail.rs
problem_type: integration_issue
component: Tauri command layer, frontend composables, GStreamer pipeline
severity: high
tags: [tauri, cross-layer, data-flow, handle, monitor-index, field-omission, integration]
---

## Problem

When a new field is added to a shared data struct (`CaptureSource.handle`), the field is correctly populated during enumeration but **silently lost** when data passes through Tauri commands. The backend reconstructs the struct from individual command parameters, defaulting the missing field to `0`/`null`/`false`. The frontend composable also omits the field from its parameter type. No compile-time or runtime error occurs — the feature simply uses a wrong fallback path.

This happened twice in the same task:
1. `start_stream` command: `CaptureSource` constructed with `handle: 0` → GStreamer pipeline used `monitor-index` instead of `monitor-handle` → **wrong display captured**
2. `capture_thumbnail` command: no `handle` parameter → thumbnail used `find_monitor_by_index` with enum-order index → **wrong display thumbnailed**

## Symptoms

- Monitor preview/stream shows the **wrong display** (e.g., Display 1 shows Display 0's content)
- Thumbnails don't match the source they represent
- The bug is **silent**: no errors, no panics, no type mismatches
- Only manifests on multi-monitor systems where enum order differs from `\\.\DISPLAY` numbering
- The `handle` field is correctly populated in `list_sources` response (because enumeration code sets it) but disappears when the frontend passes data back through other commands

## What Didn't Work

### 1. Adding the field to the struct only

Adding `handle: u64` to `CaptureSource` in `source.rs` was necessary but insufficient. The field was correctly set during `enumerate_monitors()` and returned to the frontend via `list_sources`. However:

- `lib.rs` `start_stream` command accepted individual parameters and **hardcoded `handle: 0`** when constructing `CaptureSource`
- `usePipeline.ts` `startStream()` parameter type **didn't include `handle`**
- `invoke('start_stream', {...})` **didn't pass `handle`**

### 2. Fixing only the first occurrence

After fixing `start_stream`, the same pattern existed in `capture_thumbnail`:
- `lib.rs` `capture_thumbnail` had no `handle` parameter
- `useSources.ts` `invoke('capture_thumbnail', {...})` didn't pass `handle`
- `thumbnail.rs` used `find_monitor_by_index` which relied on enum order, not HMONITOR

Fixing one code path without auditing **all** paths that construct or consume the same data type left a latent bug.

### 3. Using `monitor-index` as a display identifier

The `monitor-index` property in GStreamer's `d3d11screencapturesrc`/`d3d12screencapturesrc` uses an internal enumeration order that does **not** match the `\\.\DISPLAY` numbering used by Windows `GetMonitorInfoW`. On dual-monitor systems where monitors are swapped or have non-sequential DISPLAY numbers, `monitor-index=0` may refer to a different physical monitor than `\\.\DISPLAY1`.

## Solution

### Principle: Use OS-level handles, not index-based mapping

For Windows display identification, **always prefer HMONITOR** over index-based approaches:

1. **Enumeration**: `EnumDisplayMonitors` callback receives `HMONITOR` — store it in `CaptureSource.handle`
2. **GStreamer pipeline**: Use `monitor-handle=<HMONITOR>` property instead of `monitor-index`
3. **Thumbnail**: Use HMONITOR directly with `GetMonitorInfoW` instead of re-enumerating by index

### Fix pattern: Complete data flow audit

When adding a field to a shared struct, audit **every** path where that struct is constructed or consumed:

```
Field added to struct
  → Check all Tauri commands that construct the struct
    → Check all frontend functions that call those commands
      → Check all invoke() calls to pass the new field
        → Check all tests that mock those calls
```

### Concrete fixes applied

| Layer | File | Fix |
|-------|------|-----|
| Backend command | `lib.rs` `start_stream` | Add `handle: u64` parameter, pass to `CaptureSource` |
| Backend command | `lib.rs` `capture_thumbnail` | Add `handle: u64` parameter, pass to `thumbnail.rs` |
| Backend logic | `thumbnail.rs` `capture_monitor_thumbnail` | Accept `handle: u64`, prefer HMONITOR over index |
| Frontend composable | `usePipeline.ts` `startStream` | Add `handle` to param type and invoke call |
| Frontend composable | `useSources.ts` `fetchThumbnails` | Pass `source.handle` in invoke call |
| Frontend tests | `composables.test.ts` | Add `handle` field to test data |

### Thumbnail: HMONITOR with fallback

```rust
fn capture_monitor_thumbnail(source_id: &str, width: u32, height: u32, handle: u64) -> AppResult<Vec<u8>> {
    let monitor_handle = if handle != 0 {
        HMONITOR(handle as *mut _)  // Direct handle — reliable
    } else {
        // Fallback: parse index from source_id and enumerate
        let monitor_index: u32 = source_id.strip_prefix("screen-")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| ...)?;
        unsafe { find_monitor_by_index(monitor_index)? }
    };
    // ... use monitor_handle for BitBlt capture
}
```

## Why This Works

1. **HMONITOR is the authoritative identifier**: Windows assigns HMONITOR values that uniquely and consistently identify a physical display, regardless of `\\.\DISPLAY` numbering or enumeration order.

2. **GStreamer `monitor-handle` bypasses index mapping**: The `d3d11screencapturesrc`/`d3d12screencapturesrc` elements accept `monitor-handle` (HMONITOR) which directly opens the correct display, avoiding the internal index→adapter mapping that may differ from our `source_id` convention.

3. **Complete data flow ensures consistency**: The same `handle` value flows through:
   - `enumerate_monitors()` → `CaptureSource.handle`
   - `list_sources` → frontend `source.handle`
   - `start_stream(handle)` → GStreamer `monitor-handle`
   - `capture_thumbnail(handle)` → `GetMonitorInfoW(HMONITOR)`

## Prevention

### Rule: New struct field → Full data flow audit

When adding a field to a struct that crosses the Tauri bridge:

1. **Search all `CaptureSource { ... }` struct literals** in Rust code — ensure the new field is set
2. **Search all `invoke('command_name', { ... })` calls** in TypeScript — ensure the new field is passed
3. **Search all Tauri command function signatures** — ensure the new field is a parameter
4. **Search all test mocks** — ensure test data includes the new field
5. **Consider using source_id lookup** instead of parameter reconstruction — fetch the full `CaptureSource` from a cached store by `source_id`, eliminating the need to pass all fields through commands

### Rule: Prefer OS handles over indices

When identifying display monitors on Windows:
- **Always use HMONITOR** for programmatic identification
- **Never rely on enumeration order** (`EnumDisplayMonitors` index) as a stable identifier
- **Never assume `\\.\DISPLAY` numbering** matches any other numbering scheme
- Use index-based fallback **only** when handle is unavailable (e.g., `handle == 0`)

### Rule: Don't reconstruct structs from scattered command parameters

The `start_stream` command accepts 7+ individual parameters and reconstructs `CaptureSource`. This is fragile — any new field is easily missed. Prefer:

```rust
// Fragile: scattered parameters
async fn start_stream(source_id: String, source_type: String, ..., handle: u64, ...)

// Robust: pass only source_id, look up full struct from cache
async fn start_stream(source_id: String, state: State<'_, AppState>) {
    let source = state.source_cache.get(&source_id)?;
    pipeline_manager.start_pipeline(&source, &config)?;
}
```

## Related Issues

- `.feature/solutions/build-issues/gstreamer-rtsp-runtime-environment-2026-05-06.md` — GStreamer pipeline configuration (related `monitor-index` vs `monitor-handle` context)
- `.feature/solutions/concurrency/rust-multi-mutex-patterns-2026-04-30.md` — Lock management in `GstPipelineManager`
- `src-tauri/src/capture/platform/windows.rs` — HMONITOR enumeration with `\\.\DISPLAY` index extraction
- `src-tauri/src/capture/thumbnail.rs` — HMONITOR-based thumbnail capture
- `src-tauri/src/pipeline/gst_pipeline.rs` — `monitor-handle` in GStreamer launch string
