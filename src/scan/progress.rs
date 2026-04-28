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
/// Format: {"timestamp":"...","level":"INFO","fields":{"message":"..."},"target":"..."}
///
/// Returns (event_type, payload) where:
/// - For progress: ("progress", progress_data)
/// - For starting: ("starting", {"run_id": "...", "output": "..."})
/// - For logs: (level, {"message": "..."})
pub fn parse_json_log(line: &str) -> Option<(String, Value)> {
    let json: Value = serde_json::from_str(line).ok()?;

    let level = json.get("level")?.as_str()?;

    // fastcarve uses "fields.message" format
    let message = json
        .get("fields")
        .and_then(|f| f.get("message"))
        .and_then(|m| m.as_str())
        // Fallback to top-level message for compatibility
        .or_else(|| json.get("message").and_then(|m| m.as_str()))?;

    // Check if this is a progress message
    if message.starts_with("progress ") {
        if let Some(progress) = parse_progress_message(message) {
            return Some(("progress".to_string(), serde_json::to_value(progress).ok()?));
        }
    }

    // Check if this is a starting message (contains run output path)
    if message.starts_with("starting ") {
        if let Some(start_info) = parse_starting_message(message) {
            return Some(("starting".to_string(), start_info));
        }
    }

    // Return as log entry
    let mut payload = serde_json::Map::new();
    payload.insert("message".to_string(), Value::String(message.to_string()));

    Some((level.to_lowercase(), Value::Object(payload)))
}

/// Parse a starting message to extract run_id and output path
///
/// Format: "starting run_id=XXX input=YYY output=ZZZ workers=N chunk_size=N"
pub fn parse_starting_message(message: &str) -> Option<Value> {
    let message = message.strip_prefix("starting ")?;

    let mut run_id = String::new();
    let mut output = String::new();

    for part in message.split_whitespace() {
        if let Some(val) = part.strip_prefix("run_id=") {
            run_id = val.to_string();
        } else if let Some(val) = part.strip_prefix("output=") {
            output = val.to_string();
        }
    }

    if !output.is_empty() {
        let mut payload = serde_json::Map::new();
        payload.insert("run_id".to_string(), Value::String(run_id));
        payload.insert("output".to_string(), Value::String(output));
        return Some(Value::Object(payload));
    }

    None
}

/// Parse a progress message string.
///
/// Supports both the SwiftBeaver v0.5.1 format and the older legacy form:
///
/// * v0.5.1: `progress 100.0% scanned=A/B hits=H files=F rate=R.RMiB/s eta=N/A errs=[...]`
/// * legacy: `progress bytes_scanned=N total_bytes=N pct=N.N hits=N files=N rate_mib=N.NN eta_secs=Some(N)`
pub fn parse_progress_message(message: &str) -> Option<ScanProgress> {
    let body = message.strip_prefix("progress ")?;

    // Detect format by checking for the v0.5.1 `scanned=A/B` token. Fall back
    // to the legacy parser to keep older binaries / fixtures working.
    if body.split_whitespace().any(|p| p.starts_with("scanned=")) {
        parse_progress_v051(body)
    } else {
        parse_progress_legacy(body)
    }
}

fn parse_progress_v051(body: &str) -> Option<ScanProgress> {
    let mut progress = ScanProgress::default();
    let mut saw_scanned = false;

    for part in body.split_whitespace() {
        // Standalone percentage token, e.g. `100.0%`.
        if let Some(pct_str) = part.strip_suffix('%') {
            if !pct_str.contains('=') {
                if let Ok(pct) = pct_str.parse::<f64>() {
                    progress.pct = pct;
                }
                continue;
            }
        }

        let Some((key, value)) = part.split_once('=') else {
            continue;
        };

        match key {
            "scanned" => {
                let (a, b) = value.split_once('/')?;
                progress.bytes_scanned = a.parse().ok()?;
                progress.total_bytes = b.parse().ok()?;
                saw_scanned = true;
            }
            "hits" => {
                progress.hits = value.parse().ok()?;
            }
            "files" => {
                progress.files = value.parse().ok()?;
            }
            "rate" => {
                progress.rate_mib = parse_rate_mib(value).unwrap_or(0.0);
            }
            "eta" => {
                progress.eta_secs = parse_eta(value);
            }
            // Ignore unknown keys (e.g. `errs=[carve:0 meta:0 sql:0]`) so
            // SwiftBeaver can extend the schema without breaking parsing.
            _ => {}
        }
    }

    if saw_scanned {
        Some(progress)
    } else {
        None
    }
}

