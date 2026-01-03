//! Results browser panel

use egui::{Ui, RichText, Color32};

use crate::metadata::{MetadataReader, CarvedFile, StringArtefact, MetadataSummary};
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
        self.run_path = Some(run_path.to_string());
        self.error = None;
        
        match MetadataReader::new(run_path) {
            Ok(reader) => {
                // Load summary
                match reader.get_summary() {
                    Ok(summary) => self.summary = Some(summary),
                    Err(e) => self.error = Some(format!("Failed to load summary: {}", e)),
                }
                
                // Load files
                match reader.read_carved_files() {
                    Ok(files) => self.files = files,
                    Err(e) => self.error = Some(format!("Failed to load files: {}", e)),
                }
                
                // Load strings
                match reader.read_string_artefacts() {
                    Ok(strings) => self.strings = strings,
                    Err(e) => {
                        // Not an error if no strings
                        tracing::debug!("No string artefacts: {}", e);
                    }
                }
                
                self.reader = Some(reader);
            }
            Err(e) => {
                self.error = Some(format!("Failed to open results: {}", e));
            }
        }
    }

    /// Clear loaded results
    pub fn clear(&mut self) {
        self.run_path = None;
        self.reader = None;
        self.summary = None;
        self.files.clear();
        self.strings.clear();
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
            ui.selectable_value(&mut self.current_tab, ResultsTab::Files, format!("Files ({})", self.files.len()));
            ui.selectable_value(&mut self.current_tab, ResultsTab::Strings, format!("Strings ({})", self.strings.len()));
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
            if ui.selectable_label(self.type_filter.is_none(), "All").clicked() {
                self.type_filter = None;
            }
            
            // Get unique types
            let mut types: Vec<_> = self.files.iter()
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
        let filtered: Vec<_> = self.files.iter()
            .enumerate()
            .filter(|(_, f)| {
                if let Some(ref type_filter) = self.type_filter {
                    if &f.file_type != type_filter {
                        return false;
                    }
                }
                if !self.search_query.is_empty() {
                    if !f.output_path.to_lowercase().contains(&self.search_query.to_lowercase()) {
                        return false;
                    }
                }
                true
            })
            .collect();

        ui.label(format!("Showing {} of {} files", filtered.len(), self.files.len()));

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
                            
                            if ui.selectable_label(selected, format!("{}", file.id)).clicked() {
                                self.selected_file = Some(*idx);
                            }
                            ui.label(&file.file_type);
                            ui.label(format!("0x{:X}", file.offset));
                            ui.label(format_bytes(file.size));
                            ui.label(&file.output_path);
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
                            ui.label(format!("0x{:X} ({})", file.offset, file.offset));
                            ui.end_row();
                            
                            ui.label("Size:");
                            ui.label(format_bytes(file.size));
                            ui.end_row();
                            
                            ui.label("Path:");
                            ui.label(&file.output_path);
                            ui.end_row();
                            
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
                            
                            ui.label("Valid:");
                            ui.label(if file.is_valid { "Yes" } else { "No" });
                            ui.end_row();
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
        let filtered: Vec<_> = self.strings.iter()
            .filter(|s| {
                if !self.search_query.is_empty() {
                    return s.value.to_lowercase().contains(&self.search_query.to_lowercase());
                }
                true
            })
            .collect();

        ui.label(format!("Showing {} of {} artefacts", filtered.len(), self.strings.len()));

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
                            
                            // Truncate long values
                            let display_value = if artefact.value.len() > 80 {
                                format!("{}...", &artefact.value[..80])
                            } else {
                                artefact.value.clone()
                            };
                            ui.label(display_value);
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
            file_type: "jpeg".to_string(),
            offset: 0,
            size: 100,
            output_path: "test.jpg".to_string(),
            sha256: None,
            is_valid: true,
            mime_type: None,
            width: None,
            height: None,
        });
        
        panel.clear();
        assert!(panel.files.is_empty());
    }
}
