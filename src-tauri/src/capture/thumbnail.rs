use crate::error::{AppError, AppResult};
use image::{ImageBuffer, RgbaImage};
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS};
use windows::Win32::UI::WindowsAndMessaging::*;

pub fn capture_thumbnail(
    source_id: &str,
    source_type: &str,
    width: u32,
    height: u32,
    handle: u64,
) -> AppResult<Vec<u8>> {
    log::info!("capture_thumbnail: source_id={}, source_type={}, size={}x{}, handle={}", source_id, source_type, width, height, handle);
    let result = match source_type {
        "Monitor" => capture_monitor_thumbnail(source_id, width, height, handle),
        "Window" => capture_window_thumbnail(source_id, width, height),
        _ => Err(AppError::Capture(format!("Unknown source type: {}", source_type))),
    };
    match &result {
        Ok(data) => log::info!("capture_thumbnail: success, {} bytes", data.len()),
        Err(e) => log::error!("capture_thumbnail: failed: {}", e),
    }
    result
}

fn capture_monitor_thumbnail(source_id: &str, width: u32, height: u32, handle: u64) -> AppResult<Vec<u8>> {
    let monitor_handle = if handle != 0 {
        HMONITOR(handle as *mut _)
    } else {
        // Fallback: parse index from source_id and enumerate
        let monitor_index: u32 = source_id
            .strip_prefix("screen-")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| AppError::Capture(format!("Invalid monitor source_id: {}", source_id)))?;
        unsafe { find_monitor_by_index(monitor_index)? }
    };

    unsafe {
        
        let monitor_info = get_monitor_info(monitor_handle)?;
        let rect = monitor_info.monitorInfo.rcMonitor;
        let src_width = (rect.right - rect.left) as i32;
        let src_height = (rect.bottom - rect.top) as i32;

        if src_width <= 0 || src_height <= 0 {
            return Err(AppError::Capture(format!("Invalid monitor dimensions: {}x{}", src_width, src_height)));
        }

        let hdc_screen = GetDC(None);
        let hdc_compat = CreateCompatibleDC(hdc_screen);

        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = src_width;
        bmi.bmiHeader.biHeight = -src_height;
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB.0;

        let mut ppv_bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let hbitmap = CreateDIBSection(
            hdc_compat,
            &bmi,
            DIB_RGB_COLORS,
            &mut ppv_bits,
            None,
            0,
        ).map_err(|e| AppError::Capture(format!("CreateDIBSection failed: {}", e)))?;
        
        let old_bitmap = SelectObject(hdc_compat, hbitmap);
        // Use the monitor's screen coordinates (rect.left, rect.top) as the source offset.
        // GetDC(None) returns the DC for the entire virtual screen (all monitors),
        // so BitBlt from (rect.left, rect.top) captures the correct monitor.
        let blt_result = BitBlt(hdc_compat, 0, 0, src_width, src_height, hdc_screen, rect.left, rect.top, SRCCOPY);
        if let Err(e) = blt_result {
            log::error!("BitBlt failed for monitor at ({},{}): {}", rect.left, rect.top, e);
        }

        let pixel_count = (src_width * src_height * 4) as usize;
        let u8_ptr = ppv_bits as *const u8;
        let pixel_slice = std::slice::from_raw_parts(u8_ptr, pixel_count);

        let mut rgba_data = vec![0u8; pixel_count];
        for i in (0..pixel_count).step_by(4) {
            rgba_data[i] = pixel_slice[i + 2];
            rgba_data[i + 1] = pixel_slice[i + 1];
            rgba_data[i + 2] = pixel_slice[i];
            rgba_data[i + 3] = pixel_slice[i + 3];
        }

        let _ = SelectObject(hdc_compat, old_bitmap);
        let _ = DeleteObject(hbitmap);
        let _ = DeleteDC(hdc_compat);
        let _ = ReleaseDC(None, hdc_screen);

        let img = ImageBuffer::<image::Rgba<u8>, _>::from_raw(src_width as u32, src_height as u32, rgba_data)
            .ok_or_else(|| AppError::Capture("Failed to create image buffer".to_string()))?;
        
        let resized = image::imageops::resize(&img, width, height, image::imageops::FilterType::Triangle);
        encode_jpeg(&resized)
    }
}

