#!/usr/bin/env python3
"""Run clippy scoped to crates affected by the staged files.

Invoked from the pre-commit `clippy` hook with `pass_filenames: true`.
Maps each changed file to its owning workspace crate via the
`crates/<name>/...` path prefix, then runs `cargo clippy -p <crate>` for
each affected crate.

Fallback to a full `cargo clippy --workspace` happens when:
- A workspace-root file (`Cargo.toml`, `Cargo.lock`) changed — these
  invalidate every member's metadata, so per-crate runs would be slower
  than one workspace pass.
- 10 or more crates are affected — past that point per-crate cold
  compiles cost more than a single workspace run that shares deps.
- A changed file lives outside `crates/<name>/` and we can't map it to a
  package. Falling back is the safe choice rather than skipping.

Matches the policy in `.claude/rules/self-verification.md`:
"Do NOT run cargo clippy --workspace unless changes span 10+ crates".
"""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

WORKSPACE_THRESHOLD = 10

WORKSPACE_CLIPPY_ARGS = [
    "cargo",
    "clippy",
    "--all-targets",
    "--all-features",
    "--workspace",
    "--exclude",
    "mockforge-desktop",
    "--",
    "-D",
    "warnings",
]


def crate_for(path: Path) -> str | None:
    """Return the workspace crate name that owns `path`, or None.

    Only `crates/<name>/...` paths are recognized. By convention every
    crate under `crates/` has package name == directory name.
    """
    parts = path.parts
    if len(parts) >= 2 and parts[0] == "crates":
        return parts[1]
    return None


def main(argv: list[str]) -> int:
    files = [Path(f) for f in argv]
    if not files:
        return 0

    for f in files:
        if str(f) in {"Cargo.toml", "Cargo.lock"}:
            print(f"clippy: workspace-root file {f} changed → full workspace run", flush=True)
            return subprocess.call(WORKSPACE_CLIPPY_ARGS)

    affected: set[str] = set()
    unmapped: list[Path] = []
    for f in files:
        crate = crate_for(f)
        if crate:
            affected.add(crate)
        else:
            unmapped.append(f)

    if unmapped:
        print(
            f"clippy: {len(unmapped)} file(s) not under crates/<name>/ → full workspace run",
            flush=True,
        )
        return subprocess.call(WORKSPACE_CLIPPY_ARGS)

    if len(affected) >= WORKSPACE_THRESHOLD:
        print(
            f"clippy: {len(affected)} crates affected (>= {WORKSPACE_THRESHOLD}) → full workspace run",
            flush=True,
        )
        return subprocess.call(WORKSPACE_CLIPPY_ARGS)

    print(f"clippy: scoping to {len(affected)} crate(s): {sorted(affected)}", flush=True)
    for crate in sorted(affected):
        cmd = [
            "cargo",
            "clippy",
            "-p",
            crate,
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ]
        print(f"+ {' '.join(cmd)}", flush=True)
        rc = subprocess.call(cmd)
        if rc != 0:
            return rc
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
