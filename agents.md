# SwiftBeaverLodge - AI Agent Coding Guidelines

> Guidelines for AI coding agents (Copilot, Claude, etc.) working on this project.

---

## 1. Project Overview

**SwiftBeaverLodge** is a GUI frontend for the SwiftBeaver forensic file carver.

### Tech Stack
| Layer | Technology |
|-------|------------|
| GUI Framework | egui 0.29 + eframe |
| Language | Rust (100%) |
| Metadata | parquet + arrow |
| Async | tokio |
| File Dialogs | rfd |
| Core Engine | fastcarve binary (subprocess) |

### Project Structure
```
SwiftBeaverLodge/
├── src/
│   ├── main.rs           # Entry point, eframe setup
│   ├── lib.rs            # Library exports for tests
│   ├── app.rs            # Main application state & UI loop
│   ├── config.rs         # ScanConfig, MetadataBackend
│   ├── scan/
│   │   ├── mod.rs        # ScanState, LogEntry, exports
│   │   ├── manager.rs    # ScanManager - subprocess spawning
│   │   └── progress.rs   # ScanProgress, JSON parsing
│   ├── metadata/
│   │   ├── mod.rs        # detect_metadata_backend()
│   │   ├── reader.rs     # MetadataReader - Parquet/JSONL
│   │   └── types.rs      # CarvedFile, StringArtefact
│   └── ui/
│       ├── mod.rs        # Tab enum, panel exports
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

---

## 2. SwiftBeaver Integration

### Integration Method: Binary Subprocess

SwiftBeaverLodge uses the **pre-built fastcarve binary** from [GitHub releases](https://github.com/gaestu/SwiftBeaver/releases):

- **Stable releases** - Pinned to tested versions (v0.2.1)
- **Faster builds** - No libewf/GPU SDK required at build time
- **Cross-platform** - Pre-built binaries per platform

### Binary Location

```
SwiftBeaverLodge/
├── bin/
│   └── fastcarve          # Downloaded binary
└── download-fastcarve.sh  # Script to download binary
```

### Key Integration Points

1. **Subprocess Execution** - Spawn fastcarve with CLI arguments:
   ```rust
   Command::new(&binary_path)
       .args(&[
           "--input", &config.input_path,
           "--output", &config.output_path,
           "--log-format", "json",
           "--progress-interval-secs", "1",
           "--metadata-backend", "parquet",
       ])
       .stdout(Stdio::piped())
       .stderr(Stdio::piped())
       .spawn()
   ```

2. **Progress Parsing** - Parse JSON log output from stdout:
   ```rust
   // fastcarve with --log-format json emits:
   // {"timestamp":"...","level":"INFO","message":"progress bytes_scanned=1234 ..."}
   
   fn parse_json_log(line: &str) -> Option<(String, Value)>
   ```

3. **Cancellation** - Send SIGTERM to the subprocess:
   ```rust
   #[cfg(unix)]
   unsafe { libc::kill(pid as i32, libc::SIGTERM); }
   ```

4. **Metadata Reading** - Auto-detect Parquet (default) or JSONL:
   ```rust
   fn detect_metadata_backend(run_path: &Path) -> Option<&'static str>
   ```

### CLI Arguments Reference

| Option | Description |
|--------|-------------|
| `--input <path>` | Evidence file/device path |
| `--output <path>` | Output directory |
| `--log-format json` | Machine-readable JSON logs |
| `--progress-interval-secs N` | Progress updates every N seconds |
| `--metadata-backend parquet` | Output format (parquet/jsonl/csv) |
| `--types jpeg,png,sqlite` | File types to carve |
| `--scan-strings` | Enable string/URL/email scanning |
| `--gpu` | Enable GPU acceleration |
| `--compute-evidence-sha256` | Compute evidence hash |

---

## 3. Coding Standards

### Rust Code Style

```rust
// ✅ DO: Use proper error handling with anyhow
use anyhow::{Context, Result, bail};

pub fn read_metadata(path: &Path) -> Result<Vec<CarvedFile>> {
    let file = File::open(path)
        .context("Failed to open metadata file")?;
    // ...
}

// ✅ DO: Use Arc<Mutex<>> for shared state
use std::sync::{Arc, Mutex};

struct App {
    scan_manager: Arc<Mutex<ScanManager>>,
}

// ✅ DO: Use proper struct organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    pub input_path: String,
    pub output_path: String,
    // ...
}

// ❌ DON'T: Use unwrap() in production code
// ❌ DON'T: Block the UI thread with long operations
// ❌ DON'T: Expose raw file paths in error messages
```

### egui UI Patterns

```rust
// ✅ DO: Use immediate-mode UI pattern
impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Title");
            if ui.button("Click me").clicked() {
                self.do_something();
            }
        });
    }
}

// ✅ DO: Use panels for layout
egui::SidePanel::left("sidebar").show(ctx, |ui| { ... });
egui::TopBottomPanel::top("menu").show(ctx, |ui| { ... });
egui::CentralPanel::default().show(ctx, |ui| { ... });

