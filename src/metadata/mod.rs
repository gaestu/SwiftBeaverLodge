//! Metadata reading from Parquet and JSONL files

mod reader;
mod types;

pub use reader::MetadataReader;
pub use types::{CarvedFile, MetadataSummary, StringArtefact};

use std::path::Path;

/// Detect the metadata backend used in a run directory
pub fn detect_metadata_backend(run_path: &Path) -> Option<&'static str> {
    if run_path.join("parquet").exists() {
        Some("parquet")
    } else if run_path
        .join("metadata")
        .join("carved_files.jsonl")
        .exists()
    {
        Some("jsonl")
    } else if run_path.join("metadata").join("carved_files.csv").exists() {
        Some("csv")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_parquet_backend() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join("parquet")).unwrap();

        assert_eq!(detect_metadata_backend(temp.path()), Some("parquet"));
    }

    #[test]
    fn test_detect_jsonl_backend() {
        let temp = TempDir::new().unwrap();
        let metadata_dir = temp.path().join("metadata");
        fs::create_dir(&metadata_dir).unwrap();
        fs::write(metadata_dir.join("carved_files.jsonl"), "").unwrap();

        assert_eq!(detect_metadata_backend(temp.path()), Some("jsonl"));
    }

    #[test]
    fn test_detect_no_backend() {
        let temp = TempDir::new().unwrap();
        assert_eq!(detect_metadata_backend(temp.path()), None);
    }

    #[test]
    fn test_detect_csv_backend() {
        let temp = TempDir::new().unwrap();
        let metadata_dir = temp.path().join("metadata");
        fs::create_dir(&metadata_dir).unwrap();
        fs::write(metadata_dir.join("carved_files.csv"), "").unwrap();

        assert_eq!(detect_metadata_backend(temp.path()), Some("csv"));
    }
}
