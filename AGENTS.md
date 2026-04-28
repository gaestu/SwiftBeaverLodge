# AGENTS.md

Guidelines for AI coding agents working on SwiftBeaverLodge, the Rust egui desktop frontend for the SwiftBeaver forensic file carver.

Read this file first when working in this repository. It is the compact base instruction set for the prompt workflows in `prompts/`.

When requirements are unclear, prefer conservative, backward-compatible changes that preserve forensic safety, UI responsiveness, and the current subprocess boundary.

## Core Invariants

- Never modify evidence files or source images.
- Treat SwiftBeaver as an external binary invoked through `std::process::Command`.
- Do not reimplement carving, parsing, or metadata generation in the GUI unless the task explicitly changes that architecture.
- Keep `eframe::App::update` non-blocking.
- Preserve the selected SwiftBeaver variant and generated CLI arguments exactly.
- Load results from the run output path reported by SwiftBeaver, not a guessed path.
- Do not expose evidence paths in user-facing error messages unless the task explicitly requires it.
- Treat metadata values and carved file paths as untrusted when adding preview, open, export, or cleanup features.
- Keep normal scanning and result browsing offline/airgapped friendly.

## Project Overview

- Language: Rust 2021
- GUI: egui 0.29 + eframe
- Metadata reading: parquet + arrow, JSONL fallback
- Process integration: bundled or PATH-provided `swiftbeaver-cpu-only`, `swiftbeaver-opencl`, and `swiftbeaver-cuda`
- Standard checks:
  - `cargo fmt`
  - `cargo check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`

## Repository Layout

- `src/app.rs` - main application state and egui update loop
- `src/config.rs` - `ScanConfig`, `MetadataBackend`, `GpuVariant`, and file type lists
- `src/scan/` - binary discovery, subprocess lifecycle, cancellation, progress/log parsing
- `src/metadata/` - Parquet/JSONL detection, readers, and result types
- `src/ui/` - configuration, progress, and results panels
- `tests/` - integration tests that should not require a real SwiftBeaver binary to pass
- `bin/` - optional downloaded SwiftBeaver binary variants
- `download-swiftbeaver.sh` - helper for obtaining variants

## Code Rules

- Use existing module boundaries before adding new abstractions.
- Keep long-running work out of the UI thread.
- Build subprocess invocations with argument vectors, never shell-concatenated commands.
- Use `tracing` for app logs.
- Avoid `.unwrap()` and `.expect()` in normal application paths.
- Add context to process, filesystem, and metadata errors.
- Do not hardcode the Lodge version; use `env!("CARGO_PKG_VERSION")`.
- Avoid new dependencies unless they clearly remove meaningful complexity.

## High-Risk Areas

- `src/scan/manager.rs`: CLI argument mapping, process lifecycle, cancellation, state transitions
- `src/scan/progress.rs`: SwiftBeaver JSON log compatibility and malformed-line handling
- `src/scan/mod.rs`: binary discovery and GPU variant availability
- `src/metadata/reader.rs`: Parquet/JSONL schema assumptions, optional columns, large result sets
- `src/ui/results_panel.rs`: filtering, selected index handling, bounded rendering, future file-opening paths
- `src/app.rs`: locking, tab transitions, and repaint behavior

## Documentation and Tests

- Update README/docs when user-facing workflow, binary setup, CLI flags, metadata support, or UI behavior changes.
- Add deterministic tests for new parsing, config, metadata, and state-transition behavior.
- Use `tempfile` for test files and directories.
- Do not make the main test suite depend on a downloaded SwiftBeaver binary.

## Before Finishing

For code changes, run the narrowest useful checks and, when feasible, finish with:

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

For prompt-only changes, validate that stale references to the upstream SwiftBeaver engine, Tauri/Svelte plans, parser/carver internals, and non-existent docs are removed or explicitly marked as historical.
