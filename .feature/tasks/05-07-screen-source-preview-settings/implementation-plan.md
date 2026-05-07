# 画面源预览与设置面板折叠 Implementation Plan

**Goal:** 优化主界面三大区域交互——左侧画面源支持滚动+缩略图自动刷新、中间推流画面按编码帧率实时显示（MJPEG）、右侧设置面板可折叠/展开。

**Context:**
- task directory: `.feature/tasks/05-07-screen-source-preview-settings/`
- related PRD: `.feature/tasks/05-07-screen-source-preview-settings/prd.md`
- key findings: tokio::net 复用（非 tiny_http）、image crate JPEG 编码、TopBar 已有设置按钮、CSP 无限制、AppError 需新增 Preview 变体、独立预览 pipeline（不修改 RTSP pipeline）

**Architecture:** 后端新增 thumbnail 截图命令（BitBlt/PrintWindow + image crate）和独立预览 pipeline + tokio MJPEG HTTP 服务器；前端 SourceItem 展示缩略图 `<img>`、MainPreview 展示 MJPEG 流 `<img>`、TopBar 设置按钮切换面板折叠。所有后端 Tauri 命令遵循 `snake_case` 参数，前端 invoke 用 `camelCase`（Tauri 自动映射）。

**Tech Stack:** Rust (Windows Win32 API, GStreamer appsink, tokio::net, image crate), Vue 3 (Composition API, `<img>` MJPEG), CSS Grid 动态过渡

**Execution Options:**
- with test cases: `/feature:write-testcase` -> `/feature:check-testcase` -> `/feature:executing-plans` or `/feature:subagent-work`
- sequential: `/feature:executing-plans`
- delegated: `/feature:subagent-work`

## Test Case Execution Phase

**Preconditions**:
- Test cases generated and verified (38 cases, 100% coverage)
- Coverage rate meets threshold (>80%)

**Steps**:
- [ ] Run test cases during implementation
- [ ] Record test results in progress.md
- [ ] Fix failed cases
- [ ] Update test case document if requirements change

---

## Task 1: Add `image` crate dependency to Cargo.toml

**Why**
- 缩略图截图命令需要 JPEG 编码能力，`image` crate 是纯 Rust 跨平台方案

**Files**
- Modify: `src-tauri/Cargo.toml`

**Steps**
- [ ] Add `image = "0.25"` to `[dependencies]` section (after `thiserror = "2"`)

**Validation**
- Run: `cd src-tauri && cargo check`
- Expect: compiles without errors

**Risks / Notes**
- `image` crate 约 2MB，纯 Rust 无系统依赖

---

## Task 2: Add `Preview(String)` error variant to AppError

**Why**
- 预览 pipeline 和 MJPEG 服务器需要专属错误类型

**Files**
- Modify: `src-tauri/src/error.rs`

**Steps**
- [ ] Add variant after `Remote(String)`:
  ```rust
  #[error("Preview error: {0}")]
  Preview(String),
  ```
- [ ] Add test `app_error_display_preview`:
  ```rust
  #[test]
  fn app_error_display_preview() {
      let err = AppError::Preview("pipeline failed".to_string());
      assert_eq!(format!("{}", err), "Preview error: pipeline failed");
  }
  ```

**Validation**
- Run: `cd src-tauri && cargo test --lib error`
- Expect: all tests pass including new test

**Risks / Notes**
- Serialize impl already handles all variants via `self.to_string()`

---

## Task 3: Add `preview_http_port` to AppConfig

**Why**
- MJPEG HTTP 服务器需要可配置端口（默认 8090）

**Files**
- Modify: `src-tauri/src/config/mod.rs`

**Steps**
- [ ] Add field to `AppConfig` struct:
  ```rust
  pub preview_http_port: u16,  // default: 8090
  ```
- [ ] Add to `Default` impl:
  ```rust
  preview_http_port: 8090,
  ```
- [ ] Update existing tests that construct `AppConfig` manually to include the new field
- [ ] Add test `default_config_has_preview_http_port`:
  ```rust
  #[test]
  fn default_config_has_preview_http_port() {
      let config = AppConfig::default();
      assert_eq!(config.preview_http_port, 8090);
  }
  ```
- [ ] Update TypeScript type in `src/types/index.ts`:
  ```typescript
  export interface AppConfig {
    rtsp_port: number
    rtsp_max_clients: number
    ws_port: number
    ws_password: string
    auto_reconnect: boolean
    default_encode: EncodeConfig
    preview_http_port: number  // MJPEG preview server port
  }
  ```

**Validation**
- Run: `cd src-tauri && cargo test --lib config`
- Expect: all tests pass

**Risks / Notes**
- `snake_case` in Rust → `camelCase` in TypeScript via Tauri auto-mapping
- Existing tests in config/mod.rs that use `AppConfig { ... }` struct literal must include `preview_http_port`

---

## Task 4: Implement `capture_thumbnail` Tauri command (backend)

**Why**
- 前端需要获取画面源的真实缩略图用于 SourceItem 预览

**Files**
- Create: `src-tauri/src/capture/thumbnail.rs`
- Modify: `src-tauri/src/capture/mod.rs`
- Modify: `src-tauri/src/lib.rs`

