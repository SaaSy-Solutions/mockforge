---
allowed-tools: Bash(git *), Bash(gh *)
description: Branch + commit + push + create PR in one step
---

# /commit-push-pr — Full PR Workflow

Create a branch, commit changes, push, and open a pull request in one step.

## Steps

1. **Assess state**: Run `git status` and `git diff` to understand all changes
2. **Check branch**: If on `main`, create a new branch:
   - Auto-generate a branch name from the changes (e.g., `feat/add-xyz`, `fix/repair-abc`)
   - Run `git checkout -b <branch-name>`
3. **Commit**: Follow the same commit flow as `/commit`:
   - Analyze changes, draft message matching repo conventions (`<type>(<scope>): <description>`)
   - Stage specific files (never `.env`, `.cargo/credentials.toml`, or secrets)
   - Commit with HEREDOC message format
4. **Push**: Push the branch with tracking: `git push -u origin <branch-name>`
5. **Create PR**: Use `gh pr create` with:
   - Short title (under 70 chars)
   - Body with `## Summary` (bullet points) and `## Test plan` sections
   - Use HEREDOC for the body:
     ```bash
     gh pr create --title "the title" --body "$(cat <<'EOF'
     ## Summary
     - ...

     ## Test plan
     - [ ] ...
     EOF
     )"
     ```
6. **Report**: Show the PR URL

## Rules

- Do NOT force push
- Do NOT push to `main` directly — always use a feature branch
- Do NOT use `--no-verify`
- If the branch already exists on remote, push to it (don't create a new one)
- If a PR already exists for this branch, show its URL instead of creating a duplicate
