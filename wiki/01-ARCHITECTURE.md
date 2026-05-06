# 01 — 系统架构

## 1 分层架构总览

```
┌──────────────────────────────────────────────────────────────────┐
│                         UI Layer (Vue 3)                         │
│  ┌────────┐ ┌──────────┐ ┌───────────┐ ┌────────────────────┐  │
│  │ TopBar │ │SourceList│ │MainPreview│ │   ConfigPanel      │  │
│  └────────┘ └──────────┘ └───────────┘ └────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Composable Layer                                        │   │
│  │  usePipeline │ useSources │ useConfig │ useRemoteControl  │   │
│  └──────────────────────────────────────────────────────────┘   │
│                          │ invoke() / listen()                   │
├──────────────────────────┼───────────────────────────────────────┤
│                    Tauri IPC Bridge                              │
├──────────────────────────┼───────────────────────────────────────┤
│                     Command Layer (lib.rs)                       │
│  greet | get_config | list_sources | start_stream | ...         │
│                          │                                       │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │              AppState (Tauri Managed State)               │  │
│  │  config: Mutex<AppConfig>                                 │  │
│  │  pipeline_manager: Arc<GstPipelineManager>                │  │
│  │  remote_server: Arc<Mutex<Option<RemoteControlServer>>>   │  │
│  │  remote_injector: Arc<RemoteInjector>                     │  │
│  └───────────────────────────────────────────────────────────┘  │
│                          │                                       │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌───────────┐         │
│  │ capture  │ │ encode   │ │ pipeline │ │  remote    │         │
│  │  module  │ │  module  │ │  module  │ │  module    │         │
│  └──────────┘ └──────────┘ └──────────┘ └───────────┘         │
│  ┌──────────┐ ┌──────────┐                                     │
│  │  rtsp    │ │  config  │                                     │
│  │  module  │ │  module  │                                     │
│  └──────────┘ └──────────┘                                     │
│                          │                                       │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                OS / External Libraries                     │  │
│  │  GStreamer │ Win32/CG/X11 API │ Enigo │ tokio-tungstenite    │  │
│  └───────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

## 2 AppState 规格

```rust
pub struct AppState {
    pub config: Mutex<AppConfig>,                        // 应用配置（互斥访问）
    pub pipeline_manager: Arc<GstPipelineManager>,       // Pipeline 管理器（跨线程共享）
    pub remote_server: Arc<Mutex<Option<RemoteControlServer>>>, // WS 服务（可选，互斥）
    pub remote_injector: Arc<RemoteInjector>,            // 输入注入器（跨线程共享）
}
```

**初始化链路** (`lib.rs::run()`):

1. `AppConfig::default()` → 创建默认配置
2. `GstPipelineManager::new(config.clone())` → 创建 Pipeline 管理器
3. `RemoteInjector::new()` → 创建输入注入器（失败则 `process::exit(1)`）
4. `tauri::Builder::default()` → 注册插件、管理状态、注册命令、启动热插拔

## 3 并发模型

### 3.1 Tauri 命令执行

所有 Tauri `#[tauri::command]` 在 Tokio 异步运行时上执行。

**阻塞操作处理**: 涉及 GStreamer / OS API 的操作通过 `tokio::task::spawn_blocking()` 调度到阻塞线程池：

```rust
tokio::task::spawn_blocking(move || {
    pipeline_manager.start_pipeline(&source, &config)
}).await
```

涉及此模式的命令：
- `list_sources` → `capture::platform::enumerate_sources()`
- `start_stream` → `pipeline_manager.start_pipeline()`
- `stop_stream` → `pipeline_manager.stop_pipeline()`
- `stop_all_streams` → `pipeline_manager.stop_all()`
- `get_pipeline_status` → `pipeline_manager.get_all_status()`
- `update_encode_config` → `pipeline_manager.update_config()`
- `get_available_encoders` → `gst_pipeline::detect_available_encoders()`
- `get_gpu_capabilities` → `encode::detector::detect_gpu_capabilities()`

### 3.2 锁策略与死锁预防

**GstPipelineManager 内部锁**:
- `pipelines: Mutex<HashMap<String, PipelineHandle>>` — Pipeline 注册表
- `rtsp_server: Mutex<Option<RtspServer>>` — RTSP 服务实例

**锁顺序规则**: 先释放 `pipelines` 锁，再获取 `rtsp_server` 锁。所有方法通过作用域块 `{}` 控制锁的生命周期：

```rust
fn stop_pipeline(&self, source_id: &str) -> AppResult<()> {
    let rtsp_path = {
        let mut pipelines = self.pipelines.lock().unwrap();  // 获取 pipelines 锁
        match pipelines.remove(source_id) {
            Some(handle) => handle.rtsp_path,
            None => return Ok(()),
        }
    };  // ← pipelines 锁在此释放
    let server_guard = self.rtsp_server.lock().unwrap();     // 然后获取 rtsp_server 锁
    // ...
}
```

**RemoteInjector**: 使用 `Mutex<enigo::Enigo>` 保护，因为 Enigo 不是线程安全的。

