# 06 — Pipeline 模块

## 1 模块结构

```
pipeline/
├── mod.rs          → PipelineState, PipelineStatus, PipelineManager trait
├── gst_pipeline.rs → build_launch_string(), gpu_encoder_candidates(), detect_available_encoders()
└── manager.rs      → GstPipelineManager, PipelineHandle, detect_encoder_info()
```

## 2 Pipeline 状态机

```
  Stopped ────→ Starting ────→ Running
    ↑                              │
    └──────────────────────────────┘
                  Error(String) ←── 任何阶段异常
```

- `Stopped`: Pipeline 未运行
- `Starting`: 保留状态（当前实现中未实际使用，直接到 Running）
- `Running`: Pipeline 正在推流
- `Error(String)`: 异常，携带错误消息

## 3 GStreamer Launch String 构建 (`gst_pipeline.rs`)

### 3.1 `build_launch_string(source: &CaptureSource, config: &EncodeConfig) -> String`

**输出格式**: `( {capture_elem} ! {encoder_elem} ! {rtp_pay} )`

#### 3.1.1 采集元素 (`capture_elem`)

平台特定的采集元素通过 `#[cfg(target_os)]` 条件编译选择：

**Windows — Monitor**:
```
d3d12screencapturesrc monitor-index={monitor_idx} ! videoconvert
```

**Windows — Window**:
```
d3d12screencapturesrc window-handle={hwnd} ! videoconvert
```

**macOS — Monitor**:
```
avfvideosrc display-index={monitor_idx} ! videoconvert
```

**macOS — Window**:
```
avfvicesrc window-id={hwnd} ! videoconvert
```

**Linux — Monitor**:
```
ximagesrc monitor-index={monitor_idx} ! videoconvert
```

**Linux — Window**:
```
ximagesrc xid={hwnd} ! videoconvert  (未来)
```

- `monitor_idx`: 从 `source.id` 中解析 (`"screen-2"` → `2`)，解析失败默认 `0`
- `hwnd`: 从 `source.id` 中解析 (`"window-12345678"` → `12345678` as i64)，解析失败默认 `0`

> **为什么使用 d3d12screencapturesrc**: `d3d11screencapturesrc` 在 RTSP 媒体线程中初始化 D3D11 设备会失败（"capture object is not configured yet"），而 `d3d12screencapturesrc` 可在任意线程正常工作。

#### 3.1.2 编码元素 (`encoder_elem`)

详见 [05-ENCODE.md](./05-ENCODE.md) §4。

#### 3.1.3 RTP 载荷元素 (`rtp_pay`)

| Codec | Payloader |
|-------|-----------|
| H264 | `rtph264pay` |
| H265 | `rtph265pay` |

> RTP payloader 已内置于 `build_encoder_element()` 输出（`encoder_params ! rtph264pay`），`name=pay0 pt=96` 由 `build_launch_string()` 模板附加。

#### 3.1.4 完整示例

**Windows** Monitor H.264 (CPU):
```
( d3d12screencapturesrc monitor-index=0 ! videoconvert ! x264enc bitrate=4000 speed-preset=medium ! rtph264pay name=pay0 pt=96 )
```

**Windows** Monitor H.264 (GPU AMF):
```
( d3d12screencapturesrc monitor-index=0 ! videoconvert ! amfh264enc bitrate=4000 gop-size=30 ! rtph264pay name=pay0 pt=96 )
```

**macOS** Monitor H.264 (VideoToolbox):
```
( avfvideosrc display-index=0 ! videoconvert ! vtenc_h264 bitrate=4000 max-keyframe-distance=30 ! rtph264pay name=pay0 pt=96 )
```

**Linux** Monitor H.264 (VAAPI):
```
( ximagesrc monitor-index=0 ! videoconvert ! vaapih264enc bitrate=4000 keyframe-period=30 ! rtph264pay name=pay0 pt=96 )
```

---

## 4 GstPipelineManager (`manager.rs`)

### 4.1 数据结构

```rust
struct PipelineHandle {
    status: PipelineState,
    config: EncodeConfig,
    rtsp_path: String,           // "/screen-0"
    source: CaptureSource,
    encoder_used: String,        // "amfh264enc"
    is_gpu: bool,
}

pub struct GstPipelineManager {
    pipelines: Mutex<HashMap<String, PipelineHandle>>,
    rtsp_server: Mutex<Option<RtspServer>>,
    config: AppConfig,
}
```

