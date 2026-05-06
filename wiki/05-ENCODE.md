# 05 — 编码模块

## 1 模块结构

```
encode/
├── mod.rs       → pub mod config; pub mod detector;
├── config.rs    → EncodeConfig, Codec, EncodeMode, RateControl, EncodePreset, Resolution
└── detector.rs  → GpuCapability, detect_gpu_capabilities()
```

## 2 编码配置

详见 [02-DATA-TYPES.md](./02-DATA-TYPES.md) §1.2。

## 3 GPU 检测 (`detector.rs`)

### 3.1 `detect_gpu_capabilities() -> GpuCapability`

```rust
pub fn detect_gpu_capabilities() -> GpuCapability {
    let mut cap = GpuCapability {
        has_amf: false, has_mf: false,
        has_vaapi: false, has_videotoolbox: false,
        amf_encoders: vec![], mf_encoders: vec![],
        vt_encoders: vec![], vaapi_encoders: vec![],
    };

    // 检测 AMD AMF 编码器
    for name in &["amfh264enc", "amfh265enc", "amfh264device2enc", "amfh265device2enc"] {
        if gstreamer::ElementFactory::find(name).is_some() {
            cap.has_amf = true;
            cap.amf_encoders.push(name.to_string());
        }
    }

    // 检测 Media Foundation 编码器
    for name in &["mfh264enc", "mfh264device3enc"] {
        if gstreamer::ElementFactory::find(name).is_some() {
            cap.has_mf = true;
            cap.mf_encoders.push(name.to_string());
        }
    }

    cap

    // macOS: 检测 VideoToolbox 编码器
    #[cfg(target_os = "macos")]
    {
        for name in &["vtenc_h264", "vtenc_h265"] {
            if gstreamer::ElementFactory::find(name).is_some() {
                cap.has_videotoolbox = true;
                cap.vt_encoders.push(name.to_string());
            }
        }
    }

    // Linux: 检测 VAAPI 编码器
    #[cfg(target_os = "linux")]
    {
        for name in &["vaapih264enc", "vaapih265enc"] {
            if gstreamer::ElementFactory::find(name).is_some() {
                cap.has_vaapi = true;
                cap.vaapi_encoders.push(name.to_string());
            }
        }
    }
}
```

**检测原理**: 通过 `gstreamer::ElementFactory::find(name)` 查询 GStreamer 元素工厂是否注册了该编码器。这依赖于 `GST_PLUGIN_PATH` 环境变量指向正确的 GStreamer 插件目录。

---

## 4 编码器选择策略 (`pipeline/gst_pipeline.rs`)

### 4.1 `build_encoder_element(config: &EncodeConfig) -> String`

```
match config.mode:
  Auto | GpuOnly:
    gpu_candidates = gpu_encoder_candidates(&config.codec)
    for encoder_name in gpu_candidates:
      if ElementFactory::find(encoder_name).is_some():
        return format_encoder_params(encoder_name, config)  // 使用第一个可用的 GPU 编码器
    if GpuOnly:
      log::warn!("GPU encoder requested but none available, using CPU fallback")
    log::warn!("GPU encoder not available, falling back to CPU")
    return build_cpu_encoder(config)
  
  CpuOnly:
    return build_cpu_encoder(config)
```

### 4.2 GPU 编码器候选列表

```rust
pub fn gpu_encoder_candidates(codec: &Codec) -> Vec<&'static str> {
    #[cfg(target_os = "windows")]
    match codec {
        Codec::H264 => vec!["amfh264enc", "amfh264device2enc", "mfh264enc", "mfh264device3enc"],
        Codec::H265 => vec!["amfh265enc", "amfh265device2enc"],
    }

    #[cfg(target_os = "macos")]
    match codec {
        Codec::H264 => vec!["vtenc_h264"],
        Codec::H265 => vec!["vtenc_h265"],
    }

    #[cfg(target_os = "linux")]
    match codec {
        Codec::H264 => vec!["vaapih264enc"],
        Codec::H265 => vec!["vaapih265enc"],
    }
}
```

**Windows 优先级**: AMF → AMF device2 → Media Foundation → MF device3
**macOS**: VideoToolbox (vtenc_h264 / vtenc_h265)
**Linux**: VAAPI (vaapih264enc / vaapih265enc)

**H.265 限制**: Windows MF 不提供 H.265 编码器；macOS/Linux 均支持 H.265。

### 4.3 CPU 编码器

```rust
fn build_cpu_encoder(config: &EncodeConfig) -> String {
    match config.codec {
        Codec::H264 => format!("x264enc bitrate={} speed-preset=medium ! rtph264pay", config.bitrate_kbps),
        Codec::H265 => format!("x265enc bitrate={} speed-preset=medium ! rtph265pay", config.bitrate_kbps),
    }
}
```

