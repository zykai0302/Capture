# RTSP Client Implementation Plan

**Goal:** 为 Screencast Pro 添加 RTSP 客户端模式，支持连接远端 RTSP 流实时预览并通过 WebSocket 反控远端设备。

**Context:**
- task directory: `.feature/tasks/05-08-rtsp-client/`
- related PRD: `.feature/tasks/05-08-rtsp-client/prd.md`
- key findings: GStreamer rtspsrc 拉流复用 MJPEG 预览路径；tokio-tungstenite 实现 WS 客户端；前端传相对坐标后端映射；指数退避自动重连；source_id 使用 `rtsp-client-{id}` 前缀共享 MjpegServer

**Architecture:** RTSP 客户端拉流使用 GStreamer `rtspsrc ! decodebin ! jpegenc ! appsink`，appsink 输出 JPEG 帧通过 broadcast channel 推入共享的 MjpegServer，前端用 `<img>` 消费。反控通过 Rust 端 WebSocket 客户端（tokio-tungstenite）连接远端反控服务，前端捕获鼠标/键盘事件经 Tauri invoke 传递，后端做坐标映射后转发。两种模式通过 App.vue Tab 切换。

**Tech Stack:** Rust (GStreamer 0.23, tokio-tungstenite 0.24, serde), TypeScript (Vue 3 Composition API, Tauri v2 invoke)

**Execution Options:**
- with test cases: `/feature:write-testcase` -> `/feature:check-testcase` -> `/feature:executing-plans` or `/feature:subagent-work`
- sequential: `/feature:executing-plans`
- delegated: `/feature:subagent-work`

---

## Task 1: Add `RtspClient` error variant and types to backend

**Why**
- RTSP 客户端需要独立的错误变体以区分服务端 RTSP 错误
- 客户端状态类型（RtspClientState, RtspClientStatus, WsClientStatus）需要定义

**Files**
- Modify: `src-tauri/src/error.rs`
- Modify: `src-tauri/src/remote/mod.rs`

**Steps**
- [ ] 在 `error.rs` 的 `AppError` 枚举中添加 `RtspClient(String)` 变体
- [ ] 在 `error.rs` 的测试模块中添加 `app_error_display_rtsp_client` 测试
- [ ] 在 `remote/mod.rs` 中添加 `WsClientStatus` 结构体：
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct WsClientStatus {
      pub is_connected: bool,
      pub is_reconnecting: bool,
      pub reconnect_attempt: u32,
      pub max_reconnect_attempts: u32,
      pub remote_url: String,
  }
  ```

**Validation**
- Run: `cd src-tauri && cargo test --lib error -- --nocapture`
- Expect: 所有测试通过，包括新的 `app_error_display_rtsp_client`
- Run: `cd src-tauri && cargo test --lib remote -- --nocapture`
- Expect: 所有现有测试仍通过

**Risks / Notes**
- `AppError::RtspClient` 的 `Serialize` 实现已通过 derive 宏自动处理，无需额外代码

---

## Task 2: Implement `rtsp/client.rs` — RTSP client pipeline

**Why**
- 核心后端模块：使用 GStreamer rtspsrc 拉流并输出 JPEG 帧到 MJPEG Server

**Files**
- Create: `src-tauri/src/rtsp/client.rs`
- Modify: `src-tauri/src/rtsp/mod.rs`

**Steps**
- [ ] 创建 `src-tauri/src/rtsp/client.rs`，实现 `RtspClientManager`：

```rust
use crate::error::{AppError, AppResult};
use crate::pipeline::mjpeg_server::MjpegServer;
use gstreamer::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RtspClientState {
    Connecting,
    Connected,
    Reconnecting { attempt: u32, max_attempts: u32 },
    Disconnected,
    Error(String),
    Offline,
}

#[derive(Debug, Clone, Serialize)]
pub struct RtspClientStatus {
    pub stream_id: String,
    pub name: String,
    pub url: String,
    pub state: RtspClientState,
    pub resolution: Option<(u32, u32)>,
    pub fps: f64,
    pub latency_ms: u32,
    pub protocol: String,
}