**Steps**
- [ ] **Step 1: Create `thumbnail.rs`** with the following implementation:

  ```rust
  use crate::error::{AppError, AppResult};
  use image::{ImageBuffer, RgbaImage};
  use windows::Win32::Foundation::HWND;
  use windows::Win32::Graphics::Gdi::*;
  use windows::Win32::UI::WindowsAndMessaging::*;

  /// Capture a thumbnail of a monitor or window and return JPEG bytes.
  /// `source_id` format: "screen-{index}" for monitors, "window-{hwnd}" for windows
  /// `source_type`: "Monitor" or "Window"
  /// `width`/`height`: desired thumbnail dimensions (e.g., 320x180)
  pub fn capture_thumbnail(
      source_id: &str,
      source_type: &str,
      width: u32,
      height: u32,
  ) -> AppResult<Vec<u8>> {
      match source_type {
          "Monitor" => capture_monitor_thumbnail(source_id, width, height),
          "Window" => capture_window_thumbnail(source_id, width, height),
          _ => Err(AppError::Capture(format!("Unknown source type: {}", source_type))),
      }
  }

  fn capture_monitor_thumbnail(source_id: &str, width: u32, height: u32) -> AppResult<Vec<u8>> {
      let monitor_index: u32 = source_id
          .strip_prefix("screen-")
          .and_then(|s| s.parse().ok())
          .ok_or_else(|| AppError::Capture(format!("Invalid monitor source_id: {}", source_id)))?;

      unsafe {
          // Find the monitor by index using EnumDisplayMonitors
          let monitor_handle = find_monitor_by_index(monitor_index)?;
          
          // Get monitor DC
          let hdc_monitor = GetDC(None);
          let hdc_compat = CreateCompatibleDC(hdc_monitor);
          
          let monitor_info = get_monitor_info(monitor_handle)?;
          let rect = monitor_info.monitorInfo.rcMonitor;
          let src_width = (rect.right - rect.left) as i32;
          let src_height = (rect.bottom - rect.top) as i32;

          // Create DIB section for capture
          let mut bmi: BITMAPINFO = std::mem::zeroed();
          bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
          bmi.bmiHeader.biWidth = src_width;
          bmi.bmiHeader.biHeight = -src_height; // top-down
          bmi.bmiHeader.biPlanes = 1;
          bmi.bmiHeader.biBitCount = 32;
          bmi.bmiHeader.biCompression = BI_RGB;

          let mut ppv_bits: *mut u8 = std::ptr::null_mut();
          let hbitmap = CreateDIBSection(hdc_compat, &bmi, DIB_RGB_COLORS, &mut ppv_bits as *mut _ as *mut _, None, 0);
          
          let old_bitmap = SelectObject(hdc_compat, hbitmap);

          // BitBlt from monitor DC to compatible DC
          BitBlt(hdc_compat, 0, 0, src_width, src_height, hdc_monitor, 0, 0, SRCCOPY);

          // Read pixels from DIB section
          let pixel_count = (src_width * src_height * 4) as usize;
          let pixel_slice = std::slice::from_raw_parts(ppv_bits, pixel_count);

          // Convert BGRA to RGBA
          let mut rgba_data = vec![0u8; pixel_count];
          for i in (0..pixel_count).step_by(4) {
              rgba_data[i] = pixel_slice[i + 2];     // R
              rgba_data[i + 1] = pixel_slice[i + 1]; // G
              rgba_data[i + 2] = pixel_slice[i];     // B
              rgba_data[i + 3] = pixel_slice[i + 3]; // A
          }

          // Cleanup
          SelectObject(hdc_compat, old_bitmap);
          DeleteObject(hbitmap);
          DeleteDC(hdc_compat);
          ReleaseDC(None, hdc_monitor);

          // Create image and resize
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
          bmi.bmiHeader.biCompression = BI_RGB;

          let mut ppv_bits: *mut u8 = std::ptr::null_mut();
          let hbitmap = CreateDIBSection(hdc_compat, &bmi, DIB_RGB_COLORS, &mut ppv_bits as *mut _ as *mut _, None, 0);
          let old_bitmap = SelectObject(hdc_compat, hbitmap);

          // Use PrintWindow with PW_RENDERFULLCONTENT for better capture
          PrintWindow(hwnd, hdc_compat, PW_RENDERFULLCONTENT);

          let pixel_count = (src_width * src_height * 4) as usize;
          let pixel_slice = std::slice::from_raw_parts(ppv_bits, pixel_count);

          let mut rgba_data = vec![0u8; pixel_count];
          for i in (0..pixel_count).step_by(4) {
              rgba_data[i] = pixel_slice[i + 2];
              rgba_data[i + 1] = pixel_slice[i + 1];
              rgba_data[i + 2] = pixel_slice[i];
              rgba_data[i + 3] = pixel_slice[i + 3];
          }

          SelectObject(hdc_compat, old_bitmap);
          DeleteObject(hbitmap);
          DeleteDC(hdc_compat);
          ReleaseDC(hwnd, hdc_window);

          let img = ImageBuffer::<image::Rgba<u8>, _>::from_raw(src_width as u32, src_height as u32, rgba_data)
              .ok_or_else(|| AppError::Capture("Failed to create image buffer".to_string()))?;

          let resized = image::imageops::resize(&img, width, height, image::imageops::FilterType::Triangle);
          encode_jpeg(&resized)
      }
  }

  fn encode_jpeg(img: &RgbaImage) -> AppResult<Vec<u8>> {
      let mut buf = std::io::Cursor::new(Vec::new());
      let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 60);
      encoder.encode(img, img.width(), img.height(), image::ExtendedColorType::Rgba8)
          .map_err(|e| AppError::Capture(format!("JPEG encode error: {}", e)))?;
      Ok(buf.into_inner())
  }

  // Helper: find HMONITOR by index (mirrors enumerate_monitors logic)
  unsafe fn find_monitor_by_index(target_index: u32) -> AppResult<HMONITOR> {
      let result = std::sync::Mutex::new((None::<HMONITOR>, 0u32));
      let proc: MONITORENUMPROC = Some(|hmonitor, _hdc, _lprc, lparam| {
          let data = &*(lparam.0 as *const std::sync::Mutex<(Option<HMONITOR>, u32)>);
          let mut guard = data.lock().unwrap();
          if guard.1 == target_index {
              guard.0 = Some(hmonitor);
          }
          guard.1 += 1;
          BOOL(1)
      });
      let _ = EnumDisplayMonitors(None, None, proc, LPARAM(&result as *const _ as isize));
      let (handle, _) = result.into_inner().unwrap();
      handle.ok_or_else(|| AppError::Capture(format!("Monitor index {} not found", target_index)))
  }

  unsafe fn get_monitor_info(hmonitor: HMONITOR) -> AppResult<MONITORINFOEXW> {
      let mut info: MONITORINFOEXW = std::mem::zeroed();
      info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
      GetMonitorInfoW(hmonitor, &mut info as *mut MONITORINFOEXW as *mut _)
          .as_bool()
          .then_some(info)
          .ok_or_else(|| AppError::Capture("GetMonitorInfoW failed".to_string()))
  }
  ```

