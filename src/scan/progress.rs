//! Progress parsing from fastcarve JSON logs

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Scan progress snapshot
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanProgress {
    pub bytes_scanned: u64,
    pub total_bytes: u64,
    pub pct: f64,
    pub hits: u64,
    pub files: u64,
    pub rate_mib: f64,
    pub eta_secs: Option<u64>,
}

impl ScanProgress {
    pub fn percentage(&self) -> f32 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.bytes_scanned as f64 / self.total_bytes as f64 * 100.0) as f32
        }
    }

    pub fn eta_formatted(&self) -> String {
        match self.eta_secs {
            Some(secs) => format_duration(secs),
            None => "calculating...".to_string(),
        }
    }
}

/// Format seconds into human readable duration
pub fn format_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

/// Format bytes into human readable size
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;
    const TB: u64 = 1024 * 1024 * 1024 * 1024;

    if bytes < KB {
        format!("{} B", bytes)
    } else if bytes < MB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes < TB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    }
}

/// Parse a JSON log line from fastcarve
///
/// Format: {"timestamp":"...","level":"INFO","message":"...","fields":{...}}
///
/// Returns (event_type, payload) where:
/// - For progress: ("progress", progress_data)
/// - For logs: (level, {"message": "..."})
pub fn parse_json_log(line: &str) -> Option<(String, Value)> {
    let json: Value = serde_json::from_str(line).ok()?;
    
    let level = json.get("level")?.as_str()?;
    let message = json.get("message")?.as_str()?;
    
    // Check if this is a progress message
    if message.starts_with("progress ") {
        if let Some(progress) = parse_progress_message(message) {
            return Some(("progress".to_string(), serde_json::to_value(progress).ok()?));
        }
    }
    
    // Return as log entry
    let mut payload = serde_json::Map::new();
    payload.insert("message".to_string(), Value::String(message.to_string()));
    
    Some((level.to_lowercase(), Value::Object(payload)))
}

/// Parse a progress message string
///
/// Format: "progress bytes_scanned=N total_bytes=N pct=N.N hits=N files=N rate_mib=N.NN eta_secs=Some(N)"
pub fn parse_progress_message(message: &str) -> Option<ScanProgress> {
    let message = message.strip_prefix("progress ")?;
    
    let mut progress = ScanProgress::default();
    
    for part in message.split_whitespace() {
        let mut kv = part.splitn(2, '=');
        let key = kv.next()?;
        let value = kv.next()?;
        
        match key {
            "bytes_scanned" => progress.bytes_scanned = value.parse().ok()?,
            "total_bytes" => progress.total_bytes = value.parse().ok()?,
            "pct" => progress.pct = value.parse().ok()?,
            "hits" => progress.hits = value.parse().ok()?,
            "files" => progress.files = value.parse().ok()?,
            "rate_mib" => progress.rate_mib = value.parse().ok()?,
            "eta_secs" => {
                progress.eta_secs = if value == "None" {
                    None
                } else {
                    // Handle "Some(N)" format
                    let num_str = value
                        .strip_prefix("Some(")
                        .and_then(|s| s.strip_suffix(')'))
                        .unwrap_or(value);
                    num_str.parse().ok()
                };
            }
            _ => {}
        }
    }
    
    Some(progress)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
        assert_eq!(format_bytes(1099511627776), "1.00 TB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(30), "30s");
        assert_eq!(format_duration(90), "1m 30s");
        assert_eq!(format_duration(3661), "1h 1m");
    }

    #[test]
    fn test_parse_progress_message() {
        let msg = "progress bytes_scanned=1048576 total_bytes=10485760 pct=10.0 hits=5 files=3 rate_mib=50.25 eta_secs=Some(180)";
        
        let progress = parse_progress_message(msg).unwrap();
        
        assert_eq!(progress.bytes_scanned, 1048576);
        assert_eq!(progress.total_bytes, 10485760);
        assert_eq!(progress.pct, 10.0);
        assert_eq!(progress.hits, 5);
        assert_eq!(progress.files, 3);
        assert_eq!(progress.rate_mib, 50.25);
        assert_eq!(progress.eta_secs, Some(180));
    }

    #[test]
    fn test_parse_progress_message_no_eta() {
        let msg = "progress bytes_scanned=1000 total_bytes=10000 pct=10.0 hits=0 files=0 rate_mib=100.0 eta_secs=None";
        
        let progress = parse_progress_message(msg).unwrap();
        assert_eq!(progress.eta_secs, None);
    }

    #[test]
    fn test_parse_json_log_progress() {
        let line = r#"{"timestamp":"2024-01-01T00:00:00Z","level":"INFO","message":"progress bytes_scanned=1000 total_bytes=10000 pct=10.0 hits=0 files=0 rate_mib=100.0 eta_secs=Some(90)"}"#;
        
        let (event_type, payload) = parse_json_log(line).unwrap();
        
        assert_eq!(event_type, "progress");
        assert_eq!(payload.get("bytes_scanned").unwrap().as_u64(), Some(1000));
    }

    #[test]
    fn test_parse_json_log_regular() {
        let line = r#"{"timestamp":"2024-01-01T00:00:00Z","level":"INFO","message":"Starting scan"}"#;
        
        let (event_type, payload) = parse_json_log(line).unwrap();
        
        assert_eq!(event_type, "info");
        assert_eq!(payload.get("message").unwrap().as_str(), Some("Starting scan"));
    }

    #[test]
    fn test_progress_percentage() {
        let progress = ScanProgress {
            bytes_scanned: 50,
            total_bytes: 100,
            ..Default::default()
        };
        assert_eq!(progress.percentage(), 50.0);
    }

    #[test]
    fn test_progress_percentage_zero_total() {
        let progress = ScanProgress {
            bytes_scanned: 0,
            total_bytes: 0,
            ..Default::default()
        };
        assert_eq!(progress.percentage(), 0.0);
    }
}
