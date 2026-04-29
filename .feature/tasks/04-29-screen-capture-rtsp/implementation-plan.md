# ScreenCast Pro - Implementation Plan

**Goal:** 构建跨平台桌面抓屏推流软件，实现多路 GPU 编码 RTSP 推流 + 反控功能，Windows 优先

**Context:**
- task directory: `.feature/tasks/04-29-screen-capture-rtsp/`
- related PRD: `.feature/tasks/04-29-screen-capture-rtsp/prd.md`
- key findings: GStreamer 是唯一跨平台 GPU 全链路方案；gst-rtsp-server 原生支持多路 RTSP；Pipeline 架构天然匹配抓取→编码→封装→发送；反控通过独立 WebSocket + enigo 实现
- UI 设计稿: `design/ui-preview.html`

**Architecture:** 采用分层架构：Rust 后端通过 GStreamer Pipeline 管理抓屏/编码/推流，gst-rtsp-server 提供 RTSP 服务，独立 WebSocket 服务端处理反控信令，enigo 执行事件注入。Tauri 2.x 框架桥接 Rust 后端与 Web 前端，通过 Commands + Events 双向通信。PipelineManager 管理多路 Pipeline 实例的生命周期。

**Tech Stack:**
- Backend: Rust, GStreamer (gstreamer-rs), gst-rtsp-server, tokio, tokio-tungstenite, enigo, serde
- Frontend: Tauri 2.x, Vue 3, TypeScript, Vite
- Build: Cargo, npm/pnpm

**Execution Options:**
- with test cases: `/feature:write-testcase` -> `/feature:check-testcase` -> `/feature:executing-plans` or `/feature:subagent-work`
- sequential: `/feature:executing-plans`
- delegated: `/feature:subagent-work`

---

## Task 1: Environment Setup & GStreamer Installation

**Why**
- GStreamer 是核心依赖，必须先安装并验证环境可用，后续所有开发都依赖它

**Files**
- Create: `scripts/install-gstreamer.ps1` (Windows GStreamer 安装脚本)
- Modify: `Cargo.toml` (添加 gstreamer 依赖)
- Create: `src/gstreamer_check.rs` (环境检查工具)

**Steps**
- [x] **Step 1: 安装 GStreamer Windows 运行时和开发包** ✅ DONE