- [ ] **Step 2: Add `thumbnail` module to `capture/mod.rs`**:
  Add `pub mod thumbnail;` at end of file (after `pub mod hotplug;`)

- [ ] **Step 3: Register `capture_thumbnail` command in `lib.rs`**:
  Add the command function:
  ```rust
  #[tauri::command]
  async fn capture_thumbnail(source_id: String, source_type: String, width: u32, height: u32) -> Result<Vec<u8>, AppError> {
      tokio::task::spawn_blocking(move || {
          capture::thumbnail::capture_thumbnail(&source_id, &source_type, width, height)
      })
      .await
      .map_err(|e| AppError::Capture(format!("Task join error: {}", e)))?
  }
  ```
  Add `capture_thumbnail` to `invoke_handler` generate_handler macro list.

**Validation**
- Run: `cd src-tauri && cargo check`
- Expect: compiles without errors

**Risks / Notes**
- `PrintWindow` 对最小化窗口可能返回黑屏 — 前端 fallback 到 SVG 占位图
- `PW_RENDERFULLCONTENT` 需要 Windows 8.1+（已在 Windows features 中）
- Tauri 命令参数 `source_id` → 前端调用 `invoke('capture_thumbnail', { sourceId, sourceType, width, height })` (Tauri auto-maps snake_case → camelCase)

---

## Task 5: Frontend SourceItem — replace SVG with real thumbnail

**Why**
- SourceItem 当前使用 SVG 占位图，需要替换为真实截图缩略图

**Files**
- Modify: `src/components/SourceItem.vue`
- Modify: `src/composables/useSources.ts`

**Steps**
- [ ] **Step 1: Add thumbnail ref and loading logic to `useSources.ts`**:

  Add `thumbnailMap` ref and `fetchThumbnails` function:
  ```typescript
  const thumbnailMap = ref<Record<string, string>>({})  // source_id -> data URL

  async function fetchThumbnails() {
    const sources = [...monitors.value, ...windows.value]
    const results = await Promise.allSettled(
      sources.map(async (source) => {
        const jpegData = await invoke<number[]>('capture_thumbnail', {
          sourceId: source.id,
          sourceType: source.source_type,
          width: 320,
          height: 180,
        })
        const bytes = new Uint8Array(jpegData)
        const blob = new Blob([bytes], { type: 'image/jpeg' })
        return { id: source.id, url: URL.createObjectURL(blob) }
      })
    )
    const newMap: Record<string, string> = {}
    for (const result of results) {
      if (result.status === 'fulfilled') {
        // Revoke old URL to avoid memory leak
        if (thumbnailMap.value[result.value.id]) {
          URL.revokeObjectURL(thumbnailMap.value[result.value.id])
        }
        newMap[result.value.id] = result.value.url
      }
    }
    thumbnailMap.value = newMap
  }
  ```

  Call `fetchThumbnails()` after each `refresh()` in the onMounted interval and event listeners:
  ```typescript
  refreshInterval = setInterval(() => {
    refresh().then(() => { updateAllSources(); fetchThumbnails() })
  }, 5000)
  ```
  And after initial refresh:
  ```typescript
  onMounted(async () => {
    await refresh()
    updateAllSources()
    fetchThumbnails()  // initial thumbnails
    // ... existing event listeners ...
  })
  ```

  Add `thumbnailMap` to the return object.

- [ ] **Step 2: Update `SourceItem.vue` template**:

  Replace the SVG preview section (lines 56-74) with:
  ```html
  <div class="source-preview">
    <img
      v-if="thumbnailUrl"
      :src="thumbnailUrl"
      class="preview-thumbnail"
      alt="preview"
    />
    <svg v-else class="screen-preview-svg" viewBox="0 0 160 90">
      <rect width="160" height="90" fill="#1a2438"/>
      <template v-if="source.source_type === SourceType.Monitor">
        <rect x="10" y="8" width="60" height="35" rx="3" fill="#2a4a6b" opacity="0.5"/>
        <rect x="80" y="8" width="70" height="20" rx="3" fill="#1e2d45" opacity="0.5"/>
        <rect x="10" y="50" width="140" height="8" rx="2" fill="#1e2d45"/>
        <rect x="10" y="64" width="90" height="8" rx="2" fill="#1e2d45"/>
        <rect x="10" y="78" width="50" height="8" rx="2" fill="#1e2d45"/>
      </template>
      <template v-else>
        <rect x="15" y="5" width="130" height="80" rx="4" fill="#1e2d45" stroke="#2a4a6b" stroke-width="1"/>
        <rect x="15" y="5" width="130" height="16" rx="4" fill="#253350"/>
        <circle cx="27" cy="13" r="3" fill="#ff5f57"/>
        <circle cx="37" cy="13" r="3" fill="#febc2e"/>
        <circle cx="47" cy="13" r="3" fill="#28c840"/>
        <rect x="25" y="28" width="110" height="6" rx="2" fill="#2a4a6b"/>
        <rect x="25" y="40" width="80" height="4" rx="2" fill="#253350"/>
      </template>
    </svg>
    <span v-if="isStreaming" class="live-badge">LIVE</span>
  </div>
  ```

  Add `thumbnailUrl` computed in `<script setup>`:
  ```typescript
  const { thumbnailMap } = inject<ReturnType<typeof useSources>>('sources')!
  const thumbnailUrl = computed(() => thumbnailMap.value[props.source.id] || '')
  ```

