//! Scan manager - subprocess execution and monitoring

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::io::{BufRead, BufReader};

use anyhow::{bail, Context, Result};
use chrono::Utc;

use crate::config::ScanConfig;
use super::{ScanState, ScanProgress, LogEntry, find_fastcarve_binary, progress::parse_json_log};

/// Messages from scan thread to UI
#[derive(Debug, Clone)]
pub enum ScanMessage {
    Progress(ScanProgress),
    Log(LogEntry),
    StateChange(ScanState),
    Error(String),
    /// Run output path from fastcarve's "starting" log
    RunOutputPath { run_id: String, output_path: String },
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
                        self.error = Some(e);
                    }
                    ScanMessage::RunOutputPath { run_id, output_path } => {
                        // Update to the actual path from fastcarve
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

        // Find binary
        let binary_path = find_fastcarve_binary()
            .context("fastcarve binary not found. Run download-fastcarve.sh or add to PATH")?;

        // Validate paths
        if !PathBuf::from(&config.input_path).exists() {
            bail!("Input file does not exist: {}", config.input_path);
        }

        std::fs::create_dir_all(&config.output_path)
            .context("Failed to create output directory")?;

        // Create channels
        let (message_tx, message_rx) = channel::<ScanMessage>();
        let (cancel_tx, cancel_rx) = channel::<()>();

        // Reset state (run_id and run_output_path will be set from fastcarve's "starting" log)
        self.state = ScanState::Running;
        self.run_id = None;
        self.run_output_path = None;
        self.progress = None;
        self.logs.clear();
        self.error = None;
        self.message_rx = Some(message_rx);
        self.cancel_tx = Some(cancel_tx);

        // Build command arguments
        let args = build_cli_args(&config);

        tracing::info!("Starting fastcarve: {} {}", binary_path.display(), args.join(" "));
        
        self.logs.push(LogEntry {
            timestamp: Utc::now().to_rfc3339(),
            level: "INFO".to_string(),
            message: format!("Starting scan on: {}", config.input_path),
        });

        // Spawn scan thread (non-blocking!)
        std::thread::spawn(move || {
            run_scan_thread(binary_path, args, message_tx, cancel_rx);
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
    binary_path: PathBuf,
    args: Vec<String>,
    message_tx: Sender<ScanMessage>,
    cancel_rx: Receiver<()>,
) {
    // Spawn process
    let child_result = std::process::Command::new(&binary_path)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let mut child = match child_result {
        Ok(c) => c,
        Err(e) => {
            let _ = message_tx.send(ScanMessage::Error(format!("Failed to spawn fastcarve: {}", e)));
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
            let _ = message_tx.send(ScanMessage::Error("Failed to capture stdout".to_string()));
            let _ = message_tx.send(ScanMessage::StateChange(ScanState::Failed));
            return;
        }
    };
    
    let stderr = match child.stderr.take() {
        Some(s) => s,
        None => {
            let _ = message_tx.send(ScanMessage::Error("Failed to capture stderr".to_string()));
            let _ = message_tx.send(ScanMessage::StateChange(ScanState::Failed));
            return;
        }
    };

    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);
    
    // Clone for stderr thread
    let message_tx_stderr = message_tx.clone();

    // Spawn stderr reader thread
    let stderr_handle = std::thread::spawn(move || {
        for line in stderr_reader.lines().map_while(Result::ok) {
            let _ = message_tx_stderr.send(ScanMessage::Log(LogEntry {
                timestamp: Utc::now().to_rfc3339(),
                level: "STDERR".to_string(),
                message: line,
            }));
        }
    });

    // Process stdout on this thread
    let mut cancelled = false;
    for line in stdout_reader.lines().map_while(Result::ok) {
        // Check for cancel request (non-blocking)
        if cancel_rx.try_recv().is_ok() {
            cancelled = true;
            // Kill the process
            #[cfg(unix)]
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
            }
            #[cfg(not(unix))]
            {
                let _ = child.kill();
            }
            break;
        }

        // Parse JSON log line
        if let Some((event_type, payload)) = parse_json_log(&line) {
            if event_type == "progress" {
                if let Ok(prog) = serde_json::from_value::<ScanProgress>(payload) {
                    let _ = message_tx.send(ScanMessage::Progress(prog));
                }
            } else if event_type == "starting" {
                // Extract run_id and output path from starting message
                let run_id = payload.get("run_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let output_path = payload.get("output")
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
            } else {
                // It's a log entry
                let msg = payload.get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&line);
                
                let _ = message_tx.send(ScanMessage::Log(LogEntry {
                    timestamp: Utc::now().to_rfc3339(),
                    level: event_type.to_uppercase(),
                    message: msg.to_string(),
                }));
            }
        } else {
            // Plain text line
            let _ = message_tx.send(ScanMessage::Log(LogEntry {
                timestamp: Utc::now().to_rfc3339(),
                level: "INFO".to_string(),
                message: line,
            }));
        }
    }

    // Wait for stderr thread
    let _ = stderr_handle.join();

    // Wait for process to finish
    let status = child.wait();

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
                let _ = message_tx.send(ScanMessage::Error(
                    format!("Process exited with code: {:?}", s.code())
                ));
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

    // GPU
    if config.gpu_enabled {
        args.push("--gpu".to_string());
    }

    // Evidence hash
    if config.compute_evidence_hash {
        args.push("--compute-evidence-sha256".to_string());
    }

    // Resource limits
    if let Some(max_bytes) = config.max_bytes {
        args.push("--max-bytes".to_string());
        args.push(max_bytes.to_string());
    }
    if let Some(max_files) = config.max_files {
        args.push("--max-files".to_string());
        args.push(max_files.to_string());
    }
    if let Some(max_memory) = config.max_memory_mib {
        args.push("--max-memory-mib".to_string());
        args.push(max_memory.to_string());
    }

    // Workers
    if config.workers > 0 {
        args.push("--workers".to_string());
        args.push(config.workers.to_string());
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
}
