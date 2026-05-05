//! Opt-in smoke test for a real installed/bundled SwiftBeaver binary.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use swiftbeaverlodge::config::{MetadataBackend, ScanConfig};
use swiftbeaverlodge::metadata::MetadataReader;
use swiftbeaverlodge::scan::{discover_swiftbeaver, ScanManager, ScanState};
use tempfile::TempDir;

const SMOKE_TIMEOUT: Duration = Duration::from_secs(30);

#[test]
#[ignore = "requires SwiftBeaver v0.5.1+; run with `cargo test --test installed_swiftbeaver_smoke -- --ignored --nocapture`"]
fn installed_swiftbeaver_minimal_scan_loads_metadata() -> Result<()> {
    let discovered = discover_swiftbeaver().context(
        "missing binary: install SwiftBeaver v0.5.1+ on PATH, or place `swiftbeaver` in ./bin",
    )?;
    discovered.check_compatibility().map_err(|issue| {
        anyhow::anyhow!("wrong or unparseable version: {}", issue.user_message())
    })?;

    eprintln!(
        "using swiftbeaver binary: {} ({})",
        discovered.path.display(),
        discovered
            .version_string
            .as_deref()
            .unwrap_or("version output unavailable")
    );

    let temp = TempDir::new().context("failed to create temporary smoke-test directory")?;
    let input_path = temp.path().join("tiny-fixture.dd");
    std::fs::write(&input_path, tiny_jpeg_fixture())
        .context("failed to create temporary tiny fixture image")?;

    let output_dir = temp.path().join("output");
    let mut manager = ScanManager::new();
    manager
        .start(ScanConfig {
            input_path: input_path.display().to_string(),
            output_path: output_dir.display().to_string(),
            compute_evidence_hash: false,
            file_types: vec!["jpeg".to_string()],
            scan_strings: false,
            scan_urls: false,
            scan_emails: false,
            scan_phones: false,
            metadata_backend: MetadataBackend::Jsonl,
            workers: 1,
            chunk_size_mib: 1,
            ..Default::default()
        })
        .context("launch failure: failed to start scan manager")?;

    wait_for_scan(&mut manager)?;

    let run_path = manager
        .run_output_path()
        .map(PathBuf::from)
        .context("run output path discovery failure: SwiftBeaver did not report an output path")?;
    if !run_path.exists() {
        bail!(
            "run output path discovery failure: reported output path does not exist: {}",
            run_path.display()
        );
    }

    let reader = MetadataReader::new(&run_path).with_context(|| {
        format!(
            "metadata-load failure: no readable metadata found under {}",
            run_path.display()
        )
    })?;
    let files = reader.read_carved_files().with_context(|| {
        format!(
            "metadata-load failure: failed to read {} carved-file metadata",
            reader.backend()
        )
    })?;

    if files.is_empty() {
        bail!(
            "metadata-load failure: {} metadata loaded but contained no carved files",
            reader.backend()
        );
    }

    if !files.iter().any(|file| {
        file.file_type == "jpeg" || file.extension.as_deref().is_some_and(|ext| ext == "jpg")
    }) {
        bail!("metadata-load failure: expected at least one JPEG metadata row");
    }

    if !files.iter().any(|file| file.validated == Some(true)) {
        bail!("metadata-load failure: expected at least one validated JPEG metadata row");
    }

    Ok(())
}

fn wait_for_scan(manager: &mut ScanManager) -> Result<()> {
    let deadline = Instant::now() + SMOKE_TIMEOUT;

    loop {
        manager.poll();
        match manager.state() {
            ScanState::Completed => return Ok(()),
            ScanState::Failed => {
                bail!(
                    "launch failure: scan failed: {}\n{}",
                    manager.error().unwrap_or("unknown scan error"),
                    formatted_logs(manager)
                );
            }
            ScanState::Cancelled => {
                bail!("launch failure: scan was unexpectedly cancelled");
            }
            ScanState::Idle => {
                bail!("launch failure: scan returned to idle before running");
            }
            ScanState::Running => {
                if Instant::now() >= deadline {
                    bail!(
                        "launch failure: scan timed out after {:?}\n{}",
                        SMOKE_TIMEOUT,
                        formatted_logs(manager)
                    );
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

fn formatted_logs(manager: &ScanManager) -> String {
    manager
        .logs()
        .iter()
        .map(|entry| format!("{} {}: {}", entry.timestamp, entry.level, entry.message))
        .collect::<Vec<_>>()
        .join("\n")
}

fn tiny_jpeg_fixture() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(b"SwiftBeaverLodge smoke test prefix\n");
    data.extend_from_slice(&[
        0xFF, 0xD8, 0xFF, 0xE0, // SOI + APP0
        0x00, 0x10, // segment length = 16, including these two bytes
    ]);
    data.extend_from_slice(b"SwiftBeaver\0\0\0");
    data.extend_from_slice(&[
        0xFF, 0xDA, 0x00, 0x02, // minimal SOS
    ]);
    data.extend_from_slice(&[0x11; 520]);
    data.extend_from_slice(&[0xFF, 0xD9]); // EOI
    data.extend_from_slice(b"\nSwiftBeaverLodge smoke test suffix");
    data
}
