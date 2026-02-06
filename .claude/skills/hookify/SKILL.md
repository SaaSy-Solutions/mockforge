---
user-invocable: true
allowed-tools: [Read, Write, Glob, Grep]
description: Create behavioral hook rules from plain English
argument-hint: Rule description in plain English
---

# /hookify — Create Hook Rules from Plain English

You are a hookify rule creator. Given a plain English description of a desired behavior, create a hookify rule file.

## What is Hookify?

Hookify lets you create declarative rules that trigger warnings or blocks when certain conditions are met during Claude Code tool use. Rules are stored as `.claude/hookify.*.local.md` files.

## Input

The user provides a plain English description of the rule they want. For example:
- "Warn me before editing any template file"
- "Block deletions of test files"
- "Remind me to run template-check after editing k6_gen.rs"

## Process

1. **Parse the intent**: Understand what event triggers the rule, what conditions to check, and what action to take (warn or block)
2. **Choose the event type**:
   - `file` — Triggered on PreToolUse when a file path matches (Edit, Write, Read)
   - `tool` — Triggered on PreToolUse when a tool or command matches
   - `post_tool` — Triggered on PostToolUse after a tool completes
   - `stop` — Triggered when the agent stops
   - `prompt` — Triggered on UserPromptSubmit
3. **Define conditions**: Each condition has:
   - `field`: The context field to check (`file_path`, `tool_name`, `command`, `prompt`, etc.)
   - `operator`: `regex_match`, `contains`, `equals`, `not_contains`
   - `pattern`: The value to match against
4. **Choose action**: `warn` (shows message, continues) or `block` (shows message, prevents action)
5. **Write the message**: Clear, helpful warning text

## Rule File Format

```markdown
---
name: <kebab-case-name>
enabled: true
event: <file|tool|post_tool|stop|prompt>
action: <warn|block>
conditions:
  - field: <field_name>
    operator: <operator>
    pattern: "<regex_or_string>"
---
<Warning message body in markdown>
```

## Output

1. Generate the rule file name: `.claude/hookify.<name>.local.md`
2. Write the rule file
3. Confirm creation and explain what it does

## Examples

**"Warn before editing templates"** →
```markdown
---
name: protect-templates
enabled: true
event: file
action: warn
conditions:
  - field: file_path
    operator: regex_match
    pattern: "\\.hbs$"
---
You are about to edit a Handlebars template. Remember to run `/template-check` after editing to verify all template variables are consistent across all code paths. (Issue #79 prevention)
```

**"Block deletion of test files"** →
```markdown
---
name: protect-tests
enabled: true
event: file
action: block
conditions:
  - field: file_path
    operator: regex_match
    pattern: "tests/"
  - field: tool_name
    operator: equals
    pattern: "Write"
---
Deletion of test files is blocked. Tests should be updated, not removed.
```

## Rules

- Always use `.local.md` suffix (gitignored by default)
- Use descriptive kebab-case names
- Write actionable warning messages that explain WHY and WHAT TO DO
- Default to `warn` action unless the user explicitly says "block" or "prevent"
- After creating, list the rule back to the user for verification
