use crate::capture::source::{CaptureSource, CaptureSourceList, SourceType};
use crate::error::AppResult;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, MONITORENUMPROC, MONITORINFOEXW,
    HMONITOR, HDC,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowRect, GetWindowTextW, IsWindowVisible, GWLP_HWNDPARENT,
    GetWindowLongPtrW,
};

pub fn enumerate_sources() -> AppResult<CaptureSourceList> {
    let monitors = enumerate_monitors()?;
    let windows = enumerate_windows()?;
    Ok(CaptureSourceList { monitors, windows })
}

fn enumerate_monitors() -> AppResult<Vec<CaptureSource>> {
    let mut monitor_list: Vec<CaptureSource> = Vec::new();
    let list_ptr = &mut monitor_list as *mut Vec<CaptureSource>;

    unsafe {
        let proc: MONITORENUMPROC = Some(monitor_enum_callback);
        let _ = EnumDisplayMonitors(None, None, proc, LPARAM(list_ptr as isize));
    }

    Ok(monitor_list)
}

unsafe extern "system" fn monitor_enum_callback(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _lprc_clip: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let list = &mut *(lparam.0 as *mut Vec<CaptureSource>);

    let mut monitor_info: MONITORINFOEXW = std::mem::zeroed();
    monitor_info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

    let result = unsafe { GetMonitorInfoW(hmonitor, &mut monitor_info as *mut MONITORINFOEXW as *mut _) };

    if result.as_bool() {
        let rect = monitor_info.monitorInfo.rcMonitor;
        let width = (rect.right - rect.left) as u32;
        let height = (rect.bottom - rect.top) as u32;

        // Extract monitor index from device name (e.g., "\\.\DISPLAY1" -> 0)
        let null_pos = monitor_info.szDevice.iter().position(|&c| c == 0).unwrap_or(32);
        let device_name = String::from_utf16_lossy(&monitor_info.szDevice[..null_pos]);
        let monitor_index = device_name
            .trim_start_matches(r"\\.\DISPLAY")
            .parse::<u32>()
            .unwrap_or(1)
            - 1;

        let source = CaptureSource {
            id: format!("screen-{}", monitor_index),
            name: format!("显示器 {}", monitor_index),
            source_type: SourceType::Monitor,
            width,
            height,
            x: rect.left,
            y: rect.top,
            is_streaming: false,
            rtsp_url: None,
            handle: hmonitor.0 as u64,
        };
        list.push(source);
    }

    BOOL(1) // Continue enumeration
}

fn enumerate_windows() -> AppResult<Vec<CaptureSource>> {
    let mut window_list: Vec<CaptureSource> = Vec::new();
    let list_ptr = &mut window_list as *mut Vec<CaptureSource>;

    unsafe {
        let _ = EnumWindows(Some(enum_windows_callback), LPARAM(list_ptr as isize));
    }

    Ok(window_list)
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let list = &mut *(lparam.0 as *mut Vec<CaptureSource>);

    unsafe {
        // Skip invisible windows
        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1); // Continue
        }

        // Skip windows with no title
        let mut title_buf = [0u16; 512];
        let title_len = GetWindowTextW(hwnd, &mut title_buf);
        if title_len == 0 {
            return BOOL(1);
        }
        let title = String::from_utf16_lossy(&title_buf[..title_len as usize]);

        // Skip owned windows (child windows)
        let owner = GetWindowLongPtrW(hwnd, GWLP_HWNDPARENT);
        if owner != 0 {
            return BOOL(1);
        }

        // Skip system windows
        if title == "Program Manager" || title == "Windows Input Experience" {
            return BOOL(1);
        }

        // Get window rect
        let mut rect = RECT::default();
        let _ = GetWindowRect(hwnd, &mut rect);
        let width = (rect.right - rect.left).max(0) as u32;
        let height = (rect.bottom - rect.top).max(0) as u32;

        // Skip very small windows
        if width < 50 || height < 50 {
            return BOOL(1);
        }

        let source = CaptureSource {
            id: format!("window-{}", hwnd.0 as u64),
            name: title,
            source_type: SourceType::Window,
            width,
            height,
            x: rect.left,
            y: rect.top,
            is_streaming: false,
            rtsp_url: None,
            handle: hwnd.0 as u64,
        };
        list.push(source);
    }

    BOOL(1) // Continue enumeration
}
