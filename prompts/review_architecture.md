# Review Architecture

You are reviewing SwiftBeaverLodge changes for architecture and maintainability only.

First read the root `AGENTS.md`, then follow it strictly.

## Focus

Check for:

- fit with the existing egui panel structure
- fit with the scan manager subprocess boundary
- duplicated binary discovery, CLI argument mapping, metadata loading, or state-management mechanisms
- UI-thread blocking in `eframe::App::update` or panel rendering
- unnecessary abstractions or dependencies
- poor separation between configuration, scan execution, metadata reading, and UI presentation
- dead code introduced by the change
- confusing naming, especially around SwiftBeaver vs SwiftBeaverLodge ownership
- over-large functions doing multiple jobs
- style inconsistency with neighboring Rust modules

## Rules

- Prefer extending existing modules over inventing parallel ones.
- Keep SwiftBeaver engine responsibilities out of the GUI unless explicitly requested.
- Do not treat stale Tauri/Svelte planning docs as current architecture.
- Flag unnecessary complexity that makes scan lifecycle, UI state, or metadata loading harder to reason about.

## Output

Review only the changed files and relevant surrounding context.

Report only actual findings. If there are no findings, output exactly:

```text
PASS
```

Use this format for each finding:

```text
### [SEVERITY] filename:line - Short title

Domain: Architecture
What: Describe the issue clearly.
Why it matters: Explain the maintenance or design impact.
Suggestion: Provide a concrete fix or improvement.
```