fn parse_progress_legacy(body: &str) -> Option<ScanProgress> {
    let mut progress = ScanProgress::default();
    let mut saw_known_field = false;

    for part in body.split_whitespace() {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };

        match key {
            "bytes_scanned" => {
                progress.bytes_scanned = value.parse().ok()?;
                saw_known_field = true;
            }
            "total_bytes" => {
                progress.total_bytes = value.parse().ok()?;
                saw_known_field = true;
            }
            "pct" => {
                progress.pct = value.parse().ok()?;
                saw_known_field = true;
            }
            "hits" => {
                progress.hits = value.parse().ok()?;
                saw_known_field = true;
            }
            "files" => {
                progress.files = value.parse().ok()?;
                saw_known_field = true;
            }
            "rate_mib" => {
                progress.rate_mib = value.parse().ok()?;
                saw_known_field = true;
            }
            "eta_secs" => {
                progress.eta_secs = if value == "None" {
                    None
                } else {
                    let num_str = value
                        .strip_prefix("Some(")
                        .and_then(|s| s.strip_suffix(')'))
                        .unwrap_or(value);
                    num_str.parse().ok()
                };
                saw_known_field = true;
            }
            _ => {}
        }
    }

    if saw_known_field {
        Some(progress)
    } else {
        None
    }
}

/// Parse a SwiftBeaver `rate=` value such as `109.81MiB/s`.
fn parse_rate_mib(value: &str) -> Option<f64> {
    let trimmed = value
        .strip_suffix("MiB/s")
        .or_else(|| value.strip_suffix("MB/s"))
        .unwrap_or(value);
    trimmed.parse().ok()
}

