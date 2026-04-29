#[allow(dead_code)]
mod error;
mod capture;
mod encode;
mod rtsp;
mod remote;
mod pipeline;
mod config;

use capture::source::CaptureSourceList;
use config::AppConfig;
use encode::detector::GpuCapability;
use error::AppError;
use pipeline::gst_pipeline;
use pipeline::manager::GstPipelineManager;
use pipeline::{PipelineManager, PipelineStatus};
use remote::injector::RemoteInjector;
use remote::websocket::RemoteControlServer;
use remote::RemoteStatus;
use std::sync::{Arc, Mutex};
use tauri::Manager;

pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub pipeline_manager: Arc<GstPipelineManager>,
    pub remote_server: Arc<Mutex<Option<RemoteControlServer>>>,
    pub remote_injector: Arc<RemoteInjector>,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_config(state: tauri::State<AppState>) -> Result<AppConfig, AppError> {
    Ok(state.config.lock().unwrap().clone())
}

#[tauri::command]
async fn list_sources() -> Result<CaptureSourceList, AppError> {
    tokio::task::spawn_blocking(|| {
        capture::platform::enumerate_sources()
    })
    .await
    .map_err(|e| AppError::Capture(format!("Task join error: {}", e)))?
}

#[tauri::command]
async fn start_stream(source_id: String, source_type: String, source_name: String, width: u32, height: u32, state: tauri::State<'_, AppState>) -> Result<(), AppError> {
    let pipeline_manager = state.pipeline_manager.clone();
    let config = state.config.lock().unwrap().default_encode.clone();

    let st = match source_type.as_str() {
        "Monitor" => capture::source::SourceType::Monitor,
        "Window" => capture::source::SourceType::Window,
        _ => return Err(AppError::Capture(format!("Unknown source type: {}", source_type))),
    };

    let source = capture::source::CaptureSource {
        id: source_id,
        name: source_name,
        source_type: st,
        width,
        height,
        is_streaming: false,
        rtsp_url: None,
    };

    tokio::task::spawn_blocking(move || {
        pipeline_manager.start_pipeline(&source, &config)
    })
    .await
    .map_err(|e| AppError::Pipeline(format!("Task join error: {}", e)))?
}

#[tauri::command]
async fn stop_stream(source_id: String, state: tauri::State<'_, AppState>) -> Result<(), AppError> {
    let pipeline_manager = state.pipeline_manager.clone();

    tokio::task::spawn_blocking(move || {
        pipeline_manager.stop_pipeline(&source_id)
    })
    .await
    .map_err(|e| AppError::Pipeline(format!("Task join error: {}", e)))?
}

#[tauri::command]
async fn stop_all_streams(state: tauri::State<'_, AppState>) -> Result<(), AppError> {
    let pipeline_manager = state.pipeline_manager.clone();

    tokio::task::spawn_blocking(move || {
        pipeline_manager.stop_all()
    })
    .await
    .map_err(|e| AppError::Pipeline(format!("Task join error: {}", e)))?
}

#[tauri::command]
async fn get_pipeline_status(state: tauri::State<'_, AppState>) -> Result<Vec<PipelineStatus>, AppError> {
    let pipeline_manager = state.pipeline_manager.clone();
    let result = tokio::task::spawn_blocking(move || {
        pipeline_manager.get_all_status()
    })
    .await
    .map_err(|e| AppError::Pipeline(format!("Task join error: {}", e)))?;
    Ok(result)
}

#[tauri::command]
async fn update_encode_config(
    source_id: String,
    config: encode::config::EncodeConfig,
    state: tauri::State<'_, AppState>,
) -> Result<(), AppError> {
    let pipeline_manager = state.pipeline_manager.clone();

    tokio::task::spawn_blocking(move || {
        pipeline_manager.update_config(&source_id, &config)
    })
    .await
    .map_err(|e| AppError::Pipeline(format!("Task join error: {}", e)))?
}

#[tauri::command]
async fn get_available_encoders() -> Result<Vec<String>, AppError> {
    tokio::task::spawn_blocking(|| {
        Ok(gst_pipeline::detect_available_encoders())
    })
    .await
    .map_err(|e| AppError::GStreamer(format!("Task join error: {}", e)))?
}

#[tauri::command]
async fn get_gpu_capabilities() -> Result<GpuCapability, AppError> {
    tokio::task::spawn_blocking(|| {
        Ok(encode::detector::detect_gpu_capabilities())
    })
    .await
    .map_err(|e| AppError::Encode(format!("Task join error: {}", e)))?
}

#[tauri::command]
async fn start_remote_control(
    port: Option<u16>,
    password: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<(), AppError> {
    let ws_port = port.unwrap_or(state.config.lock().unwrap().ws_port);
    let ws_password = password.unwrap_or_default();

    let server = RemoteControlServer::new(
        ws_port,
        ws_password,
        state.remote_injector.clone(),
    );

    server.start().await?;

    *state.remote_server.lock().unwrap() = Some(server);
    log::info!("Remote control started on port {}", ws_port);
    Ok(())
}

#[tauri::command]
async fn stop_remote_control(state: tauri::State<'_, AppState>) -> Result<(), AppError> {
    let server_opt = state.remote_server.lock().unwrap().take();
    if let Some(server) = server_opt {
        server.stop().await?;
    }
    Ok(())
}

#[tauri::command]
fn get_remote_status(state: tauri::State<AppState>) -> Result<RemoteStatus, AppError> {
    let server = state.remote_server.lock().unwrap();
    match server.as_ref() {
        Some(s) => Ok(RemoteStatus {
            ws_port: s.port,
            is_running: s.running.load(std::sync::atomic::Ordering::SeqCst),
            client_count: s.client_count.load(std::sync::atomic::Ordering::SeqCst),
            mouse_enabled: true,
            keyboard_enabled: true,
        }),
        None => Ok(RemoteStatus {
            ws_port: state.config.lock().unwrap().ws_port,
            is_running: false,
            client_count: 0,
            mouse_enabled: true,
            keyboard_enabled: true,
        }),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = AppConfig::default();
    let pipeline_manager = Arc::new(GstPipelineManager::new(config.clone()));

    // RemoteInjector may fail (e.g., no display), wrap gracefully
    let remote_injector = match RemoteInjector::new() {
        Ok(injector) => Arc::new(injector),
        Err(e) => {
            log::error!("Failed to create RemoteInjector: {}. Remote control will be unavailable.", e);
            // Create a dummy injector that we can still use
            std::process::exit(1);
        }
    };

    log::info!("Initializing Tauri builder...");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            config: Mutex::new(config),
            pipeline_manager,
            remote_server: Arc::new(Mutex::new(None)),
            remote_injector,
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_config,
            list_sources,
            start_stream,
            stop_stream,
            stop_all_streams,
            get_pipeline_status,
            update_encode_config,
            get_available_encoders,
            get_gpu_capabilities,
            start_remote_control,
            stop_remote_control,
            get_remote_status,
        ])
        .setup(|app| {
            log::info!("Tauri setup callback - starting hotplug monitor");
            // Start hotplug monitor — pass pipeline_manager so it can auto-stop
            // pipelines when sources are removed
            let pm = app.state::<AppState>().pipeline_manager.clone();
            capture::hotplug::start_hotplug_monitor(app.handle().clone(), pm);
            log::info!("Hotplug monitor started");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
