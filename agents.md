# SwiftBeaverLodge - AI Agent Coding Guidelines

> Guidelines for AI coding agents (Copilot, Claude, etc.) working on this project.

---

## 1. Project Overview

**SwiftBeaverLodge** is a GUI frontend for the SwiftBeaver forensic file carver.

### Tech Stack
| Layer | Technology |
|-------|------------|
| Framework | Tauri 2.x |
| Frontend | Svelte 5 + TypeScript |
| Styling | TailwindCSS 4.x + Skeleton UI |
| Backend | Rust (Tauri commands) |
| Core Engine | SwiftBeaver/fastcarve (Rust crate) |
| Build | Vite 6.x |

### Project Structure
```
SwiftBeaverLodge/
├── src/                    # Svelte frontend
│   ├── lib/
│   │   ├── components/     # Svelte components
│   │   ├── stores/         # Svelte stores (state)
│   │   ├── api/            # Tauri IPC wrappers
│   │   └── utils/          # Utility functions
│   └── routes/             # SvelteKit routes
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── commands/       # Tauri commands
│   │   ├── scan/           # Scan management
│   │   └── results/        # Result processing
│   └── Cargo.toml
├── swiftbeaver-upstream/   # Cloned SwiftBeaver repo (reference only)
└── docs/                   # Documentation
```

---

## 2. SwiftBeaver Integration Notes

### Integration Method: Binary Subprocess

