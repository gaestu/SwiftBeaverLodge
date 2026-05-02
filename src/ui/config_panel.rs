//! Configuration panel for scan settings

use egui::{Color32, RichText, Ui};
use rfd::FileDialog;

use crate::config::{
    hash_algorithms_include_sha256, validate_flag_combinations, MetadataBackend, ScanConfig,
    FILE_TYPES, SUPPORTED_HASH_ALGORITHMS, ZIP_DERIVED_TYPES,
};
use crate::devices::{list_block_devices, BlockDevice, DeviceType};

/// Replace `config.file_types` with the catalog entries for `category`.
/// No-op if the category is not present in `FILE_TYPES`.
fn set_preset(config: &mut ScanConfig, category: &str) {
    if let Some((_, _, types)) = FILE_TYPES.iter().find(|(c, _, _)| *c == category) {
        config.file_types = types.iter().map(|t| (*t).to_string()).collect();
    }
}

fn ensure_dedupe_hashing(config: &mut ScanConfig) {
    if config.dedupe
        && !config.hash_algorithms.is_empty()
        && !hash_algorithms_include_sha256(&config.hash_algorithms)
    {
        config.hash_algorithms.push("sha256".to_string());
    }
}

fn sha256_hash_checkbox_locked(config: &ScanConfig, algorithm: &str) -> bool {
    config.dedupe
        && algorithm.eq_ignore_ascii_case("sha256")
        && config
            .hash_algorithms
            .iter()
            .any(|selected| !selected.eq_ignore_ascii_case("sha256"))
}

/// Input source type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputSource {
    File,
    Device,
}