struct RtspClientHandle {
    pipeline: gstreamer::Pipeline,
    state: RtspClientState,
    name: String,
    url: String,
    protocol: String,
    resolution: Option<(u32, u32)>,
    frame_count: Arc<AtomicU64>,
    started_at: std::time::Instant,
}

pub struct RtspClientManager {
    clients: Mutex<HashMap<String, RtspClientHandle>>,
    mjpeg_server: Arc<AsyncMutex<Option<MjpegServer>>>,
    preview_http_port: u16,
}
```

- [ ] 实现 `RtspClientManager::new()`, `connect()`, `disconnect()`, `get_status()`, `get_all_status()`, `ensure_mjpeg_server()`
- [ ] `connect()` 方法核心逻辑：
  1. `gstreamer::init()` 检查
  2. 创建 pipeline 元素：`rtspsrc name=src ! decodebin ! videoconvert ! jpegenc quality=60 ! appsink name=sink emit-signals=true max-buffers=1 drop=true`
  3. 设置 rtspsrc 属性：`location`, `protocols` (tcp→4/udp→1), `latency`=0, 可选 `user-id`/`user-pw`
  4. 连接 `decodebin` 的 `pad-added` 信号到 rtspsrc 的 dynamic pad
  5. 设置 appsink 回调（复用 preview.rs 的 `setup_appsink` 模式）
  6. 设置 bus watch 处理错误和状态变更
  7. `pipeline.set_state(Playing)`
  8. 将 appsink 的 mpsc::Receiver 转发到 broadcast channel → MjpegServer
  9. source_id 使用 `rtsp-client-{id}` 格式

- [ ] `pad-added` 信号处理中从 caps 提取分辨率存入 handle
- [ ] bus watch 中检测 `EOS` 和 `Error` 消息触发自动重连
- [ ] 自动重连逻辑：指数退避（2s→4s→8s→16s→30s cap），最多 3 次后标记 Offline
- [ ] 修改 `src-tauri/src/rtsp/mod.rs`：
  ```rust
  pub mod server;
  pub mod client;
  pub use server::RtspServer;
  pub use client::RtspClientManager;
  ```

**Validation**
- Run: `cd src-tauri && cargo check`
- Expect: 编译通过无错误
- Run: `cd src-tauri && cargo test --lib rtsp -- --nocapture`
- Expect: 所有测试通过

**Risks / Notes**
- rtspsrc 不支持 `parse::launch()` 中的动态属性设置（如 `user-id`），必须通过代码设置属性
- `decodebin` 的 pad 是动态添加的，必须在 `pad-added` 信号中链接到下游元素
- GStreamer pipeline 操作必须在 `spawn_blocking` 中执行

---

## Task 3: Implement `remote/ws_client.rs` — WebSocket remote control client

**Why**
- 反控客户端：连接远端 WebSocket 反控服务，发送 RemoteCommand

**Files**
- Create: `src-tauri/src/remote/ws_client.rs`
- Modify: `src-tauri/src/remote/mod.rs`

**Steps**
- [ ] 创建 `src-tauri/src/remote/ws_client.rs`，实现 `WsRemoteClient`：

```rust
use crate::error::{AppError, AppResult};
use crate::remote::{RemoteCommand, WsClientStatus};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;

