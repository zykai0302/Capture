# 03 — Tauri IPC 命令注册表

## 1 命令清单

### 1.1 `greet`

```rust
#[tauri::command]
fn greet(name: &str) -> String
```

| 方向 | 字段 | 类型 |
|------|------|------|
| 前端调用参数 | `name` | `string` |
| 返回值 | — | `string` |

行为: `format!("Hello, {}! You've been greeted from Rust!", name)`

---

### 1.2 `get_config`

```rust
#[tauri::command]
fn get_config(state: tauri::State<AppState>) -> Result<AppConfig, AppError>
```

| 方向 | 字段 | 类型 |
|------|------|------|
| 返回值 | — | `AppConfig` |

行为: `state.config.lock().unwrap().clone()`

---

### 1.3 `list_sources`

```rust
#[tauri::command]
async fn list_sources() -> Result<CaptureSourceList, AppError>
```

| 方向 | 字段 | 类型 |
|------|------|------|
| 返回值 | — | `CaptureSourceList` |

行为: `spawn_blocking(|| capture::platform::enumerate_sources())`

---

### 1.4 `start_stream`

```rust
#[tauri::command]
async fn start_stream(
    source_id: String,
    source_type: String,
    source_name: String,
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    handle: u64,
    state: tauri::State<'_, AppState>,
) -> Result<(), AppError>
```

| 方向 | 字段 | 类型 | 说明 |
|------|------|------|------|
| 参数 | `sourceId` | `string` | 源 ID (前端 camelCase → Rust snake_case) |
| 参数 | `sourceType` | `string` | "Monitor" / "Window" |
| 参数 | `sourceName` | `string` | 源名称 |
| 参数 | `width` | `number` | 宽度 |
| 参数 | `height` | `number` | 高度 |
| 参数 | `x` | `number` | 显示器/窗口 X 坐标 |
| 参数 | `y` | `number` | 显示器/窗口 Y 坐标 |
| 参数 | `handle` | `number` | HMONITOR (显示器) / HWND (窗口) |
| 返回值 | — | `null` | |

**行为**:
1. 解析 `source_type`: "Monitor" → `SourceType::Monitor`, "Window" → `SourceType::Window`, 其他 → Error
2. 构建 `CaptureSource` 结构体 (is_streaming=false, rtsp_url=None, handle=传入值)
3. 获取 `state.config.lock().unwrap().default_encode.clone()`
4. `spawn_blocking(|| pipeline_manager.start_pipeline(&source, &config))`

> **重要**: `handle` 参数用于 GStreamer 管线的 `monitor-handle`/`window-handle` 属性，避免 `monitor-index` 映射不一致导致显示器反转。

---

### 1.5 `stop_stream`

```rust
#[tauri::command]
async fn stop_stream(source_id: String, state: tauri::State<'_, AppState>) -> Result<(), AppError>
```

| 方向 | 字段 | 类型 |
|------|------|------|
| 参数 | `sourceId` | `string` |
| 返回值 | — | `null` |

行为: `spawn_blocking(|| pipeline_manager.stop_pipeline(&source_id))`

---

### 1.6 `stop_all_streams`

```rust
#[tauri::command]
async fn stop_all_streams(state: tauri::State<'_, AppState>) -> Result<(), AppError>
```

| 方向 | 字段 | 类型 |
|------|------|------|
| 返回值 | — | `null` |

行为: `spawn_blocking(|| pipeline_manager.stop_all())`

---

### 1.7 `get_pipeline_status`

```rust
#[tauri::command]
async fn get_pipeline_status(state: tauri::State<'_, AppState>) -> Result<Vec<PipelineStatus>, AppError>
```

| 方向 | 字段 | 类型 |
|------|------|------|
| 返回值 | — | `PipelineStatus[]` |

行为: `spawn_blocking(|| pipeline_manager.get_all_status())`

---

### 1.8 `update_encode_config`

```rust
#[tauri::command]
async fn update_encode_config(
    source_id: String,
    config: encode::config::EncodeConfig,
    state: tauri::State<'_, AppState>,
) -> Result<(), AppError>
```

