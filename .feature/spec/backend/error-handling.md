# Error Handling

> How errors are handled in this project.

---

## Overview

This project uses a unified error type pattern in Rust and Tauri Command error propagation. The Rust backend defines `AppError` as the single error type for all Tauri commands, and the frontend catches errors via `try/catch` on Tauri `invoke()` calls.

---

## Error Types

### Rust Backend: `AppError` (`src-tauri/src/error.rs`)

```rust
pub enum AppError {
    Capture(String),
    Encode(String),
    Pipeline(String),
    GStreamer(String),
    Remote(String),
    Config(String),
}
```

- Each variant corresponds to a backend module
- All Tauri commands return `Result<T, AppError>`
- `AppError` implements `Serialize` so it's sent to the frontend as a string

### TypeScript Frontend

No custom error types. Errors from `invoke()` are caught as `any` in `catch (e: any)` blocks and stored as `string` via `String(e)`.

---

## Error Handling Patterns

### Rust: Tauri Commands with `spawn_blocking`

All GStreamer/Win32 API calls must be wrapped in `spawn_blocking`:

```rust
#[tauri::command]
async fn list_sources() -> Result<CaptureSourceList, AppError> {
    tokio::task::spawn_blocking(|| {
        capture::platform::enumerate_sources()
    })
    .await
    .map_err(|e| AppError::Capture(format!("Task join error: {}", e)))?
}
```

- The outer `map_err` handles `JoinError` (task panic/cancellation)
- The inner `enumerate_sources()` returns `Result<CaptureSourceList, AppError>` directly

### Rust: Mutex Poisoning

All `Mutex` operations use `.unwrap()` — mutex poisoning is treated as unrecoverable. This is acceptable for a desktop application where a panic means a logic bug that should be fixed.

### Frontend: Error State in Composables

```typescript
const error = ref<string | null>(null)

async function someAction() {
  try {
    await invoke('some_command', args)
  } catch (e: any) {
    error.value = String(e)
    throw e  // Re-throw for component-level handling
  }
}
```

- `error` ref is displayed in the UI
- Errors are re-thrown so components can show toasts/dialogs

---

## API Error Responses

Tauri automatically serializes `AppError` as a string when the `Result` is `Err`. The frontend receives this as a rejected promise with the error message string.

---

## Common Mistakes

| Mistake | Prevention |
|---------|------------|
| Synchronous Tauri commands calling GStreamer/Win32 APIs | Always use `spawn_blocking` for any blocking operation |
| `MutexGuard` held across `.await` point | Extract needed data before `.await`, release lock first |
| `console.log` left in production code | Search for `console.log` and `console.debug` before committing |
| Ignoring errors with `let _ = ...` | Only acceptable for best-effort cleanup (e.g., `let _ = self.stop_pipeline()`) |
