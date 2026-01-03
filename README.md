# SwiftBeaverLodge

🦫 **GUI Frontend for SwiftBeaver Forensic File Carver**

A pure Rust desktop application for forensic file recovery using [egui](https://github.com/emilk/egui). Provides a user-friendly interface for the [SwiftBeaver](https://github.com/gaestu/SwiftBeaver) file carving engine.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)

## Features

- 📂 **Multi-format evidence support** - Raw images (.dd, .raw, .img), E01 format, and block devices
- 🔍 **Comprehensive file recovery** - Images, documents, archives, databases, media, and more
- 🔗 **String/URL/Email extraction** - Find text patterns in evidence
- 📊 **Real-time progress monitoring** - Live throughput, ETA, and statistics
- 📁 **Result browsing** - Filter and search carved files
- 💾 **Offline/Airgapped operation** - No network required for forensic integrity
- 🚀 **GPU acceleration** - Optional OpenCL/CUDA support
- 🎯 **Single binary** - No npm, no web stack, just `cargo build`

## Quick Start

### Prerequisites

1. **Rust 1.70+** - Install from [rustup.rs](https://rustup.rs)
2. **fastcarve binary** - Download from [SwiftBeaver releases](https://github.com/gaestu/SwiftBeaver/releases)

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

# Download fastcarve binary (Linux)
./download-fastcarve.sh

# Build
cargo build --release

# Run
./target/release/swiftbeaverlodge
```

## Screenshot

```
┌──────────────────────────────────────────────────────────────┐
│  SwiftBeaverLodge                                            │
├────────────────┬─────────────────────────────────────────────┤
│                │                                             │
│  ⚙ Configure   │  Scan Configuration                        │
│  📊 Monitor    │  ┌──────────────────────────────────────┐  │
│  📁 Results    │  │ Evidence File: [/path/to/image.dd]   │  │
│                │  │ Output Dir:    [/output/carved]      │  │
│  ▶ Start Scan  │  │                                      │  │
│                │  │ File Types: ☑ jpeg ☑ png ☑ pdf...    │  │
│  ● Idle        │  │ Metadata:   ◉ Parquet ○ JSONL ○ CSV  │  │
│                │  └──────────────────────────────────────┘  │
├────────────────┴─────────────────────────────────────────────┤
│  Ready                                  fastcarve: v0.2.1    │
└──────────────────────────────────────────────────────────────┘
```

## Usage

1. **Configure** - Set evidence file, output directory, and file types
2. **Start Scan** - Click "▶ Start Scan" to begin carving
3. **Monitor** - Watch real-time progress and logs
4. **Browse Results** - View carved files and metadata

## Project Structure

```
SwiftBeaverLodge/
├── src/
│   ├── main.rs           # Application entry point
│   ├── lib.rs            # Library exports
│   ├── app.rs            # Main application state
│   ├── config.rs         # Scan configuration types
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
│   └── fastcarve         # Downloaded binary
├── tests/
│   └── integration_tests.rs
├── Cargo.toml
└── README.md
```

## Configuration Options

| Option | Description | Default |
|--------|-------------|---------|
| Evidence File | Path to disk image or device | - |
| Output Directory | Where to save carved files | - |
| File Types | Which file types to carve | Common types |
| Metadata Format | parquet/jsonl/csv | parquet |
| String Scanning | Extract URLs/emails/phones | enabled |
| GPU Acceleration | Use GPU for faster scanning | disabled |
| Evidence Hash | Compute SHA-256 of evidence | enabled |

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

MIT License - see [LICENSE](LICENSE) for details.

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