| 方向 | 字段 | 类型 |
|------|------|------|
| 参数 | `sourceId` | `string` |
| 参数 | `config` | `EncodeConfig` |
| 返回值 | — | `null` |

行为: `spawn_blocking(|| pipeline_manager.update_config(&source_id, &config))`

---

### 1.9 `get_available_encoders`

```rust
#[tauri::command]
async fn get_available_encoders() -> Result<Vec<String>, AppError>
```

| 方向 | 字段 | 类型 |
|------|------|------|
| 返回值 | — | `string[]` |

行为: `spawn_blocking(|| Ok(gst_pipeline::detect_available_encoders()))`

---

### 1.10 `get_gpu_capabilities`

```rust
#[tauri::command]
async fn get_gpu_capabilities() -> Result<GpuCapability, AppError>
```

| 方向 | 字段 | 类型 |
|------|------|------|
| 返回值 | — | `GpuCapability` |

行为: `spawn_blocking(|| Ok(encode::detector::detect_gpu_capabilities()))`

---

### 1.11 `start_remote_control`

```rust
#[tauri::command]
async fn start_remote_control(
    port: Option<u16>,
    password: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<(), AppError>
```

| 方向 | 字段 | 类型 | 说明 |
|------|------|------|------|
| 参数 | `port` | `number \| null` | 空则用 config.ws_port (9001) |
| 参数 | `password` | `string \| null` | 空则无密码 |
| 返回值 | — | `null` | |

**行为**:
1. `ws_port = port.unwrap_or(state.config.lock().unwrap().ws_port)`
2. `ws_password = password.unwrap_or_default()`
3. `RemoteControlServer::new(ws_port, ws_password, state.remote_injector.clone())`
4. `server.start().await?`
5. `*state.remote_server.lock().unwrap() = Some(server)`

---

### 1.12 `stop_remote_control`

```rust
#[tauri::command]
async fn stop_remote_control(state: tauri::State<'_, AppState>) -> Result<(), AppError>
```

行为: `state.remote_server.lock().unwrap().take(); server.stop().await?`

---

### 1.13 `get_remote_status`

```rust
#[tauri::command]
fn get_remote_status(state: tauri::State<AppState>) -> Result<RemoteStatus, AppError>
```

**行为** (同步命令):
- 若 server 存在: `RemoteStatus { ws_port: s.port, is_running: s.running.load(SeqCst), client_count: s.client_count.load(SeqCst), mouse_enabled: true, keyboard_enabled: true }`
- 若 server 不存在: `RemoteStatus { ws_port: config.ws_port, is_running: false, client_count: 0, mouse_enabled: true, keyboard_enabled: true }`

---

### 1.14 `capture_thumbnail`

```rust
#[tauri::command]
async fn capture_thumbnail(
    source_id: String,
    source_type: String,
    width: u32,
    height: u32,
    handle: u64,
) -> Result<String, AppError>
```

| 方向 | 字段 | 类型 | 说明 |
|------|------|------|------|
| 参数 | `sourceId` | `string` | 源 ID |
| 参数 | `sourceType` | `string` | "Monitor" / "Window" |
| 参数 | `width` | `number` | 缩略图宽度 (默认 320) |
| 参数 | `height` | `number` | 缩略图高度 (默认 180) |
| 参数 | `handle` | `number` | HMONITOR / HWND，优先用于定位显示器 |
| 返回值 | — | `string` | Base64 编码的 JPEG 数据 |

**行为**:
1. `spawn_blocking` 调用 `capture::thumbnail::capture_thumbnail()`
2. Windows Monitor: 优先使用 HMONITOR 定位显示器 → `GetMonitorInfoW` → `BitBlt` 截图
3. Windows Window: 使用 HWND → `PrintWindow` 截图
4. 缩放到指定尺寸，JPEG 编码 (quality=60)
5. Base64 编码返回

> **handle 优先**: 当 `handle != 0` 时直接使用 HMONITOR 定位显示器，避免 `find_monitor_by_index` 的枚举顺序与 `\\.\DISPLAY` 编号不一致问题。

---

### 1.15 `start_preview`

```rust
#[tauri::command]
async fn start_preview(source_id: String, state: tauri::State<'_, AppState>) -> Result<(), AppError>
```

