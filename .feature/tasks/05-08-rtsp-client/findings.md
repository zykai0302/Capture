# Findings & Decisions

## Requirements
- RTSP 客户端模式：添加/管理 RTSP 流连接（名称、URL、传输协议 TCP/UDP、可选用户名/密码）
- RTSP 流实时预览（复用 MJPEG 预览架构）
- WebSocket 反控客户端（连接远端反控服务，发送鼠标/键盘指令）
- 自动重连：RTSP 流和 WebSocket 断线后指数退避重连（2s→4s→8s→...→30s，3 次失败后标记离线）
- 坐标映射：前端传相对坐标（0.0~1.0），后端根据远端分辨率计算绝对坐标
- 虚拟键盘（QWERTY 布局 + 常用快捷键）
- 客户端模式与服务端模式通过 Tab 切换

## Research Summary

### Relevant Specs
- `.feature/spec/frontend/index.md`: Vue 3 Composition API, TypeScript strict, CSS 变量体系
- `.feature/spec/backend/index.md`: Tauri v2 commands, GStreamer pipeline, tokio async runtime

### Existing Patterns

#### Backend Patterns
1. **GStreamer Pipeline 创建模式** (`pipeline/preview.rs`):
   - `gstreamer::init()` → `parse::launch()` → `downcast::<Pipeline>()` → `setup_appsink()` → `play()`
   - appsink 使用 `SyncSender<Vec<u8>>` 输出 JPEG 帧
   - Bus watch 用于错误日志和状态变更通知
   - 关键参考：`PreviewPipeline` 的创建/播放/停止生命周期

2. **MJPEG 预览路径** (`pipeline/manager.rs`):
   - `PreviewPipeline` → `mpsc::Receiver<Vec<u8>>` → `broadcast::Sender<Vec<u8>>` → `MjpegServer::add_source()`
   - 前端通过 `http://127.0.0.1:{port}/{source_id}` 的 `<img>` 标签消费
   - `start_preview()` → `ensure_mjpeg_server()` → register broadcast → spawn_blocking forward task
   - **RTSP 客户端应复用此路径**：rtspsrc pipeline 的 appsink 输出同样推到 MJPEG Server

3. **WebSocket 服务端模式** (`remote/websocket.rs`):
   - `tokio_tungstenite::accept_async()` → `split()` → `ws_rx.next().await` 循环
   - 认证流程：`{"type":"auth","password":"xxx"}` → `{"status":"ok","type":"auth"}`
   - 客户端发送 `RemoteCommand` JSON，服务端解析执行
   - **客户端模式需要反向实现**：`tokio_tungstenite::connect_async()` → 发送 auth → 发送 RemoteCommand

4. **RemoteCommand 协议** (`remote/mod.rs`):
   - `#[serde(tag = "type", rename_all = "snake_case")]` 格式
   - 6 种命令：MouseMove/MouseClick/MouseScroll/MouseDrag/KeyPress/KeyCombo
   - 所有命令含 `stream_id: String` 字段
   - 坐标使用绝对像素值 `i32`（后端负责映射）

5. **Tauri Command 模式** (`lib.rs`):
   - `#[tauri::command]` + `State<AppState>` 注入
   - GStreamer 操作用 `spawn_blocking` 包裹（避免阻塞 tokio runtime）
   - Async 操作（WebSocket, MJPEG）直接在 tokio runtime 执行
   - `invoke_handler!` 宏注册所有命令

6. **AppError 枚举** (`error.rs`):
   - 已有变体：GStreamer/Pipeline/Capture/Encode/Rtsp/Remote/Preview/Config/Io/Windows
   - 实现 `serde::Serialize`（序列化为字符串）
   - **需新增**：`RtspClient` 变体或复用 `Rtsp`/`Remote`

7. **AppConfig** (`config/mod.rs`):
   - 包含 `rtsp_port`, `ws_port`, `preview_http_port` 等配置
   - **需扩展**：客户端模式配置（默认 WS 远端端口等）

