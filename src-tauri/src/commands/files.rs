//! File operation commands
//!
//! Handles file browsing, metadata reading (Parquet/JSONL), and thumbnails

use std::path::PathBuf;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::fs::File;
use std::sync::Arc;

use tauri_plugin_dialog::DialogExt;
use tauri::AppHandle;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use arrow::array::{Array, StringArray, UInt64Array, BooleanArray, Int64Array, Float64Array};

use crate::types::{BlockDevice, GuiCarvedFile, GuiStringArtefact, GuiBrowserHistory, GuiRunSummary};

/// Browse for input file (opens native dialog)
#[tauri::command]
pub async fn browse_input_file(app: AppHandle) -> Result<Option<String>, String> {
    let file = app.dialog()
        .file()
        .add_filter("Disk Images", &["dd", "raw", "img", "E01", "e01", "001"])
        .add_filter("All Files", &["*"])
        .blocking_pick_file();
    
    Ok(file.map(|f| f.path.to_string_lossy().to_string()))
}

/// Browse for output directory
#[tauri::command]
pub async fn browse_output_dir(app: AppHandle) -> Result<Option<String>, String> {
    let dir = app.dialog()
        .file()
        .blocking_pick_folder();
    
    Ok(dir.map(|d| d.path.to_string_lossy().to_string()))
}

/// List available block devices (Linux only)
#[tauri::command]
pub async fn list_block_devices() -> Result<Vec<BlockDevice>, String> {
    #[cfg(target_os = "linux")]
    {
        let mut devices = Vec::new();
        
        // Read from /sys/block
        let block_dir = std::fs::read_dir("/sys/block")
            .map_err(|e| format!("Failed to read /sys/block: {}", e))?;
        
        for entry in block_dir.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            
            // Skip loop devices, ram disks, etc.
            if name.starts_with("loop") || name.starts_with("ram") || name.starts_with("dm-") {
                continue;
            }
            
            let device_path = format!("/dev/{}", name);
            
            // Read size (in 512-byte sectors)
            let size_path = entry.path().join("size");
            let size_bytes = if let Ok(size_str) = std::fs::read_to_string(&size_path) {
                size_str.trim().parse::<u64>().unwrap_or(0) * 512
            } else {
                0
            };
            
            // Read model
            let model_path = entry.path().join("device/model");
            let model = std::fs::read_to_string(&model_path)
                .ok()
                .map(|s| s.trim().to_string());
            
            devices.push(BlockDevice {
                path: device_path,
                name,
                size_bytes,
                model,
            });
        }
        
        Ok(devices)
    }
    
    #[cfg(not(target_os = "linux"))]
    {
        Ok(Vec::new())
    }
}

/// Detect metadata backend for a run (parquet, jsonl, or csv)
fn detect_metadata_backend(run_path: &PathBuf) -> &'static str {
    let parquet_dir = run_path.join("parquet");
    if parquet_dir.exists() && parquet_dir.is_dir() {
        return "parquet";
    }
    
    let metadata_dir = run_path.join("metadata");
    if metadata_dir.join("carved_files.jsonl").exists() {
        return "jsonl";
    }
    if metadata_dir.join("carved_files.csv").exists() {
        return "csv";
    }
    
    "unknown"
}

/// Read carved files metadata from a run (auto-detects Parquet or JSONL)
#[tauri::command]
pub async fn read_carved_metadata(run_path: String) -> Result<Vec<GuiCarvedFile>, String> {
    let run_path = PathBuf::from(&run_path);
    let backend = detect_metadata_backend(&run_path);
    
    match backend {
        "parquet" => read_carved_files_parquet(&run_path),
        "jsonl" => read_carved_files_jsonl(&run_path),
        _ => Ok(Vec::new()),
    }
}

/// Read carved files from Parquet format
fn read_carved_files_parquet(run_path: &PathBuf) -> Result<Vec<GuiCarvedFile>, String> {
    let parquet_dir = run_path.join("parquet");
    let mut files = Vec::new();
    
    // Read all files_*.parquet files
    let entries = std::fs::read_dir(&parquet_dir)
        .map_err(|e| format!("Failed to read parquet dir: {}", e))?;
    
    for entry in entries.flatten() {
        let path = entry.path();
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        
        if filename.starts_with("files_") && filename.ends_with(".parquet") {
            if let Ok(mut batch_files) = read_parquet_carved_file(&path) {
                files.append(&mut batch_files);
            }
        }
    }
    
    Ok(files)
}

