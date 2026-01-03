//! Progress monitoring panel

use egui::{Ui, RichText, Color32, ProgressBar};

use crate::scan::{ScanState, ScanProgress, LogEntry, format_bytes};

/// Progress monitoring panel
pub struct ProgressPanel;

impl Default for ProgressPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressPanel {
    pub fn new() -> Self {
        Self
    }

    /// Render the progress panel
    pub fn show(
        &mut self,
        ui: &mut Ui,
        state: ScanState,
        progress: Option<&ScanProgress>,
        logs: &[LogEntry],
    ) {
        ui.heading("Scan Progress");
        ui.add_space(10.0);

        // Status indicator
        ui.horizontal(|ui| {
            let (status_text, status_color) = match state {
                ScanState::Idle => ("Idle", Color32::GRAY),
                ScanState::Running => ("Running", Color32::GREEN),
                ScanState::Completed => ("Completed", Color32::LIGHT_GREEN),
                ScanState::Failed => ("Failed", Color32::RED),
                ScanState::Cancelled => ("Cancelled", Color32::YELLOW),
            };
            
            ui.label("Status:");
            ui.label(RichText::new(status_text).color(status_color).strong());
        });

        ui.add_space(10.0);

        if let Some(progress) = progress {
            self.show_progress_details(ui, progress);
        } else if state == ScanState::Idle {
            ui.label("Configure and start a scan to see progress.");
        } else if state == ScanState::Running {
            ui.label("Waiting for progress data...");
            ui.spinner();
        }

        ui.add_space(20.0);

        // Log viewer
        self.show_log_viewer(ui, logs);
    }

    fn show_progress_details(&mut self, ui: &mut Ui, progress: &ScanProgress) {
        // Progress bar
        ui.group(|ui| {
            ui.label(RichText::new("Scan Progress").strong());
            
            let pct = progress.percentage() / 100.0;
            ui.add(
                ProgressBar::new(pct)
                    .text(format!("{:.1}%", progress.percentage()))
                    .animate(true)
            );
            
            ui.horizontal(|ui| {
                ui.label(format!(
                    "{} / {}",
                    format_bytes(progress.bytes_scanned),
                    format_bytes(progress.total_bytes)
                ));
            });
        });

        ui.add_space(10.0);

        // Statistics grid
        ui.group(|ui| {
            ui.label(RichText::new("Statistics").strong());
            
            egui::Grid::new("progress_stats")
                .num_columns(2)
                .spacing([40.0, 5.0])
                .show(ui, |ui| {
                    ui.label("Signature Hits:");
                    ui.label(RichText::new(format!("{}", progress.hits)).strong());
                    ui.end_row();
                    
                    ui.label("Files Carved:");
                    ui.label(RichText::new(format!("{}", progress.files)).strong());
                    ui.end_row();
                    
                    ui.label("Throughput:");
                    ui.label(RichText::new(format!("{:.2} MiB/s", progress.rate_mib)).strong());
                    ui.end_row();
                    
                    ui.label("ETA:");
                    ui.label(RichText::new(progress.eta_formatted()).strong());
                    ui.end_row();
                });
        });
    }

    fn show_log_viewer(&mut self, ui: &mut Ui, logs: &[LogEntry]) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Log Output").strong());
                ui.label(format!("({} entries)", logs.len()));
            });
            
            ui.add_space(5.0);
            
            // Scrollable log area
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    if logs.is_empty() {
                        ui.label("No log entries yet.");
                    } else {
                        for entry in logs {
                            let color = match entry.level.as_str() {
                                "ERROR" => Color32::RED,
                                "WARN" | "WARNING" => Color32::YELLOW,
                                "INFO" => Color32::LIGHT_GRAY,
                                "DEBUG" => Color32::DARK_GRAY,
                                _ => Color32::GRAY,
                            };
                            
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(&entry.timestamp)
                                        .small()
                                        .color(Color32::DARK_GRAY)
                                );
                                ui.label(
                                    RichText::new(format!("[{}]", entry.level))
                                        .small()
                                        .color(color)
                                );
                                ui.label(&entry.message);
                            });
                        }
                    }
                });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_percentage() {
        let progress = ScanProgress {
            bytes_scanned: 50_000_000,
            total_bytes: 100_000_000,
            pct: 50.0,
            hits: 100,
            files: 50,
            rate_mib: 150.5,
            eta_secs: Some(300),
        };
        
        assert!((progress.percentage() - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_eta_formatted() {
        let progress = ScanProgress {
            eta_secs: Some(3661),
            ..Default::default()
        };
        
        assert_eq!(progress.eta_formatted(), "1h 1m");
    }

    #[test]
    fn test_eta_formatted_none() {
        let progress = ScanProgress {
            eta_secs: None,
            ..Default::default()
        };
        
        assert_eq!(progress.eta_formatted(), "calculating...");
    }
}
