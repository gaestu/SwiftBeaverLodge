# SwiftBeaverLodge

🦫 **GUI Frontend for SwiftBeaver Forensic File Carver**

A pure Rust desktop application for forensic file recovery using [egui](https://github.com/emilk/egui). Provides a user-friendly interface for the [SwiftBeaver](https://github.com/gaestu/SwiftBeaver) file carving engine.

![License](https://img.shields.io/badge/license-Apache-2.0-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)

## Features

- 📂 **Multi-format evidence support** - Raw images (.dd, .raw, .img), E01 format, and block devices
- � **Raw device scanning** - Direct access to /dev/sdX, /dev/nvmeXnY devices with dropdown selection
- 🔍 **Comprehensive file recovery** - Images, documents, archives, databases, media, and more
- 🔗 **String/URL/Email extraction** - Find text patterns in evidence
- 📊 **Real-time progress monitoring** - Live throughput, ETA, and statistics
- 📁 **Result browsing** - Filter and search carved files
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

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/SwiftBeaverLodge.git
cd SwiftBeaverLodge

# Install SwiftBeaver v0.6.7+ so `swiftbeaver` is on PATH
# (e.g. via your distribution's package manager or the upstream installer)
swiftbeaver --version   # should report >= 0.6.7

# Build
cargo build --release

# Run
./target/release/swiftbeaverlodge
```

## SwiftBeaver discovery

SwiftBeaverLodge requires `swiftbeaver` v0.6.7 or newer. It looks for the
binary in this order:

1. `<exe_dir>/bin/swiftbeaver` (alongside the Lodge binary)
2. `./bin/swiftbeaver` (current working directory)
3. `swiftbeaver` on `PATH`

The status bar shows the detected version. GPU acceleration is toggled via
the `--gpu` flag.

| Binary Name | Notes |
|-------------|-------|
| `swiftbeaver` | v0.6.7+; GPU enabled via `--gpu` |

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
3. **Monitor** - Watch real-time progress and logs
4. **Browse Results** - View carved files and metadata

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
│   │   ├── reader.rs     # Parquet/JSONL reader
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
| Evidence Hash | Compute SHA-256 of evidence | enabled |

### Supported file type categories

The GUI exposes every carver shipped with SwiftBeaver v0.6.7, organised into
the following categories (selectable individually or via the preset buttons
**Select All / None / Images / Documents / Media / Windows Artefacts**):

| Category | Types |
|----------|-------|
| Images | `jpeg`, `png`, `gif`, `webp`, `bmp`, `tiff`, `heic`, `ico` |
| Documents | `pdf`, `doc`, `docx`, `xls`, `xlsx`, `ppt`, `pptx`, `odt`, `ods`, `odp`, `rtf`, `eml` |
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
> not the file extensions `gz` / `bz2`.

## Tests

```bash
# Run all tests (69 total)
cargo test

# Run with output
cargo test -- --nocapture
```

## Tech Stack

- **[egui](https://github.com/emilk/egui)** - Immediate-mode GUI (pure Rust)
- **[eframe](https://github.com/emilk/egui/tree/master/crates/eframe)** - egui framework
- **[parquet](https://crates.io/crates/parquet) + [arrow](https://crates.io/crates/arrow)** - Metadata reading
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
