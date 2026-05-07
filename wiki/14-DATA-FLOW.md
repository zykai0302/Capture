# 14 — 核心数据流

## 1 推流启动完整时序

```
用户                     前端                          Tauri IPC                    Rust 后端
 │                        │                              │                            │
 │  点击"开始推流"         │                              │                            │
 │───────────────────────→│                              │                            │
 │                        │                              │                            │
 │                        │ SourceItem.emit('startStream')│                            │
 │                        │────→ SourceList               │                            │
 │                        │      usePipeline.startStream()│                            │
 │                        │                              │                            │
│                        │ invoke('start_stream', {      │                            │
│                        │   sourceId: 'screen-0',       │                            │
│                        │   sourceType: 'Monitor',      │                            │
│                        │   sourceName: '显示器 0',      │                            │
│                        │   width: 1920, height: 1080,  │                            │
│                        │   x: 0, y: 0,                 │                            │
│                        │   handle: 65537               │                            │
│                        │ })                            │                            │
 │                        │─────────────────────────────→│                            │
 │                        │                              │ start_stream()             │
 │                        │                              │───────────────────────────→│
 │                        │                              │                            │
 │                        │                              │                 解析 SourceType
 │                        │                              │                 构建 CaptureSource
 │                        │                              │                 获取 default_encode
 │                        │                              │                            │
 │                        │                              │                 spawn_blocking ──→│
 │                        │                              │                            │
│                        │                              │           ┌──── GstPipelineManager ────┐
│                        │                              │           │ 1. ensure_rtsp_server()    │
│                        │                              │           │    ├ RtspServer::new(8554) │
│                        │                              │           │    │  (spawn线程+MainContext│
│                        │                              │           │    │   +attach+MainLoop)    │
│                        │                              │           │    └ start() (空操作)      │
 │                        │                              │           │                            │
 │                        │                              │           │ 2. build_launch_string()   │
 │                        │                              │           │    ├ capture_elem          │
 │                        │                              │           │    ├ encoder_elem          │
 │                        │                              │           │    └ rtp_pay              │
 │                        │                              │           │                            │
 │                        │                              │           │ 3. rtsp_server.add_stream  │
 │                        │                              │           │    ("/screen-0", launch_str)│
 │                        │                              │           │                            │
 │                        │                              │           │ 4. detect_encoder_info()   │
 │                        │                              │           │    → ("amfh264enc", true)  │
 │                        │                              │           │                            │
 │                        │                              │           │ 5. pipelines.insert(       │
 │                        │                              │           │      "screen-0", Handle)   │
 │                        │                              │           └────────────────────────────┘
 │                        │                              │                            │
 │                        │  Result<()>                  │                            │
 │                        │←─────────────────────────────│←───────────────────────────│
 │                        │                              │                            │
 │                        │ refreshStatus()              │                            │
 │                        │ invoke('get_pipeline_status') │                            │
 │                        │─────────────────────────────→│ get_all_status()           │
 │                        │                              │───────────────────────────→│
 │                        │  PipelineStatus[]            │                            │
 │                        │←─────────────────────────────│←───────────────────────────│
 │                        │                              │                            │
 │                        │ 更新 pipelineMap             │                            │
 │                        │ Vue 响应式更新:               │                            │
 │                        │  - SourceItem: LIVE 徽章      │                            │
 │                        │  - MainPreview: HUD 叠加      │                            │
 │                        │  - PipelineVisual: 激活节点   │                            │
 │                        │  - StatusBar: 推流路数+1      │                            │
 │                        │                              │                            │
 │  UI 更新                │                              │                            │
 │←───────────────────────│                              │                            │
```

## 2 停止推流时序

```
usePipeline.stopStream(sourceId)
  │
  ├→ invoke('stop_stream', { sourceId })
  │     └→ spawn_blocking → GstPipelineManager.stop_pipeline()
  │           ├→ pipelines.lock().remove(source_id) → rtsp_path
  │           └→ rtsp_server.lock().remove_stream(&rtsp_path)
  │
  └→ refreshStatus() → 更新 pipelineMap → UI 响应式更新
```

## 3 远程反控命令时序

