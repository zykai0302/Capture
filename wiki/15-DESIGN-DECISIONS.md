# 15 — 设计决策记录

## 1 为什么选择 GStreamer 而非 FFmpeg？

|| 维度 | GStreamer | FFmpeg |
||------|-----------|--------|
|| RTSP 服务 | `gst-rtsp-server` 原生嵌入式 RTSP 服务 | 需要额外启动 `ffserver`(已废弃) 或外部 RTSP 代理 |
|| Pipeline 构建 | 声明式 launch string，一行描述全链路 | 需要手动管理 `avformat_alloc_output_context2` 等大量 C API |
|| 硬件加速 | 通过 GStreamer 元素工厂自动发现 GPU 编码器 | 需要手动检测并初始化硬件上下文 |
|| Rust 生态 | `gstreamer-rs` 0.23 绑定成熟 | `ffmpeg-next` 绑定较薄，大量 unsafe |

**决策**: GStreamer + gst-rtsp-server

---

## 2 为什么使用 D3D12 屏幕捕获？

|| 方案 | 优点 | 缺点 |
||------|------|------|
|| `d3d12screencapturesrc` (GStreamer 元素) | 与 Pipeline 无缝集成，可在 RTSP 媒体线程中正常工作 | 需要安装 GStreamer bad 插件 |
|| `d3d11screencapturesrc` (GStreamer 元素) | 与 Pipeline 无缝集成，支持 D3D11 纹理直传 | **在 RTSP 媒体线程中 D3D11 设备初始化失败**（"capture object is not configured yet"） |
|| Desktop Duplication API (DXGI) 手动实现 | 更底层控制 | 需要手动管理 D3D11 设备/纹理，与 GStreamer 集成复杂 |
|| `gdiscreencapsrc` (GStreamer 元素) | 简单 | 已废弃，CPU 拷贝，性能差 |

**决策**: `d3d12screencapturesrc` — 在 RTSP 媒体线程中稳定工作，通过 `videoconvert` 自动处理格式转换

> **为什么 d3d11screencapturesrc 失败**: gst-rtsp-server 为每个客户端连接创建独立的媒体线程，D3D11 屏幕捕获在该线程中初始化 D3D11 设备时失败。d3d12screencapturesrc 使用 D3D12 API，不受此限制。

---

## 3 为什么 GPU 编码优先 + CPU 自动 Fallback？

|| 策略 | 问题 |
||------|------|
|| 仅 GPU | 无独显的设备无法工作 |
|| 仅 CPU | 浪费 GPU 算力，高分辨率帧率低 |
|| GPU 优先 + Fallback | 最大化利用硬件，保证兼容性 |

**决策**: `EncodeMode::Auto` = GPU 优先，不可用自动 fallback CPU

---

## 4 为什么 RTSP Server 使用独立 MainContext 而非全局默认？

gst-rtsp-server 依赖 GLib 事件循环处理 RTSP 客户端连接和媒体传输。关键约束是 RTSP Server、其 socket watches、以及 MainLoop 必须运行在**同一个** MainContext 上。

**方案对比**:

|| 方案 | 问题 |
||------|------|
|| 全局默认 MainContext | Tauri 主线程或其他库可能已占用默认 context，导致 context 不匹配 |
|| 独立 `MainContext::new()` + `with_thread_default` | 每个 RtspServer 拥有专属 context，显式传递给 server/loop/source，无冲突 |

**决策**: 每个 RtspServer 实例创建独立的 `MainContext`，通过 `with_thread_default` 在 RTSP 线程中运行。主线程通过 `mpsc::channel` 发送 add/remove stream 指令，RTSP 线程通过 GLib timeout source (100ms) 轮询处理。

> **之前方案**（已废弃）：`ensure_glib_main_loop()` 在全局默认 context 上启动共享 MainLoop → 存在 server.attach 和 MainLoop 运行在不同 context 的风险。

---

## 5 为什么热插拔使用轮询而非事件驱动？

|| 方案 | 可行性 |
||------|--------|
|| `WM_DEVICECHANGE` 消息 | 仅检测显示器配置变更，不检测窗口增减 |
|| `WMI` 事件 | 需要额外 COM 初始化，且不全 |
|| `SetWinEventHook` | 可检测窗口创建/销毁，但回调在同一线程，可能有死锁 |
|| **2 秒轮询** | 简单可靠，无死锁风险，10 秒超时保护 |

**决策**: 2 秒轮询 + 10 秒超时保护。虽非实时，但对屏幕采集场景延迟可接受。

---

