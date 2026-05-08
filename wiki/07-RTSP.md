# 07 — RTSP 模块

## 1 模块结构

```
rtsp/
├── mod.rs      → pub mod server; pub mod client; pub use server::RtspServer; pub use client::RtspClientManager;
├── server.rs   → RtspServer (内含独立 GLib MainContext + MainLoop 线程)
└── client.rs   → RtspClientManager (RTSP 拉流客户端 + MJPEG 转发)
```

## 2 GLib MainContext 管理

gst-rtsp-server 要求 RTSP Server、其 socket watches、以及 MainLoop 必须运行在**同一个** GLib MainContext 上。本模块在每个 `RtspServer` 实例中创建专属的 MainContext，并在独立线程中通过 `with_thread_default` 运行。

### 2.1 线程模型

```rust
// 在 RtspServer::new() 中:
std::thread::Builder::new()
    .name("rtsp-server".into())
    .spawn(move || {
        let context = MainContext::new();
        let ctx = context.clone();

        let result = context.with_thread_default(move || {
            let main_loop = MainLoop::new(Some(&ctx), false);
            let server = RTSPServer::new();
            server.set_service(&port_str);

            // 显式传递 Some(&ctx) 确保 server 和 MainLoop 使用同一 context
            server.attach(Some(&ctx))?;

            // 通过 timeout source 轮询 channel 指令
            let timeout_source = timeout_source_new(Duration::from_millis(100), None, Priority::DEFAULT, move || {
                // 处理 add_stream / remove_stream channel 消息
                ControlFlow::Continue
            });
            timeout_source.attach(Some(&ctx));

            let _ = ready_tx.send(Ok(bound_port as u16));
            main_loop.run();
        });
    });
```

**关键点**:
- 每个 RtspServer 实例拥有独立的 `MainContext`，不依赖全局默认 context
- 使用 `context.with_thread_default(...)` 将 context 设为线程默认（glib 0.20 API）
- `MainLoop::new(Some(&ctx))` 和 `server.attach(Some(&ctx))` 显式传递同一 context，确保事件在同一线程中处理
- 通过 `mpsc::channel` + GLib timeout source 实现主线程 → RTSP 线程的异步指令传递
- `ready_tx/ready_rx` channel 用于同步等待服务器启动完成并获取实际绑定端口

### 2.2 为什么不使用全局默认 MainContext？

在 Tauri 应用中，主线程运行自己的事件循环，全局默认 MainContext 可能已被其他库占用。如果 RTSP server attach 到全局默认 context，而 MainLoop 在不同 context 上运行，会导致 RTSP 请求无法被处理。

**之前的方案**（已废弃）：`ensure_glib_main_loop()` 在全局默认 context 上启动独立线程运行 MainLoop → 存在 context 不匹配风险。

**当前方案**：每个 RtspServer 创建专属 `MainContext::new()`，server/loop/source 全部显式绑定到该 context → 无 context 冲突。

---

## 3 RtspServer

### 3.1 数据结构

```rust
pub struct RtspServer {
    port: u16,
    add_stream_tx: mpsc::Sender<(String, String)>,
    remove_stream_tx: mpsc::Sender<String>,
}
```

> 注意：`RTSPServer` 和 `RTSPMountPoints` 对象不再作为结构体字段保存，它们的生命周期由 RTSP 线程的 MainLoop 管理。主线程通过 channel 发送指令。

### 3.2 `RtspServer::new(port: u16) -> AppResult<Self>`

```
1. 创建 add_stream_tx/rx, remove_stream_tx/rx, ready_tx/rx channels
2. spawn "rtsp-server" 线程:
   a. MainContext::new() → ctx
   b. context.with_thread_default:
      - MainLoop::new(Some(&ctx))
      - RTSPServer::new() + set_service(port)
      - mount_points = server.mount_points()
      - server.attach(Some(&ctx))
      - 注册 timeout source (100ms) 轮询 channel
      - ready_tx.send(bound_port)
      - main_loop.run()
3. ready_rx.recv() → 等待服务器启动，获取实际绑定端口
4. Ok(Self { port: bound_port, add_stream_tx, remove_stream_tx })
```