已安装 GStreamer 1.28.2 MSVC 64-bit：
- 安装路径: `D:\msvc_x86_64\`
- 环境变量已配置: `GSTREAMER_1_0_ROOT_MSVC_X86_64=D:\msvc_x86_64\`
- PATH 已追加: `D:\msvc_x86_64\bin`

设置环境变量：
```
GSTREAMER_1_0_ROOT_MSVC_X86_64=D:\msvc_x86_64\
PATH 追加: D:\msvc_x86_64\bin
```
> ⚠️ 实际安装路径为 D:\msvc_x86_64\，环境变量已配置

- [ ] **Step 2: 创建环境检查脚本 `scripts/check-environment.ps1`**

```powershell
# 检查 GStreamer 安装
$gstRoot = $env:GSTREAMER_1_0_ROOT_MSVC_X86_64
if (-not $gstRoot) {
    Write-Host "GSTREAMER_1_0_ROOT_MSVC_X86_64 not set"
    Write-Host "Download from: https://gstreamer.freedesktop.org/download/"
    exit 1
}
# 验证 gst-launch-1.0 可用
& "$gstRoot\bin\gst-launch-1.0.exe" --version
# 验证关键插件
& "$gstRoot\bin\gst-inspect-1.0.exe" d3d11screencapturesrc
& "$gstRoot\bin\gst-inspect-1.0.exe" amfh264enc
& "$gstRoot\bin\gst-inspect-1.0.exe" rtph264pay
```

- [ ] **Step 3: 用 gst-launch-1.0 验证最小 Pipeline**

运行命令验证 DXGI 抓屏 + AMF 编码 + 显示 链路（AMD GPU）：
```
gst-launch-1.0 d3d11screencapturesrc monitor-index=0 ! amfh264enc bitrate=2000 ! avdec_h264 ! autovideosink
```
如果 AMF 不可用，测试软件编码：
```
gst-launch-1.0 d3d11screencapturesrc monitor-index=0 ! videoconvert ! x264enc bitrate=2000 ! autovideosink
```

**Validation**
- Run: `gst-inspect-1.0 d3d11screencapturesrc` ✅ VERIFIED
- Expect: 输出插件详情，无 "No such element" 错误
- Run: `gst-launch-1.0 d3d11screencapturesrc monitor-index=0 ! fakesink`
- Expect: Pipeline 正常运行，无错误输出

**Risks / Notes**
- GStreamer MSVC 版本必须与 Rust MSVC toolchain 匹配（都是 x86_64-msvc） ✅
- 当前 GPU 为 AMD RX 6750 GRE，使用 AMF 编码器（amfh264enc/amfh265enc），不支持 NVIDIA nvenc
- GStreamer 版本 1.28.2 ✅

---

## Task 2: Initialize Tauri 2.x Project

**Why**
- 建立项目骨架，包含 Tauri 框架 + Vue 3 前端 + Rust 后端的基础结构

**Files**
- Create: 整个 Tauri 项目结构（通过 `create-tauri-app` 生成）
- Modify: `Cargo.toml` (Rust 依赖)
- Modify: `package.json` (前端依赖)
- Modify: `tauri.conf.json` (Tauri 配置)
- Create: `src-tauri/src/main.rs` (Rust 入口)
- Create: `src/` (Vue 前端源码目录)

**Steps**
- [x] **Step 1: 用 create-tauri-app 初始化项目** ✅ DONE

- [x] **Step 2: 配置 Rust 依赖 `src-tauri/Cargo.toml`** ✅ DONE

- [x] **Step 3: 配置前端依赖 `package.json`** ✅ DONE

- [x] **Step 4: 配置 `tauri.conf.json`** ✅ DONE

- [x] **Step 5: 验证项目可构建** ✅ DONE
  - cargo check 成功 (需设置 GStreamer 环境变量 + CARGO_TARGET_DIR 避免中文路径崩溃)

**Validation**
- Run: `cargo check` ✅ PASSED (EXIT_CODE: 0)
- Expect: 编译通过，无错误

**Risks / Notes**
- GStreamer 链接可能需要在 `.cargo/config.toml` 中指定库搜索路径
- Windows MSVC 工具链必须正确配置（Visual Studio Build Tools）

---

## Task 3: Rust Backend Module Skeleton

**Why**
- 建立后端模块骨架，定义核心 trait 和数据结构，为后续实现提供清晰的接口边界

**Files**
- Create: `src-tauri/src/capture/mod.rs` (抓屏模块)
- Create: `src-tauri/src/capture/source.rs` (画面源枚举)
- Create: `src-tauri/src/encode/mod.rs` (编码模块)
- Create: `src-tauri/src/encode/config.rs` (编码配置)
- Create: `src-tauri/src/rtsp/mod.rs` (RTSP 服务模块)
- Create: `src-tauri/src/remote/mod.rs` (反控模块 + 信令协议)
- Create: `src-tauri/src/pipeline/mod.rs` (Pipeline 管理器 trait)
- Create: `src-tauri/src/config/mod.rs` (全局配置)
- Create: `src-tauri/src/error.rs` (统一错误类型)
- Modify: `src-tauri/src/main.rs` (GStreamer 初始化)
- Modify: `src-tauri/src/lib.rs` (模块声明 + AppState + Tauri 命令注册)

**Steps**
- [x] **Step 1: 定义统一错误类型 `src-tauri/src/error.rs`** ✅ DONE
- [x] **Step 2: 定义画面源数据结构 `src-tauri/src/capture/source.rs`** ✅ DONE
- [x] **Step 3: 定义编码配置 `src-tauri/src/encode/config.rs`** ✅ DONE
- [x] **Step 4: 定义 Pipeline 管理器接口 `src-tauri/src/pipeline/mod.rs`** ✅ DONE
- [x] **Step 5: 定义反控信令协议 `src-tauri/src/remote/mod.rs`** ✅ DONE
- [x] **Step 6: 定义全局配置 `src-tauri/src/config/mod.rs`** ✅ DONE
- [x] **Step 7: 修改 `src-tauri/src/main.rs` 和 `src-tauri/src/lib.rs`** ✅ DONE
  - main.rs: GStreamer 初始化 + 调用 lib::run()
  - lib.rs: 所有模块声明 + AppState + greet/get_config 命令
- [x] **Step 8: 运行 `cargo check` 验证编译** ✅ DONE

**Validation**
- Run: `cargo check` ✅ PASSED
- Expect: 所有模块骨架编译通过，无错误

**Risks / Notes**
- `gstreamer::init()` 在 main.rs 中调用，在任何 GStreamer 操作之前
- `AppError` 实现了 `Serialize` 以支持 Tauri Command 返回
- 此步骤只创建接口和类型定义，不实现具体逻辑
- lib.rs 是 Tauri 的 crate root，所有模块必须在 lib.rs 中声明
- 使用 `#[allow(dead_code)]` 抑制骨架阶段的未使用警告

---

## Task 4: Implement Screen Capture Source Enumeration

**Why**
- 用户需要看到所有可用的画面源（显示器 + 窗口），这是选择推流源的前提

**Files**
- Create: `src-tauri/src/capture/platform/mod.rs` (平台分发)
- Create: `src-tauri/src/capture/platform/windows.rs` (Windows 平台枚举)
- Modify: `src-tauri/src/capture/mod.rs` (引入 platform 模块)
- Modify: `src-tauri/src/Cargo.toml` (添加 windows crate 依赖)
- Modify: `src-tauri/src/lib.rs` (注册 list_sources 命令)

**Steps**
- [x] **Step 1: 实现 Windows 平台显示器枚举** ✅ DONE
- [x] **Step 2: 实现平台分发 `src-tauri/src/capture/platform/mod.rs`** ✅ DONE
- [x] **Step 3: 注册 Tauri Command `list_sources`** ✅ DONE
- [x] **Step 4: 验证枚举功能** ✅ DONE (cargo check 通过)

**Validation**
- Run: `cargo check` ✅ PASSED
- Expect: 编译通过

---

## Task 5: Implement Single Pipeline (DXGI + AMF + RTSP)

**Why**
- 核心链路验证：单路 抓屏→编码→RTSP推流 跑通后，多路只需复制 Pipeline 实例

**Files**
- Create: `src-tauri/src/pipeline/gst_pipeline.rs` (GStreamer launch string 构建)
- Create: `src-tauri/src/pipeline/manager.rs` (GstPipelineManager 实现)
- Create: `src-tauri/src/rtsp/server.rs` (gst-rtsp-server 封装)
- Modify: `src-tauri/src/pipeline/mod.rs` (PipelineManager trait + 导出)
- Modify: `src-tauri/src/rtsp/mod.rs` (导出 RtspServer)
- Modify: `src-tauri/src/lib.rs` (注册 start/stop/status 命令)

**Steps**
- [x] **Step 1: 实现 GStreamer Pipeline 构建** ✅ DONE
  - 使用 gst-rtsp-server 的 launch string 模式（而非手动构建 Pipeline）
  - `build_launch_string()` 构建 d3d11screencapturesrc → d3d11download → d3d11convert → encoder → rtp_pay
