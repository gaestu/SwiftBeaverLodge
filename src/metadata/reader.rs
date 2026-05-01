//! Metadata reader for Parquet, JSONL, and CSV files

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use arrow::array::{Array, BooleanArray, Int64Array, StringArray, UInt32Array, UInt64Array};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

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

        for line in reader.lines() {
            let line = line?;
            if line.is_empty() {
                continue;
            }

            let file: CarvedFile = serde_json::from_str(&line)
                .with_context(|| format!("Failed to parse line: {}", line))?;
            files.push(file);
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

            let file = CarvedFile {
                id,
                file_type: csv_string_field(&headers, &record, &["file_type", "type"]),
                offset: csv_u64_field(&headers, &record, &["offset", "global_start", "start"])?
                    .unwrap_or(0),
                size: csv_u64_field(&headers, &record, &["size", "length"])?.unwrap_or(0),
                output_path: csv_string_field(
                    &headers,
                    &record,
                    &["output_path", "carved_path", "path", "filename"],
                ),
                sha256: csv_optional_string_field(&headers, &record, &["sha256", "sha256_hex"]),
                is_valid: csv_bool_field(&headers, &record, &["is_valid", "validated"])?
                    .unwrap_or(true),
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
                .column_by_name("phone")
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
        let jsonl_path = self.run_path.join("metadata").join("strings.jsonl");

        if !jsonl_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&jsonl_path)?;
        let reader = BufReader::new(file);

        let mut artefacts = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.is_empty() {
                continue;
            }

            let artefact: StringArtefact = serde_json::from_str(&line)?;
            artefacts.push(artefact);
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
            let length = csv_u64_field(&headers, &record, &["length"])?
                .or_else(|| end.map(|end| end.saturating_sub(offset)))
                .unwrap_or(0);
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
                .with_context(|| format!("Invalid unsigned integer CSV value: {}", value))
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
                .with_context(|| format!("Invalid unsigned integer CSV value: {}", value))
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
            _ => bail!("Invalid boolean CSV value: {}", value),
        })
        .transpose()
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
        assert_eq!(files[0].offset, 1024);
        assert_eq!(files[0].size, 4096);
        assert_eq!(files[0].output_path, "carved/with, comma.jpg");
        assert_eq!(files[0].sha256.as_deref(), Some("abc123"));
        assert!(files[0].is_valid);
        assert_eq!(files[0].mime_type.as_deref(), Some("image/jpeg"));
        assert_eq!(files[0].width, Some(800));
        assert_eq!(files[0].height, Some(600));
        assert_eq!(files[1].id, 1);
        assert_eq!(files[1].sha256, None);
        assert!(!files[1].is_valid);
        assert_eq!(files[1].mime_type, None);
        assert_eq!(files[1].width, None);
        assert_eq!(files[1].height, None);
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
        let summary = reader.get_summary().unwrap();

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
        let summary = reader.get_summary().unwrap();

        assert_eq!(summary.total_files, 3);
        assert_eq!(summary.total_bytes, 3500);
        assert_eq!(summary.by_type.get("jpeg"), Some(&2));
        assert_eq!(summary.by_type.get("png"), Some(&1));
    }
}
