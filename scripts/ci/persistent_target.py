#!/usr/bin/env python3
"""Select a persistent per-runner Cargo target directory (#1449).

The self-hosted runners keep their disks between jobs, but actions/checkout
cleans `target/` inside the workspace and rust-cache never restored (per-runner
CARGO_HOME split every key and PR entries filled the 10 GB repository cache).
Each runner instead keeps one target directory outside the workspace:

    /tmp/mockforge-target-<runner name>

A runner executes one job at a time, so the directory is never shared by two
concurrent Cargo processes. The only directory this script ever removes is that
exact path, and only when it has grown past the size budget or the disk is short
on headroom; the next build then starts cold instead of filling the shared disk.
The `release` action runs at the end of every job (even after failure) and
drops the directory when the shared disk is short, so this repository's cache
gives way before the host runs out of space.

"Short" means less free space than a cold build needs. Several runners share
one host disk, and a cold workspace build on a sibling runner writes well over
30 GiB: with a 30 GiB threshold, run 41271675577 kept a 23 GiB target at
30.9 GiB free and the next cold build on the sibling runner hit ENOSPC.
"""
import argparse
import os
from pathlib import Path
import re
import shutil
import sys

GIB = 1024 ** 3
ROOT = Path("/tmp")
RUNNER = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,99}$")


def target_for(runner_name: str, root: Path = ROOT) -> Path:
    if not RUNNER.match(runner_name or ""):
        raise ValueError("unexpected runner name")
    return root / ("mockforge-target-" + runner_name)


def tree_size(path: Path) -> int:
    total = 0
    for directory, dirnames, filenames in os.walk(path, followlinks=False):
        for name in filenames:
            try:
                info = os.lstat(os.path.join(directory, name))
            except FileNotFoundError:
                continue
            total += info.st_blocks * 512
    return total


def prepare(target: Path, max_bytes: int, min_free: int) -> str:
    if target.is_symlink() or (target.exists() and not target.is_dir()):
        raise ValueError("target path is not a plain directory: " + str(target))
    reason = None
    if target.exists():
        size = tree_size(target)
        free = shutil.disk_usage(target.parent).free
        print(f"Persistent target {target}: {size / GIB:.1f} GiB used, {free / GIB:.1f} GiB free")
        if size > max_bytes:
            reason = f"over the {max_bytes // GIB} GiB budget"
        elif free < min_free:
            reason = f"less than {min_free // GIB} GiB free on the disk"
        if reason:
            print(f"Resetting {target}: {reason}")
            shutil.rmtree(target)
    target.mkdir(mode=0o700, exist_ok=True)
    return reason or "reused"


def release(target: Path, min_free: int) -> str:
    if target.is_symlink() or not target.is_dir():
        return "absent"
    free = shutil.disk_usage(target.parent).free
    if free >= min_free:
        print(f"Keeping {target}: {free / GIB:.1f} GiB free")
        return "kept"
    print(f"Releasing {target}: only {free / GIB:.1f} GiB free on the shared disk")
    shutil.rmtree(target)
    return "released"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("action", nargs="?", choices=("prepare", "release"), default="prepare")
    parser.add_argument("--runner-name", default=os.environ.get("RUNNER_NAME", ""))
    parser.add_argument("--max-gib", type=int, default=40)
    # Headroom for a cold build on a sibling runner sharing the disk.
    parser.add_argument("--min-free-gib", type=int, default=50)
    args = parser.parse_args()
    target = target_for(args.runner_name)
    if args.action == "release":
        release(target, args.min_free_gib * GIB)
        return 0
    prepare(target, args.max_gib * GIB, args.min_free_gib * GIB)
    env = os.environ.get("GITHUB_ENV")
    if env:
        with open(env, "a") as handle:
            handle.write(f"CARGO_TARGET_DIR={target}\n")
    print(f"CARGO_TARGET_DIR={target}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
