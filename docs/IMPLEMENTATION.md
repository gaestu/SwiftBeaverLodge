# SwiftBeaverLodge Implementation Notes

> Current architecture notes for the Rust egui desktop frontend.

**Status:** Current as of May 2, 2026<br>
**SwiftBeaver compatibility:** v0.5.1+ unified CLI

This document replaces the original Tauri/Svelte/direct-crate implementation
plan. That plan is historical: SwiftBeaverLodge is implemented as a pure Rust
desktop application using `egui`/`eframe`, and it treats SwiftBeaver as an
external `swiftbeaver` executable invoked through `std::process::Command`.
`AGENTS.md`, `README.md`, and the `src/` tree remain the source of truth for
active work.

## Current Architecture

SwiftBeaverLodge has three main layers:

| Layer | Files | Responsibility |
|-------|-------|----------------|
| Application state | `src/app.rs` | Owns scan state, tab transitions, background messages, and repaint requests |
| Scan integration | `src/scan/` | Discovers `swiftbeaver`, checks version compatibility, builds CLI arguments, spawns the subprocess, parses progress/log output, and handles cancellation |
| Result loading and UI | `src/metadata/`, `src/ui/` | Reads SwiftBeaver metadata and renders configuration, progress, and result views |

The GUI never reimplements carving, parsing, hashing, validation, or metadata
generation. Those responsibilities stay inside the SwiftBeaver engine.

## SwiftBeaver Integration

SwiftBeaverLodge requires the unified `swiftbeaver` CLI introduced in
SwiftBeaver v0.5.1. Discovery is centralized in `src/scan/mod.rs` and checks
these locations in order:

1. `<lodge_exe_dir>/bin/swiftbeaver` (or `swiftbeaver.exe` on Windows)
2. `./bin/swiftbeaver` (or `swiftbeaver.exe` on Windows)
3. `swiftbeaver` (or `swiftbeaver.exe`) on `PATH`

The discovered binary is probed with `swiftbeaver --version`; versions older
than v0.5.1 are rejected with an actionable compatibility message. GPU
acceleration is not selected by changing binary names. Lodge passes `--gpu`
when the GUI GPU option is enabled.

Tagged SwiftBeaverLodge releases publish bundled Linux archives whose layout
matches that discovery order: `swiftbeaverlodge` at the archive root and
`bin/swiftbeaver` beside it. Windows bundles should use `swiftbeaverlodge.exe`
and `bin/swiftbeaver.exe` once compatible upstream SwiftBeaver Windows release
assets exist.

The subprocess boundary is intentional:

- CLI arguments are built as a vector, never as a shell string.
- `eframe::App::update` stays non-blocking.
- stdout and stderr are drained on background threads.
- SwiftBeaver JSON logs are parsed defensively, with malformed/plain text lines
  surfaced as log entries instead of crashing the UI.
- The run output path is taken from SwiftBeaver's `starting` JSON log event, not
  guessed from the configured output directory.

## Configuration Surface

The GUI models common SwiftBeaver v0.5.1 options in `ScanConfig`, including:

- evidence file or raw device input;
- output directory;
- file-type filters aligned with SwiftBeaver `--types` identifiers;
- string scanning controls;
- metadata backend selection (`parquet`, `jsonl`, `csv`);
- optional `--config-path` for expert SwiftBeaver YAML settings;
- GPU, entropy, resource limit, worker, chunking, checkpoint/resume, dry-run,
  metadata-only, validation, hashing, and dedupe controls.

SwiftBeaver YAML files are delegated to SwiftBeaver. Lodge validates the path
and then lets CLI arguments generated from GUI fields override matching YAML
settings.

## Metadata Backends

Lodge reads metadata from the run directory reported by SwiftBeaver:

| Backend | Detection | Support |
|---------|-----------|---------|
| Parquet | `parquet/` exists | Default backend; reads carved files, run summary, and optional artefact tables |
| JSONL | `metadata/carved_files.jsonl` exists | Supported fallback; supports live refresh of carved files and string artefacts |
| CSV | `metadata/carved_files.csv` exists | Supported for carved-file metadata in current builds, after issue #7 |

Parquet takes precedence when multiple backend markers are present, followed by
JSONL and then CSV. Metadata values and carved paths are treated as untrusted UI
data.

## Results and Live Refresh

The Results panel renders these views when data is available:

- Overview metrics and SwiftBeaver run summary values.
- Carved files with filtering, search, selection, hashes, validation,
  truncation, duplicate, MIME, and dimension fields.
- String artefacts.
- Browser history, cookies, and downloads.
- Windows artefacts.
- Entropy regions.

During a running scan, live refresh polls the reported run directory every two
seconds when enabled. Live refresh is intentionally limited to JSONL-backed
Overview, Files, and Strings snapshots because complete JSONL rows can be read
while SwiftBeaver appends to the file. If the user is viewing a tab that cannot
refresh live, Lodge moves back to Files for the active run and reloads the full
metadata after the scan exits.

## Build and Validation

The application is a standard Cargo project. Typical local checks are:

```bash
cargo fmt
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

The main test suite must not require a downloaded SwiftBeaver binary. Discovery
tests may report whether one is available, but they should not fail solely
because the engine is absent.

## Historical Plan Status

The previous implementation plan referenced Tauri, Svelte, Vite, Tailwind,
Skeleton UI, `src-tauri`, direct `swiftbeaver` crate integration, and
SwiftBeaver 0.2.x assumptions. Those references are obsolete and should not be
used for new work.

The current project does not include:

- a Tauri backend or IPC command layer;
- a Svelte or TypeScript frontend;
- npm-based development or packaging commands;
- direct library callbacks from SwiftBeaver;
- separate Lodge-side binary selection for `swiftbeaver-cpu-only`,
  `swiftbeaver-opencl`, or `swiftbeaver-cuda`.

If future work revisits any of those ideas, it should be proposed as a new
architecture change rather than inferred from the old draft.