### 4.2 方法实现

#### `new(config: AppConfig) -> Self`

```
pipelines: Mutex::new(HashMap::new())
rtsp_server: Mutex::new(None)
config
```

#### `ensure_rtsp_server(&self) -> AppResult<()>`

```
let mut guard = self.rtsp_server.lock().unwrap();
if guard.is_none():
    let server = RtspServer::new(self.config.rtsp_port)?;
    server.start()?;
    *guard = Some(server);
```

> **TOCTOU 安全**: 先加锁再检查，避免两个线程同时创建 RTSP Server。

#### `start_pipeline(&self, source: &CaptureSource, config: &EncodeConfig) -> AppResult<()>`

```
1. ensure_rtsp_server()
2. rtsp_path = format!("/{}", source.id)        // "/screen-0"
3. launch_str = build_launch_string(source, config)
4. rtsp_server.add_stream(&rtsp_path, &launch_str)
5. (encoder_used, is_gpu) = detect_encoder_info(config)
6. handle = PipelineHandle { status: Running, config, rtsp_path, source, encoder_used, is_gpu }
7. pipelines.lock().insert(source.id, handle)
```

#### `stop_pipeline(&self, source_id: &str) -> AppResult<()>`

```
1. let rtsp_path = pipelines.lock().remove(source_id)  // 释放 pipelines 锁
2. if rtsp_path 存在:
     rtsp_server.lock().remove_stream(&rtsp_path)       // 获取 rtsp_server 锁
```

> **死锁预防**: pipelines 锁在步骤 1 结束时释放，步骤 2 才获取 rtsp_server 锁。

#### `stop_all(&self) -> AppResult<()>`

```
收集所有 source_id → 逐个调用 stop_pipeline()
```

#### `get_status(&self, source_id: &str) -> Option<PipelineStatus>`

```
1. 在 pipelines 锁内克隆所需数据 (status, encoder_used, is_gpu, bitrate_kbps, rtsp_path)
2. 释放 pipelines 锁
3. get_rtsp_url_by_path(&rtsp_path)  // 获取 rtsp_server 锁
4. 构建 PipelineStatus { fps: 0.0, latency_ms: 0, ... }
```

> **fps 和 latency_ms 当前为占位值 (0)**。

#### `get_all_status(&self) -> Vec<PipelineStatus>`

```
收集所有 source_id → 逐个调用 get_status()
```

#### `update_config(&self, source_id: &str, config: &EncodeConfig) -> AppResult<()>`

```
1. 从 pipelines 中获取现有 source.clone()
2. stop_pipeline(source_id)
3. start_pipeline(&source, config)
```

> **配置更新 = 停止 + 重启**，因为 GStreamer Pipeline 不支持运行时参数热更新。

#### `get_rtsp_url(&self, source_id: &str) -> Option<String>`

```
1. 从 pipelines 获取 rtsp_path
2. get_rtsp_url_by_path(&rtsp_path)
```

#### `get_rtsp_url_by_path(&self, rtsp_path: &str) -> Option<String>` (私有)

```
rtsp_server.lock().as_ref().map(|s| s.rtsp_url(rtsp_path))
```

---

## 5 Pipeline 数据流图

```
┌─────────────────────────────────────────────────────────────┐
│                    GStreamer Pipeline                        │
│                                                              │
│  ┌────────────────────┐    ┌─────────────┐    ┌──────────┐  │
│  │  capture source    │───→│  encoder    │───→│ rtpXpay  │  │
│  │   (platform-specific)│   │             │    │          │  │
│  │ Win: d3d12screencap │    │ Win: AMF/MF │    │ rtph264  │  │
│  │ Mac: avfvideosrc   │    │ Mac: VT     │    │ rtph265  │  │
│  │ Lin: ximagesrc     │    │ Lin: VAAPI  │    │ pay0     │  │
│  └────────────────────┘    │ CPU: x264/5 │    └──────────┘  │
│          │                 └─────────────┘    name=pay0     │
│     videoconvert                              pt=96        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    ┌──────────────────┐
                    │  gst-rtsp-server │
                    │  port: 8554      │
                    │  shared: true    │
                    │  latency: 0      │
                    └──────────────────┘
                              │
                              ▼
                    rtsp://127.0.0.1:8554/screen-0
```
