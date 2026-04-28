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

    /// Optional SwiftBeaver YAML config file (--config-path).
    /// When set, SwiftBeaver merges this file with CLI flags.
    #[serde(default)]
    pub config_path: Option<String>,

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
    /// Optional override for entropy window size in bytes (--entropy-window-bytes).
    #[serde(default)]
    pub entropy_window_bytes: Option<u64>,

    // SQLite recovery
    pub scan_sqlite_pages: bool,

    // Resource limits
    pub max_bytes: Option<u64>,
    pub max_files: Option<u64>,
    pub max_memory_mib: Option<u64>,
    /// Stop after scanning this many chunks (--max-chunks).
    #[serde(default)]
    pub max_chunks: Option<u64>,
    /// Limit maximum open file descriptors (--max-open-files, Unix only).
    #[serde(default)]
    pub max_open_files: Option<u64>,

    // Output settings
    pub metadata_backend: MetadataBackend,
    pub workers: usize,
    /// Optional override for scan worker count (--scan-workers). 0 = inherit --workers.
    #[serde(default)]
    pub scan_workers: usize,
    /// Optional override for carve worker count (--carve-workers). 0 = inherit --workers.
    #[serde(default)]
    pub carve_workers: usize,
    /// Optional override for dedicated I/O writer thread count (--write-workers). 0 = unset.
    #[serde(default)]
    pub write_workers: usize,
    pub chunk_size_mib: u64,
    /// Optional override for chunk overlap in KiB (--overlap-kib).
    #[serde(default)]
    pub overlap_kib: Option<u64>,

    // Run modes
    /// --dry-run: scan and count but don't write files.
    #[serde(default)]
    pub dry_run: bool,
    /// --metadata-only: scan and record metadata but don't write carved files.
    #[serde(default)]
    pub metadata_only: bool,

    // Post-carving validation
    /// --validate-carved: validate carved files after extraction.
    #[serde(default)]
    pub validate_carved: bool,
    /// --remove-invalid: delete files that fail validation (requires `validate_carved`).
    #[serde(default)]
    pub remove_invalid: bool,

    // Hashing & dedup
    /// --hash-algorithms: e.g. ["md5", "sha256"]. Empty = leave to SwiftBeaver default.
    #[serde(default)]
    pub hash_algorithms: Vec<String>,
    /// --dedupe: track duplicates in metadata.
    #[serde(default)]
    pub dedupe: bool,
    /// --skip-duplicates: don't write duplicate files (requires `dedupe`).
    #[serde(default)]
    pub skip_duplicates: bool,

    // Checkpoint / resume
    /// --checkpoint-path: write checkpoint state to this path on early exit.
    #[serde(default)]
    pub checkpoint_path: Option<String>,
    /// --resume-from: resume scanning from a previously written checkpoint.
    #[serde(default)]
    pub resume_from: Option<String>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            input_path: String::new(),
            output_path: String::new(),
            compute_evidence_hash: true,
            evidence_sha256: None,
            config_path: None,
            file_types: vec![
                "jpeg".into(),
                "png".into(),
                "gif".into(),
                "pdf".into(),
                "zip".into(),
                "sqlite".into(),
                "docx".into(),
                "xlsx".into(),
                "pptx".into(),
                "mp4".into(),
                "webp".into(),
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
            entropy_window_bytes: None,
            scan_sqlite_pages: false,
            max_bytes: None,
            max_files: None,
            max_memory_mib: None,
            max_chunks: None,
            max_open_files: None,
            metadata_backend: MetadataBackend::Parquet,
            workers: 0, // 0 = auto-detect
            scan_workers: 0,
            carve_workers: 0,
            write_workers: 0,
            chunk_size_mib: 64,
            overlap_kib: None,
            dry_run: false,
            metadata_only: false,
            validate_carved: false,
            remove_invalid: false,
            hash_algorithms: Vec::new(),
            dedupe: false,
            skip_duplicates: false,
            checkpoint_path: None,
            resume_from: None,
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
    (
        "Images",
        "🖼",
        &["jpeg", "png", "gif", "webp", "bmp", "tiff"],
    ),
    ("Documents", "📄", &["pdf", "docx", "xlsx", "pptx"]),
    ("Archives", "📦", &["zip", "rar", "7z"]),
    ("Databases", "🗃", &["sqlite"]),
    ("Media", "🎬", &["mp4", "mp3", "wav"]),
];

/// Hash algorithms recognised by SwiftBeaver's `--hash-algorithms` flag.
pub const SUPPORTED_HASH_ALGORITHMS: &[&str] = &["md5", "sha256"];

/// Cross-field validation for SwiftBeaver flag combinations.
///
/// Returns a list of human-readable issues. An empty `Vec` means the
/// combination is acceptable. Path/existence checks live in the UI layer.
pub fn validate_flag_combinations(config: &ScanConfig) -> Vec<String> {
    let mut issues = Vec::new();

    if config.dry_run && config.metadata_only {
        issues.push("--dry-run and --metadata-only are mutually exclusive".to_string());
    }
    if config.remove_invalid && !config.validate_carved {
        issues.push("--remove-invalid requires --validate-carved".to_string());
    }
    if config.skip_duplicates && !config.dedupe {
        issues.push("--skip-duplicates requires --dedupe".to_string());
    }
    for algo in &config.hash_algorithms {
        let lower = algo.to_ascii_lowercase();
        if !SUPPORTED_HASH_ALGORITHMS.contains(&lower.as_str()) {
            issues.push(format!(
                "Unsupported hash algorithm '{}': expected one of {}",
                algo,
                SUPPORTED_HASH_ALGORITHMS.join(", ")
            ));
        }
    }

    // Forensic safety: never allow checkpoint writes targeting evidence.
    if let Some(ckpt) = config.checkpoint_path.as_ref().filter(|p| !p.is_empty()) {
        if !config.input_path.is_empty() && paths_refer_to_same_target(ckpt, &config.input_path) {
            issues.push("--checkpoint-path must not point at the evidence input".to_string());
        } else if is_raw_device_path(ckpt) {
            issues.push(
                "--checkpoint-path must not point at a raw block device (/dev/...)".to_string(),
            );
        }
    }

    issues
}

/// True if `path` looks like a Unix raw block-device path.
fn is_raw_device_path(path: &str) -> bool {
    path.starts_with("/dev/")
}

/// Best-effort same-target comparison that tolerates non-existent paths.
/// Falls back to lexical comparison when canonicalisation fails (e.g. the
/// checkpoint target does not yet exist).
fn paths_refer_to_same_target(a: &str, b: &str) -> bool {
    use std::path::Path;
    let pa = Path::new(a);
    let pb = Path::new(b);
    if pa == pb {
        return true;
    }
    match (pa.canonicalize(), pb.canonicalize()) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => false,
    }
}

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
