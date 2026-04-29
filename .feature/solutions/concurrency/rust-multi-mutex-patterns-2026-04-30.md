---
title: "Rust Multi-Mutex Concurrency Patterns in Tauri+GStreamer Apps"
date: 2026-04-30
category: concurrency
module: src-tauri/src/pipeline, src-tauri/src/capture
problem_type: logic_error
component: GstPipelineManager, hotplug monitor
severity: high
tags: [rust, mutex, deadlock, toctou, concurrency, gstreamer, tauri]
---

## Problem

When a Rust Tauri application manages multiple shared resources (e.g., GStreamer pipelines, RTSP server, hotplug monitor) each protected by `std::sync::Mutex`, three categories of concurrency bugs arise that are easy to introduce and hard to debug:

1. **Deadlock from lock ordering** — Two functions acquire the same locks in opposite order.
2. **TOCTOU race** — Check-then-act pattern where a lock is released between the check and the action.
3. **Background thread hang** — A polling thread calls blocking OS APIs (Win32 `EnumDisplayMonitors`, `EnumWindows`) without timeout, causing the entire thread to hang indefinitely.

## Symptoms

### Deadlock (Lock Ordering)

- Application freezes when `stop_pipeline` and `get_status` are called concurrently.
- No panic, no error — just a hang. Only reproducible under specific timing.
- In our case: `stop_pipeline` held `pipelines` lock then tried to acquire `rtsp_server` lock, while `start_pipeline` held `rtsp_server` lock then tried to acquire `pipelines` lock.

### TOCTOU Race

- Two simultaneous `start_pipeline` calls both see `rtsp_server.is_none()`, both try to start the RTSP server — the second one fails with "port already in use".
- Intermittent; only happens when two sources are started at nearly the same time.

### Background Thread Hang

- Hotplug monitor stops emitting events after encountering a UAC prompt or secure desktop.
- No error logged; the thread is silently blocked forever.
- Application appears to work but never detects new source changes.

## What Didn't Work

| Attempt | Why It Failed |
|---------|---------------|
| Releasing lock before re-acquiring in different order | Still creates a window where another thread can acquire both locks in the opposite order |
| Using `try_lock` with retry | Adds complexity, still doesn't guarantee ordering across all call sites |
| Calling `enumerate_sources()` directly in hotplug thread | Win32 APIs can hang on UAC/secure desktop; no way to interrupt |
| `_ => Button::Left` catch-all in button mapping | Silently degrades right/middle click to left click without any warning |

## Solution

### 1. Consistent Lock Ordering + Release-Before-Acquire

**Rule**: Always acquire locks in the same order across all code paths. When a function needs to hold lock A then acquire lock B, ensure no other function holds lock B then acquires lock A.

**Pattern**: Release lock A, extract needed data, then acquire lock B.

```rust
// BEFORE (deadlock risk):
fn stop_pipeline(&self, source_id: &str) -> AppResult<()> {
    let mut pipelines = self.pipelines.lock().unwrap();
    let handle = pipelines.remove(source_id);
    // Still holding `pipelines` lock, now try `rtsp_server` lock
    let server_guard = self.rtsp_server.lock().unwrap(); // DEADLOCK if another path does the opposite
}

// AFTER (safe):
fn stop_pipeline(&self, source_id: &str) -> AppResult<()> {
    let rtsp_path = {
        let mut pipelines = self.pipelines.lock().unwrap();
        match pipelines.remove(source_id) {
            Some(handle) => handle.rtsp_path,
            None => return Ok(()),
        }
    }; // `pipelines` lock released here
    // Now safe to acquire `rtsp_server` lock
    let server_guard = self.rtsp_server.lock().unwrap();
    // ...
}
```

### 2. Hold Lock During Check-Then-Act (TOCTOU Fix)

**Rule**: When the "check" and "act" must be atomic, hold the lock for both.

