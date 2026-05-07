# 画面源列表滚动、实时预览、推流画面显示与设置面板折叠

## Goal

优化主界面三大区域的交互体验：左侧画面源列表支持滚动浏览和实时缩略图预览、中间推流区域实时显示推流画面（按编码帧率）、右侧设置面板支持折叠/展开切换。

## Requirements

### 1. 画面源列表滚动优化
* `.source-list` 容器确保 `overflow-y: auto` 生效，最大高度受限
* 当源列表超出可视区域时，显示滚动条并可流畅滚动
* 确保不同分辨率/窗口尺寸下均能正常滚动

### 2. 画面源实时预览（缩略图自动刷新）
* 后端新增 Tauri 命令 `capture_thumbnail(source_id: String, source_type: String, width: u32, height: u32) -> Vec<u8>`
  * Windows: 使用 BitBlt（Monitor）或 PrintWindow（Window）截取单帧
  * 输出缩放后的 JPEG 数据（约 320x180）
* 前端 SourceItem 替换 SVG 占位图为真实缩略图
  * 使用 `<img :src="thumbnailDataUrl">` 展示
  * 加载中显示 SVG 占位图作为 placeholder
  * 截图失败时显示 SVG 占位图
* **自动刷新**：每隔 N 秒（默认 5s，与源列表刷新同步）自动刷新所有源的缩略图
  * 刷新时机与 `useSources` 的 5s 轮询一致
  * 仅刷新当前可见的源（优化性能）

### 3. 推流画面实时显示（按编码帧率）
* 选中画面源且推流中时，MainPreview 中央区域显示推流实时画面
* 画面帧率需与编码帧率一致（30fps 编码 → 30fps 显示）
* **方案：后端内嵌 HTTP 端点提供 MJPEG 流**
  * Pipeline 中在 `videoconvert` 后插入 `tee` 分支
  * 主路：继续走 encoder → RTP pay → RTSP
  * 预览路：`jpegenc ! multipartmux` → 通过 HTTP 端点提供 MJPEG 流
  * 后端在 RTSP server 同一线程或新线程启动一个轻量 HTTP 服务器
  * 前端 `<img>` 标签直接请求 `http://127.0.0.1:<http_port>/<source_id>` 获取 MJPEG 流
* 非推流状态保持现有占位图/提示
* 停止推流时自动停止预览流

### 4. 设置面板折叠/展开
* 在 TopBar 右侧添加设置按钮（齿轮图标）
* 点击切换：
  * 展开状态：显示完整设置面板（现有行为）
  * 折叠状态：隐藏设置面板，中间推流区域自动扩展占据右侧空间
* 折叠动画平滑过渡（CSS transition）
* 状态：在 App.vue 中管理 `showConfigPanel` ref

## Acceptance Criteria

* [ ] 画面源列表超过可视区域时，可正常滚动浏览所有源
* [ ] SourceItem 显示真实缩略图（至少 160x90 分辨率），无截图时显示 SVG 占位图
* [ ] 缩略图每 5 秒自动刷新
* [ ] 推流中的画面源，MainPreview 中央区域实时显示推流画面（帧率 = 编码帧率）
* [ ] 未推流时，MainPreview 显示文字提示或占位图
* [ ] 停止推流时预览画面自动消失
* [ ] 点击设置按钮可折叠/展开右侧面板
* [ ] 折叠后推流画面区域自动变大
* [ ] 展开/折叠过渡动画平滑

## Test Cases

- Requirements Analysis: `testcase_analysis.md`
- Test Case Document: `testcase.md`
- Verification Report: `testcase_checkdetail.md`
- Product Type: 桌面推流软件
- Coverage Rate: 100%

## Definition of Done

* 代码通过 TypeScript 类型检查（vue-tsc）
* 代码通过 lint 检查
* 手动功能测试通过（滚动、预览、推流画面、面板折叠）
* 后端新增命令有错误处理
* GStreamer pipeline 修改不影响现有推流功能

## Out of Scope

* 多路推流画面的同时预览（仅预览当前选中源）
* 设置面板折叠状态的持久化（localStorage）
* 推流画面的录制/截图功能
* macOS/Linux 平台的缩略图实现（仅 Windows）

## Technical Approach

### 后端截图命令（thumbnail）
在 `src-tauri/src/capture/` 下新增 `thumbnail.rs`：
* 对 Monitor：`CreateDIBSection` + `BitBlt` 从桌面 DC 复制
* 对 Window：`PrintWindow` 截取窗口内容
* 缩放到 320x180，编码为 JPEG（质量 60）
* 通过 Tauri 命令返回 `Vec<u8>`
* 在 `lib.rs` 注册命令 `capture_thumbnail`

### 推流画面显示（MJPEG HTTP 端点）
这是核心改动。修改 GStreamer pipeline 架构：

**Pipeline 变更**：
```
原：d3d12screencapturesrc → videoconvert → encoder → RTP pay
新：d3d12screencapturesrc → videoconvert → tee name=t
      t. → queue → encoder → RTP pay     (主路，RTSP)
      t. → queue → jpegenc → appsink     (预览路，MJPEG)
```

