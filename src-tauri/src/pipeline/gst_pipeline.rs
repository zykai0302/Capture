use crate::encode::config::{Codec, EncodeConfig, EncodeMode};
use crate::capture::source::{CaptureSource, SourceType};

/// Build a GStreamer launch string for screen capture + encode + RTP pay
/// Returns a complete pipeline string for gst-rtsp-server
/// Note: RTSPMediaFactory automatically handles pay0 naming
pub fn build_launch_string(source: &CaptureSource, config: &EncodeConfig) -> String {
    let encoder_and_pay = match config.codec {
        Codec::H264 => format!("x264enc bitrate={} ! rtph264pay", config.bitrate_kbps),
        Codec::H265 => format!("x265enc bitrate={} ! rtph265pay", config.bitrate_kbps),
    };

    match source.source_type {
        SourceType::Monitor => {
            let monitor_idx: i32 = source
                .id
                .strip_prefix("screen-")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            // Use d3d12screencapturesrc — d3d11screencapturesrc fails in
            // RTSP media threads because D3D11 device init fails there.
            format!(
                "( d3d12screencapturesrc monitor-index={} ! videoconvert ! {} name=pay0 pt=96 )",
                monitor_idx, encoder_and_pay
            )
        }
        SourceType::Window => {
            let hwnd: i64 = source
                .id
                .strip_prefix("window-")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0) as i64;
            format!(
                "( d3d12screencapturesrc window-handle={} ! videoconvert ! {} name=pay0 pt=96 )",
                hwnd, encoder_and_pay
            )
        }
    }
}

/// Build encoder element string based on config and available hardware
fn build_encoder_element(config: &EncodeConfig) -> String {
    match config.mode {
        EncodeMode::Auto | EncodeMode::GpuOnly => {
            // Try GPU encoders first
            let gpu_candidates = gpu_encoder_candidates(&config.codec);
            for encoder_name in &gpu_candidates {
                if gstreamer::ElementFactory::find(encoder_name).is_some() {
                    log::info!("Using GPU encoder: {}", encoder_name);
                    return format_encoder_params(encoder_name, config);
                }
            }
            if matches!(config.mode, EncodeMode::GpuOnly) {
                log::warn!("GPU encoder requested but none available, using CPU fallback");
            }
            log::warn!("GPU encoder not available, falling back to CPU");
            build_cpu_encoder(config)
        }
        EncodeMode::CpuOnly => build_cpu_encoder(config),
    }
}

/// Get GPU encoder candidate names (public for manager to use)
pub fn gpu_encoder_candidates(codec: &Codec) -> Vec<&'static str> {
    match codec {
        Codec::H264 => vec![
            "amfh264enc",
            "amfh264device2enc",
            "mfh264enc",
            "mfh264device3enc",
        ],
        Codec::H265 => vec!["amfh265enc", "amfh265device2enc"],
    }
}

fn build_cpu_encoder(config: &EncodeConfig) -> String {
    match config.codec {
        Codec::H264 => format!(
            "x264enc bitrate={} speed-preset=medium",
            config.bitrate_kbps
        ),
        Codec::H265 => format!(
            "x265enc bitrate={} speed-preset=medium",
            config.bitrate_kbps
        ),
    }
}

fn format_encoder_params(encoder_name: &str, config: &EncodeConfig) -> String {
    match encoder_name {
        "amfh264enc" | "amfh264device2enc" | "amfh265enc" | "amfh265device2enc" => {
            format!(
                "{} bitrate={} gop-size={}",
                encoder_name, config.bitrate_kbps, config.gop_size
            )
        }
        "mfh264enc" | "mfh264device3enc" => {
            format!(
                "{} bitrate={} gop-size={}",
                encoder_name, config.bitrate_kbps, config.gop_size
            )
        }
        _ => format!("{} bitrate={}", encoder_name, config.bitrate_kbps),
    }
}

