//! SwiftBeaverLodge - GUI Frontend for SwiftBeaver Forensic File Carver
//!
//! A pure Rust desktop application using egui.

mod app;
mod config;
mod scan;
mod metadata;
mod ui;

use eframe::egui;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn main() -> eframe::Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive("swiftbeaverlodge=info".parse().unwrap()))
        .init();

    tracing::info!("Starting SwiftBeaverLodge");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([1024.0, 768.0])
            .with_title("SwiftBeaverLodge - Forensic File Carver"),
        ..Default::default()
    };

    eframe::run_native(
        "SwiftBeaverLodge",
        options,
        Box::new(|cc| Ok(Box::new(app::SwiftBeaverApp::new(cc)))),
    )
}
