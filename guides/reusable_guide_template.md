# Reusable Guide Template

Use this structure for **every** guide (blog, video, social). Keep it concise and outcome-driven.

## Title
<Clear, searchable title>

## Who it’s for
- FE dev / QA / SDET / BE / Platform (choose the most relevant)

## Outcome in 5–10 mins (clear promise)
- After this guide you will be able to <deliverable>, measured by <quick check>.

## Prereqs
- Mockforge version: <e.g., vX.Y.Z> (or “latest”)
- CLI installed: `mockforge --version` should print
- Sample repo / project: <repo or path>
- Optional: Node >= <version>, Docker (if used), Git

## Steps (3–7 steps; code + screenshots)
1) <Step name>
   - What & Why
   - Command(s):
     ```bash
     # commands
     ```
   - Expected result / screenshot prompt
2) <Step name>
   - ...

## Gotchas & Debugging
- Symptom → Cause → Fix
- Logs to check: `mockforge logs -f` (follow mode) or `mockforge logs` to view recent logs
- Common config pitfalls

## Automate It (CLI/API snippet for CI)
- Minimal example to run this in CI:
```yaml
# GitHub Actions
name: mockforge-example
on: [pull_request]
jobs:
  mockforge:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Mockforge CLI
        run: |
          curl -fsSL https://get.mockforge.dev | bash
      - name: Start mocks
        run: |
          mockforge serve --http-port 4000 &
          sleep 3  # Wait for server to start
      - name: Run tests
        run: npm test
      - name: Teardown
        if: always()
        run: pkill -f "mockforge serve" || true
```

## Next Up (2 cross-links)
- Related guide 1: <title + link>
- Related guide 2: <title + link>

## Assets (sample repo path, copy-paste snippets)
- Repo path(s): `<path>`
- Snippets: `<file or section anchors>`
- Diagrams: `<link or file>`

## Shorts Pack (3 tweet/LI bullets + 20–30s clip idea)
- Post 1: <copy>
- Post 2: <copy>
- Post 3: <copy>
- Clip idea: <what to show in 20–30s>