pub struct WsRemoteClient {
    url: String,
    password: String,
    status: Arc<Mutex<WsClientStatus>>,
    sender: Arc<Mutex<Option<futures_util::stream::SplitSink<...>>>>,
    shutdown_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    remote_resolution: Arc<Mutex<Option<(u32, u32)>>>,
}
```

- [ ] 实现 `WsRemoteClient::new()`, `connect()`, `disconnect()`, `send_command()`, `get_status()`
- [ ] `connect()` 方法核心逻辑：
  1. `tokio_tungstenite::connect_async(&url).await` 建立 WebSocket 连接
  2. `split()` 得到 Sink 和 Stream
  3. 如果有密码，发送 `{"type":"auth","password":"xxx"}` 消息
  4. 等待 `{"status":"ok","type":"auth"}` 响应
  5. 启动接收任务（处理服务端响应和断线检测）
  6. 更新 status 为 connected

- [ ] `send_command()` 方法核心逻辑：
  1. 接收前端传来的 `ClientRemoteCommand`（使用相对坐标 f64）
  2. 从 `remote_resolution` 获取远端分辨率
  3. 计算绝对坐标：`abs_x = (rel_x * width as f64).round() as i32`
  4. 构造 `RemoteCommand`（使用绝对坐标）
  5. 序列化为 JSON 通过 WebSocket 发送
  6. 如果分辨率未获取，返回错误提示

- [ ] 定义 `ClientRemoteCommand` 枚举（前端传递的相对坐标版本）：
  ```rust
  #[derive(Debug, Clone, Deserialize)]
  #[serde(tag = "type", rename_all = "snake_case")]
  pub enum ClientRemoteCommand {
      MouseMove { stream_id: String, data: RelativeMouseMoveData },
      MouseClick { stream_id: String, data: RelativeMouseClickData },
      MouseScroll { stream_id: String, data: RelativeMouseScrollData },
      MouseDrag { stream_id: String, data: RelativeMouseDragData },
      KeyPress { stream_id: String, data: KeyPressData },
      KeyCombo { stream_id: String, data: KeyComboData },
  }
  ```

- [ ] 断线检测：接收任务中如果收到 `Close` 或 `Err`，触发自动重连
- [ ] 自动重连逻辑：与 RTSP 客户端相同的指数退避策略
- [ ] 修改 `src-tauri/src/remote/mod.rs`：
  ```rust
  pub mod injector;
  pub mod websocket;
  pub mod ws_client;
  // ... 新增导出
  pub use ws_client::WsRemoteClient;
  ```

**Validation**
- Run: `cd src-tauri && cargo check`
- Expect: 编译通过
- Run: `cd src-tauri && cargo test --lib remote -- --nocapture`
- Expect: 所有测试通过

**Risks / Notes**
- `tokio-tungstenite` 0.24 的 `connect_async()` 返回 `(WebSocketStream<...>, Response)`
- 认证流程必须匹配服务端实现：无密码时也需发送 auth 消息（参见 commit d359dd8 修复）
- `ClientRemoteCommand` 仅用于反序列化前端请求，不用于 WebSocket 传输（WS 传输用 `RemoteCommand`）

---

## Task 4: Wire backend Tauri commands in `lib.rs`

**Why**
- 将 RtspClientManager 和 WsRemoteClient 注册到 AppState 并暴露 Tauri 命令

**Files**
- Modify: `src-tauri/src/lib.rs`

**Steps**
- [ ] 在 `AppState` 中添加字段：
  ```rust
  pub struct AppState {
      pub config: Mutex<AppConfig>,
      pub pipeline_manager: Arc<GstPipelineManager>,
      pub remote_server: Arc<Mutex<Option<RemoteControlServer>>>,
      pub remote_injector: Arc<RemoteInjector>,
      pub rtsp_client_manager: Arc<rtsp::client::RtspClientManager>,
      pub ws_remote_client: Arc<AsyncMutex<Option<remote::ws_client::WsRemoteClient>>>,
  }
  ```

- [ ] 实现 7 个 Tauri 命令：

  1. `rtsp_client_connect(name, url, protocol, username, password, state) -> Result<(), AppError>`
     - 调用 `state.rtsp_client_manager.connect()`
     - GStreamer 操作用 `spawn_blocking` 包裹

  2. `rtsp_client_disconnect(stream_id, state) -> Result<(), AppError>`
     - 调用 `state.rtsp_client_manager.disconnect()`
     - GStreamer 操作用 `spawn_blocking` 包裹

  3. `rtsp_client_status(state) -> Result<Vec<RtspClientStatus>, AppError>`
     - 调用 `state.rtsp_client_manager.get_all_status()`
     - 用 `spawn_blocking` 包裹

  4. `ws_remote_connect(url, password, state) -> Result<(), AppError>`
     - 创建 `WsRemoteClient` 并连接
     - 直接 async（WebSocket 是 tokio 操作）

  5. `ws_remote_disconnect(state) -> Result<(), AppError>`
     - 调用 `client.disconnect().await`

  6. `ws_remote_send_command(command, state) -> Result<(), AppError>`
     - 反序列化 `ClientRemoteCommand`，调用 `client.send_command()`
     - 坐标映射在 `send_command` 内部完成

  7. `ws_remote_status(state) -> Result<WsClientStatus, AppError>`
     - 返回 `client.get_status()`

- [ ] 在 `run()` 函数中：
  - 初始化 `RtspClientManager`：使用 `pipeline_manager.mjpeg_server` 的 Arc 和 `preview_http_port`
  - 在 `invoke_handler!` 中注册 7 个新命令

**Validation**
- Run: `cd src-tauri && cargo check`
- Expect: 编译通过
- Run: `cd src-tauri && cargo build`
- Expect: 构建成功

**Risks / Notes**
- `RtspClientManager` 需要访问 `MjpegServer`，但 `GstPipelineManager` 已持有 `Arc<AsyncMutex<Option<MjpegServer>>>`。让 `RtspClientManager` 持有同一 Arc 的克隆即可共享 MjpegServer
- 前端 invoke 参数名 camelCase 自动映射：`streamId` → `stream_id`

---

## Task 5: Add TypeScript types for RTSP client

**Why**
- 前端类型定义需要与 Rust 后端序列化格式对齐

**Files**
- Modify: `src/types/index.ts`

**Steps**
- [ ] 添加以下类型定义：

```typescript
// RTSP Client Types
export enum RtspClientStateEnum {
  Connecting = 'Connecting',
  Connected = 'Connected',
  Disconnected = 'Disconnected',
  Offline = 'Offline',
}

