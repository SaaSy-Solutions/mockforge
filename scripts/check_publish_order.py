#!/usr/bin/env python3
"""Reject publish lists that place workspace dependencies after their consumers."""

import json
from pathlib import Path
import re
import shlex
import subprocess
import sys


def read_publish_order(text):
    """Read the literal CRATES array without executing the publishing script."""
    match = re.search(r"^CRATES=\(\s*\n(.*?)^\)", text, re.MULTILINE | re.DOTALL)
    if match is None:
        raise ValueError("CRATES array not found")
    names = shlex.split(match.group(1), comments=True)
    if not names or any(re.fullmatch(r"mockforge-[a-z0-9-]+", name) is None for name in names):
        raise ValueError("CRATES must contain literal MockForge package names")
    return names


def ordering_errors(metadata, names):
    """Check package identities and internal normal/build edges, including optional deps.

    Dev-only edges are excluded: they do not establish library publication order
    and can legitimately cycle across test fixtures.
    """
    positions = {name: index for index, name in enumerate(names)}
    packages = {package["name"]: package for package in metadata["packages"]}
    errors = []
    if len(positions) != len(names):
        errors.append("duplicate package in CRATES")
    for name in names:
        if name not in packages:
            errors.append(f"{name} is not a workspace package")
            continue
        for dependency in packages[name]["dependencies"]:
            dependency_name = dependency["name"]
            if dependency.get("path") and dependency.get("kind") != "dev" and dependency_name in positions:
                if positions[dependency_name] >= positions[name]:
                    errors.append(f"{dependency_name} must precede {name}")
    return sorted(set(errors))


def main():
    root = Path(__file__).resolve().parent.parent
    try:
        names = read_publish_order((root / "scripts/publish-crates.sh").read_text())
        metadata = json.loads(subprocess.check_output(
            ["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"], cwd=root,
        ))
        errors = ordering_errors(metadata, names)
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"publish-order: FAIL: {error}", file=sys.stderr)
        return 1
    if errors:
        print("publish-order: FAIL", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1
    print(f"publish-order: OK — {len(names)} crates respect normal/build dependency order")
    return 0


if __name__ == "__main__":
    sys.exit(main())
