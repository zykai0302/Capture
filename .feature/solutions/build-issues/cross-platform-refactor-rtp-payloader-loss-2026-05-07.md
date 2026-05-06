---
title: "Cross-Platform Refactoring Silently Drops RTP Payloader from GStreamer Pipeline String"
date: 2026-05-07
category: build-issues
module: src-tauri/src/pipeline
problem_type: integration_issue
component: build_encoder_element, build_cpu_encoder, format_encoder_params, GStreamer launch string
severity: high
tags: [gstreamer, pipeline, rtsp, rtp, payloader, cross-platform, refactoring, conditional-compilation, cfg-target-os]
---

## Problem

When refactoring `gst_pipeline.rs` for cross-platform support (Phase 8), the `build_encoder_element()` function was introduced to replace inline encoder+payloader strings. However, the new function only returned the encoder name and parameters — the RTP payloader (`rtph264pay` / `rtph265pay`) was silently dropped.

This is particularly insidious because:
1. The application compiles without errors
2. GStreamer does not immediately fail — the pipeline constructs but RTSP clients receive no media
3. Unit tests that check for `rtph264pay` / `rtph265pay` catch the bug, but only if they exist and are run

## Symptoms

- Rust unit tests `build_launch_string_h264_uses_rtph264pay` and `build_launch_string_h265_uses_rtph265pay` fail with:
  ```
  assertion failed: result.contains("rtph264pay")
  assertion failed: result.contains("rtph265pay")
  ```
- Runtime: RTSP stream appears to start but VLC/ffplay cannot receive media data
- Pipeline launch string looks like `x264enc bitrate=4000 speed-preset=medium name=pay0 pt=96` instead of `x264enc bitrate=4000 speed-preset=medium ! rtph264pay name=pay0 pt=96`
- `name=pay0` is present but attached to the encoder, not the payloader — gst-rtsp-server expects a payloader element named `pay0`

## What Didn't Work

### Original Code (Before Cross-Platform Refactor)

```rust
pub fn build_launch_string(source: &CaptureSource, config: &EncodeConfig) -> String {
    let encoder_and_pay = match config.codec {
        Codec::H264 => format!("x264enc bitrate={} ! rtph264pay", config.bitrate_kbps),
        Codec::H265 => format!("x265enc bitrate={} ! rtph265pay", config.bitrate_kbps),
    };
    // ...
}
```

This worked but was Windows-only and hardcoded CPU encoders.

### Broken Refactor

```rust
fn build_encoder_element(config: &EncodeConfig) -> String {
    // GPU path...
    build_cpu_encoder(config)
}

fn build_cpu_encoder(config: &EncodeConfig) -> String {
    match config.codec {
        Codec::H264 => format!("x264enc bitrate={} speed-preset=medium", config.bitrate_kbps),
        //                    ^^^ NO rtph264pay!
        Codec::H265 => format!("x265enc bitrate={} speed-preset=medium", config.bitrate_kbps),
        //                    ^^^ NO rtph265pay!
    }
}

fn format_encoder_params(encoder_name: &str, config: &EncodeConfig) -> String {
    // GPU encoder params only — NO payloader suffix
    format!("{} bitrate={} gop-size={}", encoder_name, config.bitrate_kbps, config.gop_size)
}
```

The refactoring decomposed the encoder+payloader into separate concerns (encoder selection, encoder params) but forgot to reassemble the complete `encoder ! payloader` chain.

## Solution

The RTP payloader is an inseparable part of the GStreamer RTSP pipeline string. It must be appended by both `build_cpu_encoder()` and `format_encoder_params()`:

### Fixed `build_cpu_encoder`

```rust
fn build_cpu_encoder(config: &EncodeConfig) -> String {
    match config.codec {
        Codec::H264 => format!(
            "x264enc bitrate={} speed-preset=medium ! rtph264pay",
            config.bitrate_kbps
        ),
        Codec::H265 => format!(
            "x265enc bitrate={} speed-preset=medium ! rtph265pay",
            config.bitrate_kbps
        ),
    }
}
```

### Fixed `format_encoder_params` (GPU path)

