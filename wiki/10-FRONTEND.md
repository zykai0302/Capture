# 10 — 前端架构

## 1 应用入口 (`src/main.ts`)

```typescript
import { createApp } from "vue";
import App from "./App.vue";
import "./styles/global.css";

createApp(App).mount("#app");
```

## 2 根组件 (`src/App.vue`)

### 2.1 CSS Grid 布局

```
┌─────────────────────────────────────────────────────────┐
│                     TopBar (52px)                        │  grid-area: topbar
├──────────┬──────────────────────────┬──────────────────┤
│          │                          │                  │
│SourceList│      MainPreview         │   ConfigPanel    │  grid-area: left / center / right
│ (320px)  │        (1fr)             │    (340px)       │
│          │                          │                  │
├──────────┴──────────────────────────┴──────────────────┤
│                    StatusBar (28px)                      │  grid-area: status
└─────────────────────────────────────────────────────────┘
```

```css
.app {
  display: grid;
  grid-template-rows: 52px 1fr 28px;
  grid-template-columns: 320px 1fr 340px;
  grid-template-areas:
    "topbar topbar topbar"
    "left   center right"
    "status status status";
  height: 100vh;
  overflow: hidden;
}

@media (max-width: 1200px) {
  .app {
    grid-template-columns: 260px 1fr 280px;
  }
}
```

### 2.2 状态管理

```typescript
const pipelineStore = usePipeline()
const sourcesStore = useSources()
const configStore = useConfig()
const remoteStore = useRemoteControl()

provide('pipeline', pipelineStore)
provide('sources', sourcesStore)
provide('config', configStore)
provide('remote', remoteStore)
```

子组件通过 `inject` 获取：
```typescript
const { pipelines, startStream, stopStream } = inject<ReturnType<typeof usePipeline>>('pipeline')!
```

### 2.3 选中源逻辑

```typescript
const selectedSource = ref<CaptureSource | null>(null)

function onSelectSource(source: CaptureSource) {
  selectedSource.value = source
}
```

传递给 `MainPreview` 和 `ConfigPanel`，用于显示选中源的详情和配置。

---

## 3 Composable 层

### 3.1 `usePipeline`

| 导出 | 类型 | 说明 |
|------|------|------|
| `pipelines` | `ref<Record<string, PipelineStatus>>` | 以 source_id 为 key 的 Pipeline 状态映射 |
| `loading` | `ref<boolean>` | 加载状态 |
| `error` | `ref<string \| null>` | 错误信息 |
| `startStream(source)` | `async function` | 调用 `invoke('start_stream', ...)` |
| `stopStream(sourceId)` | `async function` | 调用 `invoke('stop_stream', ...)` |
| `stopAllStreams()` | `async function` | 调用 `invoke('stop_all_streams')` |
| `refreshStatus()` | `async function` | 调用 `invoke('get_pipeline_status')` |
| `getStatus(sourceId)` | `function` | 返回指定源的状态 |
| `isStreaming(sourceId)` | `function` | 判断是否正在推流 |

**IPC 超时**: 所有 invoke 调用通过 `invokeWithTimeout` 包装，**30 秒超时**：

```typescript
const INVOKE_TIMEOUT = 30000

function invokeWithTimeout<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return Promise.race([
    invoke<T>(cmd, args),
    new Promise<never>((_, reject) =>
      setTimeout(() => reject(new Error(`Command "${cmd}" timed out after ${INVOKE_TIMEOUT / 1000}s`)), INVOKE_TIMEOUT)
    ),
  ])
}
```

**轮询**: `onMounted` 时刷新状态，之后每 **3 秒**轮询 `refreshStatus()`。

**startStream 参数映射**:
```typescript
await invokeWithTimeout('start_stream', {
  sourceId: source.id,
  sourceType: source.source_type,
  sourceName: source.name,
  width: source.width,
  height: source.height,
})
```

### 3.2 `useSources`

| 导出 | 类型 | 说明 |
|------|------|------|
| `monitors` | `ref<CaptureSource[]>` | 显示器列表 |
| `windows` | `ref<CaptureSource[]>` | 窗口列表 |
| `allSources` | `ref<CaptureSource[]>` | 合并列表 |
| `loading` | `ref<boolean>` | 加载状态 |
| `error` | `ref<string \| null>` | 错误信息 |
| `refresh()` | `async function` | 调用 `invoke('list_sources')` |

