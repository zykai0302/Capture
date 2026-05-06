/// GPU encoder capability detector
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuCapability {
    // Windows
    pub has_amf: bool,
    pub has_mf: bool,
    // macOS
    pub has_videotoolbox: bool,
    // Linux
    pub has_vaapi: bool,

    // Detailed encoder lists
    pub amf_encoders: Vec<String>,
    pub mf_encoders: Vec<String>,
    pub vt_encoders: Vec<String>,
    pub vaapi_encoders: Vec<String>,
}

pub fn detect_gpu_capabilities() -> GpuCapability {
    let mut cap = GpuCapability {
        has_amf: false,
        has_mf: false,
        has_videotoolbox: false,
        has_vaapi: false,
        amf_encoders: vec![],
        mf_encoders: vec![],
        vt_encoders: vec![],
        vaapi_encoders: vec![],
    };

    // Windows: AMF (AMD/NVIDIA) and Media Foundation (Intel/NVIDIA)
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

    // macOS: VideoToolbox
    for name in &["vtenc_h264", "vtenc_h265"] {
        if gstreamer::ElementFactory::find(name).is_some() {
            cap.has_videotoolbox = true;
            cap.vt_encoders.push(name.to_string());
        }
    }

    // Linux: VAAPI
    for name in &["vaapih264enc", "vaapih265enc"] {
        if gstreamer::ElementFactory::find(name).is_some() {
            cap.has_vaapi = true;
            cap.vaapi_encoders.push(name.to_string());
        }
    }

    cap
}
