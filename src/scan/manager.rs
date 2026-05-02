//! Scan manager - subprocess execution and monitoring

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use chrono::Utc;

use super::{discover_swiftbeaver, progress::parse_json_log, LogEntry, ScanProgress, ScanState};
use crate::config::ScanConfig;

const CANCEL_GRACE_PERIOD: Duration = Duration::from_secs(15);

/// Messages from scan thread to UI
#[derive(Debug, Clone)]
pub enum ScanMessage {
    Progress(ScanProgress),
    Log(LogEntry),
    StateChange(ScanState),
    Error(String),
    /// Run output path from swiftbeaver's "starting" log
    RunOutputPath {
        run_id: String,
        output_path: String,
    },
}

/// Manages scan lifecycle
pub struct ScanManager {
    state: ScanState,
    run_id: Option<String>,
    run_output_path: Option<String>,
    progress: Option<ScanProgress>,
    logs: Vec<LogEntry>,
    error: Option<String>,

    /// Channel to receive messages from scan thread
    message_rx: Option<Receiver<ScanMessage>>,

    /// Sender to request cancellation
    cancel_tx: Option<Sender<()>>,
}

impl ScanManager {
    pub fn new() -> Self {
        Self {
            state: ScanState::Idle,
            run_id: None,
            run_output_path: None,
            progress: None,
            logs: Vec::new(),
            error: None,
            message_rx: None,
            cancel_tx: None,
        }
    }

    pub fn state(&self) -> ScanState {
        self.state
    }

    pub fn progress(&self) -> Option<&ScanProgress> {
        self.progress.as_ref()
    }

    pub fn logs(&self) -> &[LogEntry] {
        &self.logs
    }

    pub fn run_output_path(&self) -> Option<&str> {
        self.run_output_path.as_deref()
    }

    #[allow(dead_code)]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Poll for updates from the scan thread (call from UI update loop)
    pub fn poll(&mut self) {
        // Check if we have a receiver
        let should_cleanup = if let Some(rx) = &self.message_rx {
            let mut cleanup = false;

            // Drain all pending messages (non-blocking)
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ScanMessage::Progress(p) => {
                        self.progress = Some(p);
                    }
                    ScanMessage::Log(log) => {
                        self.logs.push(log);
                    }
                    ScanMessage::StateChange(new_state) => {
                        self.state = new_state;
                        if new_state != ScanState::Running {
                            // Mark for cleanup after loop
                            cleanup = true;
                        }
                    }
                    ScanMessage::Error(e) => {
                        // Push to logs so the Monitor tab shows the message,
                        // and retain in self.error for callers that inspect it.
                        self.logs.push(LogEntry {
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            level: "ERROR".to_string(),
                            message: e.clone(),
                        });
                        self.error = Some(e);
                    }
                    ScanMessage::RunOutputPath {
                        run_id,
                        output_path,
                    } => {
                        // Update to the actual path from swiftbeaver
                        self.run_id = Some(run_id);
                        self.run_output_path = Some(output_path);
                    }
                }
            }
            cleanup
        } else {
            false
        };

        // Cleanup channels after borrow is released
        if should_cleanup {
            self.message_rx = None;
            self.cancel_tx = None;
        }
    }

    /// Start a scan (non-blocking - spawns background thread)
    pub fn start(&mut self, config: ScanConfig) -> Result<()> {
        if self.state == ScanState::Running {
            bail!("Scan already in progress");
        }

        // Reject configurations with unsafe or contradictory flag combinations
        // before any filesystem mutation or subprocess spawn. This protects
        // forensic safety even when callers bypass the UI validation panel.
        let combo_issues = crate::config::validate_flag_combinations(&config);
        if !combo_issues.is_empty() {
            bail!("Invalid scan configuration: {}", combo_issues.join("; "));
        }

        // Validate paths
        if !PathBuf::from(&config.input_path).exists() {
            bail!("Input file does not exist: {}", config.input_path);
        }

        if let Some(path) = config.resume_from.as_ref().filter(|p| !p.is_empty()) {
            let resume_path = PathBuf::from(path);
            if !resume_path.is_file() {
                bail!("Resume checkpoint file does not exist");
            }
        }

        if let Some(path) = config.checkpoint_path.as_ref().filter(|p| !p.is_empty()) {
            let checkpoint_path = PathBuf::from(path);
            if let Some(parent) = checkpoint_path.parent() {
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    bail!("Checkpoint directory does not exist");
                }
            }
        }

        std::fs::create_dir_all(&config.output_path)
            .context("Failed to create output directory")?;

        // Create channels
        let (message_tx, message_rx) = channel::<ScanMessage>();
        let (cancel_tx, cancel_rx) = channel::<()>();

        // Reset state (run_id and run_output_path will be set from swiftbeaver's "starting" log)
        self.state = ScanState::Running;
        self.run_id = None;
        self.run_output_path = None;
        self.progress = None;
        self.logs.clear();
        self.error = None;
        self.message_rx = Some(message_rx);
        self.cancel_tx = Some(cancel_tx);

        // Build command arguments
        let checkpoint_configured = config
            .checkpoint_path
            .as_ref()
            .is_some_and(|path| !path.is_empty());
        let args = build_cli_args(&config);

        self.logs.push(LogEntry {
            timestamp: Utc::now().to_rfc3339(),
            level: "INFO".to_string(),
            message: format!("Starting scan on: {}", config.input_path),
        });

        // Spawn scan thread (non-blocking!)
        std::thread::spawn(move || {
            run_scan_thread(args, checkpoint_configured, message_tx, cancel_rx);
        });

        Ok(())
    }

    /// Stop the current scan
    pub fn stop(&mut self) -> Result<()> {
        if self.state != ScanState::Running {
            bail!("No scan in progress");
        }

        // Send cancel signal through channel
        if let Some(tx) = &self.cancel_tx {
            let _ = tx.send(());
        }

        Ok(())
    }
}

