# SwiftBeaverLodge - Implementation Document

> GUI Frontend for SwiftBeaver Forensic File Carver

**Version:** 1.0.0-draft  
**Date:** January 2, 2026  
**Status:** Historical planning document

> Historical note: this document predates the current SwiftBeaverLodge
> architecture. The implemented app is a Rust egui desktop frontend that invokes
> the unified `swiftbeaver` v0.5.1+ CLI through `std::process::Command`; it does
> not embed SwiftBeaver as a library crate and does not use Tauri/Svelte. Use
> `AGENTS.md`, `README.md`, and the current `src/` tree as the source of truth
> for active implementation work.

---

## 1. Project Overview

### 1.1 Purpose
SwiftBeaverLodge is a desktop GUI application that provides a user-friendly interface for the SwiftBeaver forensic file carver. It enables forensic experts to configure, execute, and analyze file carving operations with real-time monitoring and comprehensive results visualization.

### 1.2 Target Users
- Digital forensic investigators
- Law enforcement forensic analysts
- Incident response teams
- Data recovery specialists

### 1.3 Key Requirements
| Requirement | Specification |
|-------------|---------------|
| Target Platforms | Linux (primary), Windows (secondary) |
| User Level | Forensic experts (feature-rich UI) |
| Network | Fully offline/airgapped capable |
| Concurrency | Single scan at a time |
| User Model | Single user, no authentication |
| Integration | Historical plan: SwiftBeaver bundled as library crate. Current app: external `swiftbeaver` v0.5.1+ subprocess. |

---

## 2. Technology Stack

### 2.1 Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    SwiftBeaverLodge                         │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Frontend (Svelte 5)                     │   │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌───────────┐  │   │
│  │  │ Config  │ │ Monitor │ │ Results │ │  Reports  │  │   │
│  │  │  Panel  │ │  View   │ │ Browser │ │ Generator │  │   │
│  │  └────┬────┘ └────┬────┘ └────┬────┘ └─────┬─────┘  │   │
│  └───────┼──────────┼──────────┼─────────────┼─────────┘   │
│          │          │          │             │              │
│          └──────────┴──────────┴─────────────┘              │
│                          │ Tauri IPC                        │
│  ┌───────────────────────┴─────────────────────────────┐   │
│  │              Backend (Rust/Tauri)                    │   │
│  │  ┌─────────────┐ ┌─────────────┐ ┌───────────────┐  │   │
│  │  │   Scan      │ │   Event     │ │    File       │  │   │
│  │  │  Manager    │ │   Stream    │ │   Watcher     │  │   │
│  │  └──────┬──────┘ └──────┬──────┘ └───────┬───────┘  │   │
│  └─────────┼───────────────┼────────────────┼──────────┘   │
│            │               │                │               │
│  ┌─────────┴───────────────┴────────────────┴──────────┐   │
│  │           SwiftBeaver (Rust Crate)                   │   │
│  │  • File carving engine                               │   │
│  │  • Signature scanning (CPU/GPU)                      │   │
│  │  • String extraction                                 │   │
│  │  • SQLite browser history recovery                   │   │
│  │  • Metadata generation                               │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 Technology Choices

| Layer | Technology | Version | Rationale |
|-------|------------|---------|-----------|
| Framework | Tauri | 2.x | Native cross-platform, Rust backend, small bundle |
| Frontend | Svelte | 5.x | Fast development, reactive, minimal boilerplate |
| Styling | TailwindCSS | 4.x | Utility-first, dark mode support |
| UI Components | Skeleton UI | 3.x | Svelte-native components, professional look |
| Backend | Rust | 1.75+ | Direct SwiftBeaver integration |
| Carver | SwiftBeaver | 0.2.x | Core forensic engine (crate dependency) |
| Build | Vite | 6.x | Fast HMR, optimized builds |

### 2.3 SwiftBeaver Integration Strategy

SwiftBeaver will be included as a **Cargo workspace member** or **git dependency**:

```toml
# src-tauri/Cargo.toml
[dependencies]
swiftbeaver = { git = "https://github.com/gaestu/SwiftBeaver", branch = "main" }
# Or as path during development:
# swiftbeaver = { path = "../../SwiftBeaver" }
```

