#!/usr/bin/env python3
import argparse
import os
import re
import subprocess
import sys
import tomllib
from pathlib import Path


def is_text_file(path: Path) -> bool:
    try:
        data = path.read_bytes()[:8192]
    except OSError:
        return False
    return b"\x00" not in data


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="surrogateescape")


def write_text(path: Path, content: str) -> None:
    path.write_text(content, encoding="utf-8", errors="surrogateescape")


def tracked_existing(paths: list[str]) -> list[Path]:
    out: list[Path] = []
    for raw in paths:
        p = Path(raw)
        if p.exists() and p.is_file():
            out.append(p)
    return out


def run_trailing_whitespace(files: list[Path]) -> int:
    changed = 0
    for path in files:
        if not is_text_file(path):
            continue
        original = read_text(path)
        fixed = re.sub(r"[ \t]+(?=\r?\n|$)", "", original)
        if fixed != original:
            write_text(path, fixed)
            print(f"fixed trailing whitespace: {path}")
            changed += 1
    return 1 if changed else 0


def run_end_of_file_fixer(files: list[Path]) -> int:
    changed = 0
    for path in files:
        if not is_text_file(path):
            continue
        original = read_text(path)
        if original == "":
            continue
        normalized = original.rstrip("\n\r") + "\n"
        if normalized != original:
            write_text(path, normalized)
            print(f"fixed end-of-file newline: {path}")
            changed += 1
    return 1 if changed else 0


def run_mixed_line_ending(files: list[Path]) -> int:
    changed = 0
    for path in files:
        if not is_text_file(path):
            continue
        original = read_text(path)
        fixed = original.replace("\r\n", "\n").replace("\r", "\n")
        if fixed != original:
            write_text(path, fixed)
            print(f"fixed line endings: {path}")
            changed += 1
    return 1 if changed else 0


def run_check_toml(files: list[Path]) -> int:
    failed = 0
    for path in files:
        if path.suffix != ".toml":
            continue
        try:
            tomllib.loads(path.read_text(encoding="utf-8"))
        except Exception as exc:
            print(f"invalid toml: {path}: {exc}")
            failed += 1
    return 1 if failed else 0


def run_check_added_large_files(files: list[Path], max_kb: int) -> int:
    failed = 0
    limit = max_kb * 1024
    for path in files:
        try:
            size = path.stat().st_size
        except OSError:
            continue
        if size > limit:
            print(f"large file: {path} ({size} bytes > {limit} bytes)")
            failed += 1
    return 1 if failed else 0


def run_check_merge_conflict(files: list[Path]) -> int:
    failed = 0
    patterns = ("<<<<<<< ", "=======", ">>>>>>> ")
    for path in files:
        if not is_text_file(path):
            continue
        try:
            lines = read_text(path).splitlines()
        except OSError:
            continue
        if any(any(line.startswith(p) for p in patterns) for line in lines):
            print(f"merge conflict marker: {path}")
            failed += 1
    return 1 if failed else 0


def run_check_case_conflict() -> int:
    proc = subprocess.run(
        ["git", "ls-files"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        print(proc.stderr.strip() or "failed to enumerate tracked files")
        return 1
    seen: dict[str, str] = {}
    failed = 0
    for line in proc.stdout.splitlines():
        k = line.lower()
        prev = seen.get(k)
        if prev and prev != line:
            print(f"case conflict: {prev} <-> {line}")
            failed += 1
        else:
            seen[k] = line
    return 1 if failed else 0


def run_check_executables_have_shebangs(files: list[Path]) -> int:
    failed = 0
    for path in files:
        try:
            mode = path.stat().st_mode
        except OSError:
            continue
        if not (mode & 0o111):
            continue
        try:
            with path.open("rb") as f:
                first = f.read(2)
        except OSError:
            continue
        if first != b"#!":
            print(f"executable missing shebang: {path}")
            failed += 1
    return 1 if failed else 0


def run_check_shebang_scripts_are_executable(files: list[Path]) -> int:
    failed = 0
    for path in files:
        try:
            mode = path.stat().st_mode
        except OSError:
            continue
        try:
            with path.open("rb") as f:
                first = f.read(2)
        except OSError:
            continue
        if first == b"#!" and not (mode & 0o111):
            print(f"shebang file not executable: {path}")
            failed += 1
    return 1 if failed else 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("mode")
    parser.add_argument("--maxkb", type=int, default=500)
    parser.add_argument("files", nargs="*")
    args = parser.parse_args()

    files = tracked_existing(args.files)

    if args.mode == "trailing-whitespace":
        return run_trailing_whitespace(files)
    if args.mode == "end-of-file-fixer":
        return run_end_of_file_fixer(files)
    if args.mode == "mixed-line-ending":
        return run_mixed_line_ending(files)
    if args.mode == "check-toml":
        return run_check_toml(files)
    if args.mode == "check-added-large-files":
        return run_check_added_large_files(files, args.maxkb)
    if args.mode == "check-merge-conflict":
        return run_check_merge_conflict(files)
    if args.mode == "check-case-conflict":
        return run_check_case_conflict()
    if args.mode == "check-executables-have-shebangs":
        return run_check_executables_have_shebangs(files)
    if args.mode == "check-shebang-scripts-are-executable":
        return run_check_shebang_scripts_are_executable(files)

    print(f"unknown mode: {args.mode}")
    return 2


if __name__ == "__main__":
    sys.exit(main())