### 3.3 `add_stream(&self, path: &str, pipeline_launch_str: &str) -> AppResult<()>`

```rust
1. add_stream_tx.send((path, pipeline_launch_str))
2. sleep(100ms)  // 等待 RTSP 线程处理 channel 消息
```

> RTSP 线程的 timeout source 回调中会创建 factory 并挂载：
> ```rust
> let factory = RTSPMediaFactory::new();
> factory.set_launch(&launch_str);
> factory.set_shared(true);
> factory.set_latency(0);
> mount_points.add_factory(&path, factory);
> ```

### 3.4 `remove_stream(&self, path: &str)`

```rust
1. remove_stream_tx.send(path)
2. sleep(100ms)
```

### 3.5 `start(&self) -> AppResult<()>`

```rust
Ok(())  // 无操作，服务器在 new() 中已启动
```

> RTSP Server 在 `RtspServer::new()` 中即完成 attach 和 MainLoop.run()，无需额外 start 步骤。

### 3.6 `rtsp_url(&self, path: &str) -> String`

```rust
format!("rtsp://127.0.0.1:{}/{}", self.port, path.trim_start_matches('/'))
```

示例:
- `rtsp_url("/screen-0")` → `"rtsp://127.0.0.1:8554/screen-0"`
- `rtsp_url("/window-12345")` → `"rtsp://127.0.0.1:8554/window-12345"`

### 3.7 `port(&self) -> u16` (允许未使用)

```rust
#[allow(dead_code)]
pub fn port(&self) -> u16 { self.port }
```

---

## 4 RTSP 流生命周期

```
首次 start_pipeline
  │
  ├→ ensure_rtsp_server()
  │     ├→ RtspServer::new(8554)       [spawn 线程 + attach + run MainLoop]
  │     └→ (start() 为空操作)           [服务器已在 new 中启动]
  │
  ├→ add_stream("/screen-0", launch_str)
  │     └→ add_stream_tx.send()         [异步传递到 RTSP 线程]
  │           └→ RTSP 线程: mount_points.add_factory("/screen-0", factory)
  │
  └→ 外部客户端可连接 rtsp://127.0.0.1:8554/screen-0

后续 start_pipeline
  │
  ├→ ensure_rtsp_server()  [已存在，跳过]
  └→ add_stream("/screen-1", launch_str)

stop_pipeline
  │
  └→ remove_stream("/screen-0")
       └→ remove_stream_tx.send()       [异步传递到 RTSP 线程]
             └→ RTSP 线程: mount_points.remove_factory("/screen-0")
```

## 5 默认配置

| 参数 | 值 | 来源 |
|------|-----|------|
| RTSP 端口 | `8554` | `AppConfig::default().rtsp_port` |
| 最大客户端数 | `10` | `AppConfig::default().rtsp_max_clients` (声明但未在代码中强制) |
| 共享模式 | `true` | `factory.set_shared(true)` |
| 延迟 | `0` | `factory.set_latency(0)` |
| 绑定地址 | `127.0.0.1` | `rtsp_url()` 方法硬编码 |
| Channel 轮询间隔 | `100ms` | GLib timeout source |

---

## 6 RTSP 客户端 (`client.rs`)

### 6.1 概述

RTSP 客户端模式允许应用作为监控客户端，连接远端 RTSP 流并实时预览。使用 GStreamer `rtspsrc` 拉流，通过 `decodebin` + `jpegenc` + `appsink` 输出 JPEG 帧，复用 MJPEG Server 推送到 HTTP 端点供前端 `<img>` 标签消费。

### 6.2 数据结构