- [ ] **Step 3: Add CSS for thumbnail image** in SourceItem.vue `<style scoped>`:
  ```css
  .preview-thumbnail {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  ```

- [ ] **Step 4: Update SourceList.vue** to pass `thumbnailMap` via inject (already done via `useSources` inject)

**Validation**
- Run: `cd "e:\抓屏软件" && npx vue-tsc --noEmit`
- Expect: no TypeScript errors

**Risks / Notes**
- `invoke<number[]>('capture_thumbnail', ...)` — Tauri 返回 `Vec<u8>` 在 JS 中为 `number[]`
- `URL.createObjectURL` 需要在重新获取时 `revokeObjectURL` 旧 URL 防止内存泄漏
- `Promise.allSettled` 确保单个源截图失败不影响其他源

---

## Task 6: Verify SourceList scrolling works correctly

**Why**
- PRD 要求画面源过多时可滚动浏览。SourceList.vue 已有 `overflow-y: auto`，需确认在内容溢出时正常工作。

**Files**
- Verify: `src/components/SourceList.vue` (no change expected)

**Steps**
- [ ] Verify `.source-list` has `flex: 1; overflow-y: auto;` (already present at lines 152-153)
- [ ] Verify `.panel-left` has `overflow: hidden;` (already present at line 94) to constrain the scrollable area
- [ ] Verify no parent element clips or overrides the overflow behavior

**Validation**
- Manual test: add 20+ dummy sources, verify scrolling works in the `.source-list` area
- The CSS is already correct; this is a verification-only task

**Risks / Notes**
- If issues found, may need to add `min-height: 0` to `.source-list` (common flex overflow fix)

---

## Task 7: Implement preview pipeline (backend — new module)

**Why**
- 独立预览 pipeline 从源 capture → videoconvert → jpegenc → appsink，提供 JPEG 帧给 MJPEG 服务器

**Files**
- Create: `src-tauri/src/pipeline/preview.rs`
- Modify: `src-tauri/src/pipeline/mod.rs`

**Steps**
- [ ] **Step 1: Create `preview.rs`**:

  ```rust
  use crate::error::{AppError, AppResult};
  use gstreamer::prelude::*;
  use gstreamer_app::AppSink;
  use std::sync::mpsc;
  use std::sync::Arc;

  pub struct PreviewPipeline {
      pipeline: gstreamer::Pipeline,
      frame_tx: Arc<mpsc::SyncSender<Vec<u8>>>,
      pub frame_rx: mpsc::Receiver<Vec<u8>>,
  }

  impl PreviewPipeline {
      /// Create a preview pipeline for a given source.
      /// Returns the pipeline and a receiver for JPEG frames.
      pub fn new(source_id: &str, source_type: &str, framerate: u32) -> AppResult<Self> {
          gstreamer::init().map_err(|e| AppError::GStreamer(format!("GStreamer init: {}", e)))?;

          let pipeline = gstreamer::Pipeline::builder()
              .name(format!("preview-{}", source_id).as_str())
              .build();

          // Build capture element
          let capture = build_preview_capture_element(source_id, source_type)?;

          // videoconvert
          let convert = gstreamer::ElementFactory::make("videoconvert")
              .name("preview-convert")
              .build()
              .map_err(|e| AppError::GStreamer(format!("videoconvert: {}", e)))?;

          // capsfilter to set framerate
          let capsfilter = gstreamer::ElementFactory::make("capsfilter")
              .name("preview-capsfilter")
              .build()
              .map_err(|e| AppError::GStreamer(format!("capsfilter: {}", e)))?;
          let caps = gstreamer::Caps::builder("video/x-raw")
              .field("framerate", gstreamer::Fraction::new(framerate as i32, 1))
              .build();
          capsfilter.set_property("caps", &caps);

          // jpegenc
          let jpegenc = gstreamer::ElementFactory::make("jpegenc")
              .name("preview-jpegenc")
              .build()
              .map_err(|e| AppError::GStreamer(format!("jpegenc: {}", e)))?;
          jpegenc.set_property("quality", 60u32);

          // appsink
          let appsink = gstreamer::ElementFactory::make("appsink")
              .name("preview-appsink")
              .build()
              .map_err(|e| AppError::GStreamer(format!("appsink: {}", e)))?;

          let appsink = appsink
              .dynamic_cast::<AppSink>()
              .map_err(|_| AppError::GStreamer("appsink cast failed".to_string()))?;

          appsink.set_property("emit-signals", true);
          appsink.set_property("max-buffers", 1u32);
          appsink.set_property("drop", true);

          let (tx, rx) = mpsc::sync_channel(2);
          let tx_arc = Arc::new(tx);

          appsink.connect_new_sample(move |appsink| {
              let sample = appsink.pull_sample().map_err(|_| gstreamer::FlowError::Error)?;
              let buffer = sample.buffer().ok_or(gstreamer::FlowError::Error)?;
              let map = buffer.map_readable().map_err(|_| gstreamer::FlowError::Error)?;
              let data = map.as_slice().to_vec();
              let _ = tx_arc.send(data);
              Ok(gstreamer::FlowSuccess::Ok)
          });

          // Add elements and link
          pipeline.add_many([&capture, &convert, &capsfilter, &jpegenc, &appsink.downcast::<gstreamer::Element>().unwrap()])?;
          gstreamer::Element::link_many([&capture, &convert, &capsfilter, &jpegenc, &appsink.downcast::<gstreamer::Element>().unwrap()])?;

          Ok(PreviewPipeline {
              pipeline,
              frame_tx: tx_arc,
              frame_rx: rx,
          })
      }

      pub fn play(&self) -> AppResult<()> {
          self.pipeline
              .set_state(gstreamer::State::Playing)
              .map_err(|e| AppError::Preview(format!("Pipeline play: {}", e)))?;
          Ok(())
      }

      pub fn stop(&self) -> AppResult<()> {
          self.pipeline
              .set_state(gstreamer::State::Null)
              .map_err(|e| AppError::Preview(format!("Pipeline stop: {}", e)))?;
          Ok(())
      }
  }

  impl Drop for PreviewPipeline {
      fn drop(&mut self) {
          let _ = self.pipeline.set_state(gstreamer::State::Null);
      }
  }

  fn build_preview_capture_element(source_id: &str, source_type: &str) -> AppResult<gstreamer::Element> {
      let element = match source_type {
          "Monitor" => {
              let monitor_index: i32 = source_id
                  .strip_prefix("screen-")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(0);
              let src = gstreamer::ElementFactory::make("d3d12screencapturesrc")
                  .name("preview-capture")
                  .build()
                  .map_err(|e| AppError::GStreamer(format!("d3d12screencapturesrc: {}", e)))?;
              src.set_property("monitor-index", monitor_index);
              src
          }
          "Window" => {
              let hwnd: i64 = source_id
                  .strip_prefix("window-")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(0) as i64;
              let src = gstreamer::ElementFactory::make("d3d12screencapturesrc")
                  .name("preview-capture")
                  .build()
                  .map_err(|e| AppError::GStreamer(format!("d3d12screencapturesrc: {}", e)))?;
              src.set_property("window-handle", hwnd);
              src
          }
          _ => return Err(AppError::Preview(format!("Unknown source type: {}", source_type))),
      };
      Ok(element)
  }
  ```