/// Parse a SwiftBeaver `eta=` value into seconds.
///
/// Accepts `N/A` (returns `None`), bare integers (seconds), single-unit values
/// like `5s`, `2m`, `1h`, and combinations like `1h2m`, `1m30s`, `1h2m3s`.
/// Unknown forms return `None` rather than panicking.
pub fn parse_eta(value: &str) -> Option<u64> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("n/a") || value == "None" {
        return None;
    }

    // Bare integer = seconds.
    if let Ok(n) = value.parse::<u64>() {
        return Some(n);
    }

    let mut total: u64 = 0;
    let mut current: u64 = 0;
    let mut saw_unit = false;
    let mut saw_digit = false;

    for ch in value.chars() {
        if ch.is_ascii_digit() {
            current = current
                .checked_mul(10)?
                .checked_add((ch as u8 - b'0') as u64)?;
            saw_digit = true;
        } else {
            let mult = match ch {
                's' | 'S' => 1u64,
                'm' | 'M' => 60,
                'h' | 'H' => 3600,
                _ => return None,
            };
            if !saw_digit {
                return None;
            }
            total = total.checked_add(current.checked_mul(mult)?)?;
            current = 0;
            saw_digit = false;
            saw_unit = true;
        }
    }

    if saw_digit {
        // Trailing digits without a unit -> treat as seconds.
        total = total.checked_add(current)?;
        saw_unit = true;
    }

    if saw_unit {
        Some(total)
    } else {
        None
    }
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
        // Test with fields.message format (fastcarve's actual format)
        let line = r#"{"timestamp":"2024-01-01T00:00:00Z","level":"INFO","fields":{"message":"progress bytes_scanned=1000 total_bytes=10000 pct=10.0 hits=0 files=0 rate_mib=100.0 eta_secs=Some(90)"}}"#;

        let (event_type, payload) = parse_json_log(line).unwrap();

        assert_eq!(event_type, "progress");
        assert_eq!(payload.get("bytes_scanned").unwrap().as_u64(), Some(1000));
    }

    #[test]
    fn test_parse_json_log_progress_legacy_format() {
        // Test fallback to root-level message for compatibility
        let line = r#"{"timestamp":"2024-01-01T00:00:00Z","level":"INFO","message":"progress bytes_scanned=1000 total_bytes=10000 pct=10.0 hits=0 files=0 rate_mib=100.0 eta_secs=Some(90)"}"#;

        let (event_type, payload) = parse_json_log(line).unwrap();

        assert_eq!(event_type, "progress");
        assert_eq!(payload.get("bytes_scanned").unwrap().as_u64(), Some(1000));
    }

    #[test]
    fn test_parse_json_log_regular() {
        let line = r#"{"timestamp":"2024-01-01T00:00:00Z","level":"INFO","fields":{"message":"Starting scan"}}"#;

        let (event_type, payload) = parse_json_log(line).unwrap();

        assert_eq!(event_type, "info");
        assert_eq!(
            payload.get("message").unwrap().as_str(),
            Some("Starting scan")
        );
    }

    #[test]
    fn test_parse_json_log_starting() {
        let line = r#"{"timestamp":"2024-01-01T00:00:00Z","level":"INFO","fields":{"message":"starting run_id=20260103T125116Z_09ab31c2 input=/tmp/test.dd output=/tmp/out/20260103T125116Z_09ab31c2 workers=4 chunk_size=1048576"}}"#;

        let (event_type, payload) = parse_json_log(line).unwrap();

        assert_eq!(event_type, "starting");
        assert_eq!(
            payload.get("run_id").unwrap().as_str(),
            Some("20260103T125116Z_09ab31c2")
        );
        assert_eq!(
            payload.get("output").unwrap().as_str(),
            Some("/tmp/out/20260103T125116Z_09ab31c2")
        );
    }

    #[test]
    fn test_parse_starting_message() {
        let msg = "starting run_id=20260103T125116Z_09ab31c2 input=/tmp/test.dd output=/tmp/out/20260103T125116Z_09ab31c2 workers=4";

        let payload = parse_starting_message(msg).unwrap();

        assert_eq!(
            payload.get("run_id").unwrap().as_str(),
            Some("20260103T125116Z_09ab31c2")
        );
        assert_eq!(
            payload.get("output").unwrap().as_str(),
            Some("/tmp/out/20260103T125116Z_09ab31c2")
        );
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

    #[test]
    fn test_parse_progress_message_v051_eta_na() {
        // Captured from `swiftbeaver 0.5.1 --log-format json` against a 2 MiB image.
        let msg = "progress 100.1% scanned=209911808/209715200 hits=17758 files=0 rate=133.42MiB/s eta=N/A errs=[carve:0 meta:0 sql:0]";
        let progress = parse_progress_message(msg).unwrap();
        assert_eq!(progress.bytes_scanned, 209_911_808);
        assert_eq!(progress.total_bytes, 209_715_200);
        assert!((progress.pct - 100.1).abs() < 1e-6);
        assert_eq!(progress.hits, 17_758);
        assert_eq!(progress.files, 0);
        assert!((progress.rate_mib - 133.42).abs() < 1e-6);
        assert_eq!(progress.eta_secs, None);
    }

    #[test]
    fn test_parse_progress_message_v051_eta_seconds() {
        // Captured from a longer 2 GiB scan with --workers 1.
        let msg = "progress 19.2% scanned=403046400/2097152000 hits=5084 files=0 rate=345.26MiB/s eta=5s errs=[carve:0 meta:0 sql:0]";
        let progress = parse_progress_message(msg).unwrap();
        assert_eq!(progress.bytes_scanned, 403_046_400);
        assert_eq!(progress.total_bytes, 2_097_152_000);
        assert_eq!(progress.eta_secs, Some(5));
        assert!((progress.rate_mib - 345.26).abs() < 1e-6);
    }

    #[test]
    fn test_parse_progress_message_legacy_still_works() {
        let msg = "progress bytes_scanned=1048576 total_bytes=10485760 pct=10.0 hits=5 files=3 rate_mib=50.25 eta_secs=Some(180)";
        let progress = parse_progress_message(msg).unwrap();
        assert_eq!(progress.bytes_scanned, 1_048_576);
        assert_eq!(progress.eta_secs, Some(180));
    }

    #[test]
    fn test_parse_progress_message_malformed_returns_none() {
        // No `progress ` prefix.
        assert!(parse_progress_message("hello world").is_none());
        // Wrong shape after prefix (no recognized fields, no `scanned=`).
        assert!(parse_progress_message("progress garbage values here").is_none());
        // v0.5.1 form but `scanned=` value is unparseable.
        assert!(parse_progress_message(
            "progress 50% scanned=abc/def hits=0 files=0 rate=1MiB/s eta=N/A"
        )
        .is_none());
    }

    #[test]
    fn test_parse_eta_variants() {
        assert_eq!(parse_eta("N/A"), None);
        assert_eq!(parse_eta("n/a"), None);
        assert_eq!(parse_eta("None"), None);
        assert_eq!(parse_eta(""), None);
        assert_eq!(parse_eta("5s"), Some(5));
        assert_eq!(parse_eta("2m"), Some(120));
        assert_eq!(parse_eta("1h"), Some(3600));
        assert_eq!(parse_eta("1m30s"), Some(90));
        assert_eq!(parse_eta("1h2m3s"), Some(3723));
        assert_eq!(parse_eta("90"), Some(90));
        assert_eq!(parse_eta("garbage"), None);
    }

    #[test]
    fn test_parse_starting_message_v051_format() {
        // Captured from swiftbeaver 0.5.1; new fields scan_workers/carve_workers/chunk_mib
        // replace the old workers/chunk_size pair, but `output=` is still present.
        let msg = "starting run_id=20260428T140203Z_09e96cdf input=test.dd output=out/20260428T140203Z_09e96cdf scan_workers=16 carve_workers=16 chunk_mib=64";
        let payload = parse_starting_message(msg).unwrap();
        assert_eq!(
            payload.get("run_id").unwrap().as_str(),
            Some("20260428T140203Z_09e96cdf")
        );
        assert_eq!(
            payload.get("output").unwrap().as_str(),
            Some("out/20260428T140203Z_09e96cdf")
        );
    }

    #[test]
    fn test_parse_json_log_v051_progress_line() {
        let line = r#"{"timestamp":"2026-04-28T14:02:03.188362Z","level":"INFO","fields":{"message":"progress 100.0% scanned=2097152/2097152 hits=2136 files=0 rate=109.81MiB/s eta=N/A errs=[carve:0 meta:0 sql:0]"},"target":"swiftbeaver"}"#;
        let (event_type, payload) = parse_json_log(line).unwrap();
        assert_eq!(event_type, "progress");
        assert_eq!(
            payload.get("bytes_scanned").unwrap().as_u64(),
            Some(2_097_152)
        );
        assert_eq!(
            payload.get("total_bytes").unwrap().as_u64(),
            Some(2_097_152)
        );
    }

    #[test]
    fn test_parse_json_log_garbage_does_not_panic() {
        assert!(parse_json_log("not json at all").is_none());
        assert!(parse_json_log("").is_none());
        // Valid JSON but missing fields.
        assert!(parse_json_log(r#"{"foo":"bar"}"#).is_none());
    }
}
