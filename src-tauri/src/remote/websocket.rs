use crate::error::{AppError, AppResult};
use crate::remote::injector::RemoteInjector;
use crate::remote::RemoteCommand;
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;

pub struct RemoteControlServer {
    pub port: u16,
    pub password: String,
    pub running: Arc<AtomicBool>,
    pub client_count: Arc<AtomicU32>,
    injector: Arc<RemoteInjector>,
    shutdown_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

impl RemoteControlServer {
    pub fn new(port: u16, password: String, injector: Arc<RemoteInjector>) -> Self {
        Self {
            port,
            password,
            running: Arc::new(AtomicBool::new(false)),
            client_count: Arc::new(AtomicU32::new(0)),
            injector,
            shutdown_tx: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn start(&self) -> AppResult<()> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| AppError::Remote(format!("Failed to bind WebSocket: {}", e)))?;

        log::info!("Remote control WebSocket server listening on {}", addr);
        self.running.store(true, Ordering::SeqCst);

        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        let password = self.password.clone();
        let injector = self.injector.clone();
        let running = self.running.clone();
        let client_count = self.client_count.clone();

        tokio::spawn(async move {
            loop {
                let accept_result = tokio::select! {
                    result = listener.accept() => result,
                    _ = &mut shutdown_rx => break,
                };

                match accept_result {
                    Ok((stream, addr)) => {
                        let ws_stream = match tokio_tungstenite::accept_async(stream).await {
                            Ok(ws) => ws,
                            Err(e) => {
                                log::warn!("WebSocket handshake failed from {}: {}", addr, e);
                                continue;
                            }
                        };

                        client_count.fetch_add(1, Ordering::SeqCst);

                        let password = password.clone();
                        let injector = injector.clone();
                        let client_count = client_count.clone();

                        tokio::spawn(async move {
                            handle_client(ws_stream, addr, &password, &injector).await;
                            client_count.fetch_sub(1, Ordering::SeqCst);
                        });
                    }
                    Err(e) => {
                        log::error!("Failed to accept WebSocket connection: {}", e);
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
        log::info!("Remote control WebSocket server stopped");
        Ok(())
    }
}

async fn handle_client(
    ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    addr: SocketAddr,
    password: &str,
    injector: &Arc<RemoteInjector>,
) {
    let (mut ws_tx, mut ws_rx) = ws_stream.split();
    let mut authenticated = password.is_empty(); // No password = auto-auth

    log::info!("WebSocket client connected from {}", addr);

    while let Some(msg_result) = ws_rx.next().await {
        match msg_result {
            Ok(Message::Text(text)) => {
                if !authenticated {
                    // Expect auth message: {"type":"auth","password":"xxx"}
                    if let Ok(auth_msg) = serde_json::from_str::<serde_json::Value>(&text) {
                        if auth_msg.get("type").and_then(|v| v.as_str()) == Some("auth") {
                            let provided_password = auth_msg
                                .get("password")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            if provided_password == password {
                                authenticated = true;
                                let _ = ws_tx.send(Message::text(r#"{"status":"ok","type":"auth"}"#)).await;
                                log::info!("WebSocket client authenticated: {}", addr);
                                continue;
                            }
                        }
                    }
                    let _ = ws_tx
                        .send(Message::text(r#"{"status":"error","message":"Authentication failed"}"#))
                        .await;
                    break;
                }

                // Parse and execute remote command
                match serde_json::from_str::<RemoteCommand>(&text) {
                    Ok(cmd) => {
                        let result = injector.execute(&cmd);
                        let response = match result {
                            Ok(()) => r#"{"status":"ok"}"#.to_string(),
                            Err(e) => format!(r#"{{"status":"error","message":"{}"}}"#, e),
                        };
                        let _ = ws_tx.send(Message::text(response)).await;
                    }
                    Err(e) => {
                        let response = format!(r#"{{"status":"error","message":"Invalid command: {}"}}"#, e);
                        let _ = ws_tx.send(Message::text(response)).await;
                    }
                }
            }
            Ok(Message::Close(_)) => break,
            Ok(Message::Ping(data)) => {
                let _ = ws_tx.send(Message::Pong(data)).await;
            }
            Err(e) => {
                log::warn!("WebSocket error from {}: {}", addr, e);
                break;
            }
            _ => {}
        }
    }

    log::info!("WebSocket client disconnected: {}", addr);
}
