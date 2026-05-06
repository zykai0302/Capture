---
title: "GStreamer Runtime Environment + gst-rtsp-server Threading + Pipeline Launch String Integration"
date: 2026-05-06
category: build-issues
module: src-tauri/src/main, src-tauri/src/rtsp, src-tauri/src/pipeline
problem_type: integration_issue
component: GStreamer init, RTSP server, pipeline launch string
severity: high
tags: [gstreamer, rtsp, threading, environment-variables, d3d12, pipeline, vlc, windows]
---

## Problem

VLC cannot connect to the RTSP stream (`rtsp://127.0.0.1:8554/screen-0`). The RTSP server starts and reports success, but clients receive no media data. Root causes span three layers: missing runtime GStreamer environment variables, incorrect gst-rtsp-server threading model, and hardcoded CPU encoder in the launch string.

## Symptoms

- VLC error: "无法打开 MRL `rtsp://127.0.0.1:8554/screen-0`"
- Application log shows `RTSP server started on port 8554` and `Pipeline started for source: screen-0` — appears healthy
- VLC can establish TCP connection to RTSP port, but DESCRIBE/SETUP returns empty or invalid SDP
- `gst-plugin-scanner` warnings in stderr during startup
- Port discrepancy: VLC tries port 8555 but server listens on 8554 (front-end display vs actual)

## What Didn't Work

### 1. Missing Runtime GStreamer Environment

The build environment sets `GST_PLUGIN_PATH` and `GST_PLUGIN_SCANNER` for compilation, but these are **not** available at runtime when the user launches the application directly (not via `tauri dev`). Without `GST_PLUGIN_PATH`, GStreamer cannot find `d3d12screencapturesrc`, `amfh264enc`, `x264enc`, or any plugins — `gstreamer::init()` succeeds but no elements are available.

Without `GST_PLUGIN_SCANNER`, GStreamer spawns a child process that also can't find plugins, leading to noisy warnings.

### 2. gst-rtsp-server MainContext Mismatch

The original `server.rs` created the RTSP server on the calling thread's GLib main context, then called `server.attach()` and `main_loop.run()` on a different thread. This violates a fundamental gst-rtsp-server requirement:

> The RTSP server, its socket watches, and the MainLoop **must** operate on the **same** GLib MainContext.

When `attach()` runs on one context and `main_loop.run()` on another, socket watches never fire — the server binds a port but never processes incoming RTSP requests. Clients see the TCP connection succeed but get no response to DESCRIBE.

### 3. build_launch_string Hardcodes CPU Encoder

The function `build_launch_string()` constructed the encoder string directly without calling `build_encoder_element()`, which contains the GPU→CPU fallback logic. Result: every stream used `x264enc` even when GPU encoders (`amfh264enc`, `mfh264enc`) were available. This works but:
- Misses GPU acceleration (higher CPU usage)
- The `detect_encoder_info()` in `manager.rs` reported GPU encoder while the actual pipeline used CPU — status/data mismatch

### 4. d3d11 vs d3d12 Screen Capture

`d3d11screencapturesrc` fails inside gst-rtsp-server media threads because D3D11 device initialization requires a specific thread apartment model that conflicts with the RTSP media thread pool. Switching to `d3d12screencapturesrc` resolves this.

## Solution

### Fix 1: Runtime GStreamer Path Auto-Detection (main.rs)

Added environment variable detection at application startup, **before** `gstreamer::init()`:

```rust
// Priority order for GST_PLUGIN_PATH:
// 1. <exe_dir>/gstreamer-1.0  (packaged/production layout)
// 2. GSTREAMER_1_0_ROOT_MSVC_X86_64/lib/gstreamer-1.0  (dev fallback)
// Same priority for GST_PLUGIN_SCANNER
```

This ensures plugins are discoverable whether running from:
- Development (`cargo tauri dev` with env vars set)
- Installed application (plugins packaged alongside executable)
- Direct binary launch (env vars already set system-wide)

Key: only set env vars if they're not already set — don't override user/system configuration.

### Fix 2: Dedicated RTSP Thread with Same MainContext (server.rs)

