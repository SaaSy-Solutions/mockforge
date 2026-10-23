"""Configuration identity and compatible persistent-target evidence.

Warm evidence changes scheduling only; it never proves source or test correctness.
"""

import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

from rust_policy_runtime import private_directory


def recipe_arguments(arguments):
    # Only diagnostic/offline switches can vary without changing a reviewed recipe.
    return [
        a
        for a in arguments
        if a not in {"--offline", "--locked", "--quiet", "-q", "--message-format=json"}
    ]


def warmth_fingerprint(arguments, repository, worktree, target, environment):
    """Configuration identity, never source freshness or successful-test proof."""
    paths = [
        worktree / "Cargo.lock",
        worktree / "rust-toolchain.toml",
        worktree / "rust-toolchain",
    ]
    for parent in (worktree, *worktree.parents):
        paths.extend(parent / ".cargo" / name for name in ("config", "config.toml"))
    home = Path(environment.get("CARGO_HOME", str(Path.home() / ".cargo")))
    paths.extend(home / name for name in ("config", "config.toml"))
    manifests = subprocess.check_output(
        [
            "git",
            "-C",
            str(worktree),
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
            "--",
            "Cargo.toml",
            "**/Cargo.toml",
        ],
        text=True,
        timeout=5,
    ).split("\0")
    if len(manifests) > 4096:
        raise ValueError("Manifest inventory too large for lightweight classification")
    paths.extend(worktree / name for name in manifests if name)
    compiler = subprocess.check_output(
        [environment.get("RUSTC", "rustc"), "-Vv"],
        env=environment,
        cwd=worktree,
        text=True,
        timeout=5,
    )
    state = {
        "repository": repository,
        "worktree": str(worktree),
        "target": str(target),
        "arguments": recipe_arguments(arguments),
        "compiler": compiler,
        "configuration": {
            str(p): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in paths
            if p.is_file()
        },
        "environment": {
            k: v
            for k, v in environment.items()
            if k
            in {
                "CARGO_HOME",
                "CARGO_BUILD_JOBS",
                "CARGO_BUILD_TARGET",
                "CARGO_INCREMENTAL",
                "CARGO_ENCODED_RUSTFLAGS",
                "CARGO_BUILD_RUSTFLAGS",
                "RUSTFLAGS",
                "RUSTDOCFLAGS",
                "RUSTC",
                "RUSTC_WRAPPER",
                "RUSTC_WORKSPACE_WRAPPER",
                "RUSTUP_TOOLCHAIN",
                "RUSTUP_HOME",
                "CC",
                "CXX",
                "AR",
            }
            or k.startswith(("CARGO_PROFILE_", "CARGO_TARGET_"))
        },
    }
    return hashlib.sha256(json.dumps(state, sort_keys=True).encode()).hexdigest()


def artifact_identity(target):
    directory = target / "debug" / ".fingerprint"
    info = directory.stat()
    if directory.is_symlink() or not directory.is_dir():
        raise ValueError("No native persistent fingerprint directory")
    names = sorted(p.name for p in directory.iterdir())
    if not names:
        raise ValueError("No compiled dependency fingerprints")
    return [
        info.st_dev,
        info.st_ino,
        hashlib.sha256(json.dumps(names).encode()).hexdigest(),
    ]


class WarmEvidence:
    """Optional receipt state for one validation request, independent of leases."""

    def __init__(self):
        self.receipt = None
        self.receipt_context = None

    def configure(
        self, directory, policy, arguments, repository, worktree, target, environment
    ):
        light = False
        recipe = {
            "repository": repository,
            "arguments": recipe_arguments(arguments),
        }
        if not any(
            all(rule[k] == recipe[k] for k in recipe) for rule in policy["light_checks"]
        ):
            return False, "Unknown or unmeasured workload"
        try:
            worktree, target = Path(worktree).resolve(), Path(target).resolve()
            fingerprint = warmth_fingerprint(
                arguments, repository, worktree, target, environment
            )
            receipts = directory / "warm"
            private_directory(receipts)
            self.receipt = receipts / (fingerprint + ".json")
            self.receipt_context = target
            reason = "Measured recipe; cold or expired target evidence"
            # Receipt size/ownership checks are the same as policy input checks.
            if self.receipt.is_file() and not self.receipt.is_symlink():
                info = self.receipt.stat()
                if (
                    info.st_uid != os.getuid()
                    or info.st_mode & 0o022
                    or info.st_size > 512
                ):
                    return False, "Unsafe warm-target receipt"
                data = json.loads(self.receipt.read_text())
                if 0 <= time.time() - data["success_unix"] <= 3600 and data[
                    "artifact_identity"
                ] == artifact_identity(target):
                    light = True
                    reason = "Measured exact recipe and compatible warm-target receipt"
        except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError):
            # Optional scheduling evidence must never make validation unavailable.
            reason = "Warm-target evidence unavailable; conservative admission"

        return light, reason

    def record_success(self):
        if self.receipt is None or self.receipt_context is None:
            return
        temporary = None
        try:
            value = {
                "success_unix": time.time(),
                "artifact_identity": artifact_identity(self.receipt_context),
            }
            fd, temporary = tempfile.mkstemp(prefix="receipt-", dir=self.receipt.parent)
            with os.fdopen(fd, "w") as stream:
                json.dump(value, stream)
            os.replace(temporary, self.receipt)
            temporary = None
        except (OSError, ValueError):
            print(
                "build-policy: warm receipt unavailable; "
                "next check remains conservative",
                file=sys.stderr,
            )
        finally:
            if temporary is not None:
                try:
                    os.unlink(temporary)
                except OSError:
                    pass
