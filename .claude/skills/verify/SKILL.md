---
user-invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Task]
description: Run self-verification checklist on changed code
argument-hint: "[scope: all|rust|ui|bench|template]"
---

# /verify — Self-Verification Checklist

Run the verification checklist from `.claude/rules/self-verification.md` programmatically. Reports a structured PASS/FAIL summary.

## Process

### 1. Determine Changed Files

```bash
git diff --name-only HEAD
git diff --name-only --cached
```

If no changes detected, check against the last commit:
```bash
git diff --name-only HEAD~1
```

### 2. Apply Scope Filter

The user can provide a scope argument:
- **`all`** (default): Run all applicable checks
- **`rust`**: Only Rust checks (fmt, clippy, test)
- **`ui`**: Only UI checks (type-check, lint)
- **`bench`**: Rust checks + template variable consistency
- **`template`**: Only template variable cross-reference

### 3. Identify Affected Crates

Extract crate names from changed file paths:
- `crates/<crate-name>/src/*.rs` → `<crate-name>`
- Group and deduplicate

### 4. Run Checks

For each applicable check, run and capture results:

#### Formatting
```bash
cargo fmt --all --check
```

#### Clippy (per affected crate)
```bash
cargo clippy -p <crate> --all-targets -- -D warnings
```

#### Tests (per affected crate)
```bash
cargo test -p <crate>
```

#### Template Variables (when bench scope or template files changed)
Launch the `template-checker` agent to cross-reference variables.

#### UI Checks (when UI files changed)
```bash
cd crates/mockforge-ui/ui && pnpm type-check && pnpm lint
```

#### Unsafe Audit
Search changed `.rs` files for `unsafe` blocks without `// SAFETY:` comments.

### 5. Self-Correction

If any check fails:
1. Attempt to fix the issue (formatting, simple clippy fixes)
2. Re-run the failing check
3. If the fix requires user input, report it as NEEDS_ACTION

### 6. Report

Output a summary table:

```
## Verification Results

| Check | Status | Details |
|-------|--------|---------|
| Formatting | ✅ PASS | |
| Clippy (mockforge-core) | ✅ PASS | |
| Clippy (mockforge-bench) | ❌ FAIL | unused variable on line 42 |
| Tests (mockforge-core) | ✅ PASS | 23 tests passed |
| Template variables | ✅ PASS | All 3 code paths consistent |
| Unsafe audit | ✅ PASS | No new unsafe blocks |

**Overall: X/Y checks passed**
```

## Rules

- NEVER run `cargo test --workspace` or `cargo clippy --workspace` — always scope to affected crates
- If no files are changed, report "No changes detected" and exit
- Fix formatting issues automatically (run `cargo fmt --all`)
- For clippy and test failures, report them but don't auto-fix unless simple
