use crate::error::{AppError, AppResult};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{Mutex, broadcast};
use tokio::net::TcpListener;

pub struct MjpegServer {
    pub port: u16,
    pub running: Arc<AtomicBool>,
    shutdown_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    frame_sources: Arc<Mutex<HashMap<String, broadcast::Sender<Vec<u8>>>>>,
}

impl MjpegServer {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            running: Arc::new(AtomicBool::new(false)),
            shutdown_tx: Arc::new(Mutex::new(None)),
            frame_sources: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn add_source(&self, source_id: String, sender: broadcast::Sender<Vec<u8>>) {
        self.frame_sources.lock().await.insert(source_id, sender);
    }

    pub async fn remove_source(&self, source_id: &str) {
        self.frame_sources.lock().await.remove(source_id);
    }

    /// Remove source from synchronous context (e.g., from stop_pipeline called via spawn_blocking).
    /// Uses `blocking_lock` on the tokio Mutex — safe when called from a non-async thread
    /// (spawn_blocking or std::thread). Must NOT be called from a tokio async context directly.
    pub fn remove_source_sync(&self, source_id: &str) {
        let mut guard = self.frame_sources.blocking_lock();
        guard.remove(source_id);
    }

    pub async fn start(&self) -> AppResult<()> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| AppError::Preview(format!("MJPEG bind failed: {}", e)))?;

        log::info!("MJPEG preview server listening on {}", addr);
        self.running.store(true, Ordering::SeqCst);

        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        let running = self.running.clone();
        let frame_sources = self.frame_sources.clone();

        tokio::spawn(async move {
            loop {
                let accept_result = tokio::select! {
                    result = listener.accept() => result,
                    _ = &mut shutdown_rx => break,
                };

                match accept_result {
                    Ok((stream, _addr)) => {
                        let frame_sources = frame_sources.clone();
                        tokio::spawn(async move {
                            handle_mjpeg_connection(stream, &frame_sources).await;
                        });
                    }
                    Err(e) => {
                        log::error!("MJPEG accept error: {}", e);
                    }
                }

                if !running.load(Ordering::SeqCst) {
                    break;
                }
            }
        });

        Ok(())
    }

    pub async fn stop(&self) -> AppResult<()> {
        self.running.store(false, Ordering::SeqCst);
        if let Some(tx) = self.shutdown_tx.lock().await.take() {
            let _ = tx.send(());
        }
        log::info!("MJPEG preview server stopped");
        Ok(())
    }
}

async fn handle_mjpeg_connection(
    stream: tokio::net::TcpStream,
    frame_sources: &Arc<Mutex<HashMap<String, broadcast::Sender<Vec<u8>>>>>,
) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut buf = vec![0u8; 4096];
    let mut stream = stream;

    let n = match stream.read(&mut buf).await {
        Ok(0) | Err(_) => return,
        Ok(n) => n,
    };

    let request = String::from_utf8_lossy(&buf[..n]);
    let path = request.lines().next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");

    let source_id = path.trim_start_matches('/');

    // Subscribe to the broadcast channel
    let mut client_rx = {
        let sources = frame_sources.lock().await;
        match sources.get(source_id) {
            Some(sender) => sender.subscribe(),
            None => {
                let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
                let _ = stream.write_all(response.as_bytes()).await;
                return;
            }
        }
    };

    let header = "HTTP/1.1 200 OK\r\nContent-Type: multipart/x-mixed-replace; boundary=frame\r\n\r\n";
    if stream.write_all(header.as_bytes()).await.is_err() {
        return;
    }

    loop {
        match client_rx.recv().await {
            Ok(jpeg_data) => {
                let frame_header = format!(
                    "--frame\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
                    jpeg_data.len()
                );
                if stream.write_all(frame_header.as_bytes()).await.is_err() {
                    break;
                }
                if stream.write_all(&jpeg_data).await.is_err() {
                    break;
                }
                if stream.write_all(b"\r\n").await.is_err() {
                    break;
                }
            }
            Err(broadcast::error::RecvError::Lagged(_)) => {
                // Client is too slow, skip frames — this is fine
                continue;
            }
            Err(broadcast::error::RecvError::Closed) => {
                break;
            }
        }
    }
}
