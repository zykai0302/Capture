use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Codec {
    H264,
    H265,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncodeMode {
    Auto,        // GPU 优先，不可用则 fallback CPU
    GpuOnly,
    CpuOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RateControl {
    CBR,
    VBR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncodePreset {
    Speed,
    Balanced,
    Quality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Resolution {
    Original,
    Custom { width: u32, height: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodeConfig {
    pub codec: Codec,
    pub mode: EncodeMode,
    pub resolution: Resolution,
    pub framerate: u32,        // 10-60
    pub bitrate_kbps: u32,     // 500-20000
    pub max_bitrate_kbps: u32,
    pub rate_control: RateControl,
    pub gop_size: u32,         // 10-120
    pub preset: EncodePreset,
}

impl Default for EncodeConfig {
    fn default() -> Self {
        Self {
            codec: Codec::H264,
            mode: EncodeMode::Auto,
            resolution: Resolution::Original,
            framerate: 30,
            bitrate_kbps: 4000,
            max_bitrate_kbps: 6000,
            rate_control: RateControl::VBR,
            gop_size: 30,
            preset: EncodePreset::Balanced,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_h264_auto() {
        let config = EncodeConfig::default();
        assert!(matches!(config.codec, Codec::H264));
        assert!(matches!(config.mode, EncodeMode::Auto));
        assert!(matches!(config.resolution, Resolution::Original));
        assert_eq!(config.framerate, 30);
        assert_eq!(config.bitrate_kbps, 4000);
        assert_eq!(config.max_bitrate_kbps, 6000);
        assert!(matches!(config.rate_control, RateControl::VBR));
        assert_eq!(config.gop_size, 30);
        assert!(matches!(config.preset, EncodePreset::Balanced));
    }

    #[test]
    fn encode_config_serialization_roundtrip() {
        let config = EncodeConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: EncodeConfig = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized.codec, Codec::H264));
        assert!(matches!(deserialized.mode, EncodeMode::Auto));
        assert_eq!(deserialized.framerate, 30);
    }

    #[test]
    fn codec_serialization() {
        assert_eq!(
            serde_json::to_string(&Codec::H264).unwrap(),
            "\"H264\""
        );
        assert_eq!(
            serde_json::to_string(&Codec::H265).unwrap(),
            "\"H265\""
        );
    }

    #[test]
    fn encode_mode_serialization() {
        assert_eq!(
            serde_json::to_string(&EncodeMode::Auto).unwrap(),
            "\"Auto\""
        );
        assert_eq!(
            serde_json::to_string(&EncodeMode::GpuOnly).unwrap(),
            "\"GpuOnly\""
        );
        assert_eq!(
            serde_json::to_string(&EncodeMode::CpuOnly).unwrap(),
            "\"CpuOnly\""
        );
    }

    #[test]
    fn rate_control_serialization() {
        assert_eq!(
            serde_json::to_string(&RateControl::CBR).unwrap(),
            "\"CBR\""
        );
        assert_eq!(
            serde_json::to_string(&RateControl::VBR).unwrap(),
            "\"VBR\""
        );
    }

    #[test]
    fn encode_preset_serialization() {
        assert_eq!(
            serde_json::to_string(&EncodePreset::Speed).unwrap(),
            "\"Speed\""
        );
        assert_eq!(
            serde_json::to_string(&EncodePreset::Balanced).unwrap(),
            "\"Balanced\""
        );
        assert_eq!(
            serde_json::to_string(&EncodePreset::Quality).unwrap(),
            "\"Quality\""
        );
    }

    #[test]
    fn resolution_original_serialization() {
        let res = Resolution::Original;
        let json = serde_json::to_string(&res).unwrap();
        assert_eq!(json, "\"Original\"");
        let parsed: Resolution = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, Resolution::Original));
    }

    #[test]
    fn resolution_custom_serialization() {
        let res = Resolution::Custom { width: 1920, height: 1080 };
        let json = serde_json::to_string(&res).unwrap();
        let parsed: Resolution = serde_json::from_str(&json).unwrap();
        match parsed {
            Resolution::Custom { width, height } => {
                assert_eq!(width, 1920);
                assert_eq!(height, 1080);
            }
            _ => panic!("Expected Custom resolution"),
        }
    }

    #[test]
    fn custom_encode_config_serialization() {
        let config = EncodeConfig {
            codec: Codec::H265,
            mode: EncodeMode::CpuOnly,
            resolution: Resolution::Custom { width: 1280, height: 720 },
            framerate: 60,
            bitrate_kbps: 8000,
            max_bitrate_kbps: 10000,
            rate_control: RateControl::CBR,
            gop_size: 60,
            preset: EncodePreset::Speed,
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: EncodeConfig = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized.codec, Codec::H265));
        assert!(matches!(deserialized.mode, EncodeMode::CpuOnly));
        assert_eq!(deserialized.framerate, 60);
        assert_eq!(deserialized.bitrate_kbps, 8000);
    }
}