SwiftBeaverLodge uses the **pre-built fastcarve binary** from [GitHub releases](https://github.com/gaestu/SwiftBeaver/releases) rather than the crate API. This provides:

- **Stable releases** - Pinned to tested versions (v0.2.1)
- **Faster builds** - No need to compile fastcarve from source
- **Simpler dependencies** - No libewf/GPU SDK required at build time
- **Cross-platform** - Pre-built binaries per platform

### Binary Location

```
src-tauri/
├── bin/
│   └── fastcarve          # Downloaded binary (dev/bundled)
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
   
   fn parse_json_log(line: &str) -> Option<(String, Value)> {
       // Extract level, message, and progress fields
   }
   ```

3. **Cancellation** - Send SIGTERM to the subprocess:
   ```rust
   #[cfg(unix)]
   unsafe { libc::kill(pid as i32, libc::SIGTERM); }
   ```

4. **Metadata Reading** - Auto-detect Parquet (default) or JSONL:
   ```rust
   fn detect_metadata_backend(run_path: &Path) -> &'static str {
       if run_path.join("parquet").exists() { "parquet" }
       else if run_path.join("metadata/carved_files.jsonl").exists() { "jsonl" }
       else { "unknown" }
   }
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

### ProgressSnapshot Fields (from JSON logs)

```
progress bytes_scanned=N total_bytes=N pct=N.N hits=N files=N rate_mib=N.NN eta_secs=Some(N)
```

---

## 3. Coding Standards

### 3.1 Rust (src-tauri/)

```rust
// ✅ DO: Use proper error handling with anyhow/thiserror
use anyhow::{Context, Result};

#[tauri::command]
async fn start_scan(config: ScanConfig) -> Result<String, String> {
    do_scan(config)
        .await
        .map_err(|e| e.to_string())
}

// ✅ DO: Use Arc for shared state across async boundaries
use std::sync::Arc;
use tokio::sync::Mutex;

struct AppState {
    scan_manager: Arc<Mutex<ScanManager>>,
}

// ✅ DO: Emit events for real-time UI updates
use tauri::Emitter;

app.emit("scan:progress", &progress_data)?;

// ❌ DON'T: Block the async runtime
// ❌ DON'T: Use unwrap() in production code
// ❌ DON'T: Expose raw file paths in error messages (forensic security)
```

### 3.2 Svelte/TypeScript (src/)

```typescript
// ✅ DO: Use TypeScript strict mode
// ✅ DO: Define interfaces for all Tauri command responses
interface ScanProgress {
  bytes_scanned: number;
  total_bytes: number;
  throughput_mib: number;
  eta_seconds: number | null;
}

// ✅ DO: Use Svelte 5 runes ($state, $derived, $effect)
let progress = $state<ScanProgress | null>(null);
let percentage = $derived(
  progress ? (progress.bytes_scanned / progress.total_bytes) * 100 : 0
);

// ✅ DO: Wrap Tauri commands in typed functions
import { invoke } from '@tauri-apps/api/core';

export async function startScan(config: ScanConfig): Promise<string> {
  return invoke<string>('start_scan', { config });
}

// ✅ DO: Use Tauri event listeners with cleanup
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

$effect(() => {
  let unlisten: UnlistenFn;
  
  listen<ScanProgress>('scan:progress', (event) => {
    progress = event.payload;
  }).then((fn) => { unlisten = fn; });
  
  return () => { unlisten?.(); };
});

// ❌ DON'T: Use any type
// ❌ DON'T: Mutate props directly
// ❌ DON'T: Put business logic in components (use stores)
```

### 3.3 CSS/Styling

```svelte
<!-- ✅ DO: Use Tailwind utility classes -->
<div class="flex items-center gap-4 p-4 bg-surface-800 rounded-lg">

<!-- ✅ DO: Use Skeleton UI components -->
<ProgressBar value={percentage} max={100} />

<!-- ✅ DO: Support dark mode (forensic environments) -->
<div class="dark:bg-surface-900 dark:text-white">

<!-- ❌ DON'T: Use inline styles -->
<!-- ❌ DON'T: Create custom CSS when Tailwind/Skeleton has utilities -->
```

---

## 4. File Naming Conventions

| Type | Convention | Example |
|------|------------|---------|
| Svelte components | PascalCase | `FileSelector.svelte` |
| TypeScript modules | camelCase | `scanManager.ts` |
| Rust modules | snake_case | `scan_manager.rs` |
| Stores | camelCase + Store suffix | `scanStore.ts` |
| Types/Interfaces | PascalCase | `ScanConfig` |
| Constants | SCREAMING_SNAKE | `MAX_CHUNK_SIZE` |
| Tauri commands | snake_case | `start_scan` |
| Tauri events | namespace:action | `scan:progress` |

---

## 5. Component Guidelines

### Component Structure
```svelte
<script lang="ts">
  // 1. Imports
  import { ProgressBar } from '@skeletonlabs/skeleton-svelte';
  import { scanStore } from '$lib/stores/scan';
  
  // 2. Props (Svelte 5 syntax)
  interface Props {
    value: number;
    label?: string;
  }
  let { value, label = 'Progress' }: Props = $props();
  
  // 3. State
  let isHovered = $state(false);
  
  // 4. Derived values
  let displayValue = $derived(`${value.toFixed(1)}%`);
  
  // 5. Effects
  $effect(() => {
    // Side effects here
  });
  
  // 6. Functions
  function handleClick() {
    // ...
  }
</script>

<!-- Template -->
<div class="...">
  <span>{label}: {displayValue}</span>
  <ProgressBar {value} max={100} />
</div>
```

### Store Structure
```typescript
// src/lib/stores/scan.ts
import { writable, derived } from 'svelte/store';

interface ScanState {
  status: 'idle' | 'running' | 'paused' | 'completed' | 'failed';
  progress: ScanProgress | null;
  runId: string | null;
  error: string | null;
}

function createScanStore() {
  const { subscribe, set, update } = writable<ScanState>({
    status: 'idle',
    progress: null,
    runId: null,
    error: null,
  });

  return {
    subscribe,
    start: (runId: string) => update(s => ({ ...s, status: 'running', runId })),
    updateProgress: (progress: ScanProgress) => update(s => ({ ...s, progress })),
    complete: () => update(s => ({ ...s, status: 'completed' })),
    fail: (error: string) => update(s => ({ ...s, status: 'failed', error })),
    reset: () => set({ status: 'idle', progress: null, runId: null, error: null }),
  };
}

export const scanStore = createScanStore();
```

---

## 6. Tauri Command Guidelines

### Command Definition (Rust)
```rust
// src-tauri/src/commands/scan.rs

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

#[derive(Debug, Deserialize)]
pub struct ScanConfig {
    pub input_path: String,
    pub output_path: String,
    pub file_types: Vec<String>,
    // ... other fields
}

#[derive(Debug, Serialize)]
pub struct ScanHandle {
    pub run_id: String,
}

/// Start a forensic scan operation
/// 
/// # Arguments
/// * `config` - Scan configuration from frontend
/// * `state` - Application state (injected by Tauri)
/// * `app` - App handle for event emission
#[tauri::command]
pub async fn start_scan(
    config: ScanConfig,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<ScanHandle, String> {
    let mut manager = state.scan_manager.lock().await;
    manager.start(config, app)
        .await
        .map_err(|e| format!("Scan failed: {e}"))
}
```

### Command Invocation (TypeScript)
```typescript
// src/lib/api/scan.ts

import { invoke } from '@tauri-apps/api/core';

export interface ScanConfig {
  input_path: string;
  output_path: string;
  file_types: string[];
}

export interface ScanHandle {
  run_id: string;
}

export async function startScan(config: ScanConfig): Promise<ScanHandle> {
  return invoke<ScanHandle>('start_scan', { config });
}
```

---

## 7. Error Handling

### Rust Errors
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("Invalid input path: {0}")]
    InvalidInput(String),
    
    #[error("Evidence source error: {0}")]
    Evidence(#[from] anyhow::Error),
    
    #[error("Scan already in progress")]
    AlreadyRunning,
}

// Convert to user-friendly strings for frontend
impl From<ScanError> for String {
    fn from(err: ScanError) -> String {
        err.to_string()
    }
}
```

### Frontend Error Display
```svelte
<script lang="ts">
  import { Toast } from '@skeletonlabs/skeleton-svelte';
  
  async function handleStartScan() {
    try {
      const handle = await startScan(config);
      // success
    } catch (error) {
      // Show toast notification
      toastStore.trigger({
        message: `Scan failed: ${error}`,
        background: 'variant-filled-error',
      });
    }
  }
</script>
```

---

## 8. Testing Guidelines

### Rust Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let config = ScanConfig {
            input_path: "/nonexistent".into(),
            // ...
        };
        assert!(validate_config(&config).is_err());
    }

    #[tokio::test]
    async fn test_scan_lifecycle() {
        // Test start -> progress -> complete
    }
}
```

### Frontend Tests (Vitest)
```typescript
// src/lib/utils/formatters.test.ts
import { describe, it, expect } from 'vitest';
import { formatBytes, formatDuration } from './formatters';

