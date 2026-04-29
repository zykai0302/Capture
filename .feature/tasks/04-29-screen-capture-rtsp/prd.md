# 桌面抓屏软件 - 抓屏编码RTSP推流

## Goal

开发一款**跨平台**桌面抓屏软件，支持**多路同时推流**，能够抓取主屏、扩展屏、应用窗口画面，通过 H264/H265 **GPU 硬件编码**（GPU 不可用时自动软编 fallback）后以 RTSP 协议封装推流发送。支持**反控功能**（解码端可反控发流端窗口）。用户选中画面源后，系统自动完成 抓取→编码→封装→发送 的完整链路。

## What I already know

* 目标平台：**跨平台**（Windows + macOS + Linux）
* 核心功能：抓屏 + H264/H265 GPU编码 + RTSP推流
* 画面源类型：主屏、扩展屏、应用窗口
* 流程：选中画面 → 抓取 → 编码 → 封装 → 发送
* 必须支持 GPU 抓屏和 GPU 编解码
* **多路同时推流**：当前即需实现
* **反控功能**：解码端可通过 RTSP 反控发流端窗口
* GPU 不可用时自动 fallback 到软编码
* 画面源热插拔需要动态处理
* 编码参数界面可配置
* 失败/边界场景必须处理
* 用户需要先选择技术路线再设计界面

## Assumptions (temporary)

* 需要低延迟推流（实时性要求高）
* RTSP 作为服务端（其他客户端拉流）
* GPU 硬件编码优先，软编码作为 fallback
* 反控信令通过 RTSP 扩展或独立通道实现

## Open Questions

* ~~MVP 先做哪个平台~~ → 已确认：Windows 优先
* ~~延迟要求具体指标~~ → 已确认：低延迟 <500ms，画质较好
* ~~目标分辨率和帧率默认值~~ → 推荐：原始分辨率 / 30fps（用户可在 UI 中调整）
* ~~反控的具体操作范围~~ → 已确认：鼠标+键盘全控制
* ~~MVP 先做哪个平台~~ → 已确认：Windows 优先
* ~~延迟要求具体指标~~ → 已确认：低延迟 <500ms，画质较好
* ~~目标分辨率和帧率默认值~~ → 推荐：原始分辨率 / 30fps（用户可在 UI 中调整）

## Requirements (evolving)

### P0 — 核心链路（MVP 必做，Windows 优先）

* **跨平台**：Windows + macOS + Linux，MVP 从 Windows 开始
* **GPU 抓屏**：各平台使用最优 GPU 抓屏 API
* **GPU 编码**：H264/H265 硬件编码，自动适配平台硬件
* **软编 Fallback**：GPU 不可用时自动切换到 x264/x265 软编码
* **RTSP 推流**：RTSP 服务端模式，外部客户端拉流
* **多路同时推流**：多个画面源各推独立 RTSP 流
* **画面源枚举**：枚举主屏、扩展屏、应用窗口
* **实时 Pipeline**：抓取→编码→封装→发送
* **低延迟**：端到端延迟 <500ms

### P1 — 健壮性（MVP 必做）

* **画面源热插拔**：显示器接入/断开、窗口关闭时动态处理
* **编码参数可配**：分辨率、帧率、码率、编码器选择等在 UI 可配置
* **失败处理**：
  - UAC/安全桌面/DRM 内容无法抓取时给出明确提示
  - 显卡驱动不兼容时自动 fallback
  - 网络断开后 RTSP 客户端自动重连
  - 编码失败时自动降级

### P2 — 反控功能（MVP 必做）

* **解码端反控**：RTSP 客户端可向发流端发送控制指令
* 反控信令通道（RTSP 扩展 / 独立 WebSocket / 自定义 TCP）
* **反控操作范围**：鼠标（移动/点击/双击/滚动/拖拽）+ 键盘（按键/组合键/输入），远程桌面级全控制
* 反控操作映射到发流端目标窗口（通过 enigo 注入系统级事件）

