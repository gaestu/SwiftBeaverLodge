//! UI components and panels

mod config_panel;
mod progress_panel;
mod results_panel;

pub use config_panel::ConfigPanel;
pub use progress_panel::ProgressPanel;
pub use results_panel::ResultsPanel;

/// Application tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Configure,
    Monitor,
    Results,
}

impl Tab {
    pub fn label(&self) -> &'static str {
        match self {
            Tab::Configure => "⚙ Configure",
            Tab::Monitor => "📊 Monitor",
            Tab::Results => "📁 Results",
        }
    }
}
