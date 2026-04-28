# Prompt Set

Use the root `AGENTS.md` as the base instruction set for SwiftBeaverLodge tasks.

Add one task prompt when it matches the work:

- `prompts/implement_issue.md` - implement a GitHub issue end to end
- `prompts/review_uncommitted.md` - review local uncommitted changes before commit
- `prompts/review_correctness.md` - correctness-only review
- `prompts/review_security.md` - security and forensic-safety review
- `prompts/review_error_handling.md` - error-handling-only review
- `prompts/review_architecture.md` - architecture and maintainability review
- `prompts/review_documentation.md` - documentation-only review
- `prompts/review_issue_completeness.md` - verify an issue implementation against requirements

Composition rules:

- `AGENTS.md` is the repository-level source of truth.
- Task prompts add workflow and output expectations.
- Review prompts should be used as specialist reviewer briefs.
- Do not duplicate the entire project guide into task prompts unless needed for clarity.
