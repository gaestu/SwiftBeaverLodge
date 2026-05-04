//! Metadata reader for Parquet and JSONL files

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use arrow::array::{Array, BooleanArray, Int64Array, StringArray, UInt32Array, UInt64Array};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde_json::Value;

use super::types::{CarvedFile, MetadataSummary, StringArtefact};

/// Reader for scan metadata
pub struct MetadataReader {
    run_path: PathBuf,
    backend: String,
}

impl MetadataReader {
    /// Create a new metadata reader for a run directory
    pub fn new(run_path: impl AsRef<Path>) -> Result<Self> {
        let run_path = run_path.as_ref().to_path_buf();

        let backend = super::detect_metadata_backend(&run_path)
            .context("No metadata found in run directory")?
            .to_string();

        Ok(Self { run_path, backend })
    }

    /// Get the metadata backend type
    #[allow(dead_code)]
    pub fn backend(&self) -> &str {
        &self.backend
    }

    /// Read all carved files
    pub fn read_carved_files(&self) -> Result<Vec<CarvedFile>> {
        match self.backend.as_str() {
            "parquet" => self.read_carved_files_parquet(),
            "jsonl" => self.read_carved_files_jsonl(),
            "csv" => self.read_carved_files_csv(),
            _ => bail!("Unknown metadata backend: {}", self.backend),
        }
    }

