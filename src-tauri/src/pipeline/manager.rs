use crate::capture::source::CaptureSource;
use crate::config::AppConfig;
use crate::encode::config::{Codec, EncodeConfig, EncodeMode};
use crate::error::{AppError, AppResult};
use crate::pipeline::gst_pipeline;
use crate::pipeline::{PipelineManager, PipelineState, PipelineStatus};
use crate::pipeline::preview::PreviewPipeline;
use crate::pipeline::mjpeg_server::MjpegServer;
use crate::rtsp::RtspServer;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tokio::sync::Mutex as AsyncMutex;

struct PipelineHandle {
    status: PipelineState,
    config: EncodeConfig,
    rtsp_path: String,
    source: CaptureSource,
    encoder_used: String,
    is_gpu: bool,
    preview_pipeline: Option<PreviewPipeline>,
    /// Frame counter for FPS calculation (atomically incremented by preview callback)
    frame_count: Arc<AtomicU64>,
    /// Timestamp when pipeline started (for FPS calculation)
    started_at: Instant,
}

pub struct GstPipelineManager {
    pipelines: Mutex<HashMap<String, PipelineHandle>>,
    rtsp_server: Mutex<Option<RtspServer>>,
    config: AppConfig,
    mjpeg_server: Arc<AsyncMutex<Option<MjpegServer>>>,
    preview_http_port: u16,
}

impl GstPipelineManager {
    pub fn new(config: AppConfig) -> Self {
        let port = config.preview_http_port;
        Self {
            pipelines: Mutex::new(HashMap::new()),
            rtsp_server: Mutex::new(None),
            config,
            mjpeg_server: Arc::new(AsyncMutex::new(None)),
            preview_http_port: port,
        }
    }

    pub fn ensure_rtsp_server(&self) -> AppResult<()> {
        // Lock first, then check — avoids TOCTOU race where two threads both
        // see is_none() and try to start the server simultaneously.
        let mut guard = self.rtsp_server.lock().unwrap();
        if guard.is_none() {
            let server = RtspServer::new(self.config.rtsp_port)?;
            server.start()?;
            *guard = Some(server);
        }
        Ok(())
    }

    /// Get RTSP URL by path (does not need pipelines lock)
    fn get_rtsp_url_by_path(&self, rtsp_path: &str) -> Option<String> {
        let server = self.rtsp_server.lock().unwrap();
        server.as_ref().map(|s| s.rtsp_url(rtsp_path))
    }

    pub async fn ensure_mjpeg_server(&self) -> AppResult<()> {
        let mut guard = self.mjpeg_server.lock().await;
        if guard.is_none() {
            let server = MjpegServer::new(self.preview_http_port);
            server.start().await?;
            *guard = Some(server);
        }
        Ok(())
    }

    pub async fn start_preview(&self, source_id: &str) -> AppResult<()> {
        log::info!("start_preview called for source_id={}", source_id);
        let (source_type, framerate, frame_count, source_x, source_y, source_w, source_h, source_handle) = {
            let pipelines = self.pipelines.lock().unwrap();
            let handle = pipelines.get(source_id)
                .ok_or_else(|| AppError::Preview(format!("Source not streaming: {}", source_id)))?;
            (handle.source.source_type.to_string(), handle.config.framerate, handle.frame_count.clone(),
             handle.source.x, handle.source.y, handle.source.width, handle.source.height,
             handle.source.handle)
        };

        log::info!("start_preview: source_type={}, framerate={}, pos=({},{}) size={}x{}", source_type, framerate, source_x, source_y, source_w, source_h);

        let mut pp = if source_type == "Window" {
            // Try WGC first — it captures only the target window content,
            // so overlapping windows are NOT shown. If WGC play fails
            // (e.g., PixPin overlay, UWP apps), fall back to DXGI+crop.
            let wgc_pp = PreviewPipeline::new(source_id, &source_type, framerate, frame_count.clone(), source_x, source_y, source_w, source_h, source_handle)?;
            match wgc_pp.play() {
                Ok(()) => {
                    log::info!("WGC preview started for window {}", source_id);
                    wgc_pp
                }
                Err(e) => {
                    log::warn!("WGC play failed for window {}: {}, falling back to DXGI+crop", source_id, e);
                    let fallback_pp = PreviewPipeline::new_dxgi_crop_fallback(
                        source_id, framerate, frame_count.clone(), source_x, source_y, source_w, source_h, source_handle
                    )?;
                    fallback_pp.play()?;
                    log::info!("DXGI+crop fallback preview started for window {}", source_id);
                    fallback_pp
                }
            }
        } else {
            // Monitor: no fallback needed
            let pp = PreviewPipeline::new(source_id, &source_type, framerate, frame_count, source_x, source_y, source_w, source_h, source_handle)?;
            pp.play()?;
            pp
        };

        let rx = pp.frame_rx.take()
            .ok_or_else(|| AppError::Preview("frame_rx already taken".to_string()))?;

        self.ensure_mjpeg_server().await?;

        // Create broadcast channel and register with MJPEG server,
        // then spawn a forwarding task from mpsc → broadcast.
        let (btx, _) = tokio::sync::broadcast::channel::<Vec<u8>>(2);

        // Register the broadcast sender with the MJPEG server
        {
            let guard = self.mjpeg_server.lock().await;
            if let Some(server) = guard.as_ref() {
                server.add_source(source_id.to_string(), btx.clone()).await;
            }
        }

        // Forward frames from mpsc (appsink) to broadcast (MJPEG clients)
        // Note: we intentionally do NOT clean up the MJPEG source here when
        // the forwarding loop ends. The source is always removed explicitly
        // by stop_preview() or stop_pipeline() — removing it here would race
        // with a concurrent start_preview() that registered a new source
        // under the same source_id, silently deleting it.
        let _source_id_owned = source_id.to_string();
        let _mjpeg_server = self.mjpeg_server.clone();
        tokio::task::spawn_blocking(move || {
            loop {
                match rx.recv() {
                    Ok(frame) => {
                        // Send to all broadcast receivers; if none exist, that's fine
                        let _ = btx.send(frame);
                    }
                    Err(_) => break,
                }
            }
        });

        {
            let mut pipelines = self.pipelines.lock().unwrap();
            if let Some(handle) = pipelines.get_mut(source_id) {
                handle.preview_pipeline = Some(pp);
            }
        }

        Ok(())
    }

