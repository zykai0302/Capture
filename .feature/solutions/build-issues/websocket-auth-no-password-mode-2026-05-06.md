---
title: "WebSocket Authentication Bypass in No-Password Mode — Auth Message Leaking to Command Parsing"
date: 2026-05-06
category: build-issues
module: src-tauri/src/remote
problem_type: security_issue
component: WebSocket authentication, RemoteCommand deserialization
severity: high
tags: [websocket, authentication, security, no-password-mode, serde, enum-variant, remote-control]
---

## Problem

When a WebSocket server supports a "no-password" mode (empty password = auto-authenticate), two authentication bugs emerge:

1. **Auth message leaking to command parsing**: In no-password mode, clients are auto-authenticated (`authenticated = true`). When they send a legitimate `{"type":"auth","password":"xxx"}` message, the server skips auth logic and passes the message to `serde_json::from_str::<RemoteCommand>()`, which rejects it as `unknown variant 'auth'`.

2. **Wrong password accepted in no-password mode**: Because `authenticated = password.is_empty()` is `true`, any auth message hits the "already authenticated, acknowledge" branch — even with a wrong password. This means a client sending `{"type":"auth","password":"anything"}` gets `{"status":"ok"}` instead of being rejected.

## Symptoms

- Integration test `B2.2 Wrong password rejected` fails with `{'status': 'ok', 'type': 'auth'}` instead of error
- Client auth messages produce `Invalid command: unknown variant 'auth', expected one of 'mouse_move', ...` errors
- Security boundary appears enforced (password required in UI) but is actually bypassed over WebSocket

## What Didn't Work

### Attempt 1: Auth check before command parsing only when `!authenticated`

```rust
if !authenticated {
    // check auth message
}
// Parse RemoteCommand
```

This is the original code. In no-password mode, `authenticated = true` from the start, so the auth check block is never entered. Auth messages fall through to `RemoteCommand` deserialization.

### Attempt 2: Move auth detection before `!authenticated` check

```rust
if let Ok(auth_msg) = serde_json::from_str::<Value>(&text) {
    if auth_msg.get("type") == Some("auth") {
        if authenticated {
            // Already authenticated, just acknowledge  ← BUG: wrong password accepted!
            ws_tx.send(r#"{"status":"ok","type":"auth"}"#).await;
        } else {
            // verify password
        }
        continue;
    }
}
```

This fixes the command parsing leak but introduces a new bug: when `authenticated = true` (no-password mode), ANY auth message gets acknowledged — even with a wrong password.

## Solution

Use a separate `auth_message_received` flag to distinguish between:

- **Auto-authenticated** (no-password mode, never sent an auth message) — must still verify password on first auth message
- **Explicitly authenticated** (client sent a correct auth message) — subsequent auth messages can be acknowledged

```rust
let mut authenticated = password.is_empty(); // No password = auto-auth
let mut auth_message_received = false;        // Track explicit auth

// Always check for auth message first
if let Ok(auth_msg) = serde_json::from_str::<Value>(&text) {
    if auth_msg.get("type").and_then(|v| v.as_str()) == Some("auth") {
        if authenticated && auth_message_received {
            // Already explicitly authenticated, just acknowledge
            ws_tx.send(r#"{"status":"ok","type":"auth"}"#).await;
        } else {
            let provided_password = auth_msg.get("password")
                .and_then(|v| v.as_str()).unwrap_or("");
            if provided_password == password {
                authenticated = true;
                auth_message_received = true;
                ws_tx.send(r#"{"status":"ok","type":"auth"}"#).await;
            } else {
                ws_tx.send(r#"{"status":"error","message":"Authentication failed"}"#).await;
                break;
            }
        }
        continue;
    }
}
```

Key invariants:

| State | `authenticated` | `auth_message_received` | Behavior on auth message |
|-------|----------------|------------------------|--------------------------|
| No-password, first auth | `true` | `false` | Verify password (empty only) |
| No-password, re-auth | `true` | `true` | Acknowledge |
| Password mode, unauthenticated | `false` | `false` | Verify password |
| Password mode, authenticated | `true` | `true` | Acknowledge |

## Why This Works

1. **Auth messages never reach `RemoteCommand` parsing** — The auth detection block runs first and `continue`s, regardless of authentication state.

2. **Wrong password is always rejected on first auth** — Even in no-password mode (`authenticated=true`, `auth_message_received=false`), the server checks `provided_password == password`. Since `password=""`, only `{"type":"auth","password":""}` passes; `{"type":"auth","password":"anything"}` fails.

3. **Re-auth is efficient** — After explicit authentication, subsequent auth messages are acknowledged immediately without re-checking.

4. **Auto-auth for no-password mode still works** — Clients in no-password mode can send commands immediately without auth (since `authenticated=true`), but if they DO send an auth message, it's handled correctly.

## Prevention

### Rule: Never use a single boolean for multi-state auth

When an authentication system has more than two states (unauthenticated / auto-authenticated / explicitly-authenticated), a single `bool` cannot represent all cases. Either:

- Use an enum: `enum AuthState { Unauthenticated, AutoAuth, Authenticated }`
- Use a second flag: `auth_message_received: bool`

### Rule: Always parse auth messages before command deserialization

When a WebSocket server has both an authentication protocol and a command protocol over the same text channel, auth-type messages must be intercepted before attempting command deserialization. Otherwise, auth messages produce `unknown variant` errors that leak protocol internals.

### Rule: Test both password modes explicitly

When a server supports "no password" mode, integration tests must verify:

| Scenario | Expected |
|----------|----------|
| Empty password + empty auth | OK |
| Empty password + non-empty auth | **Rejected** |
| Non-empty password + correct auth | OK |
| Non-empty password + wrong auth | Rejected |
| No auth + command (password mode) | Rejected |
| No auth + command (no-password mode) | OK |

### Rule: Serde enum variants are a security surface

`serde_json::from_str::<Enum>()` with `untagged` or externally-tagged representation will reject any unknown `type` field. If auth messages share the same channel, they must be extracted before serde enum deserialization — or the enum must include an `Auth` variant.

## Related Issues

- `.feature/solutions/testing/screen-control-rtsp-testcase-design-2026-04-30.md` — Test case design methodology (the "unauthenticated command injection" gap)
- `src-tauri/src/remote/websocket.rs` — Fixed authentication implementation
- `src-tauri/src/remote/mod.rs` — `RemoteCommand` enum definition (6 variants, no `auth` variant)
- `tests/test_runtime.py` — Integration test B2.1-B2.5 authentication coverage
