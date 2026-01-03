//! ScanManager - Manages scan lifecycle via SwiftBeaver binary
//!
//! Spawns the fastcarve binary as a subprocess, parses JSON logs for progress,
//! and handles cancellation via process signals.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use chrono::Utc;
use rand::Rng;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tracing::{info, error, warn};

use crate::types::{
    GuiScanConfig, GuiScanProgress, ScanHandle, ScanState, ScanStatus,
    ScanStateChangedEvent, LogMessageEvent,
};
use super::progress::parse_json_log;

/// Manages a single scan operation
pub struct ScanManager {
    /// Current scan state
    state: ScanState,
    /// Current run ID
    run_id: Option<String>,
    /// When the scan started
    started_at: Option<chrono::DateTime<Utc>>,
    /// Cancellation flag
    cancel_requested: Arc<AtomicBool>,
    /// Child process handle (stored to allow killing)
    child_pid: Option<u32>,
    /// Last progress snapshot
    last_progress: Option<GuiScanProgress>,
    /// Error message if failed
    error: Option<String>,
    /// Output directory for current run
    run_output_dir: Option<PathBuf>,
}

impl Default for ScanManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ScanManager {
    pub fn new() -> Self {
        Self {
            state: ScanState::Idle,
            run_id: None,
            started_at: None,
            cancel_requested: Arc::new(AtomicBool::new(false)),
            child_pid: None,
            last_progress: None,
            error: None,
            run_output_dir: None,
        }
    }

    /// Get current scan status
    pub fn get_status(&self) -> ScanStatus {
        ScanStatus {
            state: self.state,
            run_id: self.run_id.clone(),
            started_at: self.started_at.map(|dt| dt.to_rfc3339()),
            progress: self.last_progress.clone(),
            error: self.error.clone(),
        }
    }

    /// Check if a scan can be started
    pub fn can_start(&self) -> bool {
        matches!(self.state, ScanState::Idle | ScanState::Completed | ScanState::Failed | ScanState::Cancelled)
    }

