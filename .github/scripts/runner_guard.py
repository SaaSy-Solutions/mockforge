"""Fail when a workflow job could run on a billed or unapproved runner.

This repository is public. GitHub's standard hosted runners are free here, but
larger runners (-N-cores, -arm, -gpu) and third-party runners such as
Blacksmith are billed, and self-hosted runners must never run untrusted PR
code. Self-hosted jobs are allowed only when listed in SELF_HOSTED_OK as
"<workflow file>:<job id>".

The check is fail-closed: anything it cannot prove free is rejected. The only
expression allowed in runs-on is ${{ matrix.<key> }} over literal labels;
runner groups, mappings, other expressions and reusable workflows from other
repositories are rejected because their runner cannot be checked here.
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
MATRIX_REF = re.compile(r"\$\{\{\s*matrix\.([A-Za-z0-9_-]+)\s*\}\}")


def matrix_labels(job: dict, name: str) -> list:
    matrix = (job.get("strategy") or {}).get("matrix")
    if not isinstance(matrix, dict):
        return []  # matrix built by an expression: cannot be checked
    values = matrix.get(name, [])
    if not isinstance(values, list):
        return []
    values = list(values) + [x[name] for x in matrix.get("include") or [] if isinstance(x, dict) and name in x]
    return values if all(isinstance(v, str) and "${{" not in v for v in values) else []


def job_problem(job: dict, key: str, approved: set) -> str | None:
    if "uses" in job:
        local = str(job["uses"]).startswith("./")
        return None if local else "calls a reusable workflow from another repository; its runners cannot be checked"
    runs_on = job.get("runs-on")
    if isinstance(runs_on, str):
        runs_on = [runs_on]
    if not isinstance(runs_on, list) or not runs_on or not all(isinstance(v, str) for v in runs_on):
        return f"runs-on {runs_on!r} must be a label or list of labels"
    if "self-hosted" in runs_on:
        return None if key in approved else "self-hosted runner is not approved"
    if len(runs_on) == 1 and (m := MATRIX_REF.fullmatch(runs_on[0].strip())):
        labels = matrix_labels(job, m.group(1))
        if not labels:
            return f"matrix.{m.group(1)} is not a literal label list"
        bad = sorted(set(labels) - FREE)
        return f"matrix.{m.group(1)} may select {bad}" if bad else None
    if any("${{" in v for v in runs_on):
        return f"runs-on expression {runs_on} cannot be proven free"
    bad = sorted(set(runs_on) - FREE)
    return f"{runs_on} is not a free standard runner" if bad else None


def problems(workflows: pathlib.Path, approved: set) -> list:
    found = []
    for workflow in sorted(workflows.glob("*.y*ml")):
        jobs = (yaml.safe_load(workflow.read_text()) or {}).get("jobs") or {}
        for job_id, job in jobs.items():
            key = f"{workflow.name}:{job_id}"
            if (problem := job_problem(job or {}, key, approved)):
                found.append(f"{key}: {problem}")
    return found


if __name__ == "__main__":
    approved = {x.strip() for x in os.environ.get("SELF_HOSTED_OK", "").split(",") if x.strip()}
    found = problems(pathlib.Path(".github/workflows"), approved)
    print("\n".join(found) or "All jobs use free standard runners or approved self-hosted ones.")
    sys.exit(1 if found else 0)
