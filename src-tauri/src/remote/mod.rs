use serde::{Deserialize, Serialize};

pub mod injector;
pub mod websocket;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RemoteCommand {
    MouseMove {
        stream_id: String,
        data: MouseMoveData,
    },
    MouseClick {
        stream_id: String,
        data: MouseClickData,
    },
    MouseScroll {
        stream_id: String,
        data: MouseScrollData,
    },
    MouseDrag {
        stream_id: String,
        data: MouseDragData,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseMoveData {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseClickData {
    pub x: i32,
    pub y: i32,
    pub button: MouseButton,
    pub action: ClickAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClickAction {
    Single,
    Double,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseScrollData {
    pub x: i32,
    pub y: i32,
    pub dx: i32,
    pub dy: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseDragData {
    pub from_x: i32,
    pub from_y: i32,
    pub to_x: i32,
    pub to_y: i32,
    pub button: MouseButton,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPressData {
    pub key: String,
    pub modifiers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyComboData {
    pub keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteStatus {
    pub ws_port: u16,
    pub is_running: bool,
    pub client_count: u32,
    pub mouse_enabled: bool,
    pub keyboard_enabled: bool,
}

#[allow(unused_imports)]
pub use injector::RemoteInjector;
#[allow(unused_imports)]
pub use websocket::RemoteControlServer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_move_command_serialization() {
        let cmd = RemoteCommand::MouseMove {
            stream_id: "screen-0".to_string(),
            data: MouseMoveData { x: 100, y: 200 },
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("\"type\":\"mouse_move\""));
        assert!(json.contains("\"stream_id\":\"screen-0\""));
        assert!(json.contains("\"x\":100"));
        assert!(json.contains("\"y\":200"));
    }

    #[test]
    fn mouse_click_command_serialization() {
        let cmd = RemoteCommand::MouseClick {
            stream_id: "screen-0".to_string(),
            data: MouseClickData {
                x: 150,
                y: 300,
                button: MouseButton::Left,
                action: ClickAction::Single,
            },
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("\"type\":\"mouse_click\""));
        assert!(json.contains("\"button\":\"Left\""));
        assert!(json.contains("\"action\":\"Single\""));
    }

    #[test]
    fn mouse_click_right_double_serialization() {
        let cmd = RemoteCommand::MouseClick {
            stream_id: "screen-1".to_string(),
            data: MouseClickData {
                x: 50,
                y: 75,
                button: MouseButton::Right,
                action: ClickAction::Double,
            },
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("\"button\":\"Right\""));
        assert!(json.contains("\"action\":\"Double\""));
    }

    #[test]
    fn mouse_scroll_command_serialization() {
        let cmd = RemoteCommand::MouseScroll {
            stream_id: "screen-0".to_string(),
            data: MouseScrollData { x: 100, y: 200, dx: 0, dy: -3 },
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("\"type\":\"mouse_scroll\""));
        assert!(json.contains("\"dy\":-3"));
    }

    #[test]
    fn mouse_drag_command_serialization() {
        let cmd = RemoteCommand::MouseDrag {
            stream_id: "screen-0".to_string(),
            data: MouseDragData {
                from_x: 10,
                from_y: 20,
                to_x: 100,
                to_y: 200,
                button: MouseButton::Left,
            },
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("\"type\":\"mouse_drag\""));
        assert!(json.contains("\"from_x\":10"));
        assert!(json.contains("\"to_x\":100"));
    }

    #[test]
    fn key_press_command_serialization() {
        let cmd = RemoteCommand::KeyPress {
            stream_id: "screen-0".to_string(),
            data: KeyPressData {
                key: "A".to_string(),
                modifiers: vec!["Ctrl".to_string(), "Shift".to_string()],
            },
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("\"type\":\"key_press\""));
        assert!(json.contains("\"key\":\"A\""));
        assert!(json.contains("\"modifiers\":[\"Ctrl\",\"Shift\"]"));
    }

    #[test]
    fn key_combo_command_serialization() {
        let cmd = RemoteCommand::KeyCombo {
            stream_id: "screen-0".to_string(),
            data: KeyComboData {
                keys: vec!["Ctrl".to_string(), "Alt".to_string(), "Delete".to_string()],
            },
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("\"type\":\"key_combo\""));
        assert!(json.contains("\"keys\":[\"Ctrl\",\"Alt\",\"Delete\"]"));
    }

    #[test]
    fn remote_command_deserialization_roundtrip() {
        let commands = vec![
            RemoteCommand::MouseMove {
                stream_id: "s0".to_string(),
                data: MouseMoveData { x: 1, y: 2 },
            },
            RemoteCommand::MouseClick {
                stream_id: "s1".to_string(),
                data: MouseClickData {
                    x: 10,
                    y: 20,
                    button: MouseButton::Middle,
                    action: ClickAction::Double,
                },
            },
            RemoteCommand::MouseScroll {
                stream_id: "s2".to_string(),
                data: MouseScrollData { x: 0, y: 0, dx: 5, dy: -5 },
            },
            RemoteCommand::MouseDrag {
                stream_id: "s3".to_string(),
                data: MouseDragData {
                    from_x: 0,
                    from_y: 0,
                    to_x: 100,
                    to_y: 100,
                    button: MouseButton::Right,
                },
            },
            RemoteCommand::KeyPress {
                stream_id: "s4".to_string(),
                data: KeyPressData {
                    key: "Escape".to_string(),
                    modifiers: vec![],
                },
            },
            RemoteCommand::KeyCombo {
                stream_id: "s5".to_string(),
                data: KeyComboData {
                    keys: vec!["Ctrl".to_string(), "C".to_string()],
                },
            },
        ];

        for cmd in &commands {
            let json = serde_json::to_string(cmd).unwrap();
            let deserialized: RemoteCommand = serde_json::from_str(&json).unwrap();
            let json2 = serde_json::to_string(&deserialized).unwrap();
            assert_eq!(json, json2, "Roundtrip failed for {:?}", cmd);
        }
    }

    #[test]
    fn remote_status_serialization() {
        let status = RemoteStatus {
            ws_port: 9001,
            is_running: true,
            client_count: 2,
            mouse_enabled: true,
            keyboard_enabled: true,
        };
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: RemoteStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.ws_port, 9001);
        assert!(deserialized.is_running);
        assert_eq!(deserialized.client_count, 2);
    }

    #[test]
    fn remote_status_default_values() {
        let status = RemoteStatus {
            ws_port: 9001,
            is_running: false,
            client_count: 0,
            mouse_enabled: true,
            keyboard_enabled: true,
        };
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: RemoteStatus = serde_json::from_str(&json).unwrap();
        assert!(!deserialized.is_running);
        assert_eq!(deserialized.client_count, 0);
    }

    #[test]
    fn serde_tag_format_uses_snake_case() {
        // Verify that the serde tag = "type" and rename_all = "snake_case" is correct
        let cmd = RemoteCommand::MouseMove {
            stream_id: "s".to_string(),
            data: MouseMoveData { x: 0, y: 0 },
        };
        let json = serde_json::to_string(&cmd).unwrap();
        // The type tag should be snake_case: "mouse_move" not "MouseMove"
        assert!(json.contains("\"type\":\"mouse_move\""));

        let click_cmd = RemoteCommand::MouseClick {
            stream_id: "s".to_string(),
            data: MouseClickData {
                x: 0, y: 0, button: MouseButton::Left, action: ClickAction::Single,
            },
        };
        let json2 = serde_json::to_string(&click_cmd).unwrap();
        assert!(json2.contains("\"type\":\"mouse_click\""));
    }

    #[test]
    fn parse_mouse_move_from_json() {
        let json = r#"{"type":"mouse_move","stream_id":"screen-0","data":{"x":500,"y":300}}"#;
        let cmd: RemoteCommand = serde_json::from_str(json).unwrap();
        match cmd {
            RemoteCommand::MouseMove { stream_id, data } => {
                assert_eq!(stream_id, "screen-0");
                assert_eq!(data.x, 500);
                assert_eq!(data.y, 300);
            }
            _ => panic!("Expected MouseMove"),
        }
    }

    #[test]
    fn parse_key_press_from_json() {
        let json = r#"{"type":"key_press","stream_id":"screen-0","data":{"key":"Enter","modifiers":["Ctrl"]}}"#;
        let cmd: RemoteCommand = serde_json::from_str(json).unwrap();
        match cmd {
            RemoteCommand::KeyPress { stream_id, data } => {
                assert_eq!(stream_id, "screen-0");
                assert_eq!(data.key, "Enter");
                assert_eq!(data.modifiers, vec!["Ctrl"]);
            }
            _ => panic!("Expected KeyPress"),
        }
    }

    #[test]
    fn invalid_command_type_returns_error() {
        let json = r#"{"type":"invalid_type","stream_id":"s","data":{}}"#;
        let result = serde_json::from_str::<RemoteCommand>(json);
        assert!(result.is_err());
    }
}