    /// Start a new scan
    pub async fn start(
        &mut self,
        config: GuiScanConfig,
        app: AppHandle,
    ) -> Result<ScanHandle> {
        if !self.can_start() {
            bail!("Cannot start scan: a scan is already in progress");
        }

        // Generate run ID
        let run_id = generate_run_id();
        
        // Validate paths
        let input_path = PathBuf::from(&config.input_path);
        if !input_path.exists() {
            bail!("Input file does not exist: {}", config.input_path);
        }

        let output_path = PathBuf::from(&config.output_path);
        std::fs::create_dir_all(&output_path)
            .context("Failed to create output directory")?;

        // Update state
        self.state = ScanState::Running;
        self.run_id = Some(run_id.clone());
        self.started_at = Some(Utc::now());
        self.error = None;
        self.last_progress = None;
        self.cancel_requested = Arc::new(AtomicBool::new(false));
        
        // Run output dir is created by fastcarve
        let run_output_dir = output_path.join(&run_id);
        self.run_output_dir = Some(run_output_dir);

        // Emit state change event
        let _ = app.emit("scan:state", ScanStateChangedEvent {
            state: ScanState::Running,
            message: Some(format!("Started scan: {}", run_id)),
        });

        // Find the fastcarve binary
        let binary_path = find_fastcarve_binary(&app)?;

        // Build command arguments
        let args = build_cli_args(&config);

        info!(
            "Starting fastcarve: {} {}",
            binary_path.display(),
            args.join(" ")
        );

        // Spawn the process
        let mut child = Command::new(&binary_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn fastcarve process")?;

        // Store child PID for cancellation
        self.child_pid = child.id();

        // Clone values for the async task
        let app_clone = app.clone();
        let run_id_clone = run_id.clone();
        let cancel_flag = self.cancel_requested.clone();

        // Spawn task to handle process output
        tokio::spawn(async move {
            let result = handle_process_output(&mut child, &app_clone, &cancel_flag).await;
            
            match result {
                Ok(exit_code) => {
                    if cancel_flag.load(Ordering::Relaxed) {
                        info!("Scan cancelled: {}", run_id_clone);
                        let _ = app_clone.emit("scan:state", ScanStateChangedEvent {
                            state: ScanState::Cancelled,
                            message: Some("Scan cancelled by user".into()),
                        });
                    } else if exit_code == 0 {
                        info!("Scan completed successfully: {}", run_id_clone);
                        let _ = app_clone.emit("scan:completed", serde_json::json!({
                            "run_id": run_id_clone,
                        }));
                        let _ = app_clone.emit("scan:state", ScanStateChangedEvent {
                            state: ScanState::Completed,
                            message: Some("Scan completed successfully".into()),
                        });
                    } else {
                        error!("Scan failed with exit code: {}", exit_code);
                        let _ = app_clone.emit("scan:failed", format!("Process exited with code {}", exit_code));
                        let _ = app_clone.emit("scan:state", ScanStateChangedEvent {
                            state: ScanState::Failed,
                            message: Some(format!("Process exited with code {}", exit_code)),
                        });
                    }
                }
                Err(e) => {
                    error!("Scan failed: {}", e);
                    let _ = app_clone.emit("scan:failed", e.to_string());
                    let _ = app_clone.emit("scan:state", ScanStateChangedEvent {
                        state: ScanState::Failed,
                        message: Some(e.to_string()),
                    });
                }
            }
        });

        Ok(ScanHandle { run_id })
    }

    /// Stop the current scan
    pub fn stop(&mut self) -> Result<()> {
        if self.state != ScanState::Running {
            bail!("No scan in progress to stop");
        }

        self.cancel_requested.store(true, Ordering::Relaxed);

        // Kill the process if we have a PID
        if let Some(pid) = self.child_pid {
            #[cfg(unix)]
            {
                // Send SIGTERM for graceful shutdown
                unsafe {
                    libc::kill(pid as i32, libc::SIGTERM);
                }
            }
            #[cfg(windows)]
            {
                // On Windows, we need to use TerminateProcess
                // The process will be killed when the child handle is dropped
                warn!("Process termination on Windows may not be graceful");
            }
        }

        self.state = ScanState::Cancelled;
        info!("Scan cancellation requested");
        Ok(())
    }

    /// Update progress (called from progress parser)
    pub fn update_progress(&mut self, progress: GuiScanProgress) {
        self.last_progress = Some(progress);
    }

    /// Mark scan as completed
    pub fn mark_completed(&mut self) {
        self.state = ScanState::Completed;
        self.child_pid = None;
    }

    /// Mark scan as failed
    pub fn mark_failed(&mut self, error: String) {
        self.state = ScanState::Failed;
        self.error = Some(error);
        self.child_pid = None;
    }

    /// Reset the manager
    pub fn reset(&mut self) {
        self.state = ScanState::Idle;
        self.run_id = None;
        self.started_at = None;
        self.cancel_requested = Arc::new(AtomicBool::new(false));
        self.child_pid = None;
        self.last_progress = None;
        self.error = None;
        self.run_output_dir = None;
    }
}

/// Generate a unique run ID
fn generate_run_id() -> String {
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let random: u32 = rand::thread_rng().gen::<u32>() % 0xFFFFFF;
    format!("{}_{:06x}", timestamp, random)
}

/// Find the fastcarve binary
fn find_fastcarve_binary(app: &AppHandle) -> Result<PathBuf> {
    // 1. Check if bundled with the app (production)
    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled = resource_dir.join("bin").join("fastcarve");
        if bundled.exists() {
            return Ok(bundled);
        }
        // Try with .exe on Windows
        #[cfg(windows)]
        {
            let bundled_exe = resource_dir.join("bin").join("fastcarve.exe");
            if bundled_exe.exists() {
                return Ok(bundled_exe);
            }
        }
    }

    // 2. Check in development bin directory
    let dev_bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bin").join("fastcarve");
    if dev_bin.exists() {
        return Ok(dev_bin);
    }
    #[cfg(windows)]
    {
        let dev_bin_exe = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bin").join("fastcarve.exe");
        if dev_bin_exe.exists() {
            return Ok(dev_bin_exe);
        }
    }

    // 3. Check PATH
    if let Ok(path) = which::which("fastcarve") {
        return Ok(path);
    }

