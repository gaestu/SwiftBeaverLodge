//! System information commands

use sysinfo::System;
use crate::types::{SystemInfo, GpuInfo};

/// Get system information
#[tauri::command]
pub fn get_system_info() -> SystemInfo {
    let mut sys = System::new_all();
    sys.refresh_all();
    
    let total_memory = sys.total_memory();
    let available_memory = sys.available_memory();
    let cpu_cores = sys.cpus().len() as u32;
    let cpu_model = sys.cpus()
        .first()
        .map(|cpu| cpu.brand().to_string());
    
    let os_name = System::name();
    let os_version = System::os_version();
    
    SystemInfo {
        os_name,
        os_version,
        cpu_cores,
        cpu_model,
        total_memory,
        available_memory,
        gpus: get_gpu_info(),
    }
}

/// Check GPU support
#[tauri::command]
pub fn check_gpu_support() -> Vec<GpuInfo> {
    get_gpu_info()
}

/// Get GPU information
fn get_gpu_info() -> Vec<GpuInfo> {
    let mut gpus = Vec::new();
    
    // Try OpenCL detection
    #[cfg(feature = "gpu-opencl")]
    {
        if let Ok(platforms) = opencl3::platform::get_platforms() {
            for platform in platforms {
                if let Ok(devices) = platform.get_devices(opencl3::device::CL_DEVICE_TYPE_GPU) {
                    for device in devices {
                        if let Ok(name) = device.name() {
                            gpus.push(GpuInfo {
                                name,
                                vendor: device.vendor().ok(),
                                driver_version: device.driver_version().ok(),
                                memory_bytes: device.global_mem_size().ok(),
                                backend: "OpenCL".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
    
    // Fallback: try to detect via sysfs on Linux
    #[cfg(target_os = "linux")]
    if gpus.is_empty() {
        gpus.extend(detect_gpus_linux());
    }
    
    gpus
}

#[cfg(target_os = "linux")]
fn detect_gpus_linux() -> Vec<GpuInfo> {
    let mut gpus = Vec::new();
    
    // Try reading from /sys/class/drm
    if let Ok(drm_dir) = std::fs::read_dir("/sys/class/drm") {
        for entry in drm_dir.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            
            // Look for cardX entries (not card0-connector entries)
            if name.starts_with("card") && !name.contains('-') {
                let device_path = entry.path().join("device");
                
                // Try to read vendor and device IDs
                let vendor_path = device_path.join("vendor");
                let _vendor = std::fs::read_to_string(&vendor_path).ok();
                
                // Try to read device name from uevent
                let uevent_path = device_path.join("uevent");
                if let Ok(uevent) = std::fs::read_to_string(&uevent_path) {
                    let mut gpu_name = format!("GPU {}", name);
                    let mut driver = None;
                    
                    for line in uevent.lines() {
                        if line.starts_with("DRIVER=") {
                            driver = Some(line.trim_start_matches("DRIVER=").to_string());
                        }
                        if line.starts_with("PCI_ID=") {
                            // Could look up vendor/device names from PCI IDs
                        }
                    }
                    
                    // Determine vendor from driver
                    let vendor = match driver.as_deref() {
                        Some("nvidia") => Some("NVIDIA".to_string()),
                        Some("amdgpu") | Some("radeon") => Some("AMD".to_string()),
                        Some("i915") | Some("xe") => Some("Intel".to_string()),
                        _ => None,
                    };
                    
                    // Try to get better name from debugfs or other sources
                    if let Some(v) = &vendor {
                        gpu_name = format!("{} GPU ({})", v, name);
                    }
                    
                    gpus.push(GpuInfo {
                        name: gpu_name,
                        vendor,
                        driver_version: driver,
                        memory_bytes: None,
                        backend: "Native".to_string(),
                    });
                }
            }
        }
    }
    
    gpus
}

/// Get SwiftBeaver version by running the binary
#[tauri::command]
pub fn get_swiftbeaver_version(app: tauri::AppHandle) -> String {
    use tauri::Manager;
    
    // Try to find and run the fastcarve binary with --version
    let binary_path = find_fastcarve_binary_for_version(&app);
    
    if let Some(path) = binary_path {
        if let Ok(output) = std::process::Command::new(&path)
            .arg("--version")
            .output()
        {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout);
                // Parse "fastcarve 0.2.1" -> "0.2.1"
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

/// Find fastcarve binary for version check (similar to scan manager)
fn find_fastcarve_binary_for_version(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    use tauri::Manager;
    
    // 1. Check bundled resources
    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled = resource_dir.join("bin").join("fastcarve");
        if bundled.exists() {
            return Some(bundled);
        }
        #[cfg(windows)]
        {
            let bundled_exe = resource_dir.join("bin").join("fastcarve.exe");
            if bundled_exe.exists() {
                return Some(bundled_exe);
            }
        }
    }

    // 2. Check dev bin directory
    let dev_bin = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bin").join("fastcarve");
    if dev_bin.exists() {
        return Some(dev_bin);
    }
    #[cfg(windows)]
    {
        let dev_bin_exe = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bin").join("fastcarve.exe");
        if dev_bin_exe.exists() {
            return Some(dev_bin_exe);
        }
    }

    // 3. Check PATH
    which::which("fastcarve").ok()
}

/// Get application version
#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