    /// Read carved files from Parquet
    fn read_carved_files_parquet(&self) -> Result<Vec<CarvedFile>> {
        let parquet_dir = self.run_path.join("parquet");
        let mut all_files = Vec::new();

        // Read all files_*.parquet files
        for entry in std::fs::read_dir(&parquet_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "parquet").unwrap_or(false)
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("files_"))
                    .unwrap_or(false)
            {
                let files = self.read_parquet_file(&path)?;
                all_files.extend(files);
            }
        }

        Ok(all_files)
    }

    /// Read a single Parquet file
    fn read_parquet_file(&self, path: &Path) -> Result<Vec<CarvedFile>> {
        let file = File::open(path)?;
        let reader = ParquetRecordBatchReaderBuilder::try_new(file)?.build()?;

        let mut files = Vec::new();
        let mut next_id: u64 = 0;

        for batch_result in reader {
            let batch = batch_result?;

            // Get column arrays - use actual fastcarve column names
            let type_col = batch
                .column_by_name("file_type")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let offset_col = batch
                .column_by_name("global_start")
                .and_then(|c| c.as_any().downcast_ref::<Int64Array>());
            let size_col = batch
                .column_by_name("size")
                .and_then(|c| c.as_any().downcast_ref::<Int64Array>());
            let output_col = batch
                .column_by_name("carved_path")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let sha256_col = batch
                .column_by_name("sha256")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let valid_col = batch
                .column_by_name("validated")
                .and_then(|c| c.as_any().downcast_ref::<BooleanArray>());
            let width_col = batch
                .column_by_name("width")
                .and_then(|c| c.as_any().downcast_ref::<UInt32Array>());
            let height_col = batch
                .column_by_name("height")
                .and_then(|c| c.as_any().downcast_ref::<UInt32Array>());

            for i in 0..batch.num_rows() {
                let file =
                    CarvedFile {
                        id: next_id,
                        file_type: type_col.map(|c| c.value(i).to_string()).unwrap_or_default(),
                        offset: offset_col.map(|c| c.value(i) as u64).unwrap_or(0),
                        size: size_col.map(|c| c.value(i) as u64).unwrap_or(0),
                        output_path: output_col
                            .map(|c| c.value(i).to_string())
                            .unwrap_or_default(),
                        sha256: sha256_col.and_then(|c| {
                            if c.is_null(i) {
                                None
                            } else {
                                Some(c.value(i).to_string())
                            }
                        }),
                        is_valid: valid_col.map(|c| c.value(i)).unwrap_or(true),
                        mime_type: None,
                        width: width_col.and_then(|c| {
                            if c.is_null(i) {
                                None
                            } else {
                                Some(c.value(i))
                            }
                        }),
                        height: height_col.and_then(|c| {
                            if c.is_null(i) {
                                None
                            } else {
                                Some(c.value(i))
                            }
                        }),
                    };
                files.push(file);
                next_id += 1;
            }
        }

        Ok(files)
    }

    /// Read carved files from JSONL
    fn read_carved_files_jsonl(&self) -> Result<Vec<CarvedFile>> {
        let jsonl_path = self.run_path.join("metadata").join("carved_files.jsonl");
        let file = File::open(&jsonl_path)?;
        let reader = BufReader::new(file);

        let mut files = Vec::new();
        let mut next_id = 0u64;

        for (line_no, line) in reader.lines().enumerate() {
            let line = line?;
            if line.is_empty() {
                continue;
            }

            let value: Value = serde_json::from_str(&line).with_context(|| {
                format!("Failed to parse carved-file metadata row {}", line_no + 1)
            })?;
            let file = Self::carved_file_from_json(&value, next_id).with_context(|| {
                format!("Failed to parse carved-file metadata row {}", line_no + 1)
            })?;
            files.push(file);
            next_id += 1;
        }

        Ok(files)
    }

    /// Read carved files from CSV
    fn read_carved_files_csv(&self) -> Result<Vec<CarvedFile>> {
        let csv_path = self.run_path.join("metadata").join("carved_files.csv");
        let mut files = Vec::new();

        for_each_csv_row(&csv_path, |idx, row| {
            let start = csv_u64(row, &["offset", "global_start"]).unwrap_or(0);
            let end = csv_u64(row, &["global_end"]);
            let size = csv_u64(row, &["size"]).or_else(|| end.map(|e| e.saturating_sub(start)));

            files.push(CarvedFile {
                id: csv_u64(row, &["id"]).unwrap_or(idx as u64),
                file_type: csv_string(row, &["file_type"]).unwrap_or_default(),
                offset: start,
                size: size.unwrap_or(0),
                output_path: csv_string(row, &["output_path", "carved_path", "path"])
                    .unwrap_or_default(),
                sha256: csv_string(row, &["sha256"]),
                is_valid: csv_bool(row, &["is_valid", "validated"]).unwrap_or(true),
                mime_type: csv_string(row, &["mime_type"]),
                width: csv_u64(row, &["width"]).and_then(|v| u32::try_from(v).ok()),
                height: csv_u64(row, &["height"]).and_then(|v| u32::try_from(v).ok()),
            });
            Ok(())
        })?;

        Ok(files)
    }

    /// Read string artefacts
    pub fn read_string_artefacts(&self) -> Result<Vec<StringArtefact>> {
        match self.backend.as_str() {
            "parquet" => self.read_string_artefacts_parquet(),
            "jsonl" => self.read_string_artefacts_jsonl(),
            "csv" => self.read_string_artefacts_csv(),
            _ => bail!("Unknown metadata backend: {}", self.backend),
        }
    }

    /// Read string artefacts from Parquet
    fn read_string_artefacts_parquet(&self) -> Result<Vec<StringArtefact>> {
        let parquet_dir = self.run_path.join("parquet");
        let mut all_artefacts = Vec::new();

        // Read URLs
        let urls_path = parquet_dir.join("artefacts_urls.parquet");
        if urls_path.exists() {
            all_artefacts.extend(self.read_url_artefacts(&urls_path)?);
        }

        // Read emails
        let emails_path = parquet_dir.join("artefacts_emails.parquet");
        if emails_path.exists() {
            all_artefacts.extend(self.read_email_artefacts(&emails_path)?);
        }

        // Read phones
        let phones_path = parquet_dir.join("artefacts_phones.parquet");
        if phones_path.exists() {
            all_artefacts.extend(self.read_phone_artefacts(&phones_path)?);
        }

        let bitlocker_path = parquet_dir.join("artefacts_bitlocker_recovery_passwords.parquet");
        if bitlocker_path.exists() {
            all_artefacts.extend(self.read_bitlocker_recovery_artefacts(&bitlocker_path)?);
        }

        // Fallback: try legacy strings.parquet
        let legacy_path = parquet_dir.join("strings.parquet");
        if legacy_path.exists() {
            all_artefacts.extend(self.read_legacy_strings(&legacy_path)?);
        }

        Ok(all_artefacts)
    }

    /// Read URL artefacts from Parquet
    fn read_url_artefacts(&self, path: &Path) -> Result<Vec<StringArtefact>> {
        let file = File::open(path)?;
        let reader = ParquetRecordBatchReaderBuilder::try_new(file)?.build()?;

        let mut artefacts = Vec::new();

        for batch_result in reader {
            let batch = batch_result?;

            let url_col = batch
                .column_by_name("url")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let offset_col = batch
                .column_by_name("global_start")
                .and_then(|c| c.as_any().downcast_ref::<Int64Array>());
            let end_col = batch
                .column_by_name("global_end")
                .and_then(|c| c.as_any().downcast_ref::<Int64Array>());

            for i in 0..batch.num_rows() {
                let offset = offset_col.map(|c| c.value(i) as u64).unwrap_or(0);
                let end = end_col.map(|c| c.value(i) as u64).unwrap_or(0);
                let artefact = StringArtefact {
                    artefact_type: "url".to_string(),
                    value: url_col.map(|c| c.value(i).to_string()).unwrap_or_default(),
                    offset,
                    length: end.saturating_sub(offset),
                };
                artefacts.push(artefact);
            }
        }

        Ok(artefacts)
    }

    /// Read email artefacts from Parquet
    fn read_email_artefacts(&self, path: &Path) -> Result<Vec<StringArtefact>> {
        let file = File::open(path)?;
        let reader = ParquetRecordBatchReaderBuilder::try_new(file)?.build()?;

        let mut artefacts = Vec::new();

        for batch_result in reader {
            let batch = batch_result?;

            let email_col = batch
                .column_by_name("email")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let offset_col = batch
                .column_by_name("global_start")
                .and_then(|c| c.as_any().downcast_ref::<Int64Array>());
            let end_col = batch
                .column_by_name("global_end")
                .and_then(|c| c.as_any().downcast_ref::<Int64Array>());

            for i in 0..batch.num_rows() {
                let offset = offset_col.map(|c| c.value(i) as u64).unwrap_or(0);
                let end = end_col.map(|c| c.value(i) as u64).unwrap_or(0);
                let artefact = StringArtefact {
                    artefact_type: "email".to_string(),
                    value: email_col
                        .map(|c| c.value(i).to_string())
                        .unwrap_or_default(),
                    offset,
                    length: end.saturating_sub(offset),
                };
                artefacts.push(artefact);
            }
        }

        Ok(artefacts)
    }

    /// Read phone artefacts from Parquet
    fn read_phone_artefacts(&self, path: &Path) -> Result<Vec<StringArtefact>> {
        let file = File::open(path)?;
        let reader = ParquetRecordBatchReaderBuilder::try_new(file)?.build()?;

        let mut artefacts = Vec::new();

        for batch_result in reader {
            let batch = batch_result?;

            let phone_col = batch
                .column_by_name("phone_e164")
                .or_else(|| batch.column_by_name("phone_raw"))
                .or_else(|| batch.column_by_name("phone"))
                .or_else(|| batch.column_by_name("number"))
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let offset_col = batch
                .column_by_name("global_start")
                .and_then(|c| c.as_any().downcast_ref::<Int64Array>());
            let end_col = batch
                .column_by_name("global_end")
                .and_then(|c| c.as_any().downcast_ref::<Int64Array>());

            for i in 0..batch.num_rows() {
                let offset = offset_col.map(|c| c.value(i) as u64).unwrap_or(0);
                let end = end_col.map(|c| c.value(i) as u64).unwrap_or(0);
                let artefact = StringArtefact {
                    artefact_type: "phone".to_string(),
                    value: phone_col
                        .map(|c| c.value(i).to_string())
                        .unwrap_or_default(),
                    offset,
                    length: end.saturating_sub(offset),
                };
                artefacts.push(artefact);
            }
        }

        Ok(artefacts)
    }

    /// Read BitLocker recovery password artefacts from Parquet
    fn read_bitlocker_recovery_artefacts(&self, path: &Path) -> Result<Vec<StringArtefact>> {
        let file = File::open(path)?;
        let reader = ParquetRecordBatchReaderBuilder::try_new(file)?.build()?;

        let mut artefacts = Vec::new();

        for batch_result in reader {
            let batch = batch_result?;

            let password_col = batch
                .column_by_name("recovery_password")
                .or_else(|| batch.column_by_name("content"))
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let offset_col = batch
                .column_by_name("global_start")
                .and_then(|c| c.as_any().downcast_ref::<Int64Array>());
            let end_col = batch
                .column_by_name("global_end")
                .and_then(|c| c.as_any().downcast_ref::<Int64Array>());

            for i in 0..batch.num_rows() {
                let offset = offset_col.map(|c| c.value(i) as u64).unwrap_or(0);
                let end = end_col.map(|c| c.value(i) as u64).unwrap_or(0);
                let artefact = StringArtefact {
                    artefact_type: "bitlocker_recovery_password".to_string(),
                    value: password_col
                        .map(|c| c.value(i).to_string())
                        .unwrap_or_default(),
                    offset,
                    length: end.saturating_sub(offset),
                };
                artefacts.push(artefact);
            }
        }

        Ok(artefacts)
    }

    /// Read legacy strings.parquet format
    fn read_legacy_strings(&self, path: &Path) -> Result<Vec<StringArtefact>> {
        let file = File::open(path)?;
        let reader = ParquetRecordBatchReaderBuilder::try_new(file)?.build()?;

        let mut artefacts = Vec::new();

        for batch_result in reader {
            let batch = batch_result?;

            let type_col = batch
                .column_by_name("type")
                .or_else(|| batch.column_by_name("artefact_type"))
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let value_col = batch
                .column_by_name("value")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let offset_col = batch
                .column_by_name("offset")
                .and_then(|c| c.as_any().downcast_ref::<UInt64Array>());
            let length_col = batch
                .column_by_name("length")
                .and_then(|c| c.as_any().downcast_ref::<UInt64Array>());

            for i in 0..batch.num_rows() {
                let artefact = StringArtefact {
                    artefact_type: type_col.map(|c| c.value(i).to_string()).unwrap_or_default(),
                    value: value_col
                        .map(|c| c.value(i).to_string())
                        .unwrap_or_default(),
                    offset: offset_col.map(|c| c.value(i)).unwrap_or(0),
                    length: length_col.map(|c| c.value(i)).unwrap_or(0),
                };
                artefacts.push(artefact);
            }
        }

        Ok(artefacts)
    }

    /// Read string artefacts from JSONL
    fn read_string_artefacts_jsonl(&self) -> Result<Vec<StringArtefact>> {
        let metadata_dir = self.run_path.join("metadata");
        let jsonl_path = {
            let current = metadata_dir.join("string_artefacts.jsonl");
            if current.exists() {
                current
            } else {
                metadata_dir.join("strings.jsonl")
            }
        };

        if !jsonl_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&jsonl_path)?;
        let reader = BufReader::new(file);

        let mut artefacts = Vec::new();

        for (line_no, line) in reader.lines().enumerate() {
            let line = line?;
            if line.is_empty() {
                continue;
            }

            let value: Value = serde_json::from_str(&line)
                .with_context(|| format!("Failed to parse string artefact row {}", line_no + 1))?;
            let artefact = Self::string_artefact_from_json(&value)
                .with_context(|| format!("Failed to parse string artefact row {}", line_no + 1))?;
            artefacts.push(artefact);
        }

        Ok(artefacts)
    }

    /// Read string artefacts from CSV
    fn read_string_artefacts_csv(&self) -> Result<Vec<StringArtefact>> {
        let csv_path = self.run_path.join("metadata").join("string_artefacts.csv");

        if !csv_path.exists() {
            return Ok(Vec::new());
        }

        let mut artefacts = Vec::new();

        for_each_csv_row(&csv_path, |_idx, row| {
            let raw_type = csv_string(row, &["artefact_type", "artefact_kind"])
                .unwrap_or_else(|| "string".to_string());
            let artefact_type = normalize_artefact_type(&raw_type);
            let start = csv_u64(row, &["offset", "global_start"]).unwrap_or(0);
            let end = csv_u64(row, &["global_end"]);
            let length = csv_u64(row, &["length"]).or_else(|| end.map(|e| e.saturating_sub(start)));
            let value_keys: &[&str] = match artefact_type.as_str() {
                "phone" => &["phone_e164", "value", "content", "phone_raw"],
                "bitlocker_recovery_password" => &["recovery_password", "value", "content"],
                _ => &["value", "content", "url", "email"],
            };

            artefacts.push(StringArtefact {
                artefact_type,
                value: csv_string(row, value_keys).unwrap_or_default(),
                offset: start,
                length: length.unwrap_or(0),
            });
            Ok(())
        })?;

        Ok(artefacts)
    }

    fn carved_file_from_json(value: &Value, fallback_id: u64) -> Result<CarvedFile> {
        if value.get("global_start").is_none()
            && value.get("carved_path").is_none()
            && value.get("validated").is_none()
        {
            let mut file: CarvedFile = serde_json::from_value(value.clone())?;
            if file.id == 0 {
                file.id = fallback_id;
            }
            return Ok(file);
        }

        let start = json_u64(value, &["offset", "global_start"]).unwrap_or(0);
        let end = json_u64(value, &["global_end"]);
        let size = json_u64(value, &["size"]).or_else(|| end.map(|e| e.saturating_sub(start)));

        Ok(CarvedFile {
            id: json_u64(value, &["id"]).unwrap_or(fallback_id),
            file_type: json_string(value, &["file_type"]).unwrap_or_default(),
            offset: start,
            size: size.unwrap_or(0),
            output_path: json_string(value, &["output_path", "carved_path"]).unwrap_or_default(),
            sha256: json_string(value, &["sha256"]),
            is_valid: json_bool(value, &["is_valid", "validated"]).unwrap_or(true),
            mime_type: json_string(value, &["mime_type"]),
            width: json_u64(value, &["width"]).and_then(|v| u32::try_from(v).ok()),
            height: json_u64(value, &["height"]).and_then(|v| u32::try_from(v).ok()),
        })
    }

    fn string_artefact_from_json(value: &Value) -> Result<StringArtefact> {
        if value.get("artefact_kind").is_none()
            && value.get("content").is_none()
            && value.get("global_start").is_none()
        {
            return Ok(serde_json::from_value(value.clone())?);
        }

        let raw_type = json_string(value, &["artefact_type", "artefact_kind"])
            .unwrap_or_else(|| "string".to_string());
        let artefact_type = normalize_artefact_type(&raw_type);
        let start = json_u64(value, &["offset", "global_start"]).unwrap_or(0);
        let end = json_u64(value, &["global_end"]);
        let length = json_u64(value, &["length"]).or_else(|| end.map(|e| e.saturating_sub(start)));

        let value_keys: &[&str] = match artefact_type.as_str() {
            "phone" => &["phone_e164", "value", "content", "phone_raw"],
            "bitlocker_recovery_password" => &["recovery_password", "value", "content"],
            _ => &["value", "content", "url", "email"],
        };

        Ok(StringArtefact {
            artefact_type,
            value: json_string(value, value_keys).unwrap_or_default(),
            offset: start,
            length: length.unwrap_or(0),
        })
    }

    /// Get summary of metadata
    pub fn get_summary(&self) -> Result<MetadataSummary> {
        let files = self.read_carved_files()?;
        let strings = self.read_string_artefacts()?;

        let mut by_type: HashMap<String, usize> = HashMap::new();
        let mut total_bytes = 0u64;

        for file in &files {
            *by_type.entry(file.file_type.clone()).or_insert(0) += 1;
            total_bytes += file.size;
        }

        Ok(MetadataSummary {
            total_files: files.len(),
            by_type,
            total_bytes,
            string_artefacts: strings.len(),
        })
    }
}

