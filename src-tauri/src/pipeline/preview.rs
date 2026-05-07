use crate::error::{AppError, AppResult};
use gstreamer::prelude::*;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct PreviewPipeline {
    pipeline: gstreamer::Pipeline,
    #[allow(dead_code)]
    frame_tx: Arc<mpsc::SyncSender<Vec<u8>>>,
    pub frame_rx: Option<mpsc::Receiver<Vec<u8>>>,
}

impl PreviewPipeline {
    /// Create a preview pipeline for a window source.
    /// First tries WGC mode (direct window capture, consistent with thumbnail).
    /// If WGC play fails, caller should use `new_dxgi_crop_fallback` instead.
    pub fn new(source_id: &str, source_type: &str, framerate: u32, frame_count: Arc<AtomicU64>,
               source_x: i32, source_y: i32, source_w: u32, source_h: u32, source_handle: u64) -> AppResult<Self> {
        gstreamer::init().map_err(|e| AppError::GStreamer(format!("GStreamer init: {}", e)))?;

        let capture_str = build_preview_capture_string(source_id, source_type, source_x, source_y, source_w, source_h, source_handle);
        let launch_str = build_full_launch_string(&capture_str, framerate);

        log::info!("Preview pipeline launch string: {}", launch_str);

        let pipeline = gstreamer::parse::launch(&launch_str)
            .map_err(|e| AppError::GStreamer(format!("Preview pipeline parse: {}", e)))?;

        let pipeline = pipeline
            .downcast::<gstreamer::Pipeline>()
            .map_err(|_| AppError::GStreamer("Preview pipeline downcast failed".to_string()))?;

        let (tx_arc, rx) = setup_appsink(&pipeline, frame_count.clone())?;
        setup_bus_watch(&pipeline, source_id)?;

        Ok(PreviewPipeline {
            pipeline,
            frame_tx: tx_arc,
            frame_rx: Some(rx),
        })
    }

    /// Create a DXGI+crop fallback preview pipeline (used when WGC fails for a window).
    pub fn new_dxgi_crop_fallback(source_id: &str, framerate: u32, frame_count: Arc<AtomicU64>,
                                   source_x: i32, source_y: i32, source_w: u32, source_h: u32,
                                   monitor_handle: u64) -> AppResult<Self> {
        gstreamer::init().map_err(|e| AppError::GStreamer(format!("GStreamer init: {}", e)))?;

        let capture_str = crate::pipeline::gst_pipeline::build_dxgi_crop_preview_capture_string(
            source_id, source_x, source_y, source_w, source_h, monitor_handle
        );
        let launch_str = build_full_launch_string(&capture_str, framerate);

        log::info!("DXGI+crop fallback preview launch string: {}", launch_str);

        let pipeline = gstreamer::parse::launch(&launch_str)
            .map_err(|e| AppError::GStreamer(format!("DXGI+crop fallback parse: {}", e)))?;

        let pipeline = pipeline
            .downcast::<gstreamer::Pipeline>()
            .map_err(|_| AppError::GStreamer("Preview pipeline downcast failed".to_string()))?;

        let (tx_arc, rx) = setup_appsink(&pipeline, frame_count.clone())?;
        setup_bus_watch(&pipeline, source_id)?;

        Ok(PreviewPipeline {
            pipeline,
            frame_tx: tx_arc,
            frame_rx: Some(rx),
        })
    }