    bail!(
        "fastcarve binary not found. Please download it from \
        https://github.com/gaestu/SwiftBeaver/releases and place it in src-tauri/bin/"
    )
}

/// Build CLI arguments from GUI config
fn build_cli_args(config: &GuiScanConfig) -> Vec<String> {
    let mut args = vec![
        "--input".to_string(),
        config.input_path.clone(),
        "--output".to_string(),
        config.output_path.clone(),
        // JSON log format for machine parsing
        "--log-format".to_string(),
        "json".to_string(),
        // Progress updates every second
        "--progress-interval-secs".to_string(),
        "1".to_string(),
    ];

    // Metadata backend (parquet is default)
    args.push("--metadata-backend".to_string());
    args.push(config.metadata_backend.clone());

    // File types filter
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
    if config.string_min_len != 8 {
        args.push("--string-min-len".to_string());
        args.push(config.string_min_len.to_string());
    }

    // GPU acceleration
    if config.gpu_enabled {
        args.push("--gpu".to_string());
    }

    // Entropy detection
    if config.scan_entropy {
        args.push("--scan-entropy".to_string());
        if config.entropy_window_bytes != 256 {
            args.push("--entropy-window-bytes".to_string());
            args.push(config.entropy_window_bytes.to_string());
        }
        if config.entropy_threshold != 7.5 {
            args.push("--entropy-threshold".to_string());
            args.push(config.entropy_threshold.to_string());
        }
    }

    // SQLite page recovery
    if config.scan_sqlite_pages {
        args.push("--scan-sqlite-pages".to_string());
    }

    // Evidence hash
    if config.compute_evidence_hash {
        args.push("--compute-evidence-sha256".to_string());
    } else if let Some(ref hash) = config.evidence_sha256 {
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

    // Chunk/overlap settings
    if config.chunk_size_mib != 64 {
        args.push("--chunk-size-mib".to_string());
        args.push(config.chunk_size_mib.to_string());
    }
    if config.overlap_kib != 4 {
        args.push("--overlap-kib".to_string());
        args.push(config.overlap_kib.to_string());
    }

    // Workers
    if config.workers > 0 {
        args.push("--workers".to_string());
        args.push(config.workers.to_string());
    }

    args
}

/// Handle process output (stdout/stderr) and emit events
async fn handle_process_output(
    child: &mut Child,
    app: &AppHandle,
    cancel_flag: &Arc<AtomicBool>,
) -> Result<i32> {
    let stdout = child.stdout.take().context("Failed to capture stdout")?;
    let stderr = child.stderr.take().context("Failed to capture stderr")?;

    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    let mut stdout_lines = stdout_reader.lines();
    let mut stderr_lines = stderr_reader.lines();

    let app_stdout = app.clone();
    let app_stderr = app.clone();

    // Process stdout (JSON logs)
    let stdout_task = tokio::spawn(async move {
        while let Ok(Some(line)) = stdout_lines.next_line().await {
            if let Some((event_type, payload)) = parse_json_log(&line) {
                match event_type.as_str() {
                    "progress" => {
                        if let Ok(progress) = serde_json::from_value::<GuiScanProgress>(payload) {
                            let _ = app_stdout.emit("scan:progress", &progress);
                        }
                    }
                    "info" | "warn" | "error" | "debug" => {
                        if let Some(msg) = payload.get("message").and_then(|v| v.as_str()) {
                            let _ = app_stdout.emit("scan:log", LogMessageEvent {
                                level: event_type,
                                timestamp: Utc::now().to_rfc3339(),
                                message: msg.to_string(),
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    // Process stderr (errors)
    let stderr_task = tokio::spawn(async move {
        while let Ok(Some(line)) = stderr_lines.next_line().await {
            let _ = app_stderr.emit("scan:log", LogMessageEvent {
                level: "error".to_string(),
                timestamp: Utc::now().to_rfc3339(),
                message: line,
            });
        }
    });

    // Wait for process to complete
    let status = child.wait().await.context("Failed to wait for process")?;

    // Wait for output tasks
    let _ = stdout_task.await;
    let _ = stderr_task.await;

    Ok(status.code().unwrap_or(-1))
}