This approach:
- ✅ Bundles SwiftBeaver into the final binary
- ✅ No separate installation required
- ✅ Direct API access (no CLI parsing)
- ✅ Allows independent SwiftBeaver development
- ✅ Real-time progress callbacks via Rust channels

---

## 3. Feature Specification

### 3.1 Core Features

#### 3.1.1 Input Configuration
| Feature | Description | Priority |
|---------|-------------|----------|
| Image Selection | Browse for .dd, .E01, raw images | P0 |
| Block Device Selection | List and select /dev/sdX (Linux) | P0 |
| Output Directory | Select carving output location | P0 |
| Config File Management | Load/save/edit YAML configs | P1 |
| Recent Files | Quick access to recent images | P2 |

#### 3.1.2 Scan Options
| Feature | Description | Priority |
|---------|-------------|----------|
| File Type Selection | Checkbox grid for all supported types | P0 |
| String Scanning | Toggle URLs, emails, phones, UTF-16 | P0 |
| GPU Acceleration | Enable/disable with backend detection | P1 |
| Entropy Analysis | Configure window size, threshold | P1 |
| Resource Limits | Max bytes, chunks, files, memory | P1 |
| Chunk Settings | Overlap bytes configuration | P2 |

#### 3.1.3 Real-time Monitoring
| Feature | Description | Priority |
|---------|-------------|----------|
| Progress Bar | Bytes/chunks processed percentage | P0 |
| Live Statistics | Files carved, by type counters | P0 |
| Speed Metrics | MB/s throughput, ETA | P0 |
| Live Log Viewer | Scrolling log with level filtering | P0 |
| File Preview | Thumbnail/preview of last carved files | P1 |
| Resource Monitor | CPU/Memory/GPU utilization | P2 |

#### 3.1.4 Results Browser
| Feature | Description | Priority |
|---------|-------------|----------|
| File Tree | Browse carved files by type | P0 |
| Thumbnail Grid | Visual grid for images | P0 |
| Metadata Table | Sortable/filterable v0.5.1 results from Parquet `parquet/files_*.parquet` and JSONL `metadata/carved_files.jsonl`, including hashes, validation, truncation, duplicate, and error fields | P0 |
| File Preview | In-app preview for images, PDFs, text | P1 |
| Hex Viewer | Raw bytes viewer for any file | P1 |
| String Artefacts | Browse URLs, emails, phones | P0 |
| Browser History | Table view of recovered history | P0 |
| Browser Cookies | Table view of recovered cookies | P1 |
| Export Selection | Export selected files to new location | P1 |

#### 3.1.5 Reporting
| Feature | Description | Priority |
|---------|-------------|----------|
| Run Summary | Display run_summary.jsonl data | P0 |
| HTML Report | Generate standalone HTML report | P1 |
| PDF Report | Generate PDF with chain of custody | P2 |
| Evidence Hash | Display/verify SHA-256 of source | P0 |
| Audit Log | Timestamped log of all operations | P1 |

### 3.2 UI Layout

```
┌──────────────────────────────────────────────────────────────────────┐
│  SwiftBeaverLodge                                    [―][□][✕]       │
├──────────────────────────────────────────────────────────────────────┤
│  [Dashboard] [Configure] [Monitor] [Results] [Reports]    ⚙ Settings │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌─── Dashboard ───────────────────────────────────────────────────┐ │
│  │                                                                 │ │
│  │  ╔═══════════════════════════════════════════════════════════╗ │ │
│  │  ║  Status: Ready                         GPU: CUDA (RTX 4090)║ │ │
│  │  ╚═══════════════════════════════════════════════════════════╝ │ │
│  │                                                                 │ │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │ │
│  │  │  Quick Start    │  │  Recent Scans   │  │  System Info    │ │ │
│  │  │                 │  │                 │  │                 │ │ │
│  │  │ [New Scan]      │  │ • image1.dd     │  │ OS: Linux 6.x   │ │ │
│  │  │ [Open Results]  │  │ • evidence.E01  │  │ Cores: 16       │ │ │
│  │  │ [Load Config]   │  │ • disk.raw      │  │ RAM: 64 GB      │ │ │
│  │  │                 │  │                 │  │ GPU: Available  │ │ │
│  │  └─────────────────┘  └─────────────────┘  └─────────────────┘ │ │
│  │                                                                 │ │
│  └─────────────────────────────────────────────────────────────────┘ │
│                                                                      │
├──────────────────────────────────────────────────────────────────────┤
│  SwiftBeaver v0.5.1+ | SwiftBeaverLodge v1.0.0 | Ready              │
└──────────────────────────────────────────────────────────────────────┘
```