```rust
pub enum RtspClientState {
    Connecting,
    Connected,
    Reconnecting { attempt: u32, max_attempts: u32 },
    Disconnected,
    Error(String),
    Offline,
}

pub struct RtspClientStatus {
    pub stream_id: String,        // "rtsp-client-{hex_timestamp}"
    pub name: String,
    pub url: String,              // RTSP URL
    pub state: RtspClientState,
    pub resolution: Option<(u32, u32)>,
    pub fps: f64,
    pub latency_ms: u32,
    pub protocol: String,         // "tcp" | "udp"
}

struct RtspClientHandle {
    pipeline: gstreamer::Pipeline,
    state: Arc<Mutex<RtspClientState>>,
    name: String,
    url: String,
    protocol: String,
    resolution: Arc<Mutex<Option<(u32, u32)>>>,
    frame_count: Arc<AtomicU64>,
    started_at: Instant,
    bus_watch_id: gstreamer::bus::BusWatchGuard,
}

pub struct RtspClientManager {
    clients: Mutex<HashMap<String, RtspClientHandle>>,
    mjpeg_server: Arc<AsyncMutex<Option<MjpegServer>>>,
    preview_http_port: u16,
}
```

### 6.3 Pipeline 创建 (`create_pipeline`)

使用 `parse::launch()` 创建 pipeline（避免 `ElementFactory::make().build()` panic）：

```rust
let launch_str = format!(
    "rtspsrc name=src protocols={} latency=0 ! \
     decodebin name=decoder ! \
     videoconvert name=vconv ! \
     jpegenc name=jenc quality=60 ! \
     appsink name=sink emit-signals=true max-buffers=1 drop=true",
    protocols_val
);
let pipeline = gstreamer::parse::launch(&launch_str)?;
```

**关键设计**:
- 所有元素显式命名（`name=src`, `name=vconv`），避免自动命名不可靠
- `rtspsrc` 动态属性（`location`, `user-id`, `user-pw`）通过 `set_property()` 在创建后设置
- `decodebin` 的 `pad-added` 信号回调中链接到 `videoconvert`
- 从 caps 中提取分辨率（`width`/`height`）存入 `RtspClientHandle.resolution`

### 6.4 分辨率检测

通过 GStreamer `pad-added` 信号从 caps 中提取远端分辨率：

```rust
decodebin.connect("pad-added", false, move |_args| {
    let pad = args[1].get::<gstreamer::Pad>().unwrap();
    let caps = pad.current_caps().unwrap();
    let s = caps.structure(0).unwrap();
    if let Ok(width) = s.get::<i32>("width") {
        if let Ok(height) = s.get::<i32>("height") {
            *resolution_clone.lock().unwrap() = Some((width as u32, height as u32));
        }
    }
    None
})
```

### 6.5 MJPEG 转发

`create_pipeline()` 返回 `mpsc::Receiver<Vec<u8>>`，调用方在 async 上下文中注册到 MJPEG Server：

```rust
let (stream_id, rx) = tokio::task::spawn_blocking(move || {
    manager.create_pipeline(name, url, protocol, username, password)
}).await??;

manager_for_mjpeg.register_mjpeg(&stream_id, rx).await?;
```

前端通过 `http://127.0.0.1:{port}/{stream_id}` 消费 MJPEG 流。

### 6.6 自动重连

RTSP 流断开后指数退避重连：
- 初始 2s，翻倍至最大 30s
- 3 次失败后标记为 `Offline`
- Bus watch 检测 `ErrorMessage` → 启动重连

### 6.7 默认配置

| 参数 | 值 | 说明 |
|------|-----|------|
| JPEG quality | `60` | `jpegenc quality=60` |
| appsink max-buffers | `1` | 仅保留最新帧 |
| appsink drop | `true` | 丢弃旧帧 |
| rtspsrc latency | `0` | 最小延迟 |
| stream_id 格式 | `rtsp-client-{hex}` | 8 位十六进制时间戳 |
| 重连初始间隔 | `2s` | 指数退避 |
| 最大重连次数 | `3` | 之后标记 Offline |