fn json_string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .map(str::to_string)
}

fn json_bool(value: &Value, keys: &[&str]) -> Option<bool> {
    keys.iter().find_map(|key| {
        let value = value.get(*key)?;
        if let Some(value) = value.as_bool() {
            return Some(value);
        }
        let Some(value) = value.as_str() else {
            tracing::warn!("Unrecognised JSON boolean value for metadata field {key}");
            return Some(false);
        };
        parse_bool_value(value, key)
    })
}

fn json_u64(value: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|key| {
        let value = value.get(*key)?;
        if let Some(n) = value.as_u64() {
            Some(n)
        } else {
            value.as_i64().and_then(|n| u64::try_from(n).ok())
        }
    })
}

fn csv_string(row: &HashMap<String, String>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| row.get(*key))
        .filter(|value| !value.is_empty())
        .cloned()
}

fn csv_bool(row: &HashMap<String, String>, keys: &[&str]) -> Option<bool> {
    keys.iter()
        .find_map(|key| {
            row.get(*key)
                .filter(|value| !value.is_empty())
                .map(|value| (*key, value))
        })
        .and_then(|(key, value)| parse_bool_value(value, key))
}

fn csv_u64(row: &HashMap<String, String>, keys: &[&str]) -> Option<u64> {
    csv_string(row, keys).and_then(|value| {
        value.parse::<u64>().ok().or_else(|| {
            value
                .parse::<i64>()
                .ok()
                .and_then(|n| u64::try_from(n).ok())
        })
    })
}