### 3.3 Configuration Panel

```
┌─── Configure Scan ─────────────────────────────────────────────────────┐
│                                                                        │
│  INPUT                                                                 │
│  ┌────────────────────────────────────────────────────────┐           │
│  │ /path/to/evidence.E01                          [Browse]│           │
│  └────────────────────────────────────────────────────────┘           │
│  ☑ Compute evidence SHA-256 before scanning                           │
│                                                                        │
│  OUTPUT                                                                │
│  ┌────────────────────────────────────────────────────────┐           │
│  │ /path/to/output                                [Browse]│           │
│  └────────────────────────────────────────────────────────┘           │
│                                                                        │
│  ─────────────────────────────────────────────────────────────────    │
│                                                                        │
│  FILE TYPES                                                            │
│  ┌──────────────────────────────────────────────────────────────┐     │
│  │ ☑ JPEG  ☑ PNG   ☑ GIF   ☑ BMP   ☑ TIFF  ☑ WEBP            │     │
│  │ ☑ PDF   ☑ ZIP   ☑ DOCX  ☑ XLSX  ☑ PPTX  ☐ RAR   ☐ 7z      │     │
│  │ ☑ SQLite ☑ MP4                                              │     │
│  │                                          [Select All] [Clear]│     │
│  └──────────────────────────────────────────────────────────────┘     │
│                                                                        │
│  STRING SCANNING                                                       │
│  ☑ Enable string scanning                                              │
│    ☑ URLs  ☑ Emails  ☑ Phone numbers                                  │
│    ☐ UTF-16 LE/BE                                                      │
│    Min length: [8    ]  Max length: [4096  ]                          │
│                                                                        │
│  ADVANCED                                        [▼ Show Advanced]    │
│  ┌──────────────────────────────────────────────────────────────┐     │
│  │ ☐ GPU Acceleration    Backend: [OpenCL ▼]                    │     │
│  │ ☐ Entropy scanning    Window: [256  ] bytes  Threshold: [7.5]│     │
│  │ ☐ SQLite page recovery                                        │     │
│  │                                                                │     │
│  │ LIMITS                                                         │     │
│  │ Max bytes:  [         ] (empty = unlimited)                   │     │
│  │ Max files:  [         ] (empty = unlimited)                   │     │
│  │ Max memory: [         ] MiB                                   │     │
│  │                                                                │     │
│  │ Overlap KiB: [4    ]     Metadata: [JSONL ▼]                  │     │
│  └──────────────────────────────────────────────────────────────┘     │
│                                                                        │
│               [Save Config]  [Load Config]  [▶ Start Scan]            │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

### 3.4 Monitor View

```
┌─── Scan Progress ──────────────────────────────────────────────────────┐
│                                                                        │
│  Evidence: /path/to/evidence.E01                                       │
│  SHA-256:  a1b2c3d4e5f6...                                            │
│  Run ID:   20260102_143052_abc123                                      │
│                                                                        │
│  ═══════════════════════════════════════════════════════════════════  │
│  ████████████████████████████░░░░░░░░░░░░░░  62.4%                    │
│  ═══════════════════════════════════════════════════════════════════  │
│                                                                        │
│  Processed: 158.4 GB / 253.7 GB    Speed: 1.24 GB/s    ETA: 1m 17s   │
│                                                                        │
│  ┌─────────────────────────────┐  ┌─────────────────────────────────┐ │
│  │  CARVED FILES               │  │  LAST CARVED                    │ │
│  │                             │  │                                 │ │
│  │  JPEG      1,247            │  │  ┌─────────┐  ┌─────────┐      │ │
│  │  PNG         342            │  │  │  IMG    │  │  IMG    │      │ │
│  │  PDF          89            │  │  │  .jpg   │  │  .png   │      │ │
│  │  SQLite       23            │  │  └─────────┘  └─────────┘      │ │
│  │  DOCX         12            │  │  carved_0012847.jpg             │ │
│  │  ─────────────────          │  │  Offset: 0x2F4A8000             │ │
│  │  TOTAL     1,713            │  │  Size: 847 KB                   │ │
│  │                             │  │                                 │ │
│  └─────────────────────────────┘  └─────────────────────────────────┘ │
│                                                                        │
│  ┌─── Log Output ────────────────────────────────────────────────────┐ │
│  │ [INFO ] 14:32:15 Chunk 15234/24521 processed                      │ │
│  │ [INFO ] 14:32:15 Carved JPEG at offset 0x2F4A8000 (847 KB)       │ │
│  │ [INFO ] 14:32:16 Found URL: https://example.com/page             │ │
│  │ [WARN ] 14:32:16 Truncated file at offset 0x2F4B0000             │ │
│  │ [INFO ] 14:32:17 Carved PNG at offset 0x2F4C2000 (124 KB)        │ │
│  │                                                        [Auto-scroll ☑]│
│  └───────────────────────────────────────────────────────────────────┘ │
│                                                                        │
│                              [⏸ Pause]  [⏹ Stop]                       │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Project Structure

