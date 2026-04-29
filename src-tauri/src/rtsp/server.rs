use crate::error::{AppError, AppResult};
use gstreamer_rtsp_server::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

/// Ensure a GLib main loop is running for gst-rtsp-server.
/// This must be called once during app startup.
static GLIB_LOOP_STARTED: AtomicBool = AtomicBool::new(false);

pub fn ensure_glib_main_loop() {
    if GLIB_LOOP_STARTED.swap(true, Ordering::SeqCst) {
        return; // Already started
    }

    std::thread::spawn(|| {
        let context = glib::MainContext::default();
        let _ = context.acquire();

        let loop_ = glib::MainLoop::new(Some(&context), false);
        log::info!("GLib main loop thread started");

        loop_.run();

        log::info!("GLib main loop thread exiting");
    });

    // Give the loop thread a moment to start and acquire the context
    std::thread::sleep(std::time::Duration::from_millis(100));
}

pub struct RtspServer {
    server: gstreamer_rtsp_server::RTSPServer,
    mount_points: gstreamer_rtsp_server::RTSPMountPoints,
    port: u16,
}

impl RtspServer {
    pub fn new(port: u16) -> AppResult<Self> {
        // Ensure GLib main loop is running before creating RTSP server
        ensure_glib_main_loop();

        let server = gstreamer_rtsp_server::RTSPServer::new();

        let mount_points = server
            .mount_points()
            .ok_or(AppError::Rtsp("Failed to get mount points".into()))?;

        // Set the service port
        server.set_service(&port.to_string());

        Ok(Self {
            server,
            mount_points,
            port,
        })
    }

    pub fn add_stream(&self, path: &str, pipeline_launch_str: &str) -> AppResult<()> {
        let factory = gstreamer_rtsp_server::RTSPMediaFactory::new();
        factory.set_launch(pipeline_launch_str);
        // Shared mode: each client gets the same stream
        factory.set_shared(true);

        self.mount_points.add_factory(path, factory);
        log::info!("Added RTSP stream at path: {}", path);
        Ok(())
    }

    pub fn remove_stream(&self, path: &str) {
        self.mount_points.remove_factory(path);
        log::info!("Removed RTSP stream at path: {}", path);
    }

    pub fn start(&self) -> AppResult<()> {
        // Attach the server to the default GLib main context
        // The GLib main loop must be running (see ensure_glib_main_loop)
        let _ = self.server.attach(None);
        log::info!("RTSP server started on port {}", self.port);
        Ok(())
    }

    pub fn rtsp_url(&self, path: &str) -> String {
        format!(
            "rtsp://127.0.0.1:{}/{}",
            self.port,
            path.trim_start_matches('/')
        )
    }

    #[allow(dead_code)]
    pub fn port(&self) -> u16 {
        self.port
    }
}