### P3 — 未来扩展（预留接口，后续实现）

* **音频采集推流**：采集系统音频/麦克风，与视频同步推流
* **录制存储**：推流同时录制为本地文件（MP4/MKV）

## Acceptance Criteria (evolving)

* [ ] 能够枚举并显示所有可用画面源（主屏、扩展屏、窗口）
* [ ] 选中画面源后能启动抓屏
* [ ] 抓屏画面能以 H264/H265 GPU 硬件编码
* [ ] GPU 不可用时自动 fallback 到软编码
* [ ] 编码后数据能以 RTSP 协议推流
* [ ] 支持多路画面源同时推流
* [ ] 外部播放器（如 VLC）能拉流播放
* [ ] 画面源热插拔时系统能动态处理（不崩溃、自动重连/断开）
* [ ] 编码参数可在 UI 中配置并实时生效
* [ ] 反控指令能从解码端传到发流端并执行
* [ ] Windows / macOS / Linux 均可运行

## Definition of Done (team quality bar)

* 核心链路跑通：抓屏→编码→RTSP推流→VLC拉流播放
* 多路推流可同时运行
* 反控链路跑通：解码端操作→发流端窗口响应
* 代码结构清晰，模块解耦
* 配置可调（分辨率、帧率、编码器、码率等）
* UI 可操作，状态反馈明确
* 热插拔和异常场景有合理处理
* 三平台均可编译运行

## Out of Scope (explicit)

* 音频采集推流（预留接口，后续实现）
* 录制存储功能（预留接口，后续实现）
* Web 远程管理界面
* 用户认证/权限管理

## Decision (ADR-lite)

**Context**: 需要跨平台 + GPU 抓屏/编码 + 多路 RTSP 服务端 + 反控信令，技术选型必须同时满足四个约束

**Decision**: 选择 **方案 C — Tauri + Rust + GStreamer**

**Consequences**:
- GStreamer Pipeline 架构天然匹配 抓取→编码→封装→发送 链路，多路只需多 Pipeline 实例
- GStreamer 各平台采集/编码插件覆盖完整，无需逐平台手动对接
- gst-rtsp-server 提供生产级 RTSP 服务端，多路推流原生支持
- GPU 内存可在 Pipeline 插件间零拷贝传递
- 反控通道可通过 RTSP ANNOUNCE 扩展或独立 WebSocket 实现
- 代价：GStreamer 运行时部署体积约 100MB；Rust GStreamer 绑定文档相对少

### 技术栈明细

| 层次 | 技术 | 说明 |
|------|------|------|
| UI 框架 | Tauri 2.x | 跨平台、轻量、Rust 后端 |
| 前端 | HTML/CSS/JS (Vue3 或 React) | Tauri webview 渲染 |
| 核心引擎 | GStreamer (gstreamer-rs) | Pipeline 架构 |
| 抓屏插件 | d3d11screencapture(Win) / avfvideosrc(macOS) / ximagesrc(Linux) | 各平台 GPU 抓屏 |
| 编码插件 | nvh264enc/nvh265enc + vaapih265enc + vtenc_h265 | GPU 硬编，自动适配 |
| 软编 fallback | x264enc / x265enc | CPU 软编码 |
| RTSP 服务 | gst-rtsp-server | 生产级 RTSP 服务端，多路支持 |
| 反控通道 | WebSocket (独立信令通道) | 低延迟双向通信 |
| 反控执行 | enigo (Rust crate) | 跨平台鼠标/键盘事件注入 |
| 语言 | Rust | 安全、高性能、GStreamer 绑定成熟 |

### 反控架构设计

```
[解码端/客户端]                    [发流端/服务端]
     |                                  |
     |--- RTSP 拉流 --->|              |--- GStreamer 抓屏编码推流 --->|
     |                                  |
     |--- WebSocket 反控信令 --->|      |--- WebSocket Server 接收反控 --->|
                                        |--- enigo 注入鼠标/键盘事件 --->[目标窗口]
```