```
SwiftBeaverLodge/
├── docs/
│   ├── IMPLEMENTATION.md          # This document
│   ├── USER_GUIDE.md              # End-user documentation
│   └── DEVELOPER.md               # Development setup guide
│
├── src/                           # Svelte frontend source
│   ├── lib/
│   │   ├── components/
│   │   │   ├── common/
│   │   │   │   ├── FileSelector.svelte
│   │   │   │   ├── ProgressBar.svelte
│   │   │   │   ├── LogViewer.svelte
│   │   │   │   └── Thumbnail.svelte
│   │   │   ├── config/
│   │   │   │   ├── InputConfig.svelte
│   │   │   │   ├── FileTypeSelector.svelte
│   │   │   │   ├── StringOptions.svelte
│   │   │   │   └── AdvancedOptions.svelte
│   │   │   ├── monitor/
│   │   │   │   ├── ProgressPanel.svelte
│   │   │   │   ├── StatsPanel.svelte
│   │   │   │   ├── LivePreview.svelte
│   │   │   │   └── LogStream.svelte
│   │   │   ├── results/
│   │   │   │   ├── FileTree.svelte
│   │   │   │   ├── ThumbnailGrid.svelte
│   │   │   │   ├── MetadataTable.svelte
│   │   │   │   ├── HexViewer.svelte
│   │   │   │   ├── StringArtefacts.svelte
│   │   │   │   └── BrowserHistory.svelte
│   │   │   └── reports/
│   │   │       ├── RunSummary.svelte
│   │   │       ├── ReportGenerator.svelte
│   │   │       └── AuditLog.svelte
│   │   │
│   │   ├── stores/
│   │   │   ├── scan.ts            # Scan state management
│   │   │   ├── config.ts          # Configuration state
│   │   │   ├── results.ts         # Results data store
│   │   │   └── ui.ts              # UI state (theme, panels)
│   │   │
│   │   ├── api/
│   │   │   ├── tauri.ts           # Tauri command wrappers
│   │   │   ├── events.ts          # Event listeners
│   │   │   └── types.ts           # TypeScript interfaces
│   │   │
│   │   └── utils/
│   │       ├── formatters.ts      # Size, time formatters
│   │       ├── validators.ts      # Input validation
│   │       └── constants.ts       # App constants
│   │
│   ├── routes/
│   │   ├── +layout.svelte         # App shell layout
│   │   ├── +page.svelte           # Dashboard (home)
│   │   ├── configure/
│   │   │   └── +page.svelte       # Configuration view
│   │   ├── monitor/
│   │   │   └── +page.svelte       # Monitor view
│   │   ├── results/
│   │   │   └── +page.svelte       # Results browser
│   │   └── reports/
│   │       └── +page.svelte       # Reports view
│   │
│   ├── app.html                   # HTML template
│   ├── app.css                    # Global styles
│   └── app.d.ts                   # Type declarations
│
├── src-tauri/                     # Rust backend
│   ├── src/
│   │   ├── main.rs                # Tauri entry point
│   │   ├── lib.rs                 # Library exports
│   │   ├── commands/
│   │   │   ├── mod.rs
│   │   │   ├── scan.rs            # Scan control commands
│   │   │   ├── config.rs          # Config management
│   │   │   ├── files.rs           # File operations
│   │   │   └── system.rs          # System info commands
│   │   │
│   │   ├── scan/
│   │   │   ├── mod.rs
│   │   │   ├── manager.rs         # Scan lifecycle manager
│   │   │   ├── progress.rs        # Progress tracking
│   │   │   └── events.rs          # Event emission
│   │   │
│   │   ├── results/
│   │   │   ├── mod.rs
│   │   │   ├── parser.rs          # Historical JSONL/CSV parser plan; current app reads Parquet/JSONL/CSV in src/metadata/reader.rs
│   │   │   ├── thumbnails.rs      # Thumbnail generation
│   │   │   └── preview.rs         # File preview generation
│   │   │
│   │   └── utils/
│   │       ├── mod.rs
│   │       ├── fs.rs              # Filesystem utilities
│   │       └── system.rs          # System detection
│   │
│   ├── Cargo.toml                 # Rust dependencies
│   ├── tauri.conf.json            # Tauri configuration
│   ├── capabilities/              # Tauri v2 permissions
│   │   └── default.json
│   └── icons/                     # App icons
│
├── static/                        # Static assets
│   └── favicon.png
│
├── package.json                   # Node dependencies
├── svelte.config.js               # Svelte configuration
├── vite.config.ts                 # Vite configuration
├── tailwind.config.js             # Tailwind configuration
├── tsconfig.json                  # TypeScript config
├── README.md                      # Project README
└── LICENSE                        # License file
```

