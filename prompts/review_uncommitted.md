# Review Uncommitted Changes

You are reviewing uncommitted SwiftBeaverLodge changes before commit.

First read the root `AGENTS.md`, then follow it strictly.

This prompt is the combined reviewer. Use specialist review prompts when separate review passes would be clearer.

## Review Focus

Check changed files for:

1. Correctness
2. Forensic safety and security
3. Error handling
4. Architecture
5. UI responsiveness and usability, when UI changed
6. Documentation, only when behavior, binary setup, CLI flags, metadata support, prompts, or docs-relevant files changed

## Critical Checks

- No writes to evidence or source images.
- SwiftBeaver remains invoked through safe `Command` argument vectors.
- No blocking process waits, large metadata loads, hashing, or filesystem scans run directly in the egui update loop.
- Scan state transitions, cancellation, and result path handling remain correct.
- SwiftBeaver discovery, release build flavor documentation, GPU config, and `--gpu` CLI behavior stay aligned.
- Metadata backend support stays aligned across config, reader, UI, README, and tests.
- No `.unwrap()` or `.expect()` in normal application paths without a clear reason.
- Use `tracing`, not `println!` or `eprintln!`, outside tests or diagnostics.
- No stale references describe this repo as the SwiftBeaver engine or a Tauri/Svelte app unless explicitly historical.

## Procedure

1. Inspect uncommitted changed files.
2. Read enough local context to understand each change.
3. Pay special attention to:
   - `src/scan/` for binary discovery, subprocess lifecycle, CLI args, progress parsing, and cancellation
   - `src/metadata/` for Parquet/JSONL assumptions and untrusted result data
   - `src/ui/` and `src/app.rs` for UI state, responsiveness, and user workflows
   - `README.md`, `docs/`, and `prompts/` for stale project guidance
4. Check docs only when relevant.
5. Report only actual findings.

## Output Format

```text
### [SEVERITY] filename:line - Short title

Domain: Correctness | Forensic Safety | Security | Error Handling | Architecture | UI | Documentation
What: Describe the issue clearly.
Why it matters: Explain the impact.
Suggestion: Provide a concrete fix or improvement.
```

## Verdict

End with exactly one of:

```text
PASS
NEEDS FIXES
ISSUES
```

Also include:

- findings by severity
- findings by domain
- files reviewed with no issues
