/// GPU encoder capability detector
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuCapability {
    pub has_amf: bool,
    pub has_mf: bool,
    pub has_vaapi: bool,
    pub has_videotoolbox: bool,
    pub amf_encoders: Vec<String>,
    pub mf_encoders: Vec<String>,
}

pub fn detect_gpu_capabilities() -> GpuCapability {
    let mut cap = GpuCapability {
        has_amf: false,
        has_mf: false,
        has_vaapi: false,
        has_videotoolbox: false,
        amf_encoders: vec![],
        mf_encoders: vec![],
    };

    for name in &["amfh264enc", "amfh265enc", "amfh264device2enc", "amfh265device2enc"] {
        if gstreamer::ElementFactory::find(name).is_some() {
            cap.has_amf = true;
            cap.amf_encoders.push(name.to_string());
        }
    }

    for name in &["mfh264enc", "mfh264device3enc"] {
        if gstreamer::ElementFactory::find(name).is_some() {
            cap.has_mf = true;
            cap.mf_encoders.push(name.to_string());
        }
    }

    cap
}
