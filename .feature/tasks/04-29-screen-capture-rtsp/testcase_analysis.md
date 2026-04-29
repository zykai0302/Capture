# Requirements Analysis Report: 桌面抓屏软件 - 抓屏编码RTSP推流

## 1. Project Background

### Current Issues
- 无现成的跨平台桌面抓屏推流工具，现有方案（OBS/VLC）无法满足多路GPU硬件编码+RTSP服务端+远程反控的集成需求
- 需要低延迟（<500ms）的专业级屏幕推流方案
- 需要远程反控能力（远程桌面级操控）

### Project Goals
开发一款跨平台桌面抓屏软件，支持多路同时推流、GPU硬件编码（自动软编fallback）、RTSP推流、远程反控（鼠标+键盘全控制）

## 2. Requirements Overview

### Core Objective
用户选择画面源后，系统自动完成"抓取→编码→封装→发送"的完整链路，外部客户端可通过RTSP协议拉流观看，并可通过WebSocket反控发送端

### Applicable Scenarios
- 远程桌面监控与操控
- 多屏幕内容同时推流分发
- 会议室/教室屏幕内容广播
- 远程技术支持与协助

### Core Features

| Feature | Description |
|---------|-------------|
| 画面源枚举 | 枚举并展示所有可用的显示器和应用窗口 |
| 单路RTSP推流 | 选中一个画面源后，自动完成抓屏→GPU编码→RTSP推流 |
| 多路同时推流 | 多个画面源同时推独立的RTSP流 |
| GPU硬件编码 | H264/H265 GPU硬件编码，GPU不可用时自动fallback到CPU软编码 |
| 远程反控 | 通过WebSocket发送鼠标/键盘指令，控制发送端桌面 |
| 画面源热插拔 | 显示器接入/断开、窗口关闭时动态处理 |
| 编码参数配置 | 分辨率、帧率、码率、编码器、GOP等参数可在UI中配置 |

## 3. Functional Requirements

### 画面源管理
- **Interaction Logic**: 用户打开应用后，左侧面板显示当前可用的显示器和窗口列表；用户可切换"显示器"和"窗口"标签页查看不同类型的源
- **Operation Path**: 左侧面板 → 画面源列表
- **Expected Result**: 每个源显示名称、类型标识、分辨率信息、推流状态

### 推流控制
- **Interaction Logic**: 用户点击某个画面源上的"开始推流"按钮，系统启动推流Pipeline；用户点击"停止推流"按钮，系统停止推流
- **Operation Path**: 画面源卡片 → 开始/停止推流按钮
- **Expected Result**: 推流启动后，状态变为"推流中"，显示RTSP拉流地址；外部播放器可拉流观看

### 编码参数配置
- **Interaction Logic**: 用户在右侧配置面板中修改编码参数（编码器、分辨率、帧率、码率等），点击"应用"后参数生效
- **Operation Path**: 右侧面板 → 编码配置 → 修改参数 → 应用
- **Expected Result**: Pipeline短暂重建后以新参数继续推流

### 远程反控
- **Interaction Logic**: 用户在反控面板中启动远程控制服务，设置端口和密码；外部客户端通过WebSocket连接后可发送鼠标/键盘指令
- **Operation Path**: 右侧面板 → 反控设置 → 启动服务
- **Expected Result**: 远程控制服务启动后，客户端连接可操控发送端桌面

### 热插拔处理
- **Interaction Logic**: 当显示器断开或窗口关闭时，系统自动检测并处理
- **Operation Path**: 系统自动处理
- **Expected Result**: 被移除的源自动停止推流，源列表自动更新

## 4. Functional Specifications

| Configuration | Limit | Notes |
|---------------|-------|-------|
| RTSP端口 | 默认8554 | 可配置 |
| WebSocket端口 | 默认9001 | 可配置 |
| 编码器 | H264/H265 | H264为默认 |
| 编码模式 | GPU优先/CPU/自动 | 自动为默认 |
| 分辨率 | 原始/自定义 | 原始为默认 |
| 帧率 | 10-60 fps | 30为默认 |
| 码率 | 500-20000 kbps | 4000为默认 |
| 码率控制 | CBR/VBR | VBR为默认 |
| GOP大小 | 10-120 | 30为默认 |
| 编码预设 | 速度/均衡/质量 | 均衡为默认 |
| 最大RTSP客户端数 | 默认10 | 可配置 |
| 反控密码 | 可选 | 空密码表示无认证 |
| 端到端延迟 | <500ms | 目标值 |

## 5. Business Design

### 核心业务流程1：单路推流
1. 用户打开应用 → 系统枚举画面源
2. 用户选择一个显示器/窗口 → 点击"开始推流"
3. 系统启动Pipeline：抓屏→GPU编码→RTSP推流
4. 系统显示RTSP拉流地址
5. 外部播放器输入RTSP地址 → 拉流观看实时画面
6. 用户点击"停止推流" → Pipeline停止

### 核心业务流程2：多路推流
1. 用户对多个画面源分别点击"开始推流"
2. 每个源独立运行Pipeline，分配独立RTSP路径
3. 外部播放器可分别拉流观看不同源

### 核心业务流程3：远程反控
1. 用户在反控面板启动远程控制服务
2. 远程客户端通过WebSocket连接并认证
3. 客户端发送鼠标/键盘指令
4. 发送端执行事件注入，远程操控桌面

### 核心业务流程4：热插拔
1. 应用运行中，外部显示器断开
2. 系统检测到源移除，自动停止对应推流Pipeline
3. 源列表更新，移除已断开的源
4. 显示器重新连接后，源列表自动更新，新源出现

## 6. Human-Interface Changes

- 新应用：三栏布局桌面应用
  - 左栏：画面源列表（显示器/窗口Tab切换）
  - 中栏：主预览区 + Pipeline流程可视化 + 状态HUD
  - 右栏：配置面板（编码配置/反控设置/网络配置Tab切换）
  - 顶栏：Logo + GPU信息 + RTSP端口显示 + 全局操作
  - 底栏：系统状态信息
- 画面源卡片：缩略图 + 类型徽章 + 分辨率 + 推流状态 + 操作按钮
- 编码配置面板：编码器选择 + 分辨率 + 帧率 + 码率 + GOP + 预设
- 反控面板：启停按钮 + 端口/密码配置 + 连接状态 + 客户端数

## 7. Technical Requirements

- 端到端延迟 < 500ms
- 单路推流帧率 ≥ 15fps（30fps目标）
- 多路推流同时运行不崩溃
- GPU编码不可用时自动fallback到CPU软编码
- 热插拔事件检测延迟 < 5秒
- RTSP服务端支持多客户端同时拉流
- WebSocket反控指令响应时间 < 100ms

## 8. Product Type

**Product Type**: 屏幕控制类（PK）
**Checklist**: testcase_checklist_PK.md

## 9. Risks and Constraints

- GPU驱动兼容性：不同厂商GPU的编码器可用性不同，需要自动检测和fallback
- UAC/安全桌面/DRM内容：无法抓取受保护的内容，需要给出明确提示
- GStreamer运行时体积：约100MB，影响安装包大小
- 中文路径问题：Windows中文用户目录可能导致Rust编译崩溃，需重定向编译输出
- 反控安全性：WebSocket密码明文传输，不适合高安全场景
- 多Pipeline资源占用：多路同时推流时CPU/GPU/内存占用较高