---

## 5. Tauri Backend API

### 5.1 Commands (Frontend → Backend)

```rust
// src-tauri/src/commands/scan.rs

/// Start a new carving scan
#[tauri::command]
async fn start_scan(
    config: ScanConfig,
    app: AppHandle,
) -> Result<ScanHandle, String>;

/// Pause the current scan
#[tauri::command]
async fn pause_scan(handle: ScanHandle) -> Result<(), String>;

/// Resume a paused scan
#[tauri::command]
async fn resume_scan(handle: ScanHandle) -> Result<(), String>;

/// Stop and cancel the current scan
#[tauri::command]
async fn stop_scan(handle: ScanHandle) -> Result<(), String>;

/// Get current scan status
#[tauri::command]
async fn get_scan_status(handle: ScanHandle) -> Result<ScanStatus, String>;
```

```rust
// src-tauri/src/commands/config.rs

/// Load configuration from YAML file
#[tauri::command]
async fn load_config(path: PathBuf) -> Result<ScanConfig, String>;

/// Save configuration to YAML file
#[tauri::command]
async fn save_config(config: ScanConfig, path: PathBuf) -> Result<(), String>;

/// Get default configuration
#[tauri::command]
fn get_default_config() -> ScanConfig;

/// Validate configuration
#[tauri::command]
fn validate_config(config: ScanConfig) -> Result<(), Vec<ValidationError>>;
```

```rust
// src-tauri/src/commands/files.rs

/// Browse for input file (opens native dialog)
#[tauri::command]
async fn browse_input_file() -> Result<Option<PathBuf>, String>;

/// Browse for output directory
#[tauri::command]
async fn browse_output_dir() -> Result<Option<PathBuf>, String>;

/// List available block devices (Linux only)
#[tauri::command]
async fn list_block_devices() -> Result<Vec<BlockDevice>, String>;

/// Read carved files metadata
#[tauri::command]
async fn read_carved_metadata(run_path: PathBuf) -> Result<Vec<CarvedFile>, String>;

/// Generate thumbnail for image file
#[tauri::command]
async fn generate_thumbnail(path: PathBuf) -> Result<String, String>; // Base64

/// Read file for hex preview
#[tauri::command]
async fn read_file_bytes(path: PathBuf, offset: u64, length: u64) -> Result<Vec<u8>, String>;
```