fn parse_bool_value(value: &str, field: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Some(true),
        "false" | "0" | "no" => Some(false),
        _ => {
            tracing::warn!("Unrecognised boolean value for metadata field {field}");
            Some(false)
        }
    }
}

fn for_each_csv_row<F>(path: &Path, mut handle_row: F) -> Result<()>
where
    F: FnMut(usize, &HashMap<String, String>) -> Result<()>,
{
    let file = File::open(path).with_context(|| {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("CSV metadata file");
        format!("Failed to open {name}")
    })?;
    let mut reader = BufReader::new(file);
    let Some(header) = read_csv_record(&mut reader)? else {
        return Ok(());
    };

    let mut row_idx = 0usize;
    while let Some(row) = read_csv_record(&mut reader)
        .with_context(|| format!("Failed to parse CSV metadata row {}", row_idx + 2))?
    {
        if row.iter().all(|cell| cell.is_empty()) {
            continue;
        }

        let mut fields = HashMap::new();
        for (idx, name) in header.iter().enumerate() {
            fields.insert(name.clone(), row.get(idx).cloned().unwrap_or_default());
        }
        handle_row(row_idx, &fields)?;
        row_idx += 1;
    }

    Ok(())
}

fn read_csv_record(reader: &mut impl BufRead) -> Result<Option<Vec<String>>> {
    let mut record = String::new();

    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            if record.is_empty() {
                return Ok(None);
            }
            return Ok(Some(parse_csv_record(&record)?));
        }

        record.push_str(&line);
        if csv_record_is_complete(&record) {
            return Ok(Some(parse_csv_record(&record)?));
        }
    }
}

