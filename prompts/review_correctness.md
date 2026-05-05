# Review Correctness

You are reviewing SwiftBeaverLodge changes for correctness only.

First read the root `AGENTS.md`, then follow it strictly.

## Focus

Check for:

- wrong scan state transitions or stale UI state
- incorrect `ScanConfig` to CLI argument mapping
- mismatches between unified binary discovery, GPU configuration, and `--gpu` behavior
- failure to preserve run output paths reported by SwiftBeaver
- broken progress/log parsing for current `fields.message` JSON or supported legacy formats
- incorrect cancellation behavior or process lifecycle handling
- metadata backend mismatches between config, reader, UI, and docs
- wrong Parquet/JSONL column assumptions or unsafe numeric conversions
- incorrect result filtering, search, selected index handling, or display limits
- regressions in default configuration or serialization
- edge cases with empty paths, missing binaries, empty result sets, or malformed metadata

## Scope

Review only the changed files and relevant surrounding context.

## Output

Report only actual findings. If there are no findings, output exactly:

```text
PASS
```

Use this format for each finding:

```text
### [SEVERITY] filename:line - Short title

Domain: Correctness
What: Describe the issue clearly.
Why it matters: Explain the functional impact.
Suggestion: Provide a concrete fix or improvement.
```
