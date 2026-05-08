# Task Plan: RTSP Client — Stream Preview with Remote Control

## Goal

为 Screencast Pro 添加 RTSP 客户端模式，支持连接远端 RTSP 流实时预览，并通过 WebSocket 对远端设备执行反控操作。

## Current Phase

Phase 4 (Validation) — All tasks implemented, bugs fixed, finish-work check passed

## Phases

### Phase 1: Requirements & Discovery
- [x] 理解用户意图：RTSP 客户端 + 反控
- [x] 研究代码库架构（GStreamer、Tauri 命令、Vue 组件体系）
- [x] 创建界面设计稿 `design/rtsp-client.html`
- [x] 完成 PRD 验证
- **Status:** complete

### Phase 2: Backend Implementation (Tasks 1-4)
- [x] Task 1: 添加 `RtspClient` 错误变体和类型
- [x] Task 2: 实现 `rtsp/client.rs` — RTSP 拉流 pipeline
- [x] Task 3: 实现 `remote/ws_client.rs` — WebSocket 反控客户端
- [x] Task 4: 在 `lib.rs` 中注册 Tauri 命令
- **Status:** pending → see `implementation-plan.md`

### Phase 3: Frontend Implementation (Tasks 5-12)
- [x] Task 5: 添加 TypeScript 类型定义
- [x] Task 6: 实现 `useRtspClient` composable
- [x] Task 7: 实现 `useWsRemote` composable
- [x] Task 8: 实现 `RtspStreamList.vue` 组件
- [x] Task 9: 实现 `RtspPreview.vue` 组件
- [x] Task 10: 实现 `VirtualKeyboard.vue` 组件
- [x] Task 11: 实现 `RemoteControlClient.vue` 组件
- [x] Task 12: 集成模式切换到 `App.vue`
- **Status:** pending → see `implementation-plan.md`

### Phase 4: Validation (Tasks 13-14)
- [x] Task 13: 完整构建和类型检查验证
- [ ] Task 14: 手动集成测试
- **Status:** pending

## Key Questions
1. ✅ 坐标映射方案？→ 后端计算（前端传相对坐标 0.0~1.0）
2. ✅ RTSP 认证是否纳入 MVP？→ 纳入，支持 user-id/user-pw
3. ✅ 自动重连是否纳入 MVP？→ 纳入，指数退避重连

## Decisions Made

| Decision | Rationale |
|----------|-----------|
| 坐标映射后端计算 | 前端无需知道远端分辨率；后端从 SDP 获取分辨率，统一处理 |
| RTSP 认证纳入 MVP | IPC/NVR 场景普遍需要认证，GStreamer rtspsrc 原生支持 |
| 自动重连纳入 MVP | 监控场景网络抖动常见，重连是基本需求 |
| 复用 MJPEG 预览架构 | 与服务端模式预览路径一致，减少前端改动 |
| 客户端模式独立组件体系 | 不修改现有服务端组件，通过 Tab 切换 |
| GStreamer rtspsrc 拉流 | 已有 GStreamer 依赖，rtspsrc 成熟稳定 |

## Errors Encountered

| Error | Attempt | Resolution |
|-------|---------|------------|
|       |         |            |

## Notes
- 更新阶段状态：pending → in_progress → complete
- 做重大决策前重读本计划
- 记录所有错误，避免重复