- [x] **Step 2: 实现编码器自动选择逻辑** ✅ DONE
  - `gpu_encoder_candidates()` 返回 AMD/Intel GPU 编码器候选列表
  - `build_encoder_element()` 自动检测可用 GPU 编码器，fallback 到 CPU
- [x] **Step 3: 实现 gst-rtsp-server 封装** ✅ DONE
  - `RtspServer::new()` / `add_stream()` / `remove_stream()` / `start()`
  - 使用 `RTSPServer::new()` + `set_service()` (非 builder 模式)
- [x] **Step 4: 实现 GstPipelineManager** ✅ DONE
  - 管理 pipelines HashMap + rtsp_server
  - `ensure_rtsp_server()` 懒启动 RTSP 服务器
- [x] **Step 5: 注册 Tauri Commands** ✅ DONE
  - `start_stream`, `stop_stream`, `get_pipeline_status`, `get_available_encoders`
- [ ] **Step 6: 端到端验证** (需要运行时测试)

**Validation**
- Run: `cargo check` ✅ PASSED
- Run: 应用 + VLC 拉流 (pending runtime test)

核心 Pipeline 构建（Windows + NVIDIA）：
```rust
use gstreamer::prelude::*;

pub fn build_screen_capture_pipeline(
    source: &CaptureSource,
    config: &EncodeConfig,
    rtsp_path: &str,
) -> AppResult<gstreamer::Pipeline> {
    gstreamer::init()?;

    let pipeline = gstreamer::Pipeline::builder()
        .name(&format!("pipeline-{}", source.id))
        .build();

    // 1. 抓屏源
    let capture_src = match source.source_type {
        SourceType::Monitor => {
            let src = gstreamer::ElementFactory::make("d3d11screencapturesrc")
                .name("capture")
                .build()?;
            let monitor_idx: i32 = source.id.strip_prefix("screen-")
                .and_then(|s| s.parse().ok())
                .unwrap_or(-1);
            src.set_property("monitor-index", monitor_idx);
            src
        }
        SourceType::Window => {
            let src = gstreamer::ElementFactory::make("d3d11screencapturesrc")
                .name("capture")
                .build()?;
            // 窗口抓取使用 window-handle 属性
            let hwnd: u64 = source.id.strip_prefix("window-")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            src.set_property("window-handle", hwnd);
            src
        }
    };

    // 2. 编码器（自动选择 GPU/CPU）
    let encoder = create_encoder(config)?;

    // 3. RTP 封装
    let rtp_pay = match config.codec {
        Codec::H264 => gstreamer::ElementFactory::make("rtph264pay")
            .name("rtp-pay")
            .build()?,
        Codec::H265 => gstreamer::ElementFactory::make("rtph265pay")
            .name("rtp-pay")
            .build()?,
    };

    // 4. 连接元素
    pipeline.add_many([&capture_src, &encoder, &rtp_pay])?;
    capture_src.link(&encoder)?;
    encoder.link(&rtp_pay)?;

    Ok(pipeline)
}
```

- [ ] **Step 2: 实现编码器自动选择逻辑**

```rust
fn create_encoder(config: &EncodeConfig) -> AppResult<gstreamer::Element> {
    let codec_name = match config.codec {
        Codec::H264 => "h264",
        Codec::H265 => "h265",
    };

    match config.mode {
        EncodeMode::Auto | EncodeMode::GpuOnly => {
            // 尝试 GPU 编码器
            for encoder_name in gpu_encoder_candidates(codec_name) {
                if let Ok(elem) = gstreamer::ElementFactory::make(&encoder_name).build() {
                    log::info!("Using GPU encoder: {}", encoder_name);
                    return Ok(elem);
                }
            }
            if matches!(config.mode, EncodeMode::GpuOnly) {
                return Err(AppError::Encode("No GPU encoder available".into()));
            }
            // Fallback to CPU
            log::warn!("GPU encoder not available, falling back to CPU");
            create_cpu_encoder(config)
        }
        EncodeMode::CpuOnly => create_cpu_encoder(config),
    }
}

fn gpu_encoder_candidates(codec: &str) -> Vec<String> {
    match codec {
        "h264" => vec!["amfh264enc", "amfh264device2enc", "mfh264enc", "mfh264device3enc"],
        "h265" => vec!["amfh265enc", "amfh265device2enc"],
        _ => vec![],
    }.into_iter().map(String::from).collect()
}

fn create_cpu_encoder(config: &EncodeConfig) -> AppResult<gstreamer::Element> {
    let name = match config.codec {
        Codec::H264 => "openh264enc",
        Codec::H265 => "x265enc",
    };
    let encoder = gstreamer::ElementFactory::make(name)
        .name("cpu-encoder")
        .build()?;
    encoder.set_property("bitrate", config.bitrate_kbps);
    Ok(encoder)
}
```

- [ ] **Step 3: 实现 gst-rtsp-server 封装 `src-tauri/src/rtsp/server.rs`**

