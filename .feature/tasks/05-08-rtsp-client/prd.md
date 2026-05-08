# RTSP Client: Stream Preview with Remote Control

## Goal

为 Screencast Pro 添加 RTSP 客户端模式，允许用户连接到远程 RTSP 流进行实时预览，并通过 WebSocket 对远端设备执行反控操作（鼠标/键盘）。这使得本应用既可以作为推流服务端（现有功能），也可以作为监控客户端使用。

## What I already know

* 项目是 Tauri v2 + Vue 3 + GStreamer 桌面应用
* 已有 RTSP 服务端（`rtsp::RtspServer`）、GStreamer pipeline 管理、MJPEG 预览服务
* 已有 WebSocket 反控服务端（`remote::RemoteControlServer`），支持 mouse_move/click/scroll/drag/key_press/key_combo
* 前端已有 `usePipeline`、`useRemoteControl`、`useSources` 等 composable
* 前端使用深色科技风格，CSS 变量体系完善
* GStreamer 已作为依赖引入，可以复用其 RTSP 客户端能力（`rtspsrc` element）
* 界面设计稿已创建：`design/rtsp-client.html`

## Assumptions (temporary)

* RTSP 拉流使用 GStreamer `rtspsrc` + `decodebin` + `jpegenc` + appsink，复用现有 MJPEG 预览架构
* 反控通过 WebSocket 客户端连接到远端设备的 WebSocket 反控服务端
* 应用同时支持服务端模式和客户端模式（通过 Tab/路由切换）
* 前端新增独立的 Vue 组件体系，不修改现有服务端组件

## Open Questions

* MVP 是否需要 PTZ 控制（云台控制）？（先排除，后续扩展）
* 是否需要同时查看多个 RTSP 流？（MVP 先支持单流预览，多流在 UI 设计中预留）

## Requirements (evolving)

### 核心功能
* 添加/管理 RTSP 流连接（名称、URL、传输协议 TCP/UDP、用户名/密码可选）
* RTSP 认证：添加流时可选择填写用户名/密码，传递给 `rtspsrc` 的 `user-id`/`user-pw` 属性
* RTSP 流实时预览（复用 MJPEG 预览架构）
* WebSocket 反控客户端（连接到远端反控服务，发送鼠标/键盘指令）
* 流状态管理（连接中/已连接/断开/错误）
* **自动重连**：RTSP 流断开后指数退避重连（初始 2s，最大 30s，3 次后标记为离线）；WebSocket 反控断线后同样指数退避重连
* 虚拟键盘（QWERTY 布局 + 常用快捷键）

### 前端组件
* RtspStreamList — 左侧面板，流列表管理
* RtspPreview — 中间区域，视频预览 + HUD
* RemoteControlClient — 右侧面板，反控配置 + 虚拟键盘
* App.vue 模式切换（服务端/客户端 Tab）

### 后端 Tauri 命令
* `rtsp_client_connect` — 建立拉流 pipeline（含 user-id/user-pw 认证参数）
* `rtsp_client_disconnect` — 断开拉流 pipeline
* `rtsp_client_status` — 获取所有客户端流状态
* `ws_remote_connect` — WebSocket 客户端连接远端反控服务
* `ws_remote_disconnect` — 断开远端反控连接
* `ws_remote_send_command` — 发送反控指令（前端传相对坐标，后端映射为绝对坐标）
* 前端传递相对坐标（0.0~1.0），后端根据远端分辨率计算绝对坐标
* `ws_remote_status` — 获取反控客户端状态

## Acceptance Criteria (evolving)

