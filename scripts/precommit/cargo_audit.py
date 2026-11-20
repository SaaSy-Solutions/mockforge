#!/usr/bin/env python3
"""Run `cargo audit` using audit.toml as the single source of truth for
ignored advisories.

The cargo-audit CLI has no native config-file support, so the pre-commit
hook previously duplicated the ignore list inline. This wrapper reads
audit.toml once and forwards each id as `--ignore`. Any arguments passed
to this script are appended to the cargo-audit invocation unchanged.

With `--all-lockfiles` (CI mode) every Cargo.lock tracked by git is
covered, not just the root workspace's:

1. `git ls-files "*Cargo.lock"` enumerates the tracked lockfiles.
2. `cargo locate-project --workspace` decides for each lockfile whether
   its directory is a standalone workspace root (audited) or a member of
   another workspace. A member lockfile is stray — cargo only honours a
   lockfile at the workspace root — so the run fails and asks for its
   deletion.
3. `cargo metadata --locked` proves each standalone lockfile is current;
   a stale lock fails the run naming the lockfile.
4. `cargo audit --file <lock> --ignore <id>...` audits each standalone
   lockfile with the same audit.toml ignores.

Every lockfile is checked before the run exits, so one run reports all
problems. All paths are interpreted relative to the repository root; run
the script from the root, as pre-commit and CI already do.
"""
from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import NamedTuple


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


class CommandResult(NamedTuple):
    status: int
    stdout: str
    stderr: str


def run_command(cmd: list[str]) -> CommandResult:
    """Run `cmd` and capture its output.

    Every git/cargo call of the --all-lockfiles path goes through this one
    function so tests can replace it and drive the whole logic without
    git or cargo installed.
    """
    completed = subprocess.run(cmd, capture_output=True, text=True, errors="replace")
    return CommandResult(completed.returncode, completed.stdout, completed.stderr)


def emit(text: str) -> None:
    """Print captured command output, keeping the line structure."""
    if not text:
        return
    sys.stdout.write(text if text.endswith("\n") else text + "\n")


def list_lockfiles() -> list[str]:
    """Every tracked Cargo.lock, as repository-root-relative paths."""
    result = run_command(["git", "ls-files", "*Cargo.lock"])
    if result.status != 0:
        raise RuntimeError(
            f"git ls-files failed with exit {result.status}: {result.stderr.strip()}"
        )
    return [line.strip() for line in result.stdout.splitlines() if line.strip()]


def workspace_root(manifest: Path, cargo: str) -> Path | None:
    """Root manifest of the workspace owning `manifest`, or None on error.

    `cargo locate-project --workspace` prints that root manifest, which is
    `manifest` itself when its directory is a standalone workspace.
    """
    result = run_command([
        cargo, "locate-project", "--workspace",
        "--message-format", "plain", "--manifest-path", str(manifest),
    ])
    if result.status != 0 or not result.stdout.strip():
        return None
    return Path(result.stdout.strip()).resolve()


def audit_all_lockfiles(extra_args: list[str]) -> int:
    """Audit every tracked standalone Cargo.lock; returns an exit status."""
    cargo = resolve_cargo()
    ids = load_ignored_ids(Path("audit.toml"))
    try:
        lockfiles = list_lockfiles()
    except RuntimeError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"Security audit of {len(lockfiles)} tracked Cargo.lock files")
    summary: list[tuple[str, str]] = []
    failed = 0
    for lockfile in lockfiles:
        manifest = Path(lockfile).parent / "Cargo.toml"
        tags: list[str] = []
        findings: list[str] = []
        root = workspace_root(manifest, cargo)
        if root is None:
            tags.append("error")
            findings.append("cannot determine its workspace (cargo locate-project failed)")
        elif root != manifest.resolve():
            tags.append("stray")
            findings.append(f"belongs to workspace {root}; cargo ignores it — delete it")
        else:
            metadata = run_command([
                cargo, "metadata", "--locked", "--format-version", "1",
                "--manifest-path", str(manifest),
            ])
            if metadata.status != 0:
                tags.append("stale")
                findings.append(
                    f"stale lock (cargo metadata --locked failed; the lockfile "
                    f"does not match {manifest})"
                )
                emit(metadata.stderr)
            audit_cmd = [cargo, "audit", "--file", lockfile]
            for adv_id in ids:
                audit_cmd.extend(["--ignore", adv_id])
            audit_cmd.extend(extra_args)
            audit = run_command(audit_cmd)
            emit(audit.stdout)
            emit(audit.stderr)
            if audit.status != 0:
                tags.append("vulnerable")
                findings.append(f"vulnerable (cargo audit exit {audit.status})")
        status = ",".join(tags) if tags else "ok"
        print(f"audit: {lockfile} " + ("; ".join(findings) or "ok"))
        summary.append((lockfile, status))
        if tags:
            failed += 1
    width = max((len(status) for _, status in summary), default=2)
    print("\nLockfile summary:")
    for lockfile, status in summary:
        print(f"  {status:<{width}}  {lockfile}")
    if failed:
        print(f"\n{failed} of {len(lockfiles)} tracked lockfiles failed the audit.")
        return 1
    print(f"\nAll {len(lockfiles)} tracked lockfiles audited clean.")
    return 0


def main(argv: list[str] | None = None) -> int:
    args = sys.argv[1:] if argv is None else list(argv)
    if "--all-lockfiles" in args:
        extra = [arg for arg in args if arg != "--all-lockfiles"]
        return audit_all_lockfiles(extra)
    ids = load_ignored_ids(Path("audit.toml"))
    cmd = [resolve_cargo(), "audit"]
    for adv_id in ids:
        cmd.extend(["--ignore", adv_id])
    cmd.extend(args)
    return subprocess.call(cmd)


if __name__ == "__main__":
    sys.exit(main())
