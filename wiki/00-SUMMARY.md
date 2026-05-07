# 00 — 项目元数据、目录结构、配置文件全文

## 1 项目元数据

| 字段 | 值 |
|------|-----|
| 项目名 | `screencast-pro` |
| 版本 | `0.1.0` |
| 产品名 | `ScreenCast Pro` |
| 标识符 | `com.screencast-pro.app` |
| Rust lib 名 | `screencast_pro_lib` |
| Rust crate type | `["staticlib", "cdylib", "rlib"]` |
| Rust edition | `2021` |
| 前端框架 | Vue 3.5+ (Composition API, `<script setup>`) |
| 后端框架 | Tauri 2 |
| 媒体框架 | GStreamer 0.23 (Rust bindings, glib 0.20) |
| 异步运行时 | Tokio (features = `["full"]`) |
| 目标平台 | Windows / macOS / Linux (三平台支持，#[cfg(target_os)] 条件编译) |
| 打包格式 | all (MSI/NSIS/dmg/deb/AppImage) |
| 日志文件 | `%TEMP%\screencast-pro.log` |
| 默认 RTSP 端口 | `8554` |
| 默认 WebSocket 端口 | `9001` |
| 默认 MJPEG 预览端口 | `8090` |

## 2 目录结构

```
项目根/
├── index.html
├── package.json
├── package-lock.json
├── tsconfig.json
├── tsconfig.node.json
├── vite.config.ts
├── vitest.config.ts
├── .gitignore
│
├── public/
│   ├── tauri.svg
│   └── vite.svg
│
├── design/
│   └── ui-preview.html
│
├── src/
│   ├── main.ts
│   ├── App.vue
│   ├── vite-env.d.ts
│   ├── assets/
│   │   └── vue.svg
│   ├── types/
│   │   └── index.ts
│   ├── composables/
│   │   ├── index.ts
│   │   ├── usePipeline.ts
│   │   ├── useSources.ts
│   │   ├── useConfig.ts
│   │   └── useRemoteControl.ts
│   ├── components/
│   │   ├── TopBar.vue
│   │   ├── SourceList.vue
│   │   ├── SourceItem.vue
│   │   ├── MainPreview.vue
│   │   ├── PipelineVisual.vue
│   │   ├── ConfigPanel.vue
│   │   ├── EncodingConfig.vue
│   │   ├── NetworkConfig.vue
│   │   ├── RemoteControl.vue
│   │   └── StatusBar.vue
│   ├── styles/
│   │   ├── variables.css
│   │   └── global.css
│   └── __tests__/
│       ├── composables.test.ts
│       └── types.test.ts
│
├── src-tauri/
│   ├── Cargo.toml
│   ├── build.rs
│   ├── tauri.conf.json
│   ├── .gitignore
│   ├── .cargo/
│   │   └── config.toml
│   ├── capabilities/
│   │   └── default.json
│   ├── icons/                      # 14个图标文件
│   ├── gen/                        # Tauri 自动生成
│   │   └── schemas/
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── error.rs
│       ├── config/
│       │   └── mod.rs
│       ├── capture/
│       │   ├── mod.rs
│       │   ├── source.rs
│       │   ├── hotplug.rs
│       │   ├── thumbnail.rs     → capture_thumbnail(), BitBlt/PrintWindow + JPEG
│       │   └── platform/
│       │       ├── mod.rs
│       │       ├── windows.rs
│       │       ├── macos.rs
│       │       └── linux.rs
│       ├── encode/
│       │   ├── mod.rs
│       │   ├── config.rs
│       │   └── detector.rs
│       ├── pipeline/
│       │   ├── mod.rs
│       │   ├── gst_pipeline.rs
│       │   ├── manager.rs
│       │   ├── preview.rs        → PreviewPipeline (capture → jpegenc → appsink)
│       │   └── mjpeg_server.rs   → MjpegServer (tokio HTTP MJPEG)
│       ├── rtsp/
│       │   ├── mod.rs
│       │   └── server.rs      → RtspServer (独立 MainContext + channel)
│       └── remote/
│           ├── mod.rs
│           ├── injector.rs
│           └── websocket.rs
│
├── scripts/
│   ├── build.ps1
│   ├── run_dev.bat
│   ├── build_and_test.bat
│   ├── build_debug.bat
│   ├── cargo_build.bat
│   ├── cargo_env.js
│   ├── run_e2e.bat
│   ├── run_tests.bat
│   └── tauri_build_debug.bat
│
└── tests/
    └── test_runtime.py
```

## 3 配置文件全文

### 3.1 `package.json`

```json
{
  "name": "screencast-pro",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vue-tsc --noEmit && vite build",
    "preview": "vite preview",
    "tauri": "tauri",
    "test": "vitest run",
    "test:watch": "vitest"
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-opener": "^2",
    "pathe": "^2.0.3",
    "vue": "^3.5.13"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2",
    "@vitejs/plugin-vue": "^5.2.1",
    "@vue/test-utils": "^2.4.9",
    "happy-dom": "^20.9.0",
    "typescript": "~5.6.2",
    "vite": "^6.0.3",
    "vitest": "^2.1.9",
    "vue-tsc": "^2.1.10"
  }
}
```

### 3.2 `tsconfig.json`

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "module": "ESNext",
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "preserve",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src/**/*.ts", "src/**/*.d.ts", "src/**/*.tsx", "src/**/*.vue"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

### 3.3 `tsconfig.node.json`

```json
{
  "compilerOptions": {
    "composite": true,
    "skipLibCheck": true,
    "module": "ESNext",
    "moduleResolution": "bundler",
    "allowSyntheticDefaultImports": true
  },
  "include": ["vite.config.ts"]
}
```

### 3.4 `vite.config.ts`

```typescript
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 1421 }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
```

### 3.5 `vitest.config.ts`

```typescript
import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  test: {
    environment: 'happy-dom',
    globals: true,
  },
})
```

### 3.6 `index.html`

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <link rel="icon" type="image/svg+xml" href="/vite.svg" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Tauri + Vue + Typescript App</title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

### 3.7 `src-tauri/Cargo.toml`

```toml
[package]
name = "screencast-pro"
version = "0.1.0"
description = "A Tauri App"
authors = ["you"]
edition = "2021"

[lib]
name = "screencast_pro_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
gstreamer = "0.23"
gstreamer-app = "0.23"
gstreamer-video = "0.23"
gstreamer-rtsp = "0.23"
gstreamer-rtsp-server = "0.23"
gstreamer-sdp = "0.23"
glib = "0.20"
tokio-tungstenite = "0.24"
enigo = "0.3"
log = "0.4"
env_logger = "0.11"
anyhow = "1"
thiserror = "2"
futures-util = "0.3"
image = "0.25"
base64 = "0.22"

[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = [
    "Win32_Graphics_Gdi",
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
] }

[target.'cfg(target_os = "macos")'.dependencies]
core-graphics = { version = "0.24", features = ["highsierra"] }
core-foundation = "0.10"
```

### 3.8 `src-tauri/tauri.conf.json`

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "ScreenCast Pro",
  "version": "0.1.0",
  "identifier": "com.screencast-pro.app",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "ScreenCast Pro",
        "width": 1280,
        "height": 800,
        "minWidth": 1024,
        "minHeight": 600,
        "decorations": true,
        "resizable": true
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "windows": {
      "digestAlgorithm": "sha256"
    },
    "macOS": {
      "minimumSystemVersion": "10.15"
    },
    "linux": {
      "deb": {
        "depends": ["libgstreamer1.0-0", "gstreamer1.0-plugins-base", "gstreamer1.0-plugins-good", "gstreamer1.0-plugins-bad", "gstreamer1.0-vaapi"]
      }
    }
  }
}
```

### 3.9 `src-tauri/build.rs`

```rust
fn main() {
    tauri_build::build()
}
```

### 3.10 `src-tauri/capabilities/default.json`

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "opener:default"
  ]
}
```

### 3.11 `src-tauri/.cargo/config.toml`

```toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-L", "D:\\msvc_x86_64\\lib"]

[env]
GSTREAMER_1_0_ROOT_MSVC_X86_64 = "D:\\msvc_x86_64\\"
```

### 3.12 `src-tauri/.gitignore`

```
# Generated by Cargo
/target/
```

### 3.13 `.gitignore`（项目根）

```
# Logs
logs
*.log
npm-debug.log*
yarn-debug.log*
yarn-error.log*
pnpm-debug.log*
lerna-debug.log*

node_modules
dist
dist-ssr
*.local

# Editor directories and files
.vscode/*
!.vscode/extensions.json
.idea
.DS_Store
*.suo
*.ntvs*
*.njsproj
*.sln
*.sw?

# Rust / Cargo
target/
e:\screencast-build/

# Tauri
src-tauri/target/

# Build output
*.msi
*.exe
*.dmg
*.AppImage
*.deb

# Environment
.env
.env.local
```
