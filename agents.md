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

### API Status: ✅ LIBRARY API AVAILABLE

SwiftBeaver exposes a **full library API** via the `fastcarve` crate:

```rust
// Key public modules from fastcarve crate:
pub mod carve;       // CarvedFile, CarveRegistry, extraction
pub mod config;      // Config, LoadedConfig, load_config()
pub mod pipeline;    // run_pipeline(), run_pipeline_with_cancel()
pub mod metadata;    // MetadataSink, RunSummary
pub mod scanner;     // SignatureScanner, build_signature_scanner()
pub mod strings;     // StringScanner, StringArtefact
pub mod evidence;    // EvidenceSource, open_source()
pub mod checkpoint;  // CheckpointState, save/load checkpoint
```

### Key Integration Points

1. **Progress Reporting** - Implement `pipeline::ProgressReporter` trait:
   ```rust
   pub trait ProgressReporter: Send + Sync {
       fn on_progress(&self, snapshot: &ProgressSnapshot);
   }
   ```

2. **Pipeline Execution** - Use `run_pipeline_with_cancel()` for GUI control:
   ```rust
   pipeline::run_pipeline_with_cancel(
       &cfg,
       evidence_source,
       sig_scanner,
       string_scanner,
       meta_sink,
       &run_output_dir,
       workers,
       chunk_size,
       overlap,
       max_bytes,
       max_chunks,
       carve_registry,
       cancel_flag,       // Arc<AtomicBool> for cancellation
       progress_config,   // Optional progress callback
       checkpoint_config, // Optional checkpointing
   )
   ```

3. **Event Types** - `pipeline::events::MetadataEvent` enum:
   - `File(CarvedFile)` - carved file notification
   - `String(StringArtefact)` - URL/email/phone found
   - `History(BrowserHistoryRecord)` - browser history
   - `Cookie(BrowserCookieRecord)` - browser cookies
   - `Download(BrowserDownloadRecord)` - browser downloads
   - `RunSummary(RunSummary)` - final statistics
   - `Entropy(EntropyRegion)` - high entropy region

4. **ProgressSnapshot Fields**:
   ```rust
   pub struct ProgressSnapshot {
       pub bytes_scanned: u64,
       pub total_bytes: u64,
       pub chunks_processed: u64,
       pub hits_found: u64,
       pub files_carved: u64,
       pub string_spans: u64,
       pub artefacts_extracted: u64,
       pub carve_errors: u64,
       pub metadata_errors: u64,
       pub sqlite_errors: u64,
       pub elapsed_seconds: f64,
       pub throughput_mib: f64,
       pub eta_seconds: Option<u64>,
   }
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

## 12. Quick Reference: SwiftBeaver Crate Usage

```rust
use fastcarve::{
    config::{Config, load_config, LoadedConfig},
    pipeline::{
        run_pipeline_with_cancel,
        ProgressReporter, ProgressSnapshot, ProgressConfig,
        CheckpointConfig,
    },
    evidence::{EvidenceSource, open_source},
    scanner::{SignatureScanner, build_signature_scanner},
    strings::{StringScanner, build_string_scanner},
    metadata::{MetadataSink, MetadataBackendKind, build_sink},
    carve::{CarvedFile, CarveRegistry},
    checkpoint::{CheckpointState, load_checkpoint, save_checkpoint},
};

// The dependency in Cargo.toml:
// fastcarve = { git = "https://github.com/gaestu/SwiftBeaver", branch = "main" }
```

---

*Last Updated: January 2, 2026*
