//! Scan manager - spawns swiftbeaver binary and monitors progress

mod manager;
pub mod progress;

pub use manager::ScanManager;
pub use progress::{ScanProgress, format_bytes};

/// Scan state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanState {
    Idle,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Log entry from swiftbeaver
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

/// Get swiftbeaver binary version
pub fn get_swiftbeaver_version() -> String {
    // Try to find and run swiftbeaver --version
    let binary_path = find_swiftbeaver_binary();
    
    if let Some(path) = binary_path {
        if let Ok(output) = std::process::Command::new(&path)
            .arg("--version")
            .output()
        {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout);
                return version
                    .trim()
                    .strip_prefix("swiftbeaver ")
                    .unwrap_or(version.trim())
                    .to_string();
            }
        }
    }
    
    "not installed".to_string()
}

/// Get installed GPU variant (cpu-only, opencl, cuda)
pub fn get_swiftbeaver_variant() -> Option<String> {
    // Check for .variant file in bin/ directory
    let variant_file = std::path::PathBuf::from("bin/.variant");
    if variant_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&variant_file) {
            return Some(content.trim().to_string());
        }
    }
    
    // Also check relative to exe
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let variant_file = exe_dir.join("bin").join(".variant");
            if variant_file.exists() {
                if let Ok(content) = std::fs::read_to_string(&variant_file) {
                    return Some(content.trim().to_string());
                }
            }
        }
    }
    
    None
}

/// Find the swiftbeaver binary
pub fn find_swiftbeaver_binary() -> Option<std::path::PathBuf> {
    // 1. Check in bin/ directory relative to executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let bin_path = exe_dir.join("bin").join("swiftbeaver");
            if bin_path.exists() {
                return Some(bin_path);
            }
            // Also check same directory
            let same_dir = exe_dir.join("swiftbeaver");
            if same_dir.exists() {
                return Some(same_dir);
            }
        }
    }
    
    // 2. Check current directory bin/
    let cwd_bin = std::path::PathBuf::from("bin/swiftbeaver");
    if cwd_bin.exists() {
        return Some(cwd_bin);
    }
    
    // 3. Check PATH
    which::which("swiftbeaver").ok()
}

// Legacy alias for backward compatibility
#[allow(dead_code)]
pub fn find_fastcarve_binary() -> Option<std::path::PathBuf> {
    find_swiftbeaver_binary()
}

#[allow(dead_code)]
pub fn get_fastcarve_version() -> String {
    get_swiftbeaver_version()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_state_equality() {
        assert_eq!(ScanState::Idle, ScanState::Idle);
        assert_ne!(ScanState::Idle, ScanState::Running);
    }

    #[test]
    fn test_find_swiftbeaver_in_path() {
        // This test just ensures the function doesn't panic
        let _ = find_swiftbeaver_binary();
    }
    
    #[test]
    fn test_get_variant() {
        // This test just ensures the function doesn't panic
        let _ = get_swiftbeaver_variant();
    }
}
