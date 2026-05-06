# 12 — 测试体系

## 1 测试层级

| 层级 | 框架 | 位置 | 运行命令 |
|------|------|------|---------|
| Rust 单元测试 | `cargo test` | `src-tauri/src/**/*.rs` (内嵌 `#[cfg(test)]`) | `cargo test --manifest-path src-tauri/Cargo.toml` |
| 前端单元测试 | Vitest + Vue Test Utils + happy-dom | `src/__tests__/` | `npm run test` |
| E2E 集成测试 | Python + OpenCV + websocket-client | `tests/` | `python tests/test_e2e.py` |

---

## 2 Rust 单元测试

每个模块内嵌 `#[cfg(test)] mod tests`，以下为完整测试清单：

### 2.1 `capture/source.rs`

| 测试名 | 断言 |
|--------|------|
| `find_by_id_returns_monitor` | monitor 列表中按 id 查找成功 |
| `find_by_id_returns_window` | window 列表中按 id 查找成功 |
| `find_by_id_returns_none_for_missing` | 不存在的 id 返回 None |
| `find_by_id_searches_both_monitors_and_windows` | 跨 monitors/windows 查找 |
| `all_returns_combined_list` | all() 返回合并列表 |
| `all_returns_empty_when_no_sources` | 空列表返回空 |
| `contains_id_returns_true_for_existing` | 存在的 id 返回 true |
| `contains_id_returns_false_for_missing` | 不存在的 id 返回 false |
| `capture_source_serialization_roundtrip` | 序列化-反序列化一致性 |
| `source_type_serialization` | SourceType 枚举序列化 |
| `capture_source_list_serialization_roundtrip` | 列表序列化-反序列化 |
| `capture_source_with_rtsp_url` | 含 rtsp_url 的源序列化 |

### 2.2 `config/mod.rs`

| 测试名 | 断言 |
|--------|------|
| `default_config_has_expected_values` | rtsp_port=8554, ws_port=9001, auto_reconnect=true |
| `default_config_inherits_encode_defaults` | codec=H264, mode=Auto, framerate=30 |
| `app_config_serialization_roundtrip` | 序列化-反序列化一致性 |
| `app_config_custom_values` | 自定义值序列化-反序列化 |
| `app_config_clone_independence` | clone 后互不影响 |
| `ws_password_serialization` | Unicode 密码 "p@ssw0rd!测试" 序列化 |

### 2.3 `encode/config.rs`

| 测试名 | 断言 |
|--------|------|
| `default_config_has_h264_auto` | 默认值正确 |
| `encode_config_serialization_roundtrip` | 序列化一致性 |
| `codec_serialization` | `"H264"`, `"H265"` |
| `encode_mode_serialization` | `"Auto"`, `"GpuOnly"`, `"CpuOnly"` |
| `rate_control_serialization` | `"CBR"`, `"VBR"` |
| `encode_preset_serialization` | `"Speed"`, `"Balanced"`, `"Quality"` |
| `resolution_original_serialization` | `"Original"` |
| `resolution_custom_serialization` | `{Custom:{width:1920,height:1080}}` |
| `custom_encode_config_serialization` | 自定义值 |

### 2.4 `pipeline/mod.rs`

| 测试名 | 断言 |
|--------|------|
| `pipeline_state_serialization` | 所有状态序列化包含正确标记 |
| `pipeline_state_stopped_roundtrip` | Stopped 反序列化 |
| `pipeline_state_running_roundtrip` | Running 反序列化 |
| `pipeline_state_error_roundtrip` | Error("encode failed") 反序列化 |
| `pipeline_status_serialization` | 完整 PipelineStatus 字段验证 |
| `pipeline_state_debug_format` | Debug 格式包含 Error 和消息 |
| `pipeline_state_clone` | Clone 正确性 |

### 2.5 `pipeline/gst_pipeline.rs`

| 测试名 | 断言 |
|--------|------|
| `gpu_encoder_candidates_h264` | 4 个候选: amfh264enc, amfh264device2enc, mfh264enc, mfh264device3enc |
| `gpu_encoder_candidates_h265` | 2 个候选: amfh265enc, amfh265device2enc |
| `h265_has_fewer_gpu_candidates_than_h264` | H265 < H264 |
| `cpu_encoder_h264` | `"x264enc bitrate=4000 speed-preset=medium"` |
| `cpu_encoder_h265` | `"x265enc bitrate=8000 speed-preset=medium"` |
| `format_amfh264enc_params` | `"amfh264enc bitrate=6000 gop-size=60"` |
| `format_amfh265enc_params` | `"amfh265enc bitrate=8000 gop-size=30"` |
| `format_mfh264enc_params` | `"mfh264enc bitrate=5000 gop-size=45"` |
| `format_unknown_encoder_params` | `"somecustomenc bitrate=3000"` |
| `build_launch_string_monitor_extracts_index` | "screen-2" → `monitor-index=2` |
| `build_launch_string_monitor_default_index` | 非法 id → `monitor-index=0` |
| `build_launch_string_window_extracts_handle` | "window-12345678" → `window-handle=12345678` |
| `build_launch_string_window_default_handle` | 非法 id → `window-handle=0` |
| `build_launch_string_h264_uses_rtph264pay` | 包含 `rtph264pay name=pay0 pt=96` |
| `build_launch_string_h265_uses_rtph265pay` | 包含 `rtph265pay name=pay0 pt=96` |
| `build_launch_string_monitor_uses_d3d12_screencapture` | 包含 d3d12screencapturesrc + monitor-index=0 |
| `build_launch_string_window_uses_d3d12_screencapture` | 包含 d3d12screencapturesrc + window-handle=100 |