```rust
use gstreamer_rtsp_server::prelude::*;

pub struct RtspServer {
    server: gstreamer_rtsp_server::RTSPServer,
    mount_points: gstreamer_rtsp_server::RTSPMountPoints,
    port: u16,
}

impl RtspServer {
    pub fn new(port: u16) -> AppResult<Self> {
        let server = gstreamer_rtsp_server::RTSPServer::builder()
            .service(&format!("{}", port))
            .build();
        let mount_points = server.mount_points()
            .ok_or(AppError::Rtsp("Failed to get mount points".into()))?;
        Ok(Self { server, mount_points, port })
    }

    pub fn add_stream(&self, path: &str, pipeline_launch_str: &str) -> AppResult<()> {
        let factory = gstreamer_rtsp_server::RTSPMediaFactory::builder()
            .launch(pipeline_launch_str)
            .build();
        self.mount_points.add_factory(path, factory);
        Ok(())
    }

    pub fn remove_stream(&self, path: &str) {
        self.mount_points.remove_factory(path);
    }

    pub fn start(&self) -> AppResult<()> {
        self.server.attach(None);
        log::info!("RTSP server started on port {}", self.port);
        Ok(())
    }

    pub fn rtsp_url(&self, path: &str) -> String {
        format!("rtsp://127.0.0.1:{}/{}", self.port, path)
    }
}
```

- [ ] **Step 4: 实现 PipelineManager `src-tauri/src/pipeline/mod.rs`**

```rust
use std::collections::HashMap;
use std::sync::Mutex;

pub struct GstPipelineManager {
    pipelines: Mutex<HashMap<String, PipelineHandle>>,
    rtsp_server: Mutex<Option<RtspServer>>,
    config: AppConfig,
}

struct PipelineHandle {
    pipeline: gstreamer::Pipeline,
    status: PipelineState,
    config: EncodeConfig,
    rtsp_path: String,
}

impl GstPipelineManager {
    pub fn new(config: AppConfig) -> Self {
        Self {
            pipelines: Mutex::new(HashMap::new()),
            rtsp_server: Mutex::new(None),
            config,
        }
    }

    pub fn start_rtsp_server(&self) -> AppResult<()> {
        let server = RtspServer::new(self.config.rtsp_port)?;
        server.start()?;
        *self.rtsp_server.lock().unwrap() = Some(server);
        Ok(())
    }
}

impl PipelineManager for GstPipelineManager {
    fn start_pipeline(&self, source: &CaptureSource, config: &EncodeConfig) -> AppResult<()> {
        let rtsp_path = format!("/{}", source.id);
        let pipeline = build_screen_capture_pipeline(source, config, &rtsp_path)?;

        // 构建 launch string 给 rtsp-server
        let launch_str = build_launch_string(source, config);
        if let Some(server) = self.rtsp_server.lock().unwrap().as_ref() {
            server.add_stream(&rtsp_path, &launch_str)?;
        }

        pipeline.set_state(gstreamer::State::Playing)?;

        let handle = PipelineHandle {
            pipeline,
            status: PipelineState::Running,
            config: config.clone(),
            rtsp_path,
        };
        self.pipelines.lock().unwrap().insert(source.id.clone(), handle);
        Ok(())
    }

    fn stop_pipeline(&self, source_id: &str) -> AppResult<()> {
        if let Some(handle) = self.pipelines.lock().unwrap().remove(source_id) {
            handle.pipeline.set_state(gstreamer::State::Null)?;
            if let Some(server) = self.rtsp_server.lock().unwrap().as_ref() {
                server.remove_stream(&handle.rtsp_path);
            }
        }
        Ok(())
    }

    fn stop_all(&self) -> AppResult<()> {
        let ids: Vec<String> = self.pipelines.lock().unwrap().keys().cloned().collect();
        for id in ids {
            self.stop_pipeline(&id)?;
        }
        Ok(())
    }

    fn get_status(&self, source_id: &str) -> Option<PipelineStatus> { /* ... */ }
    fn get_all_status(&self) -> Vec<PipelineStatus> { /* ... */ }
    fn update_config(&self, source_id: &str, config: &EncodeConfig) -> AppResult<()> { /* ... */ }
    fn get_rtsp_url(&self, source_id: &str) -> Option<String> { /* ... */ }
}
```

- [ ] **Step 5: 注册 Tauri Commands**

```rust
#[tauri::command]
fn start_stream(source_id: String, config: EncodeConfig, state: State<AppState>) -> Result<(), AppError> {
    let sources = capture::platform::enumerate_sources()?;
    let source = sources.find_by_id(&source_id)
        .ok_or(AppError::Capture("Source not found".into()))?;
    state.pipeline_manager.start_pipeline(&source, &config)
}

#[tauri::command]
fn stop_stream(source_id: String, state: State<AppState>) -> Result<(), AppError> {
    state.pipeline_manager.stop_pipeline(&source_id)
}

#[tauri::command]
fn get_pipeline_status(state: State<AppState>) -> Result<Vec<PipelineStatus>, AppError> {
    Ok(state.pipeline_manager.get_all_status())
}
```

- [ ] **Step 6: 端到端验证**

1. 启动应用
2. 调用 `list_sources` 获取画面源列表
3. 调用 `start_stream` 启动主屏推流
4. 在 VLC 中打开 `rtsp://127.0.0.1:8554/screen-0`
5. 验证能看到桌面画面

**Validation**
- Run: `cargo build`
- Expect: 编译通过
- Run: 应用 + VLC 拉流
- Expect: VLC 显示实时桌面画面，延迟 <500ms

**Risks / Notes**
- `gst-rtsp-server` 的 `RTSPMediaFactory` 使用 launch string 创建 Pipeline，需要确保 launch string 与手动构建的 Pipeline 行为一致
- Pipeline 状态变更需要处理异步回调（bus message）
- RTSP 服务需要在 GLib main loop 中运行，需与 Tauri 的 tokio runtime 协调
- 如果 NVIDIA GPU 不可用，需确认 `x264enc` fallback 正常工作

---

## Task 6: Implement GPU/CPU Encoder Auto-Fallback

**Why**
- P1 需求：GPU 不可用时必须自动切换到 CPU 软编码，保证推流不中断