fn capture_window_thumbnail(source_id: &str, width: u32, height: u32) -> AppResult<Vec<u8>> {
    let hwnd_val: u64 = source_id
        .strip_prefix("window-")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| AppError::Capture(format!("Invalid window source_id: {}", source_id)))?;

    let hwnd = HWND(hwnd_val as *mut _);

    unsafe {
        let mut rect = std::mem::zeroed();
        let _ = GetWindowRect(hwnd, &mut rect);
        let src_width = (rect.right - rect.left).max(1) as i32;
        let src_height = (rect.bottom - rect.top).max(1) as i32;

        let hdc_window = GetWindowDC(hwnd);
        let hdc_compat = CreateCompatibleDC(hdc_window);

        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = src_width;
        bmi.bmiHeader.biHeight = -src_height;
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB.0;

        let mut ppv_bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let hbitmap = CreateDIBSection(
            hdc_compat,
            &bmi,
            DIB_RGB_COLORS,
            &mut ppv_bits,
            None,
            0,
        ).map_err(|e| AppError::Capture(format!("CreateDIBSection failed: {}", e)))?;
        let old_bitmap = SelectObject(hdc_compat, hbitmap);
        let _ = PrintWindow(hwnd, hdc_compat, PRINT_WINDOW_FLAGS(2));

        let pixel_count = (src_width * src_height * 4) as usize;
        let u8_ptr = ppv_bits as *const u8;
        let pixel_slice = std::slice::from_raw_parts(u8_ptr, pixel_count);

        let mut rgba_data = vec![0u8; pixel_count];
        for i in (0..pixel_count).step_by(4) {
            rgba_data[i] = pixel_slice[i + 2];
            rgba_data[i + 1] = pixel_slice[i + 1];
            rgba_data[i + 2] = pixel_slice[i];
            rgba_data[i + 3] = pixel_slice[i + 3];
        }

        let _ = SelectObject(hdc_compat, old_bitmap);
        let _ = DeleteObject(hbitmap);
        let _ = DeleteDC(hdc_compat);
        let _ = ReleaseDC(hwnd, hdc_window);

        let img = ImageBuffer::<image::Rgba<u8>, _>::from_raw(src_width as u32, src_height as u32, rgba_data)
            .ok_or_else(|| AppError::Capture("Failed to create image buffer".to_string()))?;

        let resized = image::imageops::resize(&img, width, height, image::imageops::FilterType::Triangle);
        encode_jpeg(&resized)
    }
}

fn encode_jpeg(img: &RgbaImage) -> AppResult<Vec<u8>> {
    // JPEG does not support alpha — convert RGBA → RGB first
    let rgb_img = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();
    let mut buf = std::io::Cursor::new(Vec::new());
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 60);
    encoder.encode(&rgb_img, rgb_img.width(), rgb_img.height(), image::ExtendedColorType::Rgb8)
        .map_err(|e| AppError::Capture(format!("JPEG encode error: {}", e)))?;
    Ok(buf.into_inner())
}

unsafe fn find_monitor_by_index(target_index: u32) -> AppResult<HMONITOR> {
    let result = std::sync::Mutex::new((None::<HMONITOR>, 0u32, target_index));
    let _ = EnumDisplayMonitors(
        None,
        None,
        Some(monitor_enum_proc),
        LPARAM(&result as *const _ as isize),
    );
    let (handle, _, _) = result.into_inner().unwrap();
    handle.ok_or_else(|| AppError::Capture(format!("Monitor index {} not found", target_index)))
}

unsafe extern "system" fn monitor_enum_proc(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _lprc: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let data = &*(lparam.0 as *const std::sync::Mutex<(Option<HMONITOR>, u32, u32)>);
    let mut guard = data.lock().unwrap();
    let target_index = guard.2;
    if guard.1 == target_index {
        guard.0 = Some(hmonitor);
    }
    guard.1 += 1;
    BOOL(1)
}

unsafe fn get_monitor_info(hmonitor: HMONITOR) -> AppResult<MONITORINFOEXW> {
    let mut info: MONITORINFOEXW = std::mem::zeroed();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    GetMonitorInfoW(hmonitor, &mut info as *mut MONITORINFOEXW as *mut _)
        .as_bool()
        .then_some(info)
        .ok_or_else(|| AppError::Capture("GetMonitorInfoW failed".to_string()))
}