### 2.6 `remote/mod.rs`

| 测试名 | 断言 |
|--------|------|
| `mouse_move_command_serialization` | `"type":"mouse_move"` + x/y |
| `mouse_click_command_serialization` | `"type":"mouse_click"` + button/action |
| `mouse_click_right_double_serialization` | Right/Double |
| `mouse_scroll_command_serialization` | `"type":"mouse_scroll"` + dy |
| `mouse_drag_command_serialization` | `"type":"mouse_drag"` + from/to |
| `key_press_command_serialization` | `"type":"key_press"` + key/modifiers |
| `key_combo_command_serialization` | `"type":"key_combo"` + keys |
| `remote_command_deserialization_roundtrip` | 所有命令类型序列化-反序列化 |
| `remote_status_serialization` | ws_port/is_running/client_count |
| `remote_status_default_values` | is_running=false, client_count=0 |
| `serde_tag_format_uses_snake_case` | type 标签为 snake_case |
| `parse_mouse_move_from_json` | 从 JSON 字符串解析 |
| `parse_key_press_from_json` | 从 JSON 字符串解析 |
| `invalid_command_type_returns_error` | 无效类型解析失败 |

### 2.7 `error.rs`

| 测试名 | 断言 |
|--------|------|
| `app_error_display_gstreamer` | `"GStreamer error: pipeline failed"` |
| `app_error_display_pipeline` | `"Pipeline error: already running"` |
| `app_error_display_capture` | `"Capture error: source not found"` |
| `app_error_display_encode` | `"Encode error: encoder not available"` |
| `app_error_display_rtsp` | `"RTSP error: port in use"` |
| `app_error_display_remote` | `"Remote control error: auth failed"` |
| `app_error_display_config` | `"Config error: invalid port"` |
| `app_error_from_io_error` | From 转换 |
| `app_error_serializes_to_string` | `"Capture error: source not found"` |
| `app_error_debug_format` | Debug 包含类型和消息 |
| `app_result_ok` | Ok 分支 |
| `app_result_err` | Err 分支 |

---

## 3 前端单元测试

### 3.1 Mock 设置

```typescript
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(() => Promise.resolve(vi.fn())) }))
vi.mock('vue', async () => {
  const actual = await vi.importActual<typeof import('vue')>('vue')
  return { ...actual, onMounted: vi.fn((fn) => fn()), onUnmounted: vi.fn() }
})
```

### 3.2 `composables.test.ts`

**usePipeline**:
- 初始化为空 pipeline map
- refreshStatus 填充 pipeline map
- startStream 调用 invoke 并刷新
- stopStream 调用 invoke
- stopAllStreams 调用 invoke
- invoke 失败设置 error
- isStreaming 判断
- getStatus 返回指定源状态

**useSources**:
- 初始化为空数组
- refresh 填充 monitors 和 windows
- invoke 失败设置 error

**useConfig**:
- 初始化 config 和 gpuCaps 为 null
- loadConfig 填充 config
- loadGpuCapabilities 填充 gpuCaps
- getAvailableEncoders 返回编码器列表
- getAvailableEncoders 失败返回空数组

**useRemoteControl**:
- 初始化默认状态 (ws_port=9001, is_running=false)
- refresh 从后端更新状态
- start 调用 invoke 带 port 和 password
- stop 调用 invoke
- 失败设置 error

### 3.3 `types.test.ts`

- 各枚举值验证
- defaultEncodeConfig 默认值验证
- defaultEncodeConfig 每次返回新对象
- Resolution 类型验证
- isPipelineRunning 各状态测试
- isPipelineError 各状态测试
- getPipelineError 各状态测试
- getPipelineStateLabel 中文标签测试
- 各接口字段验证

---

## 4 E2E 测试 (Python)

### 4.1 `test_e2e.py`

**依赖**: `opencv-python`, `websocket-client`

| 测试 | 描述 | 失败条件 |
|------|------|---------|
| `test_rtsp_stream` | 连接 RTSP 流，读取帧 10 秒 | FPS < 5 |
| `test_remote_control` | WS 连接 + 认证 + mouse_move + key_press | 命令响应非 ok |
| `test_tauri_commands` | 标记为 SKIP (需运行时) | — |

**用法**: `python tests/test_e2e.py --host 127.0.0.1 --rtsp-port 8554 --ws-port 9001`

### 4.2 `test_runtime.py`

**7 项测试**:

| # | 测试 | 描述 |
|---|------|------|
| 1 | Prerequisites | 检查 RTSP/WS 端口是否开放 |
| 2 | RTSP Stream | 连接 + 帧率 + 分辨率检测 |
| 3 | Multiple Streams | 同时读取多路 RTSP 流 |
| 4 | Latency | 首帧延迟 + 帧间抖动 + 估算端到端延迟 |
| 5 | Remote Control | WS 连接 + 认证 + mouse_move/click/key_press + 无效命令拒绝 + 错误密码拒绝 |
| 6 | RTSP Auth | 验证 RTSP 无需认证 |
| 7 | RTSP Media Info | 发送 DESCRIBE 请求，检查 SDP 中的 H264 codec |

**用法**: `python tests/test_runtime.py --host 127.0.0.1 --rtsp-port 8554 --ws-port 9001`
