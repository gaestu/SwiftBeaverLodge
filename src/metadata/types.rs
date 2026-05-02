//! Types for carved file metadata

use std::collections::{BTreeMap, HashMap};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Carved file metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarvedFile {
    /// Lodge-local row ID used for selection/display.
    #[serde(default)]
    pub id: u64,
    /// SwiftBeaver run ID.
    #[serde(default)]
    pub run_id: Option<String>,
    /// Detected file type
    #[serde(default)]
    pub file_type: String,
    /// Output path relative to the run's carved directory.
    #[serde(default, alias = "output_path", alias = "carved_path")]
    pub path: String,
    /// File extension reported by SwiftBeaver JSONL metadata.
    #[serde(default)]
    pub extension: Option<String>,
    /// Start offset in source evidence.
    #[serde(default, alias = "offset")]
    pub global_start: u64,
    /// End offset in source evidence.
    #[serde(default)]
    pub global_end: u64,
    /// Size in bytes
    #[serde(default)]
    pub size: u64,
    /// Handler ID used by Parquet file shards.
    #[serde(default)]
    pub handler_id: Option<String>,
    /// MD5 hash of carved file.
    #[serde(default)]
    pub md5: Option<String>,
    /// SHA-256 hash of carved file
    #[serde(default)]
    pub sha256: Option<String>,
    /// Validation result when SwiftBeaver performed post-carving validation.
    #[serde(default, alias = "is_valid")]
    pub validated: Option<bool>,
    /// Whether SwiftBeaver truncated the carve.
    #[serde(default)]
    pub truncated: bool,
    /// Validation or carve errors reported by SwiftBeaver.
    #[serde(default)]
    pub errors: Vec<String>,
    /// Pattern ID that matched this file.
    #[serde(default)]
    pub pattern_id: Option<String>,
    /// Whether this record is a duplicate of an earlier carved file.
    #[serde(default)]
    pub is_duplicate: bool,
    /// Offset of the original file when this row is a duplicate.
    #[serde(default)]
    pub duplicate_of_offset: Option<u64>,
    /// MIME type if detected
    #[serde(default)]
    pub mime_type: Option<String>,
    /// Image width (for images)
    #[serde(default)]
    pub width: Option<u32>,
    /// Image height (for images)
    #[serde(default)]
    pub height: Option<u32>,
}

/// SwiftBeaver v0.5.1 string artefact (URL, email, phone, generic strings).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringArtefact {
    /// Type: "url", "email", "phone", "string"
    #[serde(default, alias = "artefact_type", alias = "type")]
    pub artefact_kind: String,
    /// The extracted value.
    #[serde(default, alias = "value", alias = "string")]
    pub content: String,
    /// Start offset in source evidence.
    #[serde(default, alias = "offset", alias = "start")]
    pub global_start: u64,
    /// End offset in source evidence when reported by SwiftBeaver.
    #[serde(default, alias = "end")]
    pub global_end: Option<u64>,
    /// Length in bytes when directly reported or derived from global_end.
    #[serde(default)]
    pub length: u64,
    /// Text encoding/source encoding when available.
    #[serde(default)]
    pub encoding: Option<String>,
    /// Source field/path/chunk identifier when available.
    #[serde(default, alias = "source_path", alias = "source_file")]
    pub source: Option<String>,
    /// SwiftBeaver run ID.
    #[serde(default)]
    pub run_id: Option<String>,
}

/// A flexible metadata table row for artefact outputs whose schemas can evolve.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MetadataRecord {
    pub fields: BTreeMap<String, String>,
}

impl MetadataRecord {
    pub fn new(fields: BTreeMap<String, String>) -> Self {
        Self { fields }
    }

    pub fn get(&self, names: &[&str]) -> Option<&str> {
        names.iter().find_map(|name| {
            self.fields
                .get(*name)
                .map(String::as_str)
                .filter(|value| !value.is_empty())
        })
    }

    pub fn get_u64(&self, names: &[&str]) -> Option<u64> {
        self.get(names)
            .and_then(|value| value.trim().parse::<u64>().ok())
    }
}

/// Run-level metrics emitted by SwiftBeaver v0.5.1.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunSummary {
    pub bytes_scanned: Option<u64>,
    pub chunks_processed: Option<u64>,
    pub hits: Option<u64>,
    pub files_carved: Option<u64>,
    pub rejected: Option<u64>,
    pub prevalidation_rejected: Option<u64>,
    pub overlap_skipped: Option<u64>,
    pub string_spans: Option<u64>,
    pub artefacts_extracted: Option<u64>,
    pub duplicates_found: Option<u64>,
    pub duplicates_skipped: Option<u64>,
    pub fields: BTreeMap<String, String>,
}

impl RunSummary {
    pub fn from_record(record: MetadataRecord) -> Self {
        Self {
            bytes_scanned: record.get_u64(&["bytes_scanned", "total_bytes_scanned"]),
            chunks_processed: record.get_u64(&["chunks_processed", "chunks"]),
            hits: record.get_u64(&["hits", "total_hits"]),
            files_carved: record.get_u64(&["files_carved", "carved_files", "files"]),
            rejected: record.get_u64(&["rejected", "files_rejected"]),
            prevalidation_rejected: record
                .get_u64(&["prevalidation_rejected", "prevalidated_rejected"]),
            overlap_skipped: record.get_u64(&["overlap_skipped"]),
            string_spans: record.get_u64(&["string_spans", "strings_found"]),
            artefacts_extracted: record.get_u64(&["artefacts_extracted", "artifacts_extracted"]),
            duplicates_found: record.get_u64(&["duplicates_found"]),
            duplicates_skipped: record.get_u64(&["duplicates_skipped"]),
            fields: record.fields,
        }
    }
}

