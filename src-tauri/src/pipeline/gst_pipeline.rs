use crate::encode::config::{Codec, EncodeConfig, EncodeMode};
use crate::capture::source::{CaptureSource, SourceType};
use gstreamer::prelude::*;

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{BOOL, LPARAM, RECT};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, MONITORENUMPROC, MONITORINFOEXW, HMONITOR, HDC};

/// Build a GStreamer launch string for screen capture + encode + RTP pay
/// Returns a complete pipeline string for gst-rtsp-server
/// Note: RTSPMediaFactory automatically handles pay0 naming
pub fn build_launch_string(source: &CaptureSource, config: &EncodeConfig) -> String {
    let encoder_and_pay = build_encoder_element(config);
    let capture_element = build_capture_element(source);

    // Detect which capture source we're using and build appropriate pipeline
    // D3D11/D3D12 capture sources output GPU memory, need download before CPU encoder
    let uses_d3d11 = capture_element.starts_with("d3d11screencapturesrc");
    let uses_d3d12 = capture_element.starts_with("d3d12screencapturesrc");

    let pipeline = if uses_d3d11 {
        // d3d11screencapsrc -> d3d11colorconvert -> d3d11download -> encoder
        format!(
            "( {} ! d3d11colorconvert ! d3d11download ! {} name=pay0 pt=96 )",
            capture_element, encoder_and_pay
        )
    } else if uses_d3d12 {
        // d3d12screencapsrc -> d3d12colorconvert -> d3d12download -> encoder
        format!(
            "( {} ! d3d12colorconvert ! d3d12download ! {} name=pay0 pt=96 )",
            capture_element, encoder_and_pay
        )
    } else {
        // Generic fallback (videoconvert for CPU-based sources)
        format!(
            "( {} ! videoconvert ! {} name=pay0 pt=96 )",
            capture_element, encoder_and_pay
        )
    };

    log::info!("Built RTSP pipeline: {}", pipeline);
    pipeline
}

/// Validate a pipeline launch string by attempting to create it (without starting).
/// Returns Ok(()) if the pipeline can be constructed, Err with details otherwise.
pub fn validate_pipeline_launch(launch_str: &str) -> Result<(), String> {
    match gstreamer::parse::launch(launch_str) {
        Ok(_pipeline) => {
            log::info!("Pipeline validation succeeded for: {}", launch_str);
            Ok(())
        }
        Err(e) => {
            let msg = format!("Pipeline validation failed: {} (launch: {})", e, launch_str);
            log::error!("{}", msg);
            Err(msg)
        }
    }
}

/// Build a DXGI+crop fallback pipeline for window capture.
/// Used when WGC mode fails for a specific window.
pub fn build_dxgi_crop_launch_string(source: &CaptureSource, config: &EncodeConfig) -> String {
    let encoder_and_pay = build_encoder_element(config);
    let capture_element = build_dxgi_crop_capture_element(source);

    let uses_d3d11 = capture_element.starts_with("d3d11screencapturesrc");
    let uses_d3d12 = capture_element.starts_with("d3d12screencapturesrc");

    let pipeline = if uses_d3d11 {
        format!(
            "( {} ! d3d11colorconvert ! d3d11download ! {} name=pay0 pt=96 )",
            capture_element, encoder_and_pay
        )
    } else if uses_d3d12 {
        format!(
            "( {} ! d3d12colorconvert ! d3d12download ! {} name=pay0 pt=96 )",
            capture_element, encoder_and_pay
        )
    } else {
        format!(
            "( {} ! videoconvert ! {} name=pay0 pt=96 )",
            capture_element, encoder_and_pay
        )
    };

    log::info!("Built DXGI+crop fallback RTSP pipeline: {}", pipeline);
    pipeline
}

