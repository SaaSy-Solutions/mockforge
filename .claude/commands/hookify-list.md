---
allowed-tools: Glob, Read
description: List all active hookify rules
---

# /hookify:list — List Hookify Rules

List all hookify rules and their status.

## Steps

1. Find all files matching `.claude/hookify.*.local.md`
2. For each file, read and parse the YAML frontmatter
3. Display a table:

```
| # | Name | Event | Action | Enabled | File |
|---|------|-------|--------|---------|------|
| 1 | protect-templates | file | warn | yes | hookify.protect-templates.local.md |
```

4. Show total count: "X rules (Y enabled, Z disabled)"

## Rules

- If no rules exist, say "No hookify rules found. Use `/hookify` to create one."
- Sort by name alphabetically
- Show the full condition summary for each rule
