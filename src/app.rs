//! Application state and main loop

use eframe::egui;
use std::sync::{Arc, Mutex};

use crate::config::ScanConfig;
use crate::scan::{ScanManager, ScanState};
use crate::ui::{ConfigPanel, ProgressPanel, ResultsPanel, Tab};

/// Main application state
pub struct SwiftBeaverApp {
    /// Current active tab
    current_tab: Tab,
    
    /// Scan configuration
    config: ScanConfig,
    
    /// Scan manager (shared for async operations)
    scan_manager: Arc<Mutex<ScanManager>>,
    
    /// UI panels
    config_panel: ConfigPanel,
    progress_panel: ProgressPanel,
    results_panel: ResultsPanel,
    
    /// Last scan output path (for results loading)
    last_run_path: Option<String>,
    
    /// Status message
    status_message: String,
}

impl SwiftBeaverApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            current_tab: Tab::Configure,
            config: ScanConfig::default(),
            scan_manager: Arc::new(Mutex::new(ScanManager::new())),
            config_panel: ConfigPanel::new(),
            progress_panel: ProgressPanel::new(),
            results_panel: ResultsPanel::new(),
            last_run_path: None,
            status_message: "Ready".to_string(),
        }
    }
    
    /// Start a scan with current configuration
    fn start_scan(&mut self) {
        let config = self.config.clone();
        
        // Start scan directly - the new manager spawns its own thread
        let mut mgr = self.scan_manager.lock().unwrap();
        if let Err(e) = mgr.start(config) {
            tracing::error!("Failed to start scan: {}", e);
            self.status_message = format!("Error: {}", e);
            return;
        }
        
        self.current_tab = Tab::Monitor;
        self.status_message = "Scan started...".to_string();
    }
    
    /// Stop the current scan
    fn stop_scan(&mut self) {
        let mut mgr = self.scan_manager.lock().unwrap();
        if let Err(e) = mgr.stop() {
            tracing::error!("Failed to stop scan: {}", e);
        }
        
        self.status_message = "Stopping scan...".to_string();
    }
}

impl eframe::App for SwiftBeaverApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll scan manager for updates (non-blocking!)
        {
            let mut mgr = self.scan_manager.lock().unwrap();
            mgr.poll();  // Process any pending messages from scan thread
        }
        
        // Now get current state
        let (state, progress, run_path, logs) = {
            let mgr = self.scan_manager.lock().unwrap();
            (
                mgr.state(),
                mgr.progress().cloned(),
                mgr.run_output_path().map(|p| p.to_string()),
                mgr.logs().to_vec(),
            )
        };
        
        // Update last run path
        if run_path.is_some() {
            self.last_run_path = run_path.clone();
        }
        
        // Auto-switch to results when complete
        if state == ScanState::Completed && self.current_tab == Tab::Monitor {
            self.current_tab = Tab::Results;
            self.status_message = "Scan completed!".to_string();
        }
        
        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open Evidence...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Disk Images", &["dd", "raw", "img", "E01", "e01"])
                            .add_filter("All Files", &["*"])
                            .pick_file()
                        {
                            self.config.input_path = path.display().to_string();
                        }
                        ui.close_menu();
                    }
                    if ui.button("Set Output Directory...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.config.output_path = path.display().to_string();
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                
                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        // Show about dialog
                        ui.close_menu();
                    }
                });
            });
        });
        
        // Bottom status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_message);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let version = crate::scan::get_swiftbeaver_version();
                    ui.label(format!("swiftbeaver: {}", version));
                    ui.separator();
                    ui.label(format!("SwiftBeaverLodge v{}", env!("CARGO_PKG_VERSION")));
                });
            });
        });
        
        // Left sidebar with tabs
        egui::SidePanel::left("sidebar").resizable(false).exact_width(180.0).show(ctx, |ui| {
            ui.add_space(10.0);
            ui.heading("SwiftBeaverLodge");
            ui.add_space(20.0);
            
            ui.vertical(|ui| {
                let tab_button = |ui: &mut egui::Ui, label: &str, tab: Tab, current: &mut Tab| {
                    let selected = *current == tab;
                    if ui.selectable_label(selected, format!("  {}  ", label)).clicked() {
                        *current = tab;
                    }
                };
                
                tab_button(ui, "⚙ Configure", Tab::Configure, &mut self.current_tab);
                tab_button(ui, "📊 Monitor", Tab::Monitor, &mut self.current_tab);
                tab_button(ui, "📁 Results", Tab::Results, &mut self.current_tab);
            });
            
            ui.add_space(20.0);
            ui.separator();
            
            // Scan control buttons
            ui.add_space(10.0);
            match state {
                ScanState::Idle | ScanState::Completed | ScanState::Failed | ScanState::Cancelled => {
                    let can_start = !self.config.input_path.is_empty() 
                        && !self.config.output_path.is_empty();
                    
                    if ui.add_enabled(can_start, egui::Button::new("▶ Start Scan").min_size(egui::vec2(160.0, 30.0))).clicked() {
                        self.start_scan();
                    }
                }
                ScanState::Running => {
                    if ui.add(egui::Button::new("⏹ Stop Scan").min_size(egui::vec2(160.0, 30.0))).clicked() {
                        self.stop_scan();
                    }
                }
            }
            
            // State indicator
            ui.add_space(10.0);
            let state_text = match state {
                ScanState::Idle => "● Idle",
                ScanState::Running => "● Running",
                ScanState::Completed => "● Completed",
                ScanState::Failed => "● Failed",
                ScanState::Cancelled => "● Cancelled",
            };
            let state_color = match state {
                ScanState::Idle => egui::Color32::GRAY,
                ScanState::Running => egui::Color32::YELLOW,
                ScanState::Completed => egui::Color32::GREEN,
                ScanState::Failed => egui::Color32::RED,
                ScanState::Cancelled => egui::Color32::LIGHT_RED,
            };
            ui.colored_label(state_color, state_text);
        });
        
        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_tab {
                Tab::Configure => {
                    self.config_panel.show(ui, &mut self.config);
                }
                Tab::Monitor => {
                    self.progress_panel.show(ui, state, progress.as_ref(), &logs);
                }
                Tab::Results => {
                    self.results_panel.show(ui, self.last_run_path.as_deref());
                }
            }
        });
        
        // Request repaint while scanning
        if state == ScanState::Running {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}
