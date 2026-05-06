use crate::error::{AppError, AppResult};
use gstreamer_rtsp_server::prelude::*;
use std::sync::mpsc;

/// RTSP server that runs in a dedicated thread with its own GLib main context.
///
/// The key requirement is that the RTSP server, its socket watches, and the
/// MainLoop must all operate on the SAME MainContext. We use
/// `context.with_thread_default(...)` which:
///   1. Acquires the context for the current thread
///   2. Pushes the context as the thread-default
/// Inside the callback, we explicitly pass `Some(&context)` to both
/// `MainLoop::new()` and `server.attach()` to guarantee they use our context.
pub struct RtspServer {
    port: u16,
    add_stream_tx: mpsc::Sender<(String, String)>,
    remove_stream_tx: mpsc::Sender<String>,
}

impl RtspServer {
    pub fn new(port: u16) -> AppResult<Self> {
        let (add_stream_tx, add_stream_rx) = mpsc::channel::<(String, String)>();
        let (remove_stream_tx, remove_stream_rx) = mpsc::channel::<String>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<u16, String>>();

        let port_str = port.to_string();

        std::thread::Builder::new()
            .name("rtsp-server".into())
            .spawn(move || {
                use gstreamer_rtsp_server::glib::{ControlFlow, MainContext, MainLoop, Priority};

                let context = MainContext::new();
                let ctx = context.clone();

                let result = context.with_thread_default(move || {
                    let main_loop = MainLoop::new(Some(&ctx), false);

                    let server = gstreamer_rtsp_server::RTSPServer::new();
                    server.set_service(&port_str);

                    let mount_points = match server.mount_points() {
                        Some(mp) => mp,
                        None => {
                            let _ = ready_tx.send(Err("Failed to get mount points".into()));
                            return;
                        }
                    };

                    match server.attach(Some(&ctx)) {
                        Ok(_id) => {}
                        Err(e) => {
                            let _ = ready_tx.send(Err(format!("Failed to attach server: {}", e)));
                            return;
                        }
                    }

                    let bound_port = server.bound_port();

                    // Channel polling via timeout source
                    let mp = mount_points.clone();
                    let timeout_source = gstreamer_rtsp_server::glib::timeout_source_new(
                        std::time::Duration::from_millis(100),
                        None,
                        Priority::DEFAULT,
                        move || {
                            while let Ok((path, launch_str)) = add_stream_rx.try_recv() {
                                let factory = gstreamer_rtsp_server::RTSPMediaFactory::new();
                                factory.set_launch(&launch_str);
                                factory.set_shared(true);
                                factory.set_latency(0);
                                mp.add_factory(&path, factory);
                            }

                            while let Ok(path) = remove_stream_rx.try_recv() {
                                mp.remove_factory(&path);
                            }

                            ControlFlow::Continue
                        },
                    );
                    timeout_source.attach(Some(&ctx));

                    let _ = ready_tx.send(Ok(bound_port as u16));

                    main_loop.run();
                });
                if let Err(e) = result {
                    log::error!("RTSP server: with_thread_default failed: {}", e);
                }
            })
            .expect("Failed to spawn RTSP server thread");

        let bound_port = ready_rx
            .recv()
            .map_err(|_| AppError::Rtsp("RTSP server thread panicked".into()))?
            .map_err(|e| AppError::Rtsp(e))?;

        log::info!("RTSP server started on port {}", bound_port);

        Ok(Self {
            port: bound_port,
            add_stream_tx,
            remove_stream_tx,
        })
    }

    pub fn add_stream(&self, path: &str, pipeline_launch_str: &str) -> AppResult<()> {
        self.add_stream_tx
            .send((path.to_string(), pipeline_launch_str.to_string()))
            .map_err(|e| AppError::Rtsp(format!("Failed to send add_stream request: {}", e)))?;
        std::thread::sleep(std::time::Duration::from_millis(100));
        Ok(())
    }

    pub fn remove_stream(&self, path: &str) {
        let _ = self.remove_stream_tx.send(path.to_string());
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    pub fn start(&self) -> AppResult<()> {
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
