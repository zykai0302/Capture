# 02 — 全量数据类型定义

> 前端 TypeScript 与后端 Rust 类型必须 1:1 镜像，确保 IPC 序列化一致。

## 1 Rust 类型定义

### 1.1 采集源 (`capture/source.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceType {
    Monitor,
    Window,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureSource {
    pub id: String,           // "screen-0" | "window-12345"
    pub name: String,         // "显示器 0" | "VS Code"
    pub source_type: SourceType,
    pub width: u32,
    pub height: u32,
    pub is_streaming: bool,
    pub rtsp_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureSourceList {
    pub monitors: Vec<CaptureSource>,
    pub windows: Vec<CaptureSource>,
}
```

**CaptureSourceList 方法**:
- `find_by_id(&self, id: &str) -> Option<&CaptureSource>` — 在 monitors + windows 中查找
- `all(&self) -> Vec<&CaptureSource>` — 合并所有源
- `contains_id(&self, id: &str) -> bool` — 检查是否存在

### 1.2 编码配置 (`encode/config.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Codec { H264, H265 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncodeMode {
    Auto,        // GPU 优先，不可用则 fallback CPU
    GpuOnly,
    CpuOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RateControl { CBR, VBR }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncodePreset { Speed, Balanced, Quality }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Resolution {
    Original,
    Custom { width: u32, height: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodeConfig {
    pub codec: Codec,
    pub mode: EncodeMode,
    pub resolution: Resolution,
    pub framerate: u32,        // 10-60
    pub bitrate_kbps: u32,     // 500-20000
    pub max_bitrate_kbps: u32,
    pub rate_control: RateControl,
    pub gop_size: u32,         // 10-120
    pub preset: EncodePreset,
}
```

**默认值** (`impl Default for EncodeConfig`):
| 字段 | 默认值 |
|------|--------|
| codec | H264 |
| mode | Auto |
| resolution | Original |
| framerate | 30 |
| bitrate_kbps | 4000 |
| max_bitrate_kbps | 6000 |
| rate_control | VBR |
| gop_size | 30 |
| preset | Balanced |

### 1.3 GPU 检测结果 (`encode/detector.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuCapability {
    pub has_amf: bool,
    pub has_mf: bool,
    pub has_vaapi: bool,           // Linux
    pub has_videotoolbox: bool,    // macOS
    pub amf_encoders: Vec<String>,
    pub mf_encoders: Vec<String>,
    pub vt_encoders: Vec<String>,     // macOS VideoToolbox
    pub vaapi_encoders: Vec<String>,  // Linux VAAPI
}
```

### 1.4 Pipeline 状态 (`pipeline/mod.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineState {
    Stopped,
    Starting,
    Running,
    Error(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct PipelineStatus {
    pub source_id: String,
    pub state: PipelineState,
    pub encoder_used: String,
    pub is_gpu: bool,
    pub fps: f64,
    pub bitrate_kbps: u32,
    pub latency_ms: u32,
    pub rtsp_url: String,
}
```

**PipelineManager trait**:

```rust
pub trait PipelineManager: Send + Sync {
    fn start_pipeline(&self, source: &CaptureSource, config: &EncodeConfig) -> AppResult<()>;
    fn stop_pipeline(&self, source_id: &str) -> AppResult<()>;
    fn stop_all(&self) -> AppResult<()>;
    fn get_status(&self, source_id: &str) -> Option<PipelineStatus>;
    fn get_all_status(&self) -> Vec<PipelineStatus>;
    fn update_config(&self, source_id: &str, config: &EncodeConfig) -> AppResult<()>;
    fn get_rtsp_url(&self, source_id: &str) -> Option<String>;
}
```

### 1.5 应用配置 (`config/mod.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub rtsp_port: u16,           // default: 8554
    pub rtsp_max_clients: u32,    // default: 10
    pub ws_port: u16,             // default: 9001
    pub ws_password: String,      // default: ""
    pub auto_reconnect: bool,     // default: true
    pub default_encode: EncodeConfig,
}
```

### 1.6 远程控制命令 (`remote/mod.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RemoteCommand {
    MouseMove   { stream_id: String, data: MouseMoveData },
    MouseClick  { stream_id: String, data: MouseClickData },
    MouseScroll { stream_id: String, data: MouseScrollData },
    MouseDrag   { stream_id: String, data: MouseDragData },
    KeyPress    { stream_id: String, data: KeyPressData },
    KeyCombo    { stream_id: String, data: KeyComboData },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseMoveData { pub x: i32, pub y: i32 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseClickData {
    pub x: i32, pub y: i32,
    pub button: MouseButton,
    pub action: ClickAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseButton { Left, Right, Middle }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClickAction { Single, Double }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseScrollData { pub x: i32, pub y: i32, pub dx: i32, pub dy: i32 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseDragData {
    pub from_x: i32, pub from_y: i32,
    pub to_x: i32, pub to_y: i32,
    pub button: MouseButton,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPressData { pub key: String, pub modifiers: Vec<String> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyComboData { pub keys: Vec<String> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteStatus {
    pub ws_port: u16,
    pub is_running: bool,
    pub client_count: u32,
    pub mouse_enabled: bool,
    pub keyboard_enabled: bool,
}
```

### 1.7 错误类型 (`error.rs`)

```rust
#[derive(Error, Debug)]
pub enum AppError {
    #[error("GStreamer error: {0}")]
    GStreamer(String),
    #[error("Pipeline error: {0}")]
    Pipeline(String),
    #[error("Capture error: {0}")]
    Capture(String),
    #[error("Encode error: {0}")]
    Encode(String),
    #[error("RTSP error: {0}")]
    Rtsp(String),
    #[error("Remote control error: {0}")]
    Remote(String),
    #[error("Config error: {0}")]
    Config(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

**序列化行为**: `AppError` 实现了 `Serialize`，序列化为纯字符串（`Display` 输出）：

```rust
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        serializer.serialize_str(self.to_string().as_str())
    }
}
```

**类型别名**: `pub type AppResult<T> = Result<T, AppError>;`

---

## 2 TypeScript 类型定义 (`src/types/index.ts`)

> 以下类型与 Rust 结构体 1:1 对应，ser/de 格式完全一致。

```typescript
// === 枚举 ===

export enum SourceType {
  Monitor = 'Monitor',
  Window = 'Window',
}

export enum Codec {
  H264 = 'H264',
  H265 = 'H265',
}

export enum EncodeMode {
  Auto = 'Auto',
  GpuOnly = 'GpuOnly',
  CpuOnly = 'CpuOnly',
}

export enum RateControl {
  CBR = 'CBR',
  VBR = 'VBR',
}

export enum EncodePreset {
  Speed = 'Speed',
  Balanced = 'Balanced',
  Quality = 'Quality',
}

export enum PipelineStateEnum {
  Stopped = 'Stopped',
  Starting = 'Starting',
  Running = 'Running',
}

// === 接口 ===

export interface CaptureSource {
  id: string
  name: string
  source_type: SourceType
  width: number
  height: number
  is_streaming: boolean
  rtsp_url: string | null
}

export interface CaptureSourceList {
  monitors: CaptureSource[]
  windows: CaptureSource[]
}

export interface ResolutionOriginal { Original: true }
export interface ResolutionCustom { Custom: { width: number; height: number } }
export type Resolution = ResolutionOriginal | ResolutionCustom

export interface EncodeConfig {
  codec: Codec
  mode: EncodeMode
  resolution: Resolution
  framerate: number
  bitrate_kbps: number
  max_bitrate_kbps: number
  rate_control: RateControl
  gop_size: number
  preset: EncodePreset
}

export interface PipelineStateError { Error: string }
export type PipelineState = PipelineStateEnum | PipelineStateError

export interface PipelineStatus {
  source_id: string
  state: PipelineState
  encoder_used: string
  is_gpu: boolean
  fps: number
  bitrate_kbps: number
  latency_ms: number
  rtsp_url: string
}

export interface GpuCapability {
  has_amf: boolean
  has_mf: boolean
  has_videotoolbox: boolean
  has_vaapi: boolean
  amf_encoders: string[]
  mf_encoders: string[]
  vt_encoders: string[]
  vaapi_encoders: string[]
}

export interface AppConfig {
  rtsp_port: number
  rtsp_max_clients: number
  ws_port: number
  ws_password: string
  auto_reconnect: boolean
  default_encode: EncodeConfig
}

export interface RemoteStatus {
  ws_port: number
  is_running: boolean
  client_count: number
  mouse_enabled: boolean
  keyboard_enabled: boolean
}
```

---

## 3 辅助函数 (`src/types/index.ts`)

```typescript
export function defaultEncodeConfig(): EncodeConfig {
  return {
    codec: Codec.H264,
    mode: EncodeMode.Auto,
    resolution: { Original: true },
    framerate: 30,
    bitrate_kbps: 4000,
    max_bitrate_kbps: 6000,
    rate_control: RateControl.VBR,
    gop_size: 30,
    preset: EncodePreset.Balanced,
  }
}

export function isPipelineRunning(state: PipelineState): boolean {
  if (typeof state === 'string') return state === PipelineStateEnum.Running
  return false
}

export function isPipelineError(state: PipelineState): boolean {
  return typeof state === 'object' && 'Error' in state
}

export function getPipelineError(state: PipelineState): string {
  if (typeof state === 'object' && 'Error' in state) return state.Error
  return ''
}

export function getPipelineStateLabel(state: PipelineState): string {
  if (typeof state === 'string') {
    switch (state) {
      case PipelineStateEnum.Stopped: return '已停止'
      case PipelineStateEnum.Starting: return '启动中'
      case PipelineStateEnum.Running: return '推流中'
      default: return state
    }
  }
  return state.Error
}
```

---

## 4 序列化格式对照

| Rust 类型 | JSON 示例 | TypeScript 类型 |
|-----------|-----------|----------------|
| `SourceType::Monitor` | `"Monitor"` | `SourceType.Monitor` |
| `Codec::H264` | `"H264"` | `Codec.H264` |
| `EncodeMode::Auto` | `"Auto"` | `EncodeMode.Auto` |
| `Resolution::Original` | `"Original"` | `{ Original: true }` |
| `Resolution::Custom { width: 1920, height: 1080 }` | `{"Custom":{"width":1920,"height":1080}}` | `{ Custom: { width: 1920, height: 1080 } }` |
| `PipelineState::Running` | `"Running"` | `PipelineStateEnum.Running` |
| `PipelineState::Error("xxx")` | `{"Error":"xxx"}` | `{ Error: "xxx" }` |
| `RemoteCommand::MouseMove{..}` | `{"type":"mouse_move","stream_id":"s","data":{"x":1,"y":2}}` | 同左 |

**关键差异**: Rust `Resolution` 枚举序列化格式：
- `Original` → JSON 字符串 `"Original"` (unit variant)
- `Custom { width, height }` → JSON 对象 `{"Custom":{"width":W,"height":H}}`

前端需使用区分联合类型 `ResolutionOriginal | ResolutionCustom` 处理。
