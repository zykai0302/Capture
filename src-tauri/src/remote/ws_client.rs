//! WebSocket remote control client
//!
//! This module provides `WsRemoteClient` that connects to a remote WebSocket
//! control server, authenticates, and sends `RemoteCommand` messages.

use crate::error::{AppError, AppResult};
use crate::remote::{
    ClickAction, KeyComboData, KeyPressData, MouseClickData, MouseDragData, MouseMoveData,
    MouseScrollData, MouseButton, RemoteCommand, WsClientStatus,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;

/// Command from frontend with relative coordinates (0.0~1.0)
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientRemoteCommand {
    MouseMove {
        stream_id: String,
        data: RelativeMouseMoveData,
    },
    MouseClick {
        stream_id: String,
        data: RelativeMouseClickData,
    },
    MouseScroll {
        stream_id: String,
        data: RelativeMouseScrollData,
    },
    MouseDrag {
        stream_id: String,
        data: RelativeMouseDragData,
    },
    KeyPress {
        stream_id: String,
        data: KeyPressData,
    },
    KeyCombo {
        stream_id: String,
        data: KeyComboData,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct RelativeMouseMoveData {
    pub rel_x: f64,
    pub rel_y: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RelativeMouseClickData {
    pub rel_x: f64,
    pub rel_y: f64,
    pub button: MouseButton,
    pub action: ClickAction,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RelativeMouseScrollData {
    pub rel_x: f64,
    pub rel_y: f64,
    pub dx: i32,
    pub dy: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RelativeMouseDragData {
    pub from_rel_x: f64,
    pub from_rel_y: f64,
    pub to_rel_x: f64,
    pub to_rel_y: f64,
    pub button: MouseButton,
}

/// Type alias for the WebSocket sink
type WsSink = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    Message,
>;



/// WebSocket remote control client
///
/// Connects to a remote WebSocket control server, authenticates,
/// and sends `RemoteCommand` messages.
pub struct WsRemoteClient {
    /// WebSocket server URL (ws:// or wss://)
    url: String,
    /// Authentication password
    password: String,
    /// Current connection status
    status: Arc<Mutex<WsClientStatus>>,
    /// WebSocket sink for sending messages
    sender: Arc<Mutex<Option<WsSink>>>,
    /// Shutdown signal sender
    shutdown_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    /// Remote screen resolution for coordinate mapping
    remote_resolution: Arc<Mutex<Option<(u32, u32)>>>,
}

impl WsRemoteClient {
    /// Create a new WebSocket remote client
    ///
    /// The client starts in disconnected state. Call `connect()` to establish
    /// a connection to the remote server.
    pub fn new(url: String, password: String) -> Self {
        let status = WsClientStatus {
            is_connected: false,
            is_reconnecting: false,
            reconnect_attempt: 0,
            max_reconnect_attempts: 5,
            remote_url: String::new(),
        };

        Self {
            url,
            password,
            status: Arc::new(Mutex::new(status)),
            sender: Arc::new(Mutex::new(None)),
            shutdown_tx: Arc::new(Mutex::new(None)),
            remote_resolution: Arc::new(Mutex::new(None)),
        }
    }

    /// Connect to the remote WebSocket server
    ///
    /// This method:
    /// 1. Establishes a WebSocket connection
    /// 2. Sends authentication message
    /// 3. Waits for authentication response
    /// 4. Spawns a background task to monitor the connection
    pub async fn connect(&self) -> AppResult<()> {
        // Check if already connected
        {
            let status = self.status.lock().await;
            if status.is_connected {
                return Err(AppError::Remote("Already connected".to_string()));
            }
        }

        log::info!("Connecting to WebSocket server: {}", self.url);

        // Establish WebSocket connection
        let (ws_stream, _response) = tokio_tungstenite::connect_async(&self.url)
            .await
            .map_err(|e| AppError::Remote(format!("WebSocket connect failed: {}", e)))?;

        log::debug!("WebSocket connection established, authenticating...");

        // Split into sink and stream
        let (mut sink, mut stream) = ws_stream.split();

        // Send auth message
        let auth_msg = serde_json::json!({"type": "auth", "password": self.password}).to_string();
        sink.send(Message::Text(auth_msg.into()))
            .await
            .map_err(|e| AppError::Remote(format!("Auth send failed: {}", e)))?;

        // Wait for auth response
        match stream.next().await {
            Some(Ok(Message::Text(text))) => {
                let resp: serde_json::Value = serde_json::from_str(&text)
                    .map_err(|e| AppError::Remote(format!("Invalid auth response: {}", e)))?;
                if resp.get("status").and_then(|v| v.as_str()) != Some("ok") {
                    let error_msg = resp
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Authentication failed");
                    return Err(AppError::Remote(error_msg.to_string()));
                }
                log::info!("Authentication successful");
            }
            Some(Ok(Message::Close(_))) => {
                return Err(AppError::Remote("Connection closed during auth".to_string()));
            }
            Some(Ok(other)) => {
                log::warn!("Unexpected message during auth: {:?}", other);
                return Err(AppError::Remote(format!(
                    "Unexpected message during auth: {:?}",
                    other
                )));
            }
            Some(Err(e)) => {
                return Err(AppError::Remote(format!("Auth receive error: {}", e)));
            }
            None => {
                return Err(AppError::Remote("No auth response received".to_string()));
            }
        }

        // Store sink
        *self.sender.lock().await = Some(sink);

        // Create shutdown channel for the receive task
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        // Clone status for the background task
        let status = self.status.clone();
        let sender = self.sender.clone();

        // Spawn receive task for monitoring disconnection
        tokio::spawn(async move {
            let mut shutdown_rx = shutdown_rx;
            loop {
                tokio::select! {
                    msg = stream.next() => {
                        match msg {
                            Some(Ok(Message::Close(_))) | None => {
                                log::warn!("WebSocket connection closed");
                                let mut s = status.lock().await;
                                s.is_connected = false;
                                s.is_reconnecting = false;
                                *sender.lock().await = None;
                                break;
                            }
                            Some(Ok(Message::Ping(_data))) => {
                                // Pong is auto-handled by tungstenite
                            }
                            Some(Ok(Message::Pong(_))) => {
                                // Ignore pong messages
                            }
                            Some(Ok(Message::Text(text))) => {
                                // Handle incoming messages from server
                                log::debug!("Received message from server: {}", text);
                                // Future: parse and handle server messages
                            }
                            Some(Ok(Message::Binary(data))) => {
                                log::debug!("Received binary message: {} bytes", data.len());
                            }
                            Some(Ok(Message::Frame(_))) => {
                                // Raw frame, ignore
                            }
                            Some(Err(e)) => {
                                log::error!("WebSocket receive error: {}", e);
                                let mut s = status.lock().await;
                                s.is_connected = false;
                                *sender.lock().await = None;
                                break;
                            }
                        }
                    }
                    _ = &mut shutdown_rx => {
                        log::debug!("WebSocket receive task shutting down");
                        break;
                    }
                }
            }
        });

        // Update status
        {
            let mut s = self.status.lock().await;
            s.is_connected = true;
            s.is_reconnecting = false;
            s.reconnect_attempt = 0;
            s.remote_url = self.url.clone();
        }

        log::info!("WebSocket remote client connected to {}", self.url);
        Ok(())
    }

    /// Disconnect from the remote WebSocket server
    pub async fn disconnect(&self) -> AppResult<()> {
        log::info!("Disconnecting from WebSocket server...");

        // Signal shutdown to the receive task
        if let Some(tx) = self.shutdown_tx.lock().await.take() {
            let _ = tx.send(());
        }

        // Close the WebSocket connection
        if let Some(mut sink) = self.sender.lock().await.take() {
            if let Err(e) = sink.send(Message::Close(None)).await {
                log::warn!("Error sending close message: {}", e);
            }
        }

        // Update status
        {
            let mut s = self.status.lock().await;
            s.is_connected = false;
            s.is_reconnecting = false;
        }

        log::info!("WebSocket disconnected");
        Ok(())
    }

    /// Send a command to the remote server
    ///
    /// Converts `ClientRemoteCommand` with relative coordinates to `RemoteCommand`
    /// with absolute coordinates using the remote resolution.
    pub async fn send_command(&self, command: ClientRemoteCommand) -> AppResult<()> {
        // Check connection status
        {
            let status = self.status.lock().await;
            if !status.is_connected {
                return Err(AppError::Remote("Not connected".to_string()));
            }
        }

        // Get remote resolution for coordinate conversion
        let resolution = self.remote_resolution.lock().await;
        let (w, h) = resolution
            .ok_or_else(|| AppError::Remote("Remote resolution not available".to_string()))?;

        // Convert ClientRemoteCommand to RemoteCommand
        let remote_cmd = match command {
            ClientRemoteCommand::MouseMove { stream_id, data } => RemoteCommand::MouseMove {
                stream_id,
                data: MouseMoveData {
                    x: (data.rel_x * w as f64).round() as i32,
                    y: (data.rel_y * h as f64).round() as i32,
                },
            },
            ClientRemoteCommand::MouseClick { stream_id, data } => RemoteCommand::MouseClick {
                stream_id,
                data: MouseClickData {
                    x: (data.rel_x * w as f64).round() as i32,
                    y: (data.rel_y * h as f64).round() as i32,
                    button: data.button,
                    action: data.action,
                },
            },
            ClientRemoteCommand::MouseScroll { stream_id, data } => RemoteCommand::MouseScroll {
                stream_id,
                data: MouseScrollData {
                    x: (data.rel_x * w as f64).round() as i32,
                    y: (data.rel_y * h as f64).round() as i32,
                    dx: data.dx,
                    dy: data.dy,
                },
            },
            ClientRemoteCommand::MouseDrag { stream_id, data } => RemoteCommand::MouseDrag {
                stream_id,
                data: MouseDragData {
                    from_x: (data.from_rel_x * w as f64).round() as i32,
                    from_y: (data.from_rel_y * h as f64).round() as i32,
                    to_x: (data.to_rel_x * w as f64).round() as i32,
                    to_y: (data.to_rel_y * h as f64).round() as i32,
                    button: data.button,
                },
            },
            ClientRemoteCommand::KeyPress { stream_id, data } => {
                RemoteCommand::KeyPress { stream_id, data }
            }
            ClientRemoteCommand::KeyCombo { stream_id, data } => {
                RemoteCommand::KeyCombo { stream_id, data }
            }
        };

        // Drop resolution lock before sending
        drop(resolution);

        // Serialize to JSON
        let json = serde_json::to_string(&remote_cmd)
            .map_err(|e| AppError::Remote(format!("Command serialization failed: {}", e)))?;

        // Send via WebSocket
        let mut sender = self.sender.lock().await;
        if let Some(sink) = sender.as_mut() {
            sink.send(Message::Text(json.clone().into()))
                .await
                .map_err(|e| AppError::Remote(format!("Send failed: {}", e)))?;
            log::trace!("Sent command: {}", json);
        } else {
            return Err(AppError::Remote("WebSocket sink not available".to_string()));
        }

        Ok(())
    }

    /// Get the current connection status
    pub async fn get_status(&self) -> WsClientStatus {
        self.status.lock().await.clone()
    }

    /// Set the remote screen resolution for coordinate mapping
    ///
    /// This must be called before sending mouse commands to enable
    /// proper coordinate conversion from relative (0.0~1.0) to absolute.
    pub async fn set_remote_resolution(&self, width: u32, height: u32) {
        *self.remote_resolution.lock().await = Some((width, height));
        log::debug!("Remote resolution set to {}x{}", width, height);
    }

    /// Check if currently connected
    pub async fn is_connected(&self) -> bool {
        self.status.lock().await.is_connected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_remote_command_deserialization() {
        let json = r#"{"type":"mouse_move","stream_id":"screen-0","data":{"rel_x":0.5,"rel_y":0.5}}"#;
        let cmd: ClientRemoteCommand = serde_json::from_str(json).unwrap();
        match cmd {
            ClientRemoteCommand::MouseMove { stream_id, data } => {
                assert_eq!(stream_id, "screen-0");
                assert!((data.rel_x - 0.5).abs() < f64::EPSILON);
                assert!((data.rel_y - 0.5).abs() < f64::EPSILON);
            }
            _ => panic!("Expected MouseMove"),
        }
    }

    #[test]
    fn test_relative_mouse_click_deserialization() {
        let json = r#"{"type":"mouse_click","stream_id":"screen-1","data":{"rel_x":0.25,"rel_y":0.75,"button":"Right","action":"Double"}}"#;
        let cmd: ClientRemoteCommand = serde_json::from_str(json).unwrap();
        match cmd {
            ClientRemoteCommand::MouseClick { stream_id, data } => {
                assert_eq!(stream_id, "screen-1");
                assert!((data.rel_x - 0.25).abs() < f64::EPSILON);
                assert!((data.rel_y - 0.75).abs() < f64::EPSILON);
                assert_eq!(data.button, MouseButton::Right);
                assert_eq!(data.action, ClickAction::Double);
            }
            _ => panic!("Expected MouseClick"),
        }
    }

    #[test]
    fn test_relative_mouse_scroll_deserialization() {
        let json = r#"{"type":"mouse_scroll","stream_id":"screen-0","data":{"rel_x":0.5,"rel_y":0.5,"dx":0,"dy":-10}}"#;
        let cmd: ClientRemoteCommand = serde_json::from_str(json).unwrap();
        match cmd {
            ClientRemoteCommand::MouseScroll { stream_id, data } => {
                assert_eq!(stream_id, "screen-0");
                assert_eq!(data.dx, 0);
                assert_eq!(data.dy, -10);
            }
            _ => panic!("Expected MouseScroll"),
        }
    }

    #[test]
    fn test_relative_mouse_drag_deserialization() {
        let json = r#"{"type":"mouse_drag","stream_id":"screen-0","data":{"from_rel_x":0.1,"from_rel_y":0.2,"to_rel_x":0.9,"to_rel_y":0.8,"button":"Left"}}"#;
        let cmd: ClientRemoteCommand = serde_json::from_str(json).unwrap();
        match cmd {
            ClientRemoteCommand::MouseDrag { stream_id, data } => {
                assert_eq!(stream_id, "screen-0");
                assert!((data.from_rel_x - 0.1).abs() < f64::EPSILON);
                assert!((data.from_rel_y - 0.2).abs() < f64::EPSILON);
                assert!((data.to_rel_x - 0.9).abs() < f64::EPSILON);
                assert!((data.to_rel_y - 0.8).abs() < f64::EPSILON);
                assert_eq!(data.button, MouseButton::Left);
            }
            _ => panic!("Expected MouseDrag"),
        }
    }

    #[test]
    fn test_key_press_deserialization() {
        let json = r#"{"type":"key_press","stream_id":"screen-0","data":{"key":"A","modifiers":["Ctrl","Shift"]}}"#;
        let cmd: ClientRemoteCommand = serde_json::from_str(json).unwrap();
        match cmd {
            ClientRemoteCommand::KeyPress { stream_id, data } => {
                assert_eq!(stream_id, "screen-0");
                assert_eq!(data.key, "A");
                assert_eq!(data.modifiers, vec!["Ctrl", "Shift"]);
            }
            _ => panic!("Expected KeyPress"),
        }
    }

    #[test]
    fn test_key_combo_deserialization() {
        let json = r#"{"type":"key_combo","stream_id":"screen-0","data":{"keys":["Ctrl","Alt","Delete"]}}"#;
        let cmd: ClientRemoteCommand = serde_json::from_str(json).unwrap();
        match cmd {
            ClientRemoteCommand::KeyCombo { stream_id, data } => {
                assert_eq!(stream_id, "screen-0");
                assert_eq!(data.keys, vec!["Ctrl", "Alt", "Delete"]);
            }
            _ => panic!("Expected KeyCombo"),
        }
    }

    #[test]
    fn test_ws_remote_client_new() {
        let client = WsRemoteClient::new(
            "ws://192.168.1.100:9001".to_string(),
            "password123".to_string(),
        );
        assert_eq!(client.url, "ws://192.168.1.100:9001");
        assert_eq!(client.password, "password123");
    }

    #[tokio::test]
    async fn test_ws_remote_client_initial_status() {
        let client = WsRemoteClient::new(
            "ws://192.168.1.100:9001".to_string(),
            "password123".to_string(),
        );
        let status = client.get_status().await;
        assert!(!status.is_connected);
        assert!(!status.is_reconnecting);
        assert_eq!(status.reconnect_attempt, 0);
        assert_eq!(status.remote_url, "");
    }

    #[tokio::test]
    async fn test_set_remote_resolution() {
        let client = WsRemoteClient::new(
            "ws://192.168.1.100:9001".to_string(),
            "password123".to_string(),
        );
        client.set_remote_resolution(1920, 1080).await;
        let resolution = client.remote_resolution.lock().await;
        assert_eq!(*resolution, Some((1920, 1080)));
    }

    #[tokio::test]
    async fn test_send_command_not_connected() {
        let client = WsRemoteClient::new(
            "ws://192.168.1.100:9001".to_string(),
            "password123".to_string(),
        );
        client.set_remote_resolution(1920, 1080).await;

        let cmd = ClientRemoteCommand::MouseMove {
            stream_id: "screen-0".to_string(),
            data: RelativeMouseMoveData {
                rel_x: 0.5,
                rel_y: 0.5,
            },
        };

        let result = client.send_command(cmd).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Not connected"));
    }

    #[tokio::test]
    async fn test_send_command_no_resolution() {
        let client = WsRemoteClient::new(
            "ws://192.168.1.100:9001".to_string(),
            "password123".to_string(),
        );

        // Manually set connected status (simulating a connection)
        {
            let mut status = client.status.lock().await;
            status.is_connected = true;
        }

        let cmd = ClientRemoteCommand::MouseMove {
            stream_id: "screen-0".to_string(),
            data: RelativeMouseMoveData {
                rel_x: 0.5,
                rel_y: 0.5,
            },
        };

        let result = client.send_command(cmd).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Remote resolution not available"));
    }

    #[test]
    fn test_coordinate_conversion() {
        // Test coordinate conversion logic
        let w: u32 = 1920;
        let h: u32 = 1080;

        // 0.5, 0.5 should map to center
        let x = (0.5_f64 * w as f64).round() as i32;
        let y = (0.5_f64 * h as f64).round() as i32;
        assert_eq!(x, 960);
        assert_eq!(y, 540);

        // 0.0, 0.0 should map to top-left
        let x = (0.0_f64 * w as f64).round() as i32;
        let y = (0.0_f64 * h as f64).round() as i32;
        assert_eq!(x, 0);
        assert_eq!(y, 0);

        // 1.0, 1.0 should map to bottom-right
        let x = (1.0_f64 * w as f64).round() as i32;
        let y = (1.0_f64 * h as f64).round() as i32;
        assert_eq!(x, 1920);
        assert_eq!(y, 1080);
    }
}
