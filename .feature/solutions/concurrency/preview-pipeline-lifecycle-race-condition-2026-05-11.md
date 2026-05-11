---
title: "Preview Pipeline Lifecycle Race Condition — Mode Switch and Stream Restart"
date: 2026-05-11
category: concurrency
module: src/components/MainPreview.vue, src-tauri/src/pipeline/manager.rs
problem_type: logic_error
component: MJPEG preview pipeline, Vue component lifecycle
severity: high
tags: [gstreamer, preview, vue, lifecycle, race-condition, mjpeg, v-if, onUnmounted]
---

## Problem

When switching between "推流服务" (streaming server) and "RTSP客户端" (client) modes, or when stopping and restarting a stream, the preview displays the last frozen frame instead of live video. The MJPEG preview does not recover.

## Symptoms

- Switch from server mode → client mode → back to server mode: preview shows the last frozen frame, no live updates
- Stop stream → restart stream: preview still shows the frozen frame from before the stop
- The RTSP stream itself continues to work correctly — the issue is only with the MJPEG preview in the UI
- After the fix for one scenario, the other scenario regresses

## What Didn't Work

### Attempt 1: `v-if` + `onUnmounted` stops preview on mode switch

Added `onUnmounted(() => stopPreview())` to call backend `stop_preview` when the component is destroyed (mode switch). Added `onMounted(() => startPreview())` to restart preview when the component is recreated.

**Why it failed**: `onUnmounted` is async (calls `invoke('stop_preview')`), but Vue does not await it. When switching back, `onMounted` calls `start_preview` immediately. The old `stop_preview` may still be executing concurrently. Specifically, `stop_preview` calls `remove_preview_source()` which removes the source from the MJPEG server — this can delete the source that `start_preview` just registered, leaving the new preview pipeline with no MJPEG source.

### Attempt 2: `v-show` instead of `v-if`

Tried using `v-show` to keep the component alive across mode switches.

**Why it failed**: `v-show` uses `display: none`, which causes the browser to disconnect the MJPEG multipart stream. When switching back, the `<img>` element has a stale connection. Also, grid layout with `display: contents` caused IntersectionObserver issues.

### Attempt 3: `v-if` + `:key` to force rebuild + `onUnmounted` cleanup

Used `:key="serverModeKey"` on MainPreview and incremented key on mode switch back.

**Why it failed**: Same fundamental issue as Attempt 1 — the `onUnmounted` → `stop_preview` → `remove_preview_source` race condition still applies. The key forces a rebuild, but the old component's async `stopPreview()` can still interfere with the new component's `startPreview()`.

### Attempt 4: `watch(isRunning, { immediate: false })` + `onMounted` + `onUnmounted`

Split the logic: `watch` handles streaming state changes, `onMounted` handles initial connection, `onUnmounted` handles cleanup.

**Why it failed**: Same race condition. The `onUnmounted` async `stopPreview()` deletes the MJPEG source that the new `onMounted` `startPreview()` just registered.

## Solution

Two changes, one frontend and one backend:

### Frontend: Do NOT stop preview on unmount

In `MainPreview.vue`:

- **Removed `onUnmounted` entirely** — the preview pipeline stays alive when switching modes
- **`onMounted` only sets the URL** (no backend call) — since the preview pipeline is still running in the backend, just reconnect the MJPEG `<img>` element
- **`watch(isRunning)` still calls `startPreview()`/`stopPreview()`** — this handles the real streaming state changes (start/stop stream)
- **`imgKey` ref** incremented on each `startPreview()` to force `<img>` DOM re-creation, ensuring a fresh MJPEG TCP connection

```typescript
// onMounted: just reconnect — preview pipeline is still running
onMounted(() => {
  if (isRunning.value && props.selectedSource) {
    previewUrl.value = `http://127.0.0.1:${previewPort.value}/${props.selectedSource.id}`
    imgKey.value++
  }
})
// NO onUnmounted — let preview pipeline keep running
```

### Backend: Guard `remove_preview_source` with `had_preview` flag

In `manager.rs`, `stop_preview`:

```rust
pub async fn stop_preview(&self, source_id: &str) {
    let had_preview = {
        let mut pipelines = self.pipelines.lock().unwrap();
        if let Some(handle) = pipelines.get_mut(source_id) {
            if let Some(pp) = handle.preview_pipeline.take() {
                let _ = pp.stop();
                true
            } else {
                false
            }
        } else {
            // Pipeline doesn't exist — don't touch MJPEG server
            false
        }
    };
    if had_preview {
        self.remove_preview_source(source_id).await;
    }
}
```

This ensures that `remove_preview_source` is only called when we actually stopped a preview pipeline, not when the pipeline was already gone (e.g., removed by `stop_pipeline`).

## Why This Works

**Key insight**: The preview pipeline's lifecycle should be tied to the streaming pipeline, not the UI component.

1. **Mode switching**: The `v-if` destroys/recreates the UI component, but the backend preview pipeline stays running. When the component remounts, it simply reconnects to the already-running MJPEG stream. No stop/start cycle, no race condition.

2. **Stream stop/restart**: `watch(isRunning)` detects the `true → false` change and calls `stopPreview()`, which properly stops both the preview pipeline and the MJPEG source. On restart (`false → true`), `startPreview()` creates a new preview pipeline. The backend guard prevents a stale `stop_preview` from deleting the newly registered source.

3. **`imgKey` trick**: Forces Vue to destroy and recreate the `<img>` DOM element, ensuring the browser opens a fresh TCP connection to the MJPEG server. Without this, the browser may reuse a stale/closed connection and show the last frame.

## Prevention

- **Never tie backend resource lifecycle to UI component lifecycle** when the resource (preview pipeline) is independent of the UI visibility. Use reactive state (`watch(isRunning)`) instead.
- **Guard destructive async operations** — when `stop_preview` runs asynchronously, ensure it cannot delete resources registered by a concurrent `start_preview`. The `had_preview` flag pattern prevents this.
- **Use `imgKey` pattern** for MJPEG streams — browsers cache multipart/x-mixed-replace connections. Force `<img>` re-creation to establish fresh connections.

## Related Issues

- `gstreamer-rtsp-runtime-environment-2026-05-06.md` — GST_PLUGIN_SCANNER path issues
- `rust-multi-mutex-patterns-2026-04-30.md` — Rust async mutex patterns
