# Review Issue Completeness

You are reviewing SwiftBeaverLodge changes for issue completeness only.

First read the root `AGENTS.md`, then follow it strictly.

GitHub issues are the source of truth for requirements and acceptance criteria.

## Focus

Check whether the implementation:

- solves the stated problem
- addresses each listed requirement
- satisfies acceptance criteria
- respects stated constraints
- matches the posted implementation plan, if one was added as an issue comment
- keeps changes inside SwiftBeaverLodge's ownership boundaries
- updates tests for the changed behavior
- updates docs when user-facing workflow, binary setup, CLI flags, metadata support, or prompt guidance changed
- avoids unrelated scope creep

## Scope

Use:

- the GitHub issue
- changed files
- relevant local context
- the implementation-plan comment, if available
- test and review results

## Output

Report only actual findings. If there are no findings, output exactly:

```text
PASS
```

Use this format for each finding:

```text
### [SEVERITY] filename:line - Short title

Domain: Issue Completeness
What: Describe the gap clearly.
Why it matters: Explain why the issue is not fully satisfied.
Suggestion: Provide a concrete fix or follow-up.
```