**RemoteControlServer**:
- `running: Arc<AtomicBool>` — 无锁原子状态
- `client_count: Arc<AtomicU32>` — 无锁原子计数
- `shutdown_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>` — Tokio Mutex（异步场景）

### 3.3 后台线程

| 线程 | 启动方式 | 职责 | 生命周期 |
|------|---------|------|---------|
| RTSP Server | `std::thread::Builder` (in `RtspServer::new`) | 运行 gst-rtsp-server + 专属 GLib MainContext/MainLoop + channel 轮询 | 进程级，永不退出 |
| Hotplug Monitor | `std::thread::spawn` (in `start_hotplug_monitor`) | 每 2s 轮询源变更 | 进程级，永不退出 |
| WebSocket Accept | `tokio::spawn` (in `RemoteControlServer::start`) | 接受 WS 连接 | 随服务启停 |
| WebSocket Client | `tokio::spawn` (per connection) | 处理单个 WS 客户端 | 随连接断开 |

### 3.4 热插拔线程超时保护

热插拔轮询使用 `mpsc::channel` + `recv_timeout(10s)` 防止 Win32 API 阻塞（UAC 屏幕、安全桌面等场景）：

```rust
fn enumerate_sources_with_timeout() -> Result<CaptureSourceList, String> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = enumerate_sources();
        let _ = tx.send(result);
    });
    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => Err(format!("Enumeration error: {}", e)),
        Err(RecvTimeoutError::Timeout) =>
            Err("Enumeration timed out after 10s (possible Win32 API stall)".into()),
        Err(RecvTimeoutError::Disconnected) =>
            Err("Enumeration thread panicked".into()),
    }
}
```

## 4 模块依赖关系

```
main.rs
  └→ screencast_pro_lib::run()

lib.rs
  ├→ capture::source::CaptureSourceList
  ├→ capture::hotplug::start_hotplug_monitor
  ├→ capture::platform::enumerate_sources
  ├→ config::AppConfig
  ├→ encode::detector::GpuCapability
  ├→ encode::config::EncodeConfig
  ├→ error::AppError
  ├→ pipeline::gst_pipeline
  ├→ pipeline::manager::GstPipelineManager
  ├→ pipeline::{PipelineManager, PipelineStatus}
  ├→ remote::injector::RemoteInjector
  ├→ remote::websocket::RemoteControlServer
  └→ remote::RemoteStatus

pipeline/manager.rs
  ├→ capture::source::CaptureSource
  ├→ config::AppConfig
  ├→ encode::config::{Codec, EncodeConfig, EncodeMode}
  ├→ error::{AppError, AppResult}
  ├→ pipeline::gst_pipeline
  ├→ pipeline::{PipelineManager, PipelineState, PipelineStatus}
  └→ rtsp::RtspServer

remote/websocket.rs
  ├→ error::{AppError, AppResult}
  ├→ remote::injector::RemoteInjector
  └→ remote::RemoteCommand

remote/injector.rs
  ├→ error::{AppError, AppResult}
  └→ remote::{ClickAction, MouseButton, RemoteCommand}

capture/hotplug.rs
  ├→ capture::platform::enumerate_sources
  ├→ capture::source::CaptureSourceList
  ├→ pipeline::manager::GstPipelineManager
  └→ pipeline::PipelineManager

capture/platform/windows.rs
  ├→ capture::source::{CaptureSource, CaptureSourceList, SourceType}
  └→ error::AppResult

capture/platform/macos.rs
  ├→ capture::source::{CaptureSource, CaptureSourceList, SourceType}
  ├→ core_graphics::display::{CGGetOnlineDisplayList, CGDirectDisplayID, ...}
  ├→ core_foundation::base::TCFType
  └→ error::AppResult

capture/platform/linux.rs
  ├→ capture::source::{CaptureSource, CaptureSourceList, SourceType}
  ├→ std::process::Command (xrandr)
  └→ error::AppResult
```

## 5 应用入口 `main.rs` 启动序列

```
1. std::panic::set_hook → 捕获 panic 写入日志
2. env_logger 初始化 → 输出至 %TEMP%\screencast-pro.log
3. 自动检测 GST_PLUGIN_PATH / GST_PLUGIN_SCANNER 环境变量（平台特定路径搜索）
4. 设置 GST_DEBUG 默认值 (rtspserver:2,rtspmedia:2)
5. gstreamer::init() → 失败则 process::exit(1)
6. screencast_pro_lib::run() → 启动 Tauri
```

`lib.rs::run()` 启动序列：

```
1. AppConfig::default()
2. GstPipelineManager::new(config)
3. RemoteInjector::new() → 失败则 process::exit(1)
4. tauri::Builder::default()
   .plugin(tauri_plugin_opener::init())
   .manage(AppState { ... })
   .invoke_handler(generate_handler![13个命令])
   .setup(|app| {
       capture::hotplug::start_hotplug_monitor(app.handle().clone(), pipeline_manager)
   })
   .run(generate_context!())
```