/// Configuration panel state
pub struct ConfigPanel {
    /// Show advanced options
    #[allow(dead_code)]
    show_advanced: bool,
    /// Current input source type
    input_source: InputSource,
    /// Cached list of block devices
    cached_devices: Vec<BlockDevice>,
    /// Whether to show partitions or just whole disks
    show_partitions: bool,
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
            input_source: InputSource::File,
            cached_devices: Vec::new(),
            show_partitions: false,
        }
    }

    /// Refresh the cached device list
    fn refresh_devices(&mut self) {
        self.cached_devices = list_block_devices();
    }

    /// Get filtered devices based on show_partitions setting
    fn get_filtered_devices(&self) -> Vec<&BlockDevice> {
        self.cached_devices
            .iter()
            .filter(|d| {
                if self.show_partitions {
                    true
                } else {
                    matches!(
                        d.device_type,
                        DeviceType::Disk | DeviceType::NVMe | DeviceType::Loop
                    )
                }
            })
            .collect()
    }

    /// Render the configuration panel
    pub fn show(&mut self, ui: &mut Ui, config: &mut ScanConfig) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Scan Configuration");
            ui.add_space(10.0);

            // Evidence source selection
            ui.group(|ui| {
                ui.label(RichText::new("Evidence Source").strong());

                // Source type selector
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.input_source, InputSource::File, "📄 File/Image");
                    ui.selectable_value(
                        &mut self.input_source,
                        InputSource::Device,
                        "💾 Raw Device",
                    );
                });

                ui.add_space(5.0);

                match self.input_source {
                    InputSource::File => {
                        // File input mode
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut config.input_path)
                                    .hint_text("Path to evidence file (.dd, .raw, .e01, etc.)")
                                    .desired_width(380.0),
                            );
                            if ui.button("Browse...").clicked() {
                                if let Some(path) = FileDialog::new()
                                    .add_filter("All Files", &["*"])
                                    .add_filter(
                                        "Disk Images",
                                        &["dd", "raw", "img", "dmg", "e01", "E01"],
                                    )
                                    .pick_file()
                                {
                                    config.input_path = path.display().to_string();
                                }
                            }
                        });
                    }
                    InputSource::Device => {
                        // Device selection mode
                        ui.horizontal(|ui| {
                            if ui.button("🔄 Refresh").clicked() {
                                self.refresh_devices();
                            }
                            ui.checkbox(&mut self.show_partitions, "Show partitions");
                        });

                        // Initialize devices if empty
                        if self.cached_devices.is_empty() {
                            self.refresh_devices();
                        }

                        let filtered_devices = self.get_filtered_devices();

                        if filtered_devices.is_empty() {
                            ui.colored_label(Color32::YELLOW, "⚠ No block devices found");
                            ui.label(RichText::new("Run as root/sudo to access devices").weak());
                        } else {
                            // Device dropdown
                            let current_device: String = filtered_devices
                                .iter()
                                .find(|d| d.path.to_string_lossy() == config.input_path)
                                .map(|d| d.display_name())
                                .unwrap_or_else(|| "Select a device...".to_string());

                            egui::ComboBox::from_id_salt("device_selector")
                                .width(450.0)
                                .selected_text(&current_device)
                                .show_ui(ui, |ui| {
                                    for device in &filtered_devices {
                                        let icon = match device.device_type {
                                            DeviceType::Disk | DeviceType::NVMe => "💿",
                                            DeviceType::Partition | DeviceType::NVMePartition => {
                                                "📁"
                                            }
                                            DeviceType::Loop => "🔄",
                                            DeviceType::Other => "📦",
                                        };
                                        let label = format!("{} {}", icon, device.display_name());
                                        let is_selected =
                                            device.path.to_string_lossy() == config.input_path;

                                        if ui.selectable_label(is_selected, &label).clicked() {
                                            config.input_path =
                                                device.path.to_string_lossy().to_string();
                                        }
                                    }
                                });

                            // Show selected device info
                            if let Some(device) = filtered_devices
                                .iter()
                                .find(|d| d.path.to_string_lossy() == config.input_path)
                            {
                                ui.add_space(5.0);
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Selected:").weak());
                                    ui.label(device.path.to_string_lossy().to_string());
                                    ui.label(
                                        RichText::new(format!("({})", device.size_display)).weak(),
                                    );
                                    if device.removable {
                                        ui.label(
                                            RichText::new("🔌 Removable")
                                                .color(Color32::LIGHT_BLUE),
                                        );
                                    }
                                });
                            }
                        }

                        // Warning about permissions
                        ui.add_space(5.0);
                        ui.label(
                            RichText::new("⚠ Raw device access requires root/sudo privileges")
                                .weak()
                                .color(Color32::YELLOW),
                        );
                    }
                }

                // Manual path entry (always available)
                ui.add_space(5.0);
                ui.collapsing("Manual path entry", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Path:");
                        ui.add(
                            egui::TextEdit::singleline(&mut config.input_path)
                                .hint_text("/dev/sda or /path/to/image.dd")
                                .desired_width(350.0),
                        );
                    });
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
                            .desired_width(400.0),
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
                ui.label(RichText::new("File Type Filters").strong());
                ui.add_space(5.0);

                ui.horizontal_wrapped(|ui| {
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
                    if ui.button("Images").clicked() {
                        set_preset(config, "Images");
                    }
                    if ui.button("Documents").clicked() {
                        set_preset(config, "Documents");
                    }
                    if ui.button("Media").clicked() {
                        set_preset(config, "Media");
                    }
                    if ui.button("Windows Artefacts").clicked() {
                        set_preset(config, "Windows Artefacts");
                    }
                });

                ui.add_space(5.0);

                // Show file types by category
                for (category, icon, types) in FILE_TYPES {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!("{} {}", icon, category));
                        for file_type in *types {
                            let mut selected = config.file_types.contains(&file_type.to_string());
                            let zip_derived = ZIP_DERIVED_TYPES.contains(file_type);
                            let label = if zip_derived {
                                format!("{file_type} *")
                            } else {
                                (*file_type).to_string()
                            };
                            let mut response = ui.checkbox(&mut selected, label);
                            if zip_derived {
                                response = response.on_hover_text(
                                    "ZIP-derived format: skipped when --disable-zip is set",
                                );
                            }
                            if response.changed() {
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

                ui.add_space(2.0);
                ui.label(
                    RichText::new(
                        "* ZIP-derived formats are skipped when \"Disable ZIP carving\" is on.",
                    )
                    .small()
                    .color(Color32::GRAY),
                );
            });

            ui.add_space(10.0);

            // Metadata backend
            ui.group(|ui| {
                ui.label(RichText::new("Metadata Format").strong());
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut config.metadata_backend,
                        MetadataBackend::Parquet,
                        "Parquet (recommended)",
                    );
                    ui.selectable_value(
                        &mut config.metadata_backend,
                        MetadataBackend::Jsonl,
                        "JSONL",
                    );
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
                ui.checkbox(
                    &mut config.compute_evidence_hash,
                    "Compute evidence SHA-256",
                );
                ui.checkbox(&mut config.disable_zip, "Disable ZIP archive scanning");

                ui.add_space(10.0);

                // GPU Acceleration section
                ui.label(RichText::new("GPU Acceleration").strong());
                ui.label(RichText::new("GPU support is toggled via the --gpu flag.").weak());
                ui.checkbox(&mut config.gpu_enabled, "Enable GPU acceleration (--gpu)");

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
                    ui.add(
                        egui::DragValue::new(&mut config.workers)
                            .speed(1)
                            .range(0..=64),
                    );
                    ui.label("(0 = auto)");
                });
                ui.horizontal(|ui| {
                    ui.label("Scan workers:");
                    ui.add(
                        egui::DragValue::new(&mut config.scan_workers)
                            .speed(1)
                            .range(0..=64),
                    );
                    ui.label("Carve:");
                    ui.add(
                        egui::DragValue::new(&mut config.carve_workers)
                            .speed(1)
                            .range(0..=64),
                    );
                    ui.label("Write:");
                    ui.add(
                        egui::DragValue::new(&mut config.write_workers)
                            .speed(1)
                            .range(0..=64),
                    );
                    ui.label("(0 = inherit)");
                });

                ui.add_space(10.0);

                // Chunk overlap
                ui.label(RichText::new("Chunking").strong());
                ui.horizontal(|ui| {
                    ui.label("Chunk size (MiB):");
                    ui.add(
                        egui::DragValue::new(&mut config.chunk_size_mib)
                            .speed(1)
                            .range(1..=4096),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Overlap (KiB):");
                    let mut has_overlap = config.overlap_kib.is_some();
                    if ui.checkbox(&mut has_overlap, "").changed() {
                        config.overlap_kib = if has_overlap { Some(64) } else { None };
                    }
                    if let Some(ref mut v) = config.overlap_kib {
                        ui.add(egui::DragValue::new(v).speed(1).range(0..=4096));
                    }
                });
                ui.label(
                    RichText::new(
                        "Resumed scans must use the same chunk size and overlap as the checkpointed scan.",
                    )
                    .weak(),
                );
                ui.horizontal(|ui| {
                    ui.label("Max chunks:");
                    let mut has = config.max_chunks.is_some();
                    if ui.checkbox(&mut has, "").changed() {
                        config.max_chunks = if has { Some(0) } else { None };
                    }
                    if let Some(ref mut v) = config.max_chunks {
                        ui.add(egui::DragValue::new(v).speed(10));
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Max open files:");
                    let mut has = config.max_open_files.is_some();
                    if ui.checkbox(&mut has, "").changed() {
                        config.max_open_files = if has { Some(1024) } else { None };
                    }
                    if let Some(ref mut v) = config.max_open_files {
                        ui.add(egui::DragValue::new(v).speed(16).range(16..=1_048_576));
                    }
                });

                ui.add_space(10.0);

                // String length override
                ui.label(RichText::new("String options").strong());
                ui.horizontal(|ui| {
                    ui.label("Min string length:");
                    ui.add(
                        egui::DragValue::new(&mut config.string_min_len)
                            .speed(1)
                            .range(1..=128),
                    );
                });

                ui.add_space(10.0);

                // Entropy detection
                ui.label(RichText::new("Entropy detection").strong());
                ui.checkbox(
                    &mut config.scan_entropy,
                    "Enable entropy-based region detection",
                );
                ui.add_enabled_ui(config.scan_entropy, |ui| {
                    ui.indent("entropy_options", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Threshold:");
                            ui.add(
                                egui::DragValue::new(&mut config.entropy_threshold)
                                    .speed(0.05)
                                    .range(0.0..=8.0),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Window bytes:");
                            let mut has = config.entropy_window_bytes.is_some();
                            if ui.checkbox(&mut has, "").changed() {
                                config.entropy_window_bytes = if has { Some(4096) } else { None };
                            }
                            if let Some(ref mut v) = config.entropy_window_bytes {
                                ui.add(egui::DragValue::new(v).speed(64).range(64..=1_048_576));
                            }
                        });
                    });
                });

                ui.add_space(10.0);

                // Run mode
                ui.label(RichText::new("Run mode").strong());
                ui.horizontal(|ui| {
                    if ui
                        .checkbox(&mut config.dry_run, "Dry run (count only)")
                        .changed()
                        && config.dry_run
                    {
                        config.metadata_only = false;
                    }
                    if ui
                        .checkbox(&mut config.metadata_only, "Metadata only")
                        .changed()
                        && config.metadata_only
                    {
                        config.dry_run = false;
                        config.validate_carved = false;
                        config.remove_invalid = false;
                    }
                });
                let run_mode_note = if config.dry_run {
                    "Dry run scans and counts only; SwiftBeaver output files are not written, so file validation and duplicate-body skipping are not used."
                } else if config.metadata_only {
                    "Metadata-only mode writes metadata records but does not write carved file bodies; validation/removal and duplicate-body skipping are disabled."
                } else {
                    "Normal mode writes carved files and metadata records to the run output directory."
                };
                ui.label(RichText::new(run_mode_note).weak());

                ui.add_space(10.0);

                // Validation & dedup
                ui.label(RichText::new("Validation & dedup").strong());
                let writes_carved_files = !config.dry_run && !config.metadata_only;
                ui.add_enabled_ui(writes_carved_files, |ui| {
                    ui.checkbox(
                        &mut config.validate_carved,
                        "Validate carved files (file magic check)",
                    );
                });
                ui.add_enabled_ui(config.validate_carved, |ui| {
                    ui.indent("validate_opts", |ui| {
                        ui.checkbox(
                            &mut config.remove_invalid,
                            "Remove invalid files after validation",
                        );
                    });
                });
                if !writes_carved_files {
                    config.validate_carved = false;
                    config.remove_invalid = false;
                }
                if !config.validate_carved && config.remove_invalid {
                    config.remove_invalid = false;
                }

                if ui
                    .checkbox(&mut config.dedupe, "Track duplicates in metadata")
                    .changed()
                {
                    ensure_dedupe_hashing(config);
                }
                ui.add_enabled_ui(config.dedupe, |ui| {
                    ui.indent("dedupe_opts", |ui| {
                        ui.add_enabled_ui(writes_carved_files, |ui| {
                            ui.checkbox(
                                &mut config.skip_duplicates,
                                "Skip writing duplicate file bodies",
                            );
                        });
                        if !writes_carved_files {
                            config.skip_duplicates = false;
                        }
                    });
                });
                if !config.dedupe && config.skip_duplicates {
                    config.skip_duplicates = false;
                }

                if config.skip_duplicates {
                    ui.label(
                        RichText::new(
                            "Duplicate records remain in metadata; duplicate file bodies are skipped.",
                        )
                        .weak(),
                    );
                }

                ui.horizontal(|ui| {
                    ui.label("Hash algorithms:");
                    for algo in SUPPORTED_HASH_ALGORITHMS {
                        let mut on = config
                            .hash_algorithms
                            .iter()
                            .any(|a| a.eq_ignore_ascii_case(algo));
                        let sha256_locked = sha256_hash_checkbox_locked(config, algo);
                        let response = ui.add_enabled(
                            !sha256_locked,
                            egui::Checkbox::new(&mut on, *algo),
                        );
                        if response.changed() {
                            config
                                .hash_algorithms
                                .retain(|a| !a.eq_ignore_ascii_case(algo));
                            if on {
                                config.hash_algorithms.push((*algo).to_string());
                            }
                            ensure_dedupe_hashing(config);
                        }
                    }
                });
                if config.dedupe {
                    ui.label(
                        RichText::new("Deduplication uses SHA-256; it is kept enabled when hashes are explicitly selected.")
                            .weak(),
                    );
                }

                ui.add_space(10.0);

                // Checkpoint / resume
                ui.label(RichText::new("Checkpoint / resume").strong());
                ui.horizontal(|ui| {
                    ui.label("Checkpoint path:");
                    let mut has = config.checkpoint_path.is_some();
                    if ui.checkbox(&mut has, "").changed() {
                        config.checkpoint_path = if has { Some(String::new()) } else { None };
                    }
                    if let Some(ref mut p) = config.checkpoint_path {
                        ui.add(
                            egui::TextEdit::singleline(p)
                                .hint_text("/path/to/run.ckpt")
                                .desired_width(280.0),
                        );
                        if ui.button("Browse...").clicked() {
                            if let Some(path) = FileDialog::new().save_file() {
                                *p = path.display().to_string();
                            }
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Resume from:");
                    let mut has = config.resume_from.is_some();
                    if ui.checkbox(&mut has, "").changed() {
                        config.resume_from = if has { Some(String::new()) } else { None };
                    }
                    if let Some(ref mut p) = config.resume_from {
                        ui.add(
                            egui::TextEdit::singleline(p)
                                .hint_text("/path/to/run.ckpt")
                                .desired_width(280.0),
                        );
                        if ui.button("Browse...").clicked() {
                            if let Some(path) = FileDialog::new().pick_file() {
                                *p = path.display().to_string();
                            }
                        }
                    }
                });
                let resume_active = config
                    .resume_from
                    .as_ref()
                    .is_some_and(|path| !path.is_empty());
                if resume_active {
                    ui.colored_label(
                        Color32::YELLOW,
                        "Resume mode active: this scan will continue from the selected checkpoint.",
                    );
                }
                ui.label(
                    RichText::new(
                        "Checkpoint files are written by SwiftBeaver during early exit. Resume only with matching chunk size and overlap.",
                    )
                    .weak(),
                );

                ui.add_space(10.0);

                // Optional YAML config
                ui.label(RichText::new("YAML config (optional)").strong());
                ui.horizontal(|ui| {
                    let mut has = config.config_path.is_some();
                    if ui.checkbox(&mut has, "Use --config-path").changed() {
                        config.config_path = if has { Some(String::new()) } else { None };
                    }
                    if let Some(ref mut p) = config.config_path {
                        ui.add(
                            egui::TextEdit::singleline(p)
                                .hint_text("/path/to/swiftbeaver.yaml")
                                .desired_width(260.0),
                        );
                        if ui.button("Browse...").clicked() {
                            if let Some(path) = FileDialog::new()
                                .add_filter("YAML", &["yaml", "yml"])
                                .pick_file()
                            {
                                *p = path.display().to_string();
                            }
                        }
                    }
                });

                ui.add_space(10.0);

                // Evidence SHA-256 override
                ui.label(RichText::new("Evidence SHA-256 (optional)").strong());
                ui.horizontal(|ui| {
                    let mut has = config.evidence_sha256.is_some();
                    if ui.checkbox(&mut has, "Provide hex").changed() {
                        config.evidence_sha256 = if has { Some(String::new()) } else { None };
                    }
                    if let Some(ref mut h) = config.evidence_sha256 {
                        ui.add(
                            egui::TextEdit::singleline(h)
                                .hint_text("sha256 hex")
                                .desired_width(420.0),
                        );
                    }
                });
            });

            ui.add_space(20.0);

            // Validation
            let validation = validate_config(config);
            if !validation.is_empty() {
                ui.group(|ui| {
                    ui.label(
                        RichText::new("⚠ Configuration Issues")
                            .color(Color32::YELLOW)
                            .strong(),
                    );
                    for issue in &validation {
                        ui.label(RichText::new(format!("• {}", issue)).color(Color32::YELLOW));
                    }
                });
            }
        });
    }
}

