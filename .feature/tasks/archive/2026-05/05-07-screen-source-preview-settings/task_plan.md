# Task Plan: 画面源列表滚动、实时预览、推流画面显示与设置面板折叠

## Goal

优化主界面三大区域交互：左侧画面源滚动+缩略图自动刷新、中间推流画面按编码帧率实时显示（MJPEG）、右侧设置面板可折叠/展开。

## Current Phase

Phase 5 (integration test)

## Phases

### Phase 1: 后端截图命令 + 前端缩略图展示
- [x] 新增 `src-tauri/src/capture/thumbnail.rs` — BitBlt/PrintWindow 截图
- [x] 注册 `capture_thumbnail` Tauri 命令
- [x] 前端 SourceItem 替换 SVG 为真实缩略图 `<img>`
- [x] useSources 添加缩略图获取 + 5s 自动刷新
- [x] 确认 SourceList 滚动行为正常
- **Status:** complete

### Phase 2: 后端独立预览 Pipeline + MJPEG HTTP 端点
- [x] 新增 `src-tauri/src/pipeline/preview.rs` — 独立预览 pipeline（d3d12screencapturesrc → videoconvert → jpegenc → appsink）
- [x] 新增 `src-tauri/src/pipeline/mjpeg_server.rs` — tokio MJPEG HTTP 服务
- [x] config 添加 preview_http_port（默认 8090）
- [x] PipelineManager 管理 preview pipeline 生命周期（随推流启停）
- [x] 注册 `start_preview` / `get_preview_url` Tauri 命令
- **Status:** complete

### Phase 3: 前端推流画面显示
- [x] MainPreview 推流中显示 `<img src="http://127.0.0.1:8090/<source_id>">`
- [x] 停止推流时清除 img src
- [x] 非推流状态保持现有占位图
- [x] 预览端口从 config 动态获取
- **Status:** complete

### Phase 4: 设置面板折叠/展开
- [x] App.vue 管理 `showConfigPanel` ref
- [x] TopBar 添加设置齿轮按钮点击事件
- [x] CSS Grid 动态 `grid-template-columns` 过渡
- [x] ConfigPanel 接收折叠状态
- **Status:** complete

### Phase 5: 集成测试与验证
- [ ] 滚动正常、缩略图刷新正常
- [ ] 推流画面按编码帧率显示
- [ ] 设置面板折叠/展开平滑
- [ ] vue-tsc 类型检查通过
- [ ] 后端编译通过
- **Status:** pending

## Key Questions

1. MJPEG 预览 pipeline 与 RTSP pipeline 并行运行，双重 capture 的 CPU/GPU 开销是否可接受？→ 本机 localhost 无带宽限制，额外开销可接受
2. 窗口被最小化时 PrintWindow 能否截图？→ 可能失败，需 fallback 到 SVG 占位图

## Decisions Made

| Decision | Rationale |
|----------|-----------|
| 独立预览 pipeline（非 tee 分支） | 不修改现有 RTSP pipeline，零风险不影响推流稳定性 |
| MJPEG HTTP 端点 + `<img>` | 浏览器原生支持 MJPEG，前端零依赖，延迟 < 100ms |
| tokio::net HTTP 服务器（非 tiny_http） | 复用现有 tokio 运行时，零外部依赖，参考 RemoteControlServer 模式 |
| BitBlt/PrintWindow 截图 | 简单可靠，已有 Win32 API 依赖 |
| image crate JPEG 编码 | 纯 Rust，跨平台，适合缩略图场景（非 GStreamer jpegenc） |
| 缩略图 5s 自动刷新 | 与 useSources 源列表刷新周期一致 |
| 复用 TopBar 现有设置按钮 | 已有 .btn-settings 按钮，只需添加点击事件 |

## Errors Encountered

| Error | Attempt | Resolution |
|-------|---------|------------|
|       | 1       |            |

## Notes

- Update phase status as you progress: pending → in_progress → complete
- Re-read this plan before major decisions
- Log ALL errors - they help avoid repetition
