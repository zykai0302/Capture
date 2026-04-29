# Findings & Decisions

## Requirements

### 核心需求 (P0)
- 跨平台桌面抓屏软件（Windows + macOS + Linux），MVP 从 Windows 开始
- GPU 抓屏：DXGI Desktop Duplication (Win) / AVFoundation (macOS) / X11+PipeWire (Linux)
- GPU 编码：NVENC/QSV (Win) / VideoToolbox (macOS) / VAAPI (Linux)
- 软编 Fallback：GPU 不可用时自动切换 x264/x265
- H264/H265 视频编码
- RTSP 服务端推流（gst-rtsp-server）
- 多路同时推流（多个画面源各推独立 RTSP 流）
- 低延迟 <500ms

### 健壮性需求 (P1)
- 画面源热插拔动态处理（显示器接入/断开、窗口关闭）
- 编码参数 UI 可配置（分辨率、帧率、码率、编码器、GOP、预设等）
- 失败处理：UAC/DRM 提示、驱动不兼容 fallback、网络断开重连、编码失败降级

### 反控需求 (P2)
- 解码端可通过 WebSocket 反控发流端窗口
- 鼠标全操作：移动/点击/双击/滚动/拖拽
- 键盘全操作：按键/组合键/输入
- 通过 enigo crate 注入系统级事件

### 未来扩展 (P3)
- 音频采集推流（预留接口）
- 录制存储（预留接口）

## Research Findings

### 技术选型对比

| 维度 | Tauri+Rust+FFmpeg | Electron+C+++FFmpeg | Tauri+Rust+GStreamer |
|------|-------------------|---------------------|---------------------|
| 跨平台UI | Tauri 三平台 | Electron 三平台 | Tauri 三平台 |
| GPU抓屏 | 需各平台单独实现 | 需各平台单独实现 | GStreamer 已有各平台采集插件 |
| GPU编码 | FFmpeg hwaccel | FFmpeg hwaccel | GStreamer 各平台编码插件 |
| RTSP服务 | 需额外实现 | 需额外实现 | gst-rtsp-server 原生支持 |
| Pipeline管理 | 手动编排 | 手动编排 | 声明式 Pipeline |
| 部署体积 | ~80MB | ~150MB+FFmpeg | ~100MB |

**结论**：GStreamer 是唯一跨平台 GPU 全链路覆盖方案

### GStreamer Pipeline 示例

Windows + NVIDIA 单路：
```
d3d11screencapture monitor-index=0 ! nvh264enc bitrate=4000 ! rtph264pay ! rtspclientsink
```

### 多路推流 RTSP URL 结构
- `rtsp://host:8554/screen-0` (主屏)
- `rtsp://host:8554/screen-1` (扩展屏)
- `rtsp://host:8554/window-12345` (窗口)

### 反控信令协议（JSON over WebSocket）
```json
{ "type": "mouse_move", "stream_id": "screen-0", "data": { "x": 100, "y": 200 } }
{ "type": "mouse_click", "stream_id": "screen-0", "data": { "x": 100, "y": 200, "button": "left", "action": "single" } }
{ "type": "mouse_scroll", "stream_id": "screen-0", "data": { "x": 100, "y": 200, "dx": 0, "dy": -3 } }
{ "type": "mouse_drag", "stream_id": "screen-0", "data": { "from_x": 100, "from_y": 200, "to_x": 300, "to_y": 400, "button": "left" } }
{ "type": "key_press", "stream_id": "screen-0", "data": { "key": "a", "modifiers": ["ctrl"] } }
{ "type": "key_combo", "stream_id": "screen-0", "data": { "keys": ["ctrl", "alt", "delete"] } }
```

## Technical Decisions

| Decision | Rationale |
|----------|-----------|
| Tauri 2.x + Rust | 轻量跨平台，Rust 直接调 GStreamer FFI，无需 C++ 桥接层 |
| GStreamer (gstreamer-rs) | 唯一跨平台 GPU 全链路方案，Pipeline 架构匹配需求 |
| gst-rtsp-server | 生产级 RTSP 服务端，多路原生支持 |
| WebSocket 反控（tokio-tungstenite） | 独立于 RTSP 流的低延迟双向通道 |
| enigo 事件注入 | 跨平台鼠标/键盘注入 Rust crate |
| 深色工业风 UI | 专业工具定位，JetBrains Mono + Outfit 字体 |

## Issues Encountered

| Issue | Resolution |
|-------|------------|
| Windows 环境 python3 命令不存在 | 使用 `python` 命令替代 |
| Windows 环境 cat 命令不存在 | 使用 read_file 工具替代 |
| cargo check 报 pkg-config not found | 设置 `PKG_CONFIG_PATH=D:\msvc_x86_64\lib\pkgconfig` 和 `PKG_CONFIG=D:\msvc_x86_64\bin\pkg-config.exe` |
| 中文路径导致 zerocopy 编译 STATUS_STACK_BUFFER_OVERRUN | 使用 `CARGO_TARGET_DIR=e:\screencast-build` 重定向编译输出到英文路径 |
| PowerShell 子进程不继承用户 PATH | 需在命令中手动设置 `$env:PATH = 'D:\msvc_x86_64\bin;' + $env:PATH` |
| enigo 0.3 MouseButton 不存在 | 使用 `enigo::Button` 替代 |
| enigo 0.3 click() 不存在 | 使用 `button(btn, Direction::Click)` 替代 |
| enigo 0.3 ScrollDirection 不存在 | 使用 `scroll(length, Axis)` 替代 |
| RTSPServer::builder() 不存在 (gstreamer-rtsp-server 0.23) | 使用 `RTSPServer::new()` + `set_service()` 替代 |
| MutexGuard 跨 await 点 (!Send) | 使用 `.take()` 在 await 前提取值 |
| Tauri webview 字体离线加载 | 后续需内联或本地打包字体文件 |
| Hotplug 不自动停止消失源的 Pipeline | 重写 hotplug.rs：传入 GstPipelineManager，source-removed 时自动调用 stop_pipeline |
| mouse_drag 仅支持左键 | 修复 injector.rs：Right/Middle 按钮映射到 Button::Right/Button::Middle |
| Hotplug 线程可能永久阻塞 | 添加 enumerate_sources_with_timeout()：channel + 10s recv_timeout |
| ensure_rtsp_server TOCTOU 竞态 | 改为持锁检查+启动，消除 check-then-act 窗口 |

