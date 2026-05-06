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

    // Configure GStreamer paths.
    // Production: resolve relative to the executable directory (packaged alongside).
    // Development: fall back to GSTREAMER_1_0_ROOT_MSVC_X86_64 env or common install paths.
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default();

    if std::env::var("GST_PLUGIN_PATH").is_err() {
        // 1. Try <exe_dir>/gstreamer-1.0 (packaged layout)
        let plugin_path = exe_dir.join("gstreamer-1.0");
        if plugin_path.is_dir() {
            std::env::set_var("GST_PLUGIN_PATH", &plugin_path);
        } else if let Ok(root) = std::env::var("GSTREAMER_1_0_ROOT_MSVC_X86_64") {
            // 2. Fallback: GSTREAMER_1_0_ROOT_MSVC_X86_64/lib/gstreamer-1.0 (dev)
            let dev_path = std::path::Path::new(&root).join("lib").join("gstreamer-1.0");
            if dev_path.is_dir() {
                std::env::set_var("GST_PLUGIN_PATH", &dev_path);
            }
        }
        if let Ok(p) = std::env::var("GST_PLUGIN_PATH") {
            log::info!("GST_PLUGIN_PATH={}", p);
        } else {
            log::warn!("GST_PLUGIN_PATH not set and no GStreamer runtime found");
        }
    }

    if std::env::var("GST_PLUGIN_SCANNER").is_err() {
        // 1. Try <exe_dir>/gst-plugin-scanner.exe (packaged layout)
        let scanner = exe_dir.join("gst-plugin-scanner.exe");
        if scanner.is_file() {
            std::env::set_var("GST_PLUGIN_SCANNER", &scanner);
        } else if let Ok(root) = std::env::var("GSTREAMER_1_0_ROOT_MSVC_X86_64") {
            // 2. Fallback: GSTREAMER_1_0_ROOT_MSVC_X86_64/bin/gst-plugin-scanner.exe (dev)
            let dev_scanner = std::path::Path::new(&root)
                .join("bin")
                .join("gst-plugin-scanner.exe");
            if dev_scanner.is_file() {
                std::env::set_var("GST_PLUGIN_SCANNER", &dev_scanner);
            }
        }
        if let Ok(p) = std::env::var("GST_PLUGIN_SCANNER") {
            log::info!("GST_PLUGIN_SCANNER={}", p);
        }
    }

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