```
远程客户端                   WebSocket 服务                RemoteInjector             操作系统
    │                           │                            │                        │
    │── TCP 连接 ──────────────→│                            │                        │
    │── WS 握手 ──────────────→│                            │                        │
    │                           │                            │                        │
    │── {"type":"auth",         │                            │                        │
    │    "password":"xxx"} ────→│                            │                        │
    │                           │ 验证密码                    │                        │
    │←── {"status":"ok",        │                            │                        │
    │     "type":"auth"} ───────│                            │                        │
    │                           │                            │                        │
    │── {"type":"mouse_move",   │                            │                        │
    │    "stream_id":"screen-0",│                            │                        │
    │    "data":{"x":500,       │                            │                        │
    │            "y":300}} ────→│                            │                        │
    │                           │ serde_json::from_str       │                        │
    │                           │ → RemoteCommand::MouseMove │                        │
    │                           │                            │                        │
    │                           │ injector.execute(&cmd) ───→│                        │
    │                           │                            │ enigo.move_mouse(500,   │
    │                           │                            │   300, Abs) ──────────→│
    │                           │                            │                        │
    │                           │                        Ok(())                       │
    │←── {"status":"ok"} ──────│←───────────────────────────│                        │
    │                           │                            │                        │
    │── {"type":"key_press",    │                            │                        │
    │    "data":{"key":"Enter", │                            │                        │
    │            "modifiers":   │                            │                        │
    │            ["Ctrl"]}} ───→│                            │                        │
    │                           │ injector.execute(&cmd) ───→│                        │
    │                           │                            │ enigo.key(Ctrl, Press) →│
    │                           │                            │ enigo.key(Enter, Click)→│
    │                           │                            │ enigo.key(Ctrl, Release)│
    │←── {"status":"ok"} ──────│←── Ok(()) ────────────────│                        │
```

## 4 热插拔事件时序

```
Hotplug Monitor 线程                    Tauri AppHandle                  前端
       │                                     │                            │
       │ sleep(2s)                            │                            │
       │ enumerate_sources_with_timeout()     │                            │
       │   ├→ spawn thread                    │                            │
       │   │  └→ enumerate_sources()          │                            │
       │   └→ recv_timeout(10s)               │                            │
       │                                      │                            │
       │ 比较current_sources vs last_sources  │                            │
       │                                      │                            │
       │ ┌─ 检测到新源 ─────────────────────┐ │                            │
       │ │  app.emit("source-added", source) │─→│ listen('source-added')  │
       │ └──────────────────────────────────┘ │  └→ useSources.refresh() │
       │                                      │                            │
       │ ┌─ 检测到源移除 ───────────────────┐ │                            │
       │ │  pipeline_manager                │ │                            │
       │ │    .stop_pipeline(&source.id)    │ │                            │
       │ │  app.emit("source-removed",source)│─→│ listen('source-removed')│
       │ └──────────────────────────────────┘ │  └→ useSources.refresh() │
       │                                      │                            │
       │ last_sources = current_sources       │                            │
       │ → 继续循环                            │                            │
```

## 5 编码配置更新时序

```
EncodingConfig.applyConfig()
  │
  ├→ invoke('update_encode_config', {
  │     sourceId: 'screen-0',
  │     config: { codec: 'H265', mode: 'CpuOnly', ... }
  │   })
  │
  └→ GstPipelineManager.update_config()
       ├→ pipelines.get("screen-0").source.clone()  // 获取现有源
       ├→ stop_pipeline("screen-0")
       │     ├→ pipelines.remove("screen-0")
       │     └→ rtsp_server.remove_stream("/screen-0")
       └→ start_pipeline(&source, &new_config)
             ├→ ensure_rtsp_server()  // 复用现有 RTSP Server
             ├→ build_launch_string(source, new_config)  // 新 launch string
             ├→ rtsp_server.add_stream("/screen-0", new_launch_str)
             └→ pipelines.insert("screen-0", new_handle)
```

> **注意**: 配置更新 = 停止 + 重启，会导致短暂的服务中断。

## 6 前端轮询周期

| Composable | 轮询间隔 | 轮询内容 |
|-----------|---------|---------|
| `usePipeline` | 3 秒 | `invoke('get_pipeline_status')` |
| `useSources` | 5 秒 | `invoke('list_sources')` |
| `useRemoteControl` | 5 秒 | `invoke('get_remote_status')` |

加上 Tauri 事件驱动的即时更新：
- `source-added` / `source-removed` 事件 → 立即触发 `useSources.refresh()` + `fetchThumbnails()`

