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
You are about to edit a Handlebars template. Remember to run `/template-check` after editing to verify all template variables are consistent across all code paths that render this template. (Issue #79 prevention)
