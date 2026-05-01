#!/usr/bin/env python3
"""
Pre-commit guard against migration version-prefix collisions.

sqlx records each migration by `(version_prefix, content_checksum)`. Two
migration files in the same directory that share a version prefix are an
unrunnable schema state — whichever sqlx applies first wins, the second
errors `migration N was previously applied but has been modified`, and the
registry server crash-loops on the next deploy.

This catches the *local* case: a developer stages a file whose version
prefix already exists in `crates/<crate>/migrations/` (either committed or
also staged in the same commit). The harder case — two PRs adding the same
prefix concurrently — is caught by `.github/workflows/migration-guard.yml`,
which checks the rebased state against `origin/main`.

Pre-commit invokes this with the staged paths as positional args (we only
care about `crates/*/migrations/*.sql`; everything else is filtered out).
Works whether or not pre-commit was set up with `pass_filenames`.
"""

from __future__ import annotations

import os
import re
import sys
from collections import defaultdict
from pathlib import Path

# Migration filenames are `<digits>_<name>.sql`. The digit prefix is sqlx's
# version. Anything else in the dir (READMEs, gitkeeps) is ignored.
MIGRATION_RE = re.compile(r"^(?P<version>\d+)_.+\.sql$")
MIGRATION_PATH_RE = re.compile(r"^crates/[^/]+/migrations/.+\.sql$")


def staged_migration_paths(argv: list[str]) -> list[Path]:
    """Filter argv down to staged migration .sql files. argv comes from
    pre-commit, which passes paths as forward-slash strings even on Windows."""
    out = []
    for arg in argv:
        if MIGRATION_PATH_RE.match(arg):
            p = Path(arg)
            if p.is_file():
                out.append(p)
    return out


def collisions_in(dir_path: Path) -> dict[str, list[str]]:
    """Group every migration file in `dir_path` by version prefix; return
    the groups with more than one file. Operates on the working tree, so it
    sees both committed migrations and ones being staged in this commit."""
    by_version: dict[str, list[str]] = defaultdict(list)
    if not dir_path.is_dir():
        return {}
    for entry in os.listdir(dir_path):
        m = MIGRATION_RE.match(entry)
        if m:
            by_version[m.group("version")].append(entry)
    return {v: sorted(files) for v, files in by_version.items() if len(files) > 1}


def main(argv: list[str]) -> int:
    staged = staged_migration_paths(argv)
    if not staged:
        return 0

    # We only need to inspect each migration directory once, even if the
    # commit touches several files in it.
    dirs_touched = {p.parent for p in staged}

    failed = False
    for d in sorted(dirs_touched):
        dupes = collisions_in(d)
        if not dupes:
            continue
        failed = True
        print(f"error: duplicate migration version prefix in {d}:", file=sys.stderr)
        for version, files in sorted(dupes.items()):
            print(f"  version {version}:", file=sys.stderr)
            for f in files:
                print(f"    - {f}", file=sys.stderr)
        print(
            "\nsqlx tracks (version, checksum) per migration; two files at the "
            "same version are an unrunnable schema state. Bump the version on "
            "your new migration to the next free prefix.",
            file=sys.stderr,
        )

    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
