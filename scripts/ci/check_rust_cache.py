"""Read-only disk and structural cache checks before an integration build.

The target is restricted to the checkout's target directory. Never follow
external links, delete artifacts, or infer the historical missing-rlib cause.
Native .so aliases are accepted only for nonempty versioned siblings inside
Cargo build outputs; symlink directories and Rust artifact links are rejected. A
completed library fingerprint needs at least one nonempty library artifact;
Cargo check metadata and procedural macros are valid cache contents too.
When Rust dep-info declares outputs, every declared library output is checked,
including a missing rlib whose metadata file is still present.
"""
import argparse
import json
import os
from pathlib import Path
import shutil

from rust_cache_artifacts import library_problem
from rust_cache_files import regular_files

GIB = 1024 ** 3


def existing_ancestor(path: Path) -> Path:
    while not path.exists() and path != path.parent:
        path = path.parent
    return path


def inspect(workspace: Path, min_free: int = 10 * GIB, target: Path | None = None) -> dict:
    workspace = workspace.resolve(strict=True)
    # A runner-persistent CARGO_TARGET_DIR (scripts/ci/runner-cargo-target.sh)
    # is inspected in place; its path is used as given, never resolved, so a
    # symlinked target is still reported rather than followed.
    target = workspace / "target" if target is None else Path(os.path.abspath(target))
    free = min(shutil.disk_usage(workspace).free,
               shutil.disk_usage(existing_ancestor(target)).free)
    errors = []
    size = 0
    libraries = 0
    for path, info in regular_files(target, errors):
        size += info.st_size
        count, problem = library_problem(path)
        libraries += count
        if problem is not None:
            errors.append(problem)
    if free < min_free:
        errors.append("insufficient filesystem headroom before compilation")
    return {"free_bytes": free, "target_bytes": size,
            "library_fingerprints": libraries, "errors": sorted(set(errors)),
            "large_target": size > 30 * GIB}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workspace", type=Path, default=Path.cwd())
    parser.add_argument("--target", type=Path, default=None,
                        help="Cargo target dir to inspect (default: <workspace>/target)")
    parser.add_argument("--min-free-gib", type=int, default=10)
    args = parser.parse_args()
    if args.min_free_gib < 1:
        parser.error("minimum headroom must be positive")
    try:
        result = inspect(args.workspace, args.min_free_gib * GIB, args.target)
    except OSError as error:
        print("::error::Cache inspection failed: " + type(error).__name__)
        return 1
    print(json.dumps(result, sort_keys=True))
    if result["large_target"]:
        print("::warning::target exceeds 30 GiB; inspect shared-host capacity through the infrastructure-owned cleanup process")
    for error in result["errors"]:
        print("::error::" + error)
    return int(bool(result["errors"]))


if __name__ == "__main__":
    raise SystemExit(main())