#### Frontend Patterns
1. **Composable 模式** (`composables/usePipeline.ts`):
   - `ref<Record<string, T>>` 作为响应式数据存储
   - `invoke<T>(cmd, args)` 调用 Tauri 命令
   - `onMounted` 中初始化 + `setInterval` 轮询刷新
   - `onUnmounted` 清理定时器
   - 导出 `status/loading/error` + 操作方法

2. **组件数据流** (`App.vue`):
   - App 创建 composable 实例 → `provide()` 注入 → 子组件 `inject()` 消费
   - 事件流：子组件 emit → App 处理 → composable 方法调用
   - 网格布局：`grid-template-areas: "topbar topbar topbar" "left center right" "status status status"`

3. **MJPEG 预览消费** (`MainPreview.vue`):
   - `invoke('start_preview', { sourceId })` → 获取 preview URL → `<img :src="previewUrl">`
   - HUD 叠加层：分辨率/编码器、延迟、FPS

4. **类型定义** (`types/index.ts`):
   - Rust 枚举用 TypeScript enum 镜像
   - Rust struct 用 interface 镜像
   - 辅助函数：`isPipelineRunning()`, `getPipelineStateLabel()` 等

### Candidate Files To Modify

#### New Files
| Path | Purpose |
|------|---------|
| `src-tauri/src/rtsp/client.rs` | RTSP 客户端 pipeline 管理（rtspsrc + decodebin + jpegenc + appsink） |
| `src-tauri/src/remote/ws_client.rs` | WebSocket 反控客户端（tokio-tungstenite connect_async + auth + send command） |
| `src/components/RtspClient/RtspStreamList.vue` | 流列表管理组件 |
| `src/components/RtspClient/RtspPreview.vue` | 视频预览 + HUD + 反控事件捕获 |
| `src/components/RtspClient/RemoteControlClient.vue` | 反控面板 + 虚拟键盘 |
| `src/components/RtspClient/VirtualKeyboard.vue` | 虚拟键盘组件 |
| `src/composables/useRtspClient.ts` | RTSP 客户端 composable |
| `src/composables/useWsRemote.ts` | WebSocket 反控客户端 composable |

#### Modified Files
| Path | Change |
|-------|--------|
| `src-tauri/src/rtsp/mod.rs` | 添加 `pub mod client;` + `pub use client::RtspClient;` |
| `src-tauri/src/remote/mod.rs` | 添加 `pub mod ws_client;` + 新增客户端状态类型 |
| `src-tauri/src/lib.rs` | 新增 AppState 字段 + 7 个 Tauri 命令 + invoke_handler 注册 |
| `src-tauri/src/error.rs` | 可能添加 `RtspClient` 变体（或复用 `Rtsp`） |
| `src-tauri/src/config/mod.rs` | 可能添加客户端相关默认配置 |
| `src/App.vue` | 添加模式切换 Tab（推流服务 / RTSP 客户端） |
| `src/types/index.ts` | 添加 RTSP 客户端相关类型 |
| `src/composables/index.ts` | 导出新 composable |

### Constraints

1. **GStreamer 线程模型**：GStreamer pipeline 必须在 `spawn_blocking` 中创建，因为 GStreamer 使用 GLib main context 且 pipeline 操作可能阻塞
2. **MJPEG Server 端口冲突**：客户端模式和服务端模式共享同一个 MJPEG Server 实例（端口 8090），source_id 需要命名空间区分
3. **RemoteCommand 协议兼容**：客户端发送的 RemoteCommand JSON 必须与服务端接收格式完全一致（`#[serde(tag = "type")]`）
4. **坐标映射实现**：前端传 `rel_x: f64, rel_y: f64` (0.0~1.0)，后端需从 SDP 获取远端分辨率后计算绝对坐标
5. **Windows 平台**：GStreamer rtspsrc 在 Windows 上需要正确处理 `protocols` 属性（tcp/udp）
6. **tokio-tungstenite 版本**：当前 0.24，`connect_async()` API 签名需确认
7. **Tauri v2 命令**：async 命令自动在 tokio runtime 执行；同步操作需 `spawn_blocking`

