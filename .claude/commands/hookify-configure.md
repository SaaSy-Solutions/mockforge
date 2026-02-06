---
allowed-tools: Glob, Read, Edit
description: Enable or disable hookify rules interactively
---

# /hookify:configure — Configure Hookify Rules

Enable, disable, or delete hookify rules interactively.

## Steps

1. Find all files matching `.claude/hookify.*.local.md`
2. Parse and display each rule with its current status
3. Ask the user what they want to do:
   - **Enable/disable**: Toggle the `enabled` field in the frontmatter
   - **Delete**: Remove the rule file entirely (confirm first)
   - **Edit**: Show the rule content and allow modifications
4. Apply the changes

## Rules

- Always confirm before deleting a rule file
- When toggling enabled/disabled, only change the `enabled:` line in frontmatter
- Show the updated state after making changes