**事件监听**: `onMounted` 时注册 Tauri 事件监听：
- `source-added` → `refresh().then(updateAllSources)`
- `source-removed` → `refresh().then(updateAllSources)`

**轮询**: 每 **5 秒**自动刷新。

### 3.3 `useConfig`

| 导出 | 类型 | 说明 |
|------|------|------|
| `config` | `ref<AppConfig \| null>` | 应用配置 |
| `gpuCaps` | `ref<GpuCapability \| null>` | GPU 能力 |
| `loading` | `ref<boolean>` | 加载状态 |
| `error` | `ref<string \| null>` | 错误信息 |
| `loadConfig()` | `async function` | `invoke('get_config')` |
| `loadGpuCapabilities()` | `async function` | `invoke('get_gpu_capabilities')` |
| `getAvailableEncoders()` | `async function` | `invoke('get_available_encoders')` |

**初始化**: `onMounted` 时 `Promise.all([loadConfig(), loadGpuCapabilities()])`

### 3.4 `useRemoteControl`

| 导出 | 类型 | 说明 |
|------|------|------|
| `status` | `ref<RemoteStatus>` | 默认 `{ws_port:9001, is_running:false, client_count:0, ...}` |
| `loading` | `ref<boolean>` | 加载状态 |
| `error` | `ref<string \| null>` | 错误信息 |
| `start(port?, password?)` | `async function` | `invoke('start_remote_control', ...)` |
| `stop()` | `async function` | `invoke('stop_remote_control')` |
| `refresh()` | `async function` | `invoke('get_remote_status')` |

**轮询**: 每 **5 秒**自动刷新。

**start 参数映射**:
```typescript
await invoke('start_remote_control', { port: port ?? null, password: password ?? null })
```

---

## 4 组件树

```
App.vue
├── TopBar.vue
│   └── inject: config, pipeline
├── SourceList.vue
│   ├── inject: sources, pipeline
│   └── SourceItem.vue × N
│       ├── props: source, pipelineStatus?, selected?
│       └── emits: select, startStream, stopStream
├── MainPreview.vue
│   ├── props: selectedSource, pipelineStatus?
│   ├── emits: stopStream
│   └── PipelineVisual.vue
│       └── props: pipelineStatus?
├── ConfigPanel.vue
│   ├── props: selectedSourceId
│   ├── EncodingConfig.vue
│   │   ├── props: sourceId
│   │   └── inject: config, pipeline
│   ├── RemoteControl.vue
│   │   └── inject: remote
│   └── NetworkConfig.vue
│       └── inject: config, pipeline
└── StatusBar.vue
    └── inject: pipeline, config
```

## 5 组件功能规格

### 5.1 TopBar

| 区域 | 内容 |
|------|------|
| 左 | Logo SVG + "SCREENCAST PRO" + "v0.1.0" 徽章 |
| 中 | GPU 类型指示灯(绿) + RTSP 端口指示灯(青) + 推流路数指示灯(橙) |
| 右 | 设置按钮(齿轮图标 + "设置") |

**GPU 标签逻辑**: `has_amf → "AMD GPU (AMF)"` / `has_mf → "GPU (MF)"` / 否则 → `"CPU Only"`

### 5.2 SourceList

| 区域 | 内容 |
|------|------|
| 头部 | "画面源" + "{N} 可用" 徽章 |
| Tab 栏 | "全部" / "显示器" / "窗口" |
| 列表 | SourceItem × filteredSources |
| 空状态 | "未检测到画面源" |

### 5.3 SourceItem

| 区域 | 内容 |
|------|------|
| 顶部 | SVG 缩略图(80x45) + 源名称 + 类型徽章 + 分辨率 |
| LIVE | 推流中时左上角显示红色 "LIVE" 闪烁徽章 |
| 底部 | 状态指示器(绿/灰/红) + 状态文字 + 编码器标签(GPU绿/CPU橙) |
| 按钮 | 未推流→播放按钮 / 推流中→停止按钮 |
| 编码条 | 推流中显示 GPU(青绿渐变) / CPU(橙红渐变) 进度条 |
| RTSP | 推流中显示可复制的 RTSP URL |

