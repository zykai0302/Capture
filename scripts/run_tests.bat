@echo off
set PATH=D:\msvc_x86_64\bin;%PATH%
set GSTREAMER_1_0_ROOT_MSVC_X86_64=D:\msvc_x86_64\
set PKG_CONFIG_PATH=D:\msvc_x86_64\lib\pkgconfig
set PKG_CONFIG=D:\msvc_x86_64\bin\pkg-config.exe
set CARGO_TARGET_DIR=e:\screencast-build
cargo test --manifest-path "%~dp0..\src-tauri\Cargo.toml" %*
