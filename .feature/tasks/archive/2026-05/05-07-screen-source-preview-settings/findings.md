# Findings & Decisions

## Requirements

- 画面源列表过多时支持滚动浏览
- 画面源显示真实缩略图（非 SVG 占位图），每 5s 自动刷新
- 推流中的画面源在中央区域按编码帧率实时显示
- 右侧设置面板可折叠/展开，折叠后推流画面变大

## Research Summary

### Relevant Specs
- `.feature/spec/frontend/index.md` — 指南未填充，无具体约束
- `.feature/spec/backend/index.md` — 指南未填充，无具体约束

### Existing Patterns
- **Tauri 命令模式**: `#[tauri::command] async fn xxx() -> Result<T, AppError>` + `invoke_handler` 注册
- **Composable 模式**: `export function useXxx()` + `provide/inject` 全局共享
- **异步服务器模式**: `RemoteControlServer` 使用 tokio::net::TcpListener + tokio::spawn 异步循环
- **Pipeline 管理**: GstPipelineManager 通过 Mutex<HashMap> 管理，mpsc channel 与 RTSP 线程通信
- **错误处理**: AppError 枚举 + thiserror，Serialize trait 实现

### Candidate Files To Modify

| File | Change | Risk |
|------|--------|------|
| `src-tauri/src/capture/thumbnail.rs` | 新增：BitBlt/PrintWindow 截图 | Low |
| `src-tauri/src/capture/mod.rs` | 添加 thumbnail 模块声明 | Low |
| `src-tauri/src/lib.rs` | 注册 capture_thumbnail 命令 | Low |
| `src-tauri/src/pipeline/preview.rs` | 新增：独立预览 pipeline（appsink） | Medium |
| `src-tauri/src/pipeline/mjpeg_server.rs` | 新增：tokio HTTP MJPEG 服务器 | Medium |
| `src-tauri/src/pipeline/mod.rs` | 添加 preview/mjpeg_server 模块 | Low |
| `src-tauri/src/pipeline/manager.rs` | 管理 preview pipeline 生命周期 | Medium |
| `src-tauri/src/config/mod.rs` | 添加 preview_http_port 配置 | Low |
| `src-tauri/Cargo.toml` | 添加 image crate（JPEG 编码） | Low |
| `src/components/SourceItem.vue` | 替换 SVG 为缩略图 `<img>` | Low |
| `src/components/MainPreview.vue` | 替换 SVG 为 MJPEG `<img>` | Medium |
| `src/components/TopBar.vue` | 设置按钮添加点击事件（已有按钮） | Low |
| `src/components/ConfigPanel.vue` | 接收折叠 prop | Low |
| `src/App.vue` | 管理 showConfigPanel + 动态 Grid | Medium |
| `src/composables/useSources.ts` | 添加缩略图获取逻辑 | Medium |
| `src/types/index.ts` | 可选：新增 thumbnail URL 类型 | Low |

### Constraints

1. **CSP 无限制**: `tauri.conf.json` 中 `"csp": null`，前端可自由请求 `http://127.0.0.1:8090`
2. **Windows API 依赖已有**: `Cargo.toml` 中 `windows` crate 已有 `Win32_Graphics_Gdi`、`Win32_Foundation`、`Win32_UI_WindowsAndMessaging`
3. **tokio 异步运行时已有**: 项目使用 `tokio = { version = "1", features = ["full"] }`
4. **GStreamer appsink**: 项目未使用过 appsink，需要新增 `gstreamer-app` 依赖（已在 Cargo.toml）
5. **TopBar 已有设置按钮**: `btn-settings` 类，齿轮图标 + "设置" 文字，但无点击事件
6. **Pipeline 使用 launch string**: RTSPMediaFactory 只接受 launch string，不能直接在 RTSP pipeline 中添加 appsink 分支

### Risks

1. **双重 capture 开销**: 独立预览 pipeline 与 RTSP pipeline 并行 capture，CPU/GPU 开销加倍 → 本机场景可接受
2. **d3d12screencapturesrc 并行限制**: 两个 pipeline 同时 capture 同一源可能冲突 → 需测试；如有问题可降级为仅 capture + videoconvert + jpegenc（低帧率）
3. **窗口最小化截图**: PrintWindow 对最小化窗口可能返回黑屏/错误 → 需 fallback 处理
4. **MJPEG 连接管理**: 浏览器 `<img>` 断开连接时需正确清理 → 使用 tokio select! + shutdown 信号
5. **GStreamer JPEG 编码质量**: `jpegenc` 默认质量可能不够 → 设置 quality=60 平衡大小与质量

### Open Questions

- d3d12screencapturesrc 是否支持同一源被两个 pipeline 同时 capture？→ 需实际测试，如不支持则需修改为从 RTSP pipeline tee 分支

## Research Findings

### 关键发现：无需引入 tiny_http

