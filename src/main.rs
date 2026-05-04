//! SwiftBeaverLodge - GUI Frontend for SwiftBeaver Forensic File Carver
//!
//! A pure Rust desktop application using egui.

mod app;
mod config;
mod devices;
mod metadata;
mod scan;
mod ui;

use eframe::egui;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if matches!(args.as_slice(), [arg] if arg == "--version" || arg == "-V") {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if !args.is_empty() {
        eprintln!("unsupported arguments: {}", args.join(" "));
        std::process::exit(2);
    }

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
