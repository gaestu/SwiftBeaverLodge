//! Configuration commands

use std::path::PathBuf;
use crate::types::GuiScanConfig;

/// Load configuration from YAML file
#[tauri::command]
pub async fn load_config(path: String) -> Result<GuiScanConfig, String> {
    let path = PathBuf::from(&path);
    
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("Failed to read config file: {}", e))?;
    
    let config: GuiScanConfig = serde_yaml::from_str(&content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;
    
    Ok(config)
}

/// Save configuration to YAML file
#[tauri::command]
pub async fn save_config(config: GuiScanConfig, path: String) -> Result<(), String> {
    let path = PathBuf::from(&path);
    
    let content = serde_yaml::to_string(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    
    tokio::fs::write(&path, content)
        .await
        .map_err(|e| format!("Failed to write config file: {}", e))?;
    
    Ok(())
}

/// Get default configuration
#[tauri::command]
pub fn get_default_config() -> GuiScanConfig {
    GuiScanConfig::default()
}