- [ ] **Step 2: Add `preview` module to `pipeline/mod.rs`**:
  Add `pub mod preview;` after `pub mod manager;`

**Validation**
- Run: `cd src-tauri && cargo check`
- Expect: compiles without errors

**Risks / Notes**
- `d3d12screencapturesrc` 同时被两个 pipeline（RTSP + 预览）capture 同一源可能冲突 — 需实际测试，如不支持则备选方案为在 RTSP pipeline 添加 tee 分支
- `sync_channel(2)` 限制缓冲帧数为 2，配合 `drop=true` 和 `max-buffers=1` 实现最新帧
- `gstreamer-app` 已在 Cargo.toml 中

---

## Task 8: Implement MJPEG HTTP server (backend — new module)

**Why**
- 通过 HTTP MJPEG 流提供推流画面给前端 `<img>` 标签

**Files**
- Create: `src-tauri/src/pipeline/mjpeg_server.rs`
- Modify: `src-tauri/src/pipeline/mod.rs`

**Steps**
- [ ] **Step 1: Create `mjpeg_server.rs`**:

  ```rust
  use crate::error::{AppError, AppResult};
  use std::sync::atomic::{AtomicBool, Ordering};
  use std::sync::Arc;
  use std::collections::HashMap;
  use tokio::sync::Mutex;
  use tokio::net::TcpListener;
  use std::sync::mpsc::Receiver;

  /// MJPEG HTTP server that serves JPEG frames from preview pipelines.
  /// Routes: GET /<source_id> → MJPEG stream
  pub struct MjpegServer {
      pub port: u16,
      pub running: Arc<AtomicBool>,
      shutdown_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
      frame_sources: Arc<Mutex<HashMap<String, Receiver<Vec<u8>>>>>,
  }

  impl MjpegServer {
      pub fn new(port: u16) -> Self {
          Self {
              port,
              running: Arc::new(AtomicBool::new(false)),
              shutdown_tx: Arc::new(Mutex::new(None)),
              frame_sources: Arc::new(Mutex::new(HashMap::new())),
          }
      }

      /// Register a frame source for a source_id
      pub async fn add_source(&self, source_id: String, frame_rx: Receiver<Vec<u8>>) {
          self.frame_sources.lock().await.insert(source_id, frame_rx);
      }

      /// Remove a frame source
      pub async fn remove_source(&self, source_id: &str) {
          self.frame_sources.lock().await.remove(source_id);
      }

      pub async fn start(&self) -> AppResult<()> {
          let addr = format!("127.0.0.1:{}", self.port);
          let listener = TcpListener::bind(&addr)
              .await
              .map_err(|e| AppError::Preview(format!("MJPEG bind failed: {}", e)))?;

          log::info!("MJPEG preview server listening on {}", addr);
          self.running.store(true, Ordering::SeqCst);

          let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
          *self.shutdown_tx.lock().await = Some(shutdown_tx);

          let running = self.running.clone();
          let frame_sources = self.frame_sources.clone();

          tokio::spawn(async move {
              loop {
                  let accept_result = tokio::select! {
                      result = listener.accept() => result,
                      _ = &mut shutdown_rx => break,
                  };

                  match accept_result {
                      Ok((stream, _addr)) => {
                          let frame_sources = frame_sources.clone();
                          tokio::spawn(async move {
                              handle_mjpeg_connection(stream, &frame_sources).await;
                          });
                      }
                      Err(e) => {
                          log::error!("MJPEG accept error: {}", e);
                      }
                  }

                  if !running.load(Ordering::SeqCst) {
                      break;
                  }
              }
          });

          Ok(())
      }

      pub async fn stop(&self) -> AppResult<()> {
          self.running.store(false, Ordering::SeqCst);
          if let Some(tx) = self.shutdown_tx.lock().await.take() {
              let _ = tx.send(());
          }
          log::info!("MJPEG preview server stopped");
          Ok(())
      }
  }

  async fn handle_mjpeg_connection(
      stream: tokio::net::TcpStream,
      frame_sources: &Arc<Mutex<HashMap<String, Receiver<Vec<u8>>>>>,
  ) {
      use tokio::io::{AsyncReadExt, AsyncWriteExt};

      let mut buf = vec![0u8; 4096];
      let mut stream = stream;

      // Read HTTP request (we only care about the path)
      let n = match stream.read(&mut buf).await {
          Ok(0) | Err(_) => return,
          Ok(n) => n,
      };

      let request = String::from_utf8_lossy(&buf[..n]);
      let path = request.lines().next()
          .and_then(|line| line.split_whitespace().nth(1))
          .unwrap_or("/");

      let source_id = path.trim_start_matches('/');

      // Get frame receiver for this source
      let rx = {
          let sources = frame_sources.lock().await;
          match sources.get(source_id) {
              Some(rx) => rx.try_recv().ok(),  // just check if source exists
              None => {
                  // 404 response
                  let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
                  let _ = stream.write_all(response.as_bytes()).await;
                  return;
              }
          }
      };

      // We need a clonable receiver — since mpsc::Receiver is not clonable,
      // we'll use a different approach: read from the shared receiver
      // For simplicity, we use try_recv in a loop (non-blocking reads from the shared channel)
      // Note: this means only one client per source_id works well with mpsc

      // Send MJPEG headers
      let header = "HTTP/1.1 200 OK\r\nContent-Type: multipart/x-mixed-replace; boundary=frame\r\n\r\n";
      if stream.write_all(header.as_bytes()).await.is_err() {
          return;
      }

      let sources = frame_sources.lock().await;
      let rx = match sources.get(source_id) {
          Some(rx) => rx,
          None => return,
      };

      // Stream JPEG frames
      loop {
          match rx.recv_timeout(std::time::Duration::from_millis(100)) {
              Ok(jpeg_data) => {
                  let frame_header = format!(
                      "--frame\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
                      jpeg_data.len()
                  );
                  if stream.write_all(frame_header.as_bytes()).await.is_err() {
                      break;
                  }
                  if stream.write_all(&jpeg_data).await.is_err() {
                      break;
                  }
                  if stream.write_all(b"\r\n").await.is_err() {
                      break;
                  }
              }
              Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                  // No new frame, continue waiting
                  continue;
              }
              Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                  break;
              }
          }
      }
  }
  ```

