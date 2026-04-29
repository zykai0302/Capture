use crate::capture::source::CaptureSource;
use crate::config::AppConfig;
use crate::encode::config::{Codec, EncodeConfig, EncodeMode};
use crate::error::{AppError, AppResult};
use crate::pipeline::gst_pipeline;
use crate::pipeline::{PipelineManager, PipelineState, PipelineStatus};
use crate::rtsp::RtspServer;
use std::collections::HashMap;
use std::sync::Mutex;

struct PipelineHandle {
    status: PipelineState,
    config: EncodeConfig,
    rtsp_path: String,
    source: CaptureSource,
    encoder_used: String,
    is_gpu: bool,
}

pub struct GstPipelineManager {
    pipelines: Mutex<HashMap<String, PipelineHandle>>,
    rtsp_server: Mutex<Option<RtspServer>>,
    config: AppConfig,
}

impl GstPipelineManager {
    pub fn new(config: AppConfig) -> Self {
        Self {
            pipelines: Mutex::new(HashMap::new()),
            rtsp_server: Mutex::new(None),
            config,
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
}

impl PipelineManager for GstPipelineManager {
    fn start_pipeline(&self, source: &CaptureSource, config: &EncodeConfig) -> AppResult<()> {
        // Ensure RTSP server is running
        self.ensure_rtsp_server()?;

        let rtsp_path = format!("/{}", source.id);

        // Build launch string for gst-rtsp-server
        let launch_str = gst_pipeline::build_launch_string(source, config);

        // Add stream to RTSP server — release pipelines lock first to avoid deadlock
        {
            let server_guard = self.rtsp_server.lock().unwrap();
            if let Some(server) = server_guard.as_ref() {
                match server.add_stream(&rtsp_path, &launch_str) {
                    Ok(()) => {}
                    Err(e) => {
                        log::error!("Failed to add RTSP stream for {}: {}", source.id, e);
                        return Err(e);
                    }
                }
            }
        }

        // Detect which encoder will be used
        let (encoder_used, is_gpu) = detect_encoder_info(config);

        let handle = PipelineHandle {
            status: PipelineState::Running,
            config: config.clone(),
            rtsp_path,
            source: source.clone(),
            encoder_used,
            is_gpu,
        };

        self.pipelines
            .lock()
            .unwrap()
            .insert(source.id.clone(), handle);

        log::info!("Pipeline started for source: {}", source.id);
        Ok(())
    }

    fn stop_pipeline(&self, source_id: &str) -> AppResult<()> {
        let rtsp_path = {
            let mut pipelines = self.pipelines.lock().unwrap();
            match pipelines.remove(source_id) {
                Some(handle) => handle.rtsp_path,
                None => return Ok(()),
            }
        };
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
            // Clone data we need while holding the lock, then release
            (
                (handle.status.clone(), handle.encoder_used.clone(), handle.is_gpu, handle.config.bitrate_kbps),
                handle.rtsp_path.clone(),
            )
        };

        // Now safe to get rtsp_url (which needs its own locks)
        let rtsp_url = self.get_rtsp_url_by_path(&rtsp_path).unwrap_or_default();

        Some(PipelineStatus {
            source_id: source_id.to_string(),
            state: handle_data.0,
            encoder_used: handle_data.1,
            is_gpu: handle_data.2,
            fps: 0.0,
            bitrate_kbps: handle_data.3,
            latency_ms: 0,
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

    fn get_rtsp_url(&self, source_id: &str) -> Option<String> {
        let rtsp_path = {
            let pipelines = self.pipelines.lock().unwrap();
            pipelines.get(source_id).map(|h| h.rtsp_path.clone())?
        };
        self.get_rtsp_url_by_path(&rtsp_path)
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