项目已有 tokio 异步运行时（用于 RemoteControlServer 的 WebSocket），可以直接使用 `tokio::net::TcpListener` 构建轻量 HTTP MJPEG 服务器，无需引入 tiny_http 外部依赖。

MJPEG 服务器可参考 RemoteControlServer 的模式：
- `tokio::net::TcpListener::bind` 监听端口
- `tokio::spawn` 处理每个连接
- `tokio::select!` 支持优雅关闭
- 从 appsink 的 channel 读取 JPEG 帧，写入 HTTP response

### 关键发现：TopBar 已有设置按钮

`TopBar.vue` 第 65-71 行已有 `.topbar-right .btn-settings` 按钮（齿轮图标 + "设置" 文字），但当前无点击事件。只需添加 `@click` 事件和 emit 即可，无需重新设计按钮。

### 关键发现：AppError 需扩展

需新增 `Preview(String)` 错误变体用于预览 pipeline 和 MJPEG 服务器的错误。

### 关键发现：JPEG 编码方案

对于缩略图（thumbnail），需要 JPEG 编码能力。选项：
1. **image crate**: 纯 Rust，跨平台，但速度较慢
2. **GStreamer jpegenc**: 需要 pipeline，太重
3. **手动 JPEG 编码**: 过于复杂

推荐：使用 `image` crate（~2MB），在 thumbnail.rs 中使用，无需 GStreamer。

### 关键发现：GStreamer appsink 已有依赖

`Cargo.toml` 已包含 `gstreamer-app = "0.23"`，可以直接使用 appsink 从 pipeline 获取帧数据。

### 关键发现：CSS Grid 动态过渡

App.vue 使用 `grid-template-columns: 320px 1fr 340px`。CSS `transition` 不直接支持 `grid-template-columns` 过渡，但现代浏览器（Chrome 107+）已支持。可使用：
```css
.app {
  transition: grid-template-columns 0.3s ease;
}
```

### 关键发现：useSources 5s 轮询架构

`useSources` 在 `onMounted` 时启动 5s 轮询刷新源列表。缩略图刷新可以：
1. 在同一轮询周期中批量请求所有源的缩略图（简单但可能慢）
2. 使用独立的轮询周期（更灵活但更复杂）
3. **推荐**：在 refresh 完成后，对每个源触发缩略图获取（异步批量）

## Technical Decisions

| Decision | Rationale |
|----------|-----------|
| 独立预览 pipeline（不修改 RTSP pipeline） | 零风险，不影响现有推流稳定性 |
| tokio::net HTTP MJPEG 服务器（非 tiny_http） | 复用现有 tokio 运行时，零外部依赖 |
| MJPEG HTTP 端点 + `<img>` 标签 | 浏览器原生支持 MJPEG，前端零依赖 |
| BitBlt (Monitor) + PrintWindow (Window) | 简单可靠，已有 Win32 API 依赖 |
| image crate JPEG 编码 | 纯 Rust，跨平台，适合缩略图场景 |
| 缩略图 320x180 JPEG | 平衡质量与传输大小 |
| 复用 TopBar 现有设置按钮 | 已有 UI，只需添加事件 |

## Issues Encountered

| Issue | Resolution |
|-------|------------|
| 修改 RTSP pipeline 添加 tee 分支有风险 | 采用独立预览 pipeline |
| tiny_http 额外依赖 | 改用 tokio::net 复用现有运行时 |
| CSS grid-template-columns 过渡 | 现代浏览器已原生支持 |

## Testing Findings

### Product Type
- Type: 桌面推流软件（非标准分类）
- Checklist: 无标准 Checklist，基于 PRD 验收标准自建

### Coverage Analysis
- Total test cases: 38 (original 31 + supplemented 7)
- Coverage rate: 100%
- Missing points supplemented: 键盘/鼠标滚动、滚动条可拖动、缩略图内容一致性、画面比例、长时间稳定性、默认展开、状态保持

### Test Constraints
- 长时间推流测试需30分钟
- 大量源测试需系统存在20+画面源
- 推流中断测试需模拟进程异常

## Resources

- GStreamer appsink: https://gstreamer.freedesktop.org/documentation/app/appsink.html
- Windows BitBlt: https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-bitblt
- Windows PrintWindow: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-printwindow
- image crate: https://crates.io/crates/image
- CSS grid animation: https://developer.chrome.com/blog/animated-grid-layouts
- MJPEG format: `Content-Type: multipart/x-mixed-replace; boundary=frame`

## Implementation Findings

### 关键发现：preview_http_port 硬编码 bug
- manager.rs 中 `preview_http_port` 初始值被错误硬编码为 8080
- 修正为从 `config.preview_http_port` 读取
- 默认值为 8090（在 AppConfig::default 中定义）