export interface RtspClientStateReconnecting {
  Reconnecting: { attempt: number; max_attempts: number }
}

export interface RtspClientStateError {
  Error: string
}

export type RtspClientState =
  | RtspClientStateEnum
  | RtspClientStateReconnecting
  | RtspClientStateError

export interface RtspClientStatus {
  stream_id: string
  name: string
  url: string
  state: RtspClientState
  resolution: [number, number] | null
  fps: number
  latency_ms: number
  protocol: string
}

// WebSocket Remote Client Types
export interface WsClientStatus {
  is_connected: boolean
  is_reconnecting: boolean
  reconnect_attempt: number
  max_reconnect_attempts: number
  remote_url: string
}

// Client Remote Commands (relative coordinates)
export interface RelativeMouseMoveData {
  rel_x: number  // 0.0 ~ 1.0
  rel_y: number
}

export interface RelativeMouseClickData {
  rel_x: number
  rel_y: number
  button: MouseButton
  action: ClickAction
}

export interface RelativeMouseScrollData {
  rel_x: number
  rel_y: number
  dx: number
  dy: number
}

export interface RelativeMouseDragData {
  from_rel_x: number
  from_rel_y: number
  to_rel_x: number
  to_rel_y: number
  button: MouseButton
}
```

- [ ] 添加辅助函数：
```typescript
export function isRtspClientConnected(state: RtspClientState): boolean {
  return state === RtspClientStateEnum.Connected
}

export function isRtspClientReconnecting(state: RtspClientState): boolean {
  return typeof state === 'object' && 'Reconnecting' in state
}

export function isRtspClientError(state: RtspClientState): boolean {
  return typeof state === 'object' && 'Error' in state
}

