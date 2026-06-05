---
name: reply-lint
enabled: true
event: tool
action: warn
conditions:
  - field: command
    operator: regex_match
    pattern: "gh (issue|pr) (comment|create|review|edit)"
  - field: command
    operator: contains
    pattern: "—"
---
This `gh` command posts user-facing prose containing an em dash (—). Per the no-em-dashes-in-replies rule, strip it before sending: use commas, colons, semicolons, or parentheses instead. (CHANGELOG and commit-message bodies are exempt, but PR/issue comments addressed to people are not.)