| 方向 | 字段 | 类型 | 说明 |
|------|------|------|------|
| 参数 | `sourceId` | `string` | 源 ID |
| 返回值 | — | `null` | |

**行为**:
1. 从 `pipelines` 获取 source 信息 (source_type, framerate, handle 等)
2. 创建独立预览 Pipeline: `d3d11screencapturesrc → videoconvert → jpegenc → appsink`
3. 启动 MJPEG HTTP 服务器 (端口: config.preview_http_port, 默认 8090)
4. 注册 frame source 到 MJPEG 服务器

> 预览 Pipeline 与 RTSP Pipeline 并行运行，独立启停，零风险不影响推流。

---

### 1.16 `get_preview_url`

```rust
#[tauri::command]
async fn get_preview_url(source_id: String, state: tauri::State<'_, AppState>) -> Result<Option<String>, AppError>
```

| 方向 | 字段 | 类型 | 说明 |
|------|------|------|------|
| 参数 | `sourceId` | `string` | 源 ID |
| 返回值 | — | `string \| null` | MJPEG 预览 URL |

**行为**: 若源正在推流，返回 `http://127.0.0.1:{preview_http_port}/{source_id}`，否则返回 `null`。

---

### 1.17 `stop_preview`

```rust
#[tauri::command]
async fn stop_preview(source_id: String, state: tauri::State<'_, AppState>) -> Result<(), AppError>
```

| 方向 | 字段 | 类型 | 说明 |
|------|------|------|------|
| 参数 | `sourceId` | `string` | 源 ID |
| 返回值 | — | `null` | |

**行为**: 停止预览 Pipeline 并从 MJPEG 服务器移除 frame source。

---

### 1.18 `rtsp_client_connect`

```rust
#[tauri::command]
async fn rtsp_client_connect(
    name: String,
    url: String,
    protocol: String,
    username: Option<String>,
    password: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<String, AppError>
```

|| 方向 | 字段 | 类型 | 说明 |
||------|------|------|------|
|| 参数 | `name` | `string` | 流名称 |
|| 参数 | `url` | `string` | RTSP URL |
|| 参数 | `protocol` | `string` | "tcp" / "udp" |
|| 参数 | `username` | `string \| null` | RTSP 认证用户名 |
|| 参数 | `password` | `string \| null` | RTSP 认证密码 |
|| 返回值 | — | `string` | stream_id (如 "rtsp-client-0af091c4") |

**行为**:
1. `spawn_blocking` 调用 `RtspClientManager::create_pipeline()` 创建 rtspsrc pipeline
2. 注册 MJPEG frame source 到 MJPEG Server
3. 返回生成的 stream_id

---

### 1.19 `rtsp_client_disconnect`

```rust
#[tauri::command]
async fn rtsp_client_disconnect(
    stream_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), AppError>
```

|| 方向 | 字段 | 类型 | 说明 |
||------|------|------|------|
|| 参数 | `streamId` | `string` | 流 ID |
|| 返回值 | — | `null` | |

**行为**: 停止 RTSP 客户端 pipeline 并从 MJPEG Server 移除 frame source。

---

### 1.20 `rtsp_client_status`

```rust
#[tauri::command]
async fn rtsp_client_status(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<RtspClientStatus>, AppError>
```

|| 方向 | 字段 | 类型 |
||------|------|------|
|| 返回值 | — | `RtspClientStatus[]` |

**行为**: 返回所有 RTSP 客户端流的状态（stream_id, name, url, state, resolution, fps, latency_ms, protocol）。

---

### 1.21 `ws_remote_connect`

```rust
#[tauri::command]
async fn ws_remote_connect(
    url: String,
    password: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<(), AppError>
```

|| 方向 | 字段 | 类型 | 说明 |
||------|------|------|------|
|| 参数 | `url` | `string` | WebSocket 服务端 URL (如 "ws://192.168.1.100:9001") |
|| 参数 | `password` | `string \| null` | 认证密码 |
|| 返回值 | — | `null` | |

**行为**:
1. `WsRemoteClient::connect(url, password)` 连接远端 WS 服务
2. 发送 auth 消息，验证认证
3. 启动接收循环（处理 Pong/Close）
4. 存储 `WsRemoteClient` 到 `AppState.ws_remote_client`

