use crate::error::{AppError, AppResult};
use crate::pipeline::mjpeg_server::MjpegServer;
use gstreamer::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex as AsyncMutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RtspClientState {
    Connecting,
    Connected,
    Reconnecting { attempt: u32, max_attempts: u32 },
    Disconnected,
    Error(String),
    Offline,
}

#[derive(Debug, Clone, Serialize)]
pub struct RtspClientStatus {
    pub stream_id: String,
    pub name: String,
    pub url: String,
    pub state: RtspClientState,
    pub resolution: Option<(u32, u32)>,
    pub fps: f64,
    pub latency_ms: u32,
    pub protocol: String,
}

struct RtspClientHandle {
    pipeline: gstreamer::Pipeline,
    state: Arc<Mutex<RtspClientState>>,
    name: String,
    url: String,
    protocol: String,
    resolution: Arc<Mutex<Option<(u32, u32)>>>,
    frame_count: Arc<AtomicU64>,
    started_at: Instant,
    #[allow(dead_code)]
    bus_watch_id: gstreamer::bus::BusWatchGuard,
}

pub struct RtspClientManager {
    clients: Mutex<HashMap<String, RtspClientHandle>>,
    mjpeg_server: Arc<AsyncMutex<Option<MjpegServer>>>,
    preview_http_port: u16,
}

impl RtspClientManager {
    pub fn new(mjpeg_server: Arc<AsyncMutex<Option<MjpegServer>>>, preview_http_port: u16) -> Self {
        Self {
            clients: Mutex::new(HashMap::new()),
            mjpeg_server,
            preview_http_port,
        }
    }