/// Build DXGI+crop capture element for window (no WGC, used as fallback)
fn build_dxgi_crop_capture_element(source: &CaptureSource) -> String {
    #[cfg(target_os = "windows")]
    {
        let (monitor_index, mon_x, mon_y) = get_monitor_info_for_window(source.x, source.y);
        let crop_x = (source.x - mon_x).max(0) as u32;
        let crop_y = (source.y - mon_y).max(0) as u32;

        // Use monitor-handle if available to avoid index mapping issues
        if source.handle != 0 {
            if gstreamer::ElementFactory::find("d3d11screencapturesrc").is_some() {
                format!(
                    "d3d11screencapturesrc monitor-handle={} crop-x={} crop-y={} crop-width={} crop-height={}",
                    source.handle, crop_x, crop_y, source.width, source.height
                )
            } else if gstreamer::ElementFactory::find("d3d12screencapturesrc").is_some() {
                format!(
                    "d3d12screencapturesrc monitor-handle={} crop-x={} crop-y={} crop-width={} crop-height={}",
                    source.handle, crop_x, crop_y, source.width, source.height
                )
            } else {
                format!(
                    "d3d11screencapturesrc monitor-handle={} crop-x={} crop-y={} crop-width={} crop-height={}",
                    source.handle, crop_x, crop_y, source.width, source.height
                )
            }
        } else {
            if gstreamer::ElementFactory::find("d3d11screencapturesrc").is_some() {
                format!(
                    "d3d11screencapturesrc monitor-index={} crop-x={} crop-y={} crop-width={} crop-height={}",
                    monitor_index, crop_x, crop_y, source.width, source.height
                )
            } else if gstreamer::ElementFactory::find("d3d12screencapturesrc").is_some() {
                format!(
                    "d3d12screencapturesrc monitor-index={} crop-x={} crop-y={} crop-width={} crop-height={}",
                    monitor_index, crop_x, crop_y, source.width, source.height
                )
            } else {
                format!(
                    "d3d11screencapturesrc monitor-index={} crop-x={} crop-y={} crop-width={} crop-height={}",
                    monitor_index, crop_x, crop_y, source.width, source.height
                )
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        "d3d11screencapturesrc".to_string()
    }
}

/// Build DXGI+crop capture string for preview pipeline (fallback when WGC fails)
pub fn build_dxgi_crop_preview_capture_string(_source_id: &str, source_x: i32, source_y: i32, source_w: u32, source_h: u32, monitor_handle: u64) -> String {
    #[cfg(target_os = "windows")]
    {
        let (monitor_index, mon_x, mon_y) = get_monitor_info_for_window(source_x, source_y);
        let crop_x = (source_x - mon_x).max(0) as u32;
        let crop_y = (source_y - mon_y).max(0) as u32;

        // Use monitor-handle if available to avoid index mapping issues
        if monitor_handle != 0 {
            if gstreamer::ElementFactory::find("d3d11screencapturesrc").is_some() {
                format!(
                    "d3d11screencapturesrc monitor-handle={} crop-x={} crop-y={} crop-width={} crop-height={}",
                    monitor_handle, crop_x, crop_y, source_w, source_h
                )
            } else if gstreamer::ElementFactory::find("d3d12screencapturesrc").is_some() {
                format!(
                    "d3d12screencapturesrc monitor-handle={} crop-x={} crop-y={} crop-width={} crop-height={}",
                    monitor_handle, crop_x, crop_y, source_w, source_h
                )
            } else {
                format!(
                    "d3d11screencapturesrc monitor-handle={} crop-x={} crop-y={} crop-width={} crop-height={}",
                    monitor_handle, crop_x, crop_y, source_w, source_h
                )
            }
        } else {
            if gstreamer::ElementFactory::find("d3d11screencapturesrc").is_some() {
                format!(
                    "d3d11screencapturesrc monitor-index={} crop-x={} crop-y={} crop-width={} crop-height={}",
                    monitor_index, crop_x, crop_y, source_w, source_h
                )
            } else if gstreamer::ElementFactory::find("d3d12screencapturesrc").is_some() {
                format!(
                    "d3d12screencapturesrc monitor-index={} crop-x={} crop-y={} crop-width={} crop-height={}",
                    monitor_index, crop_x, crop_y, source_w, source_h
                )
            } else {
                format!(
                    "d3d11screencapturesrc monitor-index={} crop-x={} crop-y={} crop-width={} crop-height={}",
                    monitor_index, crop_x, crop_y, source_w, source_h
                )
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        "d3d11screencapturesrc".to_string()
    }
}

/// Build capture source element string based on platform and source type
fn build_capture_element(source: &CaptureSource) -> String {
    #[cfg(target_os = "windows")]
    {
        match source.source_type {
            SourceType::Monitor => {
                // Prefer monitor-handle (HMONITOR) over monitor-index to avoid
                // index mapping mismatches between EnumDisplayMonitors and GStreamer's
                // internal DXGI output enumeration order.
                if source.handle != 0 {
                    if gstreamer::ElementFactory::find("d3d11screencapturesrc").is_some() {
                        log::info!("Using d3d11screencapturesrc monitor-handle={} for monitor {}", source.handle, source.id);
                        format!("d3d11screencapturesrc monitor-handle={}", source.handle)
                    } else if gstreamer::ElementFactory::find("d3d12screencapturesrc").is_some() {
                        log::info!("Using d3d12screencapturesrc monitor-handle={} for monitor {}", source.handle, source.id);
                        format!("d3d12screencapturesrc monitor-handle={}", source.handle)
                    } else {
                        log::warn!("No hardware screen capture source found, trying d3d11screencapturesrc anyway");
                        format!("d3d11screencapturesrc monitor-handle={}", source.handle)
                    }
                } else {
                    // Fallback to monitor-index if no handle available
                    let monitor_index: i32 = source
                        .id
                        .strip_prefix("screen-")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    if gstreamer::ElementFactory::find("d3d11screencapturesrc").is_some() {
                        log::info!("Using d3d11screencapturesrc monitor-index={} for monitor {}", monitor_index, source.id);
                        format!("d3d11screencapturesrc monitor-index={}", monitor_index)
                    } else if gstreamer::ElementFactory::find("d3d12screencapturesrc").is_some() {
                        log::info!("Using d3d12screencapturesrc monitor-index={} for monitor {}", monitor_index, source.id);
                        format!("d3d12screencapturesrc monitor-index={}", monitor_index)
                    } else {
                        log::warn!("No hardware screen capture source found, trying d3d11screencapturesrc anyway");
                        format!("d3d11screencapturesrc monitor-index={}", monitor_index)
                    }
                }
            }
            SourceType::Window => {
                // For window RTSP streaming, use DXGI+crop mode (reliable for all windows).
                // WGC may fail at PLAYING time for some windows (PixPin, UWP, etc.),
                // and since RTSP pipelines are lazily started on client connection,
                // we cannot detect WGC failures early. DXGI+crop is the safe choice.
                let hwnd: u64 = source
                    .id
                    .strip_prefix("window-")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);

                let (monitor_index, mon_x, mon_y) = get_monitor_info_for_window(source.x, source.y);
                let crop_x = (source.x - mon_x).max(0) as u32;
                let crop_y = (source.y - mon_y).max(0) as u32;

                if gstreamer::ElementFactory::find("d3d11screencapturesrc").is_some() {
                    log::info!(
                        "Using d3d11screencapturesrc DXGI+crop for window hwnd={} monitor={} origin=({},{}) win=({},{}) crop=({},{}: {}x{})",
                        hwnd, monitor_index, mon_x, mon_y, source.x, source.y, crop_x, crop_y, source.width, source.height
                    );
                    format!(
                        "d3d11screencapturesrc monitor-index={} crop-x={} crop-y={} crop-width={} crop-height={}",
                        monitor_index, crop_x, crop_y, source.width, source.height
                    )
                } else if gstreamer::ElementFactory::find("d3d12screencapturesrc").is_some() {
                    log::info!(
                        "Using d3d12screencapturesrc DXGI+crop for window hwnd={} monitor={} origin=({},{}) win=({},{}) crop=({},{}: {}x{})",
                        hwnd, monitor_index, mon_x, mon_y, source.x, source.y, crop_x, crop_y, source.width, source.height
                    );
                    format!(
                        "d3d12screencapturesrc monitor-index={} crop-x={} crop-y={} crop-width={} crop-height={}",
                        monitor_index, crop_x, crop_y, source.width, source.height
                    )
                } else {
                    log::warn!("No hardware window capture source found, trying d3d11screencapturesrc anyway");
                    format!(
                        "d3d11screencapturesrc monitor-index={} crop-x={} crop-y={} crop-width={} crop-height={}",
                        monitor_index, crop_x, crop_y, source.width, source.height
                    )
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        match source.source_type {
            SourceType::Monitor => "avfvideosrc".to_string(),
            SourceType::Window => "avfvideosrc".to_string(),
        }
    }

    #[cfg(target_os = "linux")]
    {
        match source.source_type {
            SourceType::Monitor => "ximagesrc".to_string(),
            SourceType::Window => "ximagesrc".to_string(),
        }
    }
}

/// Determine which monitor index a window is on based on its position,
/// and return the monitor's origin (left, top) for calculating relative crop coordinates.
/// DXGI captures a specific monitor, so crop-x/crop-y must be relative to that monitor's origin,
/// not absolute screen coordinates.
/// Returns (monitor_index, monitor_origin_x, monitor_origin_y).
pub fn get_monitor_info_for_window(x: i32, y: i32) -> (i32, i32, i32) {
    // Enumerate monitors and find which one contains the window position.
    // IMPORTANT: The monitor-index for d3d11screencapturesrc is derived from
    // the Windows display device name (e.g., \\.\DISPLAY1 → index 0, \\.\DISPLAY2 → index 1),
    // NOT from the enumeration order of EnumDisplayMonitors.
    #[cfg(target_os = "windows")]
    {
        let result = std::sync::Mutex::new(Vec::<(i32, i32, i32, i32, i32)>::new()); // (display_index, left, top, right, bottom)
        let _ = unsafe {
            EnumDisplayMonitors(
                None,
                None,
                Some(monitor_enum_for_index),
                LPARAM(&result as *const _ as isize),
            )
        };

        let monitors = result.lock().unwrap().clone();
        for &(display_index, left, top, right, bottom) in monitors.iter() {
            if x >= left && x < right && y >= top && y < bottom {
                return (display_index, left, top);
            }
        }
    }

    // Fallback: assume primary monitor (index 0, origin 0,0)
    (0, 0, 0)
}

/// Backward-compatible wrapper that returns only the monitor index.
pub fn get_monitor_index_for_window(x: i32, y: i32) -> i32 {
    get_monitor_info_for_window(x, y).0
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn monitor_enum_for_index(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _lprc_clip: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let data = &*(lparam.0 as *const std::sync::Mutex<Vec<(i32, i32, i32, i32, i32)>>);
    let mut guard = data.lock().unwrap();

    let mut monitor_info: MONITORINFOEXW = std::mem::zeroed();
    monitor_info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

    let result = GetMonitorInfoW(hmonitor, &mut monitor_info as *mut MONITORINFOEXW as *mut _);
    if result.as_bool() {
        let rect = monitor_info.monitorInfo.rcMonitor;

        // Extract the true display index from device name (\\.\DISPLAY1 → 0, \\.\DISPLAY2 → 1)
        // This matches how d3d11screencapturesrc interprets monitor-index.
        let null_pos = monitor_info.szDevice.iter().position(|&c| c == 0).unwrap_or(32);
        let device_name = String::from_utf16_lossy(&monitor_info.szDevice[..null_pos]);
        let display_index = device_name
            .trim_start_matches(r"\\.\DISPLAY")
            .parse::<i32>()
            .unwrap_or(1)
            - 1;

        guard.push((display_index, rect.left, rect.top, rect.right, rect.bottom));
    }

    BOOL(1)
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
            x: 0,
            y: 0,
            is_streaming: false,
            rtsp_url: None,
            handle: 0,
        }
    }

    fn make_window(id: &str) -> CaptureSource {
        CaptureSource {
            id: id.to_string(),
            name: "Window".to_string(),
            source_type: SourceType::Window,
            width: 800,
            height: 600,
            x: 100,
            y: 100,
            is_streaming: false,
            rtsp_url: None,
            handle: 0,
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
    // Note: These require GStreamer runtime for capture element detection

    #[test]
    #[ignore]
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
    #[ignore]
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
    #[ignore]
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
    #[ignore]
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
    #[ignore]
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
    #[ignore]
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