    pub fn get_preview_url(&self, source_id: &str) -> Option<String> {
        let pipelines = self.pipelines.lock().unwrap();
        if pipelines.contains_key(source_id) {
            Some(format!("http://127.0.0.1:{}/{}", self.preview_http_port, source_id))
        } else {
            None
        }
    }

    /// Clone the MJPEG server Arc for use by RTSP client manager
    pub fn mjpeg_server_clone(&self) -> Arc<AsyncMutex<Option<MjpegServer>>> {
        self.mjpeg_server.clone()
    }

    pub async fn remove_preview_source(&self, source_id: &str) {
        let guard = self.mjpeg_server.lock().await;
        if let Some(server) = guard.as_ref() {
            server.remove_source(source_id).await;
        }
    }

    pub async fn stop_preview(&self, source_id: &str) {
        // Stop preview pipeline and remove from MJPEG server.
        // Only remove MJPEG source if we actually stopped a preview pipeline —
        // this avoids a race where stop_preview removes a newly-added source
        // that was registered by a concurrent start_preview call.
        let had_preview = {
            let mut pipelines = self.pipelines.lock().unwrap();
            if let Some(handle) = pipelines.get_mut(source_id) {
                if let Some(pp) = handle.preview_pipeline.take() {
                    let _ = pp.stop();
                    true
                } else {
                    false
                }
            } else {
                // Pipeline doesn't exist — don't touch MJPEG server
                false
            }
        };
        if had_preview {
            self.remove_preview_source(source_id).await;
        }
    }
}

impl PipelineManager for GstPipelineManager {
    fn start_pipeline(&self, source: &CaptureSource, config: &EncodeConfig) -> AppResult<()> {
        // Ensure RTSP server is running
        self.ensure_rtsp_server()?;

        let rtsp_path = format!("/{}", source.id);

        // Build launch string for gst-rtsp-server (DXGI+crop for windows)
        let launch_str = gst_pipeline::build_launch_string(source, config);
        log::info!("RTSP pipeline launch string for {}: {}", source.id, launch_str);

        // Validate the pipeline can be constructed before adding to RTSP server.
        if let Err(e) = gst_pipeline::validate_pipeline_launch(&launch_str) {
            log::error!("Pipeline validation failed for {}: {}", source.id, e);
            return Err(AppError::Pipeline(format!("Pipeline 创建失败: {}", e)));
        }

        // Insert a "Starting" handle first so get_status returns correct state
        let (encoder_used, is_gpu) = detect_encoder_info(config);

        let handle = PipelineHandle {
            status: PipelineState::Starting,
            config: config.clone(),
            rtsp_path: rtsp_path.clone(),
            source: source.clone(),
            encoder_used: encoder_used.clone(),
            is_gpu,
            preview_pipeline: None,
            frame_count: Arc::new(AtomicU64::new(0)),
            started_at: Instant::now(),
        };

        self.pipelines
            .lock()
            .unwrap()
            .insert(source.id.clone(), handle);

        // Add stream to RTSP server — release pipelines lock first to avoid deadlock
        let add_result = {
            let server_guard = self.rtsp_server.lock().unwrap();
            if let Some(server) = server_guard.as_ref() {
                server.add_stream(&rtsp_path, &launch_str)
            } else {
                Err(AppError::Rtsp("RTSP server not available".to_string()))
            }
        };

        match add_result {
            Ok(()) => {
                // Update state to Running
                let mut pipelines = self.pipelines.lock().unwrap();
                if let Some(h) = pipelines.get_mut(&source.id) {
                    h.status = PipelineState::Running;
                }
                log::info!("Pipeline started for source: {}", source.id);
                Ok(())
            }
            Err(e) => {
                log::error!("Failed to add RTSP stream for {}: {}", source.id, e);
                // Update state to Error
                let mut pipelines = self.pipelines.lock().unwrap();
                if let Some(h) = pipelines.get_mut(&source.id) {
                    h.status = PipelineState::Error(format!("RTSP 流创建失败: {}", e));
                }
                Err(e)
            }
        }
    }

