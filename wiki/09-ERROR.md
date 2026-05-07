# 09 — 错误体系

## 1 错误类型定义 (`error.rs`)

```rust
#[derive(Error, Debug)]
pub enum AppError {
    #[error("GStreamer error: {0}")]
    GStreamer(String),
    #[error("Pipeline error: {0}")]
    Pipeline(String),
    #[error("Capture error: {0}")]
    Capture(String),
    #[error("Encode error: {0}")]
    Encode(String),
    #[error("RTSP error: {0}")]
    Rtsp(String),
    #[error("Remote control error: {0}")]
    Remote(String),
    #[error("Preview error: {0}")]
    Preview(String),
    #[error("Config error: {0}")]
    Config(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

## 2 序列化

```rust
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        serializer.serialize_str(self.to_string().as_str())
    }
}
```

**效果**: 前端收到的错误是纯字符串，如 `"Capture error: source not found"`

## 3 类型别名

```rust
pub type AppResult<T> = Result<T, AppError>;
```

## 4 错误传播路径

```
capture::platform::enumerate_sources()
  └→ AppError::Capture("Task join error: {e}")     [lib.rs, list_sources]

pipeline_manager.start_pipeline()
  ├→ AppError::Rtsp(...)                            [rtsp/server.rs]
  ├→ AppError::Pipeline("Task join error: {e}")     [lib.rs, start_stream]
  └→ 内部 AppError 透传

pipeline_manager.stop_pipeline()
  └→ AppError::Pipeline("Task join error: {e}")     [lib.rs, stop_stream]

pipeline_manager.update_config()
  ├→ AppError::Capture("Source not found: {id}")    [manager.rs]
  └→ start_pipeline 的错误透传

RemoteInjector::new()
  └→ AppError::Remote("Failed to create Enigo: {e}") [injector.rs]

RemoteInjector::execute()
  └→ AppError::Remote("Mouse move failed: {e}")     [injector.rs]
  └→ AppError::Remote("Mouse click failed: {e}")
  └→ AppError::Remote("Mouse scroll failed: {e}")
  └→ AppError::Remote("Unknown key: {key}")

RemoteControlServer::start()
  └→ AppError::Remote("Failed to bind WebSocket: {e}") [websocket.rs]

RemoteControlServer::stop()
  └→ 透传内部错误
```

## 5 错误在各 Tauri 命令中的包装

所有 `spawn_blocking` 的 JoinError 统一包装为对应类别的错误：

```rust
// list_sources
.map_err(|e| AppError::Capture(format!("Task join error: {}", e)))?

// start_stream
.map_err(|e| AppError::Pipeline(format!("Task join error: {}", e)))?

// stop_stream
.map_err(|e| AppError::Pipeline(format!("Task join error: {}", e)))?

// stop_all_streams
.map_err(|e| AppError::Pipeline(format!("Task join error: {}", e)))?

// get_pipeline_status
.map_err(|e| AppError::Pipeline(format!("Task join error: {}", e)))?

// update_encode_config
.map_err(|e| AppError::Pipeline(format!("Task join error: {}", e)))?

// get_available_encoders
.map_err(|e| AppError::GStreamer(format!("Task join error: {}", e)))?

// get_gpu_capabilities
.map_err(|e| AppError::Encode(format!("Task join error: {}", e)))?
```

## 6 AppError Display 输出示例

| 变体 | Display 输出 |
|------|-------------|
| `GStreamer("pipeline failed")` | `"GStreamer error: pipeline failed"` |
| `Pipeline("already running")` | `"Pipeline error: already running"` |
| `Capture("source not found")` | `"Capture error: source not found"` |
| `Encode("encoder not available")` | `"Encode error: encoder not available"` |
| `Rtsp("port in use")` | `"RTSP error: port in use"` |
| `Remote("auth failed")` | `"Remote control error: auth failed"` |
| `Preview("pipeline failed")` | `"Preview error: pipeline failed"` |
| `Config("invalid port")` | `"Config error: invalid port"` |
| `Io(io::Error)` | `"IO error: {io_error_message}""` |