### 4.4 编码器参数格式

```rust
fn format_encoder_params(encoder_name: &str, config: &EncodeConfig) -> String {
    let payloader = match config.codec {
        Codec::H264 => "rtph264pay",
        Codec::H265 => "rtph265pay",
    };
    let params = match encoder_name {
        "amfh264enc" | "amfh264device2enc" | "amfh265enc" | "amfh265device2enc"
        | "mfh264enc" | "mfh264device3enc" => {
            format!("{} bitrate={} gop-size={}", encoder_name, config.bitrate_kbps, config.gop_size)
        }
        "vtenc_h264" | "vtenc_h265" => {
            format!("{} bitrate={} max-keyframe-distance={}", encoder_name, config.bitrate_kbps, config.gop_size)
        }
        "vaapih264enc" | "vaapih265enc" => {
            format!("{} bitrate={} keyframe-period={}", encoder_name, config.bitrate_kbps, config.gop_size)
        }
        _ => format!("{} bitrate={}", encoder_name, config.bitrate_kbps),
    };
    format!("{} ! {}", params, payloader)
}
```

### 4.5 可用编码器检测

```rust
pub fn detect_available_encoders() -> Vec<String> {
    let mut available = Vec::new();

    // Windows GPU encoders
    #[cfg(target_os = "windows")]
    let candidates: &[&str] = &[
        "amfh264enc", "amfh265enc", "amfh264device2enc", "amfh265device2enc",
        "mfh264enc", "mfh264device3enc",
        "x264enc", "x265enc", "openh264enc",
    ];

    // macOS GPU encoders
    #[cfg(target_os = "macos")]
    let candidates: &[&str] = &["vtenc_h264", "vtenc_h265", "x264enc", "x265enc"];

    // Linux GPU encoders
    #[cfg(target_os = "linux")]
    let candidates: &[&str] = &["vaapih264enc", "vaapih265enc", "x264enc", "x265enc"];

    for name in candidates {
        if gstreamer::ElementFactory::find(name).is_some() {
            available.push(name.to_string());
        }
    }
    available
}
```

### 4.6 编码器信息检测 (`pipeline/manager.rs`)

```rust
fn detect_encoder_info(config: &EncodeConfig) -> (String, bool) {
    match config.mode {
        EncodeMode::Auto | EncodeMode::GpuOnly => {
            let candidates = gst_pipeline::gpu_encoder_candidates(&config.codec);
            for name in &candidates {
                if gstreamer::ElementFactory::find(name).is_some() {
                    return (name.to_string(), true);  // (编码器名, is_gpu=true)
                }
            }
            // CPU fallback
            match config.codec {
                Codec::H264 => ("x264enc".to_string(), false),
                Codec::H265 => ("x265enc".to_string(), false),
            }
        }
        EncodeMode::CpuOnly => match config.codec {
            Codec::H264 => ("x264enc".to_string(), false),
            Codec::H265 => ("x265enc".to_string(), false),
        },
    }
}
```

---

## 5 编码器参数汇总

|| 编码器 | 类型 | 支持编解码 | 参数格式 |
||--------|------|-----------|---------|
|| `amfh264enc` | GPU (AMD AMF) | H.264 | `bitrate=N gop-size=M` |
|| `amfh264device2enc` | GPU (AMD AMF D3D11) | H.264 | `bitrate=N gop-size=M` |
|| `amfh265enc` | GPU (AMD AMF) | H.265 | `bitrate=N gop-size=M` |
|| `amfh265device2enc` | GPU (AMD AMF D3D11) | H.265 | `bitrate=N gop-size=M` |
|| `mfh264enc` | GPU (Media Foundation) | H.264 | `bitrate=N gop-size=M` |
|| `mfh264device3enc` | GPU (Media Foundation D3D11) | H.264 | `bitrate=N gop-size=M` |
|| `x264enc` | CPU | H.264 | `bitrate=N speed-preset=medium` |
|| `x265enc` | CPU | H.265 | `bitrate=N speed-preset=medium` |
|| `openh264enc` | CPU | H.264 | `bitrate=N` (仅 bitrate) |
|| `vtenc_h264` | GPU (VideoToolbox) | H.264 | `bitrate=N max-keyframe-distance=M` |
|| `vtenc_h265` | GPU (VideoToolbox) | H.265 | `bitrate=N max-keyframe-distance=M` |
|| `vaapih264enc` | GPU (VAAPI) | H.264 | `bitrate=N keyframe-period=M` |
|| `vaapih265enc` | GPU (VAAPI) | H.265 | `bitrate=N keyframe-period=M` |
