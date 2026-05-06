# 07 — RTSP 服务模块

## 1 模块结构

```
rtsp/
├── mod.rs      → pub mod server; pub use server::RtspServer;
└── server.rs   → RtspServer (内含独立 GLib MainContext + MainLoop 线程)
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