    fn stop_pipeline(&self, source_id: &str) -> AppResult<()> {
        let rtsp_path = {
            let mut pipelines = self.pipelines.lock().unwrap();
            match pipelines.remove(source_id) {
                Some(handle) => {
                    // Stop preview pipeline if exists
                    if let Some(pp) = handle.preview_pipeline.as_ref() {
                        let _ = pp.stop();
                    }
                    handle.rtsp_path
                },
                None => return Ok(()),
            }
        };
        // Remove MJPEG source (synchronous wrapper around async)
        {
            let mjpeg_guard = self.mjpeg_server.blocking_lock();
            if let Some(server) = mjpeg_guard.as_ref() {
                server.remove_source_sync(source_id);
            }
        }
        // Lock pipelines released before acquiring rtsp_server lock
        let server_guard = self.rtsp_server.lock().unwrap();
        if let Some(server) = server_guard.as_ref() {
            server.remove_stream(&rtsp_path);
        }
        log::info!("Pipeline stopped for source: {}", source_id);
        Ok(())
    }

    fn stop_all(&self) -> AppResult<()> {
        let ids: Vec<String> = self.pipelines.lock().unwrap().keys().cloned().collect();
        for id in ids {
            self.stop_pipeline(&id)?;
        }
        Ok(())
    }

    fn get_status(&self, source_id: &str) -> Option<PipelineStatus> {
        let (handle_data, rtsp_path) = {
            let pipelines = self.pipelines.lock().unwrap();
            let handle = pipelines.get(source_id)?;
            (
                (handle.status.clone(), handle.encoder_used.clone(), handle.is_gpu, handle.config.bitrate_kbps,
                 handle.frame_count.clone(), handle.started_at),
                handle.rtsp_path.clone(),
            )
        };

        let rtsp_url = self.get_rtsp_url_by_path(&rtsp_path).unwrap_or_default();

        // Calculate FPS from frame counter
        let elapsed = handle_data.5.elapsed().as_secs_f64();
        let fps = if elapsed > 0.5 {
            handle_data.4.load(Ordering::Relaxed) as f64 / elapsed
        } else {
            0.0
        };

        // Estimate latency: GPU ~15ms, CPU ~40ms
        let latency_ms = if handle_data.2 { 15u32 } else { 40u32 };

        Some(PipelineStatus {
            source_id: source_id.to_string(),
            state: handle_data.0,
            encoder_used: handle_data.1,
            is_gpu: handle_data.2,
            fps,
            bitrate_kbps: handle_data.3,
            latency_ms,
            rtsp_url,
        })
    }

    fn get_all_status(&self) -> Vec<PipelineStatus> {
        let ids: Vec<String> = self.pipelines.lock().unwrap().keys().cloned().collect();
        ids.iter().filter_map(|id| self.get_status(id)).collect()
    }

    fn update_config(&self, source_id: &str, config: &EncodeConfig) -> AppResult<()> {
        // Retrieve the source from the existing pipeline handle instead of re-enumerating
        let source = {
            let pipelines = self.pipelines.lock().unwrap();
            pipelines.get(source_id)
                .map(|h| h.source.clone())
                .ok_or(AppError::Capture(format!("Source not found: {}", source_id)))?
        };
        let _ = self.stop_pipeline(source_id);
        self.start_pipeline(&source, config)
    }
}

/// Detect which encoder will be used based on config
fn detect_encoder_info(config: &EncodeConfig) -> (String, bool) {
    match config.mode {
        EncodeMode::Auto | EncodeMode::GpuOnly => {
            let candidates = gst_pipeline::gpu_encoder_candidates(&config.codec);
            for name in &candidates {
                if gstreamer::ElementFactory::find(name).is_some() {
                    return (name.to_string(), true);
                }
            }
            // CPU fallback
            match config.codec {
                Codec::H264 => ("x264enc".to_string(), false),
                Codec::H265 => ("x265enc".to_string(), false),
            }
        }
        EncodeMode::CpuOnly => match config.codec {
            Codec::H264 => ("x264enc".to_string(), false),
            Codec::H265 => ("x265enc".to_string(), false),
        },
    }
}
