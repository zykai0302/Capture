---
title: "GStreamer ElementFactory Panic + Remote Control Resolution Missing in RTSP Client"
date: 2026-05-09
category: build-issues
module: src-tauri/src/rtsp/client, src-tauri/src/remote/ws_client, src-tauri/src/lib, src/composables/useWsRemote, src/App
problem_type: runtime_error
component: GStreamer pipeline creation, WebSocket remote control coordinate mapping
severity: high
tags: [gstreamer, element-factory, panic, parse-launch, resolution, coordinate-mapping, cross-layer, rtsp-client]
---

## Problem

Two independent bugs prevent the RTSP client mode from functioning:

1. **Application panic** when clicking "连接" in RTSP client mode — `ElementFactory::make("jpegenc").build()` panics in `src/rtsp/client.rs:103`
2. **Remote control silently fails** — all mouse/keyboard commands return "Remote resolution not available" error because `WsRemoteClient.remote_resolution` is never set

## Symptoms

### Bug 1: Panic on Connect

- Clicking "连接" to add an RTSP stream causes the application to crash
- Console shows repeated PANIC messages: `location: Location { file: "src\\rtsp\\client.rs", line: 103, column: 14 }`
- The panic happens inside `spawn_blocking`, so the tokio runtime survives but the connection fails silently
- Frontend retries on a loop, producing multiple consecutive panics

### Bug 2: Remote Control Does Nothing

- RTSP stream connects and displays correctly
- WebSocket remote control connects successfully (green indicator)
- Mouse clicks/movements on the preview produce no effect on the remote machine
- No visible error in the UI — the error is caught by `useWsRemote.ts` and stored in `error.value` but not displayed
- Backend logs: `Remote resolution not available`

## What Didn't Work

### 1. Using `ElementFactory::make().build()` for GStreamer pipeline creation

The original `client.rs` used the element-by-element approach:

```rust
let jpegenc = gstreamer::ElementFactory::make("jpegenc")
    .property("quality", 60u32)
    .build()
    .map_err(|e| AppError::RtspClient(format!("Failed to create jpegenc: {}", e)))?;
```

This panics because:
- `gstreamer-rs 0.23`'s `.build()` internally calls `glib::Class::<Element>::from_type(element_type).unwrap()` which panics when the GType is invalid
- Even though the `jpegenc` plugin exists in `D:\msvc_x86_64\lib\gstreamer-1.0\`, the factory lookup + class resolution chain can fail in `spawn_blocking` threads where GStreamer's plugin registry state may differ from the main thread
- The server-side preview pipeline uses `parse::launch()` and works fine with the same `jpegenc` element

### 2. Not setting `remote_resolution` on `WsRemoteClient`

The `WsRemoteClient.send_command()` method at `ws_client.rs:309-311`:

```rust
let resolution = self.remote_resolution.lock().await;
let (w, h) = resolution
    .ok_or_else(|| AppError::Remote("Remote resolution not available".to_string()))?;
```

This requires `remote_resolution` to be set via `set_remote_resolution()` before any mouse command can work. However:
- The `ws_remote_connect` command creates a `WsRemoteClient` but never sets the resolution
- The RTSP client detects resolution from GStreamer `pad-added` caps but stores it in `RtspClientHandle.resolution` — there's no bridge between the two systems
- No Tauri command existed to set the resolution from the frontend

### 3. Auto-naming GStreamer elements in `parse_launch`

The initial `parse_launch` fix used auto-named elements:

```
rtspsrc name=src ... ! decodebin name=decoder ! videoconvert ! jpegenc quality=60 ! appsink name=sink ...
```

After `parse_launch`, elements are retrieved by `pipeline.by_name()`. `videoconvert` auto-names to `videoconvert0`, which is unreliable and undocumented. This caused element lookup failures.

## Solution

### Fix 1: Use `parse::launch()` instead of `ElementFactory::make().build()`

Replaced the element-by-element pipeline construction with `parse::launch()`, matching the server-side preview pattern:

```rust
let launch_str = format!(
    "rtspsrc name=src protocols={} latency=0 ! \
     decodebin name=decoder ! \
     videoconvert name=vconv ! \
     jpegenc name=jenc quality=60 ! \
     appsink name=sink emit-signals=true max-buffers=1 drop=true",
    protocols_val
);

let pipeline = gstreamer::parse::launch(&launch_str)?;
let pipeline = pipeline.downcast::<gstreamer::Pipeline>()?;

