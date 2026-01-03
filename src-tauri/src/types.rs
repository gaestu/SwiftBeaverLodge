//! SwiftBeaverLodge - Rust Backend Types
//! 
//! Type definitions for GUI ↔ SwiftBeaver integration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Scan Configuration (Frontend → Backend)
// =============================================================================

/// Configuration for a scan operation, received from the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiScanConfig {
    // Input/Output
    pub input_path: String,
    pub output_path: String,
    pub compute_evidence_hash: bool,
    pub evidence_sha256: Option<String>,

    // File types to carve
    pub file_types: Vec<String>,
    pub disable_zip: bool,

    // String scanning options
    pub scan_strings: bool,
    pub scan_urls: bool,
    pub scan_emails: bool,
    pub scan_phones: bool,
    pub scan_utf16: bool,
    pub string_min_len: usize,
    pub string_max_len: usize,

    // GPU acceleration
    pub gpu_enabled: bool,
    pub gpu_backend: Option<String>, // "opencl" | "cuda"

    // Entropy detection
    pub scan_entropy: bool,
    pub entropy_window_bytes: usize,
    pub entropy_threshold: f64,

    // SQLite recovery
    pub scan_sqlite_pages: bool,

    // Resource limits
    pub max_bytes: Option<u64>,
    pub max_chunks: Option<u64>,
    pub max_files: Option<u64>,
    pub max_memory_mib: Option<u64>,

    // Output settings
    pub overlap_kib: u64,
    pub metadata_backend: String, // "jsonl" | "csv" | "parquet"
    pub workers: usize,
    pub chunk_size_mib: u64,
}

impl Default for GuiScanConfig {
    fn default() -> Self {
        Self {
            input_path: String::new(),
            output_path: String::new(),
            compute_evidence_hash: true,
            evidence_sha256: None,
            file_types: vec![
                "jpeg".into(), "png".into(), "gif".into(), "pdf".into(),
                "zip".into(), "sqlite".into(), "docx".into(), "xlsx".into(), "pptx".into(),
            ],
            disable_zip: false,
            scan_strings: true,
            scan_urls: true,
            scan_emails: true,
            scan_phones: true,
            scan_utf16: false,
            string_min_len: 8,
            string_max_len: 4096,
            gpu_enabled: false,
            gpu_backend: None,
            scan_entropy: false,
            entropy_window_bytes: 256,
            entropy_threshold: 7.5,
            scan_sqlite_pages: false,
            max_bytes: None,
            max_chunks: None,
            max_files: None,
            max_memory_mib: None,
            overlap_kib: 4,
            metadata_backend: "parquet".into(), // Default to Parquet for better performance
            workers: 0,
            chunk_size_mib: 64,
        }
    }
}

// =============================================================================
// Scan Status & Progress (Backend → Frontend)
// =============================================================================

/// Current state of a scan
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScanState {
    Idle,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// Progress snapshot sent to frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiScanProgress {
    pub bytes_scanned: u64,
    pub total_bytes: u64,
    pub chunks_processed: u64,
    pub hits_found: u64,
    pub files_carved: u64,
    pub string_spans: u64,
    pub artefacts_extracted: u64,
    pub carve_errors: u64,
    pub metadata_errors: u64,
    pub sqlite_errors: u64,
    pub elapsed_seconds: f64,
    pub throughput_mib: f64,
    pub eta_seconds: Option<u64>,
}

/// Scan status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanStatus {
    pub state: ScanState,
    pub run_id: Option<String>,
    pub started_at: Option<String>,
    pub progress: Option<GuiScanProgress>,
    pub error: Option<String>,
}

impl Default for ScanStatus {
    fn default() -> Self {
        Self {
            state: ScanState::Idle,
            run_id: None,
            started_at: None,
            progress: None,
            error: None,
        }
    }
}

/// Handle returned when starting a scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanHandle {
    pub run_id: String,
}

// =============================================================================
// Carved Files & Artefacts
// =============================================================================

/// Carved file info sent to frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiCarvedFile {
    pub run_id: String,
    pub file_type: String,
    pub path: String,
    pub extension: String,
    pub global_start: u64,
    pub global_end: u64,
    pub size: u64,
    pub md5: Option<String>,
    pub sha256: Option<String>,
    pub validated: bool,
    pub truncated: bool,
    pub errors: Vec<String>,
    pub pattern_id: Option<String>,
}

/// String artefact info sent to frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiStringArtefact {
    pub run_id: String,
    pub artefact_kind: String, // "url" | "email" | "phone" | "string"
    pub content: String,
    pub encoding: String,
    pub global_start: u64,
    pub global_end: u64,
}

/// Browser history record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiBrowserHistory {
    pub run_id: String,
    pub source_file: String,
    pub browser: String,
    pub url: String,
    pub title: Option<String>,
    pub visit_count: Option<i64>,
    pub last_visit_time: Option<String>,
}

/// Run summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiRunSummary {
    pub run_id: String,
    pub bytes_scanned: u64,
    pub chunks_processed: u64,
    pub hits_found: u64,
    pub files_carved: u64,
    pub string_spans: u64,
    pub artefacts_extracted: u64,
}

// =============================================================================
// System Information
// =============================================================================

/// System information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub cpu_cores: u32,
    pub cpu_model: Option<String>,
    pub total_memory: u64,
    pub available_memory: u64,
    pub gpus: Vec<GpuInfo>,
}

/// GPU device info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub name: String,
    pub vendor: Option<String>,
    pub driver_version: Option<String>,
    pub memory_bytes: Option<u64>,
    pub backend: String,
}

/// Block device info (Linux)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDevice {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    pub model: Option<String>,
}

// =============================================================================
// Events (Backend → Frontend via Tauri emit)
// =============================================================================

/// Event: file was carved
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCarvedEvent {
    pub file_type: String,
    pub path: String,
    pub offset: u64,
    pub size: u64,
    pub sha256: Option<String>,
}

/// Event: log message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogMessageEvent {
    pub level: String, // "info" | "warn" | "error" | "debug"
    pub timestamp: String,
    pub message: String,
}

/// Event: scan state changed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanStateChangedEvent {
    pub state: ScanState,
    pub message: Option<String>,
}

/// Event: artefact found
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtefactFoundEvent {
    pub artefact_type: String,
    pub value: String,
    pub offset: u64,
}
