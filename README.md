# SwiftBeaverLodge

🦫 **GUI Frontend for SwiftBeaver Forensic File Carver**

A pure Rust desktop application for forensic file recovery using [egui](https://github.com/emilk/egui). Provides a user-friendly interface for the [SwiftBeaver](https://github.com/gaestu/SwiftBeaver) file carving engine.

![License](https://img.shields.io/badge/license-Apache-2.0-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)

## Features

- 📂 **Multi-format evidence support** - Raw images (.dd, .raw, .img), E01 format, and block devices
- **Raw device scanning** - Direct access to /dev/sdX, /dev/nvmeXnY devices with dropdown selection
- 🔍 **Comprehensive file recovery** - Images, documents, archives, databases, media, and more
- 🔗 **String/URL/Email extraction** - Find text patterns in evidence
- 📊 **Real-time progress monitoring** - Live throughput, ETA, statistics, and newly carved file metadata
- 📁 **Result browsing** - Filter and search carved files plus SwiftBeaver v0.5.1 run summaries, string artefacts, browser artefacts, Windows artefacts, and entropy regions from Parquet, JSONL, and CSV metadata
- 💾 **Offline/Airgapped operation** - No network required for forensic integrity
- 🚀 **GPU acceleration** - Optional OpenCL/CUDA support via the `--gpu` flag
- 🎯 **Single binary** - No npm, no web stack, just `cargo build`

## Quick Start

### Prerequisites

1. **Rust 1.70+** - Install from [rustup.rs](https://rustup.rs)
2. **SwiftBeaver v0.6.7+** - Install the unified `swiftbeaver` CLI on your `PATH`

### Linux System Dependencies

**Fedora:**
```bash
sudo dnf install gtk3-devel glib2-devel pango-devel atk-devel \
                 gdk-pixbuf2-devel cairo-devel cairo-gobject-devel \
                 libxkbcommon-devel wayland-devel
```

**Ubuntu/Debian:**
```bash
sudo apt install libgtk-3-dev libglib2.0-dev libpango1.0-dev \
                 libatk1.0-dev libgdk-pixbuf2.0-dev libcairo2-dev \
                 libxkbcommon-dev libwayland-dev
```

### Release Downloads

Tagged SwiftBeaverLodge releases publish a bundled Linux x86_64 archive:

```bash
# Download install.sh from a SwiftBeaverLodge release or run it from a checkout.
# Installs to /usr/local/bin when writable, otherwise $HOME/.local/bin
./install.sh
```

The installer downloads the bundled release, installs `swiftbeaverlodge` and
`bin/swiftbeaver`, verifies both binaries, and preserves the app's normal
SwiftBeaver discovery path. Set `LODGE_VERSION=v0.1.0` to install a specific
release, or `INSTALL_DIR=/custom/bin` to choose the install location. Advanced
users can replace the bundled engine during install with
`SWIFTBEAVER_VERSION=v0.5.1 SWIFTBEAVER_FLAVOR=opencl ./install.sh`.
Set `EXPOSE_SWIFTBEAVER=1` to also expose `swiftbeaver` directly in
`INSTALL_DIR`; existing files at that path are left untouched.

To install manually instead:

```bash
tar -xzf swiftbeaverlodge-linux-x86_64.tar.gz
cd swiftbeaverlodge-linux-x86_64
./swiftbeaverlodge
```

Run `swiftbeaverlodge --version` to print the installed Lodge build version.

The archive contains both the GUI and a compatible CPU-only SwiftBeaver engine:

```text
swiftbeaverlodge
bin/swiftbeaver
install.sh
README.md
LICENSE
BUNDLED_SWIFTBEAVER.txt
```

Release assets include `checksums.txt` with SHA-256 checksums. Windows bundled
archives are planned after compatible upstream SwiftBeaver Windows release
assets are available. Until then, Windows users should build from source and
place a compatible `swiftbeaver.exe` in `bin/` next to `swiftbeaverlodge.exe`,
or install `swiftbeaver.exe` on `PATH`.

### Installation From Source

```bash
# Clone the repository
git clone https://github.com/gaestu/SwiftBeaverLodge.git
cd SwiftBeaverLodge

# Install SwiftBeaver v0.6.7+ so `swiftbeaver` is on PATH
# (e.g. via your distribution's package manager or the upstream installer)
swiftbeaver --version   # should report >= 0.6.7

# Build
cargo build --release

# Run
./target/release/swiftbeaverlodge
```

If you do not want to install SwiftBeaver system-wide, the repository includes
a convenience helper:

```bash
# Installs one upstream release package as ./bin/swiftbeaver
./download-swiftbeaver.sh cpu-only

# Then run Lodge from this checkout
cargo run --bin swiftbeaverlodge
```

The upstream SwiftBeaver release artifacts are packaged by build flavor
(`cpu-only`, `opencl`, or `cuda`), but SwiftBeaverLodge always uses a single
binary path named `swiftbeaver`. GPU use is controlled by the GUI's GPU toggle,
which adds SwiftBeaver's `--gpu` flag.

## SwiftBeaver discovery

SwiftBeaverLodge requires `swiftbeaver` v0.6.7 or newer. It looks for the
binary in this order:

1. `<exe_dir>/bin/swiftbeaver` (or `swiftbeaver.exe` on Windows) alongside the Lodge binary
2. `./bin/swiftbeaver` (or `swiftbeaver.exe` on Windows) in the current working directory
3. `swiftbeaver` (or `swiftbeaver.exe`) on `PATH`

The status bar shows the detected version. GPU acceleration is toggled via
the `--gpu` flag.

| Binary Name | Notes |
|-------------|-------|
| `swiftbeaver` | v0.6.7+; GPU enabled via `--gpu` |

Older `swiftbeaver-cpu-only`, `swiftbeaver-opencl`, and `swiftbeaver-cuda`
binary selection workflows are historical. Current Lodge builds do not expose a
binary-flavor selector and do not look for those names.

## Screenshot

```
┌──────────────────────────────────────────────────────────────┐
│  SwiftBeaverLodge                                            │
├────────────────┬─────────────────────────────────────────────┤
│                │                                             │
│  ⚙ Configure   │  Scan Configuration                        │
│  📊 Monitor    │  ┌──────────────────────────────────────┐  │
│  📁 Results    │  │ Source: [📄 File] [💾 Device]        │  │
│                │  │ Evidence: [/dev/sda ▼] or Browse...  │  │
│  ▶ Start Scan  │  │ Output:   [/output/carved]           │  │
│                │  │                                      │  │
│  ● Idle        │  │ File Types: ☑ jpeg ☑ png ☑ pdf...    │  │
│                │  └──────────────────────────────────────┘  │
├────────────────┴─────────────────────────────────────────────┤
│  Ready                                  swiftbeaver: 0.6.7   │
└──────────────────────────────────────────────────────────────┘
```

## Usage

1. **Configure** - Select evidence source (file or raw device), output directory, and file types
2. **Start Scan** - Click "▶ Start Scan" to begin carving
3. **Monitor** - Watch real-time progress, logs, and newly carved files as metadata is written
4. **Browse Results** - View carved files, run metrics, and artefact metadata

Live result refresh uses SwiftBeaver's reported run output directory and can be disabled from the Monitor or Results view if a long scan's result table becomes noisy.

### Metadata and results

SwiftBeaverLodge supports the metadata backends exposed by SwiftBeaver v0.5.1:

| Backend | Lodge support |
|---------|---------------|
| Parquet | Default backend; reads `parquet/files_*.parquet` and optional artefact tables |
| JSONL | Supported fallback; enables live refresh while SwiftBeaver appends complete JSONL rows |
| CSV | Supported for carved-file metadata in current builds (added after issue #7) |

The Results view includes tabs for overview metrics, carved files, text
artefacts, browser history/cookies/downloads, Windows artefacts, and entropy
regions when the corresponding SwiftBeaver metadata tables exist. Text
artefacts can be browsed by category inside the tab, including URLs, emails,
phones, BitLocker recovery passwords, and other legacy/general strings. During
an active scan, live refresh is limited to Overview, Files, and Text Artefacts
because JSONL can be read safely while it is being appended; Parquet and CSV
are loaded after SwiftBeaver finalizes the run.

### Raw Device Scanning

To scan raw devices (requires root/sudo):

1. Select **💾 Raw Device** as the source type
2. Click **🔄 Refresh** to list available block devices
3. Select the device from the dropdown (shows size and model)
4. Optionally enable **Show partitions** to see individual partitions

**Note:** Raw device access requires elevated privileges. Run with `sudo` for device scanning.

## Project Structure

```
SwiftBeaverLodge/
├── src/
│   ├── main.rs           # Application entry point
│   ├── lib.rs            # Library exports
│   ├── app.rs            # Main application state
│   ├── config.rs         # Scan configuration types
│   ├── devices.rs        # Block device detection
│   ├── scan/             # Scan management
│   │   ├── mod.rs        # Module exports
│   │   ├── manager.rs    # Subprocess spawning
│   │   └── progress.rs   # Progress parsing
│   ├── metadata/         # Result reading
│   │   ├── mod.rs        # Module exports
│   │   ├── reader.rs     # Parquet/JSONL/CSV reader
│   │   └── types.rs      # Metadata types
│   └── ui/               # UI components
│       ├── mod.rs        # Tab enum, exports
│       ├── config_panel.rs
│       ├── progress_panel.rs
│       └── results_panel.rs
├── bin/
│   └── swiftbeaver            # Unified CLI (v0.6.7+)
├── tests/
│   └── integration_tests.rs
├── Cargo.toml
└── README.md
```

## Configuration Options

| Option | Description | Default |
|--------|-------------|---------|
| Evidence Source | File/Image or Raw Device | File |
| Evidence Path | Path to disk image or /dev/X device | - |
| Output Directory | Where to save carved files | - |
| File Types | Which file types to carve | Common types |
| Metadata Format | parquet/jsonl/csv | parquet |
| String Scanning | Extract URLs/emails/phones and BitLocker recovery passwords | enabled |
| GPU Acceleration | Use GPU for faster scanning (`--gpu`) | disabled |
| SwiftBeaver YAML Config | Pass an expert SwiftBeaver YAML file with `--config-path` | disabled |
| Evidence Hash | Compute SHA-256 of evidence | enabled |
| Chunking | Chunk size and optional overlap | 64 MiB, SwiftBeaver default overlap |
| Checkpoint / Resume | Write checkpoint state on early exit or resume from an existing checkpoint | disabled |
| Run Mode | Normal, dry-run, or metadata-only scanning | normal |
| Validation | Validate carved files and optionally remove invalid files | disabled |
| Hashing & Deduplication | Select `md5`/`sha256`, track duplicates, optionally skip duplicate file bodies | SwiftBeaver default hashes, dedupe disabled |

### SwiftBeaver YAML configs

Advanced options can pass an existing SwiftBeaver YAML configuration file to
the engine with `--config-path`. This is for expert SwiftBeaver settings that
Lodge does not model directly, such as QuickTime mode, EWF cache handles, GPU
device indexes, and per-carver limits.

- Select **Use --config-path** in the Configure tab, then enter a path or pick a
  `.yaml` / `.yml` file. Lodge reports missing or invalid config paths before
  launch.
- SwiftBeaver loads the YAML file first, then applies Lodge's generated CLI
  arguments. GUI-controlled fields such as input, output, metadata backend,
  selected file types, string scanning toggles, GPU enablement, resource limits,
  chunking, run mode, validation, hashing, dedupe, and checkpoint options
  intentionally override matching YAML values.
- YAML-only SwiftBeaver settings remain delegated to SwiftBeaver. Lodge does not
  parse, edit, or save the full SwiftBeaver YAML schema.
- Lodge presets, if added later, should remain a separate Lodge format rather
  than being conflated with SwiftBeaver YAML configs.

### Scan modes, validation, and dedupe

Advanced options expose SwiftBeaver v0.5.1's storage-management controls:

- **Dry run** (`--dry-run`) scans and counts without writing SwiftBeaver output files.
- **Metadata only** (`--metadata-only`) writes metadata records but skips carved file bodies; validation and invalid-file removal are disabled in this mode.
- **Validate carved files** (`--validate-carved`) records validation state; **Remove invalid files** (`--remove-invalid`) is only available when validation is enabled and carved file bodies are being written.
- **Hash algorithms** emits `--hash-algorithms` only when at least one algorithm is selected. Leaving the list empty lets SwiftBeaver use its default `md5,sha256` set.
- **Track duplicates** (`--dedupe`) requires SHA-256 hashing. If explicit hashes are selected in the GUI, `sha256` is kept enabled for dedupe.
- **Skip duplicate file bodies** (`--skip-duplicates`) is only available with dedupe when carved file bodies are being written; duplicate rows remain in metadata while duplicate file contents are not written again.

### Checkpoint and resume

Advanced options expose SwiftBeaver v0.5.1's long-scan checkpoint workflow:

- **Checkpoint path** emits `--checkpoint-path` and asks SwiftBeaver to write checkpoint state to that file when the scan exits early.
- **Resume from** emits `--resume-from` and makes the start action show **Resume Scan** before launch.
- **Chunk size** always emits `--chunk-size-mib` so each launched scan records explicit chunk geometry. **Overlap** emits `--overlap-kib` when enabled.
- Resumed scans must use the same chunk size and overlap as the scan that created the checkpoint. Lodge shows this warning in the Configure tab and rejects missing resume checkpoint files before launching SwiftBeaver.
- When stopping a running scan, Lodge requests a graceful SwiftBeaver shutdown and keeps draining process output so a configured checkpoint has time to be written.

### Supported file type categories

The GUI exposes every carver shipped with SwiftBeaver v0.6.7, organised into
the following categories (selectable individually or via the preset buttons
**Select All / None / Images / Documents / Media / Windows Artefacts**):

| Category | Types |
|----------|-------|
| Images | `jpeg`, `png`, `gif`, `webp`, `bmp`, `tiff`, `heic`, `ico` |
| Documents | `pdf`, `ole`, `doc`, `docx`, `xls`, `xlsx`, `ppt`, `pptx`, `odt`, `ods`, `odp`, `rtf`, `eml` |
| eBooks | `epub`, `mobi`, `fb2`, `lrf` |
| Archives | `zip`, `rar`, `7z`, `tar`, `gzip`, `bzip2`, `xz` |
| Databases | `sqlite`, `sqlite_wal`, `sqlite_page` |
| Media | `mp4`, `mov`, `avi`, `webm`, `wmv`, `mp3`, `wav`, `ogg` |
| Windows Artefacts | `lnk`, `prefetch`, `registry`, `evtx`, `bek` |
| Executables | `elf` |

**ZIP-derived classifications.** `docx`, `xlsx`, and `pptx` are ZIP-based
office formats; together with raw `zip`, they are skipped when SwiftBeaver
runs with **Disable ZIP carving** (`--disable-zip`). These entries are
marked with `*` in the file-type list and surface a tooltip in the GUI.
Other ZIP-structured formats such as `epub` and the OpenDocument types
(`odt`, `ods`, `odp`) have their own dedicated carvers in v0.6.7 and are
not affected by `--disable-zip`.

> Carver names match SwiftBeaver's `--types` / `--enable-types` identifiers
> exactly. Notably, gzip and bzip2 use their long names (`gzip`, `bzip2`),
> not the file extensions `gz` / `bz2`. `ole` enables generic OLE/CFB carving;
> SwiftBeaver classifies recognised Office OLE files as `doc`, `xls`, or `ppt`.

## Tests

```bash
# Run all tests (69 total)
cargo test

# Run with output
cargo test -- --nocapture

# Optional installed/bundled SwiftBeaver smoke test (requires v0.5.1+)
cargo test --test installed_swiftbeaver_smoke -- --ignored --nocapture
```

The optional smoke test runs a tiny temporary fixture through the real
`swiftbeaver` command discovered by Lodge, then loads metadata from the run
output path reported by SwiftBeaver. It is ignored by default so machines
without SwiftBeaver are not blocked by the normal test suite.

## Tech Stack

- **[egui](https://github.com/emilk/egui)** - Immediate-mode GUI (pure Rust)
- **[eframe](https://github.com/emilk/egui/tree/master/crates/eframe)** - egui framework
- **[parquet](https://crates.io/crates/parquet) + [arrow](https://crates.io/crates/arrow) + [csv](https://crates.io/crates/csv)** - Metadata reading
- **[tokio](https://tokio.rs/)** - Async runtime
- **[rfd](https://crates.io/crates/rfd)** - Native file dialogs

## License

Apache License 2.0 - see [LICENSE](LICENSE) for details.

## Acknowledgments

- [SwiftBeaver](https://github.com/gaestu/SwiftBeaver) - The core forensic file carver by gaestu
- [egui](https://github.com/emilk/egui) - Immediate-mode GUI library by emilk

## ⚠️ Forensic Note

**Important for forensic use:**

- This tool operates in **read-only mode** on evidence
- SHA-256 hashes are computed for verification
- All operations are logged for audit trails
- Evidence paths are never modified

For forensic cases, always work with verified copies of evidence and maintain proper chain of custody documentation.
