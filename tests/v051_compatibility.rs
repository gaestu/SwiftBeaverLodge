use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use arrow::array::{ArrayRef, BooleanArray, Int32Array, Int64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use swiftbeaverlodge::config::{validate_flag_combinations, FILE_TYPES};
use swiftbeaverlodge::metadata::{detect_metadata_backend, MetadataReader};
use swiftbeaverlodge::scan::progress::parse_json_log;
use swiftbeaverlodge::scan::ScanProgress;
use tempfile::TempDir;

fn fixture_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/swiftbeaver_v0_5_1")
        .join(relative)
}

fn fixture_lines(relative: &str) -> Vec<String> {
    std::fs::read_to_string(fixture_path(relative))
        .unwrap()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn write_parquet(path: &Path, schema: Arc<Schema>, columns: Vec<ArrayRef>) {
    let batch = RecordBatch::try_new(schema.clone(), columns).unwrap();
    let file = File::create(path).unwrap();
    let mut writer = ArrowWriter::try_new(file, schema, None).unwrap();
    writer.write(&batch).unwrap();
    writer.finish().unwrap();
}

fn write_v051_files_parquet(path: &Path) {
    let schema = Arc::new(Schema::new(vec![
        Field::new("run_id", DataType::Utf8, false),
        Field::new("tool_version", DataType::Utf8, false),
        Field::new("config_hash", DataType::Utf8, false),
        Field::new("evidence_sha256", DataType::Utf8, false),
        Field::new("handler_id", DataType::Utf8, false),
        Field::new("file_type", DataType::Utf8, false),
        Field::new("carved_path", DataType::Utf8, false),
        Field::new("extension", DataType::Utf8, false),
        Field::new("global_start", DataType::Int64, false),
        Field::new("global_end", DataType::Int64, false),
        Field::new("size", DataType::Int64, false),
        Field::new("md5", DataType::Utf8, true),
        Field::new("sha256", DataType::Utf8, true),
        Field::new("validated", DataType::Boolean, true),
        Field::new("truncated", DataType::Boolean, false),
        Field::new("error", DataType::Utf8, true),
        Field::new("pattern_id", DataType::Utf8, true),
        Field::new("is_duplicate", DataType::Boolean, false),
        Field::new("duplicate_of_offset", DataType::Int64, true),
        Field::new("mime_type", DataType::Utf8, true),
        Field::new("width", DataType::Int32, true),
        Field::new("height", DataType::Int32, true),
    ]));

    write_parquet(
        path,
        schema,
        vec![
            Arc::new(StringArray::from(vec![
                "20260428T140203Z_09e96cdf",
                "20260428T140203Z_09e96cdf",
            ])) as ArrayRef,
            Arc::new(StringArray::from(vec!["0.5.1", "0.5.1"])),
            Arc::new(StringArray::from(vec!["cfg-test", "cfg-test"])),
            Arc::new(StringArray::from(vec!["evidence-sha", "evidence-sha"])),
            Arc::new(StringArray::from(vec!["jpeg", "png"])),
            Arc::new(StringArray::from(vec!["jpeg", "png"])),
            Arc::new(StringArray::from(vec![
                "jpeg/jpeg_000000000400.jpg",
                "png/png_000000000800.png",
            ])),
            Arc::new(StringArray::from(vec!["jpg", "png"])),
            Arc::new(Int64Array::from(vec![1024, 2048])),
            Arc::new(Int64Array::from(vec![1056, 4096])),
            Arc::new(Int64Array::from(vec![32, 2048])),
            Arc::new(StringArray::from(vec![Some("md5-a"), None])),
            Arc::new(StringArray::from(vec![Some("sha-a"), Some("sha-b")])),
            Arc::new(BooleanArray::from(vec![Some(false), Some(true)])),
            Arc::new(BooleanArray::from(vec![true, false])),
            Arc::new(StringArray::from(vec![Some("short footer"), None])),
            Arc::new(StringArray::from(vec![Some("jpeg_soi"), Some("png_sig")])),
            Arc::new(BooleanArray::from(vec![true, false])),
            Arc::new(Int64Array::from(vec![Some(512), None])),
            Arc::new(StringArray::from(vec![
                Some("image/jpeg"),
                Some("image/png"),
            ])),
            Arc::new(Int32Array::from(vec![Some(16), Some(64)])),
            Arc::new(Int32Array::from(vec![Some(16), Some(32)])),
        ],
    );
}

fn write_v051_string_artefacts_parquet(path: &Path) {
    let schema = Arc::new(Schema::new(vec![
        Field::new("run_id", DataType::Utf8, false),
        Field::new("artefact_kind", DataType::Utf8, false),
        Field::new("content", DataType::Utf8, false),
        Field::new("global_start", DataType::Int64, false),
        Field::new("global_end", DataType::Int64, true),
        Field::new("length", DataType::Int64, true),
        Field::new("encoding", DataType::Utf8, true),
        Field::new("source", DataType::Utf8, true),
    ]));

    write_parquet(
        path,
        schema,
        vec![
            Arc::new(StringArray::from(vec![
                "20260428T140203Z_09e96cdf",
                "20260428T140203Z_09e96cdf",
            ])) as ArrayRef,
            Arc::new(StringArray::from(vec!["email", "url"])),
            Arc::new(StringArray::from(vec![
                "case@example.test",
                "https://example.test/case",
            ])),
            Arc::new(Int64Array::from(vec![20, 48])),
            Arc::new(Int64Array::from(vec![Some(37), None])),
            Arc::new(Int64Array::from(vec![None, Some(25)])),
            Arc::new(StringArray::from(vec![Some("utf8"), Some("utf8")])),
            Arc::new(StringArray::from(vec![Some("chunk-1"), Some("chunk-1")])),
        ],
    );
}

fn write_v051_run_summary_parquet(path: &Path) {
    let schema = Arc::new(Schema::new(vec![
        Field::new("run_id", DataType::Utf8, false),
        Field::new("bytes_scanned", DataType::Int64, false),
        Field::new("chunks_processed", DataType::Int64, false),
        Field::new("hits", DataType::Int64, false),
        Field::new("files_carved", DataType::Int64, false),
        Field::new("rejected", DataType::Int64, false),
        Field::new("prevalidation_rejected", DataType::Int64, false),
        Field::new("overlap_skipped", DataType::Int64, false),
        Field::new("string_spans", DataType::Int64, false),
        Field::new("artefacts_extracted", DataType::Int64, false),
        Field::new("duplicates_found", DataType::Int64, false),
        Field::new("duplicates_skipped", DataType::Int64, false),
    ]));

    write_parquet(
        path,
        schema,
        vec![
            Arc::new(StringArray::from(vec!["20260428T140203Z_09e96cdf"])) as ArrayRef,
            Arc::new(Int64Array::from(vec![2_097_152])),
            Arc::new(Int64Array::from(vec![2])),
            Arc::new(Int64Array::from(vec![18])),
            Arc::new(Int64Array::from(vec![4])),
            Arc::new(Int64Array::from(vec![1])),
            Arc::new(Int64Array::from(vec![0])),
            Arc::new(Int64Array::from(vec![3])),
            Arc::new(Int64Array::from(vec![2])),
            Arc::new(Int64Array::from(vec![2])),
            Arc::new(Int64Array::from(vec![1])),
            Arc::new(Int64Array::from(vec![1])),
        ],
    );
}

fn make_v051_parquet_run() -> TempDir {
    let temp = TempDir::new().unwrap();
    let parquet_dir = temp.path().join("parquet");
    std::fs::create_dir(&parquet_dir).unwrap();
    write_v051_files_parquet(&parquet_dir.join("files_0000.parquet"));
    write_v051_string_artefacts_parquet(&parquet_dir.join("string_artefacts.parquet"));
    write_v051_run_summary_parquet(&parquet_dir.join("run_summary.parquet"));
    temp
}

fn assert_v051_files(reader: &MetadataReader) {
    let files = reader.read_carved_files().unwrap();

    assert_eq!(files.len(), 2);
    assert_eq!(files[0].id, 0);
    assert_eq!(
        files[0].run_id.as_deref(),
        Some("20260428T140203Z_09e96cdf")
    );
    assert_eq!(files[0].handler_id.as_deref(), Some("jpeg"));
    assert_eq!(files[0].file_type, "jpeg");
    assert_eq!(files[0].path, "jpeg/jpeg_000000000400.jpg");
    assert_eq!(files[0].extension.as_deref(), Some("jpg"));
    assert_eq!(files[0].global_start, 1024);
    assert_eq!(files[0].global_end, 1056);
    assert_eq!(files[0].size, 32);
    assert_eq!(files[0].md5.as_deref(), Some("md5-a"));
    assert_eq!(files[0].sha256.as_deref(), Some("sha-a"));
    assert_eq!(files[0].validated, Some(false));
    assert!(files[0].truncated);
    assert_eq!(files[0].errors, vec!["short footer"]);
    assert_eq!(files[0].pattern_id.as_deref(), Some("jpeg_soi"));
    assert!(files[0].is_duplicate);
    assert_eq!(files[0].duplicate_of_offset, Some(512));
    assert_eq!(files[0].mime_type.as_deref(), Some("image/jpeg"));
    assert_eq!(files[0].width, Some(16));
    assert_eq!(files[0].height, Some(16));

    assert_eq!(files[1].id, 1);
    assert_eq!(files[1].file_type, "png");
    assert_eq!(files[1].path, "png/png_000000000800.png");
    assert_eq!(files[1].validated, Some(true));
    assert!(!files[1].truncated);
    assert!(!files[1].is_duplicate);
    assert_eq!(files[1].duplicate_of_offset, None);
    assert_eq!(files[1].mime_type.as_deref(), Some("image/png"));
    assert_eq!(files[1].width, Some(64));
    assert_eq!(files[1].height, Some(32));
}

fn assert_v051_summary_and_strings(reader: &MetadataReader) {
    let summary = reader.read_run_summary().unwrap().unwrap();
    assert_eq!(summary.bytes_scanned, Some(2_097_152));
    assert_eq!(summary.chunks_processed, Some(2));
    assert_eq!(summary.hits, Some(18));
    assert_eq!(summary.files_carved, Some(4));
    assert_eq!(summary.duplicates_found, Some(1));
    assert_eq!(summary.duplicates_skipped, Some(1));

    let strings = reader.read_string_artefacts().unwrap();
    assert_eq!(strings.len(), 2);
    assert_eq!(strings[0].artefact_kind, "email");
    assert_eq!(strings[0].content, "case@example.test");
    assert_eq!(strings[0].global_start, 20);
    assert_eq!(strings[0].global_end, Some(37));
    assert_eq!(strings[0].length, 17);
    assert_eq!(strings[0].encoding.as_deref(), Some("utf8"));
    assert_eq!(strings[0].source.as_deref(), Some("chunk-1"));
    assert_eq!(strings[1].artefact_kind, "url");
    assert_eq!(strings[1].content, "https://example.test/case");
    assert_eq!(strings[1].global_start, 48);
    assert_eq!(strings[1].length, 25);
}

fn assert_missing_optional_artefact_tables(reader: &MetadataReader) {
    let availability = reader.result_table_availability().unwrap();
    assert!(!availability.browser_history);
    assert!(!availability.browser_cookies);
    assert!(!availability.browser_downloads);
    assert!(!availability.windows_artefacts);
    assert!(!availability.entropy_regions);

    assert!(reader.read_browser_history().unwrap().is_empty());
    assert!(reader.read_browser_cookies().unwrap().is_empty());
    assert!(reader.read_browser_downloads().unwrap().is_empty());
    assert!(reader.read_windows_artefacts().unwrap().is_empty());
    assert!(reader.read_entropy_regions().unwrap().is_empty());
}

#[test]
fn v051_json_log_fixture_parses_starting_and_progress() {
    let lines = fixture_lines("logs/scan.jsonl");
    assert_eq!(lines.len(), 3);

    let (event_type, payload) = parse_json_log(&lines[0]).unwrap();
    assert_eq!(event_type, "starting");
    assert_eq!(
        payload.get("run_id").and_then(|value| value.as_str()),
        Some("20260428T140203Z_09e96cdf")
    );
    assert_eq!(
        payload.get("output").and_then(|value| value.as_str()),
        Some("out/20260428T140203Z_09e96cdf")
    );

    let (event_type, payload) = parse_json_log(&lines[1]).unwrap();
    assert_eq!(event_type, "progress");
    let progress: ScanProgress = serde_json::from_value(payload).unwrap();
    assert_eq!(progress.bytes_scanned, 1_048_576);
    assert_eq!(progress.total_bytes, 2_097_152);
    assert_eq!(progress.hits, 12);
    assert_eq!(progress.files, 3);
    assert_eq!(progress.eta_secs, Some(5));
    assert!((progress.rate_mib - 42.50).abs() < f64::EPSILON);

    let (_, payload) = parse_json_log(&lines[2]).unwrap();
    let complete: ScanProgress = serde_json::from_value(payload).unwrap();
    assert_eq!(complete.eta_secs, None);
    assert_eq!(complete.bytes_scanned, complete.total_bytes);
}

#[test]
fn v051_metadata_backend_detection_covers_parquet_jsonl_and_csv() {
    assert_eq!(
        detect_metadata_backend(&fixture_path("metadata/jsonl")),
        Some("jsonl")
    );
    assert_eq!(
        detect_metadata_backend(&fixture_path("metadata/csv")),
        Some("csv")
    );

    let temp = TempDir::new().unwrap();
    std::fs::create_dir(temp.path().join("parquet")).unwrap();
    let metadata_dir = temp.path().join("metadata");
    std::fs::create_dir(&metadata_dir).unwrap();
    std::fs::write(metadata_dir.join("carved_files.jsonl"), "").unwrap();
    std::fs::write(metadata_dir.join("carved_files.csv"), "").unwrap();

    assert_eq!(detect_metadata_backend(temp.path()), Some("parquet"));
}

#[test]
fn v051_jsonl_metadata_fixture_reads_field_names_and_missing_optional_tables() {
    let reader = MetadataReader::new(fixture_path("metadata/jsonl")).unwrap();
    assert_eq!(reader.backend(), "jsonl");

    assert_v051_files(&reader);
    assert_v051_summary_and_strings(&reader);
    assert_missing_optional_artefact_tables(&reader);
}

#[test]
fn v051_csv_metadata_fixture_reads_field_names_and_missing_optional_tables() {
    let reader = MetadataReader::new(fixture_path("metadata/csv")).unwrap();
    assert_eq!(reader.backend(), "csv");

    assert_v051_files(&reader);
    assert_v051_summary_and_strings(&reader);
    assert_missing_optional_artefact_tables(&reader);
}

#[test]
fn v051_parquet_metadata_fixture_reads_field_names_and_missing_optional_tables() {
    let temp = make_v051_parquet_run();
    let reader = MetadataReader::new(temp.path()).unwrap();
    assert_eq!(reader.backend(), "parquet");

    assert_v051_files(&reader);
    assert_v051_summary_and_strings(&reader);
    assert_missing_optional_artefact_tables(&reader);
}

#[test]
fn v051_file_type_catalog_includes_accepted_type_id_fixture() {
    let mut accepted = fixture_lines("accepted_file_types.txt");
    accepted.sort();

    let mut catalog = FILE_TYPES
        .iter()
        .flat_map(|(_, _, types)| types.iter().map(|type_id| (*type_id).to_string()))
        .collect::<Vec<_>>();
    catalog.sort();

    let missing = accepted
        .iter()
        .filter(|type_id| !catalog.contains(type_id))
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "v0.5.1 accepted file types missing from catalog: {missing:?}"
    );
    assert!(catalog.contains(&"bek".to_string()));
}

#[test]
fn v051_invalid_option_combinations_cover_dependent_flags() {
    let remove_invalid_without_validation = swiftbeaverlodge::config::ScanConfig {
        remove_invalid: true,
        validate_carved: false,
        ..Default::default()
    };
    assert_eq!(
        validate_flag_combinations(&remove_invalid_without_validation),
        vec!["--remove-invalid requires --validate-carved".to_string()]
    );

    let skip_duplicates_without_dedupe = swiftbeaverlodge::config::ScanConfig {
        skip_duplicates: true,
        dedupe: false,
        ..Default::default()
    };
    assert_eq!(
        validate_flag_combinations(&skip_duplicates_without_dedupe),
        vec!["--skip-duplicates requires --dedupe".to_string()]
    );
}