/// Read a single Parquet file containing carved file records
fn read_parquet_carved_file(path: &PathBuf) -> Result<Vec<GuiCarvedFile>, String> {
    let file = File::open(path)
        .map_err(|e| format!("Failed to open parquet file: {}", e))?;
    
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| format!("Failed to create parquet reader: {}", e))?;
    
    let reader = builder.build()
        .map_err(|e| format!("Failed to build parquet reader: {}", e))?;
    
    let mut files = Vec::new();
    
    for batch in reader {
        let batch = batch.map_err(|e| format!("Failed to read batch: {}", e))?;
        
        // Extract columns
        let run_id_col = get_string_column(&batch, "run_id");
        let file_type_col = get_string_column(&batch, "file_type");
        let path_col = get_string_column(&batch, "carved_path");
        let global_start_col = get_u64_column(&batch, "global_start");
        let global_end_col = get_u64_column(&batch, "global_end");
        let size_col = get_u64_column(&batch, "size");
        let md5_col = get_string_column(&batch, "md5");
        let sha256_col = get_string_column(&batch, "sha256");
        let validated_col = get_bool_column(&batch, "validated");
        let truncated_col = get_bool_column(&batch, "truncated");
        let pattern_id_col = get_string_column(&batch, "pattern_id");
        
        for i in 0..batch.num_rows() {
            files.push(GuiCarvedFile {
                run_id: get_string_value(&run_id_col, i).unwrap_or_default(),
                file_type: get_string_value(&file_type_col, i).unwrap_or_default(),
                path: get_string_value(&path_col, i).unwrap_or_default(),
                extension: get_extension(&get_string_value(&path_col, i).unwrap_or_default()),
                global_start: get_u64_value(&global_start_col, i).unwrap_or(0),
                global_end: get_u64_value(&global_end_col, i).unwrap_or(0),
                size: get_u64_value(&size_col, i).unwrap_or(0),
                md5: get_string_value(&md5_col, i),
                sha256: get_string_value(&sha256_col, i),
                validated: get_bool_value(&validated_col, i).unwrap_or(false),
                truncated: get_bool_value(&truncated_col, i).unwrap_or(false),
                errors: Vec::new(),
                pattern_id: get_string_value(&pattern_id_col, i),
            });
        }
    }
    
    Ok(files)
}

/// Read carved files from JSONL format (fallback)
fn read_carved_files_jsonl(run_path: &PathBuf) -> Result<Vec<GuiCarvedFile>, String> {
    let path = run_path.join("metadata").join("carved_files.jsonl");
    
    if !path.exists() {
        return Ok(Vec::new());
    }
    
    let file = File::open(&path)
        .map_err(|e| format!("Failed to open carved_files.jsonl: {}", e))?;
    
    let reader = BufReader::new(file);
    let mut files = Vec::new();
    
    for line in reader.lines() {
        let line = line.map_err(|e| format!("Failed to read line: {}", e))?;
        if line.trim().is_empty() {
            continue;
        }
        
        match serde_json::from_str::<GuiCarvedFile>(&line) {
            Ok(file) => files.push(file),
            Err(e) => tracing::warn!("Failed to parse carved file entry: {}", e),
        }
    }
    
    Ok(files)
}

/// Read string artefacts from a run
#[tauri::command]
pub async fn read_string_artefacts(run_path: String) -> Result<Vec<GuiStringArtefact>, String> {
    let run_path = PathBuf::from(&run_path);
    let backend = detect_metadata_backend(&run_path);
    
    match backend {
        "parquet" => read_string_artefacts_parquet(&run_path),
        "jsonl" => read_string_artefacts_jsonl(&run_path),
        _ => Ok(Vec::new()),
    }
}

/// Read string artefacts from Parquet format
fn read_string_artefacts_parquet(run_path: &PathBuf) -> Result<Vec<GuiStringArtefact>, String> {
    let parquet_dir = run_path.join("parquet");
    let mut artefacts = Vec::new();
    
    // Read artefacts_urls.parquet, artefacts_emails.parquet, artefacts_phones.parquet
    for artefact_type in &["urls", "emails", "phones"] {
        let path = parquet_dir.join(format!("artefacts_{}.parquet", artefact_type));
        if path.exists() {
            if let Ok(mut batch_artefacts) = read_parquet_artefacts(&path, artefact_type) {
                artefacts.append(&mut batch_artefacts);
            }
        }
    }
    
    Ok(artefacts)
}

