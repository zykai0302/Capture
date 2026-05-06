# 04 — 屏幕采集模块

## 1 模块结构

```
capture/
├── mod.rs          → pub mod source; pub mod platform; pub mod hotplug;
├── source.rs       → CaptureSource, CaptureSourceList, SourceType
├── hotplug.rs      → start_hotplug_monitor(), enumerate_sources_with_timeout()
└── platform/
    ├── mod.rs      → cfg gate 分发到 windows / macos / linux
    ├── windows.rs  → enumerate_sources(), enumerate_monitors(), enumerate_windows()
    ├── macos.rs    → enumerate_sources() (CoreGraphics)
    └── linux.rs    → enumerate_sources() (xrandr/DRM)
```

## 2 平台分发 (`platform/mod.rs`)

```rust
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::enumerate_sources;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::enumerate_sources;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::enumerate_sources;
```

> 三平台均已实现，通过 `#[cfg(target_os)]` 条件编译自动选择。

## 3 Windows 实现 (`platform/windows.rs`)

### 3.1 导入

```rust
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, MONITORENUMPROC, MONITORINFOEXW, HMONITOR, HDC,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowRect, GetWindowTextW, IsWindowVisible, GWLP_HWNDPARENT, GetWindowLongPtrW,
};
```

### 3.2 入口函数

```rust
pub fn enumerate_sources() -> AppResult<CaptureSourceList> {
    let monitors = enumerate_monitors()?;
    let windows = enumerate_windows()?;
    Ok(CaptureSourceList { monitors, windows })
}
```

### 3.3 显示器枚举

**API**: `EnumDisplayMonitors(None, None, proc, lparam)`

**回调函数** `monitor_enum_callback`:

```rust
unsafe extern "system" fn monitor_enum_callback(
    hmonitor: HMONITOR, _hdc: HDC, _lprc_clip: *mut RECT, lparam: LPARAM,
) -> BOOL
```

**处理逻辑**:
1. `GetMonitorInfoW(hmonitor, &mut MONITORINFOEXW)` 获取显示器信息
2. 从 `monitorInfo.rcMonitor` 计算宽高: `width = rect.right - rect.left`, `height = rect.bottom - rect.top`
3. 从 `szDevice` 提取设备名 (如 `\\.\DISPLAY1`)
4. 解析显示器索引: 去掉 `\\.\DISPLAY` 前缀，解析为 u32，减 1
5. 构建 CaptureSource:
   - `id`: `format!("screen-{}", monitor_index)`  (如 `"screen-0"`)
   - `name`: `format!("显示器 {}", monitor_index)`  (如 `"显示器 0"`)
   - `source_type`: `SourceType::Monitor`

**lparam 用法**: 传入 `*mut Vec<CaptureSource>` 指针，回调中往列表 push

### 3.4 窗口枚举

**API**: `EnumWindows(Some(enum_windows_callback), lparam)`

**回调函数** `enum_windows_callback`:

```rust
unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL
```

**过滤规则** (不满足则跳过，返回 `BOOL(1)` 继续枚举):

| 规则 | 检查方式 | 原因 |
|------|---------|------|
| 不可见窗口 | `!IsWindowVisible(hwnd)` | 不需要隐藏窗口 |
| 无标题窗口 | `GetWindowTextW` 返回 0 | 无法标识 |
| 子窗口 | `GetWindowLongPtrW(hwnd, GWLP_HWNDPARENT) != 0` | 只需顶层窗口 |
| 系统窗口 | title == "Program Manager" 或 "Windows Input Experience" | 桌面/输入体验 |
| 过小窗口 | width < 50 或 height < 50 | 工具提示等小窗口 |

**CaptureSource 构建**:
- `id`: `format!("window-{}", hwnd.0 as u64)` (如 `"window-12345678"`)
- `name`: 窗口标题 (String::from_utf16_lossy)
- `source_type`: `SourceType::Window`
- `width/height`: `GetWindowRect` → `rect.right - rect.left`, `rect.bottom - rect.top` (max(0))

---

## 4 macOS 实现 (`platform/macos.rs`)

### 4.1 依赖

```rust
use core_graphics::display::{
    CGGetOnlineDisplayList, CGDirectDisplayID, CGDisplayPixelsWide, CGDisplayPixelsHigh,
    CGMainDisplayID, kCGNullWindowID, kCGWindowListOptionOnScreenOnly,
};
use core_foundation::base::TCFType;
use core_foundation::dictionary::CFDictionaryRef;
use core_foundation::string::CFString;
```

Cargo.toml 依赖:
```toml
[target.'cfg(target_os = "macos")'.dependencies]
core-graphics = { version = "0.24", features = ["highsierra"] }
core-foundation = "0.10"
```

