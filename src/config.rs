//! Scan configuration types

use serde::{Deserialize, Serialize};

/// Scan configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
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

    // GPU acceleration
    pub gpu_enabled: bool,

    // Entropy detection
    pub scan_entropy: bool,
    pub entropy_threshold: f64,

    // SQLite recovery
    pub scan_sqlite_pages: bool,

    // Resource limits
    pub max_bytes: Option<u64>,
    pub max_files: Option<u64>,
    pub max_memory_mib: Option<u64>,

    // Output settings
    pub metadata_backend: MetadataBackend,
    pub workers: usize,
    pub chunk_size_mib: u64,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            input_path: String::new(),
            output_path: String::new(),
            compute_evidence_hash: true,
            evidence_sha256: None,
            file_types: vec![
                "jpeg".into(), "png".into(), "gif".into(), "pdf".into(),
                "zip".into(), "sqlite".into(), "docx".into(), "xlsx".into(), 
                "pptx".into(), "mp4".into(), "webp".into(),
            ],
            disable_zip: false,
            scan_strings: true,
            scan_urls: true,
            scan_emails: true,
            scan_phones: true,
            scan_utf16: false,
            string_min_len: 8,
            gpu_enabled: false,
            scan_entropy: false,
            entropy_threshold: 7.5,
            scan_sqlite_pages: false,
            max_bytes: None,
            max_files: None,
            max_memory_mib: None,
            metadata_backend: MetadataBackend::Parquet,
            workers: 0, // 0 = auto-detect
            chunk_size_mib: 64,
        }
    }
}

/// Metadata output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetadataBackend {
    Parquet,
    Jsonl,
    Csv,
}

impl MetadataBackend {
    pub fn as_str(&self) -> &'static str {
        match self {
            MetadataBackend::Parquet => "parquet",
            MetadataBackend::Jsonl => "jsonl",
            MetadataBackend::Csv => "csv",
        }
    }
}

/// Available file types for carving
pub const FILE_TYPES: &[(&str, &str, &[&str])] = &[
    ("Images", "🖼", &["jpeg", "png", "gif", "webp", "bmp", "tiff"]),
    ("Documents", "📄", &["pdf", "docx", "xlsx", "pptx"]),
    ("Archives", "📦", &["zip", "rar", "7z"]),
    ("Databases", "🗃", &["sqlite"]),
    ("Media", "🎬", &["mp4", "mp3", "wav"]),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ScanConfig::default();
        assert!(config.scan_strings);
        assert!(config.compute_evidence_hash);
        assert_eq!(config.metadata_backend, MetadataBackend::Parquet);
        assert!(!config.file_types.is_empty());
    }

    #[test]
    fn test_metadata_backend_str() {
        assert_eq!(MetadataBackend::Parquet.as_str(), "parquet");
        assert_eq!(MetadataBackend::Jsonl.as_str(), "jsonl");
        assert_eq!(MetadataBackend::Csv.as_str(), "csv");
    }

    #[test]
    fn test_config_serialization() {
        let config = ScanConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: ScanConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.scan_strings, config.scan_strings);
    }
}