/// Read artefacts from a single Parquet file
fn read_parquet_artefacts(path: &PathBuf, artefact_type: &str) -> Result<Vec<GuiStringArtefact>, String> {
    let file = File::open(path)
        .map_err(|e| format!("Failed to open parquet file: {}", e))?;
    
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| format!("Failed to create parquet reader: {}", e))?;
    
    let reader = builder.build()
        .map_err(|e| format!("Failed to build parquet reader: {}", e))?;
    
    let mut artefacts = Vec::new();
    
    // Map artefact_type to kind and content column
    let (kind, content_col_name) = match artefact_type {
        "urls" => ("url", "url"),
        "emails" => ("email", "email"),
        "phones" => ("phone", "phone_raw"),
        _ => return Ok(Vec::new()),
    };
    
    for batch in reader {
        let batch = batch.map_err(|e| format!("Failed to read batch: {}", e))?;
        
        let run_id_col = get_string_column(&batch, "run_id");
        let content_col = get_string_column(&batch, content_col_name);
        let global_start_col = get_u64_column(&batch, "global_start");
        let global_end_col = get_u64_column(&batch, "global_end");
        
        for i in 0..batch.num_rows() {
            artefacts.push(GuiStringArtefact {
                run_id: get_string_value(&run_id_col, i).unwrap_or_default(),
                artefact_kind: kind.to_string(),
                content: get_string_value(&content_col, i).unwrap_or_default(),
                encoding: "utf-8".to_string(),
                global_start: get_u64_value(&global_start_col, i).unwrap_or(0),
                global_end: get_u64_value(&global_end_col, i).unwrap_or(0),
            });
        }
    }
    
    Ok(artefacts)
}

/// Read string artefacts from JSONL format (fallback)
fn read_string_artefacts_jsonl(run_path: &PathBuf) -> Result<Vec<GuiStringArtefact>, String> {
    let path = run_path.join("metadata").join("string_artefacts.jsonl");
    
    if !path.exists() {
        return Ok(Vec::new());
    }
    
    let file = File::open(&path)
        .map_err(|e| format!("Failed to open string_artefacts.jsonl: {}", e))?;
    
    let reader = BufReader::new(file);
    let mut artefacts = Vec::new();
    
    for line in reader.lines() {
        let line = line.map_err(|e| format!("Failed to read line: {}", e))?;
        if line.trim().is_empty() {
            continue;
        }
        
        match serde_json::from_str::<GuiStringArtefact>(&line) {
            Ok(artefact) => artefacts.push(artefact),
            Err(e) => tracing::warn!("Failed to parse string artefact entry: {}", e),
        }
    }
    
    Ok(artefacts)
}

/// Read browser history from a run
#[tauri::command]
pub async fn read_browser_history(run_path: String) -> Result<Vec<GuiBrowserHistory>, String> {
    let run_path = PathBuf::from(&run_path);
    let backend = detect_metadata_backend(&run_path);
    
    match backend {
        "parquet" => read_browser_history_parquet(&run_path),
        "jsonl" => read_browser_history_jsonl(&run_path),
        _ => Ok(Vec::new()),
    }
}

/// Read browser history from Parquet format
fn read_browser_history_parquet(run_path: &PathBuf) -> Result<Vec<GuiBrowserHistory>, String> {
    let path = run_path.join("parquet").join("browser_history.parquet");
    
    if !path.exists() {
        return Ok(Vec::new());
    }
    
    let file = File::open(&path)
        .map_err(|e| format!("Failed to open parquet file: {}", e))?;
    
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| format!("Failed to create parquet reader: {}", e))?;
    
    let reader = builder.build()
        .map_err(|e| format!("Failed to build parquet reader: {}", e))?;
    
    let mut history = Vec::new();
    
    for batch in reader {
        let batch = batch.map_err(|e| format!("Failed to read batch: {}", e))?;
        
        let run_id_col = get_string_column(&batch, "run_id");
        let source_file_col = get_string_column(&batch, "source_file");
        let browser_col = get_string_column(&batch, "browser");
        let url_col = get_string_column(&batch, "url");
        let title_col = get_string_column(&batch, "title");
        let visit_count_col = get_i64_column(&batch, "visit_count");
        let visit_time_col = get_string_column(&batch, "visit_time");
        
        for i in 0..batch.num_rows() {
            history.push(GuiBrowserHistory {
                run_id: get_string_value(&run_id_col, i).unwrap_or_default(),
                source_file: get_string_value(&source_file_col, i).unwrap_or_default(),
                browser: get_string_value(&browser_col, i).unwrap_or_default(),
                url: get_string_value(&url_col, i).unwrap_or_default(),
                title: get_string_value(&title_col, i),
                visit_count: get_i64_value(&visit_count_col, i),
                last_visit_time: get_string_value(&visit_time_col, i),
            });
        }
    }
    
    Ok(history)
}