**Files**
- Modify: `src-tauri/src/pipeline/gst_pipeline.rs` (增强 encoder 选择逻辑)
- Create: `src-tauri/src/encode/detector.rs` (GPU 能力检测)
- Modify: `src-tauri/src/pipeline/mod.rs` (Pipeline 错误处理和 fallback)

**Steps**
- [ ] **Step 1: 实现 GPU 能力检测 `src-tauri/src/encode/detector.rs`**

```rust
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

    for name in &["amfh264enc", "amfh265enc"] {
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
```

- [ ] **Step 2: 实现 Pipeline 运行时错误监听和自动 fallback**

监听 GStreamer bus message，当编码器报错时自动重建 Pipeline：

```rust
fn setup_bus_watch(pipeline: &gstreamer::Pipeline, fallback_tx: tokio::sync::mpsc::Sender<String>) {
    let bus = pipeline.bus().expect("Pipeline has no bus");
    bus.add_watch(move |_bus, msg| {
        match msg.view() {
            gstreamer::MessageView::Error(err) => {
                log::error!("Pipeline error: {} ({:?})", err.error(), err.debug());
                let _ = fallback_tx.try_send(msg.src()
                    .map(|s| s.name().to_string())
                    .unwrap_or_default());
            }
            gstreamer::MessageView::StateChanged(s) => {
                // 记录状态变化
            }
            _ => {}
        }
        glib::Continue(true)
    }).expect("Failed to add bus watch");
}
```

- [ ] **Step 3: 验证 fallback 流程**

1. 在有 NVIDIA GPU 的机器上启动 GPU 编码推流
2. 通过任务管理器结束 NVIDIA 显示驱动（模拟 GPU 不可用）
3. 观察是否自动切换到 CPU 编码

**Validation**
- Run: 手动测试 GPU→CPU fallback
- Expect: Pipeline 自动重建，VLC 拉流短暂中断后恢复

**Risks / Notes**
- Pipeline 重建会导致短暂断流（约 1-2 秒）
- fallback 后需要通知前端更新 PipelineStatus（encoder_used 从 nvh264enc 变为 x264enc）

---

## Task 7: Implement Hot-Plug Detection for Sources

**Why**
- P1 需求：显示器接入/断开、窗口关闭时需要动态处理

**Files**
- Create: `src-tauri/src/capture/hotplug.rs` (热插拔监听)
- Modify: `src-tauri/src/capture/mod.rs` (注册热插拔回调)
- Modify: `src-tauri/src/lib.rs` (注册 Tauri Event)

**Steps**
- [ ] **Step 1: 实现显示器热插拔监听 `src-tauri/src/capture/hotplug.rs`**

使用 Windows `RegisterPowerSettingNotification` 或 `WM_DEVICECHANGE` 监听显示器变化：
```rust
use tauri::AppHandle;

pub fn start_hotplug_monitor(app: AppHandle) {
    std::thread::spawn(move || {
        // 方案 1: 定时轮询（简单可靠）
        let mut last_sources = enumerate_sources().unwrap();
        loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            let current_sources = match enumerate_sources() {
                Ok(s) => s,
                Err(_) => continue,
            };

            // 检测新增源
            for source in &current_sources.all() {
                if !last_sources.contains_id(&source.id) {
                    app.emit("source-added", source).ok();
                }
            }

            // 检测移除源
            for source in &last_sources.all() {
                if !current_sources.contains_id(&source.id) {
                    app.emit("source-removed", source).ok();
                }
            }

            last_sources = current_sources;
        }
    });
}
```

- [ ] **Step 2: 处理源移除时自动停止 Pipeline**

当 `source-removed` 事件触发时，如果该源正在推流，自动停止：
```rust
app.listen("source-removed", |event| {
    let source: CaptureSource = serde_json::from_str(event.payload()).unwrap();
    if source.is_streaming {
        pipeline_manager.stop_pipeline(&source.id).ok();
        app.emit("pipeline-stopped", &source.id).ok();
    }
});
```

- [ ] **Step 3: 前端监听热插拔事件**

```typescript
import { listen } from '@tauri-apps/api/event';

listen('source-added', (event) => {
    // 更新源列表
});

listen('source-removed', (event) => {
    // 从源列表移除，如果正在推流则显示断开提示
});
```

**Validation**
- Run: 应用运行中，拔插外接显示器
- Expect: 源列表自动更新，正在推流的被移除源自动停止

**Risks / Notes**
- 轮询方式有 2 秒延迟，对实时性要求高的场景可改用 Windows 事件监听
- 窗口关闭检测通过轮询 EnumWindows 实现，窗口数量可能较多需优化

---

## Task 8: Implement Remote Control WebSocket Server

**Why**
- P2 需求：反控功能需要 WebSocket 服务端接收解码端指令

**Files**
- Create: `src-tauri/src/remote/websocket.rs` (WebSocket 服务端实现)
- Create: `src-tauri/src/remote/injector.rs` (事件注入实现)
- Modify: `src-tauri/src/remote/mod.rs` (整合反控模块)
- Modify: `src-tauri/src/lib.rs` (注册启动/停止反控命令)

**Steps**
- [ ] **Step 1: 实现 WebSocket 服务端 `src-tauri/src/remote/websocket.rs`**