## 7 预览启动时序

```
用户选择源 + 推流中                前端                          Tauri IPC                    Rust 后端
       │                            │                              │                            │
       │  MainPreview watch(isRunning=true)                        │                            │
       │                            │                              │                            │
       │                            │ invoke('start_preview', {     │                            │
       │                            │   sourceId: 'screen-0'        │                            │
       │                            │ })                            │                            │
       │                            │─────────────────────────────→│                            │
       │                            │                              │ start_preview()            │
       │                            │                              │───────────────────────────→│
       │                            │                              │                            │
       │                            │                              │                 从 pipelines 获取 source 信息
       │                            │                              │                 (source_type, framerate, handle)
       │                            │                              │                            │
       │                            │                              │                 创建 PreviewPipeline ──→│
       │                            │                              │                   ├ d3d11screencapturesrc
       │                            │                              │                   │  monitor-handle={handle}
       │                            │                              │                   ├ videoconvert
       │                            │                              │                   ├ capsfilter(framerate=30)
       │                            │                              │                   ├ jpegenc(quality=60)
       │                            │                              │                   └ appsink → mpsc channel
       │                            │                              │                            │
       │                            │                              │                 创建 broadcast channel
       │                            │                              │                 spawn_blocking: mpsc→broadcast 转发
       │                            │                              │                            │
       │                            │                              │                 ensure_mjpeg_server()
       │                            │                              │                   ├ MjpegServer::new(8090)
       │                            │                              │                   └ TcpListener::bind
       │                            │                              │                            │
       │                            │                              │                 mjpeg_server.add_source(
       │                            │                              │                   "screen-0", btx)
       │                            │                              │                            │
       │                            │  Result<()>                  │                            │
       │                            │←─────────────────────────────│←───────────────────────────│
       │                            │                              │                            │
       │                            │ previewUrl =                 │                            │
       │                            │   "http://127.0.0.1:8090/    │                            │
       │                            │    screen-0"                  │                            │
       │                            │                              │                            │
       │  <img :src="previewUrl">   │                              │                            │
       │  浏览器请求 MJPEG 流        │                              │                            │
       │────────────────────────────────────────────────────────→│                            │
       │                            │                              │ MjpegServer 处理连接       │
       │                            │                              │  ├ subscribe broadcast     │
       │                            │                              │  └ 循环发送 JPEG 帧        │
       │  MJPEG 实时画面显示         │                              │                            │
       │←───────────────────────────│                              │                            │
```

## 8 Handle 传递链路

`handle` (HMONITOR/HWND) 是确保显示器/窗口正确识别的关键数据，需贯穿完整链路：

```
Windows API                    Rust 后端                     Tauri IPC                  前端
──────────                     ─────────                     ─────────                  ─────
EnumDisplayMonitors            enumerate_monitors()
  → HMONITOR                     CaptureSource {
    handle: 65537                  handle: hmonitor.0 as u64
                                  id: "screen-0"
                                  x: rect.left, y: rect.top
                                }
                                    │
                                    ├→ list_sources ─────────→ source.handle ────────→ CaptureSource.handle
                                    │                                                      │
                                    │                    ┌─────────────────────────────┘
                                    │                    │
                                    │           invoke('start_stream', { handle })
                                    │                    │
                                    ├→ start_stream ─────┤→ CaptureSource { handle }
                                    │                    │   │
                                    │                    │   └→ build_launch_string()
                                    │                    │        d3d12screencapturesrc
                                    │                    │          monitor-handle={handle}
                                    │                    │
                                    │           invoke('capture_thumbnail', { handle })
                                    │                    │
                                    └→ capture_thumbnail ┤→ capture_monitor_thumbnail(handle)
                                                         │   │
                                                         │   ├ handle != 0: HMONITOR(handle)
                                                         │   │   → GetMonitorInfoW → BitBlt
                                                         │   │
                                                         │   └ handle == 0: find_monitor_by_index
                                                         │       (回退方案，可能不准确)
```

> **⚠️ 关键教训**: 新增 `CaptureSource.handle` 字段后，必须审计所有构造/消费该结构体的 Tauri 命令和前端 invoke 调用，确保字段不被遗漏。详见 `.feature/solutions/integration-issues/cross-layer-field-omission-tauri-commands-2026-05-08.md`。
