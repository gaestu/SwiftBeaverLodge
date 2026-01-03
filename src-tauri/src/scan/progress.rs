//! Progress parsing for JSON log output from fastcarve
//!
//! Parses the --log-format json output to extract progress and log events

use serde_json::Value;
use crate::types::GuiScanProgress;

/// Parse a JSON log line from fastcarve
/// 
/// Returns (event_type, payload) where event_type is "progress", "info", "warn", "error", etc.
pub fn parse_json_log(line: &str) -> Option<(String, Value)> {
    let json: Value = serde_json::from_str(line).ok()?;
    
    // fastcarve JSON logs have this structure:
    // {"timestamp":"...","level":"INFO","target":"...","message":"progress bytes_scanned=..."}
    // or for structured progress:
    // {"timestamp":"...","level":"INFO","fields":{"bytes_scanned":...}}
    
    let level = json.get("level")?.as_str()?.to_lowercase();
    let message = json.get("message").and_then(|v| v.as_str()).unwrap_or("");
    
    // Check if this is a progress message
    if message.starts_with("progress ") {
        if let Some(progress) = parse_progress_message(message) {
            return Some(("progress".to_string(), serde_json::to_value(progress).ok()?));
        }
    }
    
    // Check for structured fields (newer format)
    if let Some(fields) = json.get("fields") {
        if fields.get("bytes_scanned").is_some() {
            if let Some(progress) = parse_progress_fields(fields) {
                return Some(("progress".to_string(), serde_json::to_value(progress).ok()?));
            }
        }
    }
    
    // Regular log message
    Some((level, serde_json::json!({
        "message": message,
        "target": json.get("target").and_then(|v| v.as_str()).unwrap_or(""),
        "timestamp": json.get("timestamp").and_then(|v| v.as_str()).unwrap_or(""),
    })))
}

/// Parse progress from a message string like:
/// "progress bytes_scanned=1234 total_bytes=5678 pct=21.7 hits=10 files=5 rate_mib=123.45 eta_secs=Some(60)"
fn parse_progress_message(message: &str) -> Option<GuiScanProgress> {
    let parts: std::collections::HashMap<&str, &str> = message
        .strip_prefix("progress ")?
        .split_whitespace()
        .filter_map(|part| {
            let mut split = part.splitn(2, '=');
            Some((split.next()?, split.next()?))
        })
        .collect();
    
    Some(GuiScanProgress {
        bytes_scanned: parts.get("bytes_scanned")?.parse().ok()?,
        total_bytes: parts.get("total_bytes")?.parse().ok()?,
        chunks_processed: parts.get("chunks_processed").and_then(|v| v.parse().ok()).unwrap_or(0),
        hits_found: parts.get("hits")?.parse().ok()?,
        files_carved: parts.get("files")?.parse().ok()?,
        string_spans: parts.get("string_spans").and_then(|v| v.parse().ok()).unwrap_or(0),
        artefacts_extracted: parts.get("artefacts").and_then(|v| v.parse().ok()).unwrap_or(0),
        carve_errors: parts.get("carve_errs").and_then(|v| v.parse().ok()).unwrap_or(0),
        metadata_errors: parts.get("meta_errs").and_then(|v| v.parse().ok()).unwrap_or(0),
        sqlite_errors: parts.get("sqlite_errs").and_then(|v| v.parse().ok()).unwrap_or(0),
        elapsed_seconds: parts.get("elapsed_secs").and_then(|v| v.parse().ok()).unwrap_or(0.0),
        throughput_mib: parts.get("rate_mib")?.parse().ok()?,
        eta_seconds: parse_option_u64(parts.get("eta_secs").copied()),
    })
}

/// Parse progress from structured JSON fields
fn parse_progress_fields(fields: &Value) -> Option<GuiScanProgress> {
    Some(GuiScanProgress {
        bytes_scanned: fields.get("bytes_scanned")?.as_u64()?,
        total_bytes: fields.get("total_bytes")?.as_u64()?,
        chunks_processed: fields.get("chunks_processed").and_then(|v| v.as_u64()).unwrap_or(0),
        hits_found: fields.get("hits_found").or(fields.get("hits")).and_then(|v| v.as_u64()).unwrap_or(0),
        files_carved: fields.get("files_carved").or(fields.get("files")).and_then(|v| v.as_u64()).unwrap_or(0),
        string_spans: fields.get("string_spans").and_then(|v| v.as_u64()).unwrap_or(0),
        artefacts_extracted: fields.get("artefacts_extracted").and_then(|v| v.as_u64()).unwrap_or(0),
        carve_errors: fields.get("carve_errors").and_then(|v| v.as_u64()).unwrap_or(0),
        metadata_errors: fields.get("metadata_errors").and_then(|v| v.as_u64()).unwrap_or(0),
        sqlite_errors: fields.get("sqlite_errors").and_then(|v| v.as_u64()).unwrap_or(0),
        elapsed_seconds: fields.get("elapsed_seconds").and_then(|v| v.as_f64()).unwrap_or(0.0),
        throughput_mib: fields.get("throughput_mib").and_then(|v| v.as_f64()).unwrap_or(0.0),
        eta_seconds: fields.get("eta_seconds").and_then(|v| v.as_u64()),
    })
}

/// Parse "Some(123)" or "None" to Option<u64>
fn parse_option_u64(s: Option<&str>) -> Option<u64> {
    let s = s?;
    if s == "None" {
        return None;
    }
    // Parse "Some(123)" format
    s.strip_prefix("Some(")
        .and_then(|s| s.strip_suffix(')'))
        .and_then(|s| s.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_progress_message() {
        let msg = "progress bytes_scanned=1048576 total_bytes=10485760 pct=10.0 hits=5 files=3 rate_mib=150.25 eta_secs=Some(60) carve_errs=0 meta_errs=0 sqlite_errs=0";
        let progress = parse_progress_message(msg).unwrap();
        
        assert_eq!(progress.bytes_scanned, 1048576);
        assert_eq!(progress.total_bytes, 10485760);
        assert_eq!(progress.hits_found, 5);
        assert_eq!(progress.files_carved, 3);
        assert_eq!(progress.throughput_mib, 150.25);
        assert_eq!(progress.eta_seconds, Some(60));
    }

    #[test]
    fn test_parse_option_u64() {
        assert_eq!(parse_option_u64(Some("Some(60)")), Some(60));
        assert_eq!(parse_option_u64(Some("None")), None);
        assert_eq!(parse_option_u64(None), None);
    }
}