反控信令协议（JSON over WebSocket）：
```json
// 鼠标移动
{ "type": "mouse_move", "stream_id": "screen-0", "data": { "x": 100, "y": 200 } }

// 鼠标点击（left/right/middle, single/double）
{ "type": "mouse_click", "stream_id": "screen-0", "data": { "x": 100, "y": 200, "button": "left", "action": "single" } }

// 鼠标滚动
{ "type": "mouse_scroll", "stream_id": "screen-0", "data": { "x": 100, "y": 200, "dx": 0, "dy": -3 } }

// 鼠标拖拽
{ "type": "mouse_drag", "stream_id": "screen-0", "data": { "from_x": 100, "from_y": 200, "to_x": 300, "to_y": 400, "button": "left" } }

// 键盘按键
{ "type": "key_press", "stream_id": "screen-0", "data": { "key": "a", "modifiers": ["ctrl"] } }

// 组合快捷键
{ "type": "key_combo", "stream_id": "screen-0", "data": { "keys": ["ctrl", "alt", "delete"] } }
```

## Technical Notes

### 各平台 GPU 抓屏方案
- **Windows**: GStreamer `d3d11screencapture` 插件，基于 DXGI Desktop Duplication，GPU 零拷贝
- **macOS**: GStreamer `avfvideosrc` 插件，基于 AVFoundation/ScreenCaptureKit
- **Linux**: GStreamer `ximagesrc` + PipeWire/Dmabuf，支持 Wayland

### 各平台 GPU 编码方案
- **NVIDIA**: `nvh264enc` / `nvh265enc` (NVENC)
- **Intel/AMD**: `vaapih264enc` / `vaapih265enc` (VAAPI, Linux/Win)
- **Apple**: `vtenc_h264` / `vtenc_h265` (VideoToolbox)
- **软编 fallback**: `x264enc` / `x265enc`

### Pipeline 示例 (Windows + NVIDIA，单路)
```
d3d11screencapture monitor-index=0 ! nvh264enc bitrate=4000 ! rtph264pay ! rtspclientsink
```

### 多路推流架构
每个画面源对应一个独立的 GStreamer Pipeline 实例，gst-rtsp-server 通过 media factory 为每路提供独立的 RTSP URL：
- `rtsp://host:8554/screen-0` (主屏)
- `rtsp://host:8554/screen-1` (扩展屏)
- `rtsp://host:8554/window-12345` (窗口)

### 关键 Rust Crate
- `gstreamer` - GStreamer 核心绑定
- `gstreamer-app` - AppSrc/AppSink
- `gstreamer-rtsp-server` - RTSP 服务端绑定
- `gstreamer-video` - 视频处理工具
- `tauri` - 桌面应用框架
- `tokio` - 异步运行时（WebSocket + 事件循环）
- `tungstenite` / `tokio-tungstenite` - WebSocket 服务端
- `enigo` - 跨平台鼠标/键盘事件注入
- `serde` + `serde_json` - 反控信令序列化

### 热插拔处理策略
1. 监听系统显示器/窗口变化事件
2. 变化时重新枚举画面源列表
3. 已推流的画面源消失：停止对应 Pipeline，通知 UI
4. 新画面源出现：加入枚举列表，用户可选择是否推流
5. Pipeline 内部重连：GStreamer 元素级别的 error/reconnect 处理

### 编码参数可配项
| 参数 | 范围 | 默认值 |
|------|------|--------|
| 编码器 | H264/H265 | H264 |
| 编码模式 | GPU 硬编 / CPU 软编 / 自动 | 自动 |
| 分辨率 | 原始 / 1080p / 720p / 自定义 | 原始 |
| 帧率 | 10-60 fps | 30 |
| 码率 | 500-20000 kbps | 4000 |
| 码率控制 | CBR / VBR | VBR |
| GOP 大小 | 10-120 | 30 |
| 预设 | speed/quality/balanced | balanced |
