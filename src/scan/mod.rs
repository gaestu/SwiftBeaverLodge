//! Scan manager - spawns fastcarve binary and monitors progress

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

/// Log entry from fastcarve
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

/// Get fastcarve binary version
pub fn get_fastcarve_version() -> String {
    // Try to find and run fastcarve --version
    let binary_path = find_fastcarve_binary();
    
    if let Some(path) = binary_path {
        if let Ok(output) = std::process::Command::new(&path)
            .arg("--version")
            .output()
        {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout);
                return version
                    .trim()
                    .strip_prefix("fastcarve ")
                    .unwrap_or(version.trim())
                    .to_string();
            }
        }
    }
    
    "not installed".to_string()
}

/// Find the fastcarve binary
pub fn find_fastcarve_binary() -> Option<std::path::PathBuf> {
    // 1. Check in bin/ directory relative to executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let bin_path = exe_dir.join("bin").join("fastcarve");
            if bin_path.exists() {
                return Some(bin_path);
            }
            // Also check same directory
            let same_dir = exe_dir.join("fastcarve");
            if same_dir.exists() {
                return Some(same_dir);
            }
        }
    }
    
    // 2. Check current directory bin/
    let cwd_bin = std::path::PathBuf::from("bin/fastcarve");
    if cwd_bin.exists() {
        return Some(cwd_bin);
    }
    
    // 3. Check PATH
    which::which("fastcarve").ok()
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
    fn test_find_fastcarve_in_path() {
        // This test just ensures the function doesn't panic
        let _ = find_fastcarve_binary();
    }
}
