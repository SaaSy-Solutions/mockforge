# Agent Usage Guidelines

## Available Agents

| Agent | Model | Best For |
|-------|-------|----------|
| `code-reviewer` | sonnet | Nuanced code review with confidence scoring |
| `test-runner` | sonnet | Running tests, diagnosing failures, applying fixes |
| `template-checker` | haiku | Mechanical template variable cross-referencing |
| `security-auditor` | haiku | Pattern-matching for unsafe, secrets, unwrap |
| `code-explorer` | sonnet | Tracing execution paths across crate boundaries |

## When to Use Agents

### Use `template-checker` (haiku) when:
- Any `.hbs` template file is modified
- Any Rust code that builds template data is modified (`build_template_data`, `render`, `serde_json::json!`)
- Before completing any bench-related task

### Use `code-reviewer` (sonnet) when:
- Reviewing PRs or diffs
- After completing a non-trivial implementation
- When the `/code-review` skill is invoked

### Use `test-runner` (sonnet) when:
- Tests fail and need diagnosis
- After implementing changes to run targeted tests
- When the `/verify` skill needs to run tests

### Use `security-auditor` (haiku) when:
- Changes touch authentication, crypto, or registry code
- New `unsafe` code is introduced
- New dependencies are added

### Use `code-explorer` (sonnet) when:
- Understanding how a feature works across crates
- Tracing a trait implementation through the workspace
- Planning changes that span multiple crates

## Parallel Launch Patterns

Launch independent agents in parallel to save time:

**Code review workflow**: Launch `code-reviewer` + `security-auditor` + `template-checker` simultaneously when reviewing bench-related changes.

**Post-implementation**: Launch `test-runner` (for affected crates) + `template-checker` (if bench code changed) simultaneously.

## Token Efficiency

- **haiku agents** (template-checker, security-auditor): Use for mechanical, pattern-matching tasks. ~90% cheaper than sonnet.
- **sonnet agents** (code-reviewer, test-runner, code-explorer): Use for tasks requiring judgment, diagnosis, or understanding complex code.
- **Scope to affected crates**: Always pass the specific crate names to agents rather than asking them to check the whole workspace.

## Memory Maintenance

After discovering a new bug pattern, common mistake, or useful insight:
1. Check if it's already documented in MEMORY.md
2. If not, add it using the Edit tool
3. Keep MEMORY.md under 200 lines — move detailed notes to topic files
