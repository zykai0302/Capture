# 08 — 远程反控模块

## 1 模块结构

```
remote/
├── mod.rs       → RemoteCommand, MouseMoveData, MouseClickData, ..., RemoteStatus
├── injector.rs  → RemoteInjector
└── websocket.rs → RemoteControlServer, handle_client()
```

## 2 命令协议 (`mod.rs`)

详见 [02-DATA-TYPES.md](./02-DATA-TYPES.md) §1.6。

### 2.1 序列化格式

使用 `#[serde(tag = "type", rename_all = "snake_case")]`：

| Rust 变体 | JSON type 值 |
|-----------|-------------|
| `MouseMove` | `"mouse_move"` |
| `MouseClick` | `"mouse_click"` |
| `MouseScroll` | `"mouse_scroll"` |
| `MouseDrag` | `"mouse_drag"` |
| `KeyPress` | `"key_press"` |
| `KeyCombo` | `"key_combo"` |

### 2.2 完整 JSON 示例

**鼠标移动**:
```json
{"type":"mouse_move","stream_id":"screen-0","data":{"x":500,"y":300}}
```

**鼠标点击**:
```json
{"type":"mouse_click","stream_id":"screen-0","data":{"x":100,"y":200,"button":"Left","action":"Single"}}
```

**鼠标双击**:
```json
{"type":"mouse_click","stream_id":"screen-0","data":{"x":100,"y":200,"button":"Left","action":"Double"}}
```

**鼠标滚动**:
```json
{"type":"mouse_scroll","stream_id":"screen-0","data":{"x":0,"y":0,"dx":0,"dy":-3}}
```

**鼠标拖拽**:
```json
{"type":"mouse_drag","stream_id":"screen-0","data":{"from_x":10,"from_y":20,"to_x":100,"to_y":200,"button":"Left"}}
```

**按键**:
```json
{"type":"key_press","stream_id":"screen-0","data":{"key":"Enter","modifiers":["Ctrl"]}}
```

**组合键**:
```json
{"type":"key_combo","stream_id":"screen-0","data":{"keys":["Ctrl","Alt","Delete"]}}
```

---

## 3 输入注入器 (`injector.rs`)

### 3.1 数据结构

```rust
pub struct RemoteInjector {
    enigo: Mutex<enigo::Enigo>,
}
```

### 3.2 `RemoteInjector::new() -> AppResult<Self>`

```rust
let enigo = enigo::Enigo::new(&Default::default())
    .map_err(|e| AppError::Remote(format!("Failed to create Enigo: {}", e)))?;
Ok(Self { enigo: Mutex::new(enigo) })
```

### 3.3 `execute(&self, cmd: &RemoteCommand) -> AppResult<()>`

```rust
let mut enigo = self.enigo.lock().unwrap();
match cmd {
    MouseMove → enigo.move_mouse(data.x, data.y, Coordinate::Abs)
    MouseClick → {
        button = match data.button { Left→Button::Left, Right→Button::Right, Middle→Button::Middle }
        match data.action {
            Single → enigo.button(button, Direction::Click)
            Double → enigo.button(button, Direction::Click) × 2
        }
    }
    MouseScroll → {
        if data.dy != 0: enigo.scroll(data.dy as i32, Axis::Vertical)
        if data.dx != 0: enigo.scroll(data.dx as i32, Axis::Horizontal)
    }
    MouseDrag → {
        enigo.move_mouse(data.from_x, data.from_y, Coordinate::Abs)
        enigo.button(button, Direction::Press)
        enigo.move_mouse(data.to_x, data.to_y, Coordinate::Abs)
        enigo.button(button, Direction::Release)
    }
    KeyPress → {
        mod_keys = data.modifiers.iter().filter_map(parse_key)
        key = parse_key(&data.key)
        for mod_key in &mod_keys: enigo.key(*mod_key, Direction::Press)
        enigo.key(key, Direction::Click)
        for mod_key in mod_keys.iter().rev(): enigo.key(*mod_key, Direction::Release)
    }
    KeyCombo → {
        keys = data.keys.iter().filter_map(parse_key)
        for key in &keys: enigo.key(*key, Direction::Press)
        for key in keys.iter().rev(): enigo.key(*key, Direction::Release)
    }
}
```

### 3.4 键映射 (`parse_key`)

```rust
fn parse_key(key: &str) -> Option<Key> {
    match key.to_lowercase().as_str() {
        "ctrl" | "control"  => Key::Control,
        "alt"               => Key::Alt,
        "shift"             => Key::Shift,
        "meta" | "win" | "super" => Key::Meta,
        "tab"               => Key::Tab,
        "enter" | "return"  => Key::Return,
        "escape" | "esc"    => Key::Escape,
        "backspace"         => Key::Backspace,
        "delete"            => Key::Delete,
        "home"              => Key::Home,
        "end"               => Key::End,
        "pageup"            => Key::PageUp,
        "pagedown"          => Key::PageDown,
        "up"                => Key::UpArrow,
        "down"              => Key::DownArrow,
        "left"              => Key::LeftArrow,
        "right"             => Key::RightArrow,
        "space"             => Key::Space,
        "capslock"          => Key::CapsLock,
        "f1"..="f12"        => Key::F1..Key::F12,
        s if s.len() == 1   => Key::Unicode(s.chars().next().unwrap()),
        _                   => None,
    }
}
```

