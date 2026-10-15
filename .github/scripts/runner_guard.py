"""Fail when a workflow job could run on a billed or unapproved runner.

This repository is public. GitHub's standard hosted runners are free here, but
larger runners (-N-cores, -arm, -gpu) and third-party runners such as
Blacksmith are billed, and self-hosted runners must never run untrusted PR
code. Self-hosted jobs are allowed only when listed in SELF_HOSTED_OK as
"<workflow file>:<job id>".
"""

import os
import pathlib
import re
import sys

import yaml

FREE = {
    "ubuntu-latest", "ubuntu-24.04", "ubuntu-22.04",
    "windows-latest", "windows-2025", "windows-2022",
    "macos-latest", "macos-15", "macos-14",
}
EXPRESSION = re.compile(r"\$\{\{.*\}\}", re.S)


def problems(workflows: pathlib.Path, approved: set[str]) -> list[str]:
    found = []
    for workflow in sorted(workflows.glob("*.y*ml")):
        jobs = (yaml.safe_load(workflow.read_text()) or {}).get("jobs") or {}
        for job_id, job in jobs.items():
            runs_on = job.get("runs-on")
            if runs_on is None:
                continue  # reusable workflow call
            key = f"{workflow.name}:{job_id}"
            values = runs_on if isinstance(runs_on, list) else [runs_on]
            text = " ".join(str(v) for v in values)
            if "self-hosted" in text:
                if key not in approved:
                    found.append(f"{key}: self-hosted runner is not approved")
                continue
            matrix_key = re.fullmatch(r"\$\{\{\s*matrix\.([A-Za-z0-9_-]+)\s*\}\}", text.strip())
            if matrix_key:
                matrix = (job.get("strategy") or {}).get("matrix") or {}
                name = matrix_key.group(1)
                labels = {str(v) for v in matrix.get(name) or []}
                labels |= {str(x[name]) for x in matrix.get("include") or [] if name in x}
                if not labels or not labels <= FREE:
                    found.append(f"{key}: matrix.{name} may select {sorted(labels - FREE) or 'an unchecked label'}")
                continue
            if EXPRESSION.search(text):
                # Every quoted label an expression can select must be free.
                labels = set(re.findall(r"'([^']+)'", text))
                if not labels or not labels <= FREE:
                    found.append(f"{key}: runs-on expression may select {sorted(labels - FREE) or 'an unchecked label'}")
                continue
            if not set(values) <= FREE:
                found.append(f"{key}: {values} is not a free standard runner")
    return found


if __name__ == "__main__":
    approved = {x.strip() for x in os.environ.get("SELF_HOSTED_OK", "").split(",") if x.strip()}
    found = problems(pathlib.Path(".github/workflows"), approved)
    print("\n".join(found) or "All jobs use free standard runners or approved self-hosted ones.")
    sys.exit(1 if found else 0)
