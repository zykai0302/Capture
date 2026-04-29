use crate::capture::platform::enumerate_sources;
use crate::capture::source::CaptureSourceList;
use crate::pipeline::manager::GstPipelineManager;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Maximum time to wait for enumerate_sources() before giving up on a poll cycle.
/// Prevents the hotplug thread from hanging indefinitely if Win32 APIs stall.
const ENUM_TIMEOUT_SECS: u64 = 10;

/// Start a background thread that polls for source changes every 2 seconds
/// and emits Tauri events when sources are added/removed.
/// When a source is removed, automatically stops its pipeline.
pub fn start_hotplug_monitor(app: AppHandle, pipeline_manager: Arc<GstPipelineManager>) {
    std::thread::spawn(move || {
        let mut last_sources = match enumerate_sources_with_timeout() {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to enumerate sources for hotplug: {}", e);
                // Use empty list so we retry on next cycle
                CaptureSourceList { monitors: vec![], windows: vec![] }
            }
        };

        loop {
            std::thread::sleep(Duration::from_secs(2));

            let current_sources = match enumerate_sources_with_timeout() {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("Hotplug poll enumerate failed: {}, retrying next cycle", e);
                    continue;
                }
            };

            // Detect added sources
            for source in current_sources.all() {
                if !last_sources.contains_id(&source.id) {
                    log::info!("Source added: {}", source.id);
                    let _ = app.emit("source-added", source);
                }
            }

            // Detect removed sources — stop pipeline and notify UI
            for source in last_sources.all() {
                if !current_sources.contains_id(&source.id) {
                    log::info!("Source removed: {}, stopping pipeline if running", source.id);

                    // Automatically stop the pipeline for the removed source
                    if let Err(e) = pipeline_manager.stop_pipeline(&source.id) {
                        log::warn!("Failed to stop pipeline for removed source {}: {}", source.id, e);
                    }

                    let _ = app.emit("source-removed", source);
                }
            }

            last_sources = current_sources;
        }
    });
}

/// Enumerate sources with a timeout to prevent the hotplug thread from hanging.
/// Uses a channel-based timeout since Win32 APIs may stall
/// on UAC screens, secure desktops, or hung windows.
fn enumerate_sources_with_timeout() -> Result<CaptureSourceList, String> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let result = enumerate_sources();
        let _ = tx.send(result);
    });

    match rx.recv_timeout(Duration::from_secs(ENUM_TIMEOUT_SECS)) {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => Err(format!("Enumeration error: {}", e)),
        Err(mpsc::RecvTimeoutError::Timeout) => {
            Err(format!(
                "Enumeration timed out after {}s (possible Win32 API stall)",
                ENUM_TIMEOUT_SECS
            ))
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err("Enumeration thread panicked".to_string())
        }
    }
}