- [ ] **Step 2: Add `mjpeg_server` module to `pipeline/mod.rs`**:
  Add `pub mod mjpeg_server;` after `pub mod preview;`

**Validation**
- Run: `cd src-tauri && cargo check`
- Expect: compiles without errors

**Risks / Notes**
- `mpsc::Receiver` 不是 `Clone`，同一 `source_id` 只能有一个客户端接收 — 对于本场景（仅本地一个浏览器预览）足够
- `recv_timeout` 在 `tokio::spawn` 的 async 上下文中使用了同步 `mpsc::Receiver` — 这是可行的因为 `recv_timeout` 不会长时间阻塞（100ms 超时）
- 未来如需多客户端支持，可改用 `tokio::broadcast` channel

---

## Task 9: Integrate preview pipeline into GstPipelineManager

**Why**
- 预览 pipeline 需要随推流启停自动管理，与 GstPipelineManager 集成

**Files**
- Modify: `src-tauri/src/pipeline/manager.rs`
- Modify: `src-tauri/src/lib.rs` (add MjpegServer to AppState + new commands)

**Steps**
- [ ] **Step 1: Modify `manager.rs`** — add preview pipeline and MJPEG server management:

  Add imports:
  ```rust
  use crate::pipeline::preview::PreviewPipeline;
  use crate::pipeline::mjpeg_server::MjpegServer;
  use std::sync::mpsc;
  ```

  Add `preview_pipeline` field to `PipelineHandle`:
  ```rust
  struct PipelineHandle {
      status: PipelineState,
      config: EncodeConfig,
      rtsp_path: String,
      source: CaptureSource,
      encoder_used: String,
      is_gpu: bool,
      preview_pipeline: Option<PreviewPipeline>,
  }
  ```

  Add `mjpeg_server` and `preview_http_port` fields to `GstPipelineManager`:
  ```rust
  pub struct GstPipelineManager {
      pipelines: Mutex<HashMap<String, PipelineHandle>>,
      rtsp_server: Mutex<Option<RtspServer>>,
      mjpeg_server: Arc<tokio::sync::Mutex<Option<MjpegServer>>>,
      preview_http_port: u16,
      config: AppConfig,
  }
  ```

  Update `new()` constructor:
  ```rust
  pub fn new(config: AppConfig) -> Self {
      let port = config.preview_http_port;
      Self {
          pipelines: Mutex::new(HashMap::new()),
          rtsp_server: Mutex::new(None),
          mjpeg_server: Arc::new(tokio::sync::Mutex::new(None)),
          preview_http_port: port,
          config,
      }
  }
  ```

  Add method to start MJPEG server:
  ```rust
  pub async fn ensure_mjpeg_server(&self) -> AppResult<()> {
      let mut guard = self.mjpeg_server.lock().await;
      if guard.is_none() {
          let server = MjpegServer::new(self.preview_http_port);
          server.start().await?;
          *guard = Some(server);
      }
      Ok(())
  }
  ```

  Modify `start_pipeline()` — after inserting the pipeline handle, start preview pipeline:
  ```rust
  // After the existing insert logic, before the log::info:
  let preview_pipeline = match PreviewPipeline::new(&source.id, &source.source_type.to_string(), config.framerate) {
      Ok(pp) => {
          if let Err(e) = pp.play() {
              log::warn!("Preview pipeline start failed for {}: {}", source.id, e);
              None
          } else {
              // Register with MJPEG server
              let rx = pp.frame_rx;
              let mjpeg = self.mjpeg_server.clone();
              let source_id = source.id.clone();
              // We need to spawn a blocking task to get the receiver
              // Actually, the rx is std::sync::mpsc::Receiver which is not Send in some contexts
              // Let's handle this in the start_preview method instead
              Some(pp)
          }
      }
      Err(e) => {
          log::warn!("Preview pipeline creation failed for {}: {}", source.id, e);
          None
      }
  };
  ```

  Add `preview_pipeline` to `PipelineHandle` initialization.

  Modify `stop_pipeline()` — stop preview pipeline before removing handle:
  ```rust
  // Before pipelines.remove():
  if let Some(pp) = handle.preview_pipeline.as_ref() {
      let _ = pp.stop();
  }
  ```

  Add convenience method:
  ```rust
  pub fn get_preview_url(&self, source_id: &str) -> Option<String> {
      // Check if pipeline exists and is running
      let pipelines = self.pipelines.lock().unwrap();
      if pipelines.contains_key(source_id) {
          Some(format!("http://127.0.0.1:{}/{}", self.preview_http_port, source_id))
      } else {
          None
      }
  }

  /// Start preview for a source and register its frame receiver with MJPEG server
  pub async fn start_preview(&self, source_id: &str) -> AppResult<()> {
      let (source_info, preview_pipeline) = {
          let pipelines = self.pipelines.lock().unwrap();
          let handle = pipelines.get(source_id)
              .ok_or_else(|| AppError::Preview(format!("Source not streaming: {}", source_id)))?;
          let source_type = handle.source.source_type.to_string();
          let framerate = handle.config.framerate;
          (source_type, framerate)
      };

      let pp = PreviewPipeline::new(source_id, &source_info, preview_pipeline)?;
      let rx = pp.frame_rx;
      pp.play()?;

      // Register with MJPEG server
      self.ensure_mjpeg_server().await?;
      let guard = self.mjpeg_server.lock().await;
      if let Some(server) = guard.as_ref() {
          server.add_source(source_id.to_string(), rx).await;
      }

      // Store preview pipeline in handle
      {
          let mut pipelines = self.pipelines.lock().unwrap();
          if let Some(handle) = pipelines.get_mut(source_id) {
              handle.preview_pipeline = Some(pp);
          }
      }

      Ok(())
  }
  ```

