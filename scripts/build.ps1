# ScreenCast Pro - Windows Build Script
# Builds the Tauri application and packages it as MSI/NSIS installer

param(
    [string]$GstRoot = $env:GSTREAMER_1_0_ROOT_MSVC_X86_64,
    [string]$OutputDir = ".\release-output"
)

$ErrorActionPreference = "Stop"

# Validate GStreamer installation
if (-not $GstRoot) {
    Write-Host "ERROR: GSTREAMER_1_0_ROOT_MSVC_X86_64 not set!" -ForegroundColor Red
    Write-Host "Download GStreamer MSVC x86_64 from: https://gstreamer.freedesktop.org/download/" -ForegroundColor Yellow
    exit 1
}

if (-not (Test-Path $GstRoot)) {
    Write-Host "ERROR: GStreamer root not found: $GstRoot" -ForegroundColor Red
    exit 1
}

Write-Host "=== ScreenCast Pro Windows Build ===" -ForegroundColor Cyan
Write-Host "GStreamer Root: $GstRoot"

# Set environment
$env:PATH = "$GstRoot\bin;" + $env:PATH
$env:GSTREAMER_1_0_ROOT_MSVC_X86_64 = $GstRoot
$env:PKG_CONFIG_PATH = "$GstRoot\lib\pkgconfig"
$env:PKG_CONFIG = "$GstRoot\bin\pkg-config.exe"
$env:CARGO_TARGET_DIR = "e:\screencast-build"

# Build
Write-Host "`n--- Building Tauri Application ---" -ForegroundColor Green
npm run tauri build 2>&1

if ($LASTEXITCODE -ne 0) {
    Write-Host "BUILD FAILED!" -ForegroundColor Red
    exit 1
}

Write-Host "`n--- Build Complete ---" -ForegroundColor Green

# Copy GStreamer DLLs to release directory
$releaseDir = ".\src-tauri\target\release"
$bundleDir = ".\src-tauri\target\release\bundle"

if (Test-Path $releaseDir) {
    Write-Host "`n--- Copying GStreamer Runtime DLLs ---" -ForegroundColor Yellow

    $gstBinDlls = Get-ChildItem "$GstRoot\bin\*.dll" -ErrorAction SilentlyContinue
    $gstPluginDlls = Get-ChildItem "$GstRoot\lib\gstreamer-1.0\*.dll" -ErrorAction SilentlyContinue

    $dllCount = 0
    foreach ($dll in $gstBinDlls) {
        Copy-Item $dll.FullName -Destination $releaseDir -Force -ErrorAction SilentlyContinue
        $dllCount++
    }

    Write-Host "  Copied $dllCount GStreamer DLLs to release directory"

    # Create plugins directory
    $pluginsDir = "$releaseDir\gstreamer-1.0"
    if (-not (Test-Path $pluginsDir)) {
        New-Item -ItemType Directory -Path $pluginsDir | Out-Null
    }

    $pluginCount = 0
    foreach ($dll in $gstPluginDlls) {
        Copy-Item $dll.FullName -Destination $pluginsDir -Force -ErrorAction SilentlyContinue
        $pluginCount++
    }

    Write-Host "  Copied $pluginCount GStreamer plugins to $pluginsDir"
}

# Report output
if (Test-Path $bundleDir) {
    Write-Host "`n=== Build Artifacts ===" -ForegroundColor Cyan
    Get-ChildItem $bundleDir -Recurse -Include "*.msi","*.exe" | ForEach-Object {
        Write-Host "  $($_.FullName) ($([math]::Round($_.Length / 1MB, 1)) MB)" -ForegroundColor White
    }
}

Write-Host "`nBuild script complete!" -ForegroundColor Green