/// Read browser history from JSONL format (fallback)
fn read_browser_history_jsonl(run_path: &PathBuf) -> Result<Vec<GuiBrowserHistory>, String> {
    let path = run_path.join("metadata").join("browser_history.jsonl");
    
    if !path.exists() {
        return Ok(Vec::new());
    }
    
    let file = File::open(&path)
        .map_err(|e| format!("Failed to open browser_history.jsonl: {}", e))?;
    
    let reader = BufReader::new(file);
    let mut history = Vec::new();
    
    for line in reader.lines() {
        let line = line.map_err(|e| format!("Failed to read line: {}", e))?;
        if line.trim().is_empty() {
            continue;
        }
        
        match serde_json::from_str::<GuiBrowserHistory>(&line) {
            Ok(record) => history.push(record),
            Err(e) => tracing::warn!("Failed to parse browser history entry: {}", e),
        }
    }
    
    Ok(history)
}

/// Read run summary
#[tauri::command]
pub async fn read_run_summary(run_path: String) -> Result<Option<GuiRunSummary>, String> {
    let run_path = PathBuf::from(&run_path);
    let backend = detect_metadata_backend(&run_path);
    
    match backend {
        "parquet" => read_run_summary_parquet(&run_path),
        "jsonl" => read_run_summary_jsonl(&run_path),
        _ => Ok(None),
    }
}

/// Read run summary from Parquet format
fn read_run_summary_parquet(run_path: &PathBuf) -> Result<Option<GuiRunSummary>, String> {
    let path = run_path.join("parquet").join("run_summary.parquet");
    
    if !path.exists() {
        return Ok(None);
    }
    
    let file = File::open(&path)
        .map_err(|e| format!("Failed to open parquet file: {}", e))?;
    
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| format!("Failed to create parquet reader: {}", e))?;
    
    let reader = builder.build()
        .map_err(|e| format!("Failed to build parquet reader: {}", e))?;
    
    // Get the last row (most recent summary)
    let mut last_summary: Option<GuiRunSummary> = None;
    
    for batch in reader {
        let batch = batch.map_err(|e| format!("Failed to read batch: {}", e))?;
        
        if batch.num_rows() == 0 {
            continue;
        }
        
        let i = batch.num_rows() - 1;
        
        let run_id_col = get_string_column(&batch, "run_id");
        let bytes_scanned_col = get_u64_column(&batch, "bytes_scanned");
        let chunks_processed_col = get_u64_column(&batch, "chunks_processed");
        let hits_found_col = get_u64_column(&batch, "hits_found");
        let files_carved_col = get_u64_column(&batch, "files_carved");
        let string_spans_col = get_u64_column(&batch, "string_spans");
        let artefacts_extracted_col = get_u64_column(&batch, "artefacts_extracted");
        
        last_summary = Some(GuiRunSummary {
            run_id: get_string_value(&run_id_col, i).unwrap_or_default(),
            bytes_scanned: get_u64_value(&bytes_scanned_col, i).unwrap_or(0),
            chunks_processed: get_u64_value(&chunks_processed_col, i).unwrap_or(0),
            hits_found: get_u64_value(&hits_found_col, i).unwrap_or(0),
            files_carved: get_u64_value(&files_carved_col, i).unwrap_or(0),
            string_spans: get_u64_value(&string_spans_col, i).unwrap_or(0),
            artefacts_extracted: get_u64_value(&artefacts_extracted_col, i).unwrap_or(0),
        });
    }
    
    Ok(last_summary)
}

