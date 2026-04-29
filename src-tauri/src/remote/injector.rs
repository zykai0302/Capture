use crate::error::{AppError, AppResult};
use crate::remote::{ClickAction, MouseButton, RemoteCommand};
use enigo::{Axis, Button, Coordinate, Direction, Key, Keyboard, Mouse};
use std::sync::Mutex;

pub struct RemoteInjector {
    enigo: Mutex<enigo::Enigo>,
}

impl RemoteInjector {
    pub fn new() -> AppResult<Self> {
        let enigo = enigo::Enigo::new(&Default::default())
            .map_err(|e| AppError::Remote(format!("Failed to create Enigo: {}", e)))?;
        Ok(Self {
            enigo: Mutex::new(enigo),
        })
    }

    pub fn execute(&self, cmd: &RemoteCommand) -> AppResult<()> {
        let mut enigo = self.enigo.lock().unwrap();
        match cmd {
            RemoteCommand::MouseMove { data, .. } => {
                enigo
                    .move_mouse(data.x, data.y, Coordinate::Abs)
                    .map_err(|e| AppError::Remote(format!("Mouse move failed: {}", e)))?;
            }
            RemoteCommand::MouseClick { data, .. } => {
                let button = match data.button {
                    MouseButton::Left => Button::Left,
                    MouseButton::Right => Button::Right,
                    MouseButton::Middle => Button::Middle,
                };
                match data.action {
                    ClickAction::Single => {
                        enigo
                            .button(button, Direction::Click)
                            .map_err(|e| AppError::Remote(format!("Mouse click failed: {}", e)))?;
                    }
                    ClickAction::Double => {
                        enigo
                            .button(button, Direction::Click)
                            .map_err(|e| AppError::Remote(format!("Mouse click failed: {}", e)))?;
                        enigo
                            .button(button, Direction::Click)
                            .map_err(|e| AppError::Remote(format!("Mouse click failed: {}", e)))?;
                    }
                }
            }
            RemoteCommand::MouseScroll { data, .. } => {
                if data.dy != 0 {
                    let length = data.dy as i32;
                    enigo
                        .scroll(length, Axis::Vertical)
                        .map_err(|e| AppError::Remote(format!("Mouse scroll failed: {}", e)))?;
                }
                if data.dx != 0 {
                    let length = data.dx as i32;
                    enigo
                        .scroll(length, Axis::Horizontal)
                        .map_err(|e| AppError::Remote(format!("Mouse scroll failed: {}", e)))?;
                }
            }
            RemoteCommand::MouseDrag { data, .. } => {
                let button = match data.button {
                    MouseButton::Left => Button::Left,
                    MouseButton::Right => Button::Right,
                    MouseButton::Middle => Button::Middle,
                };
                enigo
                    .move_mouse(data.from_x, data.from_y, Coordinate::Abs)
                    .map_err(|e| AppError::Remote(format!("Mouse move failed: {}", e)))?;
                enigo
                    .button(button, Direction::Press)
                    .map_err(|e| AppError::Remote(format!("Mouse press failed: {}", e)))?;
                enigo
                    .move_mouse(data.to_x, data.to_y, Coordinate::Abs)
                    .map_err(|e| AppError::Remote(format!("Mouse move failed: {}", e)))?;
                enigo
                    .button(button, Direction::Release)
                    .map_err(|e| AppError::Remote(format!("Mouse release failed: {}", e)))?;
            }
            RemoteCommand::KeyPress { data, .. } => {
                let mod_keys: Vec<Key> = data
                    .modifiers
                    .iter()
                    .filter_map(|m| parse_key(m))
                    .collect();
                let key = parse_key(&data.key)
                    .ok_or_else(|| AppError::Remote(format!("Unknown key: {}", data.key)))?;

                for mod_key in &mod_keys {
                    let _ = enigo.key(*mod_key, Direction::Press);
                }
                let _ = enigo.key(key, Direction::Click);
                for mod_key in mod_keys.iter().rev() {
                    let _ = enigo.key(*mod_key, Direction::Release);
                }
            }
            RemoteCommand::KeyCombo { data, .. } => {
                let keys: Vec<Key> = data.keys.iter().filter_map(|k| parse_key(k)).collect();
                for key in &keys {
                    let _ = enigo.key(*key, Direction::Press);
                }
                for key in keys.iter().rev() {
                    let _ = enigo.key(*key, Direction::Release);
                }
            }
        }
        Ok(())
    }
}

fn parse_key(key: &str) -> Option<Key> {
    match key.to_lowercase().as_str() {
        "ctrl" | "control" => Some(Key::Control),
        "alt" => Some(Key::Alt),
        "shift" => Some(Key::Shift),
        "meta" | "win" | "super" => Some(Key::Meta),
        "tab" => Some(Key::Tab),
        "enter" | "return" => Some(Key::Return),
        "escape" | "esc" => Some(Key::Escape),
        "backspace" => Some(Key::Backspace),
        "delete" => Some(Key::Delete),
        "home" => Some(Key::Home),
        "end" => Some(Key::End),
        "pageup" => Some(Key::PageUp),
        "pagedown" => Some(Key::PageDown),
        "up" => Some(Key::UpArrow),
        "down" => Some(Key::DownArrow),
        "left" => Some(Key::LeftArrow),
        "right" => Some(Key::RightArrow),
        "space" => Some(Key::Space),
        "capslock" => Some(Key::CapsLock),
        "f1" => Some(Key::F1),
        "f2" => Some(Key::F2),
        "f3" => Some(Key::F3),
        "f4" => Some(Key::F4),
        "f5" => Some(Key::F5),
        "f6" => Some(Key::F6),
        "f7" => Some(Key::F7),
        "f8" => Some(Key::F8),
        "f9" => Some(Key::F9),
        "f10" => Some(Key::F10),
        "f11" => Some(Key::F11),
        "f12" => Some(Key::F12),
        s if s.len() == 1 => Some(Key::Unicode(s.chars().next().unwrap())),
        _ => None,
    }
}
