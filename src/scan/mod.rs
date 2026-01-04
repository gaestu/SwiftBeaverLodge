//! Scan manager - spawns swiftbeaver binary and monitors progress

mod manager;
pub mod progress;

use crate::config::GpuVariant;

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

/// Get swiftbeaver binary version for a specific variant
pub fn get_swiftbeaver_version() -> String {
    get_swiftbeaver_version_for_variant(GpuVariant::CpuOnly)
}

/// Get swiftbeaver binary version for a specific variant
pub fn get_swiftbeaver_version_for_variant(variant: GpuVariant) -> String {
    let binary_path = find_swiftbeaver_binary_for_variant(variant);
    
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

/// Get list of available (installed) GPU variants
pub fn get_available_variants() -> Vec<GpuVariant> {
    let mut available = Vec::new();
    
    for variant in [GpuVariant::CpuOnly, GpuVariant::OpenCL, GpuVariant::Cuda] {
        if find_swiftbeaver_binary_for_variant(variant).is_some() {
            available.push(variant);
        }
    }
    
    available
}

/// Check if a specific variant is available
pub fn is_variant_available(variant: GpuVariant) -> bool {
    find_swiftbeaver_binary_for_variant(variant).is_some()
}

/// Find the swiftbeaver binary for a specific GPU variant
pub fn find_swiftbeaver_binary_for_variant(variant: GpuVariant) -> Option<std::path::PathBuf> {
    let binary_name = format!("swiftbeaver-{}", variant.as_str());
    
    // 1. Check in bin/ directory relative to executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let bin_path = exe_dir.join("bin").join(&binary_name);
            if bin_path.exists() {
                return Some(bin_path);
            }
        }
    }
    
    // 2. Check current directory bin/
    let cwd_bin = std::path::PathBuf::from(format!("bin/{}", binary_name));
    if cwd_bin.exists() {
        return Some(cwd_bin);
    }
    
    // 3. Check PATH
    which::which(&binary_name).ok()
}

/// Find any swiftbeaver binary (legacy - prefers cpu-only, then any variant)
pub fn find_swiftbeaver_binary() -> Option<std::path::PathBuf> {
    // Try variants in order of preference
    for variant in [GpuVariant::CpuOnly, GpuVariant::OpenCL, GpuVariant::Cuda] {
        if let Some(path) = find_swiftbeaver_binary_for_variant(variant) {
            return Some(path);
        }
    }
    
    // Fallback: check for generic "swiftbeaver" (symlink or legacy)
    let locations = [
        std::path::PathBuf::from("bin/swiftbeaver"),
    ];
    
    for path in locations {
        if path.exists() {
            return Some(path);
        }
    }
    
    // Check PATH
    which::which("swiftbeaver").ok()
}

// Legacy aliases for backward compatibility
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
    fn test_get_available_variants() {
        // This test just ensures the function doesn't panic
        let variants = get_available_variants();
        // variants may be empty if no binaries installed
        assert!(variants.len() <= 3);
    }
    
    #[test]
    fn test_find_variant_binary() {
        // This test just ensures the function doesn't panic
        let _ = find_swiftbeaver_binary_for_variant(GpuVariant::CpuOnly);
        let _ = find_swiftbeaver_binary_for_variant(GpuVariant::OpenCL);
        let _ = find_swiftbeaver_binary_for_variant(GpuVariant::Cuda);
    }
}
