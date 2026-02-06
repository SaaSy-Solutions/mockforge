---
allowed-tools: Bash(git *)
description: Auto-generate commit message from staged/unstaged changes
---

# /commit — Smart Commit

Generate a commit message from the current staged and unstaged changes, matching this repository's commit style.

## Steps

1. Run `git status` (no `-uall` flag) to see what's changed
2. Run `git diff --cached` and `git diff` to see actual changes
3. Run `git log --oneline -10` to learn the repo's commit message conventions
4. Analyze all changes and draft a commit message:
   - Use the repo's existing commit style: `<type>(<scope>): <description> (#issue)`
   - Types: `feat`, `fix`, `chore`, `test`, `docs`, `refactor`, `perf`
   - Scope: crate name or area (e.g., `bench`, `core`, `ui`, `cli`)
   - Summarize the "why" not the "what"
   - Keep the first line under 72 characters
   - Add a body paragraph if the change is non-trivial
5. Stage relevant files (prefer specific files over `git add -A`)
   - NEVER stage `.env`, `.cargo/credentials.toml`, or secret files
   - Warn if any such files appear in untracked/modified list
6. Create the commit using a HEREDOC for the message:
   ```bash
   git commit -m "$(cat <<'EOF'
   <message here>
   EOF
   )"
   ```
7. Run `git status` to verify the commit succeeded
8. Show the commit hash and summary

## Rules

- Do NOT push to remote — this command only commits locally
- Do NOT use `--no-verify` unless the user explicitly asks
- Do NOT amend previous commits — always create new ones
- If pre-commit hooks fail, fix the issue, re-stage, and create a NEW commit
- If there are no changes to commit, say so and stop
