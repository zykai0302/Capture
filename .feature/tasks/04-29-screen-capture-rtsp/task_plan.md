# Task Plan: 桌面抓屏软件 - 抓屏编码RTSP推流

## Goal

开发一款跨平台桌面抓屏软件，支持多路同时推流、GPU 硬件编码（自动软编 fallback）、RTSP 推流、反控功能（鼠标+键盘全控制），使用 Tauri + Rust + GStreamer 技术栈，Windows 优先。

## Current Phase

Phase 3-6 (Implementation) — 已完成，review/fix 完成，待 Phase 7-8

## Phases

### Phase 1: Requirements & Discovery
- [x] 理解用户需求：跨平台、GPU 抓屏/编码、多路推流、反控
- [x] 技术路线调研：对比 Tauri/Electron/WPF、DXGI/GDI、FFmpeg/GStreamer/MF
- [x] 确定技术栈：Tauri 2.x + Rust + GStreamer + gst-rtsp-server
- [x] 确认反控范围：鼠标+键盘全控制（远程桌面级）
- [x] 确认 MVP 平台：Windows 优先
- [x] 确认延迟目标：<500ms
- [x] 编写 PRD 文档
- [x] 设计 UI 界面（frontend-design）
- **Status:** complete

### Phase 2: Planning & Structure
- [x] 确定项目目录结构（Rust + Tauri 前端）
- [x] 定义 Rust 模块划分（capture / encode / rtsp / remote / config）
- [x] 定义 GStreamer Pipeline 管理器接口
- [x] 定义 Tauri 前后端通信接口（Commands + Events）
- [x] 定义反控信令协议细节
- [x] 编写 implementation-plan.md
- **Status:** complete

### Phase 3: Implementation - Scaffolding
- [x] 初始化 Tauri 2.x 项目（cargo + npm）
- [x] 集成 GStreamer Rust 绑定
- [x] 实现基础模块骨架（capture / encode / rtsp / config）
- [x] 实现单路 Pipeline：DXGI 抓屏 → AMF H.264 → RTP 封装 → RTSP 推流
- [ ] 验证：VLC 拉流播放成功（需要运行时测试）
- **Status:** mostly complete (needs runtime verification)

### Phase 4: Implementation - Multi-stream & Config
- [x] 实现多路 Pipeline 管理（PipelineManager）
- [x] 实现画面源枚举（显示器 + 窗口）
- [x] 实现 GPU 硬编 / CPU 软编自动 fallback
- [x] 实现编码参数运行时配置（update_encode_config Command + 前端绑定）
- [x] 实现画面源热插拔处理
- **Status:** complete

### Phase 5: Implementation - Remote Control
- [x] 实现 WebSocket 信令服务端
- [x] 实现反控信令解析
- [x] 实现 enigo 鼠标/键盘事件注入
- [x] 实现反控安全校验（密码认证）
- **Status:** complete

### Phase 6: Implementation - UI
- [x] 实现 Tauri 前端界面（基于 UI 设计稿）
- [x] 画面源列表与状态展示
- [x] Pipeline 状态可视化
- [x] 编码参数配置面板
- [x] 反控状态面板
- [ ] 实时预览显示（需 GStreamer AppSink 集成，后续优化）
- **Status:** mostly complete

### Phase 7: Testing & Verification
- [ ] 单路推流端到端测试
- [ ] 多路推流压力测试
- [ ] GPU fallback 切换测试
- [ ] 热插拔场景测试
- [ ] 反控功能测试
- [ ] 延迟测量（<500ms 验证）
- [ ] VLC / FFplay 拉流兼容性测试
- **Status:** pending

### Phase 8: Cross-Platform Adaptation
- [ ] macOS 适配（avfvideosrc + VideoToolbox）
- [ ] Linux 适配（ximagesrc + VAAPI）
- [ ] 各平台编译与打包
- **Status:** pending

## Key Questions
1. ~~跨平台框架选型？~~ → Tauri + Rust
2. ~~核心引擎选型？~~ → GStreamer
3. ~~反控范围？~~ → 鼠标+键盘全控制
4. ~~MVP 平台？~~ → Windows 优先
5. ~~延迟目标？~~ → <500ms
6. GStreamer Windows 部署方式？（MSYS2 vs vcpkg vs 预编译包）
7. 前端框架选型？（Vue3 vs React，Tauri webview 内）
8. Tauri 2.x 与 GStreamer 的线程模型如何协调？

## Decisions Made

| Decision | Rationale |
|----------|-----------|
| Tauri 2.x + Rust | 跨平台、轻量、Rust 直接调 GStreamer FFI |
| GStreamer + gst-rtsp-server | 唯一跨平台 GPU 全链路覆盖方案，Pipeline 架构天然匹配，RTSP 服务端原生支持 |
| DXGI Desktop Duplication | Windows 最高效 GPU 抓屏，GStreamer d3d11screencapture 插件封装 |
| NVENC + VAAPI + VideoToolbox | 各平台最优 GPU 编码，自动 fallback 到 x264/x265 |
| WebSocket 反控通道 | 低延迟双向通信，独立于 RTSP 流，JSON 信令协议 |
| enigo 事件注入 | 跨平台鼠标/键盘注入，Rust 原生 crate |
| 深色工业风 UI | 专业视频工具定位，HUD 状态可视化，Pipeline 流程展示 |
| JetBrains Mono + Outfit | 数据展示用等宽字体，界面用现代无衬线字体 |

## Errors Encountered

| Error | Attempt | Resolution |
|-------|---------|------------|
| Windows 环境 `python3` 命令不可用 | 1 | 使用 `python` 代替 `python3` |
| Windows 环境 `cat` 命令不可用 | 1 | 使用 `read_file` 工具代替 |

## Notes
- PRD 已在 `.feature/tasks/04-29-screen-capture-rtsp/prd.md`
- UI 设计稿在 `design/ui-preview.html`
- 此任务为 Complex Task，需要完整的工作流：research → before-dev → write-plan → write-testcase → executing-plans