// Get elements by explicit name
let rtspsrc = pipeline.by_name("src").ok_or_else(|| ...)?;
let decodebin = pipeline.by_name("decoder").ok_or_else(|| ...)?;
let videoconvert_el = pipeline.by_name("vconv").ok_or_else(|| ...)?;
let appsink_el = pipeline.by_name("sink").ok_or_else(|| ...)?;
```

Key changes:
- All elements get explicit `name=` attributes (no auto-naming)
- `rtspsrc` dynamic properties (`location`, `user-id`, `user-pw`) set after creation via `set_property()`
- Removed redundant `gstreamer::init()` call (already done in `main.rs`)

### Fix 2: Add `ws_remote_set_resolution` command + auto-sync

**Backend** — New Tauri command in `lib.rs`:

```rust
#[tauri::command]
async fn ws_remote_set_resolution(
    width: u32,
    height: u32,
    state: tauri::State<'_, AppState>,
) -> Result<(), AppError> {
    let client = state.ws_remote_client.lock().await;
    if let Some(client) = client.as_ref() {
        client.set_remote_resolution(width, height).await;
        Ok(())
    } else {
        Err(AppError::Remote("WebSocket client not connected".to_string()))
    }
}
```

**Frontend** — New method in `useWsRemote.ts`:

```typescript
async function setResolution(width: number, height: number) {
  try {
    await invokeWithTimeout('ws_remote_set_resolution', { width, height })
  } catch (e: any) {
    error.value = String(e)
  }
}
```

**Auto-sync** — `App.vue` watches RTSP client resolution changes:

```typescript
watch(
  () => rtspClientStore.streams.value,
  (streams) => {
    if (!wsRemoteStore.status.value.is_connected) return
    for (const streamId in streams) {
      const s = streams[streamId]
      if (s.state === 'Connected' && s.resolution) {
        const [w, h] = s.resolution
        wsRemoteStore.setResolution(w, h)
        break
      }
    }
  },
  { deep: true }
)
```

### Fix 3: `manager` move-after-use compilation error

After refactoring `rtsp_client_connect` to use `parse_launch`, the `manager` was moved into `spawn_blocking` but still needed by `register_mjpeg`:

```rust
let manager = state.rtsp_client_manager.clone();
let manager_for_mjpeg = manager.clone();  // Clone before move
let (stream_id, rx) = tokio::task::spawn_blocking(move || {
    manager.create_pipeline(...)
}).await??;
manager_for_mjpeg.register_mjpeg(&stream_id, rx).await?;
```

## Why This Works

1. **`parse::launch()` delegates element creation to GStreamer's C API** — The C `gst_parse_launch()` handles element factory lookup, property setting, and linking internally, avoiding the Rust bindings' `.build()` unwrap chain that panics on missing/invalid elements.

2. **Explicit element naming** ensures `pipeline.by_name()` always finds the expected element — `videoconvert name=vconv` is deterministic, while `videoconvert` → `videoconvert0` is not.

3. **Resolution auto-sync** bridges the gap between RTSP client (which discovers resolution from SDP/caps) and WebSocket client (which needs resolution for coordinate mapping). The `watch` on `rtspClientStore.streams` fires every time `rtsp_client_status` polling updates, keeping the resolution in sync.

4. **Cloning `manager` before move** is the standard Rust pattern for using an `Arc` value both inside and outside a `move` closure.

## Prevention

### Rule: Use `parse::launch()` for GStreamer pipelines in this project

The server-side preview (`pipeline/preview.rs`) uses `parse::launch()`. The RTSP client should follow the same pattern. **Avoid `ElementFactory::make().build()`** — it panics on missing elements instead of returning an error, and the panic is unrecoverable in `spawn_blocking`.

Exceptions: When you need dynamic property setting (like `rtspsrc`'s `location`, `user-id`, `user-pw`), create the pipeline with `parse::launch()`, then set properties via `element.set_property()` after retrieving the element by name.

### Rule: Always set `remote_resolution` when connecting WS remote control

The `WsRemoteClient` requires `remote_resolution` for coordinate mapping. Any code path that:
1. Creates a `WsRemoteClient` (via `ws_remote_connect`)
2. Then expects mouse commands to work

Must also call `set_remote_resolution()` with the remote screen dimensions. The auto-sync watch in `App.vue` handles this, but if the architecture changes, this invariant must be maintained.

### Rule: Audit cross-layer data dependencies

When two subsystems (RTSP client and WS remote client) need shared data (resolution), ensure there's a bridge:
- **Backend**: Tauri command to set the value
- **Frontend**: Composable method to call the command
- **Glue**: `watch()` or event handler to auto-sync when the source data changes

Without this bridge, each subsystem works in isolation but the integrated feature fails silently.

## Related Issues

- `.feature/solutions/build-issues/gstreamer-rtsp-runtime-environment-2026-05-06.md` — GStreamer runtime environment (companion: plugin discovery vs. element creation panic)
- `.feature/solutions/integration-issues/cross-layer-field-omission-tauri-commands-2026-05-08.md` — Cross-layer data flow (same pattern: field exists in one layer but not bridged to another)
- `src-tauri/src/rtsp/client.rs` — RTSP client pipeline creation
- `src-tauri/src/remote/ws_client.rs` — WebSocket client with coordinate mapping
- `src-tauri/src/pipeline/preview.rs` — Server-side preview using `parse::launch()` (reference pattern)
