# Review Documentation

You are reviewing SwiftBeaverLodge changes for documentation only.

First read the root `AGENTS.md`, then follow it strictly.

Run this review when:

- `README.md` changed
- files under `docs/` changed
- files under `prompts/` changed
- binary setup, SwiftBeaver CLI flags, GPU variants, metadata support, or visible UI workflow changed
- tests or code reveal stale project guidance

## Focus

Check for:

- README and docs aligned with the current Rust egui application
- no stale claims that SwiftBeaverLodge is the SwiftBeaver engine, a Tauri/Svelte app, or a direct SwiftBeaver library wrapper unless explicitly historical
- binary download and variant instructions matching `download-swiftbeaver.sh`, `bin/`, and `src/scan/mod.rs`
- CLI option descriptions matching `build_cli_args`
- metadata support descriptions matching `MetadataBackend` and `MetadataReader`
- user-facing text that is accurate, concise, and not misleading
- prompt files that reference the correct repository name and risk areas
- removal of contradictory instructions

## Output

Review only the changed files and relevant surrounding context.

Report only actual findings. If there are no findings, output exactly:

```text
PASS
```

Use this format for each finding:

```text
### [SEVERITY] filename:line - Short title

Domain: Documentation
What: Describe the issue clearly.
Why it matters: Explain the user or maintainer impact.
Suggestion: Provide a concrete fix or improvement.
```
