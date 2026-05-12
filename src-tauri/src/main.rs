// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Set up panic hook to log panics
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("PANIC: {:?}", info);
        log::error!("{}", msg);
        eprintln!("{}", msg);
    }));

    // Initialize logger to file
    let log_path = std::env::temp_dir().join("screencast-pro.log");
    let log_file = std::fs::File::create(&log_path).ok();

    let log_target: Box<dyn std::io::Write + Send> = match log_file {
        Some(f) => Box::new(std::io::BufWriter::new(f)),
        None => Box::new(std::io::stderr()),
    };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .target(env_logger::Target::Pipe(log_target))
        .init();

    log::info!("ScreenCast Pro starting...");

    // Configure GStreamer paths BEFORE gstreamer::init()
    add_gst_dll_search_path();
    configure_gstreamer_paths();
    configure_gst_plugin_scanner();

    // Initialize GStreamer
    match gstreamer::init() {
        Ok(()) => log::info!("GStreamer initialized successfully"),
        Err(e) => {
            log::error!("Failed to initialize GStreamer: {}", e);
            eprintln!("Failed to initialize GStreamer: {}", e);
            std::process::exit(1);
        }
    }

    log::info!("Launching Tauri application...");
    screencast_pro_lib::run();

    log::info!("Application exited normally.");
}

/// Add GStreamer runtime DLL directory to the process search path on Windows.
/// When packaged, GStreamer DLLs are in <exe_dir>/gstreamer-runtime/bin/ (NSIS)
/// or <exe_dir>/../resources/gstreamer-runtime/bin/ (MSI/WiX)
/// which Windows won't search by default.
fn add_gst_dll_search_path() {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default();

    // Try multiple layout patterns:
    // 1. NSIS: <exe_dir>/gstreamer-runtime/bin/
    // 2. MSI:  <exe_dir>/../resources/gstreamer-runtime/bin/
    let gst_bin = find_resource_path(&exe_dir, &["gstreamer-runtime", "bin"]);
    let gst_bin = match gst_bin {
        Some(p) => p,
        None => return, // No GStreamer runtime found
    };

    if gst_bin.is_dir() {
        // On Windows, use SetDllDirectoryW to add the GStreamer bin directory
        // to the DLL search path. This is more reliable than modifying PATH
        // because LoadLibrary checks SetDllDirectory paths before PATH,
        // and it correctly handles recursive DLL dependencies
        // (gstreamer-1.0-0.dll -> glib-2.0-0.dll -> intl-8.dll etc.)
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::ffi::OsStrExt;
            let wide: Vec<u16> = std::ffi::OsStr::new(&gst_bin)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let result = unsafe { SetDllDirectoryW(wide.as_ptr()) };
            if result == 0 {
                log::warn!("SetDllDirectoryW failed for {}", gst_bin.display());
            } else {
                log::info!("SetDllDirectoryW succeeded for {}", gst_bin.display());
            }
        }

        // Also add to PATH for child processes (gst-plugin-scanner) and
        // as fallback for non-Windows platforms
        if let Ok(path) = std::env::var("PATH") {
            std::env::set_var("PATH", format!("{};{}", gst_bin.display(), path));
        } else {
            std::env::set_var("PATH", gst_bin.to_string_lossy().to_string());
        }
        log::info!("Added GStreamer bin to PATH: {}", gst_bin.display());
    }
}

#[cfg(target_os = "windows")]
extern "system" {
    /// https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-setdlldirectoryw
    /// Adds a directory to the search path used to locate DLLs for the application.
    /// Returns nonzero on success.
    fn SetDllDirectoryW(lpPathName: *const u16) -> i32;
}

/// Find a resource path that works with both NSIS and MSI (WiX) installers.
///
/// NSIS layout: `<exe_dir>/<path_segments...>`
/// MSI layout:  `<exe_dir>/../resources/<path_segments...>`
///
/// Returns the first existing path, preferring NSIS layout (exe_dir relative).
fn find_resource_path(exe_dir: &std::path::Path, segments: &[&str]) -> Option<std::path::PathBuf> {
    // 1. NSIS layout: resources next to the exe
    let nsis_path = segments.iter().fold(exe_dir.to_path_buf(), |acc, s| acc.join(s));
    if nsis_path.exists() {
        return Some(nsis_path);
    }

    // 2. MSI (WiX) layout: resources in <exe_dir>/../resources/
    let msi_path = segments.iter().fold(
        exe_dir.join("..").join("resources"),
        |acc, s| acc.join(s),
    );
    if msi_path.exists() {
        // Canonicalize to resolve the ".." for cleaner paths
        return Some(msi_path.canonicalize().unwrap_or(msi_path));
    }

    None
}

/// Configure GST_PLUGIN_PATH based on platform and install layout.
fn configure_gstreamer_paths() {
    if std::env::var("GST_PLUGIN_PATH").is_ok() {
        log::info!("GST_PLUGIN_PATH already set, skipping auto-detection");
        return;
    }

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default();

    let plugin_path = find_gst_plugin_path(&exe_dir);
    if let Some(path) = plugin_path {
        std::env::set_var("GST_PLUGIN_PATH", &path);
        log::info!("GST_PLUGIN_PATH={}", path.display());
    } else {
        log::warn!("GST_PLUGIN_PATH not set and no GStreamer runtime found");
    }
}

