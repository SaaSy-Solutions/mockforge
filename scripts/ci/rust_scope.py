#!/usr/bin/env python3
"""Decide how much Rust CI a pull request needs (#1451).

Reads changed paths (one per line) on stdin and the workspace graph from
`cargo metadata --no-deps`, then writes GitHub step outputs:

  code=true|false   false only when every changed path is docs/agent config
  scope=full|partial
  cargo_args=...    `--workspace --exclude mockforge-desktop` or `-p a -p b`

The decision fails closed. An empty diff, a path outside every workspace
package (root manifests, Cargo.lock, toolchain, workflows, shared fixtures,
proto, vendor, scripts), or an unreadable graph selects the full suite. A
partial scope is the changed packages plus every workspace package that
depends on them (normal, dev, or build edge, transitively).
"""
import argparse
import json
import os
from pathlib import PurePosixPath
import re
import subprocess
import sys

EXCLUDED = {"mockforge-desktop"}  # Never built by the PR Rust lanes.
FULL_ARGS = "--workspace --exclude mockforge-desktop"
NAME = re.compile(r"^[A-Za-z0-9_-]+$")
# Packages whose tests spawn the built `mockforge` binary (through the
# mockforge-test resolver) depend on mockforge-cli at runtime, not via Cargo.
BINARY_PROVIDER = "mockforge-cli"
BINARY_RESOLVER = "mockforge-test"
LIB_KINDS = {"lib", "rlib", "dylib", "cdylib", "staticlib", "proc-macro"}
# Above this share of the workspace, a package list buys nothing.
FULL_THRESHOLD = 0.75


def docs_only(path: str) -> bool:
    return path.startswith(("book/", "docs/", ".claude/")) or path.endswith(".md")


def load_graph(root: str, metadata: str | None):
    raw = metadata if metadata is not None else subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"],
        cwd=root, check=True, capture_output=True, text=True).stdout
    data = json.loads(raw)
    workspace_root = PurePosixPath(data["workspace_root"])
    members = set(data["workspace_members"])
    packages = {}
    by_dir = {}
    libs = set()
    for package in data["packages"]:
        if package["id"] not in members:
            continue
        name = package["name"]
        if not NAME.match(name):
            raise ValueError("unexpected package name: " + name)
        directory = PurePosixPath(package["manifest_path"]).parent.relative_to(workspace_root)
        if str(directory) == ".":
            # A root package would own every path; never narrow scope then.
            raise ValueError("root package makes scope narrowing unsafe")
        deps = {dep["name"] for dep in package["dependencies"] if "path" in dep}
        if BINARY_RESOLVER in deps or name == BINARY_RESOLVER:
            deps.add(BINARY_PROVIDER)
        packages[name] = deps - {name}
        if any(kind in LIB_KINDS for target in package["targets"] for kind in target["kind"]):
            libs.add(name)
        by_dir[str(directory)] = name
    return packages, by_dir, libs


def owner(path: str, by_dir: dict) -> str | None:
    best = None
    for directory, name in by_dir.items():
        if path == directory or path.startswith(directory + "/"):
            if best is None or len(directory) > len(best[0]):
                best = (directory, name)
    return best[1] if best else None


def decide(files, packages, by_dir, libs=None):
    files = [f.strip() for f in files if f.strip()]
    if not files:
        return {"code": "true", "scope": "full", "cargo_args": FULL_ARGS,
                "reason": "empty diff: running the full suite rather than skipping"}
    code_files = [f for f in files if not docs_only(f)]
    if not code_files:
        return {"code": "false", "scope": "none", "cargo_args": "",
                "reason": "docs/agent-config only"}
    changed = set()
    for path in code_files:
        name = owner(path, by_dir)
        if name is None:
            return {"code": "true", "scope": "full", "cargo_args": FULL_ARGS,
                    "reason": "path outside every workspace package: " + path}
        changed.add(name)
    dependents = {name: set() for name in packages}
    for name, deps in packages.items():
        for dep in deps:
            if dep in dependents:
                dependents[dep].add(name)
    affected, stack = set(), list(changed)
    while stack:
        name = stack.pop()
        if name in affected:
            continue
        affected.add(name)
        stack.extend(dependents.get(name, ()))
    selected = sorted(affected - EXCLUDED)
    buildable = len(set(packages) - EXCLUDED)
    # `cargo clippy -p <bin-only> --lib` errors when no selected package has a
    # library, so such a selection runs the full suite instead.
    no_library = libs is not None and not set(selected) & libs
    if not selected or no_library or len(selected) >= FULL_THRESHOLD * buildable:
        return {"code": "true", "scope": "full", "cargo_args": FULL_ARGS,
                "reason": f"{len(selected)} of {buildable} packages affected"}
    return {"code": "true", "scope": "partial",
            "cargo_args": " ".join("-p " + name for name in selected),
            "reason": f"{len(selected)} of {buildable} packages affected: " + ", ".join(selected)}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".")
    parser.add_argument("--metadata", help="cargo metadata JSON file (tests)")
    parser.add_argument("--full", action="store_true", help="force the full suite")
    args = parser.parse_args()
    files = sys.stdin.read().splitlines()
    if args.full:
        result = {"code": "true", "scope": "full", "cargo_args": FULL_ARGS, "reason": "non-PR event"}
    else:
        try:
            metadata = open(args.metadata).read() if args.metadata else None
            packages, by_dir, libs = load_graph(args.root, metadata)
            result = decide(files, packages, by_dir, libs)
        except (OSError, ValueError, KeyError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
            # Never let a broken graph turn into a skip.
            result = {"code": "true", "scope": "full", "cargo_args": FULL_ARGS,
                      "reason": "workspace graph unavailable (" + type(error).__name__ + ")"}
    print(f"Rust CI scope: {result['scope']} ({result['reason']})")
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        with open(output, "a") as handle:
            for key in ("code", "scope", "cargo_args"):
                handle.write(f"{key}={result[key]}\n")
    else:
        print(json.dumps(result))
    return 0


if __name__ == "__main__":
    sys.exit(main())