    /// Start the pipeline playing.
    pub fn play(&self) -> AppResult<()> {
        match self.pipeline.set_state(gstreamer::State::Playing) {
            Ok(_) => {
                log::info!("Preview pipeline set to Playing state");
                Ok(())
            }
            Err(e) => {
                let msg = format!("Pipeline play failed: {}", e);
                log::error!("{}", msg);
                Err(AppError::Preview(msg))
            }
        }
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

/// Build full launch string from capture element string and framerate
fn build_full_launch_string(capture_str: &str, framerate: u32) -> String {
    let uses_d3d11 = capture_str.starts_with("d3d11screencapturesrc");
    let uses_d3d12 = capture_str.starts_with("d3d12screencapturesrc");

    if uses_d3d11 {
        format!(
            "{} ! d3d11colorconvert ! d3d11download ! videoconvert ! capsfilter caps=video/x-raw,framerate={}/1 ! jpegenc quality=60 ! appsink name=sink emit-signals=true max-buffers=1 drop=true",
            capture_str, framerate
        )
    } else if uses_d3d12 {
        format!(
            "{} ! d3d12colorconvert ! d3d12download ! videoconvert ! capsfilter caps=video/x-raw,framerate={}/1 ! jpegenc quality=60 ! appsink name=sink emit-signals=true max-buffers=1 drop=true",
            capture_str, framerate
        )
    } else {
        format!(
            "{} ! videoconvert ! capsfilter caps=video/x-raw,framerate={}/1 ! jpegenc quality=60 ! appsink name=sink emit-signals=true max-buffers=1 drop=true",
            capture_str, framerate
        )
    }
}

/// Set up appsink callbacks on the pipeline
fn setup_appsink(pipeline: &gstreamer::Pipeline, frame_count: Arc<AtomicU64>) -> AppResult<(Arc<mpsc::SyncSender<Vec<u8>>>, mpsc::Receiver<Vec<u8>>)> {
    let sink_element = pipeline.by_name("sink")
        .ok_or_else(|| AppError::GStreamer("appsink 'sink' not found".to_string()))?;
    let appsink = sink_element
        .downcast::<gstreamer_app::AppSink>()
        .map_err(|_| AppError::GStreamer("sink downcast to AppSink failed".to_string()))?;

    let (tx, rx) = mpsc::sync_channel(2);
    let tx_arc = Arc::new(tx);
    let tx_clone = tx_arc.clone();

    appsink.set_callbacks(
        gstreamer_app::AppSinkCallbacks::builder()
            .new_sample(move |appsink| {
                let sample = match appsink.pull_sample() {
                    Ok(s) => s,
                    Err(_) => return Err(gstreamer::FlowError::Error),
                };
                let buffer = match sample.buffer() {
                    Some(b) => b,
                    None => return Err(gstreamer::FlowError::Error),
                };
                let map = match buffer.map_readable() {
                    Ok(m) => m,
                    Err(_) => return Err(gstreamer::FlowError::Error),
                };
                let data = map.as_slice().to_vec();
                let _ = tx_clone.send(data);
                frame_count.fetch_add(1, Ordering::Relaxed);
                Ok(gstreamer::FlowSuccess::Ok)
            })
            .build(),
    );

    Ok((tx_arc, rx))
}

/// Set up bus watch for pipeline error logging
fn setup_bus_watch(pipeline: &gstreamer::Pipeline, source_id: &str) -> AppResult<()> {
    let bus = pipeline.bus().unwrap();
    let source_id_log = source_id.to_string();
    let _ = bus.add_watch(move |_bus, msg| {
        match msg.view() {
            gstreamer::MessageView::Error(err) => {
                log::error!("Preview pipeline error for {}: {} ({})", source_id_log, err.error(), err.debug().unwrap_or_default());
            }
            gstreamer::MessageView::Warning(w) => {
                log::warn!("Preview pipeline warning for {}: {}", source_id_log, w.error());
            }
            gstreamer::MessageView::StateChanged(s) => {
                if s.current() == gstreamer::State::Playing {
                    log::info!("Preview pipeline for {} is now PLAYING", source_id_log);
                }
            }
            _ => {}
        }
        gstreamer::glib::ControlFlow::Continue
    }).map_err(|e| AppError::GStreamer(format!("Bus watch failed: {}", e)))?;
    Ok(())
}

/// Build capture source element string for preview.
/// For windows: try WGC first (consistent with thumbnail).
/// If WGC play fails, caller uses `new_dxgi_crop_fallback`.
fn build_preview_capture_string(source_id: &str, source_type: &str, _source_x: i32, _source_y: i32, _source_w: u32, _source_h: u32, source_handle: u64) -> String {
    match source_type {
        "Monitor" => {
            // Prefer monitor-handle (HMONITOR) to avoid index mapping issues
            if source_handle != 0 {
                if gstreamer::ElementFactory::find("d3d11screencapturesrc").is_some() {
                    log::info!("Using d3d11screencapturesrc monitor-handle={} for monitor preview", source_handle);
                    format!("d3d11screencapturesrc monitor-handle={}", source_handle)
                } else if gstreamer::ElementFactory::find("d3d12screencapturesrc").is_some() {
                    log::info!("Using d3d12screencapturesrc monitor-handle={} for monitor preview", source_handle);
                    format!("d3d12screencapturesrc monitor-handle={}", source_handle)
                } else {
                    format!("d3d11screencapturesrc monitor-handle={}", source_handle)
                }
            } else {
                let monitor_index: i32 = source_id
                    .strip_prefix("screen-")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                if gstreamer::ElementFactory::find("d3d11screencapturesrc").is_some() {
                    log::info!("Using d3d11screencapturesrc monitor-index={} for monitor preview", monitor_index);
                    format!("d3d11screencapturesrc monitor-index={}", monitor_index)
                } else if gstreamer::ElementFactory::find("d3d12screencapturesrc").is_some() {
                    log::info!("Using d3d12screencapturesrc monitor-index={} for monitor preview", monitor_index);
                    format!("d3d12screencapturesrc monitor-index={}", monitor_index)
                } else {
                    format!("d3d11screencapturesrc monitor-index={}", monitor_index)
                }
            }
        }
        "Window" => {
            // Try WGC mode first — direct window capture (consistent with thumbnail).
            // If play() fails, caller will fall back to DXGI+crop.
            let hwnd: u64 = source_id
                .strip_prefix("window-")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);

            if gstreamer::ElementFactory::find("d3d11screencapturesrc").is_some() {
                log::info!("Using d3d11screencapturesrc WGC for window preview hwnd={}", hwnd);
                format!(
                    "d3d11screencapturesrc capture-api=wgc window-handle={} window-capture-mode=default",
                    hwnd
                )
            } else if gstreamer::ElementFactory::find("d3d12screencapturesrc").is_some() {
                log::info!("Using d3d12screencapturesrc WGC for window preview hwnd={}", hwnd);
                format!(
                    "d3d12screencapturesrc capture-api=wgc window-handle={} window-capture-mode=default",
                    hwnd
                )
            } else {
                format!(
                    "d3d11screencapturesrc capture-api=wgc window-handle={} window-capture-mode=default",
                    hwnd
                )
            }
        }
        _ => "d3d11screencapturesrc".to_string(),
    }
}