```rust
use tokio_tungstenite::accept_async;
use tokio::net::TcpListener;

pub struct RemoteControlServer {
    port: u16,
    password: String,
    clients: Arc<Mutex<Vec<WebSocketClient>>>,
    injector: Arc<RemoteInjector>,
}

impl RemoteControlServer {
    pub async fn start(&self) -> AppResult<()> {
        let listener = TcpListener::bind(&format!("0.0.0.0:{}", self.port)).await?;
        log::info!("Remote control WebSocket server listening on port {}", self.port);

        loop {
            let (stream, addr) = listener.accept().await?;
            let ws = accept_async(stream).await?;
            let password = self.password.clone();
            let injector = self.injector.clone();

            tokio::spawn(async move {
                handle_client(ws, addr, &password, &injector).await;
            });
        }
    }
}

async fn handle_client(
    ws: WebSocketStream<TcpStream>,
    addr: SocketAddr,
    password: &str,
    injector: &Arc<RemoteInjector>,
) {
    // 1. 等待认证消息
    // 2. 认证通过后进入命令处理循环
    // 3. 收到命令 -> 解析 -> 注入事件 -> 返回 ack
    while let Some(msg) = ws.next().await {
        let text = msg?.to_text()?;
        let cmd: RemoteCommand = serde_json::from_str(text)?;
        injector.execute(&cmd)?;
        ws.send(Message::text(r#"{"status":"ok"}"#)).await?;
    }
}
```

- [ ] **Step 2: 实现事件注入 `src-tauri/src/remote/injector.rs`**

```rust
use enigo::{Enigo, Key, Keyboard, Mouse, MouseButton, MouseCursor};

pub struct RemoteInjector {
    enigo: Mutex<Enigo>,
}

impl RemoteInjector {
    pub fn new() -> AppResult<Self> {
        Ok(Self {
            enigo: Mutex::new(Enigo::new(&Default::default())?),
        })
    }

    pub fn execute(&self, cmd: &RemoteCommand) -> AppResult<()> {
        let mut enigo = self.enigo.lock().unwrap();
        match cmd {
            RemoteCommand::MouseMove { data, .. } => {
                enigo.move_mouse(data.x, data.y, enigo::Coordinate::Abs)?;
            }
            RemoteCommand::MouseClick { data, .. } => {
                let button = match data.button {
                    MouseButton::Left => enigo::MouseButton::Left,
                    MouseButton::Right => enigo::MouseButton::Right,
                    MouseButton::Middle => enigo::MouseButton::Middle,
                };
                match data.action {
                    ClickAction::Single => enigo.click(button)?,
                    ClickAction::Double => {
                        enigo.click(button)?;
                        enigo.click(button)?;
                    }
                }
            }
            RemoteCommand::MouseScroll { data, .. } => {
                enigo.scroll(data.y, enigo::ScrollDirection::Vertical)?;
            }
            RemoteCommand::KeyPress { data, .. } => {
                // 处理 modifier + key
                for mod_key in &data.modifiers {
                    enigo.key_down(parse_key(mod_key))?;
                }
                enigo.key_down(parse_key(&data.key))?;
                enigo.key_up(parse_key(&data.key))?;
                for mod_key in data.modifiers.iter().rev() {
                    enigo.key_up(parse_key(mod_key))?;
                }
            }
            RemoteCommand::KeyCombo { data, .. } => {
                for key in &data.keys {
                    enigo.key_down(parse_key(key))?;
                }
                for key in data.keys.iter().rev() {
                    enigo.key_up(parse_key(key))?;
                }
            }
            RemoteCommand::MouseDrag { data, .. } => {
                let button = match data.button {
                    MouseButton::Left => enigo::MouseButton::Left,
                    _ => enigo::MouseButton::Left,
                };
                enigo.move_mouse(data.from_x, data.from_y, enigo::Coordinate::Abs)?;
                enigo.button(button, enigo::Direction::Press)?;
                enigo.move_mouse(data.to_x, data.to_y, enigo::Coordinate::Abs)?;
                enigo.button(button, enigo::Direction::Release)?;
            }
        }
        Ok(())
    }
}
```

- [ ] **Step 3: 注册 Tauri Commands**

```rust
#[tauri::command]
fn start_remote_control(port: u16, password: String, state: State<AppState>) -> Result<(), AppError> {
    // 在 tokio runtime 中启动 WebSocket 服务
}

#[tauri::command]
fn stop_remote_control(state: State<AppState>) -> Result<(), AppError> {
    // 停止 WebSocket 服务
}

#[tauri::command]
fn get_remote_status(state: State<AppState>) -> Result<RemoteStatus, AppError> {
    // 返回连接客户端数、各能力启用状态
}
```

- [ ] **Step 4: 端到端验证**

1. 启动应用
2. 使用 `wscat` 或自定义 WebSocket 客户端连接 `ws://127.0.0.1:9001`
3. 发送 `{"type":"mouse_move","stream_id":"screen-0","data":{"x":500,"y":300}}`
4. 观察鼠标是否移动到指定位置

**Validation**
- Run: WebSocket 客户端发送鼠标移动命令
- Expect: 鼠标指针移动到指定坐标
- Run: WebSocket 客户端发送键盘输入命令
- Expect: 目标窗口接收到键盘输入

**Risks / Notes**
- `enigo` 在 Windows 上使用 SendInput API，需要窗口焦点
- 鼠标坐标是屏幕绝对坐标，需要与解码端画面尺寸做映射
- 反控密码认证需要防止明文传输（可考虑首次认证后使用 token）

---

## Task 9: Implement Vue 3 Frontend UI

**Why**
- 用户交互界面，基于 UI 设计稿实现三栏布局和所有交互功能