export function getRtspClientStateLabel(state: RtspClientState): string {
  if (typeof state === 'string') {
    switch (state) {
      case RtspClientStateEnum.Connecting: return '连接中'
      case RtspClientStateEnum.Connected: return '已连接'
      case RtspClientStateEnum.Disconnected: return '已断开'
      case RtspClientStateEnum.Offline: return '离线'
      default: return state
    }
  }
  if ('Reconnecting' in state) return `重连中 (${state.Reconnecting.attempt}/${state.Reconnecting.max_attempts})`
  return state.Error
}
```

- [ ] 复用已有的 `MouseButton` 和 `ClickAction` 枚举（需从 Rust 的 `remote/mod.rs` 导出并在 TS 中定义）
  - 在 `types/index.ts` 中添加：
  ```typescript
  export enum MouseButton {
    Left = 'Left',
    Right = 'Right',
    Middle = 'Middle',
  }

  export enum ClickAction {
    Single = 'Single',
    Double = 'Double',
  }
  ```

**Validation**
- Run: `cd "e:\抓屏软件" && npx vue-tsc --noEmit`
- Expect: 类型检查通过

**Risks / Notes**
- Rust `RtspClientState::Reconnecting { attempt, max_attempts }` 序列化为 `{ "Reconnecting": { "attempt": 1, "max_attempts": 3 } }` — TypeScript 必须匹配此嵌套格式
- `RtspClientState::Error(String)` 序列化为 `{ "Error": "message" }` — 与 `PipelineState::Error` 格式一致

---

## Task 6: Implement `useRtspClient` composable

**Why**
- RTSP 客户端状态管理和 Tauri 命令调用封装

**Files**
- Create: `src/composables/useRtspClient.ts`
- Modify: `src/composables/index.ts`

**Steps**
- [ ] 创建 `src/composables/useRtspClient.ts`：

```typescript
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { RtspClientStatus, RtspClientState } from '../types'

export function useRtspClient() {
  const streams = ref<Record<string, RtspClientStatus>>({})
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function connect(name: string, url: string, protocol: string, username?: string, password?: string) { ... }
  async function disconnect(streamId: string) { ... }
  async function refreshStatus() { ... }
  function getStatus(streamId: string): RtspClientStatus | undefined { ... }
  function isConnected(streamId: string): boolean { ... }
  function getPreviewUrl(streamId: string, port: number): string { ... }

  let refreshInterval: ReturnType<typeof setInterval> | null = null
  onMounted(async () => { await refreshStatus(); refreshInterval = setInterval(refreshStatus, 3000) })
  onUnmounted(() => { if (refreshInterval) clearInterval(refreshInterval) })

  return { streams, loading, error, connect, disconnect, refreshStatus, getStatus, isConnected, getPreviewUrl }
}
```

- [ ] `connect()` 调用 `invoke('rtsp_client_connect', { name, url, protocol, username: username ?? null, password: password ?? null })`
- [ ] `disconnect()` 调用 `invoke('rtsp_client_disconnect', { streamId })`
- [ ] `refreshStatus()` 调用 `invoke<RtspClientStatus[]>('rtsp_client_status')`
- [ ] `getPreviewUrl()` 返回 `http://127.0.0.1:{port}/rtsp-client-{streamId}`
- [ ] 修改 `src/composables/index.ts` 添加导出：
  ```typescript
  export { useRtspClient } from './useRtspClient'
  ```

**Validation**
- Run: `cd "e:\抓屏软件" && npx vue-tsc --noEmit`
- Expect: 类型检查通过

**Risks / Notes**
- Tauri invoke 参数 camelCase 映射：`streamId` → `stream_id`

---

## Task 7: Implement `useWsRemote` composable

**Why**
- WebSocket 反控客户端状态管理和命令发送封装

**Files**
- Create: `src/composables/useWsRemote.ts`
- Modify: `src/composables/index.ts`

**Steps**
- [ ] 创建 `src/composables/useWsRemote.ts`：

```typescript
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { WsClientStatus, MouseButton, ClickAction } from '../types'

export function useWsRemote() {
  const status = ref<WsClientStatus>({
    is_connected: false,
    is_reconnecting: false,
    reconnect_attempt: 0,
    max_reconnect_attempts: 3,
    remote_url: '',
  })
  const loading = ref(false)
  const error = ref<string | null>(null)
  const controlEnabled = ref(false)

  async function connect(url: string, password?: string) { ... }
  async function disconnect() { ... }
  async function refreshStatus() { ... }

  // Mouse control methods
  async function sendMouseMove(streamId: string, relX: number, relY: number) { ... }
  async function sendMouseClick(streamId: string, relX: number, relY: number, button: MouseButton, action: ClickAction) { ... }
  async function sendMouseScroll(streamId: string, relX: number, relY: number, dx: number, dy: number) { ... }
  async function sendMouseDrag(streamId: string, fromRelX: number, fromRelY: number, toRelX: number, toRelY: number, button: MouseButton) { ... }

  // Keyboard control methods
  async function sendKeyPress(streamId: string, key: string, modifiers: string[]) { ... }
  async function sendKeyCombo(streamId: string, keys: string[]) { ... }

  let refreshInterval: ReturnType<typeof setInterval> | null = null
  onMounted(async () => { await refreshStatus(); refreshInterval = setInterval(refreshStatus, 3000) })
  onUnmounted(() => { if (refreshInterval) clearInterval(refreshInterval) })

  return { status, loading, error, controlEnabled, connect, disconnect, refreshStatus,
           sendMouseMove, sendMouseClick, sendMouseScroll, sendMouseDrag, sendKeyPress, sendKeyCombo }
}
```