### 4.2 显示器枚举

**API**: `CGGetOnlineDisplayList(max_displays, &mut display_ids, &mut display_count)`

**处理逻辑**:
1. 调用 `CGGetOnlineDisplayList` 获取在线显示器 ID 列表
2. 对每个 `CGDirectDisplayID`:
   - 宽高: `CGDisplayPixelsWide(id)`, `CGDisplayPixelsHigh(id)`
   - 主屏检测: `id == CGMainDisplayID()`
3. 构建 CaptureSource:
   - `id`: `format!("screen-{}", index)`
   - `name`: `format!("显示器 {}", index)` (主屏追加 " (主)")
   - `source_type`: `SourceType::Monitor`

### 4.3 窗口枚举

**API**: `CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly, kCGNullWindowID)`

**过滤规则**:

| 规则 | 原因 |
|------|------|
| layer != 0 | 仅普通窗口 |
| 无标题或标题为空 | 无法标识 |
| bounds 面积 < 2500 | 过滤小窗口 |
| onScreen == false | 仅屏幕上可见窗口 |

**CaptureSource 构建**:
- `id`: `format!("window-{}", window_id)`
- `name`: 窗口名称
- `source_type`: `SourceType::Window`

### 4.4 GStreamer 采集元素

| 类型 | 元素 | 属性 |
|------|------|------|
| 显示器 | `avfvideosrc` | `display-index=N` |
| 窗口 | `avfvicesrc` | `window-id=N` |

---

## 5 Linux 实现 (`platform/linux.rs`)

### 5.1 显示器枚举

**优先级策略**: `xrandr --query` → `/sys/class/drm` → 兜底 1920x1080

**方案 1: xrandr** (首选)
1. `Command::new("xrandr").arg("--query")` 执行
2. 解析输出：` connected` 行 → 提取分辨率 `WIDTHxHEIGHT`
3. 分辨率解析：`parse_resolution("1920x1080")` → `(1920, 1080)`

**方案 2: /sys/class/drm** (备选)
1. 读取 `/sys/class/drm/` 目录
2. 过滤 `card*-` 开头的目录（排除虚拟 `Virtual-1`）
3. 从 `modes` 文件读取第一个分辨率

**方案 3: 兜底**
- 返回单个 1920x1080 显示器

### 5.2 窗口枚举

当前返回空列表（X11 窗口枚举需要 `x11rb` 或 `xcb` 依赖，后续集成）。

### 5.3 GStreamer 采集元素

| 类型 | 元素 | 属性 |
|------|------|------|
| 显示器 | `ximagesrc` | `monitor-index=N` |
| 窗口 | `ximagesrc` | `xid=N` (未来) |

> **Wayland**: `pipewiresrc` (未来支持，需 PipeWire + portal 权限)

---

## 6 热插拔监控 (`hotplug.rs`)

### 6.1 常量

| 常量 | 值 | 说明 |
|------|-----|------|
| `ENUM_TIMEOUT_SECS` | `10` | 枚举超时（防止 Win32 API 阻塞） |

### 6.2 `start_hotplug_monitor(app: AppHandle, pipeline_manager: Arc<GstPipelineManager>)`

在独立线程中运行，生命周期与进程一致：

```
1. 首次调用 enumerate_sources_with_timeout() → last_sources
2. loop {
     sleep(2s)
     current_sources = enumerate_sources_with_timeout()
     
     // 检测新增源
     for source in current_sources.all():
       if !last_sources.contains_id(source.id):
         log::info!("Source added: {}", source.id)
         app.emit("source-added", source)
     
     // 检测移除源
     for source in last_sources.all():
       if !current_sources.contains_id(source.id):
         log::info!("Source removed: {}, stopping pipeline if running", source.id)
         pipeline_manager.stop_pipeline(&source.id)  // 自动停止 Pipeline
         app.emit("source-removed", source)
     
     last_sources = current_sources
   }
```

### 6.3 `enumerate_sources_with_timeout() -> Result<CaptureSourceList, String>`

超时保护机制：

```
1. 创建 mpsc::channel
2. spawn 新线程调用 enumerate_sources()，结果通过 tx.send 发送
3. rx.recv_timeout(Duration::from_secs(10))
   - Ok(Ok(result)) → 返回 result
   - Ok(Err(e)) → Err("Enumeration error: {e}")
   - Err(Timeout) → Err("Enumeration timed out after 10s (possible Win32 API stall)")
   - Err(Disconnected) → Err("Enumeration thread panicked")
```

**为什么需要超时**: Win32 API 在 UAC 屏幕、安全桌面、或窗口无响应时可能永久阻塞。