**Files**
- Create: `src/App.vue` (主布局)
- Create: `src/components/SourceList.vue` (左栏 - 画面源列表)
- Create: `src/components/SourceItem.vue` (画面源卡片)
- Create: `src/components/MainPreview.vue` (中栏 - 主预览)
- Create: `src/components/PipelineVisual.vue` (Pipeline 状态可视化)
- Create: `src/components/ConfigPanel.vue` (右栏 - 配置面板)
- Create: `src/components/EncodingConfig.vue` (编码配置)
- Create: `src/components/RemoteControl.vue` (反控设置)
- Create: `src/components/NetworkConfig.vue` (网络配置)
- Create: `src/components/TopBar.vue` (顶栏)
- Create: `src/components/StatusBar.vue` (底栏)
- Create: `src/composables/useSources.ts` (画面源 composable)
- Create: `src/composables/usePipeline.ts` (Pipeline 状态 composable)
- Create: `src/composables/useRemoteControl.ts` (反控 composable)
- Create: `src/styles/variables.css` (设计 Token / CSS 变量)
- Create: `src/styles/global.css` (全局样式)
- Modify: `src/main.ts` (Vue 入口)

**Steps**
- [x] **Step 1: 建立设计 Token `src/styles/variables.css`** ✅ DONE

从 `design/ui-preview.html` 提取所有 CSS 变量：
```css
:root {
  --bg-deep: #0a0e17;
  --bg-primary: #0f1523;
  --bg-secondary: #151d2e;
  --border: #1e2d45;
  --accent-cyan: #00d4ff;
  --accent-green: #00ff88;
  --accent-red: #ff3860;
  --font-display: 'Outfit', sans-serif;
  --font-mono: 'JetBrains Mono', monospace;
  /* ... 所有设计 token */
}
```

- [x] **Step 2: 实现主布局 `src/App.vue`** ✅ DONE

```vue
<template>
  <div class="app">
    <TopBar />
    <SourceList />
    <MainPreview />
    <ConfigPanel />
    <StatusBar />
  </div>
</template>
```

- [x] **Step 3: 实现 `useSources` composable** ✅ DONE

```typescript
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export function useSources() {
  const monitors = ref<CaptureSource[]>([]);
  const windows = ref<CaptureSource[]>([]);

  async function refresh() {
    const list = await invoke<CaptureSourceList>('list_sources');
    monitors.value = list.monitors;
    windows.value = list.windows;
  }

  onMounted(async () => {
    await refresh();
    // 监听热插拔事件
    listen('source-added', () => refresh());
    listen('source-removed', () => refresh());
    // 定时刷新
    const interval = setInterval(refresh, 5000);
    onUnmounted(() => clearInterval(interval));
  });

  return { monitors, windows, refresh };
}
```

- [x] **Step 4: 实现 `usePipeline` composable** ✅ DONE

```typescript
export function usePipeline() {
  const pipelines = ref<Map<string, PipelineStatus>>(new Map());

  async function startStream(sourceId: string, config: EncodeConfig) {
    await invoke('start_stream', { sourceId, config });
  }

  async function stopStream(sourceId: string) {
    await invoke('stop_stream', { sourceId });
  }

  async function refreshStatus() {
    const statuses = await invoke<PipelineStatus[]>('get_pipeline_status');
    pipelines.value = new Map(statuses.map(s => [s.source_id, s]));
  }

  return { pipelines, startStream, stopStream, refreshStatus };
}
```

- [x] **Step 5: 逐个实现 UI 组件** ✅ DONE

按照 `design/ui-preview.html` 的设计逐个实现：
1. `TopBar.vue` - 顶栏（Logo + GPU信息 + RTSP 端口 + 设置按钮）
2. `SourceList.vue` - 左栏（Tab 切换 + 源卡片列表）
3. `SourceItem.vue` - 源卡片（预览缩略图 + 状态 + 操作按钮）
4. `PipelineVisual.vue` - Pipeline 流程可视化
5. `MainPreview.vue` - 主预览区 + HUD 叠加
6. `ConfigPanel.vue` - 右栏（Tab 切换）
7. `EncodingConfig.vue` - 编码参数配置表单
8. `RemoteControl.vue` - 反控状态面板
9. `NetworkConfig.vue` - 网络/RTSP 配置
10. `StatusBar.vue` - 底部状态栏

- [x] **Step 6: 集成验证** ✅ DONE (vue-tsc + vite build 通过)

1. `npm run dev` 启动应用
2. 验证 UI 三栏布局正确显示
3. 验证画面源列表正确加载
4. 验证点击"开始推流"按钮后 Pipeline 启动
5. 验证 VLC 拉流播放
6. 验证编码参数修改后生效

**Validation**
- Run: `npm run dev`
- Expect: UI 完整显示，所有交互功能正常
- Run: VLC 拉流
- Expect: 实时画面正常

**Risks / Notes**
- Tauri webview 中字体需要从本地加载或内联（离线环境无法加载 Google Fonts）
- Pipeline 状态刷新频率需要控制，避免频繁 invoke 调用影响性能
- 实时预览缩略图需要通过 GStreamer 采样或 AppSink 获取帧数据

---

## Task 10: Runtime Encode Config Updates

**Why**
- P1 需求：编码参数需要在 UI 中可配并实时生效

**Files**
- Modify: `src-tauri/src/pipeline/mod.rs` (update_config 实现)
- Modify: `src/components/EncodingConfig.vue` (配置表单绑定)
- Modify: `src-tauri/src/lib.rs` (注册 update_encode_config 命令)

**Steps**
- [x] **Step 1: 实现 Pipeline 运行时配置更新** ✅ DONE

配置更新策略：停止当前 Pipeline → 用新配置重建 → 重新启动

