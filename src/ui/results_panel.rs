//! Results browser panel

use egui::{Color32, RichText, Ui};

use crate::metadata::{CarvedFile, MetadataReader, MetadataSummary, StringArtefact};
use crate::scan::format_bytes;

/// Results browser panel
pub struct ResultsPanel {
    /// Current run path
    run_path: Option<String>,
    /// Loaded metadata reader
    reader: Option<MetadataReader>,
    /// Cached summary
    summary: Option<MetadataSummary>,
    /// Cached files
    files: Vec<CarvedFile>,
    /// Cached string artefacts
    strings: Vec<StringArtefact>,
    /// Current tab
    current_tab: ResultsTab,
    /// File type filter
    type_filter: Option<String>,
    /// Search query
    search_query: String,
    /// Selected file index
    selected_file: Option<usize>,
    /// Error message
    error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ResultsTab {
    #[default]
    Overview,
    Files,
    Strings,
}

impl Default for ResultsPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ResultsPanel {
    pub fn new() -> Self {
        Self {
            run_path: None,
            reader: None,
            summary: None,
            files: Vec::new(),
            strings: Vec::new(),
            current_tab: ResultsTab::Overview,
            type_filter: None,
            search_query: String::new(),
            selected_file: None,
            error: None,
        }
    }

    /// Load results from a run directory
    pub fn load(&mut self, run_path: &str) {
        self.clear_loaded_state();
        self.run_path = Some(run_path.to_string());

        match MetadataReader::new(run_path) {
            Ok(reader) => {
                let files = match reader.read_carved_files() {
                    Ok(files) => files,
                    Err(e) => {
                        self.error = Some(format_load_error("Failed to load files", &e));
                        return;
                    }
                };

                let strings = match reader.read_string_artefacts() {
                    Ok(strings) => strings,
                    Err(e) => {
                        self.error = Some(format_load_error("Failed to load string artefacts", &e));
                        Vec::new()
                    }
                };

                let summary = match MetadataSummary::from_results(&files, &strings) {
                    Ok(summary) => summary,
                    Err(e) => {
                        self.error = Some(format_load_error("Failed to summarize results", &e));
                        return;
                    }
                };

                self.summary = Some(summary);
                self.files = files;
                self.strings = strings;
                self.reader = Some(reader);
            }
            Err(e) => {
                self.error = Some(format_load_error("Failed to open results", &e));
            }
        }
    }

    /// Clear loaded results
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.run_path = None;
        self.clear_loaded_state();
    }

    fn clear_loaded_state(&mut self) {
        self.reader = None;
        self.summary = None;
        self.files.clear();
        self.strings.clear();
        self.type_filter = None;
        self.search_query.clear();
        self.selected_file = None;
        self.error = None;
    }

    /// Render the results panel
    pub fn show(&mut self, ui: &mut Ui, run_output_path: Option<&str>) {
        ui.heading("Scan Results");
        ui.add_space(10.0);

        // Check if we need to load new results
        if let Some(path) = run_output_path {
            if self.run_path.as_deref() != Some(path) {
                self.load(path);
            }
        }

        // Show error if any
        if let Some(error) = &self.error {
            ui.colored_label(Color32::RED, format!("Error: {}", error));
            ui.add_space(10.0);
        }

        // No results loaded
        if self.reader.is_none() {
            ui.label("No results loaded. Run a scan or load previous results.");

            ui.add_space(10.0);

            if ui.button("Load Previous Results...").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.load(&path.display().to_string());
                }
            }