## Build Environment Setup

cargo check/build 需要以下环境变量：
```powershell
$env:PATH = 'D:\msvc_x86_64\bin;' + $env:PATH
$env:GSTREAMER_1_0_ROOT_MSVC_X86_64 = 'D:\msvc_x86_64\'
$env:PKG_CONFIG_PATH = 'D:\msvc_x86_64\lib\pkgconfig'
$env:PKG_CONFIG = 'D:\msvc_x86_64\bin\pkg-config.exe'
$env:CARGO_TARGET_DIR = 'e:\screencast-build'  # 避免中文路径导致的编译崩溃
```

## Resources

- GStreamer Rust 绑定: https://gitlab.freedesktop.org/gstreamer/gstreamer-rs
- gst-rtsp-server 文档: https://gstreamer.freedesktop.org/documentation/gst-rtsp-server/
- Tauri 2.x 文档: https://v2.tauri.app/
- enigo crate: https://crates.io/crates/enigo
- tokio-tungstenite: https://crates.io/crates/tokio-tungstenite
- DXGI Desktop Duplication: https://learn.microsoft.com/en-us/windows/win32/direct3ddxgi/desktop-dup-api
- UI 设计稿: `design/ui-preview.html`
- PRD 文档: `.feature/tasks/04-29-screen-capture-rtsp/prd.md`

## Visual/Browser Findings

- UI 设计预览已生成：三栏布局（画面源列表 / 主预览+Pipeline / 配置面板）
- Pipeline 可视化：流动粒子动画表示数据流方向
- 画面源卡片：缩略图 + 类型徽章 + 推流状态 + 编码器徽章
- 反控面板：6 项能力开关 + WebSocket 连接状态 + 客户端数
- 编码配置：编码器选择 + 分辨率/帧率/码率/GOP + GPU 信息

## Frontend Architecture Findings

- Vue 3 前端采用 Composition API + TypeScript
- 4 个 Composables 封装了所有 Tauri IPC 通信:
  - `useSources`: 画面源列表 + source-added/source-removed 事件监听 + 5s 定时刷新
  - `usePipeline`: Pipeline 状态管理 + start/stop + 3s 定时刷新
  - `useRemoteControl`: 反控服务启停 + 5s 定时刷新
  - `useConfig`: 全局配置 + GPU 能力检测
- TypeScript 类型定义 (`src/types/index.ts`) 镜像 Rust 后端数据结构，使用 serde 的 camelCase 约定
- Tauri Command 使用 snake_case (Rust) → camelCase (TypeScript/JS) 自动映射
- `update_encode_config` Command 接受 EncodeConfig 参数，实现运行时编码参数更新
- Pipeline 状态轮询间隔: Pipeline 3s, Sources 5s, Remote 5s
- 实时预览需要 GStreamer AppSink 集成（后续优化，当前使用 SVG 占位）

## Testing Findings

### Product Type
- Type: Screen Control (PK)
- Checklist: testcase_checklist_PK.md (自定义)

### Coverage Analysis
- Total test cases: 82 (H:23, M:42, L:17)
- Coverage rate: 100%
- Modules covered: 画面源管理(5), 推流控制(8), RTSP拉流(7), 编码配置(10), 远程反控(19), 热插拔(4), 编码降级(4), 异常处理(8), GPU检测(2), UI交互(3), 安全性(3), 性能指标(5), 综合场景(4)

### Supplemented Test Points (20 cases added)
- RTSP地址复制与格式验证
- 多客户端拉同一RTSP流
- 分辨率/预设/码控模式修改
- 未推流时修改配置
- 鼠标中键点击、多客户端并发连接与操控
- 未认证发送指令被拒（安全性关键）
- 仅CPU模式验证
- 端口冲突（RTSP/WebSocket）
- UI布局/Tab切换/Pipeline可视化
- 反控响应时间<100ms、多路资源占用

### Test Constraints
- RTSP拉流测试需要VLC或FFplay
- 反控测试需要WebSocket客户端（Python websocket-client 或浏览器）
- 性能测试需要秒表或时间戳工具
- 热插拔测试需要外接显示器
- GPU降级测试需要可控的GPU驱动环境

### Compound Learning (2026-04-30)
- **核心学习**: 屏幕控制类(PK)产品的测试用例设计方法论
- **关键发现**: 初始55个用例经模块交互检查后补充20个，最关键缺失为"未认证跳过认证发送指令"(安全边界)和"反控响应时间"(性能指标)
- **方法论**: 对每个模块不仅检查功能覆盖，还要检查与其他模块的交互和边界条件
- **安全边界测试模式**: 有效凭证→成功、无效凭证→拒绝、无凭证(跳过认证)→拒绝 — 第3项最易遗漏
- **Solution文档**: `.feature/solutions/testing/screen-control-rtsp-testcase-design-2026-04-30.md`

---
*Update this file after every 2 view/browser/search operations*
