---
user-invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Task]
description: Validate benchmark scripts and k6 output
argument-hint: "[spec-file]"
---

# /bench-review — Benchmark Script Review

Review and validate benchmark-related code, k6 script generation, and bench test output.

## Process

### 1. Run Bench Tests

```bash
cargo test -p mockforge-bench
```

Report results. If tests fail, diagnose using the test-runner agent.

### 2. Validate k6 Script Generation (if spec provided)

If a spec file is provided, generate a k6 script and validate:

1. Check that the generated script is valid JavaScript
2. Verify all functions defined in the script are actually CALLED (not just defined)
3. Verify no undefined template variables remain (no literal `{{...}}` in output)
4. Check that security test functions are injected AND invoked when security testing is enabled

### 3. Template Consistency Check

Launch the `template-checker` agent to verify all template variables are consistent across code paths.

### 4. Review Key Files

Read and review:
- `crates/mockforge-bench/src/k6_gen.rs` — K6Config and build_template_data
- `crates/mockforge-bench/src/command.rs` — The 3 execution flows
- `crates/mockforge-bench/src/templates/k6_script.hbs` — The template itself
- `crates/mockforge-bench/src/security_payloads.rs` — Security test generation

### 5. Output

```
## Bench Review

### Test Results
- mockforge-bench tests: PASS/FAIL (N tests)

### k6 Script Validation (if spec provided)
- Valid JavaScript: ✅/❌
- Functions defined AND called: ✅/❌
- No raw template vars: ✅/❌
- Security functions injected: ✅/❌ (if security enabled)

### Template Consistency
- (template-checker results)

### Findings
1. ...
```

## Rules

- Always run `cargo test -p mockforge-bench` first
- The most common bug class is functions being defined but not called — always verify call sites
- `{{#if false_or_missing}}` blocks are silently skipped — this is the #1 source of bench bugs
