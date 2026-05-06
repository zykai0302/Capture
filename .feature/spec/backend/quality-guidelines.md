# Quality Guidelines

> Code quality standards for backend development.

---

## Overview

This project's Rust backend follows strict concurrency safety rules due to the multi-Mutex architecture of `GstPipelineManager`. All GStreamer and OS API calls must respect async/blocking boundaries. The codebase uses `#[cfg(target_os)]` conditional compilation for cross-platform support (Windows + macOS + Linux).

---

## Forbidden Patterns

| Pattern | Why | Do Instead |
|---------|-----|------------|
| Synchronous Tauri commands calling GStreamer APIs | Blocks tokio runtime, freezes UI | Use `async` + `spawn_blocking` |
| Holding `MutexGuard` across `.await` | `MutexGuard` is not `Send`, causes compile error or deadlock | Extract data, drop guard, then `.await` |
| Acquiring multiple locks in inconsistent order | Deadlock risk | Always acquire `pipelines` before `rtsp_server`; or release one before acquiring the other |
| Check-then-act with lock release in between | TOCTOU race | Hold lock for entire check+act sequence |
| Calling blocking OS APIs in polling threads without timeout | Thread can hang forever on UAC/secure desktop | Use `mpsc::channel` + `recv_timeout` |
| `_ =>` catch-all in enum match for distinct semantics | Silently degrades behavior | Use explicit arms for all variants |
| `console.log` / `console.debug` in frontend | Noisy production logs | Use `log::info!` / `log::warn!` in Rust; `console.error` only for error tracking |
| Platform-specific code without `#[cfg(target_os)]` guard | Compile error or runtime crash on other platforms | Use conditional compilation with platform-specific branches |

---

## Required Patterns

### Tauri Command Pattern

```rust
#[tauri::command]
async fn command_name(state: tauri::State<'_, AppState>) -> Result<T, AppError> {
    let cloned_resource = state.resource.clone();
    tokio::task::spawn_blocking(move || {
        // Blocking work here
    })
    .await
    .map_err(|e| AppError::Module(format!("Task join error: {}", e)))?
}
```

### Lock Ordering for GstPipelineManager

Documented order: `pipelines` → `rtsp_server`

When a function needs both locks:
1. Acquire `pipelines`, extract needed data
2. Release `pipelines` (drop guard)
3. Acquire `rtsp_server`
4. Perform operation
5. Release `rtsp_server`

### Cross-Platform Conditional Compilation

Platform-specific code must use `#[cfg(target_os)]`:

```rust
// Capture element — different GStreamer sources per platform
fn build_capture_element(_source: &CaptureSource) -> &'static str {
    #[cfg(target_os = "windows")]
    { "d3d12screencapturesrc" }
    #[cfg(target_os = "macos")]
    { "avfvideosrc" }
    #[cfg(target_os = "linux")]
    { "ximagesrc" }
}

// GPU encoder candidates — platform-specific lists
pub fn gpu_encoder_candidates(codec: &Codec) -> Vec<&'static str> {
    #[cfg(target_os = "windows")]
    { match codec { Codec::H264 => vec!["amfh264enc", ...], ... } }
    #[cfg(target_os = "macos")]
    { match codec { Codec::H264 => vec!["vtenc_h264"], ... } }
    #[cfg(target_os = "linux")]
    { match codec { Codec::H264 => vec!["vaapih264enc"], ... } }
}
```

Platform-specific dependencies in `Cargo.toml`:
```toml
[target.'cfg(target_os = "macos")'.dependencies]
core-graphics = { version = "0.24", features = ["highsierra"] }
core-foundation = "0.10"
```

### Serde Naming Convention

Rust uses `snake_case` field names; Tauri auto-converts to `camelCase` in JSON. TypeScript types must use `camelCase` for `invoke()` args, but the `types/index.ts` interfaces use `snake_case` to match the deserialized Rust struct names.

**Exception**: Tauri command parameter names follow the `snake_case` → `camelCase` mapping. E.g., Rust `source_id: String` becomes TypeScript `sourceId`.

---

## Testing Requirements

- All Tauri commands that interact with GStreamer must be tested via `spawn_blocking` mock
- Frontend tests must pass full source objects to `startStream` (not just IDs)
- `vitest run` must pass with 0 failures before commit
- `vue-tsc --noEmit` must pass with 0 errors

---

## Code Review Checklist

- [ ] No synchronous Tauri commands calling GStreamer/Win32 APIs
- [ ] Lock ordering is consistent (pipelines → rtsp_server)
- [ ] No `MutexGuard` held across `.await`
- [ ] No TOCTOU races (check and act under same lock hold)
- [ ] Background threads have timeout protection
- [ ] Enum matches are exhaustive (no `_ =>` for distinct variants)
- [ ] Rust ↔ TypeScript type mapping is consistent