- [ ] 每个 `send*` 方法构造对应的 `ClientRemoteCommand` JSON 并调用 `invoke('ws_remote_send_command', { command })`
- [ ] 修改 `src/composables/index.ts` 添加导出：
  ```typescript
  export { useWsRemote } from './useWsRemote'
  ```

**Validation**
- Run: `cd "e:\抓屏软件" && npx vue-tsc --noEmit`
- Expect: 类型检查通过

**Risks / Notes**
- `sendMouseMove` 等方法的 `relX/relY` 参数范围 0.0~1.0，由前端从鼠标事件计算
- `ws_remote_send_command` 的参数是整个 command 对象，Tauri 会自动序列化

---

## Task 8: Implement `RtspStreamList.vue` component

**Why**
- 左侧面板：RTSP 流列表管理（添加/删除/选择流）

**Files**
- Create: `src/components/RtspClient/RtspStreamList.vue`

**Steps**
- [ ] 创建组件目录 `src/components/RtspClient/`
- [ ] 实现 `RtspStreamList.vue`，功能包括：
  - 添加流对话框（名称、RTSP URL、传输协议 TCP/UDP 选择、可选用户名/密码）
  - 流卡片列表：显示名称、状态徽章（用颜色区分 Connecting/Connected/Reconnecting/Disconnected/Error/Offline）、URL
  - 选中流高亮，点击触发 `selectStream` emit
  - 每条流的操作按钮：断开/删除
  - 空状态提示
- [ ] Props: `selectedStreamId: string | null`
- [ ] Emits: `selectStream(streamId: string)`, `disconnectStream(streamId: string)`, `deleteStream(streamId: string)`
- [ ] Inject `useRtspClient` composable
- [ ] 样式：复用现有 CSS 变量（`--bg-deep`, `--bg-secondary`, `--border`, `--accent-cyan`, `--text-primary` 等），客户端模式使用紫色系 accent（`--accent-purple: #b44dff`）
- [ ] 参考 `design/rtsp-client.html` 中流列表部分的视觉设计

**Validation**
- Run: `cd "e:\抓屏软件" && npx vue-tsc --noEmit`
- Expect: 类型检查通过

**Risks / Notes**
- 添加流对话框中的 RTSP URL 需要基础验证（以 `rtsp://` 开头）
- 传输协议默认 TCP（监控场景更稳定）

---

## Task 9: Implement `RtspPreview.vue` component

**Why**
- 中间区域：视频预览 + HUD + 反控鼠标事件捕获

**Files**
- Create: `src/components/RtspClient/RtspPreview.vue`

**Steps**
- [ ] 实现 `RtspPreview.vue`，功能包括：
  - MJPEG 预览画面（`<img :src="previewUrl">`），与 `MainPreview.vue` 模式一致
  - HUD 叠加层：分辨率、延迟、FPS、反控状态
  - 鼠标事件捕获（反控模式）：
    - `mousemove` → 计算 relX/relY → 调用 `wsRemote.sendMouseMove()`
    - `mousedown/mouseup` → 调用 `sendMouseClick()`
    - `wheel` → 调用 `sendMouseScroll()`
    - `drag` → 调用 `sendMouseDrag()`（记录 mousedown 位置）
  - 十字准星光标（反控激活时）
  - 状态显示：Connecting 动画、Error 提示、Reconnecting 进度
  - 反控模式开关（Toggle）
