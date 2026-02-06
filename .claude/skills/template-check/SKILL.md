---
user-invocable: true
allowed-tools: [Glob, Grep, Read, Task]
description: Cross-reference Handlebars template variables with all Rust code paths
argument-hint: "[template-name]"
---

# /template-check — Template Variable Cross-Reference

Cross-reference Handlebars template variables with all Rust code paths that render them. This directly defends against the issue #79 class of bugs.

## Process

### 1. Find Target Templates

If a template name is provided, find that specific template. Otherwise, check all `.hbs` templates:

```
crates/mockforge-bench/src/templates/*.hbs
```

And any other `.hbs` files in the workspace.

### 2. Launch Template Checker Agent

Launch the `template-checker` agent (runs on haiku for efficiency) with the list of templates to check.

The agent will:
1. Extract all `{{variable}}`, `{{#if flag}}`, `{{#each list}}` references from each template
2. Find all Rust code paths that render each template
3. Compare the variable sets
4. Report mismatches

### 3. Report Results

Display the agent's findings. Highlight:
- **CRITICAL**: Variables in template but missing from a code path (will silently fail at runtime)
- **WARNING**: Variables provided by code but not used in template (dead code)
- **INFO**: Variables used in `{{#if}}` blocks (must be boolean, not just present)

### Key Templates to Watch

| Template | Location | Render Sites |
|----------|----------|-------------|
| `k6_script.hbs` | `crates/mockforge-bench/src/templates/` | `k6_gen.rs`, `command.rs` (3 paths) |
| OWASP templates | `crates/mockforge-bench/src/owasp_api/` | `generator.rs` |

### Historical Context

Issue #79 was caused by `security_testing_enabled` being present in CRUD flow template data but NOT in standard bench flow template data. The template had `{{#if security_testing_enabled}}` which silently evaluated to false, causing security test functions to be defined but never called.

## Rules

- Always check ALL code paths, not just the most obvious one
- `{{#if missing_var}}` evaluates to false silently — this is the most dangerous pattern
- When reporting, show the exact line in the Rust code where each code path builds its template data
