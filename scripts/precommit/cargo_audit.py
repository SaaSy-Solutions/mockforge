#!/usr/bin/env python3
"""Run `cargo audit` using audit.toml as the single source of truth for
ignored advisories.

The cargo-audit CLI has no native config-file support, so the pre-commit
hook previously duplicated the ignore list inline. This wrapper reads
audit.toml once and forwards each id as `--ignore`. Any arguments passed
to this script are appended to the cargo-audit invocation unchanged.
"""
from __future__ import annotations

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


def main() -> int:
    ids = load_ignored_ids(Path("audit.toml"))
    cmd = ["cargo", "audit"]
    for adv_id in ids:
        cmd.extend(["--ignore", adv_id])
    cmd.extend(sys.argv[1:])
    return subprocess.call(cmd)


if __name__ == "__main__":
    sys.exit(main())