```rust
impl PipelineManager for GstPipelineManager {
    fn update_config(&self, source_id: &str, config: &EncodeConfig) -> AppResult<()> {
        // 获取当前源信息
        let handle = self.pipelines.lock().unwrap().get(source_id).cloned();
        if let Some(handle) = handle {
            // 1. 停止旧 Pipeline
            self.stop_pipeline(source_id)?;
            // 2. 重新获取源信息并启动新 Pipeline
            let sources = capture::platform::enumerate_sources()?;
            let source = sources.find_by_id(source_id)
                .ok_or(AppError::Capture("Source not found".into()))?;
            self.start_pipeline(&source, config)?;
        }
        Ok(())
    }
}
```

- [x] **Step 2: 注册 Command** ✅ DONE

```rust
#[tauri::command]
fn update_encode_config(source_id: String, config: EncodeConfig, state: State<AppState>) -> Result<(), AppError> {
    state.pipeline_manager.update_config(&source_id, &config)
}
```

- [x] **Step 3: 前端配置表单双向绑定** ✅ DONE

```typescript
const config = ref<EncodeConfig>(defaultEncodeConfig());

async function applyConfig() {
    await invoke('update_encode_config', {
        sourceId: selectedSourceId.value,
        config: config.value,
    });
}
```

**Validation**
- Run: 修改码率 4000 → 8000 并应用
- Expect: Pipeline 短暂重建后继续推流，VLC 拉流码率变化

**Risks / Notes**
- Pipeline 重建期间有 1-2 秒断流
- 可考虑使用 GStreamer 的 `set_property` 运行时修改部分参数（如 bitrate）而不重建

---

## Task 11: End-to-End Integration Test

**Why**
- 验证所有功能集成后端到端可用

**Files**
- Create: `tests/e2e_test.rs` (集成测试)
- Create: `tests/test_client.py` (Python 测试客户端)

**Steps**
- [ ] **Step 1: 编写 RTSP 拉流测试脚本**

```python
# tests/test_client.py
import cv2
import time

def test_rtsp_stream(url, duration=10):
    cap = cv2.VideoCapture(url)
    assert cap.isOpened(), f"Failed to open RTSP stream: {url}"

    start = time.time()
    frame_count = 0
    while time.time() - start < duration:
        ret, frame = cap.read()
        if ret:
            frame_count += 1
    cap.release()

    fps = frame_count / duration
    print(f"Stream {url}: {fps:.1f} fps over {duration}s")
    assert fps > 10, f"FPS too low: {fps}"
    return fps

if __name__ == '__main__':
    test_rtsp_stream("rtsp://127.0.0.1:8554/screen-0")
```

- [ ] **Step 2: 编写 WebSocket 反控测试脚本**

```python
import websocket
import json
import time

def test_remote_control():
    ws = websocket.create_connection("ws://127.0.0.1:9001")
    # 认证
    ws.send(json.dumps({"type": "auth", "password": ""}))
    resp = json.loads(ws.recv())
    assert resp.get("status") == "ok"

    # 鼠标移动
    ws.send(json.dumps({
        "type": "mouse_move",
        "stream_id": "screen-0",
        "data": {"x": 500, "y": 300}
    }))
    resp = json.loads(ws.recv())
    assert resp.get("status") == "ok"

    ws.close()

if __name__ == '__main__':
    test_remote_control()
```

- [ ] **Step 3: 运行完整验证流程**

1. 启动应用
2. 启动主屏推流
3. 运行 RTSP 拉流测试 → 验证帧率 >10fps
4. 运行 WebSocket 反控测试 → 验证命令执行成功
5. 启动第二路推流（扩展屏或窗口）
6. 同时拉两路流 → 验证多路推流
7. 断开/重连网络 → 验证自动重连
8. 拔插外接显示器 → 验证热插拔处理

**Validation**
- Run: `python tests/test_client.py`
- Expect: 所有测试通过
- Run: 手动验证 VLC 拉流延迟 <500ms
- Expect: 端到端延迟在可接受范围

**Risks / Notes**
- 测试需要实际硬件（显示器、GPU）
- RTSP 拉流测试依赖 OpenCV（`pip install opencv-python`）
- 延迟测量需要精确的时间同步方案

---

## Task 12: Windows Build & Package

**Why**
- 最终可交付的 Windows 安装包

**Files**
- Modify: `tauri.conf.json` (打包配置)
- Create: `scripts/build.ps1` (构建脚本)

**Steps**
- [ ] **Step 1: 配置 Tauri 打包**

`tauri.conf.json` 打包部分：
```json
{
  "bundle": {
    "active": true,
    "targets": ["msi", "nsis"],
    "icon": ["icons/icon.ico"],
    "resources": [],
    "externalBin": [],
    "windows": {
      "certificateThumbprint": null,
      "digestAlgorithm": "sha256",
      "timestampUrl": ""
    }
  }
}
```

- [ ] **Step 2: 处理 GStreamer DLL 分发**

GStreamer 运行时 DLL 需要随应用分发：
- 方案 A: 将 GStreamer bin 目录下所有 DLL 复制到应用根目录
- 方案 B: 安装时要求用户先安装 GStreamer Runtime
- 推荐方案 A，在构建脚本中自动化

```powershell
# scripts/build.ps1
$gstBin = "$env:GSTREAMER_1_0_ROOT_MSVC_X86_64\bin\*.*"
$targetDir = "src-tauri\target\release\"
Copy-Item $gstBin $targetDir -Force
```

- [ ] **Step 3: 构建并测试安装包**

```bash
npm run tauri build
```

**Validation**
- Run: `npm run tauri build`
- Expect: 生成 MSI 和 NSIS 安装包
- Run: 安装并启动应用
- Expect: 应用正常运行，所有功能可用

**Risks / Notes**
- GStreamer DLL 体积约 80-100MB，安装包会较大
- 需要确认所有 GStreamer 插件 DLL 都被包含
- NSIS 安装包可能需要管理员权限（用于 GStreamer DLL 注册）
