use thiserror::Error;

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
    #[error("RTSP client error: {0}")]
    RtspClient(String),
    #[error("Preview error: {0}")]
    Preview(String),
    #[error("Config error: {0}")]
    Config(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Windows error: {0}")]
    Windows(#[from] windows_result::Error),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_error_display_gstreamer() {
        let err = AppError::GStreamer("pipeline failed".to_string());
        assert_eq!(format!("{}", err), "GStreamer error: pipeline failed");
    }

    #[test]
    fn app_error_display_pipeline() {
        let err = AppError::Pipeline("already running".to_string());
        assert_eq!(format!("{}", err), "Pipeline error: already running");
    }

    #[test]
    fn app_error_display_capture() {
        let err = AppError::Capture("source not found".to_string());
        assert_eq!(format!("{}", err), "Capture error: source not found");
    }

    #[test]
    fn app_error_display_encode() {
        let err = AppError::Encode("encoder not available".to_string());
        assert_eq!(format!("{}", err), "Encode error: encoder not available");
    }

    #[test]
    fn app_error_display_rtsp() {
        let err = AppError::Rtsp("port in use".to_string());
        assert_eq!(format!("{}", err), "RTSP error: port in use");
    }

    #[test]
    fn app_error_display_remote() {
        let err = AppError::Remote("auth failed".to_string());
        assert_eq!(format!("{}", err), "Remote control error: auth failed");
    }

    #[test]
    fn app_error_display_rtsp_client() {
        let err = AppError::RtspClient("connection failed".to_string());
        assert_eq!(format!("{}", err), "RTSP client error: connection failed");
    }

    #[test]
    fn app_error_display_preview() {
        let err = AppError::Preview("pipeline failed".to_string());
        assert_eq!(format!("{}", err), "Preview error: pipeline failed");
    }

    #[test]
    fn app_error_display_config() {
        let err = AppError::Config("invalid port".to_string());
        assert_eq!(format!("{}", err), "Config error: invalid port");
    }

    #[test]
    fn app_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let app_err = AppError::from(io_err);
        assert!(matches!(app_err, AppError::Io(_)));
        assert!(format!("{}", app_err).contains("file missing"));
    }

    #[test]
    fn app_error_serializes_to_string() {
        let err = AppError::Capture("source not found".to_string());
        let json = serde_json::to_string(&err).unwrap();
        // Should serialize as a plain string
        assert_eq!(json, "\"Capture error: source not found\"");
    }

    #[test]
    fn app_error_debug_format() {
        let err = AppError::GStreamer("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("GStreamer"));
        assert!(debug.contains("test"));
    }

    #[test]
    fn app_result_ok() {
        let result: AppResult<i32> = Ok(42);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn app_result_err() {
        let result: AppResult<i32> = Err(AppError::Pipeline("failed".to_string()));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AppError::Pipeline(_)));
    }
}