/// Read run summary from JSONL format (fallback)
fn read_run_summary_jsonl(run_path: &PathBuf) -> Result<Option<GuiRunSummary>, String> {
    let path = run_path.join("metadata").join("run_summary.jsonl");
    
    if !path.exists() {
        return Ok(None);
    }
    
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read run_summary.jsonl: {}", e))?;
    
    // Get the last line (most recent summary)
    for line in content.lines().rev() {
        if line.trim().is_empty() {
            continue;
        }
        
        match serde_json::from_str::<GuiRunSummary>(line) {
            Ok(summary) => return Ok(Some(summary)),
            Err(e) => tracing::warn!("Failed to parse run summary: {}", e),
        }
    }
    
    Ok(None)
}

/// Generate thumbnail for image file (returns base64)
#[tauri::command]
pub async fn generate_thumbnail(path: String, max_size: u32) -> Result<String, String> {
    let path = PathBuf::from(&path);
    
    // Load and resize image
    let img = image::open(&path)
        .map_err(|e| format!("Failed to open image: {}", e))?;
    
    let thumbnail = img.thumbnail(max_size, max_size);
    
    // Encode as PNG to bytes
    let mut bytes: Vec<u8> = Vec::new();
    thumbnail.write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
        .map_err(|e| format!("Failed to encode thumbnail: {}", e))?;
    
    // Convert to base64
    let base64 = BASE64.encode(&bytes);
    
    Ok(format!("data:image/png;base64,{}", base64))
}

/// Read file bytes for hex viewer
#[tauri::command]
pub async fn read_file_bytes(path: String, offset: u64, length: u64) -> Result<Vec<u8>, String> {
    let path = PathBuf::from(&path);
    
    let mut file = File::open(&path)
        .map_err(|e| format!("Failed to open file: {}", e))?;
    
    file.seek(SeekFrom::Start(offset))
        .map_err(|e| format!("Failed to seek: {}", e))?;
    
    let mut buffer = vec![0u8; length as usize];
    let bytes_read = file.read(&mut buffer)
        .map_err(|e| format!("Failed to read: {}", e))?;
    
    buffer.truncate(bytes_read);
    Ok(buffer)
}

// =============================================================================
// Parquet Helper Functions
// =============================================================================

fn get_string_column(batch: &arrow::record_batch::RecordBatch, name: &str) -> Option<StringArray> {
    batch.column_by_name(name)
        .and_then(|col| col.as_any().downcast_ref::<StringArray>())
        .cloned()
}

fn get_u64_column(batch: &arrow::record_batch::RecordBatch, name: &str) -> Option<UInt64Array> {
    batch.column_by_name(name)
        .and_then(|col| col.as_any().downcast_ref::<UInt64Array>())
        .cloned()
}

fn get_i64_column(batch: &arrow::record_batch::RecordBatch, name: &str) -> Option<Int64Array> {
    batch.column_by_name(name)
        .and_then(|col| col.as_any().downcast_ref::<Int64Array>())
        .cloned()
}

fn get_bool_column(batch: &arrow::record_batch::RecordBatch, name: &str) -> Option<BooleanArray> {
    batch.column_by_name(name)
        .and_then(|col| col.as_any().downcast_ref::<BooleanArray>())
        .cloned()
}

fn get_string_value(col: &Option<StringArray>, idx: usize) -> Option<String> {
    col.as_ref().and_then(|c| {
        if c.is_null(idx) {
            None
        } else {
            Some(c.value(idx).to_string())
        }
    })
}

fn get_u64_value(col: &Option<UInt64Array>, idx: usize) -> Option<u64> {
    col.as_ref().and_then(|c| {
        if c.is_null(idx) {
            None
        } else {
            Some(c.value(idx))
        }
    })
}

fn get_i64_value(col: &Option<Int64Array>, idx: usize) -> Option<i64> {
    col.as_ref().and_then(|c| {
        if c.is_null(idx) {
            None
        } else {
            Some(c.value(idx))
        }
    })
}

fn get_bool_value(col: &Option<BooleanArray>, idx: usize) -> Option<bool> {
    col.as_ref().and_then(|c| {
        if c.is_null(idx) {
            None
        } else {
            Some(c.value(idx))
        }
    })
}

fn get_extension(path: &str) -> String {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string()
}
