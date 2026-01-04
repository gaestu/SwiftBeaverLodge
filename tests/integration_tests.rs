//! Integration tests for scan operations

use std::process::Command;
use tempfile::TempDir;

/// Test that swiftbeaver binary is findable
#[test]
fn test_swiftbeaver_binary_discovery() {
    // Try to find swiftbeaver in various locations
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));
    
    let possible_paths = [
        "bin/swiftbeaver",
        "./bin/swiftbeaver",
        "../bin/swiftbeaver",
    ];
    
    let mut found = false;
    for path in &possible_paths {
        if std::path::Path::new(path).exists() {
            found = true;
            break;
        }
    }
    
    // Also check PATH
    if !found {
        found = which::which("swiftbeaver").is_ok();
    }
    
    // This test doesn't fail - it just reports the status
    if found {
        println!("swiftbeaver binary found");
    } else {
        println!("swiftbeaver binary not found - download from GitHub releases");
    }
}

/// Test that we can create a mock evidence file and output directory
#[test]
fn test_scan_directory_setup() {
    let temp = TempDir::new().unwrap();
    
    // Create mock input
    let input_file = temp.path().join("test.dd");
    std::fs::write(&input_file, b"test evidence data").unwrap();
    
    // Create output directory
    let output_dir = temp.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();
    
    assert!(input_file.exists());
    assert!(output_dir.exists());
}

/// Test that JSONL metadata can be created and read
#[test]
fn test_jsonl_metadata_roundtrip() {
    use swiftbeaverlodge::metadata::CarvedFile;
    
    let file = CarvedFile {
        id: 1,
        file_type: "jpeg".to_string(),
        offset: 1024,
        size: 4096,
        output_path: "carved/0001.jpg".to_string(),
        sha256: Some("abc123".to_string()),
        is_valid: true,
        mime_type: Some("image/jpeg".to_string()),
        width: Some(800),
        height: Some(600),
    };
    
    // Serialize to JSON
    let json = serde_json::to_string(&file).unwrap();
    
    // Deserialize back
    let parsed: CarvedFile = serde_json::from_str(&json).unwrap();
    
    assert_eq!(parsed.id, file.id);
    assert_eq!(parsed.file_type, file.file_type);
    assert_eq!(parsed.offset, file.offset);
    assert_eq!(parsed.size, file.size);
    assert_eq!(parsed.sha256, file.sha256);
    assert_eq!(parsed.width, file.width);
    assert_eq!(parsed.height, file.height);
}

/// Test config serialization to YAML
#[test]
fn test_config_yaml_roundtrip() {
    use swiftbeaverlodge::config::{ScanConfig, MetadataBackend};
    
    let config = ScanConfig {
        input_path: "/evidence/disk.dd".to_string(),
        output_path: "/output/carved".to_string(),
        file_types: vec!["jpeg".to_string(), "png".to_string()],
        metadata_backend: MetadataBackend::Parquet,
        scan_strings: true,
        gpu_enabled: false,
        ..Default::default()
    };
    
    // Serialize to YAML
    let yaml = serde_yaml::to_string(&config).unwrap();
    
    // Deserialize back
    let parsed: ScanConfig = serde_yaml::from_str(&yaml).unwrap();
    
    assert_eq!(parsed.input_path, config.input_path);
    assert_eq!(parsed.output_path, config.output_path);
    assert_eq!(parsed.file_types, config.file_types);
    assert_eq!(parsed.metadata_backend, config.metadata_backend);
    assert_eq!(parsed.scan_strings, config.scan_strings);
}

/// Test progress message parsing
#[test]
fn test_progress_parsing() {
    use swiftbeaverlodge::scan::ScanProgress;
    use swiftbeaverlodge::scan::progress::parse_progress_message;
    
    let msg = "progress bytes_scanned=52428800 total_bytes=104857600 pct=50.0 hits=100 files=75 rate_mib=250.5 eta_secs=Some(200)";
    
    let progress = parse_progress_message(msg).unwrap();
    
    assert_eq!(progress.bytes_scanned, 52428800);
    assert_eq!(progress.total_bytes, 104857600);
    assert_eq!(progress.pct, 50.0);
    assert_eq!(progress.hits, 100);
    assert_eq!(progress.files, 75);
    assert!((progress.rate_mib - 250.5).abs() < 0.01);
    assert_eq!(progress.eta_secs, Some(200));
}

/// Test metadata reader with mock JSONL data
#[test]
fn test_metadata_reader_mock() {
    use swiftbeaverlodge::metadata::MetadataReader;
    
    let temp = TempDir::new().unwrap();
    let metadata_dir = temp.path().join("metadata");
    std::fs::create_dir(&metadata_dir).unwrap();
    
    // Create mock carved_files.jsonl
    let jsonl_data = r#"{"id":1,"file_type":"jpeg","offset":1024,"size":4096,"output_path":"carved/0001.jpg","is_valid":true}
{"id":2,"file_type":"png","offset":5120,"size":2048,"output_path":"carved/0002.png","is_valid":true}
{"id":3,"file_type":"jpeg","offset":7168,"size":8192,"output_path":"carved/0003.jpg","is_valid":true}"#;
    
    std::fs::write(metadata_dir.join("carved_files.jsonl"), jsonl_data).unwrap();
    
    // Read metadata
    let reader = MetadataReader::new(temp.path()).unwrap();
    let files = reader.read_carved_files().unwrap();
    
    assert_eq!(files.len(), 3);
    
    // Check summary
    let summary = reader.get_summary().unwrap();
    assert_eq!(summary.total_files, 3);
    assert_eq!(summary.total_bytes, 4096 + 2048 + 8192);
    assert_eq!(summary.by_type.get("jpeg"), Some(&2));
    assert_eq!(summary.by_type.get("png"), Some(&1));
}