```rust
// src-tauri/src/commands/system.rs

/// Get system information
#[tauri::command]
fn get_system_info() -> SystemInfo;

/// Check GPU availability
#[tauri::command]
fn check_gpu_support() -> GpuInfo;

/// Get SwiftBeaver version
#[tauri::command]
fn get_swiftbeaver_version() -> String;
```

### 5.2 Events (Backend → Frontend)

```rust
// src-tauri/src/scan/events.rs

/// Emitted periodically during scan
#[derive(Clone, Serialize)]
struct ScanProgress {
    bytes_processed: u64,
    total_bytes: u64,
    chunks_processed: u64,
    total_chunks: u64,
    files_carved: HashMap<String, u64>,  // type -> count
    throughput_bps: f64,
    eta_seconds: Option<u64>,
}

/// Emitted when a file is carved
#[derive(Clone, Serialize)]
struct FileCarved {
    file_type: String,
    path: PathBuf,
    offset: u64,
    size: u64,
    sha256: String,
}

/// Emitted for log messages
#[derive(Clone, Serialize)]
struct LogMessage {
    level: String,      // "info", "warn", "error"
    timestamp: String,
    message: String,
}

/// Emitted when scan state changes
#[derive(Clone, Serialize)]
struct ScanStateChanged {
    state: ScanState,   // "running", "paused", "completed", "failed", "cancelled"
    message: Option<String>,
}

/// Emitted when string artefact found
#[derive(Clone, Serialize)]
struct ArtefactFound {
    artefact_type: String,  // "url", "email", "phone"
    value: String,
    offset: u64,
}
```

### 5.3 Data Types

```typescript
// src/lib/api/types.ts

interface ScanConfig {
  input_path: string;
  output_path: string;
  compute_evidence_hash: boolean;
  evidence_sha256?: string;
  
  // File types
  file_types: string[];  // ["jpeg", "png", "pdf", ...]
  disable_zip: boolean;
  
  // String scanning
  scan_strings: boolean;
  scan_urls: boolean;
  scan_emails: boolean;
  scan_phones: boolean;
  scan_utf16: boolean;
  string_min_len: number;
  string_max_len: number;
  
  // Advanced
  gpu_enabled: boolean;
  gpu_backend?: "opencl" | "cuda";
  scan_entropy: boolean;
  entropy_window_bytes: number;
  entropy_threshold: number;
  scan_sqlite_pages: boolean;
  
  // Limits
  max_bytes?: number;
  max_chunks?: number;
  max_files?: number;
  max_memory_mib?: number;
  
  // Output
  overlap_kib: number;
  metadata_backend: "jsonl" | "csv" | "parquet";
}

interface ScanStatus {
  state: "idle" | "running" | "paused" | "completed" | "failed" | "cancelled";
  run_id?: string;
  started_at?: string;
  progress?: ScanProgress;
  error?: string;
}

interface SystemInfo {
  os: string;
  os_version: string;
  cpu_cores: number;
  total_memory_bytes: number;
  available_memory_bytes: number;
}

interface GpuInfo {
  available: boolean;
  opencl_available: boolean;
  cuda_available: boolean;
  devices: GpuDevice[];
}

interface CarvedFile {
  id: string;
    run_id?: string;
  file_type: string;
  path: string;
    extension?: string;
    global_start: number;
    global_end: number;
  size: number;
    md5?: string;
    sha256?: string;
    validated: boolean;
    truncated: boolean;
    errors: string[];
    pattern_id?: string;
    is_duplicate: boolean;
    duplicate_of_offset?: number;
}
```

---

## 6. SwiftBeaver Integration

### 6.1 Cargo Dependency

```toml
# src-tauri/Cargo.toml

[package]
name = "swiftbeaverlodge"
version = "1.0.0"
edition = "2021"

[dependencies]
tauri = { version = "2", features = ["protocol-asset"] }
tauri-plugin-dialog = "2"
tauri-plugin-fs = "2"
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"
image = "0.25"  # For thumbnails

# SwiftBeaver as dependency
swiftbeaver = { git = "https://github.com/gaestu/SwiftBeaver", branch = "main" }
# During development, use local path:
# swiftbeaver = { path = "../../SwiftBeaver" }

[features]
default = ["ewf"]
ewf = ["swiftbeaver/ewf"]
gpu-opencl = ["swiftbeaver/gpu-opencl"]
gpu-cuda = ["swiftbeaver/gpu-cuda"]
```

