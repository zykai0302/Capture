use crate::capture::source::{CaptureSource, CaptureSourceList, SourceType};
use crate::error::{AppError, AppResult};

/// Enumerate display monitors on macOS.
///
/// macOS strategy:
/// - Monitors: Use CoreGraphics CGGetOnlineDisplayList via the `core-graphics` crate
/// - Windows: Use CGWindowListCopyWindowInfo via the `core-graphics` crate
///
/// GStreamer capture element: `avfvideosrc display-index=N` for monitors
pub fn enumerate_sources() -> AppResult<CaptureSourceList> {
    let monitors = enumerate_monitors()?;
    let windows = enumerate_windows()?;
    Ok(CaptureSourceList { monitors, windows })
}

fn enumerate_monitors() -> AppResult<Vec<CaptureSource>> {
    let mut monitor_list = Vec::new();

    // Use core-graphics crate for display enumeration
    #[cfg(target_os = "macos")]
    {
        use core_graphics::display::{CGGetOnlineDisplayList, CGDirectDisplayID, kCGNullDirectDisplayID};

        let mut display_ids: [CGDirectDisplayID; 16] = [0; 16];
        let mut display_count: u32 = 0;

        let result = unsafe {
            CGGetOnlineDisplayList(16, display_ids.as_mut_ptr(), &mut display_count)
        };

        if result != 0 {
            return Err(AppError::Capture(
                format!("CGGetOnlineDisplayList failed with error {}", result)
            ));
        }

        for i in 0..display_count as usize {
            let display_id = display_ids[i];
            let width = unsafe { core_graphics::display::CGDisplayPixelsWide(display_id) };
            let height = unsafe { core_graphics::display::CGDisplayPixelsHigh(display_id) };

            if width == 0 || height == 0 {
                continue;
            }

            // avfvideosrc uses display-index starting from 0
            let source = CaptureSource {
                id: format!("screen-{}", i),
                name: format!("显示器 {}", i),
                source_type: SourceType::Monitor,
                width,
                height,
                is_streaming: false,
                rtsp_url: None,
            };
            monitor_list.push(source);
        }
    }

    Ok(monitor_list)
}

fn enumerate_windows() -> AppResult<Vec<CaptureSource>> {
    // Window capture on macOS uses avfvideosrc with window-id
    // For now, return empty list — window enumeration on macOS requires
    // CGWindowListCopyWindowInfo which needs the core-graphics crate
    // with the `highsierra` feature enabled.
    // This can be extended later for full window capture support.
    Ok(Vec::new())
}
