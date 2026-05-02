//! Metadata reader for Parquet, JSONL, and CSV files

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use arrow::array::{
    Array, BooleanArray, Int16Array, Int32Array, Int64Array, Int8Array, StringArray, UInt16Array,
    UInt32Array, UInt64Array, UInt8Array,
};
use arrow::record_batch::RecordBatch;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde::Deserialize;

use super::types::{CarvedFile, StringArtefact};

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
        let mut next_id: u64 = 0;

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
                let files = self.read_parquet_file(&path, &mut next_id)?;
                all_files.extend(files);
            }
        }

        Ok(all_files)
    }

    /// Read a single Parquet file
    fn read_parquet_file(&self, path: &Path, next_id: &mut u64) -> Result<Vec<CarvedFile>> {
        let file = File::open(path)?;
        let reader = ParquetRecordBatchReaderBuilder::try_new(file)?.build()?;

        let mut files = Vec::new();

        for batch_result in reader {
            let batch = batch_result?;

            for i in 0..batch.num_rows() {
                let id = parquet_u64_field(&batch, &["id"], i)?.unwrap_or(*next_id);
                let global_start =
                    parquet_u64_field(&batch, &["global_start", "offset", "start"], i)?
                        .unwrap_or(0);
                let size = parquet_u64_field(&batch, &["size", "length"], i)?.unwrap_or(0);
                let global_end = parquet_u64_field(&batch, &["global_end", "end"], i)?.unwrap_or(
                    global_start.checked_add(size).with_context(|| {
                        format!("Parquet row {i} global_start + size overflowed")
                    })?,
                );
                let path = parquet_string_field(
                    &batch,
                    &["carved_path", "path", "output_path", "filename"],
                    i,
                )?
                .unwrap_or_default();
                let error = parquet_string_field(&batch, &["error", "errors"], i)?;
                let validated = parquet_bool_field(&batch, &["validated", "is_valid"], i)?;

                let file = CarvedFile {
                    id,
                    run_id: parquet_string_field(&batch, &["run_id"], i)?,
                    file_type: parquet_string_field(&batch, &["file_type", "type"], i)?
                        .unwrap_or_default(),
                    extension: parquet_string_field(&batch, &["extension"], i)?
                        .or_else(|| extension_from_path(&path)),
                    global_start,
                    global_end,
                    size,
                    path,
                    handler_id: parquet_string_field(&batch, &["handler_id"], i)?,
                    md5: parquet_string_field(&batch, &["md5"], i)?,
                    sha256: parquet_string_field(&batch, &["sha256", "sha256_hex"], i)?,
                    validated,
                    truncated: parquet_bool_field(&batch, &["truncated"], i)?.unwrap_or(false),
                    errors: error_values(error),
                    pattern_id: parquet_string_field(&batch, &["pattern_id"], i)?,
                    is_duplicate: parquet_bool_field(&batch, &["is_duplicate"], i)?
                        .unwrap_or(false),
                    duplicate_of_offset: parquet_u64_field(&batch, &["duplicate_of_offset"], i)?,
                    mime_type: parquet_string_field(&batch, &["mime_type", "mime"], i)?,
                    width: parquet_u32_field(&batch, &["width"], i)?,
                    height: parquet_u32_field(&batch, &["height"], i)?,
                };
                files.push(file);
                *next_id = next_id
                    .checked_add(1)
                    .context("Exhausted Parquet fallback id space")?;
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

        for (line_index, line) in reader.lines().enumerate() {
            let line = line?;
            if line.is_empty() {
                continue;
            }

            let row: JsonCarvedFileRow = serde_json::from_str(&line).with_context(|| {
                format!("Failed to parse carved_files.jsonl line {}", line_index + 1)
            })?;
            files.push(row.into_carved_file(line_index as u64)?);
        }

        Ok(files)
    }

    /// Read carved files from CSV
    fn read_carved_files_csv(&self) -> Result<Vec<CarvedFile>> {
        let csv_path = self.run_path.join("metadata").join("carved_files.csv");
        let mut reader = csv::Reader::from_path(&csv_path)
            .with_context(|| "Failed to open CSV carved file metadata")?;
        let headers = reader
            .headers()
            .with_context(|| "Failed to read CSV carved file headers")?
            .clone();

        let mut files = Vec::new();
        let mut explicit_ids = HashSet::new();
        let mut missing_id_indexes = Vec::new();

        for (row_index, record) in reader.records().enumerate() {
            let record = record
                .with_context(|| format!("Failed to read CSV carved file row {}", row_index + 1))?;
            let id = match csv_u64_field(&headers, &record, &["id"])? {
                Some(id) => {
                    explicit_ids.insert(id);
                    id
                }
                None => {
                    missing_id_indexes.push(files.len());
                    0
                }
            };
            let global_start =
                csv_u64_field(&headers, &record, &["global_start", "offset", "start"])?
                    .unwrap_or(0);
            let size = csv_u64_field(&headers, &record, &["size", "length"])?.unwrap_or(0);
            let global_end = csv_u64_field(&headers, &record, &["global_end", "end"])?.unwrap_or(
                global_start.checked_add(size).with_context(|| {
                    format!(
                        "CSV carved file row {} global_start + size overflowed",
                        row_index + 1
                    )
                })?,
            );
            let path = csv_string_field(
                &headers,
                &record,
                &["path", "carved_path", "output_path", "filename"],
            );
            let errors = error_values(csv_optional_string_field(
                &headers,
                &record,
                &["errors", "error"],
            ));

            let validated = csv_bool_field(&headers, &record, &["validated", "is_valid"])?;

            let file = CarvedFile {
                id,
                run_id: csv_optional_string_field(&headers, &record, &["run_id"]),
                file_type: csv_string_field(&headers, &record, &["file_type", "type"]),
                path,
                extension: csv_optional_string_field(&headers, &record, &["extension"]),
                global_start,
                global_end,
                size,
                handler_id: csv_optional_string_field(&headers, &record, &["handler_id"]),
                md5: csv_optional_string_field(&headers, &record, &["md5"]),
                sha256: csv_optional_string_field(&headers, &record, &["sha256", "sha256_hex"]),
                validated,
                truncated: csv_bool_field(&headers, &record, &["truncated"])?.unwrap_or(false),
                errors,
                pattern_id: csv_optional_string_field(&headers, &record, &["pattern_id"]),
                is_duplicate: csv_bool_field(&headers, &record, &["is_duplicate"])?
                    .unwrap_or(false),
                duplicate_of_offset: csv_u64_field(&headers, &record, &["duplicate_of_offset"])?,
                mime_type: csv_optional_string_field(&headers, &record, &["mime_type", "mime"]),
                width: csv_u32_field(&headers, &record, &["width"])?,
                height: csv_u32_field(&headers, &record, &["height"])?,
            };
            files.push(file);
        }

        for index in missing_id_indexes {
            let mut generated_id = index as u64;
            while explicit_ids.contains(&generated_id) {
                generated_id = generated_id
                    .checked_add(1)
                    .context("Exhausted CSV fallback id space")?;
            }
            files[index].id = generated_id;
            explicit_ids.insert(generated_id);
        }

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
            for i in 0..batch.num_rows() {
                let offset =
                    parquet_u64_field(&batch, &["global_start", "offset"], i)?.unwrap_or(0);
                let end = parquet_u64_field(&batch, &["global_end", "end"], i)?.unwrap_or(0);
                let artefact = StringArtefact {
                    artefact_type: "url".to_string(),
                    value: url_col.map(|c| c.value(i).to_string()).unwrap_or_default(),
                    offset,
                    length: parquet_span_length("URL", i, offset, end)?,
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
            for i in 0..batch.num_rows() {
                let offset =
                    parquet_u64_field(&batch, &["global_start", "offset"], i)?.unwrap_or(0);
                let end = parquet_u64_field(&batch, &["global_end", "end"], i)?.unwrap_or(0);
                let artefact = StringArtefact {
                    artefact_type: "email".to_string(),
                    value: email_col
                        .map(|c| c.value(i).to_string())
                        .unwrap_or_default(),
                    offset,
                    length: parquet_span_length("email", i, offset, end)?,
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
                .column_by_name("phone")
                .or_else(|| batch.column_by_name("phone_raw"))
                .or_else(|| batch.column_by_name("number"))
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            for i in 0..batch.num_rows() {
                let offset =
                    parquet_u64_field(&batch, &["global_start", "offset"], i)?.unwrap_or(0);
                let end = parquet_u64_field(&batch, &["global_end", "end"], i)?.unwrap_or(0);
                let artefact = StringArtefact {
                    artefact_type: "phone".to_string(),
                    value: phone_col
                        .map(|c| c.value(i).to_string())
                        .unwrap_or_default(),
                    offset,
                    length: parquet_span_length("phone", i, offset, end)?,
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
        let jsonl_path = metadata_dir.join("string_artefacts.jsonl");
        let legacy_jsonl_path = metadata_dir.join("strings.jsonl");
        let jsonl_path = if jsonl_path.exists() {
            jsonl_path
        } else {
            legacy_jsonl_path
        };

        if !jsonl_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&jsonl_path)?;
        let reader = BufReader::new(file);

        let mut artefacts = Vec::new();

        for (line_index, line) in reader.lines().enumerate() {
            let line = line?;
            if line.is_empty() {
                continue;
            }

            let artefact: JsonStringArtefactRow =
                serde_json::from_str(&line).with_context(|| {
                    format!(
                        "Failed to parse {} line {}",
                        metadata_file_name(&jsonl_path),
                        line_index + 1
                    )
                })?;
            artefacts.push(artefact.into_string_artefact(&jsonl_path, line_index + 1)?);
        }

        Ok(artefacts)
    }

    /// Read string artefacts from CSV
    fn read_string_artefacts_csv(&self) -> Result<Vec<StringArtefact>> {
        let metadata_dir = self.run_path.join("metadata");
        let mut all_artefacts = Vec::new();

        all_artefacts.extend(self.read_named_csv_artefacts(
            &metadata_dir.join("artefacts_urls.csv"),
            "url",
            &["url", "value"],
        )?);
        all_artefacts.extend(self.read_named_csv_artefacts(
            &metadata_dir.join("artefacts_emails.csv"),
            "email",
            &["email", "value"],
        )?);
        all_artefacts.extend(self.read_named_csv_artefacts(
            &metadata_dir.join("artefacts_phones.csv"),
            "phone",
            &["phone", "number", "value"],
        )?);
        all_artefacts.extend(self.read_csv_artefacts(&metadata_dir.join("strings.csv"))?);

        Ok(all_artefacts)
    }

    fn read_named_csv_artefacts(
        &self,
        path: &Path,
        artefact_type: &str,
        value_columns: &[&str],
    ) -> Result<Vec<StringArtefact>> {
        if !path.exists() {
            return Ok(Vec::new());
        }

        self.read_csv_artefact_file(path, Some(artefact_type), value_columns)
    }

    fn read_csv_artefacts(&self, path: &Path) -> Result<Vec<StringArtefact>> {
        if !path.exists() {
            return Ok(Vec::new());
        }

        self.read_csv_artefact_file(path, None, &["value", "string"])
    }

    fn read_csv_artefact_file(
        &self,
        path: &Path,
        fixed_type: Option<&str>,
        value_columns: &[&str],
    ) -> Result<Vec<StringArtefact>> {
        let mut reader = csv::Reader::from_path(path)
            .with_context(|| "Failed to open CSV string artefact metadata")?;
        let headers = reader
            .headers()
            .with_context(|| "Failed to read CSV string artefact headers")?
            .clone();

        let mut artefacts = Vec::new();

        for (row_index, record) in reader.records().enumerate() {
            let record = record.with_context(|| {
                format!("Failed to read CSV string artefact row {}", row_index + 1)
            })?;
            let offset = csv_u64_field(&headers, &record, &["offset", "global_start", "start"])?
                .unwrap_or(0);
            let end = csv_u64_field(&headers, &record, &["global_end", "end"])?;
            let length = match csv_u64_field(&headers, &record, &["length"])? {
                Some(length) => length,
                None => match end {
                    Some(end) => end.checked_sub(offset).with_context(|| {
                        format!(
                            "CSV string artefact row {} global_end precedes global_start",
                            row_index + 1
                        )
                    })?,
                    None => 0,
                },
            };
            let artefact_type = fixed_type
                .map(str::to_string)
                .unwrap_or_else(|| csv_string_field(&headers, &record, &["artefact_type", "type"]));

            artefacts.push(StringArtefact {
                artefact_type,
                value: csv_string_field(&headers, &record, value_columns),
                offset,
                length,
            });
        }

        Ok(artefacts)
    }
}

#[derive(Debug, Deserialize)]
struct JsonCarvedFileRow {
    #[serde(default)]
    id: Option<u64>,
    #[serde(default)]
    run_id: Option<String>,
    #[serde(default)]
    file_type: String,
    #[serde(default, alias = "output_path", alias = "carved_path")]
    path: String,
    #[serde(default)]
    extension: Option<String>,
    #[serde(default, alias = "offset")]
    global_start: Option<u64>,
    #[serde(default)]
    global_end: Option<u64>,
    #[serde(default)]
    size: Option<u64>,
    #[serde(default)]
    handler_id: Option<String>,
    #[serde(default)]
    md5: Option<String>,
    #[serde(default)]
    sha256: Option<String>,
    #[serde(default, alias = "is_valid")]
    validated: Option<bool>,
    #[serde(default)]
    truncated: Option<bool>,
    #[serde(default)]
    errors: Vec<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    pattern_id: Option<String>,
    #[serde(default)]
    is_duplicate: Option<bool>,
    #[serde(default)]
    duplicate_of_offset: Option<u64>,
    #[serde(default)]
    mime_type: Option<String>,
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
}

impl JsonCarvedFileRow {
    fn into_carved_file(mut self, fallback_id: u64) -> Result<CarvedFile> {
        let global_start = self.global_start.unwrap_or(0);
        let size = self.size.unwrap_or(0);
        let global_end =
            self.global_end
                .unwrap_or(global_start.checked_add(size).with_context(|| {
                    format!("JSONL row {fallback_id} global_start + size overflowed")
                })?);
        if let Some(error) = self.error.take().filter(|error| !error.is_empty()) {
            self.errors.push(error);
        }

        Ok(CarvedFile {
            id: self.id.unwrap_or(fallback_id),
            run_id: self.run_id,
            file_type: self.file_type,
            extension: self.extension.or_else(|| extension_from_path(&self.path)),
            global_start,
            global_end,
            size,
            path: self.path,
            handler_id: self.handler_id,
            md5: self.md5,
            sha256: self.sha256,
            validated: self.validated,
            truncated: self.truncated.unwrap_or(false),
            errors: self.errors,
            pattern_id: self.pattern_id,
            is_duplicate: self.is_duplicate.unwrap_or(false),
            duplicate_of_offset: self.duplicate_of_offset,
            mime_type: self.mime_type,
            width: self.width,
            height: self.height,
        })
    }
}

#[derive(Debug, Deserialize)]
struct JsonStringArtefactRow {
    #[serde(default, alias = "artefact_kind")]
    artefact_type: String,
    #[serde(default, alias = "content")]
    value: String,
    #[serde(default, alias = "global_start")]
    offset: u64,
    #[serde(default)]
    global_end: Option<u64>,
    #[serde(default)]
    length: Option<u64>,
}

impl JsonStringArtefactRow {
    fn into_string_artefact(
        self,
        source_path: &Path,
        line_number: usize,
    ) -> Result<StringArtefact> {
        let length = match self.length {
            Some(length) => length,
            None => match self.global_end {
                Some(end) => end.checked_sub(self.offset).with_context(|| {
                    format!(
                        "{} line {} global_end precedes global_start",
                        metadata_file_name(source_path),
                        line_number
                    )
                })?,
                None => 0,
            },
        };

        Ok(StringArtefact {
            artefact_type: self.artefact_type,
            value: self.value,
            offset: self.offset,
            length,
        })
    }
}

fn parquet_string_field(
    batch: &RecordBatch,
    names: &[&str],
    row_index: usize,
) -> Result<Option<String>> {
    for name in names {
        if let Some(column) = batch.column_by_name(name) {
            if column.is_null(row_index) {
                return Ok(None);
            }
            let array = column
                .as_any()
                .downcast_ref::<StringArray>()
                .with_context(|| format!("Parquet column {name} is not a UTF-8 string column"))?;
            return Ok(Some(array.value(row_index).to_string()));
        }
    }
    Ok(None)
}

fn parquet_bool_field(
    batch: &RecordBatch,
    names: &[&str],
    row_index: usize,
) -> Result<Option<bool>> {
    for name in names {
        if let Some(column) = batch.column_by_name(name) {
            if column.is_null(row_index) {
                return Ok(None);
            }
            let array = column
                .as_any()
                .downcast_ref::<BooleanArray>()
                .with_context(|| format!("Parquet column {name} is not a boolean column"))?;
            return Ok(Some(array.value(row_index)));
        }
    }
    Ok(None)
}

fn parquet_u64_field(batch: &RecordBatch, names: &[&str], row_index: usize) -> Result<Option<u64>> {
    for name in names {
        if let Some(column) = batch.column_by_name(name) {
            if column.is_null(row_index) {
                return Ok(None);
            }
            if let Some(array) = column.as_any().downcast_ref::<UInt64Array>() {
                return Ok(Some(array.value(row_index)));
            }
            if let Some(array) = column.as_any().downcast_ref::<UInt32Array>() {
                return Ok(Some(u64::from(array.value(row_index))));
            }
            if let Some(array) = column.as_any().downcast_ref::<UInt16Array>() {
                return Ok(Some(u64::from(array.value(row_index))));
            }
            if let Some(array) = column.as_any().downcast_ref::<UInt8Array>() {
                return Ok(Some(u64::from(array.value(row_index))));
            }
            if let Some(array) = column.as_any().downcast_ref::<Int64Array>() {
                return u64::try_from(array.value(row_index))
                    .with_context(|| format!("Parquet column {name} contains a negative value"))
                    .map(Some);
            }
            if let Some(array) = column.as_any().downcast_ref::<Int32Array>() {
                return u64::try_from(array.value(row_index))
                    .with_context(|| format!("Parquet column {name} contains a negative value"))
                    .map(Some);
            }
            if let Some(array) = column.as_any().downcast_ref::<Int16Array>() {
                return u64::try_from(array.value(row_index))
                    .with_context(|| format!("Parquet column {name} contains a negative value"))
                    .map(Some);
            }
            if let Some(array) = column.as_any().downcast_ref::<Int8Array>() {
                return u64::try_from(array.value(row_index))
                    .with_context(|| format!("Parquet column {name} contains a negative value"))
                    .map(Some);
            }
            bail!("Parquet column {name} is not an integer column");
        }
    }
    Ok(None)
}

fn parquet_u32_field(batch: &RecordBatch, names: &[&str], row_index: usize) -> Result<Option<u32>> {
    parquet_u64_field(batch, names, row_index)?
        .map(|value| {
            u32::try_from(value)
                .with_context(|| format!("Parquet value {value} does not fit in u32"))
        })
        .transpose()
}

fn parquet_span_length(
    artefact_type: &str,
    row_index: usize,
    offset: u64,
    end: u64,
) -> Result<u64> {
    end.checked_sub(offset).with_context(|| {
        format!("Parquet {artefact_type} artefact row {row_index} global_end precedes global_start")
    })
}

fn error_values(error: Option<String>) -> Vec<String> {
    // SwiftBeaver v0.5.1 writes Parquet `error` as nullable Utf8 and JSONL
    // `errors` as an array; CSV compatibility may contain either a JSON array
    // string or one plain error string.
    match error.filter(|error| !error.is_empty()) {
        Some(error) => serde_json::from_str::<Vec<String>>(&error).unwrap_or_else(|_| vec![error]),
        None => Vec::new(),
    }
}

fn extension_from_path(path: &str) -> Option<String> {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .filter(|extension| !extension.is_empty())
        .map(ToOwned::to_owned)
}

fn metadata_file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("metadata JSONL")
        .to_string()
}

fn csv_field<'a>(
    headers: &csv::StringRecord,
    record: &'a csv::StringRecord,
    names: &[&str],
) -> Option<&'a str> {
    names.iter().find_map(|name| {
        headers
            .iter()
            .position(|header| header == *name)
            .and_then(|index| record.get(index))
            .filter(|value| !value.is_empty())
    })
}

fn csv_string_field(
    headers: &csv::StringRecord,
    record: &csv::StringRecord,
    names: &[&str],
) -> String {
    csv_field(headers, record, names)
        .unwrap_or_default()
        .to_string()
}

fn csv_optional_string_field(
    headers: &csv::StringRecord,
    record: &csv::StringRecord,
    names: &[&str],
) -> Option<String> {
    csv_field(headers, record, names).map(ToOwned::to_owned)
}

fn csv_u64_field(
    headers: &csv::StringRecord,
    record: &csv::StringRecord,
    names: &[&str],
) -> Result<Option<u64>> {
    csv_field(headers, record, names)
        .map(|value| {
            value
                .trim()
                .parse::<u64>()
                .with_context(|| "Invalid unsigned integer CSV value")
        })
        .transpose()
}

fn csv_u32_field(
    headers: &csv::StringRecord,
    record: &csv::StringRecord,
    names: &[&str],
) -> Result<Option<u32>> {
    csv_field(headers, record, names)
        .map(|value| {
            value
                .trim()
                .parse::<u32>()
                .with_context(|| "Invalid unsigned integer CSV value")
        })
        .transpose()
}

fn csv_bool_field(
    headers: &csv::StringRecord,
    record: &csv::StringRecord,
    names: &[&str],
) -> Result<Option<bool>> {
    csv_field(headers, record, names)
        .map(|value| match value.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "y" => Ok(true),
            "false" | "0" | "no" | "n" => Ok(false),
            _ => bail!("Invalid boolean CSV value"),
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::MetadataSummary;
    use arrow::array::{ArrayRef, BooleanArray, Int32Array, Int64Array, StringArray};
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow::record_batch::RecordBatch;
    use parquet::arrow::ArrowWriter;
    use std::fs;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn write_v051_files_parquet(path: &Path) {
        let schema = Arc::new(Schema::new(vec![
            Field::new("run_id", DataType::Utf8, false),
            Field::new("tool_version", DataType::Utf8, false),
            Field::new("config_hash", DataType::Utf8, false),
            Field::new("evidence_path", DataType::Utf8, false),
            Field::new("evidence_sha256", DataType::Utf8, false),
            Field::new("handler_id", DataType::Utf8, false),
            Field::new("file_type", DataType::Utf8, false),
            Field::new("carved_path", DataType::Utf8, false),
            Field::new("global_start", DataType::Int64, false),
            Field::new("global_end", DataType::Int64, false),
            Field::new("size", DataType::Int64, false),
            Field::new("md5", DataType::Utf8, true),
            Field::new("sha256", DataType::Utf8, true),
            Field::new("pattern_id", DataType::Utf8, true),
            Field::new("validated", DataType::Boolean, false),
            Field::new("truncated", DataType::Boolean, false),
            Field::new("error", DataType::Utf8, true),
            Field::new("is_duplicate", DataType::Boolean, false),
            Field::new("duplicate_of_offset", DataType::Int64, true),
        ]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(StringArray::from(vec!["run-1", "run-1"])) as ArrayRef,
                Arc::new(StringArray::from(vec!["0.5.1", "0.5.1"])),
                Arc::new(StringArray::from(vec!["cfg", "cfg"])),
                Arc::new(StringArray::from(vec!["/evidence.dd", "/evidence.dd"])),
                Arc::new(StringArray::from(vec!["evidence-sha", "evidence-sha"])),
                Arc::new(StringArray::from(vec!["jpeg", "jpeg"])),
                Arc::new(StringArray::from(vec!["jpeg", "jpeg"])),
                Arc::new(StringArray::from(vec![
                    "jpeg/jpeg_000000000400.jpg",
                    "jpeg/jpeg_000000000800.jpg",
                ])),
                Arc::new(Int64Array::from(vec![1024, 2048])),
                Arc::new(Int64Array::from(vec![1056, 4096])),
                Arc::new(Int64Array::from(vec![32, 2048])),
                Arc::new(StringArray::from(vec![Some("md5-a"), None])),
                Arc::new(StringArray::from(vec![Some("sha-a"), Some("sha-b")])),
                Arc::new(StringArray::from(vec![Some("jpeg_soi"), None])),
                Arc::new(BooleanArray::from(vec![true, false])),
                Arc::new(BooleanArray::from(vec![false, true])),
                Arc::new(StringArray::from(vec![None, Some("truncated footer")])),
                Arc::new(BooleanArray::from(vec![false, true])),
                Arc::new(Int64Array::from(vec![None, Some(1024)])),
            ],
        )
        .unwrap();
        let file = File::create(path).unwrap();
        let mut writer = ArrowWriter::try_new(file, schema, None).unwrap();
        writer.write(&batch).unwrap();
        writer.finish().unwrap();
    }

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
        assert_eq!(files[0].path, "out/1.jpg");
        assert_eq!(files[0].global_start, 1024);
        assert_eq!(files[0].global_end, 5120);
        assert_eq!(files[0].validated, Some(true));
        assert_eq!(files[1].id, 2);
        assert_eq!(files[1].file_type, "png");
    }

    #[test]
    fn test_reader_jsonl_missing_validation_is_not_run() {
        let temp = TempDir::new().unwrap();
        let metadata_dir = temp.path().join("metadata");
        fs::create_dir(&metadata_dir).unwrap();

        let data =
            r#"{"id":1,"file_type":"jpeg","offset":1024,"size":4096,"output_path":"out/1.jpg"}"#;
        fs::write(metadata_dir.join("carved_files.jsonl"), data).unwrap();

        let reader = MetadataReader::new(temp.path()).unwrap();
        let files = reader.read_carved_files().unwrap();

        assert_eq!(files[0].validated, None);
    }

    #[test]
    fn test_reader_jsonl_v051_with_data() {
        let temp = TempDir::new().unwrap();
        let metadata_dir = temp.path().join("metadata");
        fs::create_dir(&metadata_dir).unwrap();

        let data = r#"{"run_id":"run-1","file_type":"jpeg","path":"jpeg/jpeg_000000000400.jpg","extension":"jpg","global_start":1024,"global_end":1056,"size":32,"md5":"md5-a","sha256":"sha-a","validated":false,"truncated":true,"errors":["short footer"],"pattern_id":"jpeg_soi","is_duplicate":true,"duplicate_of_offset":512}
{"run_id":"run-1","file_type":"png","path":"png/png_000000000800.png","extension":"png","global_start":2048,"global_end":4096,"size":2048,"md5":null,"sha256":"sha-b","validated":true,"truncated":false,"errors":[],"pattern_id":"png_sig","is_duplicate":false,"duplicate_of_offset":null}"#;
        fs::write(metadata_dir.join("carved_files.jsonl"), data).unwrap();

        let reader = MetadataReader::new(temp.path()).unwrap();
        let files = reader.read_carved_files().unwrap();

        assert_eq!(files.len(), 2);
        assert_eq!(files[0].id, 0);
        assert_eq!(files[0].run_id.as_deref(), Some("run-1"));
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
        assert_eq!(files[1].id, 1);
        assert!(!files[1].is_duplicate);
    }

    #[test]
    fn test_reader_parquet_v051_with_data() {
        let temp = TempDir::new().unwrap();
        let parquet_dir = temp.path().join("parquet");
        fs::create_dir(&parquet_dir).unwrap();
        write_v051_files_parquet(&parquet_dir.join("files_jpeg.parquet"));

        let reader = MetadataReader::new(temp.path()).unwrap();
        assert_eq!(reader.backend(), "parquet");
        let files = reader.read_carved_files().unwrap();

        assert_eq!(files.len(), 2);
        assert_eq!(files[0].id, 0);
        assert_eq!(files[0].run_id.as_deref(), Some("run-1"));
        assert_eq!(files[0].handler_id.as_deref(), Some("jpeg"));
        assert_eq!(files[0].file_type, "jpeg");
        assert_eq!(files[0].path, "jpeg/jpeg_000000000400.jpg");
        assert_eq!(files[0].extension.as_deref(), Some("jpg"));
        assert_eq!(files[0].global_start, 1024);
        assert_eq!(files[0].global_end, 1056);
        assert_eq!(files[0].size, 32);
        assert_eq!(files[0].md5.as_deref(), Some("md5-a"));
        assert_eq!(files[0].sha256.as_deref(), Some("sha-a"));
        assert_eq!(files[0].validated, Some(true));
        assert!(!files[0].truncated);
        assert_eq!(files[0].pattern_id.as_deref(), Some("jpeg_soi"));
        assert!(!files[0].is_duplicate);

        assert_eq!(files[1].id, 1);
        assert_eq!(files[1].validated, Some(false));
        assert!(files[1].truncated);
        assert_eq!(files[1].errors, vec!["truncated footer"]);
        assert!(files[1].is_duplicate);
        assert_eq!(files[1].duplicate_of_offset, Some(1024));
    }

    #[test]
    fn test_parquet_integer_helper_accepts_int32() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("width", DataType::Int32, false),
            Field::new("duplicate_of_offset", DataType::Int32, true),
        ]));
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(Int32Array::from(vec![800])) as ArrayRef,
                Arc::new(Int32Array::from(vec![Some(1024)])),
            ],
        )
        .unwrap();

        assert_eq!(parquet_u32_field(&batch, &["width"], 0).unwrap(), Some(800));
        assert_eq!(
            parquet_u64_field(&batch, &["duplicate_of_offset"], 0).unwrap(),
            Some(1024)
        );
    }

    #[test]
    fn test_reader_csv_with_data() {
        let temp = TempDir::new().unwrap();
        let metadata_dir = temp.path().join("metadata");
        fs::create_dir(&metadata_dir).unwrap();

        let data = r#"id,file_type,global_start,size,carved_path,sha256,validated,mime_type,width,height
7,jpeg,1024,4096,"carved/with, comma.jpg",abc123,true,image/jpeg,800,600
,png,5120,2048,carved/2.png,,false,,,"#;
        fs::write(metadata_dir.join("carved_files.csv"), data).unwrap();

        let reader = MetadataReader::new(temp.path()).unwrap();
        assert_eq!(reader.backend(), "csv");

        let files = reader.read_carved_files().unwrap();

        assert_eq!(files.len(), 2);
        assert_eq!(files[0].id, 7);
        assert_eq!(files[0].file_type, "jpeg");
        assert_eq!(files[0].global_start, 1024);
        assert_eq!(files[0].global_end, 5120);
        assert_eq!(files[0].size, 4096);
        assert_eq!(files[0].path, "carved/with, comma.jpg");
        assert_eq!(files[0].sha256.as_deref(), Some("abc123"));
        assert_eq!(files[0].validated, Some(true));
        assert_eq!(files[0].mime_type.as_deref(), Some("image/jpeg"));
        assert_eq!(files[0].width, Some(800));
        assert_eq!(files[0].height, Some(600));
        assert_eq!(files[1].id, 1);
        assert_eq!(files[1].sha256, None);
        assert_eq!(files[1].validated, Some(false));
        assert_eq!(files[1].mime_type, None);
        assert_eq!(files[1].width, None);
        assert_eq!(files[1].height, None);
    }

    #[test]
    fn test_reader_csv_missing_validation_is_not_run() {
        let temp = TempDir::new().unwrap();
        let metadata_dir = temp.path().join("metadata");
        fs::create_dir(&metadata_dir).unwrap();
        fs::write(
            metadata_dir.join("carved_files.csv"),
            "id,file_type,global_start,size,carved_path\n1,jpeg,0,100,1.jpg\n",
        )
        .unwrap();

        let reader = MetadataReader::new(temp.path()).unwrap();
        let files = reader.read_carved_files().unwrap();

        assert_eq!(files[0].validated, None);
    }

    #[test]
    fn test_reader_csv_missing_artefacts_are_empty() {
        let temp = TempDir::new().unwrap();
        let metadata_dir = temp.path().join("metadata");
        fs::create_dir(&metadata_dir).unwrap();
        fs::write(
            metadata_dir.join("carved_files.csv"),
            "file_type,global_start,size,carved_path\njpeg,0,100,1.jpg\n",
        )
        .unwrap();

        let reader = MetadataReader::new(temp.path()).unwrap();
        let artefacts = reader.read_string_artefacts().unwrap();

        assert!(artefacts.is_empty());
    }

    #[test]
    fn test_reader_csv_string_artefacts() {
        let temp = TempDir::new().unwrap();
        let metadata_dir = temp.path().join("metadata");
        fs::create_dir(&metadata_dir).unwrap();
        fs::write(
            metadata_dir.join("carved_files.csv"),
            "file_type,global_start,size,carved_path\njpeg,0,100,1.jpg\n",
        )
        .unwrap();
        fs::write(
            metadata_dir.join("artefacts_urls.csv"),
            "url,global_start,global_end\nhttps://example.test,10,30\n",
        )
        .unwrap();
        fs::write(
            metadata_dir.join("strings.csv"),
            "type,value,offset,length\nstring,hello,40,5\n",
        )
        .unwrap();

        let reader = MetadataReader::new(temp.path()).unwrap();
        let artefacts = reader.read_string_artefacts().unwrap();

        assert_eq!(artefacts.len(), 2);
        assert_eq!(artefacts[0].artefact_type, "url");
        assert_eq!(artefacts[0].value, "https://example.test");
        assert_eq!(artefacts[0].offset, 10);
        assert_eq!(artefacts[0].length, 20);
        assert_eq!(artefacts[1].artefact_type, "string");
        assert_eq!(artefacts[1].value, "hello");
        assert_eq!(artefacts[1].offset, 40);
        assert_eq!(artefacts[1].length, 5);
    }

    #[test]
    fn test_reader_csv_rejects_malformed_numeric_field() {
        let temp = TempDir::new().unwrap();
        let metadata_dir = temp.path().join("metadata");
        fs::create_dir(&metadata_dir).unwrap();
        fs::write(
            metadata_dir.join("carved_files.csv"),
            "file_type,global_start,size,carved_path\njpeg,not-a-number,100,1.jpg\n",
        )
        .unwrap();

        let reader = MetadataReader::new(temp.path()).unwrap();
        let result = reader.read_carved_files();

        assert!(result.is_err());
    }

    #[test]
    fn test_reader_csv_missing_ids_do_not_collide_with_explicit_ids() {
        let temp = TempDir::new().unwrap();
        let metadata_dir = temp.path().join("metadata");
        fs::create_dir(&metadata_dir).unwrap();
        fs::write(
            metadata_dir.join("carved_files.csv"),
            "id,file_type,global_start,size,carved_path\n1,jpeg,0,100,1.jpg\n,png,100,200,2.png\n,gif,300,50,3.gif\n",
        )
        .unwrap();

        let reader = MetadataReader::new(temp.path()).unwrap();
        let files = reader.read_carved_files().unwrap();

        assert_eq!(files[0].id, 1);
        assert_eq!(files[1].id, 2);
        assert_eq!(files[2].id, 3);
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
        let files = reader.read_carved_files().unwrap();
        let strings = reader.read_string_artefacts().unwrap();
        let summary = MetadataSummary::from_results(&files, &strings).unwrap();

        assert_eq!(summary.total_files, 3);
        assert_eq!(summary.total_bytes, 3500);
        assert_eq!(summary.by_type.get("jpeg"), Some(&2));
        assert_eq!(summary.by_type.get("png"), Some(&1));
    }

    #[test]
    fn test_reader_csv_summary() {
        let temp = TempDir::new().unwrap();
        let metadata_dir = temp.path().join("metadata");
        fs::create_dir(&metadata_dir).unwrap();

        let data = r#"file_type,global_start,size,carved_path
jpeg,0,1000,1.jpg
jpeg,1000,2000,2.jpg
png,3000,500,3.png"#;
        fs::write(metadata_dir.join("carved_files.csv"), data).unwrap();

        let reader = MetadataReader::new(temp.path()).unwrap();
        let files = reader.read_carved_files().unwrap();
        let strings = reader.read_string_artefacts().unwrap();
        let summary = MetadataSummary::from_results(&files, &strings).unwrap();

        assert_eq!(summary.total_files, 3);
        assert_eq!(summary.total_bytes, 3500);
        assert_eq!(summary.by_type.get("jpeg"), Some(&2));
        assert_eq!(summary.by_type.get("png"), Some(&1));
    }
}
