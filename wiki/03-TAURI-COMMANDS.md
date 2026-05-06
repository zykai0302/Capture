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
| 返回值 | — | `null` | |

**行为**:
1. 解析 `source_type`: "Monitor" → `SourceType::Monitor`, "Window" → `SourceType::Window`, 其他 → Error
2. 构建 `CaptureSource` 结构体 (is_streaming=false, rtsp_url=None)
3. 获取 `state.config.lock().unwrap().default_encode.clone()`
4. `spawn_blocking(|| pipeline_manager.start_pipeline(&source, &config))`

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

其他参数 (`width`, `height`, `port`, `password`, `config`) 名称一致，无需映射。