/// File type category for filtering
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileType {
    Image,
    Document,
    Archive,
    Database,
    Media,
    Other,
}

impl FileType {
    #[allow(dead_code)]
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "jpeg" | "jpg" | "png" | "gif" | "bmp" | "webp" | "ico" | "tiff" => FileType::Image,
            "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "rtf" => {
                FileType::Document
            }
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" => FileType::Archive,
            "sqlite" | "db" | "mdb" => FileType::Database,
            "mp3" | "mp4" | "avi" | "mkv" | "wav" | "flac" | "ogg" => FileType::Media,
            _ => FileType::Other,
        }
    }

    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            FileType::Image => "Images",
            FileType::Document => "Documents",
            FileType::Archive => "Archives",
            FileType::Database => "Databases",
            FileType::Media => "Media",
            FileType::Other => "Other",
        }
    }
}

/// Summary of scan metadata
#[derive(Debug, Clone, Default)]
pub struct MetadataSummary {
    /// Total carved files
    pub total_files: usize,
    /// Files by type
    pub by_type: HashMap<String, usize>,
    /// Total bytes carved
    pub total_bytes: u64,
    /// String artefacts count
    pub string_artefacts: usize,
}

impl MetadataSummary {
    pub fn from_results(files: &[CarvedFile], strings: &[StringArtefact]) -> Result<Self> {
        let mut by_type: HashMap<String, usize> = HashMap::new();
        let mut total_bytes = 0u64;

        for file in files {
            *by_type.entry(file.file_type.clone()).or_insert(0) += 1;
            total_bytes = total_bytes
                .checked_add(file.size)
                .context("Metadata summary total_bytes overflowed")?;
        }

        Ok(Self {
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

    #[test]
    fn test_file_type_from_extension() {
        assert_eq!(FileType::from_extension("jpeg"), FileType::Image);
        assert_eq!(FileType::from_extension("JPEG"), FileType::Image);
        assert_eq!(FileType::from_extension("pdf"), FileType::Document);
        assert_eq!(FileType::from_extension("zip"), FileType::Archive);
        assert_eq!(FileType::from_extension("sqlite"), FileType::Database);
        assert_eq!(FileType::from_extension("mp3"), FileType::Media);
        assert_eq!(FileType::from_extension("xyz"), FileType::Other);
    }

    #[test]
    fn test_file_type_labels() {
        assert_eq!(FileType::Image.label(), "Images");
        assert_eq!(FileType::Document.label(), "Documents");
    }

    #[test]
    fn test_carved_file_deserialization() {
        let json = r#"{
            "id": 1,
            "file_type": "jpeg",
            "offset": 1024,
            "size": 4096,
            "output_path": "output/0001.jpg",
            "is_valid": true
        }"#;

        let file: CarvedFile = serde_json::from_str(json).unwrap();
        assert_eq!(file.id, 1);
        assert_eq!(file.file_type, "jpeg");
        assert_eq!(file.global_start, 1024);
        assert_eq!(file.size, 4096);
        assert_eq!(file.path, "output/0001.jpg");
        assert_eq!(file.validated, Some(true));
    }

    #[test]
    fn test_string_artefact_deserialization() {
        let json = r#"{
            "artefact_type": "email",
            "value": "test@example.com",
            "offset": 2048,
            "length": 16
        }"#;

        let artefact: StringArtefact = serde_json::from_str(json).unwrap();
        assert_eq!(artefact.artefact_kind, "email");
        assert_eq!(artefact.content, "test@example.com");
        assert_eq!(artefact.global_start, 2048);
        assert_eq!(artefact.length, 16);
    }

    #[test]
    fn test_string_artefact_v051_deserialization() {
        let json = r#"{
            "artefact_kind": "url",
            "content": "https://example.test",
            "global_start": 10,
            "global_end": 30,
            "encoding": "utf8",
            "source": "chunk-1"
        }"#;

        let artefact: StringArtefact = serde_json::from_str(json).unwrap();
        assert_eq!(artefact.artefact_kind, "url");
        assert_eq!(artefact.content, "https://example.test");
        assert_eq!(artefact.global_start, 10);
        assert_eq!(artefact.global_end, Some(30));
        assert_eq!(artefact.encoding.as_deref(), Some("utf8"));
        assert_eq!(artefact.source.as_deref(), Some("chunk-1"));
    }

    #[test]
    fn test_run_summary_from_record() {
        let mut fields = BTreeMap::new();
        fields.insert("bytes_scanned".to_string(), "4096".to_string());
        fields.insert("chunks_processed".to_string(), "2".to_string());
        fields.insert("hits".to_string(), "3".to_string());
        fields.insert("files_carved".to_string(), "1".to_string());
        fields.insert("duplicates_skipped".to_string(), "4".to_string());

        let summary = RunSummary::from_record(MetadataRecord::new(fields));

        assert_eq!(summary.bytes_scanned, Some(4096));
        assert_eq!(summary.chunks_processed, Some(2));
        assert_eq!(summary.hits, Some(3));
        assert_eq!(summary.files_carved, Some(1));
        assert_eq!(summary.duplicates_skipped, Some(4));
    }
}
