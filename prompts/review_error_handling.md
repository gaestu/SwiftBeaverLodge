# Review Error Handling

You are reviewing SwiftBeaverLodge changes for error handling only.

First read the root `AGENTS.md`, then follow it strictly.

## Focus

Check for:

- `.unwrap()` or `.expect()` in normal application paths
- missing or weak `Result` propagation around filesystem, process, channel, and metadata operations
- missing context for missing SwiftBeaver binaries, spawn failures, output directory creation, and metadata load failures
- swallowed errors that should be logged or surfaced in UI state
- fallback behavior that hides real scan, parse, or result-loading failures
- panics in response to malformed SwiftBeaver JSON logs or metadata files
- lock handling that can poison or block the UI without recovery or clear failure behavior
- errors that expose sensitive evidence paths unnecessarily in new user-facing text

## Rules

- `anyhow` is acceptable in this application/integration codebase; introduce `thiserror` only when structured reusable domain errors help.
- Prefer actionable UI errors for missing binaries and invalid configuration.
- Prefer `tracing` for diagnostics.
- Avoid panics in normal failure paths.

## Output

Review only the changed files and relevant surrounding context.

Report only actual findings. If there are no findings, output exactly:

```text
PASS
```

Use this format for each finding:

```text
### [SEVERITY] filename:line - Short title

Domain: Error Handling
What: Describe the issue clearly.
Why it matters: Explain the reliability or debugging impact.
Suggestion: Provide a concrete fix or improvement.
```
