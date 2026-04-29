use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceType {
    Monitor,
    Window,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureSource {
    pub id: String,           // "screen-0", "window-12345"
    pub name: String,         // "主显示器", "VS Code"
    pub source_type: SourceType,
    pub width: u32,
    pub height: u32,
    pub is_streaming: bool,
    pub rtsp_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureSourceList {
    pub monitors: Vec<CaptureSource>,
    pub windows: Vec<CaptureSource>,
}

impl CaptureSourceList {
    pub fn find_by_id(&self, id: &str) -> Option<&CaptureSource> {
        self.monitors
            .iter()
            .chain(self.windows.iter())
            .find(|s| s.id == id)
    }

    pub fn all(&self) -> Vec<&CaptureSource> {
        self.monitors.iter().chain(self.windows.iter()).collect()
    }

    pub fn contains_id(&self, id: &str) -> bool {
        self.find_by_id(id).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_monitor(id: &str, name: &str) -> CaptureSource {
        CaptureSource {
            id: id.to_string(),
            name: name.to_string(),
            source_type: SourceType::Monitor,
            width: 1920,
            height: 1080,
            is_streaming: false,
            rtsp_url: None,
        }
    }

    fn make_window(id: &str, name: &str) -> CaptureSource {
        CaptureSource {
            id: id.to_string(),
            name: name.to_string(),
            source_type: SourceType::Window,
            width: 800,
            height: 600,
            is_streaming: false,
            rtsp_url: None,
        }
    }

    #[test]
    fn find_by_id_returns_monitor() {
        let list = CaptureSourceList {
            monitors: vec![make_monitor("screen-0", "主显示器")],
            windows: vec![],
        };
        let found = list.find_by_id("screen-0");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "主显示器");
    }

    #[test]
    fn find_by_id_returns_window() {
        let list = CaptureSourceList {
            monitors: vec![],
            windows: vec![make_window("window-123", "VS Code")],
        };
        let found = list.find_by_id("window-123");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "VS Code");
    }

    #[test]
    fn find_by_id_returns_none_for_missing() {
        let list = CaptureSourceList {
            monitors: vec![make_monitor("screen-0", "主显示器")],
            windows: vec![],
        };
        assert!(list.find_by_id("screen-99").is_none());
    }

    #[test]
    fn find_by_id_searches_both_monitors_and_windows() {
        let list = CaptureSourceList {
            monitors: vec![make_monitor("screen-0", "主显示器")],
            windows: vec![make_window("window-1", "Terminal")],
        };
        // Window id should be found even though it's not in monitors
        assert!(list.find_by_id("window-1").is_some());
        // Monitor id should be found even though it's not in windows
        assert!(list.find_by_id("screen-0").is_some());
    }

    #[test]
    fn all_returns_combined_list() {
        let list = CaptureSourceList {
            monitors: vec![make_monitor("screen-0", "主显示器")],
            windows: vec![make_window("window-1", "Terminal")],
        };
        let all = list.all();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn all_returns_empty_when_no_sources() {
        let list = CaptureSourceList {
            monitors: vec![],
            windows: vec![],
        };
        assert!(list.all().is_empty());
    }

    #[test]
    fn contains_id_returns_true_for_existing() {
        let list = CaptureSourceList {
            monitors: vec![make_monitor("screen-0", "主显示器")],
            windows: vec![],
        };
        assert!(list.contains_id("screen-0"));
    }

    #[test]
    fn contains_id_returns_false_for_missing() {
        let list = CaptureSourceList {
            monitors: vec![make_monitor("screen-0", "主显示器")],
            windows: vec![],
        };
        assert!(!list.contains_id("screen-99"));
    }

    #[test]
    fn capture_source_serialization_roundtrip() {
        let source = make_monitor("screen-0", "主显示器");
        let json = serde_json::to_string(&source).unwrap();
        let deserialized: CaptureSource = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "screen-0");
        assert_eq!(deserialized.name, "主显示器");
        assert_eq!(deserialized.width, 1920);
        assert_eq!(deserialized.height, 1080);
    }

    #[test]
    fn source_type_serialization() {
        let monitor = SourceType::Monitor;
        let window = SourceType::Window;
        assert_eq!(
            serde_json::to_string(&monitor).unwrap(),
            serde_json::to_string(&window).unwrap().replace("Window", "Monitor")
        );
        // Verify roundtrip
        let json = serde_json::to_string(&SourceType::Monitor).unwrap();
        let parsed: SourceType = serde_json::from_str(&json).unwrap();
        matches!(parsed, SourceType::Monitor);
    }

    #[test]
    fn capture_source_list_serialization_roundtrip() {
        let list = CaptureSourceList {
            monitors: vec![make_monitor("screen-0", "主显示器")],
            windows: vec![make_window("window-1", "Terminal")],
        };
        let json = serde_json::to_string(&list).unwrap();
        let deserialized: CaptureSourceList = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.monitors.len(), 1);
        assert_eq!(deserialized.windows.len(), 1);
        assert_eq!(deserialized.monitors[0].id, "screen-0");
        assert_eq!(deserialized.windows[0].id, "window-1");
    }

    #[test]
    fn capture_source_with_rtsp_url() {
        let mut source = make_monitor("screen-0", "主显示器");
        source.is_streaming = true;
        source.rtsp_url = Some("rtsp://127.0.0.1:8554/screen-0".to_string());
        let json = serde_json::to_string(&source).unwrap();
        let deserialized: CaptureSource = serde_json::from_str(&json).unwrap();
        assert!(deserialized.is_streaming);
        assert_eq!(
            deserialized.rtsp_url,
            Some("rtsp://127.0.0.1:8554/screen-0".to_string())
        );
    }
}