impl Default for ScanManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Run the scan in a background thread - completely non-blocking to UI
fn run_scan_thread(
    args: Vec<String>,
    checkpoint_configured: bool,
    message_tx: Sender<ScanMessage>,
    cancel_rx: Receiver<()>,
) {
    let discovered = match discover_swiftbeaver().with_context(|| {
        "swiftbeaver binary not found. Install SwiftBeaver v0.5.1+ on PATH, \
         or place the `swiftbeaver` binary in <exe_dir>/bin/ or ./bin/."
            .to_string()
    }) {
        Ok(discovered) => discovered,
        Err(err) => {
            let _ = message_tx.send(ScanMessage::Error(err.to_string()));
            let _ = message_tx.send(ScanMessage::StateChange(ScanState::Failed));
            return;
        }
    };

    if let Err(issue) = discovered.check_compatibility() {
        let _ = message_tx.send(ScanMessage::Error(issue.user_message()));
        let _ = message_tx.send(ScanMessage::StateChange(ScanState::Failed));
        return;
    }

    let binary_path = discovered.path;

    tracing::info!(
        "Starting swiftbeaver: {} {}",
        binary_path.display(),
        args.join(" ")
    );

    // Spawn process
    let child_result = std::process::Command::new(&binary_path)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let mut child = match child_result {
        Ok(c) => c,
        Err(e) => {
            let _ = message_tx.send(ScanMessage::Error(format!(
                "Failed to spawn swiftbeaver: {}",
                e
            )));
            let _ = message_tx.send(ScanMessage::StateChange(ScanState::Failed));
            return;
        }
    };

    let pid = child.id();
    let _ = message_tx.send(ScanMessage::Log(LogEntry {
        timestamp: Utc::now().to_rfc3339(),
        level: "INFO".to_string(),
        message: format!("Process started with PID: {}", pid),
    }));

    // Get stdout/stderr
    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => {
            let _ = child.kill();
            let _ = message_tx.send(ScanMessage::Error("Failed to capture stdout".to_string()));
            let _ = message_tx.send(ScanMessage::StateChange(ScanState::Failed));
            return;
        }
    };

    let stderr = match child.stderr.take() {
        Some(s) => s,
        None => {
            let _ = child.kill();
            let _ = message_tx.send(ScanMessage::Error("Failed to capture stderr".to_string()));
            let _ = message_tx.send(ScanMessage::StateChange(ScanState::Failed));
            return;
        }
    };

    let (line_tx, line_rx) = channel::<(String, &'static str)>();

    let stdout_handle = spawn_stream_reader(stdout, line_tx.clone(), "INFO");
    let stderr_handle = spawn_stream_reader(stderr, line_tx, "STDERR");

    let mut cancelled = false;
    let mut cancel_requested_at: Option<Instant> = None;
    let mut forced_kill_sent = false;
    let status = loop {
        drain_log_lines(&line_rx, &message_tx);

        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Err(err) => break Err(err),
            Ok(None) => {}
        }

        if !cancelled && cancel_rx.try_recv().is_ok() {
            cancelled = true;
            cancel_requested_at = Some(Instant::now());
            let detail = if checkpoint_configured {
                "Cancellation requested; asking SwiftBeaver to stop so it can write checkpoint state"
            } else {
                "Cancellation requested; asking SwiftBeaver to stop"
            };
            let _ = message_tx.send(ScanMessage::Log(LogEntry {
                timestamp: Utc::now().to_rfc3339(),
                level: "INFO".to_string(),
                message: detail.to_string(),
            }));
            if let Err(err) = request_graceful_termination(&mut child, pid) {
                let _ = message_tx.send(ScanMessage::Log(LogEntry {
                    timestamp: Utc::now().to_rfc3339(),
                    level: "WARN".to_string(),
                    message: format!("Failed to request graceful stop: {err}"),
                }));
            }
        }

        if let Some(started) = cancel_requested_at {
            if !forced_kill_sent && started.elapsed() >= CANCEL_GRACE_PERIOD {
                forced_kill_sent = true;
                let _ = message_tx.send(ScanMessage::Log(LogEntry {
                    timestamp: Utc::now().to_rfc3339(),
                    level: "WARN".to_string(),
                    message:
                        "SwiftBeaver did not stop after cancellation grace period; forcing exit"
                            .to_string(),
                }));
                let _ = child.kill();
            }
        }

        std::thread::sleep(Duration::from_millis(50));
    };

    let _ = stdout_handle.join();
    let _ = stderr_handle.join();
    drain_log_lines(&line_rx, &message_tx);

    // Determine final state and send it
    let final_state = if cancelled {
        let _ = message_tx.send(ScanMessage::Log(LogEntry {
            timestamp: Utc::now().to_rfc3339(),
            level: "INFO".to_string(),
            message: "Scan cancelled by user".to_string(),
        }));
        ScanState::Cancelled
    } else {
        match status {
            Ok(s) if s.success() => {
                let _ = message_tx.send(ScanMessage::Log(LogEntry {
                    timestamp: Utc::now().to_rfc3339(),
                    level: "INFO".to_string(),
                    message: "Scan completed successfully".to_string(),
                }));
                ScanState::Completed
            }
            Ok(s) => {
                let _ = message_tx.send(ScanMessage::Error(format!(
                    "Process exited with code: {:?}",
                    s.code()
                )));
                let _ = message_tx.send(ScanMessage::Log(LogEntry {
                    timestamp: Utc::now().to_rfc3339(),
                    level: "ERROR".to_string(),
                    message: format!("Scan failed: exit code {:?}", s.code()),
                }));
                ScanState::Failed
            }
            Err(e) => {
                let _ = message_tx.send(ScanMessage::Error(format!("Wait error: {}", e)));
                ScanState::Failed
            }
        }
    };

    let _ = message_tx.send(ScanMessage::StateChange(final_state));
}