    /// Create an RTSP client pipeline (sync-only, no async operations).
    /// Returns (stream_id, mpsc::Receiver for JPEG frames).
    /// The caller must call register_mjpeg() in an async context afterwards.
    pub fn create_pipeline(
        &self,
        name: &str,
        url: &str,
        protocol: &str,
        username: Option<&str>,
        password: Option<&str>,
    ) -> AppResult<(String, mpsc::Receiver<Vec<u8>>)> {
        // Generate stream_id: rtsp-client-{timestamp_short}
        let timestamp_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let id_short = format!("{:08x}", (timestamp_nanos & 0xFFFFFFFF));
        let stream_id = format!("rtsp-client-{}", id_short);

        log::info!("RTSP client create_pipeline: stream_id={}, url={}, protocol={}", stream_id, url, protocol);

        // Build pipeline using parse_launch (C API, avoids Rust .build() panics).
        // We use two separate bin groups separated by a space (not !) so that
        // rtspsrc and decodebin are NOT auto-linked — they need dynamic pad-added linking.
        let protocols_val = if protocol.to_lowercase() == "tcp" { "4" } else { "1" };

        // rtspsrc name=src ...  decodebin name=decoder ! videoconvert name=vconv ! jpegenc name=jenc quality=60 ! appsink name=sink ...
        // Note: space between rtspsrc and decodebin means they are NOT linked.
        let launch_str = format!(
            "rtspsrc name=src protocols={} latency=0  decodebin name=decoder ! videoconvert name=vconv ! jpegenc name=jenc quality=60 ! appsink name=sink emit-signals=true max-buffers=1 drop=true",
            protocols_val
        );

        log::info!("RTSP client pipeline launch string: {}", launch_str);

        let pipeline = gstreamer::parse::launch(&launch_str)
            .map_err(|e| AppError::RtspClient(format!("Failed to parse launch: {}", e)))?;
        log::info!("RTSP client: parse_launch succeeded");

        let pipeline = pipeline
            .downcast::<gstreamer::Pipeline>()
            .map_err(|_| AppError::RtspClient("Pipeline downcast failed".to_string()))?;
        log::info!("RTSP client: pipeline downcast succeeded");

        // Get elements from pipeline by name
        let rtspsrc = pipeline.by_name("src")
            .ok_or_else(|| AppError::RtspClient("rtspsrc element not found".to_string()))?;
        let decodebin = pipeline.by_name("decoder")
            .ok_or_else(|| AppError::RtspClient("decodebin element not found".to_string()))?;
        let videoconvert_el = pipeline.by_name("vconv")
            .ok_or_else(|| AppError::RtspClient("videoconvert element not found".to_string()))?;
        let appsink_el = pipeline.by_name("sink")
            .ok_or_else(|| AppError::RtspClient("appsink element not found".to_string()))?;
        log::info!("RTSP client: all elements found in pipeline");

        // Set rtspsrc properties (location cannot be in launch string for URLs with special chars)
        rtspsrc.set_property("location", url);
        if let Some(user) = username {
            rtspsrc.set_property("user-id", user);
        }
        if let Some(pass) = password {
            rtspsrc.set_property("user-pw", pass);
        }

        // Shared state for this client
        let state = Arc::new(Mutex::new(RtspClientState::Connecting));
        let resolution = Arc::new(Mutex::new(None));
        let frame_count = Arc::new(AtomicU64::new(0));

        // Handle pad-added signal on rtspsrc -> link to decodebin
        let decodebin_clone = decodebin.clone();
        #[allow(unused_variables)]
        let rtspsrc_sig_id = rtspsrc.connect("pad-added", false, move |args| {
            let pad = args[1].get::<gstreamer::Pad>().ok()?;
            let sink_pad = decodebin_clone.static_pad("sink")?;
            if !sink_pad.is_linked() {
                if let Err(e) = pad.link(&sink_pad) {
                    log::error!("Failed to link rtspsrc pad to decodebin: {:?}", e);
                }
            }
            None
        });

        // Handle pad-added signal on decodebin -> link to videoconvert
        let videoconvert_clone = videoconvert_el.clone();
        let resolution_clone = resolution.clone();
        #[allow(unused_variables)]
        let decodebin_sig_id = decodebin.connect("pad-added", false, move |args| {
            let pad = args[1].get::<gstreamer::Pad>().ok()?;
            
            // Extract resolution from caps
            if let Some(caps) = pad.current_caps() {
                if let Some(s) = caps.structure(0) {
                    if let Ok(w) = s.get::<i32>("width") {
                        if let Ok(h) = s.get::<i32>("height") {
                            *resolution_clone.lock().unwrap() = Some((w as u32, h as u32));
                            log::info!("RTSP client detected resolution: {}x{}", w, h);
                        }
                    }
                }
            }
            
            let sink_pad = videoconvert_clone.static_pad("sink")?;
            if !sink_pad.is_linked() {
                if let Err(e) = pad.link(&sink_pad) {
                    log::error!("Failed to link decodebin pad to videoconvert: {:?}", e);
                }
            }
            None
        });

        // Setup appsink callbacks
        let appsink = appsink_el
            .downcast::<gstreamer_app::AppSink>()
            .map_err(|_| AppError::RtspClient("Failed to downcast to AppSink".to_string()))?;
        log::info!("RTSP client: appsink downcast succeeded");

        let (tx, rx) = mpsc::sync_channel(2);
        let tx_clone = Arc::new(tx);
        let frame_count_clone = frame_count.clone();
        let state_for_sample = state.clone();

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
                    let count = frame_count_clone.fetch_add(1, Ordering::Relaxed);
                    // Update state to Connected on first frame
                    if count == 0 {
                        log::info!("RTSP client: first frame received, updating state to Connected");
                    }
                    if let Ok(mut st) = state_for_sample.lock() {
                        if !matches!(*st, RtspClientState::Connected) {
                            *st = RtspClientState::Connected;
                        }
                    }
                    Ok(gstreamer::FlowSuccess::Ok)
                })
                .build(),
        );

        // Setup bus watch
        let bus = pipeline
            .bus()
            .ok_or_else(|| AppError::RtspClient("Pipeline has no bus".to_string()))?;

        let stream_id_log = stream_id.clone();
        let state_for_bus = state.clone();
        let bus_watch_id = bus
            .add_watch(move |_bus, msg| {
                match msg.view() {
                    gstreamer::MessageView::Error(err) => {
                        log::error!(
                            "RTSP client pipeline error for {}: {} ({})",
                            stream_id_log,
                            err.error(),
                            err.debug().unwrap_or_default()
                        );
                        if let Ok(mut st) = state_for_bus.lock() {
                            *st = RtspClientState::Error(err.error().to_string());
                        }
                    }
                    gstreamer::MessageView::StateChanged(s) => {
                        log::info!(
                            "RTSP client pipeline for {} state: {:?} -> {:?}",
                            stream_id_log,
                            s.old(),
                            s.current()
                        );
                        if s.current() == gstreamer::State::Playing {
                            if let Ok(mut st) = state_for_bus.lock() {
                                *st = RtspClientState::Connected;
                            }
                        }
                    }
                    gstreamer::MessageView::Eos(_) => {
                        log::warn!("RTSP client pipeline EOS for {}", stream_id_log);
                    }
                    gstreamer::MessageView::Warning(w) => {
                        log::warn!("RTSP client pipeline warning for {}: {}", stream_id_log, w.error());
                    }
                    _ => {}
                }
                gstreamer::glib::ControlFlow::Continue
            })
            .map_err(|e| AppError::RtspClient(format!("Failed to add bus watch: {}", e)))?;

        // Set pipeline to Playing
        pipeline
            .set_state(gstreamer::State::Playing)
            .map_err(|e| AppError::RtspClient(format!("Failed to set pipeline to Playing: {}", e)))?;

        log::info!("RTSP client pipeline set to Playing: stream_id={}", stream_id);

        // Store handle
        let handle = RtspClientHandle {
            pipeline,
            state,
            name: name.to_string(),
            url: url.to_string(),
            protocol: protocol.to_string(),
            resolution,
            frame_count,
            started_at: Instant::now(),
            bus_watch_id,
        };

        self.clients.lock().unwrap().insert(stream_id.clone(), handle);

        log::info!("RTSP client pipeline created: stream_id={}", stream_id);

        Ok((stream_id, rx))
    }

    /// Register an RTSP client's frame output with the MJPEG server.
    /// Must be called from an async context (not spawn_blocking).
    pub async fn register_mjpeg(&self, stream_id: &str, rx: mpsc::Receiver<Vec<u8>>) -> AppResult<()> {
        let mjpeg_server_arc = self.mjpeg_server.clone();
        let preview_port = self.preview_http_port;

        // Ensure MJPEG server is running
        {
            let mut guard = mjpeg_server_arc.lock().await;
            if guard.is_none() {
                let server = MjpegServer::new(preview_port);
                server.start().await?;
                *guard = Some(server);
            }
        }

        // Create broadcast channel and register with MJPEG server
        let (btx, _) = tokio::sync::broadcast::channel::<Vec<u8>>(2);
        let mjpeg_source_id = stream_id.to_string();
        {
            let guard = mjpeg_server_arc.lock().await;
            if let Some(server) = guard.as_ref() {
                server.add_source(mjpeg_source_id, btx.clone()).await;
            }
        }

        // Forward frames: mpsc -> broadcast in a dedicated thread
        let forward_mjpeg = self.mjpeg_server.clone();
        let forward_source_id = stream_id.to_string();
        let rt_handle = tokio::runtime::Handle::current();
        std::thread::spawn(move || {
            loop {
                match rx.recv() {
                    Ok(frame) => {
                        let _ = btx.send(frame);
                    }
                    Err(_) => break,
                }
            }
            // Cleanup: remove from MJPEG server
            let mjpeg = forward_mjpeg;
            let sid = forward_source_id;
            let _ = rt_handle.spawn(async move {
                let guard = mjpeg.lock().await;
                if let Some(server) = guard.as_ref() {
                    server.remove_source(&sid).await;
                }
            });
        });

        Ok(())
    }

    /// Disconnect from an RTSP source.
    pub fn disconnect(&self, stream_id: &str) -> AppResult<()> {
        let mut clients = self.clients.lock().unwrap();
        if let Some(handle) = clients.remove(stream_id) {
            // Pipeline will be set to Null state when dropped
            drop(handle);
            log::info!("RTSP client disconnected: {}", stream_id);
        }
        // Remove from MJPEG server — use spawn_blocking's tokio handle if available
        let mjpeg_server = self.mjpeg_server.clone();
        let sid = stream_id.to_string();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let _ = handle.spawn(async move {
                let guard = mjpeg_server.lock().await;
                if let Some(server) = guard.as_ref() {
                    server.remove_source(&sid).await;
                }
            });
        } else {
            log::warn!("No tokio runtime available for MJPEG cleanup of {}", sid);
        }
        Ok(())
    }

    /// Get status of a specific RTSP client.
    pub fn get_status(&self, stream_id: &str) -> Option<RtspClientStatus> {
        let clients = self.clients.lock().unwrap();
        let handle = clients.get(stream_id)?;

        let state = handle.state.lock().unwrap().clone();
        let resolution = *handle.resolution.lock().unwrap();

        // Calculate FPS from frame counter
        let elapsed = handle.started_at.elapsed().as_secs_f64();
        let fps = if elapsed > 0.5 {
            handle.frame_count.load(Ordering::Relaxed) as f64 / elapsed
        } else {
            0.0
        };

        // Estimate latency based on protocol
        let latency_ms = if handle.protocol.to_lowercase() == "tcp" {
            20u32
        } else {
            10u32
        };

        Some(RtspClientStatus {
            stream_id: stream_id.to_string(),
            name: handle.name.clone(),
            url: handle.url.clone(),
            state,
            resolution,
            fps,
            latency_ms,
            protocol: handle.protocol.clone(),
        })
    }

    /// Get status of all RTSP clients.
    pub fn get_all_status(&self) -> Vec<RtspClientStatus> {
        let clients = self.clients.lock().unwrap();
        let stream_ids: Vec<String> = clients.keys().cloned().collect();
        drop(clients);

        stream_ids
            .iter()
            .filter_map(|id| self.get_status(id))
            .collect()
    }

    /// Disconnect all RTSP clients.
    pub fn disconnect_all(&self) -> AppResult<()> {
        let stream_ids: Vec<String> = {
            let clients = self.clients.lock().unwrap();
            clients.keys().cloned().collect()
        };
        for id in stream_ids {
            self.disconnect(&id)?;
        }
        Ok(())
    }
}

impl Drop for RtspClientHandle {
    fn drop(&mut self) {
        let _ = self.pipeline.set_state(gstreamer::State::Null);
    }
}