* [ ] 可以添加 RTSP 流地址并成功拉流预览
* [ ] 添加流时可选择填写 RTSP 用户名/密码进行认证
* [ ] 可以断开 RTSP 流连接
* [ ] 流状态实时更新（连接中/已连接/错误/离线）
* [ ] RTSP 流断开后自动重连（指数退避，2s→4s→8s→...→30s，3 次失败后标记离线）
* [ ] WebSocket 反控断线后自动重连（指数退避）
* [ ] 重连状态在 UI 中可见（"重连中 (2/3)..."）
* [ ] 可以连接远端 WebSocket 反控服务（含认证）
* [ ] 可以通过鼠标在预览画面上操作远端设备
* [ ] 可以通过虚拟键盘发送按键指令
* [ ] 可以通过快捷键面板发送组合键
* [ ] 客户端模式与服务端模式可通过 Tab 切换
* [ ] 界面风格与现有设计一致

## Definition of Done

* 后端新增命令有对应的 Rust 类型定义和序列化测试
* 前端 composable 有 TypeScript 类型安全
* Lint / typecheck 通过
* 手动功能测试通过（至少测试一个真实 RTSP 流）
* 错误场景处理（连接超时、认证失败、流中断）

## Decision (ADR-lite)

**Context**: 需要确定 RTSP 客户端模式的坐标映射方案和认证方案
**Decision**: 
1. 坐标映射采用后端计算方案 — 前端传递相对坐标 (0.0~1.0)，后端根据 SDP 获取的远端分辨率计算绝对坐标
2. RTSP 认证纳入 MVP — 添加流时可填写用户名/密码，传递给 rtspsrc 的 user-id/user-pw
3. 自动重连纳入 MVP — RTSP 和 WebSocket 断线后指数退避重连
**Consequences**: 后端需额外存储远端分辨率信息；增加 UI 表单复杂度但提升实用性；自动重连增加状态管理复杂度但显著提升健壮性

## Out of Scope

* PTZ 云台控制
* 多流同时预览（网格布局）
* 流录制/截图功能（后续扩展）
* RTSP 流发现/搜索
* HTTPS/WSS 支持（MVP 仅 ws://）
* SDP 协商或 WebRTC 播放

## Technical Notes

### 关键架构决策

**1. RTSP 拉流方案**：使用 GStreamer `rtspsrc` + `decodebin` + `jpegenc` + `appsink`，复用现有 MJPEG Server 基础设施将 JPEG 帧推送到 HTTP 端点供前端 `<img>` 标签消费。这与当前推流模式的预览路径完全一致。

**2. 反控客户端方案**：在 Rust 端实现 WebSocket 客户端（复用 `tokio-tungstenite`），前端捕获鼠标/键盘事件后通过 Tauri invoke 传递给后端，后端 WebSocket 客户端转发到远端服务。

**3. 坐标映射方案（后端计算）**：前端传递相对坐标（0.0~1.0 浮点数），后端根据存储的远端分辨率（从 RTSP 流 SDP 信息中获取）计算绝对坐标后写入 `RemoteCommand`。前端无需知道远端分辨率。

**4. 模式切换**：App.vue 中增加 Tab 切换（推流服务 / RTSP 客户端），客户端模式使用独立组件体系。

### 文件影响

**新增文件**:
- `src-tauri/src/rtsp/client.rs` — RTSP 客户端 pipeline 管理
- `src-tauri/src/remote/ws_client.rs` — WebSocket 反控客户端
- `src/components/RtspClient/` — 客户端模式组件目录
  - `RtspStreamList.vue`
  - `RtspPreview.vue`
  - `RemoteControlClient.vue`
  - `VirtualKeyboard.vue`
- `src/composables/useRtspClient.ts`
- `src/composables/useWsRemote.ts`

**修改文件**:
- `src-tauri/src/rtsp/mod.rs` — 添加 client 模块
- `src-tauri/src/remote/mod.rs` — 添加 ws_client 模块
- `src-tauri/src/lib.rs` — 添加新的 Tauri 命令
- `src/App.vue` — 添加模式切换
- `src/types/index.ts` — 添加客户端相关类型
- `src/composables/index.ts` — 导出新 composable

### 约束

* 必须复用 GStreamer（已引入，支持 `rtspsrc`）
* 必须复用 `tokio-tungstenite`（已有依赖）
* 前端不引入新依赖
* 保持与现有 UI 风格一致（CSS 变量体系）
