use crate::capture::source::{CaptureSource, CaptureSourceList, SourceType};
use crate::error::{AppError, AppResult};

/// Enumerate display monitors on Linux.
///
/// Linux strategy:
/// - Monitors: Parse XRandR output via `xrandr --query` command
/// - Windows: Not supported (X11 window capture requires complex EWMH queries)
///
/// GStreamer capture element: `ximagesrc display-name=:0 monitor-index=N` for X11,
/// or `pipewiresrc` for Wayland (future).
pub fn enumerate_sources() -> AppResult<CaptureSourceList> {
    let monitors = enumerate_monitors()?;
    let windows = enumerate_windows()?;
    Ok(CaptureSourceList { monitors, windows })
}

fn enumerate_monitors() -> AppResult<Vec<CaptureSource>> {
    let mut monitor_list = Vec::new();

    // Try xrandr first (X11)
    if let Ok(output) = std::process::Command::new("xrandr")
        .arg("--query")
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut index = 0;

            for line in stdout.lines() {
                // xrandr connected lines look like:
                // "DP-1 connected primary 1920x1080+0+0 (normal left...)"
                if line.contains(" connected") {
                    // Parse resolution from the line
                    let width: u32;
                    let height: u32;

                    // Extract the resolution (first WxH pattern before +)
                    if let Some(res_part) = line.split_whitespace().find(|s| s.contains('x') && s.contains('+')) {
                        let res_str = res_part.split('+').next().unwrap_or("");
                        let parts: Vec<&str> = res_str.split('x').collect();
                        if parts.len() == 2 {
                            width = parts[0].parse().unwrap_or(0);
                            height = parts[1].parse().unwrap_or(0);
                        } else {
                            continue;
                        }
                    } else {
                        // Connected but no active resolution (disabled output)
                        continue;
                    }

                    if width == 0 || height == 0 {
                        continue;
                    }

                    // Extract output name (e.g., "DP-1", "HDMI-0")
                    let output_name = line.split_whitespace().next().unwrap_or("unknown");

                    let source = CaptureSource {
                        id: format!("screen-{}", index),
                        name: format!("{} ({})", output_name, index),
                        source_type: SourceType::Monitor,
                        width,
                        height,
                        x: 0,
                        y: 0,
                        is_streaming: false,
                        rtsp_url: None,
                        handle: 0,
                    };
                    monitor_list.push(source);
                    index += 1;
                }
            }
        }
    }

    // Fallback: if xrandr not available, try reading /sys/class/drm
    if monitor_list.is_empty() {
        if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
            let mut index = 0;
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                // DRM connected cards look like "card0-DP-1" or "card0-HDMI-A-0"
                if name.starts_with("card") && !name.contains("-") {
                    // This is the card itself, skip
                    continue;
                }
                if name.starts_with("card") {
                    // Check if connected
                    let status_path = entry.path().join("status");
                    if let Ok(status) = std::fs::read_to_string(&status_path) {
                        if status.trim() == "connected" {
                            // Try to read mode (resolution)
                            let modes_path = entry.path().join("modes");
                            if let Ok(modes) = std::fs::read_to_string(&modes_path) {
                                if let Some(first_mode) = modes.lines().next() {
                                    let parts: Vec<&str> = first_mode.split('x').collect();
                                    if parts.len() == 2 {
                                        if let (Ok(w), Ok(h)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                                            if w > 0 && h > 0 {
                                                let source = CaptureSource {
                                                    id: format!("screen-{}", index),
                                                    name: format!("显示器 {} ({})", index, name),
                                                    source_type: SourceType::Monitor,
                                                    width: w,
                                                    height: h,
                                                    x: 0,
                                                    y: 0,
                                                    is_streaming: false,
                                                    rtsp_url: None,
                                                    handle: 0,
                                                };
                                                monitor_list.push(source);
                                                index += 1;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Ultimate fallback: assume one monitor at 1920x1080
    if monitor_list.is_empty() {
        log::warn!("No displays detected via xrandr or /sys/class/drm, assuming single 1920x1080 display");
        monitor_list.push(CaptureSource {
            id: "screen-0".to_string(),
            name: "显示器 0".to_string(),
            source_type: SourceType::Monitor,
            width: 1920,
            height: 1080,
            x: 0,
            y: 0,
            is_streaming: false,
            rtsp_url: None,
            handle: 0,
        });
    }

    Ok(monitor_list)
}

fn enumerate_windows() -> AppResult<Vec<CaptureSource>> {
    // Window enumeration on Linux requires X11 EWMH queries or PipeWire portal.
    // For now, return empty list.
    // Future: use xdotool search --name or gtk4 PipeWire portal for window capture.
    Ok(Vec::new())
}