/// Detect which GPU encoders are available
pub fn detect_available_encoders() -> Vec<String> {
    let mut available = Vec::new();

    for name in &[
        "amfh264enc",
        "amfh265enc",
        "amfh264device2enc",
        "amfh265device2enc",
        "mfh264enc",
        "mfh264device3enc",
        "x264enc",
        "x265enc",
        "openh264enc",
    ] {
        if gstreamer::ElementFactory::find(name).is_some() {
            available.push(name.to_string());
        }
    }

    available
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_monitor(id: &str) -> CaptureSource {
        CaptureSource {
            id: id.to_string(),
            name: "Monitor".to_string(),
            source_type: SourceType::Monitor,
            width: 1920,
            height: 1080,
            is_streaming: false,
            rtsp_url: None,
        }
    }

    fn make_window(id: &str) -> CaptureSource {
        CaptureSource {
            id: id.to_string(),
            name: "Window".to_string(),
            source_type: SourceType::Window,
            width: 800,
            height: 600,
            is_streaming: false,
            rtsp_url: None,
        }
    }

    // === GPU encoder candidate tests (pure logic, no GStreamer needed) ===

    #[test]
    fn gpu_encoder_candidates_h264() {
        let candidates = gpu_encoder_candidates(&Codec::H264);
        assert_eq!(candidates, vec!["amfh264enc", "amfh264device2enc", "mfh264enc", "mfh264device3enc"]);
    }

    #[test]
    fn gpu_encoder_candidates_h265() {
        let candidates = gpu_encoder_candidates(&Codec::H265);
        assert_eq!(candidates, vec!["amfh265enc", "amfh265device2enc"]);
    }

    #[test]
    fn h265_has_fewer_gpu_candidates_than_h264() {
        let h264 = gpu_encoder_candidates(&Codec::H264);
        let h265 = gpu_encoder_candidates(&Codec::H265);
        assert!(h265.len() < h264.len());
    }

    // === CPU encoder tests (pure logic) ===

    #[test]
    fn cpu_encoder_h264() {
        let config = EncodeConfig {
            mode: EncodeMode::CpuOnly,
            codec: Codec::H264,
            bitrate_kbps: 4000,
            ..EncodeConfig::default()
        };
        let result = build_cpu_encoder(&config);
        assert_eq!(result, "x264enc bitrate=4000 speed-preset=medium");
    }

    #[test]
    fn cpu_encoder_h265() {
        let config = EncodeConfig {
            mode: EncodeMode::CpuOnly,
            codec: Codec::H265,
            bitrate_kbps: 8000,
            ..EncodeConfig::default()
        };
        let result = build_cpu_encoder(&config);
        assert_eq!(result, "x265enc bitrate=8000 speed-preset=medium");
    }

    // === format_encoder_params tests ===

    #[test]
    fn format_amfh264enc_params() {
        let config = EncodeConfig {
            codec: Codec::H264,
            bitrate_kbps: 6000,
            gop_size: 60,
            ..EncodeConfig::default()
        };
        let result = format_encoder_params("amfh264enc", &config);
        assert_eq!(result, "amfh264enc bitrate=6000 gop-size=60");
    }

    #[test]
    fn format_amfh265enc_params() {
        let config = EncodeConfig {
            codec: Codec::H265,
            bitrate_kbps: 8000,
            gop_size: 30,
            ..EncodeConfig::default()
        };
        let result = format_encoder_params("amfh265enc", &config);
        assert_eq!(result, "amfh265enc bitrate=8000 gop-size=30");
    }

    #[test]
    fn format_mfh264enc_params() {
        let config = EncodeConfig {
            codec: Codec::H264,
            bitrate_kbps: 5000,
            gop_size: 45,
            ..EncodeConfig::default()
        };
        let result = format_encoder_params("mfh264enc", &config);
        assert_eq!(result, "mfh264enc bitrate=5000 gop-size=45");
    }

    #[test]
    fn format_unknown_encoder_params() {
        let config = EncodeConfig {
            bitrate_kbps: 3000,
            ..EncodeConfig::default()
        };
        let result = format_encoder_params("somecustomenc", &config);
        assert_eq!(result, "somecustomenc bitrate=3000");
    }

    // === Monitor index parsing tests ===

    #[test]
    fn build_launch_string_monitor_extracts_index() {
        let source = make_monitor("screen-2");
        let config = EncodeConfig {
            mode: EncodeMode::CpuOnly,
            ..EncodeConfig::default()
        };
        let result = build_launch_string(&source, &config);
        assert!(result.contains("monitor-index=2"));
    }

    #[test]
    fn build_launch_string_monitor_default_index() {
        let source = CaptureSource {
            id: "invalid-id".to_string(),
            name: "Bad Monitor".to_string(),
            source_type: SourceType::Monitor,
            width: 1920,
            height: 1080,
            is_streaming: false,
            rtsp_url: None,
        };
        let config = EncodeConfig {
            mode: EncodeMode::CpuOnly,
            ..EncodeConfig::default()
        };
        let result = build_launch_string(&source, &config);
        // Should default to monitor-index=0 when id doesn't match "screen-N"
        assert!(result.contains("monitor-index=0"));
    }

    #[test]
    fn build_launch_string_window_extracts_handle() {
        let source = make_window("window-12345678");
        let config = EncodeConfig {
            mode: EncodeMode::CpuOnly,
            ..EncodeConfig::default()
        };
        let result = build_launch_string(&source, &config);
        assert!(result.contains("window-handle=12345678"));
    }

    #[test]
    fn build_launch_string_window_default_handle() {
        let source = CaptureSource {
            id: "invalid-id".to_string(),
            name: "Bad Window".to_string(),
            source_type: SourceType::Window,
            width: 800,
            height: 600,
            is_streaming: false,
            rtsp_url: None,
        };
        let config = EncodeConfig {
            mode: EncodeMode::CpuOnly,
            ..EncodeConfig::default()
        };
        let result = build_launch_string(&source, &config);
        assert!(result.contains("window-handle=0"));
    }

    // === RTP payloader tests ===

    #[test]
    fn build_launch_string_h264_uses_rtph264pay() {
        let source = make_monitor("screen-0");
        let config = EncodeConfig {
            codec: Codec::H264,
            mode: EncodeMode::CpuOnly,
            ..EncodeConfig::default()
        };
        let result = build_launch_string(&source, &config);
        assert!(result.contains("rtph264pay"));
    }

    #[test]
    fn build_launch_string_h265_uses_rtph265pay() {
        let source = make_monitor("screen-0");
        let config = EncodeConfig {
            codec: Codec::H265,
            mode: EncodeMode::CpuOnly,
            ..EncodeConfig::default()
        };
        let result = build_launch_string(&source, &config);
        assert!(result.contains("rtph265pay"));
    }

    // === Capture source type tests ===

    #[test]
    fn build_launch_string_monitor_uses_d3d12_screencapture() {
        let source = make_monitor("screen-0");
        let config = EncodeConfig {
            mode: EncodeMode::CpuOnly,
            ..EncodeConfig::default()
        };
        let result = build_launch_string(&source, &config);
        assert!(result.contains("d3d12screencapturesrc"));
        assert!(result.contains("monitor-index=0"));
    }

    #[test]
    fn build_launch_string_window_uses_d3d12_screencapture() {
        let source = make_window("window-100");
        let config = EncodeConfig {
            mode: EncodeMode::CpuOnly,
            ..EncodeConfig::default()
        };
        let result = build_launch_string(&source, &config);
        assert!(result.contains("d3d12screencapturesrc"));
        assert!(result.contains("window-handle=100"));
    }
}