### 6.2 Integration Pattern

```rust
// src-tauri/src/scan/manager.rs

use swiftbeaver::{Config, Scanner, ScanProgress as SwiftProgress};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tauri::{AppHandle, Emitter};

pub struct ScanManager {
    scanner: Option<Arc<Mutex<Scanner>>>,
    cancel_tx: Option<mpsc::Sender<()>>,
}

impl ScanManager {
    pub async fn start(
        &mut self,
        config: ScanConfig,
        app: AppHandle,
    ) -> Result<String, String> {
        // Convert GUI config to SwiftBeaver config
        let swift_config = self.build_swift_config(&config)?;
        
        // Create progress callback that emits Tauri events
        let app_clone = app.clone();
        let progress_callback = move |progress: SwiftProgress| {
            let _ = app_clone.emit("scan:progress", ScanProgress::from(progress));
        };
        
        // Create file carved callback
        let app_clone = app.clone();
        let file_callback = move |file: CarvedFileInfo| {
            let _ = app_clone.emit("scan:file_carved", FileCarved::from(file));
        };
        
        // Start scanner in background task
        let scanner = Scanner::new(swift_config)
            .with_progress_callback(progress_callback)
            .with_file_callback(file_callback);
        
        let (cancel_tx, cancel_rx) = mpsc::channel(1);
        self.cancel_tx = Some(cancel_tx);
        
        let scanner = Arc::new(Mutex::new(scanner));
        self.scanner = Some(scanner.clone());
        
        // Spawn scan task
        tokio::spawn(async move {
            let result = scanner.lock().await.run(cancel_rx).await;
            match result {
                Ok(summary) => {
                    app.emit("scan:completed", summary).ok();
                }
                Err(e) => {
                    app.emit("scan:failed", e.to_string()).ok();
                }
            }
        });
        
        Ok(run_id)
    }
    
    pub async fn stop(&mut self) -> Result<(), String> {
        if let Some(tx) = self.cancel_tx.take() {
            tx.send(()).await.map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
```

### 6.3 Required SwiftBeaver API (may need contribution)

For optimal GUI integration, SwiftBeaver should expose:

```rust
// Desired SwiftBeaver public API (may require PR to SwiftBeaver)

pub struct Scanner {
    // ...
}

impl Scanner {
    /// Create scanner with configuration
    pub fn new(config: Config) -> Self;
    
    /// Set progress callback (called periodically)
    pub fn with_progress_callback<F>(self, callback: F) -> Self
    where F: Fn(ScanProgress) + Send + 'static;
    
    /// Set file carved callback (called per file)
    pub fn with_file_callback<F>(self, callback: F) -> Self
    where F: Fn(CarvedFile) + Send + 'static;
    
    /// Run the scan (async, cancellable)
    pub async fn run(
        &mut self,
        cancel: mpsc::Receiver<()>,
    ) -> Result<RunSummary, Error>;
    
    /// Pause scanning
    pub fn pause(&mut self);
    
    /// Resume scanning
    pub fn resume(&mut self);
}

pub struct ScanProgress {
    pub bytes_processed: u64,
    pub total_bytes: u64,
    pub chunks_processed: u64,
    pub total_chunks: u64,
    pub files_carved: HashMap<String, u64>,
    pub throughput_bps: f64,
}
```

> **Note:** If SwiftBeaver doesn't expose this API, we have two options:
> 1. Contribute a PR to add library API to SwiftBeaver
> 2. Fall back to CLI wrapper (spawn process, parse stdout)

---

## 7. Development Phases

### Phase 1: Foundation (Week 1-2)
- [ ] Initialize Tauri + Svelte project
- [ ] Set up project structure
- [ ] Integrate SwiftBeaver crate
- [ ] Implement basic file dialogs
- [ ] Create configuration form UI
- [ ] Implement config save/load

### Phase 2: Core Scanning (Week 3-4)
- [ ] Implement scan manager
- [ ] Create progress event system
- [ ] Build progress UI with live stats
- [ ] Implement log streaming
- [ ] Add pause/resume/stop controls
- [ ] Test with various image formats

