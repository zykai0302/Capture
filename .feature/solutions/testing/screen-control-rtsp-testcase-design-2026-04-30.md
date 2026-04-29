---
title: "Screen Control Product Test Case Design for RTSP Streaming Apps"
date: 2026-04-30
category: testing
module: all modules (cross-cutting)
problem_type: best_practice
component: test case design, coverage analysis
severity: low
tags: [testing, testcase, rtsp, screen-control, coverage, e2e, product-type-PK]
---

## Context

Designing test cases for a screen capture + RTSP streaming + remote control desktop application requires systematic coverage of a complex data pipeline: **screen capture → GPU/CPU encoding → RTSP packaging → network streaming → client playback**, plus a separate **WebSocket remote control** channel with mouse/keyboard injection. The product type is "Screen Control" (PK), which has unique test dimensions not found in typical web or mobile apps.

Initial test case generation produced 55 cases with apparent full coverage, but a module-by-module check against PRD requirements revealed 20 critical gaps — including security (unauthenticated command injection), performance (remote control response time), and runtime conflicts (port occupation).

## Guidance

### 1. Identify the Product Type First

Screen control products have mandatory test dimensions beyond standard functional testing:

| Dimension | Why It Matters | Example Gap Found |
|-----------|---------------|-------------------|
| **Multi-client concurrency** | RTSP is inherently multi-consumer | Multiple VLC clients pulling same stream |
| **Input injection completeness** | Mouse has 3 buttons + scroll + drag | Middle-click, right-drag were missing |
| **Authentication enforcement** | Remote control is a security boundary | Unauthenticated commands must be rejected |
| **Resource lifecycle** | GPU encoders, network ports, threads | Port conflict when RTSP/WebSocket ports occupied |
| **Degradation paths** | GPU→CPU fallback, encoder switching | CPU-only mode not tested |
| **Hot-plug reliability** | Sources appear/disappear at runtime | Pipeline must auto-stop on source removal |

### 2. Module-by-Module Coverage Check Method

Instead of checking "did I write tests for feature X", check "did I test ALL INTERACTIONS of feature X":

```
For each PRD requirement:
  1. List all user-facing behaviors (happy path)
  2. List all error/boundary paths
  3. List all interactions with OTHER modules
  4. Check: does an existing test case cover each?
  5. If not → write one
```

**Applied to this project:**

| Module | Initial Cases | Missing Interactions Discovered | Cases Added |
|--------|:------------:|--------------------------------|:-----------:|
| 推流控制 | 6 | RTSP URL copy, URL format validation | +2 |
| RTSP拉流 | 6 | Multi-client same stream | +1 |
| 编码配置 | 6 | Resolution/preset/rate-control change, pre-configure before streaming | +4 |
| 远程反控 | 14 | Middle-click, multi-client connect, unauthenticated rejection | +5 |
| 编码降级 | 3 | CPU-only mode | +1 |
| 异常处理 | 6 | Port conflicts (RTSP + WebSocket) | +2 |
| UI交互 | 0 | Layout, tab switching, pipeline visualization | +3 |
| 性能指标 | 3 | Remote control response time, multi-stream resource usage | +2 |
| **Total** | **55** | | **+20** |

### 3. Priority Distribution Rule

Aim for H:M:L ≈ 3:5:2. The 82-case final distribution was H:23 (28%) / M:42 (51%) / L:17 (21%), which approximates this target.

**H (Must-pass before release):**
- Core pipeline: start/stop stream, VLC playback, GPU→CPU fallback
- Security: unauthenticated command rejection
- Performance: latency <500ms, remote control response <100ms
- Hot-plug: source removal auto-stops pipeline

**M (Should-pass, acceptable with known issues):**
- Parameter changes, multi-client scenarios, port conflicts, UI layout

**L (Nice-to-have, low user impact):**
- GOP boundary, preset changes, multi-client separate control, pipeline visualization

### 4. Test Constraints to Document

Screen control testing has external dependencies that must be documented:

