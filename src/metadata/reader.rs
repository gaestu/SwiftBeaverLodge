//! Metadata reader for Parquet and JSONL files

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use arrow::array::{Array, StringArray, UInt64Array, BooleanArray, UInt32Array};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

use super::types::{CarvedFile, StringArtefact, MetadataSummary};

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
                && path.file_name()
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
        let reader = ParquetRecordBatchReaderBuilder::try_new(file)?
            .build()?;

        let mut files = Vec::new();

        for batch_result in reader {
            let batch = batch_result?;
            
            // Get column arrays
            let id_col = batch.column_by_name("id")
                .and_then(|c| c.as_any().downcast_ref::<UInt64Array>());
            let type_col = batch.column_by_name("file_type")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let offset_col = batch.column_by_name("offset")
                .and_then(|c| c.as_any().downcast_ref::<UInt64Array>());
            let size_col = batch.column_by_name("size")
                .and_then(|c| c.as_any().downcast_ref::<UInt64Array>());
            let output_col = batch.column_by_name("output_path")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let sha256_col = batch.column_by_name("sha256")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let valid_col = batch.column_by_name("is_valid")
                .and_then(|c| c.as_any().downcast_ref::<BooleanArray>());
            let width_col = batch.column_by_name("width")
                .and_then(|c| c.as_any().downcast_ref::<UInt32Array>());
            let height_col = batch.column_by_name("height")
                .and_then(|c| c.as_any().downcast_ref::<UInt32Array>());

            for i in 0..batch.num_rows() {
                let file = CarvedFile {
                    id: id_col.map(|c| c.value(i)).unwrap_or(0),
                    file_type: type_col.and_then(|c| c.value(i).to_string().into()).unwrap_or_default(),
                    offset: offset_col.map(|c| c.value(i)).unwrap_or(0),
                    size: size_col.map(|c| c.value(i)).unwrap_or(0),
                    output_path: output_col.and_then(|c| Some(c.value(i).to_string())).unwrap_or_default(),
                    sha256: sha256_col.and_then(|c| {
                        if c.is_null(i) { None } else { Some(c.value(i).to_string()) }
                    }),
                    is_valid: valid_col.map(|c| c.value(i)).unwrap_or(true),
                    mime_type: None,
                    width: width_col.and_then(|c| {
                        if c.is_null(i) { None } else { Some(c.value(i)) }
                    }),
                    height: height_col.and_then(|c| {
                        if c.is_null(i) { None } else { Some(c.value(i)) }
                    }),
                };
                files.push(file);
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

    /// Read string artefacts
    pub fn read_string_artefacts(&self) -> Result<Vec<StringArtefact>> {
        match self.backend.as_str() {
            "parquet" => self.read_string_artefacts_parquet(),
            "jsonl" => self.read_string_artefacts_jsonl(),
            _ => bail!("Unknown metadata backend: {}", self.backend),
        }
    }

    /// Read string artefacts from Parquet
    fn read_string_artefacts_parquet(&self) -> Result<Vec<StringArtefact>> {
        let parquet_path = self.run_path.join("parquet").join("strings.parquet");
        
        if !parquet_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&parquet_path)?;
        let reader = ParquetRecordBatchReaderBuilder::try_new(file)?
            .build()?;

        let mut artefacts = Vec::new();

        for batch_result in reader {
            let batch = batch_result?;
            
            let type_col = batch.column_by_name("type")
                .or_else(|| batch.column_by_name("artefact_type"))
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let value_col = batch.column_by_name("value")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let offset_col = batch.column_by_name("offset")
                .and_then(|c| c.as_any().downcast_ref::<UInt64Array>());
            let length_col = batch.column_by_name("length")
                .and_then(|c| c.as_any().downcast_ref::<UInt64Array>());

            for i in 0..batch.num_rows() {
                let artefact = StringArtefact {
                    artefact_type: type_col.and_then(|c| Some(c.value(i).to_string())).unwrap_or_default(),
                    value: value_col.and_then(|c| Some(c.value(i).to_string())).unwrap_or_default(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

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
