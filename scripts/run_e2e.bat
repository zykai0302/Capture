@echo off
chcp 65001 >nul 2>&1
set PATH=D:\msvc_x86_64\bin;%PATH%
set GSTREAMER_1_0_ROOT_MSVC_X86_64=D:\msvc_x86_64\
set PKG_CONFIG_PATH=D:\msvc_x86_64\lib\pkgconfig
set PKG_CONFIG=D:\msvc_x86_64\bin\pkg-config.exe
set CARGO_TARGET_DIR=e:\screencast-build
set GST_PLUGIN_PATH=D:\msvc_x86_64\lib\gstreamer-1.0
set RUST_BACKTRACE=1
set RUST_LOG=info

echo Starting ScreenCast Pro...
start "" "e:\screencast-build\debug\screencast-pro.exe"

echo Waiting 10 seconds for application to start...
timeout /t 10 /nobreak >nul

echo Application started. Check the window for status.
