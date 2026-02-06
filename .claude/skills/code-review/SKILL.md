---
user-invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Task]
description: Launch parallel review agents for comprehensive code review
argument-hint: "[PR#|file|blank for uncommitted changes]"
---

# /code-review — Comprehensive Code Review

Launch parallel review agents for a thorough code review with confidence-scored findings.

## Process

### 1. Determine Review Target

- **No argument**: Review all uncommitted changes (`git diff` + `git diff --cached`)
- **PR number**: Fetch PR diff with `gh pr diff <number>`
- **File path**: Review that specific file's recent changes

### 2. Analyze Changes

```bash
git diff --stat  # Overview of what changed
git diff         # Full diff
```

Identify:
- Which crates are affected
- Whether bench/template code changed
- Whether auth/crypto/security code changed

### 3. Launch Agents in Parallel

Always launch:
- **code-reviewer** (sonnet): Full 4-pass review with confidence scoring

Conditionally launch:
- **security-auditor** (haiku): If changes touch `auth`, `crypto`, `jwt`, `token`, `password`, `secret`, registry, or middleware code
- **template-checker** (haiku): If changes touch `.hbs` files, `k6_gen.rs`, `command.rs`, or any template data building code

### 4. Consolidate Results

Collect findings from all agents. Apply the 80+ confidence threshold:
- **Report** findings with confidence >= 80
- **Suppress** findings with confidence < 80 (mention count but not details)

### 5. Output

```
## Code Review Summary

### Changes Reviewed
- Files: N (list crates affected)
- Lines: +X / -Y

### Findings (confidence >= 80)
1. [CRITICAL] file:line — Description (confidence: 95)
2. [WARNING] file:line — Description (confidence: 82)

### Suppressed
- N low-confidence findings suppressed (use `/code-review --verbose` to see all)

### Security Audit
- (security-auditor results, if launched)

### Template Consistency
- (template-checker results, if launched)

### Recommendation: APPROVE | REQUEST_CHANGES | NEEDS_DISCUSSION
```

## Rules

- Always launch code-reviewer; other agents are conditional
- The 80+ confidence threshold is the default; `--verbose` shows everything
- Don't duplicate findings across agents — if security-auditor and code-reviewer both find the same issue, report it once
