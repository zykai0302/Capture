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
    // 1. Packaged layout: <exe_dir>/gstreamer-1.0 (all platforms)
    let packaged = exe_dir.join("gstreamer-1.0");
    if packaged.is_dir() {
        return Some(packaged);
    }

    // 2. Platform-specific development fallbacks
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
        // Homebrew install paths
        let homebrew_paths = [
            "/opt/homebrew/lib/gstreamer-1.0",       // Apple Silicon
            "/usr/local/lib/gstreamer-1.0",           // Intel Macs
        ];
        for path in &homebrew_paths {
            let p = std::path::Path::new(path);
            if p.is_dir() {
                return Some(p.to_path_buf());
            }
        }
        // Also check GSTREAMER_1_0_ROOT (official GStreamer macOS installer)
        if let Ok(root) = std::env::var("GSTREAMER_1_0_ROOT") {
            let dev_path = std::path::Path::new(&root).join("lib").join("gstreamer-1.0");
            if dev_path.is_dir() {
                return Some(dev_path);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Standard Linux GStreamer plugin paths
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
    // 1. Packaged layout: <exe_dir>/gst-plugin-scanner[.exe]
    let scanner_name = if cfg!(target_os = "windows") {
        "gst-plugin-scanner.exe"
    } else {
        "gst-plugin-scanner"
    };
    let packaged = exe_dir.join(scanner_name);
    if packaged.is_file() {
        return Some(packaged);
    }

    // 2. Platform-specific development fallbacks
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