fn spawn_stream_reader<R: std::io::Read + Send + 'static>(
    stream: R,
    line_tx: Sender<(String, &'static str)>,
    default_level: &'static str,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let reader = BufReader::new(stream);
        for line in reader.lines().map_while(std::result::Result::ok) {
            if line_tx.send((line, default_level)).is_err() {
                break;
            }
        }
    })
}

fn drain_log_lines(line_rx: &Receiver<(String, &'static str)>, message_tx: &Sender<ScanMessage>) {
    while let Ok((line, default_level)) = line_rx.try_recv() {
        dispatch_log_line(&line, message_tx, default_level);
    }
}

fn request_graceful_termination(_child: &mut Child, pid: u32) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        // SwiftBeaver installs a Ctrl-C handler; SIGINT matches that graceful
        // interruption path more closely than unconditional termination.
        let result = unsafe { libc::kill(pid as i32, libc::SIGINT) };
        if result == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }

    #[cfg(not(unix))]
    {
        _child.kill()
    }
}

/// Parse a single log line from SwiftBeaver and forward it as one or more
/// [`ScanMessage`]s. Used for both stdout and stderr; `default_level` is the
/// fallback log level for plain-text lines that do not parse as JSON.
fn dispatch_log_line(line: &str, message_tx: &Sender<ScanMessage>, default_level: &str) {
    let Some((event_type, payload)) = parse_json_log(line) else {
        // Plain text or malformed JSON. Surface it as a log entry so users
        // still see what SwiftBeaver wrote, without crashing the manager.
        if !line.is_empty() {
            let _ = message_tx.send(ScanMessage::Log(LogEntry {
                timestamp: Utc::now().to_rfc3339(),
                level: default_level.to_string(),
                message: line.to_string(),
            }));
        }
        return;
    };

    match event_type.as_str() {
        "progress" => {
            if let Ok(prog) = serde_json::from_value::<ScanProgress>(payload) {
                let _ = message_tx.send(ScanMessage::Progress(prog));
            }
        }
        "starting" => {
            let run_id = payload
                .get("run_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let output_path = payload
                .get("output")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if !output_path.is_empty() {
                let _ = message_tx.send(ScanMessage::RunOutputPath {
                    run_id: run_id.clone(),
                    output_path: output_path.clone(),
                });
                let _ = message_tx.send(ScanMessage::Log(LogEntry {
                    timestamp: Utc::now().to_rfc3339(),
                    level: "INFO".to_string(),
                    message: format!("Run ID: {} → Output: {}", run_id, output_path),
                }));
            }
        }
        _ => {
            let msg = payload
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or(line);
            let _ = message_tx.send(ScanMessage::Log(LogEntry {
                timestamp: Utc::now().to_rfc3339(),
                level: event_type.to_uppercase(),
                message: msg.to_string(),
            }));
        }
    }
}

/// Generate a unique run ID
#[allow(dead_code)]
fn generate_run_id() -> String {
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let random: u32 = rand::random::<u32>() % 0xFFFFFF;
    format!("{}_{:06x}", timestamp, random)
}

/// Build CLI arguments from config
fn build_cli_args(config: &ScanConfig) -> Vec<String> {
    let mut args = vec![
        "--input".to_string(),
        config.input_path.clone(),
        "--output".to_string(),
        config.output_path.clone(),
        "--log-format".to_string(),
        "json".to_string(),
        "--progress-interval-secs".to_string(),
        "1".to_string(),
        "--metadata-backend".to_string(),
        config.metadata_backend.as_str().to_string(),
    ];

    // Optional YAML config file (merged with CLI flags by SwiftBeaver)
    if let Some(path) = config.config_path.as_ref().filter(|p| !p.is_empty()) {
        args.push("--config-path".to_string());
        args.push(path.clone());
    }

    // File types
    if !config.file_types.is_empty() {
        args.push("--types".to_string());
        args.push(config.file_types.join(","));
    }

    if config.disable_zip {
        args.push("--disable-zip".to_string());
    }

    // String scanning
    if config.scan_strings {
        args.push("--scan-strings".to_string());
    }
    if config.scan_utf16 {
        args.push("--scan-utf16".to_string());
    }
    if !config.scan_urls {
        args.push("--no-scan-urls".to_string());
    }
    if !config.scan_emails {
        args.push("--no-scan-emails".to_string());
    }
    if !config.scan_phones {
        args.push("--no-scan-phones".to_string());
    }
    if config.string_min_len > 0 && config.string_min_len != ScanConfig::default().string_min_len {
        args.push("--string-min-len".to_string());
        args.push(config.string_min_len.to_string());
    }

    // Entropy
    if config.scan_entropy {
        args.push("--scan-entropy".to_string());
        if let Some(window) = config.entropy_window_bytes {
            args.push("--entropy-window-bytes".to_string());
            args.push(window.to_string());
        }
        // Only emit threshold when it differs from the default to avoid
        // overriding SwiftBeaver's tuned default unintentionally.
        if (config.entropy_threshold - ScanConfig::default().entropy_threshold).abs() > f64::EPSILON
        {
            args.push("--entropy-threshold".to_string());
            args.push(format!("{}", config.entropy_threshold));
        }
    }

    // GPU
    if config.gpu_enabled {
        args.push("--gpu".to_string());
    }

    // Evidence hash
    if config.compute_evidence_hash {
        args.push("--compute-evidence-sha256".to_string());
    }
    if let Some(hash) = config.evidence_sha256.as_ref().filter(|h| !h.is_empty()) {
        args.push("--evidence-sha256".to_string());
        args.push(hash.clone());
    }

    // Resource limits
    if let Some(max_bytes) = config.max_bytes {
        args.push("--max-bytes".to_string());
        args.push(max_bytes.to_string());
    }
    if let Some(max_chunks) = config.max_chunks {
        args.push("--max-chunks".to_string());
        args.push(max_chunks.to_string());
    }
    if let Some(max_files) = config.max_files {
        args.push("--max-files".to_string());
        args.push(max_files.to_string());
    }
    if let Some(max_memory) = config.max_memory_mib {
        args.push("--max-memory-mib".to_string());
        args.push(max_memory.to_string());
    }
    if let Some(max_open) = config.max_open_files {
        args.push("--max-open-files".to_string());
        args.push(max_open.to_string());
    }

    // Workers
    if config.workers > 0 {
        args.push("--workers".to_string());
        args.push(config.workers.to_string());
    }
    if config.scan_workers > 0 {
        args.push("--scan-workers".to_string());
        args.push(config.scan_workers.to_string());
    }
    if config.carve_workers > 0 {
        args.push("--carve-workers".to_string());
        args.push(config.carve_workers.to_string());
    }
    if config.write_workers > 0 {
        args.push("--write-workers".to_string());
        args.push(config.write_workers.to_string());
    }

    // Emit chunk size explicitly so every launched scan is self-describing,
    // including future resumes if SwiftBeaver's default ever changes.
    args.push("--chunk-size-mib".to_string());
    args.push(config.chunk_size_mib.to_string());
    if let Some(overlap) = config.overlap_kib {
        args.push("--overlap-kib".to_string());
        args.push(overlap.to_string());
    }

    // Run mode
    if config.dry_run {
        args.push("--dry-run".to_string());
    }
    if config.metadata_only {
        args.push("--metadata-only".to_string());
    }

    // Post-carving validation
    if config.validate_carved {
        args.push("--validate-carved".to_string());
    }
    if config.remove_invalid {
        args.push("--remove-invalid".to_string());
    }

    // Hashing & dedup
    if !config.hash_algorithms.is_empty() {
        args.push("--hash-algorithms".to_string());
        args.push(config.hash_algorithms.join(","));
    }
    if config.dedupe {
        args.push("--dedupe".to_string());
    }
    if config.skip_duplicates {
        args.push("--skip-duplicates".to_string());
    }

    // Checkpoint / resume
    if let Some(path) = config.checkpoint_path.as_ref().filter(|p| !p.is_empty()) {
        args.push("--checkpoint-path".to_string());
        args.push(path.clone());
    }
    if let Some(path) = config.resume_from.as_ref().filter(|p| !p.is_empty()) {
        args.push("--resume-from".to_string());
        args.push(path.clone());
    }

    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_run_id() {
        let id1 = generate_run_id();
        let id2 = generate_run_id();
        assert_ne!(id1, id2);
        assert!(id1.len() > 10);
    }

    #[test]
    fn test_build_cli_args_basic() {
        let config = ScanConfig {
            input_path: "/tmp/test.dd".to_string(),
            output_path: "/tmp/output".to_string(),
            ..Default::default()
        };

        let args = build_cli_args(&config);

        assert!(args.contains(&"--input".to_string()));
        assert!(args.contains(&"/tmp/test.dd".to_string()));
        assert!(args.contains(&"--output".to_string()));
        assert!(args.contains(&"--log-format".to_string()));
        assert!(args.contains(&"json".to_string()));
        assert!(args.contains(&"--metadata-backend".to_string()));
        assert!(args.contains(&"parquet".to_string()));
    }

    #[test]
    fn test_build_cli_args_with_options() {
        let config = ScanConfig {
            input_path: "/tmp/test.dd".to_string(),
            output_path: "/tmp/output".to_string(),
            scan_strings: true,
            gpu_enabled: true,
            max_files: Some(1000),
            ..Default::default()
        };

        let args = build_cli_args(&config);

        assert!(args.contains(&"--scan-strings".to_string()));
        assert!(args.contains(&"--gpu".to_string()));
        assert!(args.contains(&"--max-files".to_string()));
        assert!(args.contains(&"1000".to_string()));
    }

    #[test]
    fn test_scan_manager_initial_state() {
        let manager = ScanManager::new();
        assert_eq!(manager.state(), ScanState::Idle);
        assert!(manager.progress().is_none());
        assert!(manager.logs().is_empty());
    }

    /// Helper: locate a single-arg flag and return its value.
    fn flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
        let idx = args.iter().position(|a| a == flag)?;
        args.get(idx + 1).map(|s| s.as_str())
    }

    #[test]
    fn test_build_cli_args_default_emits_chunk_size_but_no_optional_flags() {
        let config = ScanConfig {
            input_path: "/tmp/test.dd".to_string(),
            output_path: "/tmp/output".to_string(),
            ..Default::default()
        };
        let args = build_cli_args(&config);

        // None of the new optional flags should appear by default.
        for flag in [
            "--config-path",
            "--scan-workers",
            "--carve-workers",
            "--write-workers",
            "--overlap-kib",
            "--string-min-len",
            "--scan-entropy",
            "--entropy-window-bytes",
            "--entropy-threshold",
            "--max-chunks",
            "--max-open-files",
            "--checkpoint-path",
            "--resume-from",
            "--evidence-sha256",
            "--dry-run",
            "--metadata-only",
            "--validate-carved",
            "--remove-invalid",
            "--hash-algorithms",
            "--dedupe",
            "--skip-duplicates",
        ] {
            assert!(
                !args.iter().any(|a| a == flag),
                "default args unexpectedly contain {flag}: {args:?}"
            );
        }
        assert_eq!(flag_value(&args, "--chunk-size-mib"), Some("64"));
    }

    #[test]
    fn test_build_cli_args_advanced_options_snapshot() {
        let config = ScanConfig {
            input_path: "/evidence/disk.dd".to_string(),
            output_path: "/out".to_string(),
            config_path: Some("/etc/swiftbeaver.yaml".to_string()),
            evidence_sha256: Some("deadbeef".to_string()),
            scan_workers: 4,
            carve_workers: 8,
            write_workers: 2,
            workers: 16,
            chunk_size_mib: 128,
            overlap_kib: Some(128),
            string_min_len: 12,
            scan_entropy: true,
            entropy_window_bytes: Some(4096),
            entropy_threshold: 6.5,
            max_chunks: Some(10),
            max_open_files: Some(2048),
            checkpoint_path: Some("/var/tmp/run.ckpt".to_string()),
            resume_from: Some("/var/tmp/run.ckpt".to_string()),
            dry_run: false,
            metadata_only: false,
            validate_carved: true,
            remove_invalid: true,
            hash_algorithms: vec!["md5".to_string(), "sha256".to_string()],
            dedupe: true,
            skip_duplicates: true,
            ..Default::default()
        };

        let args = build_cli_args(&config);

        assert_eq!(
            flag_value(&args, "--config-path"),
            Some("/etc/swiftbeaver.yaml")
        );
        assert_eq!(flag_value(&args, "--evidence-sha256"), Some("deadbeef"));
        assert_eq!(flag_value(&args, "--workers"), Some("16"));
        assert_eq!(flag_value(&args, "--scan-workers"), Some("4"));
        assert_eq!(flag_value(&args, "--carve-workers"), Some("8"));
        assert_eq!(flag_value(&args, "--write-workers"), Some("2"));
        assert_eq!(flag_value(&args, "--chunk-size-mib"), Some("128"));
        assert_eq!(flag_value(&args, "--overlap-kib"), Some("128"));
        assert_eq!(flag_value(&args, "--string-min-len"), Some("12"));
        assert!(args.iter().any(|a| a == "--scan-entropy"));
        assert_eq!(flag_value(&args, "--entropy-window-bytes"), Some("4096"));
        assert_eq!(flag_value(&args, "--entropy-threshold"), Some("6.5"));
        assert_eq!(flag_value(&args, "--max-chunks"), Some("10"));
        assert_eq!(flag_value(&args, "--max-open-files"), Some("2048"));
        assert_eq!(
            flag_value(&args, "--checkpoint-path"),
            Some("/var/tmp/run.ckpt")
        );
        assert_eq!(
            flag_value(&args, "--resume-from"),
            Some("/var/tmp/run.ckpt")
        );
        assert!(!args.iter().any(|a| a == "--metadata-only"));
        assert!(!args.iter().any(|a| a == "--dry-run"));
        assert!(args.iter().any(|a| a == "--validate-carved"));
        assert!(args.iter().any(|a| a == "--remove-invalid"));
        assert_eq!(flag_value(&args, "--hash-algorithms"), Some("md5,sha256"));
        assert!(args.iter().any(|a| a == "--dedupe"));
        assert!(args.iter().any(|a| a == "--skip-duplicates"));
    }

    #[test]
    fn test_build_cli_args_empty_optional_strings_are_skipped() {
        let config = ScanConfig {
            input_path: "/tmp/test.dd".to_string(),
            output_path: "/tmp/output".to_string(),
            config_path: Some(String::new()),
            evidence_sha256: Some(String::new()),
            checkpoint_path: Some(String::new()),
            resume_from: Some(String::new()),
            ..Default::default()
        };
        let args = build_cli_args(&config);
        for flag in [
            "--config-path",
            "--evidence-sha256",
            "--checkpoint-path",
            "--resume-from",
        ] {
            assert!(
                !args.iter().any(|a| a == flag),
                "empty optional emitted {flag}: {args:?}"
            );
        }
    }

    #[test]
    fn test_build_cli_args_checkpoint_mode_includes_chunk_geometry() {
        let config = ScanConfig {
            input_path: "/tmp/test.dd".to_string(),
            output_path: "/tmp/output".to_string(),
            checkpoint_path: Some("/tmp/run.ckpt".to_string()),
            chunk_size_mib: 256,
            overlap_kib: Some(96),
            ..Default::default()
        };

        let args = build_cli_args(&config);

        assert_eq!(
            flag_value(&args, "--checkpoint-path"),
            Some("/tmp/run.ckpt")
        );
        assert_eq!(flag_value(&args, "--chunk-size-mib"), Some("256"));
        assert_eq!(flag_value(&args, "--overlap-kib"), Some("96"));
        assert_eq!(flag_value(&args, "--resume-from"), None);
    }

    #[test]
    fn test_build_cli_args_resume_mode_includes_chunk_geometry() {
        let config = ScanConfig {
            input_path: "/tmp/test.dd".to_string(),
            output_path: "/tmp/output".to_string(),
            resume_from: Some("/tmp/run.ckpt".to_string()),
            chunk_size_mib: 128,
            overlap_kib: Some(32),
            ..Default::default()
        };

        let args = build_cli_args(&config);

        assert_eq!(flag_value(&args, "--resume-from"), Some("/tmp/run.ckpt"));
        assert_eq!(flag_value(&args, "--chunk-size-mib"), Some("128"));
        assert_eq!(flag_value(&args, "--overlap-kib"), Some("32"));
        assert_eq!(flag_value(&args, "--checkpoint-path"), None);
    }

    #[test]
    fn test_build_cli_args_resume_mode_emits_default_chunk_size() {
        let config = ScanConfig {
            input_path: "/tmp/test.dd".to_string(),
            output_path: "/tmp/output".to_string(),
            resume_from: Some("/tmp/run.ckpt".to_string()),
            ..Default::default()
        };

        let args = build_cli_args(&config);

        assert_eq!(flag_value(&args, "--chunk-size-mib"), Some("64"));
    }

    #[test]
    fn test_build_cli_args_entropy_window_only_when_scan_entropy_enabled() {
        let config = ScanConfig {
            input_path: "/tmp/x".to_string(),
            output_path: "/tmp/o".to_string(),
            scan_entropy: false,
            entropy_window_bytes: Some(8192),
            ..Default::default()
        };
        let args = build_cli_args(&config);
        assert!(!args.iter().any(|a| a == "--entropy-window-bytes"));
        assert!(!args.iter().any(|a| a == "--scan-entropy"));
    }

    #[test]
    fn test_build_cli_args_metadata_only_mode() {
        let config = ScanConfig {
            input_path: "/tmp/x".to_string(),
            output_path: "/tmp/o".to_string(),
            metadata_only: true,
            ..Default::default()
        };
        let args = build_cli_args(&config);

        assert!(args.iter().any(|a| a == "--metadata-only"));
        assert!(!args.iter().any(|a| a == "--validate-carved"));
        assert!(!args.iter().any(|a| a == "--remove-invalid"));
    }

    #[test]
    fn test_validate_flag_combinations_detects_invalid_pairs() {
        use crate::config::validate_flag_combinations;

        let config = ScanConfig {
            dry_run: true,
            metadata_only: true,
            validate_carved: false,
            remove_invalid: true,
            dedupe: false,
            skip_duplicates: true,
            hash_algorithms: vec!["sha1".to_string()],
            chunk_size_mib: 0,
            ..Default::default()
        };

        let issues = validate_flag_combinations(&config);
        assert!(issues
            .iter()
            .any(|i| i.contains("dry-run") && i.contains("metadata-only")));
        assert!(issues.iter().any(|i| i.contains("--remove-invalid")));
        assert!(issues.iter().any(|i| i.contains("--skip-duplicates")));
        assert!(issues.iter().any(|i| i.contains("sha1")));
        assert!(issues.iter().any(|i| i.contains("--chunk-size-mib")));
    }

    #[test]
    fn test_validate_flag_combinations_accepts_valid_combo() {
        use crate::config::validate_flag_combinations;

        let config = ScanConfig {
            dry_run: false,
            metadata_only: false,
            validate_carved: true,
            remove_invalid: true,
            dedupe: true,
            skip_duplicates: true,
            hash_algorithms: vec!["MD5".to_string(), "sha256".to_string()],
            ..Default::default()
        };
        assert!(validate_flag_combinations(&config).is_empty());
    }

    #[test]
    fn test_validate_flag_combinations_rejects_checkpoint_on_evidence() {
        use crate::config::validate_flag_combinations;

        let config = ScanConfig {
            input_path: "/evidence/disk.dd".to_string(),
            checkpoint_path: Some("/evidence/disk.dd".to_string()),
            ..Default::default()
        };
        let issues = validate_flag_combinations(&config);
        assert!(
            issues
                .iter()
                .any(|i| i.contains("--checkpoint-path") && i.contains("evidence")),
            "expected evidence-overwrite rejection, got: {issues:?}"
        );
    }

    #[test]
    fn test_validate_flag_combinations_rejects_checkpoint_on_raw_device() {
        use crate::config::validate_flag_combinations;

        let config = ScanConfig {
            input_path: "/dev/sda".to_string(),
            checkpoint_path: Some("/dev/sdb".to_string()),
            ..Default::default()
        };
        let issues = validate_flag_combinations(&config);
        assert!(
            issues.iter().any(|i| i.contains("raw block device")),
            "expected raw-device rejection, got: {issues:?}"
        );
    }

    #[test]
    fn test_validate_flag_combinations_allows_safe_checkpoint_path() {
        use crate::config::validate_flag_combinations;

        let config = ScanConfig {
            input_path: "/evidence/disk.dd".to_string(),
            checkpoint_path: Some("/var/tmp/run.ckpt".to_string()),
            ..Default::default()
        };
        assert!(validate_flag_combinations(&config).is_empty());
    }

    #[test]
    fn test_start_rejects_unsafe_checkpoint_before_spawning() {
        let mut manager = ScanManager::new();
        let config = ScanConfig {
            // Intentionally point input at a likely-existing path so we know
            // failure comes from the validator, not the path-existence check.
            input_path: "/tmp".to_string(),
            output_path: "/tmp/sblodge_test_output".to_string(),
            checkpoint_path: Some("/dev/null".to_string()),
            ..Default::default()
        };

        let err = manager
            .start(config)
            .expect_err("start() must reject raw-device checkpoint paths");
        let msg = format!("{err}");
        assert!(
            msg.contains("Invalid scan configuration") && msg.contains("raw block device"),
            "unexpected error: {msg}"
        );
        assert_eq!(manager.state(), ScanState::Idle);
    }

    #[test]
    fn test_start_rejects_missing_resume_checkpoint_before_spawning() {
        let temp = tempfile::tempdir().unwrap();
        let missing_checkpoint = temp.path().join("missing.ckpt");
        let mut manager = ScanManager::new();
        let config = ScanConfig {
            input_path: "/tmp".to_string(),
            output_path: temp.path().join("out").display().to_string(),
            resume_from: Some(missing_checkpoint.display().to_string()),
            ..Default::default()
        };

        let err = manager
            .start(config)
            .expect_err("start() must reject missing resume checkpoints");
        let msg = format!("{err}");
        assert!(
            msg.contains("Resume checkpoint file does not exist"),
            "unexpected error: {msg}"
        );
        assert_eq!(manager.state(), ScanState::Idle);
    }

    #[test]
    fn test_start_rejects_missing_checkpoint_parent_before_spawning() {
        let temp = tempfile::tempdir().unwrap();
        let checkpoint = temp.path().join("missing-dir").join("run.ckpt");
        let mut manager = ScanManager::new();
        let config = ScanConfig {
            input_path: "/tmp".to_string(),
            output_path: temp.path().join("out").display().to_string(),
            checkpoint_path: Some(checkpoint.display().to_string()),
            ..Default::default()
        };

        let err = manager
            .start(config)
            .expect_err("start() must reject missing checkpoint parent directories");
        let msg = format!("{err}");
        assert!(
            msg.contains("Checkpoint directory does not exist"),
            "unexpected error: {msg}"
        );
        assert_eq!(manager.state(), ScanState::Idle);
    }

    #[test]
    fn test_dispatch_log_line_v051_progress() {
        let (tx, rx) = channel::<ScanMessage>();
        let line = r#"{"timestamp":"2026-04-28T14:02:03.188362Z","level":"INFO","fields":{"message":"progress 50.0% scanned=1048576/2097152 hits=12 files=0 rate=42.5MiB/s eta=5s errs=[carve:0 meta:0 sql:0]"},"target":"swiftbeaver"}"#;
        dispatch_log_line(line, &tx, "INFO");
        match rx.try_recv().expect("expected a progress message") {
            ScanMessage::Progress(p) => {
                assert_eq!(p.bytes_scanned, 1_048_576);
                assert_eq!(p.total_bytes, 2_097_152);
                assert_eq!(p.eta_secs, Some(5));
            }
            other => panic!("expected Progress, got {other:?}"),
        }
    }

    #[test]
    fn test_dispatch_log_line_starting_emits_run_output_path() {
        let (tx, rx) = channel::<ScanMessage>();
        let line = r#"{"timestamp":"2026-04-28T14:02:03.166515Z","level":"INFO","fields":{"message":"starting run_id=20260428T140203Z_09e96cdf input=test.dd output=out/20260428T140203Z_09e96cdf scan_workers=16 carve_workers=16 chunk_mib=64"},"target":"swiftbeaver"}"#;
        dispatch_log_line(line, &tx, "INFO");
        let first = rx.try_recv().expect("expected RunOutputPath message");
        match first {
            ScanMessage::RunOutputPath {
                run_id,
                output_path,
            } => {
                assert_eq!(run_id, "20260428T140203Z_09e96cdf");
                assert_eq!(output_path, "out/20260428T140203Z_09e96cdf");
            }
            other => panic!("expected RunOutputPath, got {other:?}"),
        }
        // A follow-up Log entry is also emitted.
        assert!(matches!(rx.try_recv(), Ok(ScanMessage::Log(_))));
    }

    #[test]
    fn test_dispatch_log_line_plain_text_uses_default_level() {
        let (tx, rx) = channel::<ScanMessage>();
        dispatch_log_line("not json at all", &tx, "STDERR");
        match rx.try_recv().expect("expected log message") {
            ScanMessage::Log(entry) => {
                assert_eq!(entry.level, "STDERR");
                assert_eq!(entry.message, "not json at all");
            }
            other => panic!("expected Log, got {other:?}"),
        }
    }

    #[test]
    fn test_dispatch_log_line_empty_line_is_ignored() {
        let (tx, rx) = channel::<ScanMessage>();
        dispatch_log_line("", &tx, "STDERR");
        assert!(rx.try_recv().is_err(), "empty line must not emit a message");
    }

    #[test]
    fn test_dispatch_log_line_does_not_panic_on_garbage_json() {
        let (tx, rx) = channel::<ScanMessage>();
        // Valid JSON but missing the fields parse_json_log requires.
        dispatch_log_line(r#"{"foo":"bar"}"#, &tx, "INFO");
        // Should be surfaced as a plain log line at the default level.
        match rx.try_recv().expect("expected log message") {
            ScanMessage::Log(entry) => {
                assert_eq!(entry.level, "INFO");
                assert!(entry.message.contains("foo"));
            }
            other => panic!("expected Log, got {other:?}"),
        }
    }
}