            return;
        }

        // Tab bar
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.current_tab, ResultsTab::Overview, "Overview");
            ui.selectable_value(
                &mut self.current_tab,
                ResultsTab::Files,
                format!("Files ({})", self.files.len()),
            );
            ui.selectable_value(
                &mut self.current_tab,
                ResultsTab::Strings,
                format!("Strings ({})", self.strings.len()),
            );
        });

        ui.separator();

        match self.current_tab {
            ResultsTab::Overview => self.show_overview(ui),
            ResultsTab::Files => self.show_files(ui),
            ResultsTab::Strings => self.show_strings(ui),
        }
    }

    fn show_overview(&mut self, ui: &mut Ui) {
        if let Some(summary) = &self.summary {
            ui.group(|ui| {
                ui.label(RichText::new("Summary").strong());

                egui::Grid::new("summary_grid")
                    .num_columns(2)
                    .spacing([40.0, 5.0])
                    .show(ui, |ui| {
                        ui.label("Total Files:");
                        ui.label(RichText::new(format!("{}", summary.total_files)).strong());
                        ui.end_row();

                        ui.label("Total Size:");
                        ui.label(RichText::new(format_bytes(summary.total_bytes)).strong());
                        ui.end_row();

                        ui.label("String Artefacts:");
                        ui.label(RichText::new(format!("{}", summary.string_artefacts)).strong());
                        ui.end_row();
                    });
            });

            ui.add_space(10.0);

            // Files by type
            ui.group(|ui| {
                ui.label(RichText::new("Files by Type").strong());

                let mut sorted_types: Vec<_> = summary.by_type.iter().collect();
                sorted_types.sort_by(|a, b| b.1.cmp(a.1));

                egui::Grid::new("types_grid")
                    .num_columns(2)
                    .spacing([20.0, 5.0])
                    .show(ui, |ui| {
                        for (file_type, count) in sorted_types {
                            ui.label(file_type);
                            ui.label(format!("{}", count));
                            ui.end_row();
                        }
                    });
            });
        }
    }

    fn show_files(&mut self, ui: &mut Ui) {
        // Filters
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.search_query);

            if ui.button("Clear").clicked() {
                self.search_query.clear();
                self.type_filter = None;
            }
        });

        ui.horizontal(|ui| {
            ui.label("Type:");
            if ui
                .selectable_label(self.type_filter.is_none(), "All")
                .clicked()
            {
                self.type_filter = None;
            }

            // Get unique types
            let mut types: Vec<_> = self
                .files
                .iter()
                .map(|f| f.file_type.as_str())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            types.sort();

            for file_type in types {
                let selected = self.type_filter.as_deref() == Some(file_type);
                if ui.selectable_label(selected, file_type).clicked() {
                    self.type_filter = Some(file_type.to_string());
                }
            }
        });

        ui.separator();

        // Filtered files
        let filtered: Vec<_> = self
            .files
            .iter()
            .enumerate()
            .filter(|(_, f)| {
                if let Some(ref type_filter) = self.type_filter {
                    if &f.file_type != type_filter {
                        return false;
                    }
                }
                if !self.search_query.is_empty()
                    && !f
                        .path
                        .to_lowercase()
                        .contains(&self.search_query.to_lowercase())
                {
                    return false;
                }
                true
            })
            .collect();

        ui.label(format!(
            "Showing {} of {} files",
            filtered.len(),
            self.files.len()
        ));

        // File list
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                egui::Grid::new("files_grid")
                    .num_columns(5)
                    .striped(true)
                    .spacing([10.0, 5.0])
                    .show(ui, |ui| {
                        // Header
                        ui.label(RichText::new("ID").strong());
                        ui.label(RichText::new("Type").strong());
                        ui.label(RichText::new("Offset").strong());
                        ui.label(RichText::new("Size").strong());
                        ui.label(RichText::new("Path").strong());
                        ui.end_row();

                        // Files
                        for (idx, file) in filtered.iter().take(500) {
                            let selected = self.selected_file == Some(*idx);

                            if ui
                                .selectable_label(selected, format!("{}", file.id))
                                .clicked()
                            {
                                self.selected_file = Some(*idx);
                            }
                            ui.label(&file.file_type);
                            ui.label(format!("0x{:X}", file.global_start));
                            ui.label(format_bytes(file.size));
                            ui.label(&file.path);
                            ui.end_row();
                        }

                        if filtered.len() > 500 {
                            ui.label("...");
                            ui.label(format!("({} more files)", filtered.len() - 500));
                            ui.end_row();
                        }
                    });
            });

        // File details
        if let Some(idx) = self.selected_file {
            if let Some(file) = self.files.get(idx) {
                ui.add_space(10.0);
                ui.separator();

                ui.group(|ui| {
                    ui.label(RichText::new("File Details").strong());

                    egui::Grid::new("file_details")
                        .num_columns(2)
                        .spacing([20.0, 5.0])
                        .show(ui, |ui| {
                            ui.label("ID:");
                            ui.label(format!("{}", file.id));
                            ui.end_row();

                            ui.label("Type:");
                            ui.label(&file.file_type);
                            ui.end_row();

                            ui.label("Offset:");
                            ui.label(format!("0x{:X} ({})", file.global_start, file.global_start));
                            ui.end_row();

                            ui.label("End:");
                            ui.label(format!("0x{:X} ({})", file.global_end, file.global_end));
                            ui.end_row();

                            ui.label("Size:");
                            ui.label(format_bytes(file.size));
                            ui.end_row();

                            ui.label("Path:");
                            ui.label(&file.path);
                            ui.end_row();

                            if let Some(md5) = &file.md5 {
                                ui.label("MD5:");
                                ui.label(md5);
                                ui.end_row();
                            }

                            if let Some(sha256) = &file.sha256 {
                                ui.label("SHA-256:");
                                ui.label(sha256);
                                ui.end_row();
                            }

                            if let (Some(w), Some(h)) = (file.width, file.height) {
                                ui.label("Dimensions:");
                                ui.label(format!("{}x{}", w, h));
                                ui.end_row();
                            }

                            ui.label("Validation:");
                            ui.label(validation_status_label(file.validated));
                            ui.end_row();

                            ui.label("Truncated:");
                            ui.label(if file.truncated { "Yes" } else { "No" });
                            ui.end_row();

                            if let Some(pattern_id) = &file.pattern_id {
                                ui.label("Pattern:");
                                ui.label(pattern_id);
                                ui.end_row();
                            }

                            ui.label("Duplicate:");
                            ui.label(if file.is_duplicate { "Yes" } else { "No" });
                            ui.end_row();

                            if file.is_duplicate {
                                if let Some(offset) = file.duplicate_of_offset {
                                    ui.label("Original Offset:");
                                    ui.label(format!("0x{offset:X} ({offset})"));
                                    ui.end_row();
                                }
                            }

                            if !file.errors.is_empty() {
                                ui.label("Errors:");
                                ui.label(format_error_list(&file.errors));
                                ui.end_row();
                            }

                            if let Some(run_id) = &file.run_id {
                                ui.label("Run ID:");
                                ui.label(run_id);
                                ui.end_row();
                            }

                            if let Some(handler_id) = &file.handler_id {
                                ui.label("Handler:");
                                ui.label(handler_id);
                                ui.end_row();
                            }

                            if let Some(extension) = &file.extension {
                                ui.label("Extension:");
                                ui.label(extension);
                                ui.end_row();
                            }
                        });
                });
            }
        }
    }

    fn show_strings(&mut self, ui: &mut Ui) {
        if self.strings.is_empty() {
            ui.label("No string artefacts found. Enable string scanning in configuration.");
            return;
        }

        // Type filter
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.search_query);
        });

        ui.separator();

        // Filtered strings
        let filtered: Vec<_> = self
            .strings
            .iter()
            .filter(|s| {
                if !self.search_query.is_empty() {
                    return s
                        .value
                        .to_lowercase()
                        .contains(&self.search_query.to_lowercase());
                }
                true
            })
            .collect();

        ui.label(format!(
            "Showing {} of {} artefacts",
            filtered.len(),
            self.strings.len()
        ));

        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                egui::Grid::new("strings_grid")
                    .num_columns(4)
                    .striped(true)
                    .spacing([10.0, 5.0])
                    .show(ui, |ui| {
                        // Header
                        ui.label(RichText::new("Type").strong());
                        ui.label(RichText::new("Offset").strong());
                        ui.label(RichText::new("Length").strong());
                        ui.label(RichText::new("Value").strong());
                        ui.end_row();

                        // Strings
                        for artefact in filtered.iter().take(500) {
                            ui.label(&artefact.artefact_type);
                            ui.label(format!("0x{:X}", artefact.offset));
                            ui.label(format!("{}", artefact.length));

                            ui.label(truncate_chars(&artefact.value, 80));
                            ui.end_row();
                        }

                        if filtered.len() > 500 {
                            ui.label("...");
                            ui.label(format!("({} more artefacts)", filtered.len() - 500));
                            ui.end_row();
                        }
                    });
            });
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let mut truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        truncated.push_str("...");
    }
    truncated
}