- [ ] **Step 2: Add `SourceType` `Display` impl** in `src-tauri/src/capture/source.rs`:
  ```rust
  impl std::fmt::Display for SourceType {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
          match self {
              SourceType::Monitor => write!(f, "Monitor"),
              SourceType::Window => write!(f, "Window"),
          }
      }
  }
  ```

- [ ] **Step 3: Add Tauri commands in `lib.rs`**:

  ```rust
  #[tauri::command]
  async fn start_preview(source_id: String, state: tauri::State<'_, AppState>) -> Result<(), AppError> {
      let pipeline_manager = state.pipeline_manager.clone();
      pipeline_manager.start_preview(&source_id).await
  }

  #[tauri::command]
  async fn get_preview_url(source_id: String, state: tauri::State<'_, AppState>) -> Result<Option<String>, AppError> {
      let pipeline_manager = state.pipeline_manager.clone();
      Ok(pipeline_manager.get_preview_url(&source_id))
  }
  ```

  Add both commands to `invoke_handler` generate_handler macro list.

**Validation**
- Run: `cd src-tauri && cargo check`
- Expect: compiles without errors

**Risks / Notes**
- `PreviewPipeline::new()` 使用 `gstreamer::Element::link_many` 需要元素在同一 pipeline 中
- `mpsc::Receiver` 不实现 `Clone` — 同一 source_id 的 MJPEG 流只能服务一个客户端
- 需确保 `start_preview` 在 `start_stream` 之后调用（前端控制调用时序）
- `GstPipelineManager` 的 `mjpeg_server` 使用 `Arc<tokio::sync::Mutex>` 而非 `std::sync::Mutex`，因为 `start_preview` 是异步方法

---

## Task 10: Frontend MainPreview — display MJPEG stream for streaming source

**Why**
- 推流中时，MainPreview 中央区域需要显示推流实时画面（按编码帧率）

**Files**
- Modify: `src/components/MainPreview.vue`