describe('formatBytes', () => {
  it('formats bytes correctly', () => {
    expect(formatBytes(1024)).toBe('1.00 KB');
    expect(formatBytes(1048576)).toBe('1.00 MB');
  });
});
```

---

## 9. Security Considerations

### Forensic Integrity
- ✅ ALWAYS use read-only access to evidence files
- ✅ ALWAYS compute/verify SHA-256 hashes
- ✅ ALWAYS log operations for audit trail
- ❌ NEVER modify evidence files
- ❌ NEVER include evidence paths in error messages shown to users (use generic messages)

### Tauri Security
```json
// src-tauri/capabilities/default.json
{
  "permissions": [
    "core:default",
    "dialog:default",
    "fs:allow-read",
    "fs:allow-write"
  ]
}
```
- Only request necessary permissions
- No network permissions (offline-capable)
- No shell execution (except controlled commands)

---

## 10. Performance Guidelines

### Large File Handling
```typescript
// ✅ DO: Stream large data, don't load all at once
// ✅ DO: Use pagination for metadata tables
// ✅ DO: Generate thumbnails lazily

// ❌ DON'T: Load entire JSONL into memory
// ❌ DON'T: Generate all thumbnails at startup
```

### UI Responsiveness
```svelte
<!-- ✅ DO: Debounce rapid updates -->
<script>
  import { debounce } from '$lib/utils/debounce';
  
  const debouncedUpdate = debounce((value) => {
    // Update UI
  }, 100);
</script>
```

---

## 11. Commit Message Format

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
chore(deps): update tauri to 2.1.0
```

---

## 12. Quick Reference: Binary Integration

### Download Binary

```bash
cd src-tauri
./download-fastcarve.sh  # Downloads v0.2.1 to bin/fastcarve
```

### Spawn Subprocess

```rust
use std::process::{Command, Stdio};

let mut child = Command::new("bin/fastcarve")
    .args(&[
        "--input", &input_path,
        "--output", &output_path,
        "--log-format", "json",
        "--progress-interval-secs", "1",
        "--metadata-backend", "parquet",
    ])
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;
```

### Parse Progress

```rust
// JSON log line: {"timestamp":"...","level":"INFO","message":"progress bytes_scanned=..."}
fn parse_progress_message(message: &str) -> Option<GuiScanProgress> {
    let parts: HashMap<&str, &str> = message
        .strip_prefix("progress ")?
        .split_whitespace()
        .filter_map(|p| { let mut s = p.splitn(2, '='); Some((s.next()?, s.next()?)) })
        .collect();
    // Extract bytes_scanned, total_bytes, hits, files, rate_mib, eta_secs
}
```

### Read Parquet Results

```rust
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

let file = File::open("output/run_id/parquet/files_jpeg.parquet")?;
let reader = ParquetRecordBatchReaderBuilder::try_new(file)?.build()?;

for batch in reader {
    // Process Arrow RecordBatch
}
```

---

*Last Updated: January 2, 2026*