fn format_error_list(errors: &[String]) -> String {
    truncate_chars(&redact_path_like_values(&errors.join("; ")), 240)
}

fn validation_status_label(validated: Option<bool>) -> &'static str {
    match validated {
        Some(true) => "Passed",
        Some(false) => "Failed",
        None => "Not run",
    }
}

fn format_load_error(context: &str, error: &anyhow::Error) -> String {
    tracing::error!(error = ?error, error_chain = %format!("{:#}", error), "{context}");
    format!("{}. See application logs for details.", context)
}

fn redact_path_like_values(value: &str) -> String {
    value
        .split_whitespace()
        .map(|token| {
            let trimmed = token.trim_matches(|c: char| matches!(c, ',' | ';' | ')' | '('));
            if is_path_like_token(trimmed) {
                token.replace(trimmed, "[path]")
            } else {
                token.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_path_like_token(token: &str) -> bool {
    token.starts_with('/')
        || token.starts_with('\\')
        || token.contains(":\\")
        || token.contains(":/")
        || token.contains('\\')
        || token.contains('/')
        || std::path::Path::new(token).is_absolute()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_results_panel_new() {
        let panel = ResultsPanel::new();
        assert!(panel.run_path.is_none());
        assert!(panel.files.is_empty());
        assert!(panel.strings.is_empty());
    }

    #[test]
    fn test_results_panel_clear() {
        let mut panel = ResultsPanel::new();
        panel.files.push(CarvedFile {
            id: 1,
            run_id: None,
            file_type: "jpeg".to_string(),
            path: "test.jpg".to_string(),
            extension: Some("jpg".to_string()),
            global_start: 0,
            global_end: 100,
            size: 100,
            handler_id: None,
            md5: None,
            sha256: None,
            validated: Some(true),
            truncated: false,
            errors: Vec::new(),
            pattern_id: None,
            is_duplicate: false,
            duplicate_of_offset: None,
            mime_type: None,
            width: None,
            height: None,
        });

        panel.clear();
        assert!(panel.files.is_empty());
    }

    #[test]
    fn test_truncate_chars_preserves_utf8_boundaries() {
        assert_eq!(truncate_chars("abcdef", 3), "abc...");
        assert_eq!(truncate_chars("åß∂ƒ©", 3), "åß∂...");
    }

    #[test]
    fn test_format_error_list_redacts_path_like_values() {
        let errors = vec![
            "failed reading /cases/image.dd at C:\\case\\file.dd via \\\\srv\\share\\img.dd and cases/image.dd".to_string(),
        ];
        let formatted = format_error_list(&errors);

        assert!(formatted.contains("[path]"));
        assert!(!formatted.contains("/cases/image.dd"));
        assert!(!formatted.contains("C:\\case\\file.dd"));
        assert!(!formatted.contains("\\\\srv\\share\\img.dd"));
        assert!(!formatted.contains("cases/image.dd"));
    }

    #[test]
    fn test_validation_status_label_distinguishes_missing_validation() {
        assert_eq!(validation_status_label(Some(true)), "Passed");
        assert_eq!(validation_status_label(Some(false)), "Failed");
        assert_eq!(validation_status_label(None), "Not run");
    }
}