## 6 为什么选择 Enigo 而非 Windows Hook？

|| 方案 | 优点 | 缺点 |
||------|------|------|
|| Windows Hook (`SetWindowsHookEx`) | 底层控制 | 仅 Windows，需要 DLL 注入，复杂 |
|| **Enigo** | 跨平台 (Windows/macOS/Linux) | 无法拦截输入，仅注入 |

**决策**: Enigo — 跨平台设计，macOS/Linux 可直接复用 `RemoteInjector`

---

## 7 为什么 Pipeline 配置更新需要重启？

GStreamer Pipeline 一旦进入 `PLAYING` 状态，无法修改编码器、码率、分辨率等参数。`gst_element_set_state(pipeline, NULL)` 后 Pipeline 结构被销毁，必须重新构建。

**决策**: `update_config()` = `stop_pipeline()` + `start_pipeline()`。短暂中断可接受。

---

## 8 为什么 WS 协议使用 `0.0.0.0` 而 RTSP URL 使用 `127.0.0.1`？

- WS 服务绑定 `0.0.0.0` — 允许局域网内其他设备连接进行远程控制
- RTSP URL 生成使用 `127.0.0.1` — 显示给用户的本地播放地址，外部可通过实际 IP 访问

**决策**: WS 监听 `0.0.0.0:9001`，RTSP URL 显示 `127.0.0.1:8554`

---

## 9 为什么 `AppError` 序列化为纯字符串而非 JSON 对象？

Tauri IPC 的错误处理机制：当 `#[tauri::command]` 返回 `Result<T, E>` 且 `E: Serialize` 时，错误被直接传到前端 `catch`。序列化为字符串可让前端直接 `String(e)` 显示，无需解析嵌套 JSON。

**决策**: `serializer.serialize_str(self.to_string().as_str())`

---

## 10 已知限制与未来规划

|| # | 限制 | 规划 |
||---|------|------|
|| 1 | ~~仅 Windows 平台实现~~ | ✅ 三平台已实现：Windows (Win32) / macOS (CoreGraphics) / Linux (xrandr+DRM) |
|| 2 | 配置不持久化 | 计划支持 JSON 文件存储 (AppConfig → ~/.screencast-pro/config.json) |
|| 3 | 预览为 SVG 占位符 | 计划集成 WebRTC 低延迟预览或嵌入 VLC 播放器 |
|| 4 | FPS / 延迟为占位数据 (0) | 计划从 GStreamer Pipeline 提取实时统计 (`GstQuery`) |
|| 5 | 热插拔轮询效率 | 评估 `SetWinEventHook` + 轮询混合方案 |
|| 6 | WS 协议无 TLS | 生产环境需 WSS 反向代理 (nginx) |
|| 7 | 无音频采集 | 计划添加 `audiotestsrc` / `wasapisrc` 音频源和 AAC 编码 |
|| 8 | `rtsp_max_clients` 声明但未强制 | 需要在 gst-rtsp-server 中实现会话数限制 |
|| 9 | CSP 为 null | 开发便利，生产需配置严格 CSP |

---

## 11 为什么使用 `#[cfg(target_os)]` 条件编译而非 trait 对象？

|| 方案 | 优点 | 缺点 |
||------|------|------|
|| Trait 对象 (`dyn PlatformCapture`) | 运行时多态，可测试 | 需要 Box/动态分发，增加抽象层 |
|| **`#[cfg(target_os)]`** | 零开销，编译时确定，无运行时分支 | 每个平台需单独编译测试 |

**决策**: `#[cfg(target_os)]` — 屏幕采集是平台核心差异，不存在运行时切换需求，编译时确定更安全高效。

---

## 12 为什么 WebSocket 认证需要双标志 (`authenticated` + `auth_message_received`)？

单 `bool` 无法区分"空密码自动认证"和"客户端已显式认证"。如果只用 `authenticated`，空密码模式下任何密码的 auth 消息都会走"已认证确认"分支，导致安全绕过。

**决策**: `authenticated = password.is_empty()` + `auth_message_received = false`，首次 auth 必须通过密码验证。

---

## 13 为什么 `build_encoder_element` 必须包含 RTP payloader？

gst-rtsp-server 要求 pipeline 中有 `name=pay0` 的 payloader 元素（`rtph264pay` / `rtph265pay`）。如果 `name=pay0` 被错误地附加到编码器而非 payloader，RTSP 客户端将无法接收媒体数据。

**决策**: `build_cpu_encoder` 和 `format_encoder_params` 都必须输出 `encoder_params ! rtpXxxpay` 完整链路。
