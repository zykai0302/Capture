// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::Write;

fn main() {
    // Set up panic hook to log panics
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("PANIC: {:?}", info);
        log::error!("{}", msg);
        eprintln!("{}", msg);
    }));

    // Initialize logger to file for debugging
    let log_path = std::env::temp_dir().join("screencast-pro.log");
    let log_file = std::fs::File::create(&log_path).ok();

    let log_target: Box<dyn Write + Send> = match log_file {
        Some(f) => Box::new(std::io::BufWriter::new(f)),
        None => Box::new(std::io::stderr()),
    };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .target(env_logger::Target::Pipe(log_target))
        .init();

    log::info!("ScreenCast Pro starting...");
    log::info!("Log file: {:?}", log_path);
    log::info!("PID: {}", std::process::id());

    // Check GStreamer plugin path
    if let Ok(plugin_path) = std::env::var("GST_PLUGIN_PATH") {
        log::info!("GST_PLUGIN_PATH={}", plugin_path);
    } else {
        log::warn!("GST_PLUGIN_PATH not set!");
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
