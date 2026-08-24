#!/usr/bin/env bash
# Fail if any self-hosted job omits `timeout-minutes` (#1000).
#
# Why this guard exists: mockforge has exactly TWO dedicated self-hosted runner
# registrations. A job without an explicit timeout inherits GitHub's default of
# 360 minutes, so one hung or pathologically slow job removes 50% of CI capacity
# for six hours. On 2026-08-23 a 2h16m `Load Testing` job on main held one slot
# while the required `Security Audit` for the v0.3.214 release sat queued behind
# it, delaying a customer-facing release by roughly two hours.
#
# GitHub-hosted jobs are exempt: there the default is merely wasteful, not a
# capacity outage.
set -euo pipefail

cd "$(dirname "$0")/.."

python3 - <<'PY'
import sys, pathlib
try:
    import yaml
except ImportError:
    print("check-workflow-timeouts: PyYAML not available, skipping")
    sys.exit(0)

missing = []
checked = 0
for f in sorted(pathlib.Path('.github/workflows').glob('*.yml')):
    try:
        doc = yaml.safe_load(f.read_text())
    except Exception as e:
        print(f"  {f.name}: YAML parse error: {e}")
        missing.append(f"{f.name}: unparsable")
        continue
    for name, job in (doc.get('jobs') or {}).items():
        if not isinstance(job, dict):
            continue
        if 'self-hosted' not in str(job.get('runs-on')):
            continue
        checked += 1
        if job.get('timeout-minutes') is None:
            missing.append(f"{f.name}: job '{name}'")

if missing:
    print(f"workflow-timeouts: {len(missing)} self-hosted job(s) without timeout-minutes:")
    for m in missing:
        print(f"  - {m}")
    print()
    print("Add an explicit `timeout-minutes:` to each. Size it generously (roughly")
    print("3x the observed maximum) so host contention cannot turn a slow but")
    print("passing job into a timeout failure.")
    sys.exit(1)

print(f"workflow-timeouts: OK -- all {checked} self-hosted jobs declare timeout-minutes")
PY
