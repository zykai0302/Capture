use serde::{Deserialize, Serialize};
use crate::encode::config::EncodeConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub rtsp_port: u16,           // default: 8554
    pub rtsp_max_clients: u32,    // default: 10
    pub ws_port: u16,             // default: 9001
    pub ws_password: String,
    pub auto_reconnect: bool,
    pub default_encode: EncodeConfig,
    pub preview_http_port: u16,   // default: 8090
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            rtsp_port: 8554,
            rtsp_max_clients: 10,
            ws_port: 9001,
            ws_password: String::new(),
            auto_reconnect: true,
            default_encode: EncodeConfig::default(),
            preview_http_port: 8090,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::config::{Codec, EncodeMode};

    #[test]
    fn default_config_has_expected_values() {
        let config = AppConfig::default();
        assert_eq!(config.rtsp_port, 8554);
        assert_eq!(config.rtsp_max_clients, 10);
        assert_eq!(config.ws_port, 9001);
        assert!(config.ws_password.is_empty());
        assert!(config.auto_reconnect);
    }

    #[test]
    fn default_config_has_preview_http_port() {
        let config = AppConfig::default();
        assert_eq!(config.preview_http_port, 8090);
    }

    #[test]
    fn default_config_inherits_encode_defaults() {
        let config = AppConfig::default();
        assert!(matches!(config.default_encode.codec, Codec::H264));
        assert!(matches!(config.default_encode.mode, EncodeMode::Auto));
        assert_eq!(config.default_encode.framerate, 30);
        assert_eq!(config.default_encode.bitrate_kbps, 4000);
    }

    #[test]
    fn app_config_serialization_roundtrip() {
        let config = AppConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.rtsp_port, 8554);
        assert_eq!(deserialized.rtsp_max_clients, 10);
        assert_eq!(deserialized.ws_port, 9001);
        assert!(deserialized.auto_reconnect);
    }

    #[test]
    fn app_config_custom_values() {
        let config = AppConfig {
            rtsp_port: 9000,
            rtsp_max_clients: 5,
            ws_port: 8080,
            ws_password: "secret".to_string(),
            auto_reconnect: false,
            default_encode: EncodeConfig {
                codec: Codec::H265,
                mode: EncodeMode::CpuOnly,
                ..EncodeConfig::default()
            },
            preview_http_port: 9090,
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.rtsp_port, 9000);
        assert_eq!(deserialized.rtsp_max_clients, 5);
        assert_eq!(deserialized.ws_port, 8080);
        assert_eq!(deserialized.ws_password, "secret");
        assert!(!deserialized.auto_reconnect);
        assert!(matches!(deserialized.default_encode.codec, Codec::H265));
        assert_eq!(deserialized.preview_http_port, 9090);
    }

    #[test]
    fn app_config_clone_independence() {
        let config = AppConfig::default();
        let mut cloned = config.clone();
        cloned.rtsp_port = 9999;
        // Original should be unchanged
        assert_eq!(config.rtsp_port, 8554);
        assert_eq!(cloned.rtsp_port, 9999);
    }

    #[test]
    fn ws_password_serialization() {
        let config = AppConfig {
            ws_password: "p@ssw0rd!测试".to_string(),
            ..AppConfig::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.ws_password, "p@ssw0rd!测试");
    }
}
