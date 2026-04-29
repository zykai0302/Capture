# Quality Guidelines

> Code quality standards for backend development.

---

## Overview

This project's Rust backend follows strict concurrency safety rules due to the multi-Mutex architecture of `GstPipelineManager`. All GStreamer and Win32 API calls must respect async/blocking boundaries.

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
