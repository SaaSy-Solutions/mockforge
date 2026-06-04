---
user-invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Task]
description: Convene the relevant specialist agents on a hard decision, gather independent verdicts, and synthesize options to choose from
argument-hint: "<the decision or question>"
---

# /round-table — Specialist Decision Panel

For a tough call, convene the relevant specialists as an independent panel
(each one opinionated and narrow, like a real review meeting), collect their
verdicts in parallel, then synthesize a short set of options for the human to
pick from. The point is independent perspectives before committing, not one
generalist guessing.

## Process

### 1. Frame the decision
Restate the question in one sentence and identify the decision type. Pick the
panel from the type:

| Decision touches... | Panel (dispatch in parallel) |
|---------------------|------------------------------|
| Releasing / publishing / versioning | `release-guardian` + `test-runner` |
| Auth / SSO / registry / tenancy | `auth-sentinel` + `security-auditor` |
| A protocol admin/TUI surface | `protocol-parity` + `code-explorer` |
| Bench / k6 / templates | `template-checker` + `code-reviewer` |
| A cross-crate design / refactor | `code-explorer` + `code-reviewer` |
| Anything broad / unclear | `code-reviewer` + `code-explorer` + `test-runner` |

Add or drop seats to fit the actual question; 2-3 seats is the sweet spot.
Prefer the cheapest agent that can hold the seat (haiku for mechanical lenses,
sonnet for judgment) per `.claude/rules/agent-usage.md`.

### 2. Dispatch the panel (parallel)
Launch the chosen agents simultaneously (one Task call each, same message). Give
EACH the same decision plus its lens, and ask for: a recommendation, the top
risk it sees, and a confidence level. Tell each to answer from its specialty
only and to disagree if it disagrees — do not seek consensus at this stage.

### 3. Synthesize
After all seats report, produce:
- **Points of agreement** (where the panel converged)
- **Tensions** (where they disagreed, and why — this is the signal)
- **2-3 concrete options**, each with its main trade-off and which seat favors it
- **A recommendation** with the reasoning, clearly separated from the options

### 4. Hand the choice back
Present the options to the human and let them pick. Do NOT auto-execute a
hard/irreversible decision from a round-table without confirmation. If a seat
with veto authority (`release-guardian` NO-GO, `auth-sentinel` BLOCK) objected,
surface that prominently — a veto is not outvoted by the others.

## Rules
- Independent first, synthesize second. Don't let one agent's output bias the
  others (dispatch them in the same batch, blind to each other).
- Keep it proportional: a small call gets a 2-seat panel, not a tribunal.
- Name the seats and their verdicts in the summary so the human sees who said
  what, not just a blended answer.
