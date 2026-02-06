# Self-Verification Rule

Before reporting any task as complete, you MUST run through this verification checklist. Scope checks to only the crates/files you actually changed — never run `--workspace` checks when a scoped check will do.

## 1. Determine Scope

Identify which crates and files were modified:
```bash
git diff --name-only HEAD  # or compare against your starting point
```

Group changed files by crate (extract from `crates/<crate-name>/`). This is your **affected crate set**.

## 2. Plan Compliance

If working from a plan or task list:
- Re-read the original requirements
- Verify every specified item is addressed
- Do not claim items are done if they were skipped or deferred

## 3. Formatting Check

```bash
cargo fmt --all --check
```
If it fails, run `cargo fmt --all` and include the formatting fix.

## 4. Clippy (Per Affected Crate)

For EACH affected crate, run:
```bash
cargo clippy -p <crate-name> --all-targets -- -D warnings
```
Do NOT run `cargo clippy --workspace` unless changes span 10+ crates. Fix all warnings before proceeding.

## 5. Tests (Per Affected Crate)

For EACH affected crate, run:
```bash
cargo test -p <crate-name>
```
All tests must pass. If a test fails:
1. Read the failure output carefully
2. Determine if your change caused the failure
3. Fix the issue
4. Re-run the test

## 6. Template Variable Consistency

**When**: Any file in `crates/mockforge-bench/src/templates/` or template-data-building code was changed.

Run `/template-check` or manually verify:
1. Extract all `{{variable}}` references from changed `.hbs` templates
2. Find ALL Rust code paths that call `handlebars.render()` with that template
3. Verify every variable is present in every code path's template data
4. Pay special attention to `{{#if flag}}` — the flag must be a boolean in ALL render calls

## 7. Unsafe Audit

**When**: Any `.rs` file was added or modified.

Search for new `unsafe` blocks in your changes:
- Every `unsafe` block MUST have a `// SAFETY:` comment directly above it explaining soundness
- If `unsafe` isn't absolutely necessary, refactor to safe code
- Remember: `unsafe_code = "deny"` is set workspace-wide

## 8. UI Checks

**When**: Any file in `crates/mockforge-ui/` was changed.

```bash
cd crates/mockforge-ui/ui
pnpm type-check   # TypeScript compilation
pnpm lint          # ESLint
```

Fix any errors before proceeding.

## 9. Browser Verification

**When**: UI components were changed AND the dev server is running.

Use the Playwright MCP to:
1. Navigate to the affected page
2. Take a snapshot to verify the UI renders correctly
3. Check the console for errors
4. Verify interactive elements work

## 10. Self-Correction Loop

If ANY check fails:
1. Fix the issue
2. Re-run the failing check
3. Repeat until clean
4. Then continue with remaining checks

Do not skip a failing check and move on. Fix it first.

## 11. Completion Report

Before telling the user the task is done, confirm:
- [ ] All changed crates pass `cargo clippy -p <crate> -- -D warnings`
- [ ] All changed crates pass `cargo test -p <crate>`
- [ ] `cargo fmt --all --check` passes
- [ ] No new `unsafe` without `// SAFETY:` comments
- [ ] Template variables are consistent across all render paths (if applicable)
- [ ] UI type-check and lint pass (if applicable)
- [ ] The original task requirements are fully met

Only then report the task as complete. If any item cannot be verified, explicitly state what was skipped and why.