| Constraint | Tool Required | Workaround |
|-----------|---------------|------------|
| RTSP stream playback | VLC / FFplay / OpenCV | Cannot verify without a player |
| WebSocket remote control | Python websocket-client or browser | Script required |
| Performance measurement | Stopwatch or timestamp tool | Manual measurement acceptable |
| Hot-plug (monitor) | External display | Use window source instead |
| GPU degradation | Controllable GPU driver | Disable GPU in Device Manager |

### 5. The 20 Supplemented Test Points

These were the most impactful gaps found during coverage analysis:

| # | Test Point | Priority | Why It Was Missing |
|---|-----------|----------|-------------------|
| 1 | RTSP URL copy to clipboard | M | Assumed "display = accessible" |
| 2 | RTSP URL format validation | M | Assumed format was obvious |
| 3 | Same stream, multiple clients | M | Only tested multiple streams, not multiple clients per stream |
| 4 | Change resolution | M | Only tested bitrate/fps/codec |
| 5 | Change encoding preset | L | Low-impact parameter |
| 6 | Change rate control mode | L | Low-impact parameter |
| 7 | Configure before streaming | M | Only tested runtime changes |
| 8 | Middle mouse click | M | Assumed left/right was sufficient |
| 9 | Multiple WS clients connect | M | Only tested single client |
| 10 | Multiple WS clients control | L | Complex scenario |
| 11 | **Unauthenticated command rejection** | **H** | **Critical security gap** — only tested valid auth flows |
| 12 | CPU-only encoding mode | M | Only tested GPU→CPU fallback |
| 13 | RTSP port conflict | M | Only tested normal startup |
| 14 | WebSocket port conflict | M | Only tested normal startup |
| 15 | Three-column layout display | M | No UI test cases existed |
| 16 | Tab switching | M | No UI test cases existed |
| 17 | Pipeline visualization | L | No UI test cases existed |
| 18 | **Remote control response <100ms** | **H** | **Only measured RTSP latency, not control latency** |
| 19 | Multi-stream resource usage | M | Only measured single-stream |
| 20 | — | — | — |

### 6. Key Insight: Security Testing Is Easy to Miss

The most critical gap was **unauthenticated command rejection** (5.4.3). The initial test cases covered:
- 5.4.1: No-password mode → client connects successfully ✅
- 5.4.2: Password mode → wrong password rejected, correct password accepted ✅

But missed:
- 5.4.3: **Client skips authentication entirely and sends commands directly** ❌

This is the classic "tested the happy paths of auth, but not the bypass" pattern. For any security boundary, always include a "skip the boundary entirely" test case.

## Why This Matters

Without systematic coverage analysis, critical gaps like unauthenticated command injection and missing performance metrics would reach production. The module-by-module interaction check caught these before runtime testing began.

## When to Apply

- Designing test cases for any product with a security boundary (auth, access control)
- Testing a data pipeline product (capture → encode → transmit → consume)
- Testing remote control / screen sharing / desktop-as-a-service products
- When initial test case generation feels "complete" — do the module interaction check anyway

## Examples

### Coverage Check Template

```markdown
| Module | PRD Requirements | Test Cases | Missing Interactions | Cases Added |
|--------|-----------------|------------|---------------------|-------------|
| Module A | R1, R2, R3 | TC1, TC2 | R3×ModuleB interaction | +1 |
| Module B | R4, R5 | TC3 | Error path for R4 | +1 |
```

### Security Boundary Test Pattern

For any feature with authentication/authorization:

1. ✅ Valid credentials → success
2. ✅ Invalid credentials → rejection
3. ✅ **No credentials (skip auth)** → rejection ← easy to forget
4. ✅ Expired/revoked credentials → rejection

## Related

- `.feature/tasks/04-29-screen-capture-rtsp/testcase.md` — 82 test cases
- `.feature/tasks/04-29-screen-capture-rtsp/testcase_analysis.md` — Requirements analysis
- `.feature/tasks/04-29-screen-capture-rtsp/testcase_checkdetail.md` — Coverage verification report
- `.feature/solutions/concurrency/rust-multi-mutex-patterns-2026-04-30.md` — Concurrency bugs found during review that test cases now cover
- `.feature/solutions/build-issues/tauri-gstreamer-windows-build-2026-04-30.md` — Build environment that E2E tests depend on
