@echo off
chcp 65001 >nul 2>&1

:: Set GStreamer environment
set PATH=D:\msvc_x86_64\bin;%PATH%
set GSTREAMER_1_0_ROOT_MSVC_X86_64=D:\msvc_x86_64\
set PKG_CONFIG_PATH=D:\msvc_x86_64\lib\pkgconfig
set PKG_CONFIG=D:\msvc_x86_64\bin\pkg-config.exe
set CARGO_TARGET_DIR=e:\screencast-build
set GST_PLUGIN_PATH=D:\msvc_x86_64\lib\gstreamer-1.0

cd /d "e:\抓屏软件"

echo Building debug version with embedded frontend...
npx tauri build --debug --no-bundle

echo.
echo Done! Run: e:\screencast-build\debug\screencast-pro.exe