- [ ] 坐标计算逻辑：
  ```typescript
  const rect = (e.target as HTMLElement).getBoundingClientRect()
  const relX = (e.clientX - rect.left) / rect.width   // 0.0 ~ 1.0
  const relY = (e.clientY - rect.top) / rect.height    // 0.0 ~ 1.0
  ```
- [ ] MJPEG stream 断线后自动重新加载 `<img>` 标签：
  ```typescript
  imgElement.onerror = () => { setTimeout(() => { imgElement.src = previewUrl + '?t=' + Date.now() }, 2000) }
  ```
- [ ] Inject `useRtspClient` 和 `useWsRemote` composable
- [ ] 参考 `design/rtsp-client.html` 中预览区的视觉设计

**Validation**
- Run: `cd "e:\抓屏软件" && npx vue-tsc --noEmit`
- Expect: 类型检查通过

**Risks / Notes**
- 鼠标事件需要节流（mousemove 约 30fps）避免 WS 命令发送过快
- MJPEG `<img>` 的 onerror 处理需要小心避免无限重载循环

---

## Task 10: Implement `VirtualKeyboard.vue` component

**Why**
- 虚拟键盘：QWERTY 布局 + 常用快捷键

**Files**
- Create: `src/components/RtspClient/VirtualKeyboard.vue`

**Steps**
- [ ] 实现 `VirtualKeyboard.vue`，功能包括：
  - QWERTY 键盘布局（3 行字母 + 数字行 + 功能键行）
  - 常用快捷键网格：Ctrl+C, Ctrl+V, Ctrl+Z, Ctrl+A, Alt+F4, Ctrl+Alt+Delete, Win+D, Ctrl+S, Esc, Enter
  - 按键点击触发 `keyPress` emit（单个键）或 `keyCombo` emit（快捷键）
  - 修饰键状态指示（Ctrl/Alt/Shift 按下时高亮）
- [ ] Props: `enabled: boolean`
- [ ] Emits: `keyPress(key: string, modifiers: string[])`, `keyCombo(keys: string[])`
- [ ] 键盘数据结构：
  ```typescript
  const keyboardRows = [
    ['1','2','3','4','5','6','7','8','9','0'],
    ['Q','W','E','R','T','Y','U','I','O','P'],
    ['A','S','D','F','G','H','J','K','L'],
    ['Z','X','C','V','B','N','M'],
  ]
  const shortcuts = [
    { label: 'Ctrl+C', keys: ['Ctrl','C'] },
    { label: 'Ctrl+V', keys: ['Ctrl','V'] },
    // ...
  ]
  ```

**Validation**
- Run: `cd "e:\抓屏软件" && npx vue-tsc --noEmit`
- Expect: 类型检查通过

**Risks / Notes**
- 键名需与 Rust `parse_key()` 函数匹配（如 "Ctrl", "Alt", "Enter", "Escape" 等）

---

## Task 11: Implement `RemoteControlClient.vue` component

**Why**
- 右侧面板：WebSocket 反控配置 + 虚拟键盘

**Files**
- Create: `src/components/RtspClient/RemoteControlClient.vue`

**Steps**
- [ ] 实现 `RemoteControlClient.vue`，功能包括：
  - WebSocket 连接配置：URL 输入框、密码输入框、连接/断开按钮
  - 连接状态指示器（与现有 `RemoteControl.vue` 风格一致）
  - 控制能力网格（鼠标移动/点击/滚动/拖拽、键盘、快捷键）
  - 虚拟键盘组件 `<VirtualKeyboard>`
- [ ] Inject `useWsRemote` composable
- [ ] 事件转发：VirtualKeyboard 的 `keyPress`/`keyCombo` emit → 调用 `wsRemote.sendKeyPress()`/`wsRemote.sendKeyCombo()`
- [ ] 参考 `design/rtsp-client.html` 中右侧面板的视觉设计

**Validation**
- Run: `cd "e:\抓屏软件" && npx vue-tsc --noEmit`
- Expect: 类型检查通过

