pub mod gst_pipeline;
pub mod manager;
pub mod preview;
pub mod mjpeg_server;

use crate::capture::source::CaptureSource;
use crate::encode::config::EncodeConfig;
use crate::error::AppResult;

/// Pipeline 状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PipelineState {
    Stopped,
    Starting,
    Running,
    Error(String),
}

/// 单路 Pipeline 实例的状态信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct PipelineStatus {
    pub source_id: String,
    pub state: PipelineState,
    pub encoder_used: String,
    pub is_gpu: bool,
    pub fps: f64,
    pub bitrate_kbps: u32,
    pub latency_ms: u32,
    pub rtsp_url: String,
}

/// Pipeline 管理器 trait
pub trait PipelineManager: Send + Sync {
    fn start_pipeline(&self, source: &CaptureSource, config: &EncodeConfig) -> AppResult<()>;
    fn stop_pipeline(&self, source_id: &str) -> AppResult<()>;
    fn stop_all(&self) -> AppResult<()>;
    fn get_status(&self, source_id: &str) -> Option<PipelineStatus>;
    fn get_all_status(&self) -> Vec<PipelineStatus>;
    fn update_config(&self, source_id: &str, config: &EncodeConfig) -> AppResult<()>;
    fn get_rtsp_url(&self, source_id: &str) -> Option<String>;
}

#[allow(unused_imports)]
pub use manager::GstPipelineManager;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_state_serialization() {
        let states = vec![
            PipelineState::Stopped,
            PipelineState::Starting,
            PipelineState::Running,
            PipelineState::Error("encoder failed".to_string()),
        ];

        let json = serde_json::to_string(&states).unwrap();
        assert!(json.contains("\"Stopped\""));
        assert!(json.contains("\"Starting\""));
        assert!(json.contains("\"Running\""));
        assert!(json.contains("\"Error\""));
    }

    #[test]
    fn pipeline_state_stopped_roundtrip() {
        let state = PipelineState::Stopped;
        let json = serde_json::to_string(&state).unwrap();
        let parsed: PipelineState = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, PipelineState::Stopped));
    }

    #[test]
    fn pipeline_state_running_roundtrip() {
        let state = PipelineState::Running;
        let json = serde_json::to_string(&state).unwrap();
        let parsed: PipelineState = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, PipelineState::Running));
    }

    #[test]
    fn pipeline_state_error_roundtrip() {
        let state = PipelineState::Error("encode failed".to_string());
        let json = serde_json::to_string(&state).unwrap();
        let parsed: PipelineState = serde_json::from_str(&json).unwrap();
        match parsed {
            PipelineState::Error(msg) => assert_eq!(msg, "encode failed"),
            _ => panic!("Expected Error state"),
        }
    }

    #[test]
    fn pipeline_status_serialization() {
        let status = PipelineStatus {
            source_id: "screen-0".to_string(),
            state: PipelineState::Running,
            encoder_used: "amfh264enc".to_string(),
            is_gpu: true,
            fps: 30.0,
            bitrate_kbps: 4000,
            latency_ms: 15,
            rtsp_url: "rtsp://127.0.0.1:8554/screen-0".to_string(),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"source_id\":\"screen-0\""));
        assert!(json.contains("\"is_gpu\":true"));
        assert!(json.contains("\"fps\":30.0"));
        assert!(json.contains("\"rtsp_url\":\"rtsp://127.0.0.1:8554/screen-0\""));
    }

    #[test]
    fn pipeline_state_debug_format() {
        let state = PipelineState::Error("test error".to_string());
        let debug = format!("{:?}", state);
        assert!(debug.contains("Error"));
        assert!(debug.contains("test error"));
    }

    #[test]
    fn pipeline_state_clone() {
        let state = PipelineState::Running;
        let cloned = state.clone();
        assert!(matches!(cloned, PipelineState::Running));
    }
}
