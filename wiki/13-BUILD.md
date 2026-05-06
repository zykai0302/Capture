# 13 — 构建与运行

## 1 环境要求

|| 依赖 | 版本 | 用途 | 平台 |
||------|------|------|------|
|| Node.js | ≥ 18 | 前端构建 | 全平台 |
|| npm | 随 Node.js | 包管理 | 全平台 |
|| Rust | stable (2021 edition) | 后端编译 | 全平台 |
|| GStreamer | 1.24+ | 媒体框架 | 全平台 |
|| Windows SDK | — | Win32 API | Windows |
|| Xcode CLT | — | CoreGraphics | macOS |
|| xrandr / libdrm | — | 显示器检测 | Linux |

## 2 环境变量

|| 变量 | 示例值 | 必需 | 说明 |
||------|--------|------|------|
|| `GSTREAMER_1_0_ROOT_MSVC_X86_64` | `D:\msvc_x86_64\` | 是 | GStreamer 安装根目录 |
|| `GST_PLUGIN_PATH` | `D:\msvc_x86_64\lib\gstreamer-1.0` | 是 | GStreamer 插件路径 |
|| `PKG_CONFIG_PATH` | `D:\msvc_x86_64\lib\pkgconfig` | 是 | pkg-config 搜索路径 |
|| `PKG_CONFIG` | `D:\msvc_x86_64\bin\pkg-config.exe` | 是 | pkg-config 可执行文件 |
|| `CARGO_TARGET_DIR` | `e:\screencast-build` | 否 | Cargo 构建输出目录 |
|| `RUST_BACKTRACE` | `1` | 否 | Rust 调用栈回溯 |
|| `RUST_LOG` | `info` | 否 | Rust 日志级别 |
|| `GST_DEBUG` | `rtspserver:2,rtspmedia:2` | 否 | GStreamer 调试级别 (main.rs 自动设置) |

**PATH 前置**: `D:\msvc_x86_64\bin` 必须添加到 PATH 最前面。

### 2a macOS 环境变量

|| 变量 | 示例值 | 说明 |
||------|--------|------|
|| `GSTREAMER_1_0_ROOT` | `/Library/Frameworks/GStreamer.framework/` | 官方安装器 |
|| — | `/opt/homebrew/` | Homebrew 安装 |

GStreamer 插件路径搜索顺序: Homebrew (`/opt/homebrew/lib/gstreamer-1.0`) → 官方安装器 (`$GSTREAMER_1_0_ROOT/lib/gstreamer-1.0`)

### 2b Linux 环境变量

|| 变量 | 示例值 | 说明 |
||------|--------|------|
|| `GST_PLUGIN_PATH` | `/usr/lib/x86_64-linux-gnu/gstreamer-1.0` | 标准路径 |

GStreamer 插件路径搜索顺序: `/usr/lib/x86_64-linux-gnu/gstreamer-1.0` → `/usr/lib/gstreamer-1.0` → `/usr/local/lib/gstreamer-1.0`

## 3 npm 脚本

|| 命令 | 作用 |
||------|------|
|| `npm run dev` | 启动 Vite 开发服务器 (端口 1420) |
|| `npm run build` | `vue-tsc --noEmit && vite build` (类型检查 + 构建) |
|| `npm run preview` | Vite 预览构建产物 |
|| `npm run tauri` | 透传 tauri CLI |
|| `npm run test` | `vitest run` (前端单元测试) |
|| `npm run test:watch` | `vitest` (监听模式) |

## 4 常用构建命令

### 4.1 开发模式 (HMR)

```bash
npm run tauri dev
```

**流程**: 
1. Vite 开发服务器在 `localhost:1420` 启动
2. Cargo 编译 Rust 后端
3. Tauri 窗口加载 Vite 开发服务器 URL
4. 前端修改 → Vite HMR 热更新
5. 后端修改 → Cargo 重编译 → 窗口重载

### 4.2 发布构建

```bash
npm run tauri build
```

**流程**:
1. `npm run build` → `vue-tsc --noEmit && vite build` → `dist/`
2. Cargo release 编译
3. 打包平台安装程序 (Windows: MSI/NSIS, macOS: dmg/app, Linux: deb/AppImage)

**产物位置**: `src-tauri/target/release/bundle/` (各平台子目录)

## 5 构建脚本

### 5.1 `scripts/build.ps1` (Windows PowerShell)

**参数**: `-GstRoot` (默认 `$env:GSTREAMER_1_0_ROOT_MSVC_X86_64`), `-OutputDir` (默认 `.\release-output`)

**流程**:
1. 验证 GStreamer 安装
2. 设置环境变量
3. `npm run tauri build`
4. 复制 GStreamer DLL 到 release 目录:
   - `{GstRoot}\bin\*.dll` → `src-tauri\target\release\`
   - `{GstRoot}\lib\gstreamer-1.0\*.dll` → `src-tauri\target\release\gstreamer-1.0\`
5. 报告构建产物 (MSI/EXE 大小)

### 5.2 `scripts/run_dev.bat`

设置环境变量 → 启动 `e:\screencast-build\debug\screencast-pro.exe`

### 5.3 `scripts/build_and_test.bat`

设置环境变量 → `npx tauri build --debug --no-bundle`

### 5.4 `scripts/build_debug.bat`

设置环境变量 → `npx tauri build --debug --no-bundle`

### 5.5 `scripts/cargo_build.bat`

设置环境变量 → `cargo build --manifest-path "e:\抓屏软件\src-tauri\Cargo.toml"`

### 5.6 `scripts/cargo_env.js` (Node.js)

```javascript
const env = {
  ...process.env,
  PATH: 'D:\\msvc_x86_64\\bin;' + process.env.PATH,
  GSTREAMER_1_0_ROOT_MSVC_X86_64: 'D:\\msvc_x86_64\\',
  PKG_CONFIG_PATH: 'D:\\msvc_x86_64\\lib\\pkgconfig',
  PKG_CONFIG: 'D:\\msvc_x86_64\\bin\\pkg-config.exe',
  CARGO_TARGET_DIR: 'e:\\screencast-build',
};
const cmd = process.argv.slice(2).join(' ');
execSync(cmd, { env, encoding: 'utf8', stdio: 'pipe' });
```

### 5.7 `scripts/run_tests.bat`

设置环境变量 → `cargo test --manifest-path "%~dp0..\src-tauri\Cargo.toml" %*`

### 5.8 `scripts/run_e2e.bat`

设置环境变量 (含 `RUST_LOG=info`) → 启动应用 → 等待 10 秒

### 5.9 `scripts/tauri_build_debug.bat`

设置环境变量 → `npx tauri build --debug --no-bundle`

## 6 Cargo 配置 (`src-tauri/.cargo/config.toml`)

```toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-L", "D:\\msvc_x86_64\\lib"]

[env]
GSTREAMER_1_0_ROOT_MSVC_X86_64 = "D:\\msvc_x86_64\\"
```

**作用**: 为 MSVC 工具链指定 GStreamer 库搜索路径，确保链接时找到 `gstreamer-1.0.lib` 等。

## 7 Tauri 权限 (`capabilities/default.json`)

```json
{
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "opener:default"
  ]
}
```

## 8 打包配置

|| 配置 | 值 |
||------|-----|
|| 打包格式 | `"all"` (Windows: MSI/NSIS, macOS: dmg, Linux: deb/AppImage) |
|| 图标 | `32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.icns`, `icon.ico` |
|| Windows 摘要算法 | `sha256` |
|| macOS 最低版本 | `10.15` |
|| Linux deb 依赖 | libgstreamer1.0-0, gstreamer1.0-plugins-*, gstreamer1.0-vaapi |
|| 窗口标题 | `"ScreenCast Pro"` |
|| 默认尺寸 | 1280×800 |
|| 最小尺寸 | 1024×600 |
|| CSP | null (关闭) |
