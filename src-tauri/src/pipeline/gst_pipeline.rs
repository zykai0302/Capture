use crate::encode::config::{Codec, EncodeConfig, EncodeMode};
use crate::capture::source::{CaptureSource, SourceType};

/// Build a GStreamer launch string for screen capture + encode + RTP pay
/// Returns a complete pipeline string for gst-rtsp-server
/// Note: RTSPMediaFactory automatically handles pay0 naming
pub fn build_launch_string(source: &CaptureSource, config: &EncodeConfig) -> String {
    let encoder_and_pay = build_encoder_element(config);
    let capture_element = build_capture_element(source);

    match source.source_type {
        SourceType::Monitor => {
            let monitor_idx: i32 = source
                .id
                .strip_prefix("screen-")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);

            #[cfg(target_os = "windows")]
            {
                format!(
                    "( {} monitor-index={} ! videoconvert ! {} name=pay0 pt=96 )",
                    capture_element, monitor_idx, encoder_and_pay
                )
            }

            #[cfg(target_os = "macos")]
            {
                format!(
                    "( {} display-index={} ! videoconvert ! {} name=pay0 pt=96 )",
                    capture_element, monitor_idx, encoder_and_pay
                )
            }

            #[cfg(target_os = "linux")]
            {
                format!(
                    "( {} monitor-index={} ! videoconvert ! {} name=pay0 pt=96 )",
                    capture_element, monitor_idx, encoder_and_pay
                )
            }
        }
        SourceType::Window => {
            let hwnd: i64 = source
                .id
                .strip_prefix("window-")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0) as i64;

            #[cfg(target_os = "windows")]
            {
                format!(
                    "( {} window-handle={} ! videoconvert ! {} name=pay0 pt=96 )",
                    capture_element, hwnd, encoder_and_pay
                )
            }

            #[cfg(not(target_os = "windows"))]
            {
                // macOS: avfvicesrc with window-id
                // Linux: ximagesrc with xid
                format!(
                    "( {} window-id={} ! videoconvert ! {} name=pay0 pt=96 )",
                    capture_element, hwnd, encoder_and_pay
                )
            }
        }
    }
}