### Risks

1. **rtspsrc + decodebin 协商延迟**：首次连接 RTSP 流时，SDP 协商和 codec 初始化可能耗时数秒，需在 UI 中展示"连接中"状态
2. **SDP 分辨率获取**：从 GStreamer pipeline 中获取远端分辨率需要在 `pad-added` 信号回调中读取 caps，实现复杂度较高。备选方案：让用户手动输入远端分辨率，或在首次收到帧时从 JPEG 解析
3. **MJPEG Server 共享**：如果服务端模式和客户端模式同时使用 MJPEG Server，source_id 需要避免冲突
4. **WebSocket 重连风暴**：多个 RTSP 客户端同时重连时可能造成资源消耗，需要限制并发重连数
5. **浏览器 img 标签重连**：MJPEG stream 断开后 `<img>` 标签不会自动重连，需要在前端处理重新加载
6. **坐标映射精度**：相对坐标 0.0~1.0 转绝对坐标时，浮点精度可能导致 1px 偏差，需使用 `round()` 或 `floor()`

### Open Questions

1. **SDP 分辨率获取方案**：是使用 GStreamer `pad-added` 信号回调从 caps 中提取分辨率，还是让用户手动输入？前者更自动化但实现复杂，后者简单但不够用户友好。**决定**：使用 pad-added 信号回调方案，在 Task 2 中实现
2. **客户端模式是否需要独立的 MJPEG Server**：还是与服务端共享？共享更简单但需要 source_id 命名空间；独立更隔离但消耗更多端口。**决定**：共享，使用 `rtsp-client-{id}` 前缀区分
3. **App.vue 模式切换方案**：是 Tab 切换还是路由切换？Tab 更简单但代码耦合；路由更清晰但需引入 vue-router。**决定**：Tab 切换（保持简单，不引入新依赖）

### Planning Constraints (discovered during plan writing)

1. **rtspsrc 不支持 parse::launch 动态属性**：rtspsrc 的 `user-id`/`user-pw`/`protocols` 属性必须通过代码设置（`element.set_property()`），不能在 launch string 中指定。因此需要用 `ElementFactory::make()` 创建各元素再手动链接
2. **decodebin 动态 pad**：decodebin 的 src pad 是动态添加的，必须在 `pad-added` 信号回调中链接到 videoconvert，不能在 launch string 中静态链接
3. **RtspClientManager 共享 MjpegServer**：通过克隆 `GstPipelineManager` 持有的 `Arc<AsyncMutex<Option<MjpegServer>>>` 实现，不需要新的端口
4. **ClientRemoteCommand 与 RemoteCommand 分离**：前端传递 `ClientRemoteCommand`（相对坐标 f64），后端映射为 `RemoteCommand`（绝对坐标 i32）后通过 WebSocket 发送。两个枚举独立定义，仅在后端 `send_command` 方法中做转换

### Prior Learnings
- 项目使用 GStreamer 0.23，已有 `gstreamer-rtsp` 和 `gstreamer-sdp` 依赖，可以直接使用 SDP 解析
- 已有 `tokio-tungstenite` 0.24 依赖，`futures-util` 0.3 用于 Stream/Sink 操作
- 现有 WebSocket 服务端的认证流程（auth message → ok/error）可直接被客户端复用
- `RemoteInjector` 使用 `enigo` 库注入，客户端模式不需要注入，只需要发送命令
- GStreamer `rtspsrc` 支持 `user-id`、`user-pw`、`protocols` 属性设置认证和传输协议

### Historical Context
- `d359dd8`: 修复了 WebSocket 认证绕过问题（无密码模式下也必须显式认证），客户端实现需注意此行为
- `e9f30e9`: 最近一次重大功能更新，添加了 MJPEG 预览和缩略图功能，是 RTSP 客户端预览路径的直接参考

