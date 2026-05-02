//! Integration tests for scan operations

use tempfile::TempDir;

/// Test that swiftbeaver binary is findable
#[test]
fn test_swiftbeaver_binary_discovery() {
    // Try to find swiftbeaver in various locations
    let _exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));

    let possible_paths = ["bin/swiftbeaver", "./bin/swiftbeaver", "../bin/swiftbeaver"];

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
        run_id: Some("run-1".to_string()),
        file_type: "jpeg".to_string(),
        path: "carved/0001.jpg".to_string(),
        extension: Some("jpg".to_string()),
        global_start: 1024,
        global_end: 5120,
        size: 4096,
        handler_id: Some("jpeg".to_string()),
        md5: Some("def456".to_string()),
        sha256: Some("abc123".to_string()),
        validated: Some(true),
        truncated: false,
        errors: Vec::new(),
        pattern_id: Some("jpeg_soi".to_string()),
        is_duplicate: false,
        duplicate_of_offset: None,
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
    assert_eq!(parsed.global_start, file.global_start);
    assert_eq!(parsed.global_end, file.global_end);
    assert_eq!(parsed.path, file.path);
    assert_eq!(parsed.md5, file.md5);
    assert_eq!(parsed.size, file.size);
    assert_eq!(parsed.sha256, file.sha256);
    assert_eq!(parsed.validated, file.validated);
    assert_eq!(parsed.truncated, file.truncated);
    assert_eq!(parsed.is_duplicate, file.is_duplicate);
    assert_eq!(parsed.width, file.width);
    assert_eq!(parsed.height, file.height);
}

/// Test config serialization to YAML
#[test]
fn test_config_yaml_roundtrip() {
    use swiftbeaverlodge::config::{MetadataBackend, ScanConfig};

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
    use swiftbeaverlodge::metadata::{MetadataReader, MetadataSummary};

    let temp = TempDir::new().unwrap();
    let metadata_dir = temp.path().join("metadata");
    std::fs::create_dir(&metadata_dir).unwrap();

    // Create mock carved_files.jsonl
    let jsonl_data = r#"{"run_id":"run-1","file_type":"jpeg","path":"jpeg/jpeg_000000000400.jpg","extension":"jpg","global_start":1024,"global_end":5120,"size":4096,"sha256":"sha-a","validated":true,"truncated":false,"errors":[],"pattern_id":"jpeg_soi","is_duplicate":false,"duplicate_of_offset":null}
{"run_id":"run-1","file_type":"png","path":"png/png_000000001400.png","extension":"png","global_start":5120,"global_end":7168,"size":2048,"sha256":"sha-b","validated":true,"truncated":false,"errors":[],"pattern_id":"png_sig","is_duplicate":false,"duplicate_of_offset":null}
{"run_id":"run-1","file_type":"jpeg","path":"jpeg/jpeg_000000001C00.jpg","extension":"jpg","global_start":7168,"global_end":15360,"size":8192,"sha256":"sha-c","validated":true,"truncated":false,"errors":[],"pattern_id":"jpeg_soi","is_duplicate":false,"duplicate_of_offset":null}"#;

    std::fs::write(metadata_dir.join("carved_files.jsonl"), jsonl_data).unwrap();

    // Read metadata
    let reader = MetadataReader::new(temp.path()).unwrap();
    let files = reader.read_carved_files().unwrap();

    assert_eq!(files.len(), 3);

    // Check summary
    let strings = reader.read_string_artefacts().unwrap();
    let summary = MetadataSummary::from_results(&files, &strings).unwrap();
    assert_eq!(summary.total_files, 3);
    assert_eq!(summary.total_bytes, 4096 + 2048 + 8192);
    assert_eq!(summary.by_type.get("jpeg"), Some(&2));
    assert_eq!(summary.by_type.get("png"), Some(&1));
}
