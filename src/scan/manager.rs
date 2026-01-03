//! Scan manager - subprocess execution and monitoring

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::io::{BufRead, BufReader};

use anyhow::{bail, Context, Result};
use chrono::Utc;

use crate::config::ScanConfig;
use super::{ScanState, ScanProgress, LogEntry, find_fastcarve_binary, progress::parse_json_log};

/// Manages scan lifecycle
pub struct ScanManager {
    state: ScanState,
    run_id: Option<String>,
    run_output_path: Option<String>,
    progress: Option<ScanProgress>,
    logs: Vec<LogEntry>,
    cancel_flag: Arc<AtomicBool>,
    child_pid: Option<u32>,
    error: Option<String>,
}

impl ScanManager {
    pub fn new() -> Self {
        Self {
            state: ScanState::Idle,
            run_id: None,
            run_output_path: None,
            progress: None,
            logs: Vec::new(),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            child_pid: None,
            error: None,
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

    /// Start a scan
    pub async fn start(&mut self, config: ScanConfig) -> Result<()> {
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

        // Generate run ID
        let run_id = generate_run_id();
        let run_output_path = PathBuf::from(&config.output_path).join(&run_id);
        
        // Reset state
        self.state = ScanState::Running;
        self.run_id = Some(run_id.clone());
        self.run_output_path = Some(run_output_path.display().to_string());
        self.progress = None;
        self.logs.clear();
        self.cancel_flag = Arc::new(AtomicBool::new(false));
        self.error = None;

        // Build command arguments
        let args = build_cli_args(&config);

        tracing::info!("Starting fastcarve: {} {}", binary_path.display(), args.join(" "));
        
        self.logs.push(LogEntry {
            timestamp: Utc::now().to_rfc3339(),
            level: "INFO".to_string(),
            message: format!("Starting scan: {}", run_id),
        });

        // Spawn process
        let mut child = std::process::Command::new(&binary_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn fastcarve")?;

        self.child_pid = Some(child.id());

        // Read stdout for progress
        let stdout = child.stdout.take().context("Failed to capture stdout")?;
        let stderr = child.stderr.take().context("Failed to capture stderr")?;

        let cancel_flag = self.cancel_flag.clone();

        // Process output in a blocking manner (we're already in an async context)
        let stdout_reader = BufReader::new(stdout);
        let stderr_reader = BufReader::new(stderr);

        // Spawn threads to read output
        let logs_for_stdout = &mut self.logs;
        let progress_holder = &mut self.progress;

        std::thread::scope(|s| {
            // Stdout thread
            let stdout_handle = s.spawn(|| {
                let mut local_logs = Vec::new();
                let mut local_progress = None;
                
                for line in stdout_reader.lines() {
                    if cancel_flag.load(Ordering::Relaxed) {
                        break;
                    }
                    
                    if let Ok(line) = line {
                        if let Some((event_type, payload)) = parse_json_log(&line) {
                            match event_type.as_str() {
                                "progress" => {
                                    if let Ok(prog) = serde_json::from_value::<ScanProgress>(payload) {
                                        local_progress = Some(prog);
                                    }
                                }
                                level => {
                                    if let Some(msg) = payload.get("message").and_then(|v| v.as_str()) {
                                        local_logs.push(LogEntry {
                                            timestamp: Utc::now().to_rfc3339(),
                                            level: level.to_uppercase(),
                                            message: msg.to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                
                (local_logs, local_progress)
            });

            // Stderr thread
            let stderr_handle = s.spawn(|| {
                let mut local_logs = Vec::new();
                
                for line in stderr_reader.lines() {
                    if let Ok(line) = line {
                        local_logs.push(LogEntry {
                            timestamp: Utc::now().to_rfc3339(),
                            level: "ERROR".to_string(),
                            message: line,
                        });
                    }
                }
                
                local_logs
            });

            // Wait for threads
            if let Ok((stdout_logs, progress)) = stdout_handle.join() {
                logs_for_stdout.extend(stdout_logs);
                if progress.is_some() {
                    *progress_holder = progress;
                }
            }
            
            if let Ok(stderr_logs) = stderr_handle.join() {
                logs_for_stdout.extend(stderr_logs);
            }
        });

        // Wait for process
        let status = child.wait().context("Failed to wait for process")?;

        // Update state based on result
        if self.cancel_flag.load(Ordering::Relaxed) {
            self.state = ScanState::Cancelled;
            self.logs.push(LogEntry {
                timestamp: Utc::now().to_rfc3339(),
                level: "INFO".to_string(),
                message: "Scan cancelled by user".to_string(),
            });
        } else if status.success() {
            self.state = ScanState::Completed;
            self.logs.push(LogEntry {
                timestamp: Utc::now().to_rfc3339(),
                level: "INFO".to_string(),
                message: "Scan completed successfully".to_string(),
            });
        } else {
            self.state = ScanState::Failed;
            self.error = Some(format!("Process exited with code: {:?}", status.code()));
            self.logs.push(LogEntry {
                timestamp: Utc::now().to_rfc3339(),
                level: "ERROR".to_string(),
                message: format!("Scan failed: exit code {:?}", status.code()),
            });
        }

        self.child_pid = None;
        Ok(())
    }

    /// Stop the current scan
    pub fn stop(&mut self) -> Result<()> {
        if self.state != ScanState::Running {
            bail!("No scan in progress");
        }

        self.cancel_flag.store(true, Ordering::Relaxed);

        if let Some(pid) = self.child_pid {
            #[cfg(unix)]
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
            }
        }

        Ok(())
    }
}

impl Default for ScanManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a unique run ID
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
    use crate::config::MetadataBackend;

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