Refactored `RtspServer::new()` to spawn a dedicated thread that:
1. Creates its own `MainContext`
2. Acquires it with `context.with_thread_default()`
3. Creates `MainLoop`, `RTSPServer`, calls `attach()` — **all on the same context**
4. Uses a channel-based polling loop (`timeout_source_new` at 100ms) to handle `add_stream`/`remove_stream` requests from the main thread

```rust
std::thread::spawn(move || {
    let context = MainContext::new();
    let ctx = context.clone();
    context.with_thread_default(move || {
        let main_loop = MainLoop::new(Some(&ctx), false);
        let server = RTSPServer::new();
        server.set_service(&port_str);
        server.attach(Some(&ctx));  // SAME context as MainLoop
        // ...
        main_loop.run();
    });
});
```

Synchronization: a `ready_tx/ready_rx` channel pair ensures `RtspServer::new()` blocks until the server thread reports success or failure.

### Fix 3: Launch String Uses GPU Encoder Selection

`build_launch_string()` now uses the full encoder selection chain (GPU→CPU fallback) via `build_encoder_element()`. The launch string format is:

```
( d3d12screencapturesrc monitor-index=0 ! videoconvert ! <encoder> ! <rtp-payloader> name=pay0 pt=96 )
```

Where `<encoder>` is dynamically selected based on available hardware.

### Fix 4: Pipeline Visual URL Update

`PipelineVisual.vue` default URL updated from `rtsp://localhost:8554` to `rtsp://127.0.0.1:8554` for consistency with the backend.

## Why This Works

1. **Runtime env detection** → GStreamer finds all plugins regardless of launch method. `d3d12screencapturesrc`, `amfh264enc`, `x264enc` are all discoverable. Without this, `ElementFactory::find()` returns `None` for every element.

2. **Same-context threading** → gst-rtsp-server's socket watches are on the same GLib context as the MainLoop, so incoming RTSP requests are processed. The `with_thread_default()` pattern is the canonical way to achieve this in gstreamer-rs.

3. **GPU encoder in launch string** → The pipeline uses the best available encoder. The status reported to the frontend matches the actual encoder in use.

4. **d3d12screencapturesrc** → Works in RTSP media threads where d3d11 doesn't. D3D12 device creation is more flexible about thread context.

## Prevention

### For GStreamer Runtime Apps

Always set `GST_PLUGIN_PATH` and `GST_PLUGIN_SCANNER` before `gstreamer::init()`. Don't rely on them being set in the shell environment — users may launch the app from shortcuts, file managers, or auto-start where no shell profile is loaded.

### For gst-rtsp-server in Rust

The **golden rule**: RTSPServer, socket watches, and MainLoop must share one MainContext. The safest pattern is:
1. Create a dedicated thread
2. Create a new MainContext in that thread
3. Use `with_thread_default()` to do everything
4. Communicate with other threads via channels + `timeout_source_new` polling

### For Pipeline Launch Strings

When building launch strings for `RTSPMediaFactory::set_launch()`, always use the same encoder selection logic as the status reporting. A mismatch between "what the pipeline uses" and "what the status shows" causes confusion and prevents GPU acceleration.

### For GStreamer Screen Capture on Windows

- `d3d11screencapturesrc`: Fails in RTSP media threads (D3D11 device init issue)
- `d3d12screencapturesrc`: Works in RTSP media threads (prefer this)
- Both require `GST_PLUGIN_PATH` to include the plugin directory

## Related Issues

- `.feature/solutions/build-issues/tauri-gstreamer-windows-build-2026-04-30.md` — Build-time environment setup (companion to this runtime setup)
- `.feature/solutions/concurrency/rust-multi-mutex-patterns-2026-04-30.md` — TOCTOU fix in `ensure_rtsp_server` (related lock management)
- `src-tauri/src/main.rs` — Runtime env detection implementation
- `src-tauri/src/rtsp/server.rs` — Thread-per-context RTSP server implementation
- `src-tauri/src/pipeline/gst_pipeline.rs` — Launch string builder with GPU fallback