fn csv_record_is_complete(text: &str) -> bool {
    let mut chars = text.chars().peekable();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            _ => {}
        }
    }

    !in_quotes
}

fn parse_csv_record(text: &str) -> Result<Vec<String>> {
    let mut row = Vec::new();
    let mut field = String::new();
    let mut chars = text.chars().peekable();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => {
                in_quotes = !in_quotes;
            }
            ',' if !in_quotes => {
                row.push(std::mem::take(&mut field));
            }
            '\n' if !in_quotes => {
                row.push(std::mem::take(&mut field));
                return Ok(row);
            }
            '\r' if !in_quotes => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                row.push(std::mem::take(&mut field));
                return Ok(row);
            }
            _ => field.push(ch),
        }
    }

    if in_quotes {
        bail!("Unterminated quoted CSV field");
    }

    row.push(field);
    Ok(row)
}

fn normalize_artefact_type(raw: &str) -> String {
    match raw {
        "Url" | "URL" | "url" => "url",
        "Email" | "email" => "email",
        "Phone" | "phone" => "phone",
        "BitlockerRecoveryPassword"
        | "BitLockerRecoveryPassword"
        | "bitlocker_recovery_password" => "bitlocker_recovery_password",
        "GenericString" | "string" => "string",
        other => other,
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_reader_requires_metadata() {
        let temp = TempDir::new().unwrap();
        let result = MetadataReader::new(temp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_reader_jsonl_empty() {
        let temp = TempDir::new().unwrap();
        let metadata_dir = temp.path().join("metadata");
        fs::create_dir(&metadata_dir).unwrap();
        fs::write(metadata_dir.join("carved_files.jsonl"), "").unwrap();

        let reader = MetadataReader::new(temp.path()).unwrap();
        assert_eq!(reader.backend(), "jsonl");

        let files = reader.read_carved_files().unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_reader_jsonl_with_data() {
        let temp = TempDir::new().unwrap();
        let metadata_dir = temp.path().join("metadata");
        fs::create_dir(&metadata_dir).unwrap();

        let data = r#"{"id":1,"file_type":"jpeg","offset":1024,"size":4096,"output_path":"out/1.jpg","is_valid":true}
{"id":2,"file_type":"png","offset":5120,"size":2048,"output_path":"out/2.png","is_valid":true}"#;
        fs::write(metadata_dir.join("carved_files.jsonl"), data).unwrap();

        let reader = MetadataReader::new(temp.path()).unwrap();
        let files = reader.read_carved_files().unwrap();

        assert_eq!(files.len(), 2);
        assert_eq!(files[0].id, 1);
        assert_eq!(files[0].file_type, "jpeg");
        assert_eq!(files[1].id, 2);
        assert_eq!(files[1].file_type, "png");
    }

    #[test]
    fn test_reader_jsonl_v067_carved_file_schema() {
        let temp = TempDir::new().unwrap();
        let metadata_dir = temp.path().join("metadata");
        fs::create_dir(&metadata_dir).unwrap();

        let data = r#"{"run_id":"r1","file_type":"bek","carved_path":"carved/0001.bek","global_start":64,"global_end":192,"sha256":"abc","validated":true}"#;
        fs::write(metadata_dir.join("carved_files.jsonl"), data).unwrap();

        let reader = MetadataReader::new(temp.path()).unwrap();
        let files = reader.read_carved_files().unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_type, "bek");
        assert_eq!(files[0].offset, 64);
        assert_eq!(files[0].size, 128);
        assert_eq!(files[0].output_path, "carved/0001.bek");
        assert_eq!(files[0].sha256.as_deref(), Some("abc"));
        assert!(files[0].is_valid);
    }

    #[test]
    fn test_reader_jsonl_v067_string_artefacts_schema() {
        let temp = TempDir::new().unwrap();
        let metadata_dir = temp.path().join("metadata");
        fs::create_dir(&metadata_dir).unwrap();
        fs::write(metadata_dir.join("carved_files.jsonl"), "").unwrap();

        let data = r#"{"run_id":"r1","artefact_kind":"Phone","content":"+1 415 555 2671","global_start":10,"global_end":24,"phone_e164":"+14155552671","phone_country":"US"}
{"run_id":"r1","artefact_kind":"BitlockerRecoveryPassword","content":"000000-000000-000000-000000-000000-000000-000000-000000","global_start":100,"global_end":155}"#;
        fs::write(metadata_dir.join("string_artefacts.jsonl"), data).unwrap();

        let reader = MetadataReader::new(temp.path()).unwrap();
        let artefacts = reader.read_string_artefacts().unwrap();

        assert_eq!(artefacts.len(), 2);
        assert_eq!(artefacts[0].artefact_type, "phone");
        assert_eq!(artefacts[0].value, "+14155552671");
        assert_eq!(artefacts[0].offset, 10);
        assert_eq!(artefacts[0].length, 14);
        assert_eq!(artefacts[1].artefact_type, "bitlocker_recovery_password");
    }

    #[test]
    fn test_reader_csv_v067_schema() {
        let temp = TempDir::new().unwrap();
        let metadata_dir = temp.path().join("metadata");
        fs::create_dir(&metadata_dir).unwrap();

        let files = "run_id,file_type,path,extension,global_start,global_end,size,md5,sha256,validated,truncated,errors,pattern_id,is_duplicate,duplicate_of_offset,tool_version,config_hash,evidence_path,evidence_sha256\n\
r1,bek,carved/0001.bek,bek,64,192,128,,abc,true,false,,,false,,0.6.7,hash,/evidence/source.dd,\n";
        fs::write(metadata_dir.join("carved_files.csv"), files).unwrap();

        let strings = "run_id,artefact_kind,content,encoding,global_start,global_end,tool_version,config_hash,evidence_path,evidence_sha256\n\
r1,phone,+1 415 555 2671,ascii,10,24,0.6.7,hash,/evidence/source.dd,\n\
r1,bitlocker_recovery_password,\"000000-000000,000000-000000\",ascii,100,125,0.6.7,hash,/evidence/source.dd,\n";
        fs::write(metadata_dir.join("string_artefacts.csv"), strings).unwrap();

        let reader = MetadataReader::new(temp.path()).unwrap();
        assert_eq!(reader.backend(), "csv");

        let files = reader.read_carved_files().unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_type, "bek");
        assert_eq!(files[0].offset, 64);
        assert_eq!(files[0].size, 128);
        assert_eq!(files[0].output_path, "carved/0001.bek");
        assert_eq!(files[0].sha256.as_deref(), Some("abc"));
        assert!(files[0].is_valid);

        let artefacts = reader.read_string_artefacts().unwrap();
        assert_eq!(artefacts.len(), 2);
        assert_eq!(artefacts[0].artefact_type, "phone");
        assert_eq!(artefacts[0].value, "+1 415 555 2671");
        assert_eq!(artefacts[0].length, 14);
        assert_eq!(artefacts[1].artefact_type, "bitlocker_recovery_password");
        assert_eq!(artefacts[1].value, "000000-000000,000000-000000");
    }

    #[test]
    fn test_reader_csv_unknown_validated_value_is_not_valid() {
        let temp = TempDir::new().unwrap();
        let metadata_dir = temp.path().join("metadata");
        fs::create_dir(&metadata_dir).unwrap();

        let files = "run_id,file_type,path,global_start,global_end,size,validated\n\
r1,bek,carved/0001.bek,64,192,128,validated\n";
        fs::write(metadata_dir.join("carved_files.csv"), files).unwrap();

        let reader = MetadataReader::new(temp.path()).unwrap();
        let files = reader.read_carved_files().unwrap();

        assert_eq!(files.len(), 1);
        assert!(!files[0].is_valid);
    }

    #[test]
    fn test_reader_csv_missing_carved_file_has_clear_error() {
        let temp = TempDir::new().unwrap();
        let metadata_dir = temp.path().join("metadata");
        fs::create_dir(&metadata_dir).unwrap();
        let carved_path = metadata_dir.join("carved_files.csv");
        fs::write(&carved_path, "").unwrap();

        let reader = MetadataReader::new(temp.path()).unwrap();
        fs::remove_file(carved_path).unwrap();

        let err = reader.read_carved_files().unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("Failed to open carved_files.csv"));
    }

    #[test]
    fn test_reader_summary() {
        let temp = TempDir::new().unwrap();
        let metadata_dir = temp.path().join("metadata");
        fs::create_dir(&metadata_dir).unwrap();

        let data = r#"{"id":1,"file_type":"jpeg","offset":0,"size":1000,"output_path":"1.jpg","is_valid":true}
{"id":2,"file_type":"jpeg","offset":1000,"size":2000,"output_path":"2.jpg","is_valid":true}
{"id":3,"file_type":"png","offset":3000,"size":500,"output_path":"3.png","is_valid":true}"#;
        fs::write(metadata_dir.join("carved_files.jsonl"), data).unwrap();

        let reader = MetadataReader::new(temp.path()).unwrap();
        let summary = reader.get_summary().unwrap();

        assert_eq!(summary.total_files, 3);
        assert_eq!(summary.total_bytes, 3500);
        assert_eq!(summary.by_type.get("jpeg"), Some(&2));
        assert_eq!(summary.by_type.get("png"), Some(&1));
    }
}