**Risks / Notes**
- URL 默认值：`ws://{rtsp-host}:9001`（从当前选中流的 RTSP URL 推导）

---

## Task 12: Integrate mode switch in `App.vue`

**Why**
- 在 App.vue 中添加推流服务/RTSP 客户端模式 Tab 切换

**Files**
- Modify: `src/App.vue`

**Steps**
- [ ] 在 App.vue 中添加：
  - `appMode` ref：`'server' | 'client'`
  - TopBar 中添加模式切换 Tab（两个按钮组）
  - 客户端模式的 composable 实例：`useRtspClient()`, `useWsRemote()`
  - `provide('rtspClient', rtspClientStore)` / `provide('wsRemote', wsRemoteStore)`
  - 条件渲染：`v-if="appMode === 'server'"` 显示现有组件，`v-if="appMode === 'client'"` 显示客户端组件
  - 客户端模式布局：`RtspStreamList` | `RtspPreview` | `RemoteControlClient`

- [ ] 客户端模式的事件处理：
  ```typescript
  const selectedStreamId = ref<string | null>(null)
  function onSelectStream(streamId: string) { selectedStreamId.value = streamId }
  async function onDisconnectStream(streamId: string) { await rtspClientStore.disconnect(streamId) }
  async function onDeleteStream(streamId: string) { await rtspClientStore.disconnect(streamId) }
  ```

- [ ] 客户端模式使用独立 grid 布局（三栏），服务端模式保持现有布局
- [ ] 模式切换 Tab 样式：服务端用青色标识，客户端用紫色标识

**Validation**
- Run: `cd "e:\抓屏软件" && npx vue-tsc --noEmit`
- Expect: 类型检查通过
- Run: `cd "e:\抓屏软件" && npm run build`
- Expect: 构建成功

**Risks / Notes**
- 模式切换时不需要停止另一模式的运行状态（用户可能同时推流和监控）
- 客户端模式的 composable 生命周期跟随 App.vue，不会因 Tab 切换而销毁

---

## Task 13: Full build and typecheck validation

**Why**
- 确保前后端完整构建通过

**Files**
- None (validation only)

**Steps**
- [ ] 运行 Rust 后端构建：`cd src-tauri && cargo build`
- [ ] 运行前端类型检查：`cd "e:\抓屏软件" && npx vue-tsc --noEmit`
- [ ] 运行前端构建：`cd "e:\抓屏软件" && npm run build`
- [ ] 运行 Rust 测试：`cd src-tauri && cargo test --lib`
- [ ] 修复任何编译或类型错误

**Validation**
- Run: `cd src-tauri && cargo test --lib`
- Expect: 所有测试通过
- Run: `cd "e:\抓屏软件" && npm run build`
- Expect: 构建成功

**Risks / Notes**
- 此步骤可能暴露跨层类型不匹配的问题

---

## Task 14: Manual integration test

**Why**
- 验证完整功能链路：RTSP 拉流 → MJPEG 预览 → WebSocket 反控

**Files**
- None (manual testing)

**Steps**
- [ ] 启动应用：`cd "e:\抓屏软件" && npm run tauri dev`
- [ ] 切换到"RTSP 客户端"模式 Tab
- [ ] 测试添加 RTSP 流（使用测试 RTSP 地址如 `rtsp://wowzaec2demo.streamlock.net/vod/mp4:BigBuckBunny_115k.mp4`）
- [ ] 验证预览画面显示
- [ ] 测试断开连接
- [ ] 测试带认证的 RTSP 流（如有测试环境）
- [ ] 测试 WebSocket 反控连接
- [ ] 测试鼠标反控操作
- [ ] 测试虚拟键盘和快捷键
- [ ] 测试自动重连（断开远端 RTSP 流或 WS 服务后观察重连行为）
- [ ] 切换回"推流服务"模式 Tab，验证现有功能未受影响

**Validation**
- 验证所有 PRD 验收标准
- 记录任何发现的问题到 progress.md

**Risks / Notes**
- 需要可用的 RTSP 测试流
- 自动重连测试可能需要模拟网络中断