### 关键发现：cargo check shell 问题
- 此环境中 cargo check 命令无法正常执行（shell 超时问题）
- 子代理也无法执行 cargo check
- 前端 vue-tsc --noEmit 可正常执行且通过
- 后端编译验证需要用户手动运行

### 关键发现：TypeScript AppConfig 命名约定
- 项目 TypeScript 中 AppConfig 使用 snake_case 字段名（preview_http_port）
- 与 Tauri 默认的 camelCase 映射不同，但项目选择保持 snake_case 一致

### Check 阶段发现：MJPEG 服务器线程安全 bug
- `mpsc::Receiver` 不可克隆，导致同一 source_id 只能有一个 MJPEG 客户端
- 修复：改用 `tokio::sync::broadcast` channel，支持多客户端订阅
- 架构调整：`start_preview` 中创建 broadcast channel 并注册到 MjpegServer，`spawn_blocking` 做 mpsc→broadcast 转发
- `MjpegServer::add_source` 接受 `broadcast::Sender`，仅做 HashMap insert，无锁争用

### Check 阶段发现：stop_pipeline 未清理 MJPEG source
- `stop_pipeline` 停止了 preview pipeline 但未从 MJPEG 服务器移除 source
- 导致 MJPEG HTTP 连接挂起，无法正确断开
- 修复：添加 `remove_source_sync` 方法，使用 `blocking_lock` 在同步上下文中移除
- 添加调用约束注释：必须从 `spawn_blocking` 或 `std::thread` 调用，不能从 tokio async 上下文直接调用

### Check 阶段发现：前端未调用 stop_preview
- MainPreview 的 `watch(isRunning)` 在推流停止时只清空了 `previewUrl`
- 后端 preview pipeline 和 MJPEG source 未被清理
- 修复：添加 `stop_preview` Tauri 命令，前端在 isRunning 变 false 时调用
- `stop_preview` 停止 preview pipeline 并从 MJPEG 服务器移除 source

### Review 阶段发现：add_source 中 spawn_blocking 反复获取异步锁
- 原实现中 `spawn_blocking` 转发循环每帧都调用 `sources.blocking_lock()`
- 导致与 `remove_source`/`remove_source_sync` 的锁争用
- 修复：将 broadcast channel 创建和 mpsc→broadcast 转发逻辑移到 `start_preview` 中
- `spawn_blocking` 中仅持有克隆的 `broadcast::Sender`（Clone），直接 `btx.send()` 无需加锁

### Bug Fix 发现：显示器推流预览反转 — handle 未从前端传到后端
- `start_stream` Tauri 命令中 `CaptureSource` 构造时 `handle: 0`
- 前端 `startStream` 函数的参数类型不包含 `handle`，调用 `invoke` 时也未传递
- 导致所有推流和预览管线走 `monitor-index` 分支，而 `monitor-index` 与 DXGI 内部枚举顺序不一致
- 修复：
  1. `lib.rs` `start_stream` 新增 `handle: u64` 参数
  2. `usePipeline.ts` `startStream` 新增 `handle` 参数并传给后端
  3. `composables.test.ts` 测试用例添加 `handle` 字段
- 教训：新增 `CaptureSource.handle` 字段后，需完整检查从枚举到管线构建的完整数据流，不能遗漏中间环节

### Review 发现：缩略图 `find_monitor_by_index` 索引映射与 source_id 不一致
- `find_monitor_by_index` 按 `EnumDisplayMonitors` 的枚举顺序（0, 1, 2...）匹配
- 但 `source_id`（`screen-N`）中的 N 来自 `\\.\DISPLAY` 编号减1（如 `\\.\DISPLAY1` → `screen-0`）
- 两种编号体系不一致时（如双显示器互换），缩略图会抓取错误的显示器
- 修复：`capture_monitor_thumbnail` 优先使用 HMONITOR 直接定位（与推流管线一致），仅当 `handle == 0` 时回退到索引枚举
- 同步修复：`capture_thumbnail` Tauri 命令 + `useSources.ts` 传递 `handle` 参数
- 教训：同一类型 bug（索引映射不一致）需全面搜索所有代码路径，不能仅修复一处

### Review 发现：媒体查询覆盖动态 Grid
- `App.vue` `@media (max-width: 1200px)` 中 `grid-template-columns: 260px 1fr 280px` 固定了右侧 280px
- 与 `:style="{ gridTemplateColumns: showConfigPanel ? '320px 1fr 340px' : '320px 1fr 0px' }"` 动态绑定冲突
- CSS 媒体查询优先级高于 inline style（在某些浏览器中），导致折叠面板时右侧仍占 280px
- 修复：媒体查询默认折叠（`260px 1fr 0px`），通过 `.config-visible` 类控制展开
- 教训：CSS Grid 动态布局不应在媒体查询中硬编码列宽

---

*Update this file after every 2 view/browser/search operations*
