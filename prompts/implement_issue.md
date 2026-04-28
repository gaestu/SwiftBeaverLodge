# Implement GitHub Issue

You are implementing a GitHub issue for SwiftBeaverLodge.

First read the root `AGENTS.md`, then follow it strictly.

## Workflow

1. Read the issue carefully and extract:
   - problem statement
   - requirements
   - acceptance criteria
   - constraints
   - affected user workflow

2. Inspect the relevant code paths and identify:
   - files to change
   - existing patterns to follow
   - tests that need updates
   - documentation that may become stale

3. Create an implementation plan before editing code. Post the plan as a comment to the github issue.

4. Implement conservatively:
   - preserve the egui/subprocess architecture unless the issue explicitly changes it
   - keep SwiftBeaver CLI argument mapping centralized in `src/scan/manager.rs`
   - keep long-running work out of the UI thread
   - preserve backward compatibility for existing Parquet/JSONL result loading
   - avoid unnecessary dependencies
   - keep changes focused on the issue

5. Add or update deterministic tests:
   - unit tests for config, progress parsing, metadata reading, formatting, and state helpers
   - integration tests for cross-module behavior
   - regression tests for bug fixes
   - CLI argument-vector tests when scan options change
   - metadata tests for backend/schema behavior when result loading changes

6. Update documentation only when user-facing behavior, binary setup, CLI flags, result metadata support, or prompt guidance changes. If `CHANGELOG.md` exists, add an appropriate Unreleased entry.

7. Run specialist review passes after implementation:
   - always run:
     - `prompts/review_correctness.md`
     - `prompts/review_security.md`
     - `prompts/review_error_handling.md`
     - `prompts/review_architecture.md`
   - run `prompts/review_documentation.md` when docs, README, prompts, UI text, binary setup, CLI flags, or metadata support changed
   - run `prompts/review_issue_completeness.md` for issue-driven work
   - if review subagents are available, run independent review passes in parallel
   - if any review finds real issues, fix them and re-run only the failed review passes

8. Before finishing code changes, run the most relevant checks. Prefer:

   ```bash
   cargo fmt
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test
   ```

   For prompt-only or documentation-only changes, validate stale references instead of running unnecessary Rust builds.

9.  Return:
   - summary of changes
   - files modified
   - tests added or updated
   - docs updated, if any
   - review passes run
   - checks run and results
   - remaining risks or follow-ups
   - a proposed commit message

## Extra SwiftBeaverLodge Checks

- Never write to evidence or source images.
- Do not replace the SwiftBeaver subprocess boundary with ad hoc parser/carver logic.
- Keep unified CLI discovery, version compatibility, and `gpu_enabled`/`--gpu` behavior aligned across config, UI, binary discovery, and CLI args.
- Keep result loading aligned across `MetadataBackend`, `MetadataReader`, tests, and README.
- Ensure missing SwiftBeaver binaries produce actionable errors.
