//! SwiftBeaverLodge - Tauri Backend
//!
//! GUI frontend for the SwiftBeaver forensic file carver.

pub mod commands;
pub mod scan;
pub mod types;

use std::sync::Arc;
use tokio::sync::Mutex;

use scan::manager::ScanManager;

/// Application state shared across Tauri commands
pub struct AppState {
    pub scan_manager: Arc<Mutex<ScanManager>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            scan_manager: Arc::new(Mutex::new(ScanManager::new())),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Run the Tauri application
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_os::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            // Scan commands
            commands::scan::start_scan,
            commands::scan::stop_scan,
            commands::scan::get_scan_status,
            // Config commands
            commands::config::load_config,
            commands::config::save_config,
            commands::config::get_default_config,
            // File commands
            commands::files::browse_input_file,
            commands::files::browse_output_dir,
            commands::files::list_block_devices,
            commands::files::read_carved_metadata,
            commands::files::read_string_artefacts,
            commands::files::read_browser_history,
            commands::files::read_run_summary,
            commands::files::generate_thumbnail,
            commands::files::read_file_bytes,
            // System commands
            commands::system::get_system_info,
            commands::system::check_gpu_support,
            commands::system::get_swiftbeaver_version,
            commands::system::get_app_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
