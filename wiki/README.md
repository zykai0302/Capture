# ScreenCast Pro — Wiki 索引

> **Wiki 即代码**：仅依据本 wiki 即可完整复现整个工程，无需查看源码。

## 文档目录

| 文件 | 内容 | 关键词 |
|------|------|--------|
| [00-SUMMARY.md](./00-SUMMARY.md) | 项目元数据、目录结构、配置文件全文 | 包名、版本、Cargo.toml、package.json |
| [01-ARCHITECTURE.md](./01-ARCHITECTURE.md) | 系统架构、分层、AppState、并发模型 | 前后端分层、Tauri IPC、锁策略 |
| [02-DATA-TYPES.md](./02-DATA-TYPES.md) | 全量数据结构定义（Rust + TypeScript 镜像） | 结构体、枚举、序列化格式 |
| [03-TAURI-COMMANDS.md](./03-TAURI-COMMANDS.md) | IPC 命令注册表、参数/返回值/行为规格 | invoke、命令签名、事件 |
| [04-CAPTURE.md](./04-CAPTURE.md) | 屏幕采集模块、Win32 API 调用、热插拔 | DXGI、EnumDisplayMonitors、过滤规则 |
| [05-ENCODE.md](./05-ENCODE.md) | 编码配置、GPU 检测、编码器选择策略 | AMF、MF、x264、fallback |
| [06-PIPELINE.md](./06-PIPELINE.md) | GStreamer Pipeline 构建、管理器生命周期 | launch string、PipelineManager trait |
| [07-RTSP.md](./07-RTSP.md) | RTSP 服务器、GLib MainLoop、流挂载 | gst-rtsp-server、mount points |
| [08-REMOTE.md](./08-REMOTE.md) | 远程反控协议、WebSocket 服务、输入注入 | Enigo、命令协议、认证流程 |
| [09-ERROR.md](./09-ERROR.md) | 错误类型体系、错误传播链 | AppError、thiserror、序列化 |
| [10-FRONTEND.md](./10-FRONTEND.md) | 前端架构、组件树、Composable、布局规格 | Vue 3、provide/inject、Grid 布局 |
| [11-THEME.md](./11-THEME.md) | CSS 变量、暗色主题、动画定义 | 色值、字体、keyframes |
| [12-TESTING.md](./12-TESTING.md) | 测试体系：Rust 单元、前端单元、E2E | vitest、cargo test、Python |
| [13-BUILD.md](./13-BUILD.md) | 构建环境、脚本、打包、环境变量 | GStreamer、MSI、NSIS |
| [14-DATA-FLOW.md](./14-DATA-FLOW.md) | 核心数据流时序图 | 推流流程、反控流程、热插拔流程 |
| [15-DESIGN-DECISIONS.md](./15-DESIGN-DECISIONS.md) | 技术选型决策记录与原因 | GStreamer vs FFmpeg、轮询 vs 事件 |