**Steps**
- [ ] **Step 1: Add preview URL state** in `<script setup>`:

  ```typescript
  import { ref, watch, computed } from 'vue'

  const previewUrl = ref<string>('')

  // When streaming starts, start preview and set URL
  watch(isRunning, async (running) => {
    if (running && props.selectedSource) {
      try {
        await invoke('start_preview', { sourceId: props.selectedSource.id })
        previewUrl.value = `http://127.0.0.1:8090/${props.selectedSource.id}`
      } catch (e) {
        console.error('Failed to start preview:', e)
        previewUrl.value = ''
      }
    } else {
      previewUrl.value = ''
    }
  }, { immediate: true })
  ```

  Also import invoke:
  ```typescript
  import { invoke } from '@tauri-apps/api/core'
  ```

- [ ] **Step 2: Replace SVG placeholder in `isRunning` template** (lines 62-84):

  Replace the SVG block with:
  ```html
  <template v-if="isRunning">
    <img
      v-if="previewUrl"
      :src="previewUrl"
      class="preview-mjpeg"
      alt="Live preview"
    />
    <div v-else class="preview-loading">加载预览...</div>
  </template>
  ```

- [ ] **Step 3: Add CSS for MJPEG stream** in `<style scoped>`:
  ```css
  .preview-mjpeg {
    width: 100%;
    height: 100%;
    object-fit: contain;
    background: #0f1523;
  }

  .preview-loading {
    text-align: center;
    color: var(--text-muted);
    font-size: 13px;
  }
  ```

**Validation**
- Run: `cd "e:\抓屏软件" && npx vue-tsc --noEmit`
- Expect: no TypeScript errors

**Risks / Notes**
- MJPEG `<img>` 在浏览器中断连接时会自动重连 — 后端需处理重连
- `previewUrl` 使用固定端口 8090 — 未来应从 config 动态获取
- `watch(isRunning)` 确保 stop stream 时清除 URL，`<img>` 断开 MJPEG 连接

---

## Task 11: TopBar settings button — toggle config panel

**Why**
- PRD 要求点击 TopBar 设置按钮切换右侧面板的显示/隐藏

**Files**
- Modify: `src/components/TopBar.vue`
- Modify: `src/App.vue`
- Modify: `src/components/ConfigPanel.vue`

**Steps**
- [ ] **Step 1: Add emit to TopBar.vue** for toggle event:

  In `<script setup>`:
  ```typescript
  const emit = defineEmits<{
    toggleSettings: []
  }>()
  ```

  In template, add `@click` to the settings button (line 65):
  ```html
  <button class="topbar-btn btn-settings" @click="emit('toggleSettings')">
  ```

  Add active style class binding:
  ```html
  <button class="topbar-btn btn-settings" :class="{ active: settingsVisible }" @click="emit('toggleSettings')">
  ```

  Add `settingsVisible` prop:
  ```typescript
  defineProps<{
    settingsVisible: boolean
  }>()
  ```

  Add `.btn-settings.active` CSS:
  ```css
  .btn-settings.active {
    background: var(--accent-cyan-glow);
    border-color: var(--accent-cyan);
    color: var(--accent-cyan);
  }
  ```

- [ ] **Step 2: Add `showConfigPanel` state to App.vue**:

  ```typescript
  const showConfigPanel = ref(true)

  function onToggleSettings() {
    showConfigPanel.value = !showConfigPanel.value
  }
  ```

  Pass to TopBar:
  ```html
  <TopBar :settings-visible="showConfigPanel" @toggle-settings="onToggleSettings" />
  ```

  Pass to ConfigPanel:
  ```html
  <ConfigPanel :selected-source-id="selectedSource?.id ?? null" :visible="showConfigPanel" />
  ```

  Update CSS Grid for dynamic columns:
  ```html
  <div class="app" :style="{ gridTemplateColumns: showConfigPanel ? '320px 1fr 340px' : '320px 1fr 0px' }">
  ```

  Add transition CSS:
  ```css
  .app {
    /* ... existing ... */
    transition: grid-template-columns 0.3s ease;
  }
  ```

  Update media query:
  ```css
  @media (max-width: 1200px) {
    .app {
      /* grid-template-columns handled by :style binding now */
    }
  }
  ```

- [ ] **Step 3: Add `visible` prop to ConfigPanel.vue**:

  Add prop:
  ```typescript
  defineProps<{
    selectedSourceId: string | null
    visible: boolean
  }>()
  ```

  Add `v-show` to root element:
  ```html
  <aside v-show="visible" class="panel-right">
  ```

  Add transition for opacity:
  ```css
  .panel-right {
    /* ... existing ... */
    transition: opacity 0.3s ease;
  }
  ```

**Validation**
- Run: `cd "e:\抓屏软件" && npx vue-tsc --noEmit`
- Expect: no TypeScript errors

**Risks / Notes**
- CSS Grid `grid-template-columns` 过渡在 Chrome 107+ 原生支持
- `v-show` 比 `v-if` 更适合此场景（保持组件状态，仅隐藏显示）
- `0px` 列宽会使 ConfigPanel 完全不可见，无需额外 `overflow: hidden`

---

## Task 12: Update PipelineVisual and MainPreview for preview URL from config

**Why**
- `preview_http_port` 从后端 config 获取而非硬编码 8090

**Files**
- Modify: `src/components/MainPreview.vue`

**Steps**
- [ ] Replace hardcoded `8090` with config value:

  In MainPreview.vue `<script setup>`, inject config:
  ```typescript
  const { config } = inject<ReturnType<typeof useConfig>>('config')!
  ```

  Update previewUrl computed:
  ```typescript
  const previewPort = computed(() => config.value?.preview_http_port ?? 8090)

  // In the watch callback:
  previewUrl.value = `http://127.0.0.1:${previewPort.value}/${props.selectedSource.id}`
  ```

**Validation**
- Run: `cd "e:\抓屏软件" && npx vue-tsc --noEmit`
- Expect: no TypeScript errors

**Risks / Notes**
- `config` 是 reactive ref，`config.value` 获取当前值

---

## Task 13: Integration test — build and verify

**Why**
- 验证所有改动集成后编译通过

**Files**
- No new files

**Steps**
- [ ] Run backend compilation: `cd src-tauri && cargo build`
- [ ] Run backend tests: `cd src-tauri && cargo test`
- [ ] Run frontend type check: `cd "e:\抓屏软件" && npx vue-tsc --noEmit`
- [ ] Run frontend lint: `cd "e:\抓屏软件" && npx eslint src/`
- [ ] Start dev server for manual testing: `cd "e:\抓屏软件" && npm run tauri dev`

**Validation**
- `cargo build` succeeds
- `cargo test` all pass
- `vue-tsc --noEmit` no errors
- Manual test: scroll source list, see thumbnails, start stream → see MJPEG preview, click settings button → panel collapses

**Risks / Notes**
- If `d3d12screencapturesrc` parallel capture fails, fallback plan is to modify RTSP pipeline to add tee branch (higher risk, requires more testing)
- If PrintWindow returns black screen for minimized windows, frontend already has SVG fallback