**HTTP MJPEG 服务**：
* 使用 `tiny_http` 或 `actix-web` 作为轻量 HTTP 服务器
* 端口：配置中新增 `preview_http_port`（默认 8090）
* 路由：`GET /<source_id>` → MJPEG 流
* MJPEG 格式：`multipart/x-mixed-replace` boundary 帧
* 从 appsink 获取 JPEG 帧，写入 HTTP response

**前端**：
* MainPreview 中推流时使用 `<img src="http://127.0.0.1:8090/<source_id>">` 
* 浏览器原生支持 MJPEG 流的 `<img>` 显示
* 停止推流时将 img src 设为空

### 设置面板折叠
* App.vue 管理 `showConfigPanel` 状态（ref<boolean>）
* CSS Grid 使用动态 `grid-template-columns`
* 折叠时：`320px 1fr 0px`，展开时：`320px 1fr 340px`
* 使用 CSS `transition` 实现平滑过渡
* 设置按钮放在 TopBar 右侧

## Decision (ADR-lite)

**Context**: 推流画面需要按编码帧率实时显示
**Decision**: 采用 MJPEG HTTP 端点方案
**Reasoning**:
1. 浏览器原生支持 `<img>` 标签播放 MJPEG 流，前端实现最简单
2. 不需要前端引入额外依赖（不需要 HLS.js、JSMpeg 等）
3. MJPEG 帧率可通过 `jpegenc` 的 `framerate` 参数控制
4. 后端实现简洁：GStreamer `tee` + `jpegenc` + `appsink` + 轻量 HTTP 服务器
5. 延迟极低（< 100ms），远优于 RTSP → HLS 方案（> 2s）
**Consequences**: 
- MJPEG 带宽较大（每帧完整 JPEG），但本机 localhost 无带宽限制
- CPU 开销：jpegenc 编码，但可控制预览帧率（如 15fps）降低开销
- 需要新增 `tiny_http` 依赖

## Technical Notes

### 关键文件变更
| 文件 | 变更 |
|------|------|
| `src/components/SourceList.vue` | 滚动优化（确认现有 overflow 生效） |
| `src/components/SourceItem.vue` | 缩略图展示，替换 SVG |
| `src/components/MainPreview.vue` | 推流画面 MJPEG 显示 |
| `src/components/ConfigPanel.vue` | 接收折叠状态 |
| `src/components/TopBar.vue` | 添加设置按钮 |
| `src/App.vue` | 管理 showConfigPanel 状态，动态 Grid 布局 |
| `src/composables/useSources.ts` | 添加缩略图获取逻辑 |
| `src/types/index.ts` | 新增 thumbnail 相关类型 |
| `src-tauri/src/lib.rs` | 注册 capture_thumbnail 命令 |
| `src-tauri/src/capture/thumbnail.rs` | 截图实现（新增） |
| `src-tauri/src/capture/mod.rs` | 添加 thumbnail 模块 |
| `src-tauri/src/pipeline/gst_pipeline.rs` | 修改 pipeline 添加 tee + 预览分支 |
| `src-tauri/src/pipeline/manager.rs` | 管理 MJPEG appsink 和 HTTP 端点 |
| `src-tauri/src/config/mod.rs` | 新增 preview_http_port 配置 |
| `src-tauri/Cargo.toml` | 新增 tiny_http 依赖 |

### Windows 截图 API 选择
* BitBlt：简单可靠，支持所有 Windows 版本，不支持 DX 全屏
* PrintWindow：支持窗口截图，PW_RENDERFULLCONTENT 支持更多内容
* 不使用 Windows.Graphics.Capture API（复杂，需要 WinRT，仅 Win10+）

### MJPEG 流格式
```
HTTP/1.1 200 OK
Content-Type: multipart/x-mixed-replace; boundary=frame

--frame
Content-Type: image/jpeg
Content-Length: <len>

<jpeg bytes>
--frame
Content-Type: image/jpeg
Content-Length: <len>

<jpeg bytes>
...
```

### Pipeline 变更详情
原 launch string:
```
( d3d12screencapturesrc monitor-index=0 ! videoconvert ! x264enc bitrate=4000 ! rtph264pay name=pay0 pt=96 )
```

新 launch string 需要改为程序化构建（不再使用纯字符串）：
- 因为 RTSPMediaFactory 只支持 launch string，tee 分支的预览路需要在 RTSP pipeline 外单独构建
- **替代方案**：预览路使用独立的 GStreamer pipeline（不修改 RTSP pipeline），通过 Tauri 命令按需启停

### 修订：独立预览 Pipeline 方案
为避免修改现有 RTSP pipeline（可能导致推流不稳定），采用独立预览 pipeline：

```
预览 pipeline: d3d12screencapturesrc → videoconvert → jpegenc → appsink
```
- 与 RTSP pipeline 并行，独立启停
- 使用相同的 capture source 参数
- appsink 获取 JPEG 帧 → HTTP MJPEG 端点输出
- 优点：零风险，不影响推流稳定性
- 缺点：额外 CPU/GPU 开销（双重 capture）
