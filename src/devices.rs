//! Block device detection for raw device scanning

use std::path::PathBuf;

/// Information about a block device
#[derive(Debug, Clone)]
pub struct BlockDevice {
    /// Device path (e.g., /dev/sda)
    pub path: PathBuf,
    /// Device name (e.g., sda)
    pub name: String,
    /// Size in bytes
    #[allow(dead_code)]
    pub size: u64,
    /// Human-readable size
    pub size_display: String,
    /// Device model/description if available
    pub model: Option<String>,
    /// Whether this is a removable device
    pub removable: bool,
    /// Device type (disk, partition, etc.)
    pub device_type: DeviceType,
}

/// Type of block device
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    /// Whole disk (e.g., /dev/sda)
    Disk,
    /// Partition (e.g., /dev/sda1)
    Partition,
    /// Loop device (e.g., /dev/loop0)
    Loop,
    /// NVMe device (e.g., /dev/nvme0n1)
    NVMe,
    /// NVMe partition (e.g., /dev/nvme0n1p1)
    NVMePartition,
    /// Other/unknown
    Other,
}

impl BlockDevice {
    /// Display string for UI
    pub fn display_name(&self) -> String {
        let model_str = self.model.as_deref().unwrap_or("Unknown");
        format!("{} - {} ({})", self.name, model_str, self.size_display)
    }
}

/// Format bytes to human-readable string
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Detect block device type from name
fn detect_device_type(name: &str) -> DeviceType {
    if name.starts_with("nvme") {
        if name.contains("p")
            && name
                .chars()
                .last()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
        {
            // Check if it's nvme0n1p1 pattern (partition)
            if let Some(pos) = name.rfind('p') {
                if name[pos + 1..].chars().all(|c| c.is_ascii_digit()) {
                    return DeviceType::NVMePartition;
                }
            }
        }
        DeviceType::NVMe
    } else if name.starts_with("loop") {
        DeviceType::Loop
    } else if name.starts_with("sd") || name.starts_with("hd") || name.starts_with("vd") {
        // Check if it ends with a number (partition) or just letters (disk)
        if name
            .chars()
            .last()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
        {
            DeviceType::Partition
        } else {
            DeviceType::Disk
        }
    } else {
        DeviceType::Other
    }
}

/// List available block devices on the system
///
/// This reads from /sys/block on Linux to enumerate devices.
/// On non-Linux systems, returns an empty list.
#[cfg(target_os = "linux")]
pub fn list_block_devices() -> Vec<BlockDevice> {
    let mut devices = Vec::new();

    // Read /sys/block for block devices
    let sys_block = PathBuf::from("/sys/block");
    if !sys_block.exists() {
        return devices;
    }

    let entries = match std::fs::read_dir(&sys_block) {
        Ok(e) => e,
        Err(_) => return devices,
    };

    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip ram devices and certain virtual devices
        if name.starts_with("ram") || name.starts_with("zram") {
            continue;
        }

        let device_path = PathBuf::from("/dev").join(&name);
        if !device_path.exists() {
            continue;
        }

        // Get device size
        let size_path = sys_block.join(&name).join("size");
        let size = std::fs::read_to_string(&size_path)
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .map(|sectors| sectors * 512) // Convert sectors to bytes
            .unwrap_or(0);

        // Skip devices with 0 size
        if size == 0 {
            continue;
        }

        // Get removable status
        let removable_path = sys_block.join(&name).join("removable");
        let removable = std::fs::read_to_string(&removable_path)
            .ok()
            .map(|s| s.trim() == "1")
            .unwrap_or(false);

        // Try to get model
        let model_path = sys_block.join(&name).join("device/model");
        let model = std::fs::read_to_string(&model_path)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        // If no model, try vendor
        let model = model.or_else(|| {
            let vendor_path = sys_block.join(&name).join("device/vendor");
            std::fs::read_to_string(&vendor_path)
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        });

        let device_type = detect_device_type(&name);

        devices.push(BlockDevice {
            path: device_path,
            name,
            size,
            size_display: format_size(size),
            model,
            removable,
            device_type,
        });
    }

    // Sort by name
    devices.sort_by(|a, b| a.name.cmp(&b.name));

    devices
}

/// List available block devices (non-Linux stub)
#[cfg(not(target_os = "linux"))]
pub fn list_block_devices() -> Vec<BlockDevice> {
    // On non-Linux systems, return empty list
    // Users can still manually enter device paths
    Vec::new()
}

/// Filter devices to show only whole disks (not partitions)
#[allow(dead_code)]
pub fn list_disks_only() -> Vec<BlockDevice> {
    list_block_devices()
        .into_iter()
        .filter(|d| {
            matches!(
                d.device_type,
                DeviceType::Disk | DeviceType::NVMe | DeviceType::Loop
            )
        })
        .collect()
}

/// Check if a path looks like a block device
#[allow(dead_code)]
pub fn is_block_device_path(path: &str) -> bool {
    path.starts_with("/dev/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_size(1024 * 1024 * 1024 * 1024), "1.0 TB");
    }

    #[test]
    fn test_detect_device_type() {
        assert_eq!(detect_device_type("sda"), DeviceType::Disk);
        assert_eq!(detect_device_type("sda1"), DeviceType::Partition);
        assert_eq!(detect_device_type("nvme0n1"), DeviceType::NVMe);
        assert_eq!(detect_device_type("nvme0n1p1"), DeviceType::NVMePartition);
        assert_eq!(detect_device_type("loop0"), DeviceType::Loop);
    }

    #[test]
    fn test_is_block_device_path() {
        assert!(is_block_device_path("/dev/sda"));
        assert!(is_block_device_path("/dev/nvme0n1"));
        assert!(!is_block_device_path("/home/user/image.dd"));
        assert!(!is_block_device_path("image.dd"));
    }

    #[test]
    fn test_list_block_devices() {
        // Just ensure it doesn't panic
        let devices = list_block_devices();
        // On a real system, there should be some devices
        // But in CI/test env, might be empty
        for device in &devices {
            assert!(!device.name.is_empty());
            assert!(device.path.starts_with("/dev"));
        }
    }
}
