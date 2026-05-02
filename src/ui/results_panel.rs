//! Results browser panel

use egui::{Color32, RichText, Ui};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use crate::metadata::{
    CarvedFile, MetadataReader, MetadataRecord, MetadataSummary, RunSummary, StringArtefact,
};
use crate::scan::{format_bytes, ScanState};

const LIVE_REFRESH_INTERVAL: Duration = Duration::from_secs(2);

/// Results browser panel
pub struct ResultsPanel {
    /// Current run path
    run_path: Option<String>,
    /// Loaded metadata reader
    reader: Option<MetadataReader>,
    /// Cached summary
    summary: Option<MetadataSummary>,
    /// Cached SwiftBeaver run summary
    run_summary: Option<RunSummary>,
    /// Whether the run summary has been loaded or attempted.
    run_summary_loaded: bool,
    /// Cached files
    files: Vec<CarvedFile>,
    /// Cached string artefacts
    strings: Vec<StringArtefact>,
    /// Cached browser history artefacts
    browser_history: Option<Vec<MetadataRecord>>,
    /// Cached browser cookie artefacts
    browser_cookies: Option<Vec<MetadataRecord>>,
    /// Cached browser download artefacts
    browser_downloads: Option<Vec<MetadataRecord>>,
    /// Cached Windows artefacts
    windows_artefacts: Option<Vec<MetadataRecord>>,
    /// Cached entropy regions
    entropy_regions: Option<Vec<MetadataRecord>>,
    /// Current tab
    current_tab: ResultsTab,
    /// File type filter
    type_filter: Option<String>,
    /// Search query
    search_query: String,
    /// Selected file index
    selected_file: Option<usize>,
    /// Error messages
    errors: Vec<String>,
    /// Whether live metadata polling is enabled during active scans.
    live_refresh_enabled: bool,
    /// Background live refresh receiver, when a refresh is in flight.
    live_refresh_rx: Option<Receiver<LiveRefreshResult>>,
    /// Last live refresh start time, used to bound polling.
    live_refresh_last_started: Option<Instant>,
    /// Non-fatal live refresh status shown in the panel.
    live_refresh_status: Option<String>,
    /// Whether the currently displayed snapshot came from live refresh.
    live_snapshot_active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ResultsTab {
    #[default]
    Overview,
    Files,
    Strings,
    BrowserHistory,
    BrowserCookies,
    BrowserDownloads,
    WindowsArtefacts,
    EntropyRegions,
}

impl ResultsTab {
    fn supports_live_refresh(self) -> bool {
        matches!(self, Self::Overview | Self::Files | Self::Strings)
    }
}

struct LiveSnapshot {
    reader: MetadataReader,
    files: Vec<CarvedFile>,
    strings: Vec<StringArtefact>,
}

struct LiveRefreshResult {
    run_path: String,
    result: anyhow::Result<LiveSnapshot>,
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
            run_summary: None,
            run_summary_loaded: false,
            files: Vec::new(),
            strings: Vec::new(),
            browser_history: None,
            browser_cookies: None,
            browser_downloads: None,
            windows_artefacts: None,
            entropy_regions: None,
            current_tab: ResultsTab::Overview,
            type_filter: None,
            search_query: String::new(),
            selected_file: None,
            errors: Vec::new(),
            live_refresh_enabled: true,
            live_refresh_rx: None,
            live_refresh_last_started: None,
            live_refresh_status: None,
            live_snapshot_active: false,
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
                        self.push_error("Failed to load files", &e);
                        return;
                    }
                };

                let strings = match reader.read_string_artefacts() {
                    Ok(strings) => strings,
                    Err(e) => {
                        self.push_error("Failed to load string artefacts", &e);
                        Vec::new()
                    }
                };

                let summary = match MetadataSummary::from_results(&files, &strings) {
                    Ok(summary) => summary,
                    Err(e) => {
                        self.push_error("Failed to summarize results", &e);
                        return;
                    }
                };

                self.summary = Some(summary);
                self.files = files;
                self.strings = strings;
                self.reader = Some(reader);
            }
            Err(e) => {
                self.push_error("Failed to open results", &e);
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
        self.run_summary = None;
        self.run_summary_loaded = false;
        self.files.clear();
        self.strings.clear();
        self.browser_history = None;
        self.browser_cookies = None;
        self.browser_downloads = None;
        self.windows_artefacts = None;
        self.entropy_regions = None;
        self.type_filter = None;
        self.search_query.clear();
        self.selected_file = None;
        self.errors.clear();
        self.clear_live_runtime_state();
        self.live_snapshot_active = false;
    }

    fn clear_live_runtime_state(&mut self) {
        self.live_refresh_rx = None;
        self.live_refresh_last_started = None;
        self.live_refresh_status = None;
    }

    fn push_error(&mut self, context: &str, error: &anyhow::Error) {
        self.errors.push(format_load_error(context, error));
    }

    fn ensure_run_summary_loaded(&mut self) {
        if self.run_summary_loaded {
            return;
        }
        self.run_summary_loaded = true;
        let result = self
            .reader
            .as_ref()
            .map(MetadataReader::read_run_summary)
            .transpose();
        match result {
            Ok(summary) => self.run_summary = summary.flatten(),
            Err(e) => self.push_error("Failed to load run summary", &e),
        }
    }

    fn ensure_browser_history_loaded(&mut self) {
        if self.browser_history.is_none() {
            self.browser_history = Some(
                self.load_records("Failed to load browser history", |reader| {
                    reader.read_browser_history()
                }),
            );
        }
    }

    fn ensure_browser_cookies_loaded(&mut self) {
        if self.browser_cookies.is_none() {
            self.browser_cookies = Some(
                self.load_records("Failed to load browser cookies", |reader| {
                    reader.read_browser_cookies()
                }),
            );
        }
    }

    fn ensure_browser_downloads_loaded(&mut self) {
        if self.browser_downloads.is_none() {
            self.browser_downloads = Some(
                self.load_records("Failed to load browser downloads", |reader| {
                    reader.read_browser_downloads()
                }),
            );
        }
    }

    fn ensure_windows_artefacts_loaded(&mut self) {
        if self.windows_artefacts.is_none() {
            self.windows_artefacts = Some(
                self.load_records("Failed to load Windows artefacts", |reader| {
                    reader.read_windows_artefacts()
                }),
            );
        }
    }

    fn ensure_entropy_regions_loaded(&mut self) {
        if self.entropy_regions.is_none() {
            self.entropy_regions = Some(
                self.load_records("Failed to load entropy regions", |reader| {
                    reader.read_entropy_regions()
                }),
            );
        }
    }

    fn load_records(
        &mut self,
        context: &str,
        load: impl FnOnce(&MetadataReader) -> anyhow::Result<Vec<MetadataRecord>>,
    ) -> Vec<MetadataRecord> {
        let Some(reader) = self.reader.as_ref() else {
            return Vec::new();
        };

        match load(reader) {
            Ok(records) => records,
            Err(e) => {
                self.push_error(context, &e);
                Vec::new()
            }
        }
    }

    /// Render the results panel
    pub fn show(&mut self, ui: &mut Ui, run_output_path: Option<&str>, scan_state: ScanState) {
        ui.heading("Scan Results");
        ui.add_space(10.0);

        self.poll_live_refresh();

        // Check if we need to load new results
        if let Some(path) = run_output_path {
            if self.run_path.as_deref() != Some(path) {
                if scan_state == ScanState::Running {
                    self.prepare_live_run(path);
                } else {
                    self.load(path);
                }
            } else if scan_state != ScanState::Running && self.live_snapshot_active {
                self.load(path);
            }
        }

        if scan_state == ScanState::Running {
            if let Some(path) = run_output_path {
                self.show_live_refresh_controls(ui, path);
                self.maybe_start_live_refresh(path);
                ui.add_space(10.0);
            }
        }

        // Show errors if any
        for error in &self.errors {
            ui.colored_label(Color32::RED, format!("Error: {}", error));
            ui.add_space(10.0);
        }

        // No results loaded
        if self.reader.is_none() {
            if scan_state == ScanState::Running && run_output_path.is_some() {
                ui.label("Waiting for SwiftBeaver metadata in the reported run directory.");
            } else {
                ui.label("No results loaded. Run a scan or load previous results.");
            }

            ui.add_space(10.0);

            if scan_state != ScanState::Running && ui.button("Load Previous Results...").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.load(&path.display().to_string());
                }
            }

            return;
        }

        // Tab bar
        let previous_tab = self.current_tab;
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
            ui.selectable_value(
                &mut self.current_tab,
                ResultsTab::BrowserHistory,
                tab_label("History", self.browser_history.as_ref()),
            );
            ui.selectable_value(
                &mut self.current_tab,
                ResultsTab::BrowserCookies,
                tab_label("Cookies", self.browser_cookies.as_ref()),
            );
            ui.selectable_value(
                &mut self.current_tab,
                ResultsTab::BrowserDownloads,
                tab_label("Downloads", self.browser_downloads.as_ref()),
            );
            ui.selectable_value(
                &mut self.current_tab,
                ResultsTab::WindowsArtefacts,
                tab_label("Windows", self.windows_artefacts.as_ref()),
            );
            ui.selectable_value(
                &mut self.current_tab,
                ResultsTab::EntropyRegions,
                tab_label("Entropy", self.entropy_regions.as_ref()),
            );
        });
        if self.current_tab != previous_tab {
            self.search_query.clear();
            self.type_filter = None;
        }

        ui.separator();

        match self.current_tab {
            ResultsTab::Overview => self.show_overview(ui),
            ResultsTab::Files => self.show_files(ui),
            ResultsTab::Strings => self.show_strings(ui),
            ResultsTab::BrowserHistory => self.show_browser_history(ui),
            ResultsTab::BrowserCookies => self.show_browser_cookies(ui),
            ResultsTab::BrowserDownloads => self.show_browser_downloads(ui),
            ResultsTab::WindowsArtefacts => self.show_windows_artefacts(ui),
            ResultsTab::EntropyRegions => self.show_entropy_regions(ui),
        }
    }

    fn prepare_live_run(&mut self, run_path: &str) {
        let previous_tab = self.current_tab;
        self.clear_loaded_state();
        self.run_path = Some(run_path.to_string());
        if !previous_tab.supports_live_refresh() {
            self.current_tab = ResultsTab::Files;
        }
        self.live_refresh_status = Some("Waiting for live metadata...".to_string());
        self.live_snapshot_active = true;
    }

    fn show_live_refresh_controls(&mut self, ui: &mut Ui, run_path: &str) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.live_refresh_enabled, "Live refresh");

            let refresh_in_flight = self.live_refresh_rx.is_some();
            if ui
                .add_enabled(
                    self.live_refresh_enabled && !refresh_in_flight,
                    egui::Button::new("Refresh now"),
                )
                .clicked()
            {
                self.start_live_refresh(run_path);
            }

            if refresh_in_flight {
                ui.spinner();
                ui.label("Refreshing...");
            } else if let Some(status) = &self.live_refresh_status {
                ui.label(status);
            }
        });
    }

    fn maybe_start_live_refresh(&mut self, run_path: &str) {
        if !self.live_refresh_enabled || self.live_refresh_rx.is_some() {
            return;
        }

        let now = Instant::now();
        if self
            .live_refresh_last_started
            .is_some_and(|last_started| now.duration_since(last_started) < LIVE_REFRESH_INTERVAL)
        {
            return;
        }

        self.start_live_refresh(run_path);
    }

    fn start_live_refresh(&mut self, run_path: &str) {
        if self.live_refresh_rx.is_some() {
            return;
        }

        let run_path = run_path.to_string();
        let reader = self.reader.clone();
        let (tx, rx) = channel();
        self.live_refresh_rx = Some(rx);
        self.live_refresh_last_started = Some(Instant::now());

        std::thread::spawn(move || {
            let result = reader
                .map_or_else(|| MetadataReader::new(&run_path), Ok)
                .and_then(|reader| {
                    let files = reader.read_live_carved_files()?;
                    let strings = reader.read_live_string_artefacts()?;
                    Ok(LiveSnapshot {
                        reader,
                        files,
                        strings,
                    })
                });
            let _ = tx.send(LiveRefreshResult { run_path, result });
        });
    }

    fn poll_live_refresh(&mut self) {
        let Some(received) = self
            .live_refresh_rx
            .as_ref()
            .and_then(|rx| match rx.try_recv() {
                Ok(result) => Some(Ok(result)),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(Err(())),
            })
        else {
            return;
        };

        self.live_refresh_rx = None;

        match received {
            Ok(result) => self.apply_live_refresh_result(result),
            Err(()) => {
                self.live_refresh_status = Some("Live refresh worker stopped.".to_string());
            }
        }
    }

    fn apply_live_refresh_result(&mut self, refresh: LiveRefreshResult) {
        if self.run_path.as_deref() != Some(refresh.run_path.as_str()) {
            return;
        }

        match refresh.result {
            Ok(snapshot) => {
                self.reader = Some(snapshot.reader);
                self.live_snapshot_active = true;
                let previous_len = self.files.len();
                self.replace_files_preserving_selection(snapshot.files);
                self.strings = snapshot.strings;

                let added = self.files.len().saturating_sub(previous_len);
                let mut status = if added > 0 {
                    format!("Live results: {} files (+{})", self.files.len(), added)
                } else {
                    format!("Live results: {} files", self.files.len())
                };

                if let Err(error) = self.refresh_summary_from_loaded_results() {
                    tracing::debug!(
                        error = ?error,
                        "Live metadata refresh kept previous summary after recompute failure"
                    );
                    status.push_str("; summary unchanged");
                }

                self.live_refresh_status = Some(status);
            }
            Err(error) => {
                tracing::debug!(
                    error = ?error,
                    "Live metadata refresh did not produce a usable snapshot"
                );
                let error_chain = format!("{:#}", error);
                self.live_refresh_status = Some(
                    if error_chain.contains(MetadataReader::LIVE_REFRESH_UNSUPPORTED_BACKEND) {
                        MetadataReader::LIVE_REFRESH_UNSUPPORTED_BACKEND.to_string()
                    } else if self.reader.is_some() {
                        "Live refresh skipped while metadata is being written.".to_string()
                    } else {
                        "Waiting for live metadata...".to_string()
                    },
                );
            }
        }
    }

    fn refresh_summary_from_loaded_results(&mut self) -> anyhow::Result<()> {
        self.summary = Some(MetadataSummary::from_results(&self.files, &self.strings)?);
        Ok(())
    }

    fn replace_files_preserving_selection(&mut self, files: Vec<CarvedFile>) {
        let selected_key = self
            .selected_file
            .and_then(|idx| self.files.get(idx))
            .map(carved_file_key);

        self.files = files;

        self.selected_file = selected_key.and_then(|key| {
            self.files
                .iter()
                .position(|file| carved_file_key(file) == key)
        });
    }

    fn show_browser_history(&mut self, ui: &mut Ui) {
        self.ensure_browser_history_loaded();
        show_records_table(
            ui,
            &mut self.search_query,
            self.browser_history.as_deref().unwrap_or(&[]),
            "No browser history artefacts found.",
            "browser_history_grid",
            &[
                "url",
                "title",
                "visit_time",
                "last_visit_time",
                "visit_count",
                "source",
            ],
        );
    }

    fn show_browser_cookies(&mut self, ui: &mut Ui) {
        self.ensure_browser_cookies_loaded();
        show_records_table(
            ui,
            &mut self.search_query,
            self.browser_cookies.as_deref().unwrap_or(&[]),
            "No browser cookie artefacts found.",
            "browser_cookies_grid",
            &[
                "host", "domain", "name", "value", "path", "expires", "source",
            ],
        );
    }

    fn show_browser_downloads(&mut self, ui: &mut Ui) {
        self.ensure_browser_downloads_loaded();
        show_records_table(
            ui,
            &mut self.search_query,
            self.browser_downloads.as_deref().unwrap_or(&[]),
            "No browser download artefacts found.",
            "browser_downloads_grid",
            &[
                "url",
                "target_path",
                "path",
                "start_time",
                "end_time",
                "source",
            ],
        );
    }

    fn show_windows_artefacts(&mut self, ui: &mut Ui) {
        self.ensure_windows_artefacts_loaded();
        show_records_table(
            ui,
            &mut self.search_query,
            self.windows_artefacts.as_deref().unwrap_or(&[]),
            "No Windows artefacts found.",
            "windows_artefacts_grid",
            &[
                "artefact_kind",
                "path",
                "target_path",
                "timestamp",
                "source",
            ],
        );
    }

    fn show_entropy_regions(&mut self, ui: &mut Ui) {
        self.ensure_entropy_regions_loaded();
        show_records_table(
            ui,
            &mut self.search_query,
            self.entropy_regions.as_deref().unwrap_or(&[]),
            "No entropy regions found. Enable entropy scanning in configuration.",
            "entropy_regions_grid",
            &["global_start", "global_end", "length", "entropy", "source"],
        );
    }

    fn show_overview(&mut self, ui: &mut Ui) {
        self.ensure_run_summary_loaded();

        if let Some(summary) = &self.summary {
            if let Some(run_summary) = &self.run_summary {
                ui.group(|ui| {
                    ui.label(RichText::new("Run Summary").strong());

                    egui::Grid::new("run_summary_grid")
                        .num_columns(2)
                        .spacing([40.0, 5.0])
                        .show(ui, |ui| {
                            summary_metric_row(
                                ui,
                                "Bytes Scanned:",
                                run_summary.bytes_scanned.map(format_bytes),
                            );
                            summary_metric_row(
                                ui,
                                "Chunks Processed:",
                                run_summary.chunks_processed.map(|value| value.to_string()),
                            );
                            summary_metric_row(
                                ui,
                                "Hits:",
                                run_summary.hits.map(|value| value.to_string()),
                            );
                            summary_metric_row(
                                ui,
                                "Files Carved:",
                                run_summary.files_carved.map(|value| value.to_string()),
                            );
                            summary_metric_row(
                                ui,
                                "Rejected:",
                                run_summary.rejected.map(|value| value.to_string()),
                            );
                            summary_metric_row(
                                ui,
                                "Prevalidation Rejected:",
                                run_summary
                                    .prevalidation_rejected
                                    .map(|value| value.to_string()),
                            );
                            summary_metric_row(
                                ui,
                                "Overlap Skipped:",
                                run_summary.overlap_skipped.map(|value| value.to_string()),
                            );
                            summary_metric_row(
                                ui,
                                "String Spans:",
                                run_summary.string_spans.map(|value| value.to_string()),
                            );
                            summary_metric_row(
                                ui,
                                "Artefacts Extracted:",
                                run_summary
                                    .artefacts_extracted
                                    .map(|value| value.to_string()),
                            );
                            summary_metric_row(
                                ui,
                                "Duplicates Found:",
                                run_summary.duplicates_found.map(|value| value.to_string()),
                            );
                            summary_metric_row(
                                ui,
                                "Duplicates Skipped:",
                                run_summary
                                    .duplicates_skipped
                                    .map(|value| value.to_string()),
                            );
                        });
                });

                ui.add_space(10.0);
            }

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
                        .content
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
                    .num_columns(7)
                    .striped(true)
                    .spacing([10.0, 5.0])
                    .show(ui, |ui| {
                        // Header
                        ui.label(RichText::new("Artefact Kind").strong());
                        ui.label(RichText::new("Global Start").strong());
                        ui.label(RichText::new("Global End").strong());
                        ui.label(RichText::new("Length").strong());
                        ui.label(RichText::new("Encoding").strong());
                        ui.label(RichText::new("Source").strong());
                        ui.label(RichText::new("Content").strong());
                        ui.end_row();

                        // Strings
                        for artefact in filtered.iter().take(500) {
                            ui.label(&artefact.artefact_kind);
                            ui.label(format!("0x{:X}", artefact.global_start));
                            ui.label(
                                artefact
                                    .global_end
                                    .map(|end| format!("0x{end:X}"))
                                    .unwrap_or_default(),
                            );
                            ui.label(format!("{}", artefact.length));
                            ui.label(artefact.encoding.as_deref().unwrap_or(""));
                            ui.label(
                                artefact
                                    .source
                                    .as_deref()
                                    .map(|source| truncate_chars(source, 40))
                                    .unwrap_or_default(),
                            );
                            ui.label(truncate_chars(&artefact.content, 80));
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

fn carved_file_key(file: &CarvedFile) -> (u64, u64, u64, String) {
    (file.id, file.global_start, file.size, file.path.clone())
}

fn summary_metric_row(ui: &mut Ui, label: &str, value: Option<String>) {
    ui.label(label);
    ui.label(RichText::new(value.unwrap_or_else(|| "Not reported".to_string())).strong());
    ui.end_row();
}

fn tab_label(label: &str, records: Option<&Vec<MetadataRecord>>) -> String {
    records
        .map(|records| format!("{label} ({})", records.len()))
        .unwrap_or_else(|| label.to_string())
}

fn show_records_table(
    ui: &mut Ui,
    search_query: &mut String,
    records: &[MetadataRecord],
    empty_message: &str,
    grid_id: &'static str,
    preferred_columns: &[&str],
) {
    if records.is_empty() {
        ui.label(empty_message);
        return;
    }

    ui.horizontal(|ui| {
        ui.label("Filter:");
        ui.text_edit_singleline(search_query);
        if ui.button("Clear").clicked() {
            search_query.clear();
        }
    });

    ui.separator();

    let query = search_query.to_lowercase();
    let filtered: Vec<_> = records
        .iter()
        .filter(|record| {
            query.is_empty()
                || record
                    .fields
                    .values()
                    .any(|value| value.to_lowercase().contains(&query))
        })
        .collect();
    let columns = record_columns(records, preferred_columns);

    ui.label(format!(
        "Showing {} of {} rows",
        filtered.len(),
        records.len()
    ));

    egui::ScrollArea::vertical()
        .max_height(400.0)
        .show(ui, |ui| {
            egui::Grid::new(grid_id)
                .num_columns(columns.len().max(1))
                .striped(true)
                .spacing([10.0, 5.0])
                .show(ui, |ui| {
                    for column in &columns {
                        ui.label(RichText::new(column_label(column)).strong());
                    }
                    ui.end_row();

                    for record in filtered.iter().take(500) {
                        for column in &columns {
                            let value = record
                                .fields
                                .get(column)
                                .map(|value| truncate_chars(value, 80))
                                .unwrap_or_default();
                            ui.label(value);
                        }
                        ui.end_row();
                    }

                    if filtered.len() > 500 {
                        ui.label("...");
                        ui.label(format!("({} more rows)", filtered.len() - 500));
                        ui.end_row();
                    }
                });
        });
}

fn record_columns(records: &[MetadataRecord], preferred_columns: &[&str]) -> Vec<String> {
    let mut columns = Vec::new();

    for column in preferred_columns {
        if records
            .iter()
            .any(|record| record.fields.contains_key(*column))
        {
            columns.push((*column).to_string());
        }
    }

    for record in records.iter().take(100) {
        for column in record.fields.keys() {
            if !columns.iter().any(|existing| existing == column) {
                columns.push(column.clone());
            }
            if columns.len() >= 10 {
                return columns;
            }
        }
    }

    columns
}

fn column_label(column: &str) -> String {
    column
        .split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut label = first.to_uppercase().to_string();
                    label.push_str(chars.as_str());
                    label
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
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

    fn sample_file(id: u64, path: &str) -> CarvedFile {
        CarvedFile {
            id,
            run_id: None,
            file_type: "jpeg".to_string(),
            path: path.to_string(),
            extension: Some("jpg".to_string()),
            global_start: id * 100,
            global_end: id * 100 + 100,
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
        }
    }

    fn sample_string(content: &str) -> StringArtefact {
        StringArtefact {
            artefact_kind: "email".to_string(),
            content: content.to_string(),
            global_start: 10,
            global_end: Some(20),
            length: 10,
            encoding: Some("utf8".to_string()),
            source: None,
            run_id: None,
        }
    }

    fn temp_jsonl_reader() -> (tempfile::TempDir, String, MetadataReader) {
        let temp = tempfile::TempDir::new().unwrap();
        let metadata_dir = temp.path().join("metadata");
        std::fs::create_dir(&metadata_dir).unwrap();
        std::fs::write(metadata_dir.join("carved_files.jsonl"), "").unwrap();
        let run_path = temp.path().display().to_string();
        let reader = MetadataReader::new(temp.path()).unwrap();
        (temp, run_path, reader)
    }

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
        panel.files.push(sample_file(1, "test.jpg"));

        panel.clear();
        assert!(panel.files.is_empty());
    }

    #[test]
    fn test_live_refresh_preserves_selected_file_by_metadata_key() {
        let mut panel = ResultsPanel::new();
        panel.files = vec![sample_file(1, "one.jpg"), sample_file(2, "two.jpg")];
        panel.selected_file = Some(1);

        panel.replace_files_preserving_selection(vec![
            sample_file(3, "new.jpg"),
            sample_file(1, "one.jpg"),
            sample_file(2, "two.jpg"),
        ]);

        assert_eq!(panel.selected_file, Some(2));
        assert_eq!(panel.files[2].path, "two.jpg");
    }

    #[test]
    fn test_live_refresh_clears_selection_when_file_disappears() {
        let mut panel = ResultsPanel::new();
        panel.files = vec![sample_file(1, "one.jpg"), sample_file(2, "two.jpg")];
        panel.selected_file = Some(1);

        panel.replace_files_preserving_selection(vec![sample_file(1, "one.jpg")]);

        assert_eq!(panel.selected_file, None);
    }

    #[test]
    fn test_live_refresh_updates_strings_and_summary() {
        let (_temp, run_path, reader) = temp_jsonl_reader();
        let mut panel = ResultsPanel::new();
        panel.run_path = Some(run_path.clone());

        panel.apply_live_refresh_result(LiveRefreshResult {
            run_path,
            result: Ok(LiveSnapshot {
                reader,
                files: vec![sample_file(1, "one.jpg")],
                strings: vec![sample_string("a@example.test")],
            }),
        });

        assert_eq!(panel.files.len(), 1);
        assert_eq!(panel.strings.len(), 1);
        assert_eq!(panel.summary.as_ref().unwrap().string_artefacts, 1);
    }

    #[test]
    fn test_live_refresh_preserves_summary_on_recompute_error() {
        let (_temp, run_path, reader) = temp_jsonl_reader();
        let mut panel = ResultsPanel::new();
        panel.run_path = Some(run_path.clone());
        panel.summary =
            Some(MetadataSummary::from_results(&[sample_file(1, "old.jpg")], &[]).unwrap());

        let mut huge_file = sample_file(1, "huge.jpg");
        huge_file.size = u64::MAX;
        let mut extra_file = sample_file(2, "extra.jpg");
        extra_file.size = 1;

        panel.apply_live_refresh_result(LiveRefreshResult {
            run_path,
            result: Ok(LiveSnapshot {
                reader,
                files: vec![huge_file, extra_file],
                strings: Vec::new(),
            }),
        });

        let summary = panel.summary.as_ref().unwrap();
        assert_eq!(summary.total_files, 1);
        assert!(panel
            .live_refresh_status
            .as_deref()
            .unwrap()
            .contains("summary unchanged"));
    }

    #[test]
    fn test_prepare_live_run_preserves_live_backed_tabs() {
        let mut panel = ResultsPanel::new();
        panel.current_tab = ResultsTab::Overview;
        panel.prepare_live_run("/tmp/run-one");
        assert_eq!(panel.current_tab, ResultsTab::Overview);

        panel.current_tab = ResultsTab::BrowserHistory;
        panel.prepare_live_run("/tmp/run-two");
        assert_eq!(panel.current_tab, ResultsTab::Files);
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