### Phase 3: Results Browser (Week 5-6)
- [ ] Historical plan: implement JSONL/CSV parser. Current app reads Parquet `files_*.parquet`, JSONL `carved_files.jsonl`, and CSV via `src/metadata/reader.rs`.
- [ ] Build file tree component
- [ ] Create thumbnail generation
- [ ] Build metadata table with sorting/filtering
- [ ] Implement hex viewer
- [ ] Create string artefacts viewer
- [ ] Build browser history viewer

### Phase 4: Advanced Features (Week 7-8)
- [ ] Implement live file preview during scan
- [ ] Add GPU detection and configuration
- [ ] Build entropy visualization
- [ ] Create run summary display
- [ ] Implement HTML report generation
- [ ] Add audit logging

### Phase 5: Polish & Testing (Week 9-10)
- [ ] Windows build and testing
- [ ] Performance optimization
- [ ] Error handling improvements
- [ ] UI/UX refinements
- [ ] Documentation
- [ ] Packaging and distribution

---

## 8. Build & Distribution

### 8.1 Development

```bash
# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Run with GPU features
npm run tauri dev -- --features gpu-opencl
```

### 8.2 Production Build

```bash
# Linux (AppImage, deb)
npm run tauri build

# Linux with GPU support
npm run tauri build -- --features gpu-opencl

# Windows (MSI, exe)
npm run tauri build -- --target x86_64-pc-windows-msvc
```

### 8.3 Distribution Artifacts

| Platform | Format | Size (est.) |
|----------|--------|-------------|
| Linux | AppImage | ~15 MB |
| Linux | .deb | ~12 MB |
| Windows | .msi | ~18 MB |
| Windows | .exe (NSIS) | ~15 MB |

### 8.4 System Requirements

| Requirement | Minimum | Recommended |
|-------------|---------|-------------|
| OS | Linux (glibc 2.31+), Windows 10 | Latest LTS |
| CPU | 4 cores | 8+ cores |
| RAM | 8 GB | 32 GB |
| Storage | 500 MB + output space | SSD recommended |
| GPU (optional) | OpenCL 1.2 / CUDA 11 | RTX 3000+ / RX 6000+ |

---

## 9. Security Considerations

### 9.1 Forensic Integrity
- Read-only access to evidence files
- SHA-256 verification before/after scan
- Audit log of all operations
- No network access (offline capable)
- Chain of custody documentation in reports

### 9.2 Application Security
- No external API calls
- Local file access only (user-selected paths)
- No telemetry or analytics
- Sandboxed via Tauri capabilities

### 9.3 Tauri Capabilities

```json
// src-tauri/capabilities/default.json
{
  "identifier": "default",
  "description": "Default capabilities for SwiftBeaverLodge",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "dialog:default",
    "fs:default",
    "fs:allow-read",
    "fs:allow-write"
  ]
}
```

---

## 10. Future Enhancements (v2.0+)

| Feature | Description |
|---------|-------------|
| Multi-scan queue | Queue multiple images for batch processing |
| Remote scanning | Connect to remote SwiftBeaver instances |
| Plugin system | Custom file type carvers |
| Timeline view | Visual timeline of carved file creation dates |
| Network artefacts | PCAP carving support |
| Machine learning | ML-based file classification |
| Case management | Organize multiple evidence sources |
| Collaboration | Export/import case files |

---

## 11. Open Questions

1. **SwiftBeaver Library API**: Does SwiftBeaver expose a library API with callbacks, or only CLI? May need to contribute PR.

2. **E01 on Windows**: libewf availability on Windows - may need to bundle or provide instructions.

3. **GPU on Windows**: OpenCL/CUDA driver requirements for Windows users.

4. **Licensing**: Confirm Apache 2.0 license compatibility for GUI distribution.

---

## Appendix A: References

- [Tauri Documentation](https://tauri.app/v2/)
- [Svelte Documentation](https://svelte.dev/)
- [SwiftBeaver Repository](https://github.com/gaestu/SwiftBeaver)
- [Skeleton UI](https://www.skeleton.dev/)
- [TailwindCSS](https://tailwindcss.com/)

---

*Document Version: 1.0.0-draft*  
*Last Updated: January 2, 2026*
