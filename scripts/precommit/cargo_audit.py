#!/usr/bin/env python3
"""Run `cargo audit` using audit.toml as the single source of truth for
ignored advisories.

The cargo-audit CLI has no native config-file support, so the pre-commit
hook previously duplicated the ignore list inline. This wrapper reads
audit.toml once and forwards each id as `--ignore`. Any arguments passed
to this script are appended to the cargo-audit invocation unchanged.
"""
from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path


def load_ignored_ids(config_path: Path) -> list[str]:
    if not config_path.exists():
        return []
    with config_path.open("rb") as f:
        data = tomllib.load(f)
    ids: list[str] = []
    for entry in data.get("advisories", {}).get("ignore", []):
        if isinstance(entry, str):
            ids.append(entry)
        elif isinstance(entry, dict) and "id" in entry:
            ids.append(entry["id"])
    return ids


def resolve_cargo() -> str:
    """Locate the cargo binary.

    The Security Audit CI job sets `CARGO_HOME=/tmp/cargo-mockforge-<run_id>`
    and `dtolnay/rust-toolchain` adds `$CARGO_HOME/bin` to `GITHUB_PATH`.
    That propagates fine for bash steps, but the wrapper here has been
    observed to fail with `FileNotFoundError: 'cargo'` on the self-hosted
    runner anyway (#447 / PR #497 Security Audit) — most likely a
    step-boundary PATH-propagation race where `subprocess.call(["cargo",
    ...])` sees a PATH that does not include the toolchain dir even though
    the previous bash step found cargo fine.

    Falling back to the conventional rustup-managed locations
    (`$CARGO_HOME/bin`, `~/.cargo/bin`) keeps the wrapper working when
    `shutil.which` returns `None`. We only return absolute paths to a
    binary that actually exists and is executable — preserving the
    original ENOENT failure mode for true "no rust toolchain installed"
    environments instead of masking those.
    """
    found = shutil.which("cargo")
    if found:
        return found
    candidates = [
        os.environ.get("CARGO_HOME"),
        os.path.expanduser("~/.cargo"),
    ]
    for base in candidates:
        if not base:
            continue
        candidate = os.path.join(base, "bin", "cargo")
        if os.path.isfile(candidate) and os.access(candidate, os.X_OK):
            return candidate
    return "cargo"


def main() -> int:
    ids = load_ignored_ids(Path("audit.toml"))
    cmd = [resolve_cargo(), "audit"]
    for adv_id in ids:
        cmd.extend(["--ignore", adv_id])
    cmd.extend(sys.argv[1:])
    return subprocess.call(cmd)


if __name__ == "__main__":
    sys.exit(main())
