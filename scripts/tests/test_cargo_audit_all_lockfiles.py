"""Offline unit tests for the --all-lockfiles audit mode; no git or cargo run."""

import importlib.util
import io
import os
from contextlib import redirect_stdout
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "cargo_audit", ROOT / "scripts/precommit/cargo_audit.py")
CARGO_AUDIT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CARGO_AUDIT)

STANDALONE_LOCKS = [
    "Cargo.lock",
    "crates/mockforge-core/fuzz/Cargo.lock",
    "examples/plugins/template-advanced/Cargo.lock",
    "examples/plugins/webhook-example/Cargo.lock",
    "scripts/security/sigv4a-sdk-tests/Cargo.lock",
    "vendor/aws-sigv4/Cargo.lock",
]


def standalone_roots(lockfiles):
    """locate-project answers: every given lockfile dir is its own workspace."""
    return {
        str(Path(lockfile).parent / "Cargo.toml"): str(Path(lockfile).parent / "Cargo.toml")
        for lockfile in lockfiles
    }


class FakeRunner:
    """Answers the script's run_command calls and records every command."""

    def __init__(self, lockfiles, roots, metadata_failures=(), audit_failures=None):
        self.lockfiles = list(lockfiles)
        self.roots = dict(roots)
        self.metadata_failures = set(metadata_failures)
        self.audit_failures = dict(audit_failures or {})
        self.commands = []

    def __call__(self, cmd):
        self.commands.append(cmd)
        if cmd[:2] == ["git", "ls-files"]:
            return CARGO_AUDIT.CommandResult(
                0, "".join(path + "\n" for path in self.lockfiles), "")
        if "locate-project" in cmd:
            manifest = cmd[cmd.index("--manifest-path") + 1]
            if manifest not in self.roots:
                return CARGO_AUDIT.CommandResult(
                    1, "", f"error: manifest `{manifest}` does not exist")
            return CARGO_AUDIT.CommandResult(0, self.roots[manifest], "")
        if "metadata" in cmd:
            manifest = cmd[cmd.index("--manifest-path") + 1]
            if manifest in self.metadata_failures:
                return CARGO_AUDIT.CommandResult(
                    101, "", "error: the lock file needs to be updated but --locked was passed")
            return CARGO_AUDIT.CommandResult(0, "{}", "")
        if cmd[1] == "audit":
            lockfile = cmd[cmd.index("--file") + 1]
            status = self.audit_failures.get(lockfile, 0)
            detail = "vulnerabilities found" if status else "no vulnerabilities"
            return CARGO_AUDIT.CommandResult(status, detail, "")
        raise AssertionError(f"unexpected command: {cmd!r}")


class AllLockfilesTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="cargo-audit-all ")
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.root.joinpath("audit.toml").write_text(
            "[advisories]\n"
            "ignore = [\n"
            '  { id = "RUSTSEC-2026-0001" },\n'
            '  "RUSTSEC-2026-0002",\n'
            "]\n"
        )
        self.previous = Path.cwd()
        os.chdir(self.root)
        self.addCleanup(os.chdir, self.previous)

    def audit(self, runner, args=("--all-lockfiles",)):
        with patch.object(CARGO_AUDIT, "run_command", runner), \
                patch.object(CARGO_AUDIT, "resolve_cargo", return_value="cargo"), \
                redirect_stdout(io.StringIO()) as out:
            status = CARGO_AUDIT.main(list(args))
        return status, out.getvalue()

    def audited_paths(self, runner):
        return [cmd[cmd.index("--file") + 1]
                for cmd in runner.commands if cmd[1] == "audit"]

    def test_all_clean_lockfiles_pass(self):
        runner = FakeRunner(STANDALONE_LOCKS, standalone_roots(STANDALONE_LOCKS))
        status, out = self.audit(runner)
        self.assertEqual(status, 0)
        for lockfile in STANDALONE_LOCKS:
            self.assertIn(f"audit: {lockfile} ok", out)
        self.assertIn("All 6 tracked lockfiles audited clean.", out)
        audits = [cmd for cmd in runner.commands if cmd[1] == "audit"]
        self.assertEqual(len(audits), len(STANDALONE_LOCKS))
        for lockfile, cmd in zip(STANDALONE_LOCKS, audits):
            self.assertEqual(cmd, [
                "cargo", "audit", "--file", lockfile,
                "--ignore", "RUSTSEC-2026-0001",
                "--ignore", "RUSTSEC-2026-0002",
            ])
        metadatas = [cmd for cmd in runner.commands if "metadata" in cmd]
        self.assertEqual(len(metadatas), len(STANDALONE_LOCKS))
        for cmd in metadatas:
            self.assertIn("--locked", cmd)

    def test_stray_member_lockfile_is_refused(self):
        stray = "examples/plugins/stray/Cargo.lock"
        lockfiles = ["Cargo.lock", stray, "vendor/aws-sigv4/Cargo.lock"]
        roots = standalone_roots(["Cargo.lock", "vendor/aws-sigv4/Cargo.lock"])
        roots[str(Path(stray).parent / "Cargo.toml")] = "Cargo.toml"
        runner = FakeRunner(lockfiles, roots)
        status, out = self.audit(runner)
        self.assertEqual(status, 1)
        self.assertIn(
            f"audit: {stray} belongs to workspace {Path('Cargo.toml').resolve()}", out)
        self.assertIn("cargo ignores it — delete it", out)
        self.assertIn(stray, out.split("Lockfile summary:", 1)[1])
        # Refused means not audited; the two real standalone locks still are.
        self.assertEqual(self.audited_paths(runner),
                         ["Cargo.lock", "vendor/aws-sigv4/Cargo.lock"])
        self.assertIn("1 of 3 tracked lockfiles failed the audit.", out)

    def test_stale_standalone_lock_fails_the_run(self):
        stale = "scripts/security/sigv4a-sdk-tests/Cargo.lock"
        lockfiles = ["Cargo.lock", stale, "vendor/aws-sigv4/Cargo.lock"]
        runner = FakeRunner(lockfiles, standalone_roots(lockfiles),
                            metadata_failures=["scripts/security/sigv4a-sdk-tests/Cargo.toml"])
        status, out = self.audit(runner)
        self.assertEqual(status, 1)
        self.assertIn(f"audit: {stale} stale lock (cargo metadata --locked failed", out)
        self.assertIn("the lock file needs to be updated", out)
        self.assertEqual(self.audited_paths(runner), lockfiles)
        self.assertIn("1 of 3 tracked lockfiles failed the audit.", out)

    def test_vulnerability_in_non_root_lockfile_fails_the_run(self):
        # Negative control for #1631: the root-only audit this replaces passes
        # here because cargo audit never looks at the example plugin lockfile.
        infected = "examples/plugins/webhook-example/Cargo.lock"
        lockfiles = ["Cargo.lock", "crates/mockforge-core/fuzz/Cargo.lock",
                     infected, "vendor/aws-sigv4/Cargo.lock"]
        runner = FakeRunner(lockfiles, standalone_roots(lockfiles),
                            audit_failures={infected: 1})
        status, out = self.audit(runner)
        self.assertEqual(status, 1)
        self.assertIn(f"audit: {infected} vulnerable (cargo audit exit 1)", out)
        self.assertIn("vulnerabilities found", out)
        self.assertIn("audit: Cargo.lock ok", out)
        self.assertEqual(self.audited_paths(runner), lockfiles)
        self.assertIn("1 of 4 tracked lockfiles failed the audit.", out)

    def test_every_problem_is_reported_in_one_run(self):
        stray = "examples/plugins/stray/Cargo.lock"
        stale = "scripts/security/sigv4a-sdk-tests/Cargo.lock"
        vulnerable = "crates/mockforge-core/fuzz/Cargo.lock"
        lockfiles = ["Cargo.lock", stale, stray, vulnerable, "vendor/aws-sigv4/Cargo.lock"]
        roots = standalone_roots(["Cargo.lock", stale, vulnerable, "vendor/aws-sigv4/Cargo.lock"])
        roots[str(Path(stray).parent / "Cargo.toml")] = "Cargo.toml"
        runner = FakeRunner(lockfiles, roots,
                            metadata_failures=["scripts/security/sigv4a-sdk-tests/Cargo.toml"],
                            audit_failures={vulnerable: 2})
        status, out = self.audit(runner)
        self.assertEqual(status, 1)
        for expected in (
            f"audit: {stray} belongs to workspace",
            f"audit: {stale} stale lock",
            f"audit: {vulnerable} vulnerable (cargo audit exit 2)",
            "audit: Cargo.lock ok",
        ):
            self.assertIn(expected, out)
        self.assertIn("3 of 5 tracked lockfiles failed the audit.", out)

    def test_unclassifiable_lockfile_fails_the_run(self):
        broken = "crates/mockforge-core/fuzz/Cargo.lock"
        runner = FakeRunner(["Cargo.lock", broken], standalone_roots(["Cargo.lock"]))
        status, out = self.audit(runner)
        self.assertEqual(status, 1)
        self.assertIn(f"audit: {broken} cannot determine its workspace", out)
        self.assertIn("1 of 2 tracked lockfiles failed the audit.", out)

    def test_without_the_flag_only_the_root_workspace_is_audited(self):
        def fail(_cmd):
            raise AssertionError("run_command must not be used without --all-lockfiles")
        with patch.object(CARGO_AUDIT, "resolve_cargo", return_value="cargo"), \
                patch.object(CARGO_AUDIT, "run_command", fail), \
                patch.object(CARGO_AUDIT.subprocess, "call", return_value=0) as call:
            status = CARGO_AUDIT.main(["--file", "vendor/aws-sigv4/Cargo.lock"])
        self.assertEqual(status, 0)
        call.assert_called_once_with([
            "cargo", "audit",
            "--ignore", "RUSTSEC-2026-0001",
            "--ignore", "RUSTSEC-2026-0002",
            "--file", "vendor/aws-sigv4/Cargo.lock",
        ])
        with patch.object(CARGO_AUDIT, "resolve_cargo", return_value="cargo"), \
                patch.object(CARGO_AUDIT, "run_command", fail), \
                patch.object(CARGO_AUDIT.subprocess, "call", return_value=1):
            self.assertEqual(CARGO_AUDIT.main([]), 1)


if __name__ == "__main__":
    unittest.main()