fn find_gst_plugin_path(exe_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    // 1. Packaged layout: <exe_dir>/gstreamer-runtime/lib/gstreamer-1.0 (NSIS)
    //    or <exe_dir>/../resources/gstreamer-runtime/lib/gstreamer-1.0 (MSI)
    if let Some(p) = find_resource_path(exe_dir, &["gstreamer-runtime", "lib", "gstreamer-1.0"]) {
        if p.is_dir() {
            return Some(p);
        }
    }

    // 2. Dev layout: <exe_dir>/gstreamer-1.0 (flat layout)
    let dev_flat = exe_dir.join("gstreamer-1.0");
    if dev_flat.is_dir() {
        return Some(dev_flat);
    }

    // 3. Platform-specific development fallbacks
    #[cfg(target_os = "windows")]
    {
        if let Ok(root) = std::env::var("GSTREAMER_1_0_ROOT_MSVC_X86_64") {
            let dev_path = std::path::Path::new(&root).join("lib").join("gstreamer-1.0");
            if dev_path.is_dir() {
                return Some(dev_path);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let homebrew_paths = [
            "/opt/homebrew/lib/gstreamer-1.0",
            "/usr/local/lib/gstreamer-1.0",
        ];
        for path in &homebrew_paths {
            let p = std::path::Path::new(path);
            if p.is_dir() {
                return Some(p.to_path_buf());
            }
        }
        if let Ok(root) = std::env::var("GSTREAMER_1_0_ROOT") {
            let dev_path = std::path::Path::new(&root).join("lib").join("gstreamer-1.0");
            if dev_path.is_dir() {
                return Some(dev_path);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let linux_paths = [
            "/usr/lib/x86_64-linux-gnu/gstreamer-1.0",
            "/usr/lib64/gstreamer-1.0",
            "/usr/lib/gstreamer-1.0",
            "/usr/local/lib/gstreamer-1.0",
        ];
        for path in &linux_paths {
            let p = std::path::Path::new(path);
            if p.is_dir() {
                return Some(p.to_path_buf());
            }
        }
    }

    None
}

/// Configure GST_PLUGIN_SCANNER based on platform and install layout.
fn configure_gst_plugin_scanner() {
    if std::env::var("GST_PLUGIN_SCANNER").is_ok() {
        return;
    }

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default();

    let scanner_path = find_gst_plugin_scanner(&exe_dir);
    if let Some(path) = scanner_path {
        std::env::set_var("GST_PLUGIN_SCANNER", &path);
        log::info!("GST_PLUGIN_SCANNER={}", path.display());
    }
}

fn find_gst_plugin_scanner(exe_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    let scanner_name = if cfg!(target_os = "windows") {
        "gst-plugin-scanner.exe"
    } else {
        "gst-plugin-scanner"
    };

    // 1. Packaged layout: <exe_dir>/gstreamer-runtime/bin/gst-plugin-scanner[.exe] (NSIS)
    //    or <exe_dir>/../resources/gstreamer-runtime/bin/gst-plugin-scanner[.exe] (MSI)
    if let Some(p) = find_resource_path(exe_dir, &["gstreamer-runtime", "bin", scanner_name]) {
        if p.is_file() {
            return Some(p);
        }
    }

    // 2. Packaged layout: <exe_dir>/gstreamer-runtime/lib/gstreamer-1.0/gst-plugin-scanner[.exe]
    //    or <exe_dir>/../resources/gstreamer-runtime/lib/gstreamer-1.0/gst-plugin-scanner[.exe] (MSI)
    //    (alternative location used by prepare-gstreamer-runtime.ps1)
    if let Some(p) = find_resource_path(exe_dir, &["gstreamer-runtime", "lib", "gstreamer-1.0", scanner_name]) {
        if p.is_file() {
            return Some(p);
        }
    }

    // 3. Flat layout: <exe_dir>/gst-plugin-scanner[.exe]
    let flat = exe_dir.join(scanner_name);
    if flat.is_file() {
        return Some(flat);
    }

    // 3. Platform-specific development fallbacks
    #[cfg(target_os = "windows")]
    {
        if let Ok(root) = std::env::var("GSTREAMER_1_0_ROOT_MSVC_X86_64") {
            let dev_scanner = std::path::Path::new(&root)
                .join("bin")
                .join("gst-plugin-scanner.exe");
            if dev_scanner.is_file() {
                return Some(dev_scanner);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let homebrew_scanners = [
            "/opt/homebrew/libexec/gstreamer-1.0/gst-plugin-scanner",
            "/usr/local/libexec/gstreamer-1.0/gst-plugin-scanner",
        ];
        for path in &homebrew_scanners {
            let p = std::path::Path::new(path);
            if p.is_file() {
                return Some(p.to_path_buf());
            }
        }
        if let Ok(root) = std::env::var("GSTREAMER_1_0_ROOT") {
            let dev_scanner = std::path::Path::new(&root)
                .join("libexec")
                .join("gstreamer-1.0")
                .join("gst-plugin-scanner");
            if dev_scanner.is_file() {
                return Some(dev_scanner);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let linux_scanners = [
            "/usr/lib/x86_64-linux-gnu/gstreamer-1.0/gst-plugin-scanner",
            "/usr/lib64/gstreamer-1.0/gst-plugin-scanner",
            "/usr/libexec/gstreamer-1.0/gst-plugin-scanner",
            "/usr/lib/gstreamer-1.0/gst-plugin-scanner",
        ];
        for path in &linux_scanners {
            let p = std::path::Path::new(path);
            if p.is_file() {
                return Some(p.to_path_buf());
            }
        }
    }

    None
}
