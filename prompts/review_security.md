# Review Security

You are reviewing SwiftBeaverLodge changes for security and forensic safety only.

First read the root `AGENTS.md`, then follow it strictly.

## Focus

Check for:

- writes to evidence files, disk images, block devices, or source paths
- shell injection or command construction risks around SwiftBeaver invocation
- arbitrary file writes, opens, previews, exports, or deletes outside intended user-selected locations
- unsafe trust in SwiftBeaver metadata values such as `carved_path`, offsets, sizes, URLs, or strings
- path traversal risks when joining or displaying metadata-provided paths
- denial-of-service risks from unbounded metadata loading, rendering, allocation, or log growth
- integer overflow or lossy signed-to-unsigned conversions for offsets and sizes
- exposing sensitive evidence paths in new user-facing error text unnecessarily
- runtime network access added to normal scan/result workflows
- unsafe cancellation or process cleanup behavior

## High-Risk Areas

- `src/scan/manager.rs` for process spawning, cancellation, output directory handling, and CLI args
- `src/scan/progress.rs` for malformed log handling
- `src/metadata/reader.rs` for untrusted Parquet/JSONL data
- `src/ui/results_panel.rs` for future file-open/export/preview actions
- file dialog and path validation code

## Output

Review only the changed files and relevant surrounding context.

Report only actual findings. If there are no findings, output exactly:

```text
PASS
```

Use this format for each finding:

```text
### [SEVERITY] filename:line - Short title

Domain: Security
What: Describe the issue clearly.
Why it matters: Explain the security or forensic-safety impact.
Suggestion: Provide a concrete fix or improvement.
```
