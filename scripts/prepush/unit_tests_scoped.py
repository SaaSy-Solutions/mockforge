#!/usr/bin/env python3
"""Run the tests of the crates with a file in the commits being pushed.

Invoked from the pre-commit `unit-tests` hook at the pre-push stage, with no
file names. The commits are those on no remote. Each file they changed is
mapped to its workspace crate by the `crates/<name>/...` path prefix, and one
`cargo test` runs for the crates changed: the unit tests of their libraries
and binaries, and the integration tests a changed file under a crate's
`tests/` belongs to. A file deeper in that directory may be included by any
of them, so then every integration test of the crates changed runs.

It tests the crates changed, not the crates that depend on them, and not what
is only staged or left in the working tree. Two kinds of change are not scoped
to crates here: a change to the workspace manifest, and a Rust file outside
`crates/`. For those the hook says what it left out and lets the push through,
since CI runs the whole workspace's tests.

To push past the hook once: `SKIP=unit-tests git push`.

usage: unit_tests_scoped.py [--dry-run]
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CARGO = ROOT / "scripts" / "dev" / "cargo"
# Not built where the hooks run: the scoped clippy leaves it out as well.
LEFT_OUT = {"mockforge-desktop"}
ROOT_MANIFESTS = {"Cargo.toml", "Cargo.lock"}
# What `plan` gives for the integration tests when all of them are to run.
EVERY_TEST = None

LEFT_TO_CI = (
    "unit tests: {what} changed, which this hook does not scope to crates; "
    "CI runs the whole workspace's tests"
)


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True)


def commits_being_pushed() -> list[str]:
    """The commits on no remote, oldest first: what this push adds."""
    listed = git("rev-list", "--reverse", "HEAD", "--not", "--remotes")
    return [line for line in listed.splitlines() if line]


def files_changed(commits: list[str]) -> list[str]:
    """The files the commits changed, read from the parent of the oldest to
    HEAD, so that a merge in between is counted once and as a whole."""
    if not commits:
        return []
    oldest = commits[0]
    try:
        start = git("rev-parse", f"{oldest}^").strip()
    except subprocess.CalledProcessError:
        # The oldest commit has no parent: everything it holds is a change.
        start = subprocess.check_output(
            ["git", "hash-object", "-t", "tree", "--stdin"], cwd=ROOT, text=True, input=""
        ).strip()
    listed = git("diff", "--name-only", start, "HEAD")
    return [line for line in listed.splitlines() if line]


def crate_for(path: str) -> str | None:
    """The workspace crate that owns `path`, by the `crates/<name>/` prefix.
    By convention every crate under `crates/` is named after its directory."""
    parts = path.split("/")
    if len(parts) >= 3 and parts[0] == "crates" and parts[1]:
        return parts[1]
    return None


def integration_test_of(path: str) -> str | None:
    """The integration test a file under a crate's `tests/` belongs to, by
    name. An empty name when it is deeper, as any test may include it. None
    for a file that is not under one."""
    parts = path.split("/")
    if len(parts) < 4 or parts[2] != "tests":
        return None
    if len(parts) == 4 and parts[3].endswith(".rs"):
        return parts[3][:-3]
    if len(parts) == 5 and parts[4] == "main.rs":
        return parts[3]
    return ""


def plan(files: list[str]) -> tuple[list[str], list[str] | None, list[str]]:
    """For the files changed: the crates to test; the integration tests to
    include, by name, or None for all of them; and what was left to CI.

    Any file under a crate selects it, not only Rust: a suite or a schema a
    crate includes when it compiles is tested by its tests too.
    """
    crates: set[str] = set()
    tests: set[str] = set()
    every = False
    left: list[str] = []
    for path in files:
        crate = crate_for(path)
        if crate and crate in LEFT_OUT:
            continue
        if crate:
            crates.add(crate)
            test = integration_test_of(path)
            every = every or test == ""
            if test:
                tests.add(test)
        elif path in ROOT_MANIFESTS:
            left.append(f"the workspace manifest (`{path}`)")
        elif path.endswith(".rs"):
            left.append(f"a Rust file outside `crates/` (`{path}`)")
    return sorted(crates), (EVERY_TEST if every else sorted(tests)), left


def test_args(crates: list[str], tests: list[str] | None) -> list[str]:
    """What `cargo test` is given: every crate in one run, so that what they
    compile is compiled once."""
    args: list[str] = []
    for crate in crates:
        args += ["-p", crate]
    if tests is EVERY_TEST:
        # Every target that has tests: the libraries, the binaries and all
        # the integration tests.
        return args + ["--tests"]
    for test in sorted(tests):
        args += ["--test", test]
    return args + ["--lib", "--bins"]


def main(argv: list[str]) -> int:
    if argv not in ([], ["--dry-run"]):
        print(__doc__.split("usage:", 1)[1].strip(), file=sys.stderr)
        return 2
    commits = commits_being_pushed()
    if not commits:
        print("unit tests: no commit is on no remote; nothing to run", flush=True)
        return 0
    files = files_changed(commits)
    crates, tests, left = plan(files)
    for what in left:
        print(LEFT_TO_CI.format(what=what), flush=True)
    if not crates:
        print(
            f"unit tests: none of the {len(files)} changed file(s) is under crates/<name>/; nothing to run",
            flush=True,
        )
        return 0
    if tests is EVERY_TEST:
        what = "every test"
    else:
        what = "the unit tests" + (f" and {', '.join(tests)}" if tests else "")
    print(
        f"unit tests: {len(commits)} commit(s), {len(files)} file(s), "
        f"{len(crates)} crate(s): {', '.join(crates)}; running {what}",
        flush=True,
    )
    command = [str(CARGO), "test", *test_args(crates, tests)]
    print("+ " + " ".join(command), flush=True)
    if argv == ["--dry-run"]:
        return 0
    return subprocess.call(command, cwd=ROOT)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