---

## 4 WebSocket 服务 (`websocket.rs`)

### 4.1 数据结构

```rust
pub struct RemoteControlServer {
    pub port: u16,
    pub password: String,
    pub running: Arc<AtomicBool>,
    pub client_count: Arc<AtomicU32>,
    injector: Arc<RemoteInjector>,
    shutdown_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}
```

> 注意：`port`, `password`, `running`, `client_count` 为 `pub`，被 `lib.rs::get_remote_status()` 直接读取。

### 4.2 `RemoteControlServer::new(port, password, injector) -> Self`

```
port, password, injector = 参数
running = AtomicBool::new(false)
client_count = AtomicU32::new(0)
shutdown_tx = Arc::new(Mutex::new(None))
```

### 4.3 `start(&self) -> AppResult<()>`

```
1. TcpListener::bind("0.0.0.0:{port}")
2. running.store(true, SeqCst)
3. 创建 oneshot channel (shutdown_tx, shutdown_rx)
4. tokio::spawn 主循环:
   loop {
     tokio::select! {
       result = listener.accept() → 处理新连接
       _ = shutdown_rx → break
     }
     
     新连接:
       tokio_tungstenite::accept_async(stream)  // WS 握手
       client_count.fetch_add(1)
       tokio::spawn handle_client(...)
   }
```

### 4.4 `stop(&self) -> AppResult<()>`

```
1. running.store(false, SeqCst)
2. shutdown_tx.lock().await.take().send(())  // 通知主循环退出
```

### 4.5 `handle_client(ws_stream, addr, password, injector)`

```rust
let (mut ws_tx, mut ws_rx) = ws_stream.split();
let mut authenticated = password.is_empty();      // 无密码 = 自动认证
let mut auth_message_received = false;            // 追踪显式认证

loop {
    match ws_rx.next().await {
        Ok(Message::Text(text)) => {
            // 始终先检查认证消息（防止泄漏到命令解析）
            if let Ok(auth_msg) = serde_json::from_str::<Value>(&text):
                if auth_msg["type"] == "auth":
                    if authenticated && auth_message_received:
                        // 已显式认证，确认即可
                        ws_tx.send({"status":"ok","type":"auth"})
                    else:
                        // 验证密码（包括空密码模式下的首次认证）
                        if provided_password == password:
                            authenticated = true
                            auth_message_received = true
                            ws_tx.send({"status":"ok","type":"auth"})
                        else:
                            ws_tx.send({"status":"error","message":"Authentication failed"})
                            break  // 断开连接
                    continue

            // 未认证则拒绝
            if !authenticated:
                ws_tx.send({"status":"error","message":"Not authenticated"})
                break

            // 已认证，执行命令
            match serde_json::from_str::<RemoteCommand>(&text):
                Ok(cmd) → injector.execute(&cmd)
                    Ok → ws_tx.send({"status":"ok"})
                    Err → ws_tx.send({"status":"error","message":"{e}"})
                Err(e) → ws_tx.send({"status":"error","message":"Invalid command: {e}"})
        }
        Ok(Message::Close(_)) => break
        Ok(Message::Ping(data)) => ws_tx.send(Message::Pong(data))
        Err(e) => break  // 连接错误
        _ => {}  // Binary/Pong 等忽略
    }
}

client_count.fetch_sub(1)
```

### 4.6 认证流程时序

```
客户端                           服务端
  │                               │
  │──── TCP 连接 ────────────────→│
  │──── WS 握手 ─────────────────→│
  │                               │
  │  (若有密码)                    │
  │──── {"type":"auth",           │
  │      "password":"xxx"} ──────→│
  │                               │ 验证密码
  │←─── {"status":"ok",           │
  │       "type":"auth"} ─────────│
  │                               │
  │  (认证失败)                    │
  │←─── {"status":"error",        │
  │       "message":"Authentication │
  │        failed"} ─────────────│
  │          (连接断开)            │
  │                               │
  │  (已认证，发送命令)             │
  │──── {"type":"mouse_move",...}→│
  │←─── {"status":"ok"} ─────────│
```

### 4.7 认证安全（双标志机制）

| 状态 | `authenticated` | `auth_message_received` | 行为 |
|------|----------------|------------------------|------|
| 空密码，首次 auth | `true` | `false` | 验证密码（仅空串通过） |
| 空密码，重复 auth | `true` | `true` | 确认即可 |
| 有密码，未认证 | `false` | `false` | 验证密码 |
| 有密码，已认证 | `true` | `true` | 确认即可 |

> **为什么需要 `auth_message_received`**: 如果仅用 `authenticated`，空密码模式下 `authenticated=true`，任何 auth 消息都会走"已认证确认"分支——即使用户输入了错误密码也会被接受。添加 `auth_message_received` 确保首次 auth 消息必须通过密码验证。

---

## 5 默认配置

| 参数 | 值 | 来源 |
|------|-----|------|
| WS 端口 | `9001` | `AppConfig::default().ws_port` |
| WS 密码 | `""` (空，自动认证) | `AppConfig::default().ws_password` |
| 绑定地址 | `0.0.0.0` | `websocket.rs` 硬编码 |