// ✅ DO: Use RichText for styling
ui.label(RichText::new("Important").strong().color(Color32::RED));

// ✅ DO: Request repaint for animations
if self.is_scanning {
    ctx.request_repaint_after(Duration::from_millis(100));
}
```

---

## 4. File Naming Conventions

| Type | Convention | Example |
|------|------------|---------|
| Modules | snake_case | `scan_manager.rs` |
| Structs | PascalCase | `ScanConfig` |
| Functions | snake_case | `parse_progress_message` |
| Constants | SCREAMING_SNAKE | `FILE_TYPES` |
| Enums | PascalCase | `MetadataBackend` |
| Test functions | snake_case with test_ prefix | `test_parse_progress` |

---

## 5. Module Guidelines

### Module Structure
```rust
//! Module documentation
//! 
//! Describes the purpose of this module.

mod submodule;

pub use submodule::PublicType;

// Public types
pub struct MyType { ... }

// Public functions
pub fn my_function() -> Result<()> { ... }

// Private helpers
fn helper() { ... }

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_something() { ... }
}
```

### UI Panel Structure
```rust
pub struct ConfigPanel {
    // Panel-specific state
}

impl Default for ConfigPanel {
    fn default() -> Self { Self::new() }
}

impl ConfigPanel {
    pub fn new() -> Self { ... }
    
    pub fn show(&mut self, ui: &mut Ui, config: &mut ScanConfig) {
        // Render UI
    }
}
```

---

## 6. Testing Guidelines

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_progress_message() {
        let msg = "progress bytes_scanned=1000 total_bytes=10000 ...";
        let progress = parse_progress_message(msg).unwrap();
        assert_eq!(progress.bytes_scanned, 1000);
    }
}
```

### Integration Tests (tests/ directory)
```rust
use swiftbeaverlodge::metadata::MetadataReader;

#[test]
fn test_metadata_reader_mock() {
    let temp = TempDir::new().unwrap();
    // Create mock data
    let reader = MetadataReader::new(temp.path()).unwrap();
    // Assert results
}
```

### Test Count Target
- **Minimum:** 50 tests across unit + integration
- **Current:** 69 tests (28 lib + 35 bin + 6 integration)

---

## 7. Error Handling

### Pattern: Result with Context
```rust
use anyhow::{Context, Result, bail};

pub fn do_thing(path: &Path) -> Result<Data> {
    if !path.exists() {
        bail!("Path does not exist: {}", path.display());
    }
    
    let file = File::open(path)
        .context("Failed to open file")?;
    
    let data = parse_file(file)
        .with_context(|| format!("Failed to parse: {}", path.display()))?;
    
    Ok(data)
}
```

### UI Error Display
```rust
if let Some(error) = &self.error {
    ui.colored_label(Color32::RED, format!("Error: {}", error));
}
```

---

## 8. Security Considerations

### Forensic Integrity
- ✅ ALWAYS use read-only access to evidence files
- ✅ ALWAYS compute/verify SHA-256 hashes
- ✅ ALWAYS log operations for audit trail
- ❌ NEVER modify evidence files
- ❌ NEVER include evidence paths in user-facing error messages

### Input Validation
```rust
fn validate_config(config: &ScanConfig) -> Vec<String> {
    let mut issues = Vec::new();
    
    if config.input_path.is_empty() {
        issues.push("Input file path is required".to_string());
    } else if !Path::new(&config.input_path).exists() {
        issues.push("Input file does not exist".to_string());
    }
    
    issues
}
```

---

## 9. Performance Guidelines

### Large File Handling
```rust
// ✅ DO: Stream large data, don't load all at once
// ✅ DO: Use pagination for metadata tables
// ✅ DO: Limit displayed items (e.g., 500 max)

// ❌ DON'T: Load entire JSONL into memory at once
// ❌ DON'T: Display unbounded lists
```

### UI Responsiveness
```rust
// ✅ DO: Run long operations in separate thread
std::thread::spawn(move || {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async { ... });
});

// ✅ DO: Poll state from UI thread
fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    let state = self.scan_manager.lock().unwrap().state();
    // Update UI based on state
}
```

---

## 10. Commit Message Format

```
type(scope): description

[optional body]

[optional footer]
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

Examples:
```
feat(scan): add pause/resume functionality
fix(ui): correct progress bar overflow on long scans
docs(readme): update installation instructions
chore(deps): update egui to 0.30
```

---

## 11. Quick Reference

### Build Commands
```bash
cargo build           # Debug build
cargo build --release # Release build
cargo test            # Run all tests
cargo check           # Type check only
cargo clippy          # Lint
```

### Binary Download
```bash
./download-fastcarve.sh  # Downloads to bin/fastcarve
```

### Key Types
- `ScanConfig` - Configuration for a scan
- `ScanManager` - Manages subprocess lifecycle
- `ScanProgress` - Progress snapshot from fastcarve
- `MetadataReader` - Reads Parquet/JSONL results
- `CarvedFile` - Single carved file metadata
- `Tab` - UI tab enum (Configure/Monitor/Results)

---

*Last Updated: January 3, 2025*