```rust
fn format_encoder_params(encoder_name: &str, config: &EncodeConfig) -> String {
    let payloader = match config.codec {
        Codec::H264 => "rtph264pay",
        Codec::H265 => "rtph265pay",
    };
    let params = match encoder_name {
        "amfh264enc" | "amfh264device2enc" | "amfh265enc" | "amfh265device2enc" => {
            format!("{} bitrate={} gop-size={}", encoder_name, config.bitrate_kbps, config.gop_size)
        }
        "mfh264enc" | "mfh264device3enc" => { /* ... */ }
        "vtenc_h264" | "vtenc_h265" => { /* ... */ }
        "vaapih264enc" | "vaapih265enc" => { /* ... */ }
        _ => format!("{} bitrate={}", encoder_name, config.bitrate_kbps),
    };
    format!("{} ! {}", params, payloader)
}
```

### Updated Test Assertions (6 tests)

All `format_encoder_params` and `build_cpu_encoder` tests needed assertion updates:

| Test | Old Expected | New Expected |
|------|-------------|-------------|
| `format_amfh264enc_params` | `amfh264enc bitrate=6000 gop-size=60` | `amfh264enc bitrate=6000 gop-size=60 ! rtph264pay` |
| `format_vtenc_h264_params` | `vtenc_h264 bitrate=5000 max-keyframe-distance=30` | `vtenc_h264 bitrate=5000 max-keyframe-distance=30 ! rtph264pay` |
| `format_vaapih264enc_params` | `vaapih264enc bitrate=4000 keyframe-period=45` | `vaapih264enc bitrate=4000 keyframe-period=45 ! rtph264pay` |
| `format_unknown_encoder_params` | `somecustomenc bitrate=3000` | `somecustomenc bitrate=3000 ! rtph264pay` |
| `cpu_encoder_h264` | `x264enc bitrate=4000 speed-preset=medium` | `x264enc bitrate=4000 speed-preset=medium ! rtph264pay` |
| `cpu_encoder_h265` | `x265enc bitrate=8000 speed-preset=medium` | `x265enc bitrate=8000 speed-preset=medium ! rtph265pay` |

## Why This Works

1. **The RTP payloader is required by gst-rtsp-server**: The `name=pay0` attribute must be on a payloader element (`rtph264pay`/`rtph265pay`), not on an encoder. Without the payloader, gst-rtsp-server cannot create proper SDP or RTP packets.

2. **The payloader depends on codec, not encoder**: Whether using GPU (`amfh264enc`, `vtenc_h264`, `vaapih264enc`) or CPU (`x264enc`), the RTP payloader is always `rtph264pay` for H.264 and `rtph265pay` for H.265. This makes `config.codec` the correct source for payloader selection.

3. **Single responsibility preserved**: `format_encoder_params` handles encoder-specific parameters, then appends the codec-determined payloader. `build_cpu_encoder` does the same for CPU encoders. Both produce a complete `encoder ! payloader` string that plugs directly into the launch string template.

## Prevention

### Rule: When decomposing a function that produces a GStreamer pipeline fragment, always verify the complete pipeline string

Before refactoring:
```
Source ! Encoder ! Payloader name=pay0
```

After refactoring, each sub-function must produce a string that, when combined, still produces the complete chain. Add a comment in the function signature:

```rust
/// Build encoder + RTP payloader string.
/// Returns "encoder_name params ! rtpXxxpay" — the payloader is REQUIRED.
fn build_encoder_element(config: &EncodeConfig) -> String {
```

### Rule: GStreamer pipeline string tests must check both encoder AND payloader

When writing tests for pipeline string builders, always assert on the payloader:

```rust
#[test]
fn build_launch_string_h264_uses_rtph264pay() {
    let result = build_launch_string(&source, &config);
    assert!(result.contains("rtph264pay"));  // Payloader present
    assert!(result.contains("name=pay0"));    // pay0 on the payloader
}
```

### Rule: Cross-platform refactoring of pipeline strings requires full test suite re-run

`#[cfg(target_os)]` conditional compilation can hide entire code branches from compilation. After any cross-platform refactor, run `cargo test` on the primary platform to verify that non-`#[cfg]` code still produces correct output.

## Related Issues

- `.feature/solutions/build-issues/gstreamer-rtsp-runtime-environment-2026-05-06.md` — Original `build_launch_string` hardcoded CPU encoder issue (related but different root cause)
- `src-tauri/src/pipeline/gst_pipeline.rs` — Fixed encoder + payloader string generation
- `.feature/spec/backend/quality-guidelines.md` — Cross-platform `#[cfg(target_os)]` pattern documentation
