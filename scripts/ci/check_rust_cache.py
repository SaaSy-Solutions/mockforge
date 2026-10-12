"""Read-only disk and structural cache checks before an integration build.

The target is restricted to the checkout's target directory. Never follow
symlinks, delete artifacts, or infer the historical missing-rlib cause. A
completed library fingerprint needs at least one nonempty library artifact;
Cargo check metadata and procedural macros are valid cache contents too.
When Rust dep-info declares outputs, every declared library output is checked,
including a missing rlib whose metadata file is still present.
"""
import argparse
import json
from pathlib import Path
import shutil

from rust_cache_artifacts import library_problem
from rust_cache_files import regular_files

GIB = 1024 ** 3


def inspect(workspace: Path, min_free: int = 10 * GIB) -> dict:
    workspace = workspace.resolve(strict=True)
    target = workspace / "target"
    free = shutil.disk_usage(workspace).free
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
    parser.add_argument("--min-free-gib", type=int, default=10)
    args = parser.parse_args()
    if args.min_free_gib < 1:
        parser.error("minimum headroom must be positive")
    try:
        result = inspect(args.workspace, args.min_free_gib * GIB)
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
