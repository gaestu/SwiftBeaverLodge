//! Types for carved file metadata

use serde::{Deserialize, Serialize};

/// Carved file metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarvedFile {
    /// Unique ID within the scan
    pub id: u64,
    /// Detected file type
    pub file_type: String,
    /// Offset in source evidence
    pub offset: u64,
    /// Size in bytes
    pub size: u64,
    /// Output filename
    pub output_path: String,
    /// SHA-256 hash of carved file
    #[serde(default)]
    pub sha256: Option<String>,
    /// Whether the file appears valid
    #[serde(default)]
    pub is_valid: bool,
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

/// String artefact (URL, email, phone)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringArtefact {
    /// Type: "url", "email", "phone", "string"
    pub artefact_type: String,
    /// The extracted value
    pub value: String,
    /// Offset in source evidence
    pub offset: u64,
    /// Length in bytes
    pub length: u64,
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
            "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "rtf" => FileType::Document,
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
    pub by_type: std::collections::HashMap<String, usize>,
    /// Total bytes carved
    pub total_bytes: u64,
    /// String artefacts count
    pub string_artefacts: usize,
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
        assert_eq!(file.offset, 1024);
        assert_eq!(file.size, 4096);
        assert!(file.is_valid);
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
        assert_eq!(artefact.artefact_type, "email");
        assert_eq!(artefact.value, "test@example.com");
    }
}
