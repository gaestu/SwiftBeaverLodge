//! Configuration panel for scan settings

use egui::{Ui, RichText, Color32};
use rfd::FileDialog;

use crate::config::{ScanConfig, MetadataBackend, GpuVariant, FILE_TYPES};
use crate::scan::get_swiftbeaver_variant;

/// Configuration panel state
pub struct ConfigPanel {
    /// Show advanced options
    #[allow(dead_code)]
    show_advanced: bool,
}

impl Default for ConfigPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigPanel {
    pub fn new() -> Self {
        Self {
            show_advanced: false,
        }
    }

    /// Render the configuration panel
    pub fn show(&mut self, ui: &mut Ui, config: &mut ScanConfig) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Scan Configuration");
            ui.add_space(10.0);

            // Input file
            ui.group(|ui| {
                ui.label(RichText::new("Evidence File").strong());
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut config.input_path)
                            .hint_text("Path to evidence file or device")
                            .desired_width(400.0)
                    );
                    if ui.button("Browse...").clicked() {
                        if let Some(path) = FileDialog::new()
                            .add_filter("All Files", &["*"])
                            .add_filter("Disk Images", &["dd", "raw", "img", "dmg", "e01", "E01"])
                            .pick_file()
                        {
                            config.input_path = path.display().to_string();
                        }
                    }
                });
            });

            ui.add_space(10.0);

            // Output directory
            ui.group(|ui| {
                ui.label(RichText::new("Output Directory").strong());
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut config.output_path)
                            .hint_text("Directory for carved files")
                            .desired_width(400.0)
                    );
                    if ui.button("Browse...").clicked() {
                        if let Some(path) = FileDialog::new().pick_folder() {
                            config.output_path = path.display().to_string();
                        }
                    }
                });
            });

            ui.add_space(10.0);

            // File types (organized by category)
            ui.group(|ui| {
                ui.label(RichText::new("File Types to Carve").strong());
                ui.add_space(5.0);
                
                ui.horizontal(|ui| {
                    if ui.button("Select All").clicked() {
                        config.file_types.clear();
                        for (_category, _icon, types) in FILE_TYPES {
                            for t in *types {
                                config.file_types.push((*t).to_string());
                            }
                        }
                    }
                    if ui.button("Select None").clicked() {
                        config.file_types.clear();
                    }
                    if ui.button("Images Only").clicked() {
                        config.file_types = vec![
                            "jpeg".to_string(),
                            "png".to_string(),
                            "gif".to_string(),
                            "bmp".to_string(),
                            "webp".to_string(),
                        ];
                    }
                });
                
                ui.add_space(5.0);
                
                // Show file types by category
                for (category, icon, types) in FILE_TYPES {
                    ui.horizontal(|ui| {
                        ui.label(format!("{} {}", icon, category));
                        for file_type in *types {
                            let mut selected = config.file_types.contains(&file_type.to_string());
                            if ui.checkbox(&mut selected, *file_type).changed() {
                                if selected {
                                    if !config.file_types.contains(&file_type.to_string()) {
                                        config.file_types.push(file_type.to_string());
                                    }
                                } else {
                                    config.file_types.retain(|t| t != *file_type);
                                }
                            }
                        }
                    });
                }
            });

            ui.add_space(10.0);

            // Metadata backend
            ui.group(|ui| {
                ui.label(RichText::new("Metadata Format").strong());
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut config.metadata_backend, MetadataBackend::Parquet, "Parquet (recommended)");
                    ui.selectable_value(&mut config.metadata_backend, MetadataBackend::Jsonl, "JSONL");
                    ui.selectable_value(&mut config.metadata_backend, MetadataBackend::Csv, "CSV");
                });
            });

            ui.add_space(10.0);

            // String scanning
            ui.group(|ui| {
                ui.label(RichText::new("String Scanning").strong());
                ui.checkbox(&mut config.scan_strings, "Enable string extraction");
                
                ui.add_enabled_ui(config.scan_strings, |ui| {
                    ui.indent("string_options", |ui| {
                        ui.checkbox(&mut config.scan_utf16, "Scan UTF-16 strings");
                        ui.checkbox(&mut config.scan_urls, "Extract URLs");
                        ui.checkbox(&mut config.scan_emails, "Extract email addresses");
                        ui.checkbox(&mut config.scan_phones, "Extract phone numbers");
                    });
                });
            });

            ui.add_space(10.0);

            // Advanced options
            ui.collapsing("Advanced Options", |ui| {
                ui.checkbox(&mut config.compute_evidence_hash, "Compute evidence SHA-256");
                ui.checkbox(&mut config.disable_zip, "Disable ZIP archive scanning");
                
                ui.add_space(10.0);
                
                // GPU Acceleration section
                ui.label(RichText::new("GPU Acceleration").strong());
                
                // Show installed variant
                let installed_variant = get_swiftbeaver_variant();
                let variant_text = match &installed_variant {
                    Some(v) => format!("Installed: {}", v),
                    None => "Variant unknown (run download-swiftbeaver.sh)".to_string(),
                };
                ui.label(RichText::new(&variant_text).weak().italics());
                
                // GPU variant selection (for download guidance)
                ui.horizontal(|ui| {
                    ui.label("GPU Variant:");
                    egui::ComboBox::from_id_salt("gpu_variant")
                        .selected_text(config.gpu_variant.display_name())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut config.gpu_variant, GpuVariant::CpuOnly, GpuVariant::CpuOnly.display_name());
                            ui.selectable_value(&mut config.gpu_variant, GpuVariant::OpenCL, GpuVariant::OpenCL.display_name());
                            ui.selectable_value(&mut config.gpu_variant, GpuVariant::Cuda, GpuVariant::Cuda.display_name());
                        });
                });
                
                // GPU enable checkbox (only if variant supports GPU)
                let gpu_available = config.gpu_variant.supports_gpu();
                ui.add_enabled_ui(gpu_available, |ui| {
                    ui.checkbox(&mut config.gpu_enabled, "Enable GPU acceleration (--gpu)");
                });
                
                if !gpu_available && config.gpu_enabled {
                    config.gpu_enabled = false;
                }
                
                if config.gpu_variant != GpuVariant::CpuOnly && !config.gpu_enabled {
                    ui.label(RichText::new("ℹ GPU variant installed but GPU disabled").weak());
                }
                
                // Mismatch warning
                if let Some(ref installed) = installed_variant {
                    let expected = config.gpu_variant.as_str();
                    if installed != expected {
                        ui.colored_label(
                            Color32::YELLOW,
                            format!("⚠ Mismatch: {} installed, {} selected. Run: ./download-swiftbeaver.sh {}", 
                                installed, expected, expected)
                        );
                    }
                }
                
                ui.add_space(10.0);

                // Resource limits
                ui.label(RichText::new("Resource Limits").strong());
                
                ui.horizontal(|ui| {
                    ui.label("Max bytes to scan:");
                    let mut has_limit = config.max_bytes.is_some();
                    if ui.checkbox(&mut has_limit, "").changed() {
                        config.max_bytes = if has_limit { Some(0) } else { None };
                    }
                    if let Some(ref mut max) = config.max_bytes {
                        ui.add(egui::DragValue::new(max).speed(1_000_000).prefix("bytes: "));
                    }
                });
                
                ui.horizontal(|ui| {
                    ui.label("Max files to carve:");
                    let mut has_limit = config.max_files.is_some();
                    if ui.checkbox(&mut has_limit, "").changed() {
                        config.max_files = if has_limit { Some(1000) } else { None };
                    }
                    if let Some(ref mut max) = config.max_files {
                        ui.add(egui::DragValue::new(max).speed(100));
                    }
                });
                
                ui.horizontal(|ui| {
                    ui.label("Max memory (MiB):");
                    let mut has_limit = config.max_memory_mib.is_some();
                    if ui.checkbox(&mut has_limit, "").changed() {
                        config.max_memory_mib = if has_limit { Some(4096) } else { None };
                    }
                    if let Some(ref mut max) = config.max_memory_mib {
                        ui.add(egui::DragValue::new(max).speed(100).range(256..=65536));
                    }
                });
                
                ui.horizontal(|ui| {
                    ui.label("Worker threads:");
                    ui.add(egui::DragValue::new(&mut config.workers).speed(1).range(0..=64));
                    ui.label("(0 = auto)");
                });
            });

            ui.add_space(20.0);

            // Validation
            let validation = validate_config(config);
            if !validation.is_empty() {
                ui.group(|ui| {
                    ui.label(RichText::new("⚠ Configuration Issues").color(Color32::YELLOW).strong());
                    for issue in &validation {
                        ui.label(RichText::new(format!("• {}", issue)).color(Color32::YELLOW));
                    }
                });
            }
        });
    }
}

/// Validate configuration and return issues
fn validate_config(config: &ScanConfig) -> Vec<String> {
    let mut issues = Vec::new();
    
    if config.input_path.is_empty() {
        issues.push("Input file path is required".to_string());
    } else if !std::path::Path::new(&config.input_path).exists() {
        issues.push("Input file does not exist".to_string());
    }
    
    if config.output_path.is_empty() {
        issues.push("Output directory is required".to_string());
    }
    
    if config.file_types.is_empty() {
        issues.push("At least one file type must be selected".to_string());
    }
    
    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_empty_config() {
        let config = ScanConfig {
            input_path: String::new(),
            output_path: String::new(),
            file_types: Vec::new(),
            ..Default::default()
        };
        
        let issues = validate_config(&config);
        assert!(issues.len() >= 3);
    }

    #[test]
    fn test_validate_valid_config() {
        let config = ScanConfig {
            input_path: "/tmp".to_string(), // exists on most systems
            output_path: "/tmp/output".to_string(),
            file_types: vec!["jpeg".to_string()],
            ..Default::default()
        };
        
        let issues = validate_config(&config);
        assert!(issues.is_empty() || issues.len() == 1); // May not exist
    }
}
