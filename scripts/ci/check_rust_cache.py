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
import os
from pathlib import Path
import re
import shutil
import stat

GIB = 1024 ** 3


def inspect(workspace: Path, min_free: int = 10 * GIB) -> dict:
    workspace = workspace.resolve(strict=True)
    target = workspace / "target"
    free = shutil.disk_usage(workspace).free
    errors = []
    size = 0
    libraries = 0
    if target.is_symlink():
        errors.append("target is a symlink; refusing to inspect an external cache")
    elif target.exists():
        if not target.is_dir():
            errors.append("target is not a directory")
        else:
            for directory, directories, files in os.walk(target, followlinks=False, onerror=raise_io):
                root = Path(directory)
                for name in list(directories):
                    if (root / name).is_symlink():
                        directories.remove(name)
                        errors.append("cache contains a directory symlink")
                for name in files:
                    path = root / name
                    info = path.lstat()
                    if not stat.S_ISREG(info.st_mode):
                        errors.append("cache contains a nonregular file")
                        continue
                    size += info.st_size
                    if root.parent.name != ".fingerprint" or not name.startswith("lib-"):
                        continue
                    # .json and dep-info siblings aren't success fingerprints.
                    if "." in name:
                        continue
                    digest = root.name.rsplit("-", 1)[-1]
                    if not re.fullmatch(r"[0-9a-f]{16}", digest):
                        continue
                    libraries += 1
                    stem = "lib" + name[4:].replace("-", "_") + "-" + digest
                    deps = root.parent.parent / "deps"
                    if deps.is_symlink():
                        errors.append("cache artifact directory is a symlink")
                        continue
                    crate = name[4:].replace("-", "_")
                    candidates = [stem + suffix for suffix in (".rlib", ".rmeta", ".so", ".a")]
                    candidates += ["lib" + crate + suffix for suffix in (".so", ".a")]
                    dep_info = deps / (crate + "-" + digest + ".d")
                    declared = declared_outputs(dep_info, candidates)
                    if declared:
                        missing = [n for n in declared if not regular_nonempty(deps / n)]
                        if missing:
                            errors.append("declared library output missing or empty: " + root.name)
                    elif not any(regular_nonempty(deps / n) for n in candidates):
                        errors.append("completed library fingerprint lacks an artifact: " + root.name)
    if free < min_free:
        errors.append("insufficient filesystem headroom before compilation")
    return {"free_bytes": free, "target_bytes": size,
            "library_fingerprints": libraries, "errors": sorted(set(errors)),
            "large_target": size > 30 * GIB}


def declared_outputs(dep_info: Path, candidates: list[str]) -> list[str]:
    # Cached dep-info may contain the previous runner's absolute paths.
    # Match only known output basenames; never open paths supplied by it.
    if not regular_nonempty(dep_info):
        return []
    if dep_info.stat().st_size > 8 * 1024 * 1024:
        raise OSError("oversized dep-info")
    text = dep_info.read_text(errors="replace")
    return [name for name in candidates if re.search(r"(?:^|[/ ])" + re.escape(name) + r":", text, re.MULTILINE)]


def raise_io(error: OSError) -> None:
    raise error


def regular_nonempty(path: Path) -> bool:
    try:
        info = path.lstat()
    except FileNotFoundError:
        return False
    return stat.S_ISREG(info.st_mode) and info.st_size > 0


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