/// Validate configuration and return issues.
pub(crate) fn validate_config(config: &ScanConfig) -> Vec<String> {
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

    if let Some(path) = config.resume_from.as_ref().filter(|p| !p.is_empty()) {
        if !std::path::Path::new(path).is_file() {
            issues.push("Resume checkpoint file does not exist".to_string());
        }
    }

    if let Some(path) = config.checkpoint_path.as_ref().filter(|p| !p.is_empty()) {
        let checkpoint_path = std::path::Path::new(path);
        if let Some(parent) = checkpoint_path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                issues.push("Checkpoint directory does not exist".to_string());
            }
        }
    }

    issues.extend(validate_flag_combinations(config));

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

    #[test]
    fn ensure_dedupe_hashing_adds_sha256_for_explicit_hash_selection() {
        let mut config = ScanConfig {
            dedupe: true,
            hash_algorithms: vec!["md5".to_string()],
            ..Default::default()
        };

        ensure_dedupe_hashing(&mut config);

        assert!(config
            .hash_algorithms
            .iter()
            .any(|algo| algo.eq_ignore_ascii_case("sha256")));
    }

    #[test]
    fn ensure_dedupe_hashing_leaves_default_hash_selection_unset() {
        let mut config = ScanConfig {
            dedupe: true,
            hash_algorithms: Vec::new(),
            ..Default::default()
        };

        ensure_dedupe_hashing(&mut config);

        assert!(config.hash_algorithms.is_empty());
    }

    #[test]
    fn ensure_dedupe_hashing_is_idempotent_when_sha256_already_selected() {
        let mut config = ScanConfig {
            dedupe: true,
            hash_algorithms: vec!["md5".to_string(), "sha256".to_string()],
            ..Default::default()
        };

        ensure_dedupe_hashing(&mut config);

        assert_eq!(
            config
                .hash_algorithms
                .iter()
                .filter(|algo| algo.eq_ignore_ascii_case("sha256"))
                .count(),
            1
        );
    }

    #[test]
    fn sha256_hash_checkbox_lock_allows_return_to_default_hashes() {
        let only_sha256 = ScanConfig {
            dedupe: true,
            hash_algorithms: vec!["sha256".to_string()],
            ..Default::default()
        };
        assert!(!sha256_hash_checkbox_locked(&only_sha256, "sha256"));

        let md5_and_sha256 = ScanConfig {
            dedupe: true,
            hash_algorithms: vec!["md5".to_string(), "sha256".to_string()],
            ..Default::default()
        };
        assert!(sha256_hash_checkbox_locked(&md5_and_sha256, "sha256"));
        assert!(!sha256_hash_checkbox_locked(&md5_and_sha256, "md5"));
    }
}