```rust
// BEFORE (TOCTOU race):
pub fn ensure_rtsp_server(&self) -> AppResult<()> {
    let needs_start = self.rtsp_server.lock().unwrap().is_none(); // lock released
    if needs_start {
        self.start_rtsp_server()?; // re-acquires lock — another thread may have started it
    }
    Ok(())
}

// AFTER (atomic):
pub fn ensure_rtsp_server(&self) -> AppResult<()> {
    let mut guard = self.rtsp_server.lock().unwrap();
    if guard.is_none() {
        let server = RtspServer::new(self.config.rtsp_port)?;
        server.start()?;
        *guard = Some(server);
    }
    Ok(())
}
```

**Caveat**: If the action while holding the lock (e.g., `server.start()`) is slow or may itself acquire locks, this approach could cause contention. In our case `RtspServer::start()` is fast and doesn't acquire other locks, so this is safe.

### 3. Channel-Based Timeout for Blocking OS API Calls

**Rule**: Never call potentially-blocking OS APIs directly in a long-lived polling thread. Use a channel with timeout.

```rust
fn enumerate_sources_with_timeout() -> Result<CaptureSourceList, String> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = enumerate_sources(); // May call EnumDisplayMonitors, EnumWindows
        let _ = tx.send(result);
    });
    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => Err(format!("Enumeration error: {}", e)),
        Err(RecvTimeoutError::Timeout) => {
            Err("Enumeration timed out after 10s (possible Win32 API stall)".into())
        }
        Err(RecvTimeoutError::Disconnected) => {
            Err("Enumeration thread panicked".into())
        }
    }
}
```

The spawned thread may leak if it never returns, but the polling thread stays alive and can retry on the next cycle.

### 4. Exhaustive Pattern Matching Over Catch-Alls

```rust
// BEFORE (silent degradation):
let button = match data.button {
    MouseButton::Left => Button::Left,
    _ => Button::Left, // Right/Middle silently mapped to Left!
};

// AFTER (explicit):
let button = match data.button {
    MouseButton::Left => Button::Left,
    MouseButton::Right => Button::Right,
    MouseButton::Middle => Button::Middle,
};
```

This ensures the compiler will error if a new variant is added to `MouseButton`.

## Why This Works

1. **Lock ordering**: By always releasing lock A before acquiring lock B, we eliminate the possibility of circular wait — one of the four Coffman conditions for deadlock.
2. **Atomic check-then-act**: Holding the mutex across both the check and the action makes them atomic from the perspective of other threads.
3. **Channel timeout**: The `mpsc::recv_timeout` decouples the polling thread's liveness from the OS API's responsiveness. Even if the API hangs forever, the polling thread can log a warning and continue.
4. **Exhaustive matching**: Rust's exhaustive pattern matching turns silent runtime bugs into compile-time errors.

## Prevention

| Scenario | Prevention Rule |
|----------|----------------|
| Multiple `Mutex` fields in one struct | Document lock ordering in a comment on the struct. Always acquire in documented order. |
| Check-then-act on shared state | Always hold the lock for the entire check+act sequence. |
| Background thread calling OS APIs | Always wrap with a channel + `recv_timeout`. Never trust OS API liveness. |
| Enum matching with fallback | Never use `_ =>` for semantically distinct variants. Use explicit arms. |
| `spawn_blocking` for Tauri commands | Any Rust Tauri command that calls GStreamer or Win32 APIs must use `spawn_blocking` to avoid blocking the tokio runtime. |

### Audit Checklist

When reviewing code that uses multiple `Mutex` fields:

- [ ] Is the lock ordering documented and consistent across all call sites?
- [ ] Does any function release and re-acquire the same lock (TOCTOU window)?
- [ ] Do any background threads call blocking APIs without timeout?
- [ ] Are all `match` arms explicit (no `_` catch-all for distinct semantics)?
- [ ] Are all GStreamer/Win32 Tauri commands wrapped in `spawn_blocking`?

## Related Issues

- `src-tauri/src/pipeline/manager.rs` — `stop_pipeline` lock ordering fix, `ensure_rtsp_server` TOCTOU fix, `get_status` nested lock elimination
- `src-tauri/src/capture/hotplug.rs` — channel-based timeout for `enumerate_sources()`
- `src-tauri/src/remote/injector.rs` — exhaustive button matching
- `src-tauri/src/lib.rs` — `get_gpu_capabilities` async + `spawn_blocking`