/// Build capture source element based on platform
fn build_capture_element(_source: &CaptureSource) -> &'static str {
    #[cfg(target_os = "windows")]
    {
        // d3d12screencapturesrc — d3d11screencapturesrc fails in RTSP media threads
        "d3d12screencapturesrc"
    }

    #[cfg(target_os = "macos")]
    {
        // AVFoundation video source for screen/window capture
        "avfvideosrc"
    }

    #[cfg(target_os = "linux")]
    {
        // X11 screen capture (Wayland uses pipewiresrc, future)
        "ximagesrc"
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

/// Get GPU encoder candidate names (platform-specific)
pub fn gpu_encoder_candidates(codec: &Codec) -> Vec<&'static str> {
    #[cfg(target_os = "windows")]
    {
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

    #[cfg(target_os = "macos")]
    {
        match codec {
            // VideoToolbox hardware encoders
            Codec::H264 => vec!["vtenc_h264"],
            Codec::H265 => vec!["vtenc_h265"],
        }
    }

    #[cfg(target_os = "linux")]
    {
        match codec {
            // VAAPI hardware encoders
            Codec::H264 => vec!["vaapih264enc"],
            Codec::H265 => vec!["vaapih265enc"],
        }
    }
}

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

fn format_encoder_params(encoder_name: &str, config: &EncodeConfig) -> String {
    let payloader = match config.codec {
        Codec::H264 => "rtph264pay",
        Codec::H265 => "rtph265pay",
    };
    let params = match encoder_name {
        // Windows AMF encoders
        "amfh264enc" | "amfh264device2enc" | "amfh265enc" | "amfh265device2enc" => {
            format!(
                "{} bitrate={} gop-size={}",
                encoder_name, config.bitrate_kbps, config.gop_size
            )
        }
        // Windows Media Foundation encoders
        "mfh264enc" | "mfh264device3enc" => {
            format!(
                "{} bitrate={} gop-size={}",
                encoder_name, config.bitrate_kbps, config.gop_size
            )
        }
        // macOS VideoToolbox encoders
        "vtenc_h264" | "vtenc_h265" => {
            format!(
                "{} bitrate={} max-keyframe-distance={}",
                encoder_name, config.bitrate_kbps, config.gop_size
            )
        }
        // Linux VAAPI encoders
        "vaapih264enc" | "vaapih265enc" => {
            format!(
                "{} bitrate={} keyframe-period={}",
                encoder_name, config.bitrate_kbps, config.gop_size
            )
        }
        _ => format!("{} bitrate={}", encoder_name, config.bitrate_kbps),
    };
    format!("{} ! {}", params, payloader)
}

/// Detect which GPU encoders are available (platform-aware)
pub fn detect_available_encoders() -> Vec<String> {
    let mut available = Vec::new();

    let candidates: Vec<&str> = {
        #[cfg(target_os = "windows")]
        {
            vec![
                "amfh264enc", "amfh265enc",
                "amfh264device2enc", "amfh265device2enc",
                "mfh264enc", "mfh264device3enc",
                "x264enc", "x265enc", "openh264enc",
            ]
        }

        #[cfg(target_os = "macos")]
        {
            vec![
                "vtenc_h264", "vtenc_h265",
                "x264enc", "x265enc",
            ]
        }

        #[cfg(target_os = "linux")]
        {
            vec![
                "vaapih264enc", "vaapih265enc",
                "x264enc", "x265enc", "openh264enc",
            ]
        }
    };

    for name in &candidates {
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
    fn gpu_encoder_candidates_h264_not_empty() {
        let candidates = gpu_encoder_candidates(&Codec::H264);
        assert!(!candidates.is_empty());
    }

    #[test]
    fn gpu_encoder_candidates_h265_not_empty() {
        let candidates = gpu_encoder_candidates(&Codec::H265);
        assert!(!candidates.is_empty());
    }

    #[test]
    fn h265_has_fewer_or_equal_candidates_than_h264() {
        let h264 = gpu_encoder_candidates(&Codec::H264);
        let h265 = gpu_encoder_candidates(&Codec::H265);
        assert!(h265.len() <= h264.len());
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
        assert_eq!(result, "x264enc bitrate=4000 speed-preset=medium ! rtph264pay");
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
        assert_eq!(result, "x265enc bitrate=8000 speed-preset=medium ! rtph265pay");
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
        assert_eq!(result, "amfh264enc bitrate=6000 gop-size=60 ! rtph264pay");
    }

    #[test]
    fn format_vtenc_h264_params() {
        let config = EncodeConfig {
            codec: Codec::H264,
            bitrate_kbps: 5000,
            gop_size: 30,
            ..EncodeConfig::default()
        };
        let result = format_encoder_params("vtenc_h264", &config);
        assert_eq!(result, "vtenc_h264 bitrate=5000 max-keyframe-distance=30 ! rtph264pay");
    }

    #[test]
    fn format_vaapih264enc_params() {
        let config = EncodeConfig {
            codec: Codec::H264,
            bitrate_kbps: 4000,
            gop_size: 45,
            ..EncodeConfig::default()
        };
        let result = format_encoder_params("vaapih264enc", &config);
        assert_eq!(result, "vaapih264enc bitrate=4000 keyframe-period=45 ! rtph264pay");
    }

    #[test]
    fn format_unknown_encoder_params() {
        let config = EncodeConfig {
            bitrate_kbps: 3000,
            ..EncodeConfig::default()
        };
        let result = format_encoder_params("somecustomenc", &config);
        assert_eq!(result, "somecustomenc bitrate=3000 ! rtph264pay");
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
        // Should contain some form of index reference
        assert!(result.contains("2"));
    }

    #[test]
    fn build_launch_string_window_extracts_handle() {
        let source = make_window("window-12345678");
        let config = EncodeConfig {
            mode: EncodeMode::CpuOnly,
            ..EncodeConfig::default()
        };
        let result = build_launch_string(&source, &config);
        assert!(result.contains("12345678"));
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

    // === Capture source element test ===

    #[test]
    fn build_launch_string_contains_videoconvert() {
        let source = make_monitor("screen-0");
        let config = EncodeConfig {
            mode: EncodeMode::CpuOnly,
            ..EncodeConfig::default()
        };
        let result = build_launch_string(&source, &config);
        assert!(result.contains("videoconvert"));
    }

    #[test]
    fn build_launch_string_contains_pay0() {
        let source = make_monitor("screen-0");
        let config = EncodeConfig {
            mode: EncodeMode::CpuOnly,
            ..EncodeConfig::default()
        };
        let result = build_launch_string(&source, &config);
        assert!(result.contains("name=pay0"));
    }
}