**类型徽章**: Monitor → 青色显示器图标 + "显示器" / Window → 紫色窗口图标 + "窗口"

**状态映射**: `isPipelineRunning → "推流中"(绿)` / `isPipelineError → 错误信息(红)` / 其他 → `"就绪"(灰)`

**选中态**: 左侧 3px 青色竖条 + 青色边框 + 青色背景光晕

**推流态**: 左侧 3px 绿色竖条 + 绿色边框 + 绿色背景光晕

### 5.4 MainPreview

| 区域 | 内容 |
|------|------|
| 标题栏 | 源名称 + RTSP URL(可点击复制) + 停止按钮(推流中) |
| Pipeline | PipelineVisual 组件 |
| 预览 | 推流中→SVG 模拟桌面画面 / 未推流→占位符(图标+提示文字) |
| HUD | 推流中叠加: 左上→"{W}x{H} {encoder}" + "延迟: {N}ms" / 右上→"{N} FPS"(绿) |

### 5.5 PipelineVisual

4 节点流程图: `屏幕捕获 → 编码 → RTP 封装 → RTSP 推流`

| 节点 | 图标色 | 标签 | 详情 |
|------|--------|------|------|
| 抓屏 | 青色 | "屏幕捕获" | "d3d12screencapturesrc" |
| 编码 | 绿色 | "{encoder_used}" | "{bitrate}kbps {GPU/CPU}" |
| 封装 | 橙色 | "RTP 封装" | "rtph264pay" |
| 推流 | 紫色 | "RTSP 推流" | "{rtsp_url}" |

推流中: 节点 active + 箭头绿色 + 粒子流动动画

### 5.6 ConfigPanel

3 个 Tab: "编码配置" / "反控设置" / "网络"

### 5.7 EncodingConfig

| 分组 | 字段 |
|------|------|
| 编码器 | 视频编码(select: H.264/H.265) + 编码模式(select: Auto/GpuOnly/CpuOnly) |
| 画面参数 | 分辨率(select: 原始/1920x1080/1280x720) + 帧率(range: 10-60) |
| 码率控制 | 码率模式(select: VBR/CBR) + 目标码率(input: 500-20000) + 最大码率(input: 500-30000) + GOP(input: 10-120) + 编码预设(select: Speed/Balanced/Quality) |
| GPU 信息 | GPU 标签 + GPU 编码能力(H.264 + H.265 AMF / H.264 MF / 无) |

**应用按钮**: `invoke('update_encode_config', { sourceId, config })` + 刷新 Pipeline 状态

**分辨率映射**: `"Original" → { Original: true }` / `"1920x1080" → { Custom: { width: 1920, height: 1080 } }`

### 5.8 RemoteControl

| 分组 | 字段 |
|------|------|
| 反控服务 | 启用开关(toggle) + WebSocket 端口(input) |
| 连接状态 | 运行指示灯 + 客户端数 + 6 项能力网格(鼠标移动/点击/滚动/键盘/快捷键/拖拽) |
| 安全设置 | 反控密码(input password) |

**启用开关逻辑**: `enableRemote = computed({ get: status.is_running, set: val ? start() : stop() })`

### 5.9 NetworkConfig

| 分组 | 字段 |
|------|------|
| RTSP 服务 | 端口(input) + 最大客户端数(input) |
| 推流地址 | 每路推流: sourceId + rtspUrl + 复制按钮 |
| 网络状态 | 活跃推流数(绿) + 自动重连(toggle) |

### 5.10 StatusBar

| 位置 | 内容 |
|------|------|
| 左 | 绿点 + "服务运行中" + "推流: N路" + "GPU: {label}" |
| 右 | "GStreamer 1.28" + "Windows" |

---

## 6 `src/composables/index.ts`

```typescript
export { usePipeline } from './usePipeline'
export { useSources } from './useSources'
export { useConfig } from './useConfig'
export { useRemoteControl } from './useRemoteControl'
```

## 7 `src/vite-env.d.ts`

```typescript
/// <reference types="vite/client" />
```