### Framework / Dependency Notes
- **GStreamer rtspsrc** (0.23):
  - 属性：`location` (RTSP URL), `user-id`, `user-pw`, `protocols` (tcp/udp), `latency` (ms)
  - 需要处理 `pad-added` 信号连接到 `decodebin`
  - 错误处理通过 bus message
- **tokio-tungstenite** (0.24):
  - `connect_async(url)` → `WebSocketStream`
  - `split()` → `Sink + Stream`
  - 发送: `sink.send(Message::text(json)).await`
  - 接收: `stream.next().await`
- **Tauri v2**:
  - `#[tauri::command]` 宏，async 命令自动在 tokio context 执行
  - `State<T>` 注入，需要 `.manage()` 注册
  - 前端 `invoke(cmd, args)` 调用，args 使用 camelCase

## Technical Decisions

| Decision | Rationale |
|----------|-----------|
| 复用 MJPEG 预览路径 | RTSP 客户端拉流后用 `decodebin ! jpegenc ! appsink` 输出 JPEG 帧，推到 MJPEG Server，前端用 `<img>` 消费，与服务端预览完全一致 |
| WebSocket 客户端在 Rust 端 | 前端捕获事件 → Tauri invoke → Rust WebSocket 客户端转发，避免浏览器 WebSocket 限制 |
| 前端传相对坐标 | 简化前端逻辑，后端拥有 SDP 信息可直接获取分辨率 |
| 指数退避重连 | 初始 2s，翻倍至最大 30s，3 次后标记离线，避免频繁重连消耗资源 |
| 客户端模式用紫色系标识 | 区别于服务端模式的青色系，视觉上明确区分两种模式 |
| 共享 MJPEG Server | 避免额外端口消耗，source_id 使用 `rtsp-client-{id}` 前缀区分 |
| Tab 切换模式 | 不引入 vue-router，保持简单 |
| 优先 GStreamer pad-added 获取分辨率 | 自动化优先，备选方案为用户手动输入 |

## Issues Encountered

| Issue | Resolution |
|-------|------------|
| `ElementFactory::make("jpegenc").build()` panics in RTSP client connect | Replaced with `parse::launch()` + explicit element naming + `set_property()` for dynamic props |
| `videoconvert0` auto-name unreliable in `pipeline.by_name()` | Added explicit `name=vconv` in launch string |
| `manager` moved into `spawn_blocking` then used again | Clone `manager_for_mjpeg` before the move closure |
| `remote_resolution` never set → all remote commands fail | Added `ws_remote_set_resolution` command + auto-sync watch in App.vue |
| Redundant `gstreamer::init()` in `client.rs` | Removed — already called in `main.rs` |

## Resources
- 设计稿：`design/rtsp-client.html`
- 现有 RTSP 服务端：`src-tauri/src/rtsp/server.rs`
- 现有反控协议：`src-tauri/src/remote/mod.rs`
- 现有反控 WebSocket 服务端：`src-tauri/src/remote/websocket.rs`
- 现有 MJPEG Server：`src-tauri/src/pipeline/mjpeg_server.rs`
- 现有 Pipeline Manager：`src-tauri/src/pipeline/manager.rs`
- 现有预览 Pipeline：`src-tauri/src/pipeline/preview.rs`
- GStreamer rtspsrc 文档：https://gstreamer.freedesktop.org/documentation/rtsp/rtspsrc.html

## Visual/Browser Findings
- 界面设计稿已创建并预览，三栏布局：左侧流列表 | 中间预览 | 右侧反控面板
- 流列表支持添加/删除/选择流，每条流显示名称、状态徽章、URL、分辨率/延迟/FPS
- 预览区有 HUD 叠加、信号强度指示器、反控模式十字准星
- 右侧反控面板包含 WebSocket 配置、控制能力网格、虚拟键盘

---
*Last updated: Research pass completed*