---

### 1.22 `ws_remote_disconnect`

```rust
#[tauri::command]
async fn ws_remote_disconnect(
    state: tauri::State<'_, AppState>,
) -> Result<(), AppError>
```

**行为**: 断开 WebSocket 连接，取消重连任务。

---

### 1.23 `ws_remote_send_command`

```rust
#[tauri::command]
async fn ws_remote_send_command(
    command: ClientRemoteCommand,
    state: tauri::State<'_, AppState>,
) -> Result<(), AppError>
```

|| 方向 | 字段 | 类型 | 说明 |
||------|------|------|------|
|| 参数 | `command` | `ClientRemoteCommand` | 客户端命令（相对坐标） |
|| 返回值 | — | `null` | |

**行为**:
1. 从 `remote_resolution` 读取远端分辨率
2. 将 `ClientRemoteCommand`（相对坐标 0.0~1.0）映射为 `RemoteCommand`（绝对坐标）
3. 通过 WebSocket 发送到远端服务

> **前提**: 必须先调用 `ws_remote_set_resolution` 设置远端分辨率，否则返回错误。

---

### 1.24 `ws_remote_status`

```rust
#[tauri::command]
async fn ws_remote_status(
    state: tauri::State<'_, AppState>,
) -> Result<WsClientStatus, AppError>
```

|| 方向 | 字段 | 类型 |
||------|------|------|
|| 返回值 | — | `WsClientStatus` |

**行为**: 返回 WebSocket 客户端连接状态（is_connected, is_reconnecting, reconnect_attempt, max_reconnect_attempts, remote_url）。

---

### 1.25 `ws_remote_set_resolution`

```rust
#[tauri::command]
async fn ws_remote_set_resolution(
    width: u32,
    height: u32,
    state: tauri::State<'_, AppState>,
) -> Result<(), AppError>
```

|| 方向 | 字段 | 类型 | 说明 |
||------|------|------|------|
|| 参数 | `width` | `number` | 远端屏幕宽度 |
|| 参数 | `height` | `number` | 远端屏幕高度 |
|| 返回值 | — | `null` | |

**行为**: 设置 `WsRemoteClient.remote_resolution`，用于 `send_command()` 的坐标映射。前端通过 `watch` 在 RTSP 客户端检测到分辨率时自动调用。

---

## 2 命令注册表

```rust
tauri::generate_handler![
    greet,
    get_config,
    list_sources,
    start_stream,
    stop_stream,
    stop_all_streams,
    get_pipeline_status,
    update_encode_config,
    get_available_encoders,
    get_gpu_capabilities,
    start_remote_control,
    stop_remote_control,
    get_remote_status,
    capture_thumbnail,
    start_preview,
    get_preview_url,
    stop_preview,
    rtsp_client_connect,
    rtsp_client_disconnect,
    rtsp_client_status,
    ws_remote_connect,
    ws_remote_disconnect,
    ws_remote_send_command,
    ws_remote_status,
    ws_remote_set_resolution,
]
```

## 3 Tauri 事件

| 事件名 | 发射时机 | 载荷类型 | 发射位置 |
|--------|---------|---------|---------|
| `source-added` | 热插拔检测到新源 | `CaptureSource` | `hotplug.rs` |
| `source-removed` | 热插拔检测到源移除（已自动停止 Pipeline） | `CaptureSource` | `hotplug.rs` |

**前端监听方式**:

```typescript
import { listen } from '@tauri-apps/api/event'

const unlisten = await listen('source-added', (event) => {
  // event.payload: CaptureSource
})
```

## 4 前端 IPC 调用参数映射

Tauri IPC 使用 **camelCase** 参数名（前端）映射到 **snake_case**（Rust）：

| Rust 参数名 | 前端调用参数名 |
|-------------|---------------|
| `source_id` | `sourceId` |
| `source_type` | `sourceType` |
| `source_name` | `sourceName` |

其他参数 (`width`, `height`, `x`, `y`, `handle`, `port`, `password`, `config`, `url`, `protocol`, `username`) 名称一致，无需映射。
