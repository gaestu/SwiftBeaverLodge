//! Scan control commands

use tauri::{AppHandle, State};

use crate::AppState;
use crate::types::{GuiScanConfig, ScanHandle, ScanStatus};

/// Start a new scan
#[tauri::command]
pub async fn start_scan(
    config: GuiScanConfig,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<ScanHandle, String> {
    let mut manager = state.scan_manager.lock().await;
    manager.start(config, app).await.map_err(|e| e.to_string())
}

/// Stop the current scan
#[tauri::command]
pub async fn stop_scan(state: State<'_, AppState>) -> Result<(), String> {
    let mut manager = state.scan_manager.lock().await;
    manager.stop().map_err(|e| e.to_string())
}

/// Get current scan status
#[tauri::command]
pub async fn get_scan_status(state: State<'_, AppState>) -> Result<ScanStatus, String> {
    let manager = state.scan_manager.lock().await;
    Ok(manager.get_status())
}
