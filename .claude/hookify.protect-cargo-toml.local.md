---
name: protect-cargo-toml
enabled: true
event: file
action: warn
conditions:
  - field: file_path
    operator: regex_match
    pattern: "Cargo\\.toml$"
---
You are about to edit a Cargo.toml file. Remember: (1) workspace dependencies should use `dep.workspace = true`, (2) adding new dependencies affects compile time for all downstream crates, (3) run `cargo deny check licenses sources bans` after adding external dependencies.
