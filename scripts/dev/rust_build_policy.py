"""Versioned host admission contract shared verbatim by the four Rust repositories.

No daemon, global shell edits or compiler cache policy. All cooperating clients
read one runtime policy; updates require a drained queue and inherited leases.
"""

from __future__ import annotations

from contextlib import AbstractContextManager, contextmanager
import fcntl
import hashlib
import json
import os
import re
from pathlib import Path
import stat
import subprocess
import tempfile
import sys
import time

CLASSES = ("check", "targeted-test", "executable", "workspace", "release")
FIELDS = {
    "schema_version",
    "max_concurrent",
    "reserved_checks",
    "memory_budget_mib",
    "cpu_budget_percent",
    "classes",
}
V2_FIELDS = FIELDS | {"light_checks", "minimum_available_mib"}
ADMISSION_CLASSES = {"LIGHT_CHECK", "STANDARD_CHECK", "HEAVY_BUILD"}
LIMITS = {"memory_mib", "cpu_percent", "jobs"}


def host_budgets():
    memory = (
        int(
            next(
                line.split()[1]
                for line in Path("/proc/meminfo").read_text().splitlines()
                if line.startswith("MemTotal:")
            )
        )
        // 1024
    )
    return min(65536, memory * 4 // 5), max(100, len(os.sched_getaffinity(0)) * 80)


def default_policy():
    memory, cpu = host_budgets()
    memory, cpu = min(49152, memory), min(800, cpu)
    slots = max(1, min(4, cpu // 200, memory // 12288))
    return {
        "schema_version": 2,
        "max_concurrent": slots,
        "reserved_checks": max(0, slots - 2),
        "minimum_available_mib": min(16384, max(256, memory // 3)),
        "light_checks": [
            {
                "repository": "SaaSy-Solutions/aether",
                "arguments": ["check", "-p", "founderos-proto", "--lib"],
                "evidence": "saas-platform/docs/development/rust-build-performance-2026-10-22/SSD_RECALIBRATION.md",
            }
        ],
        "memory_budget_mib": memory,
        "cpu_budget_percent": cpu,
        "classes": {
            name: {
                "memory_mib": min(12288, memory // slots),
                "cpu_percent": min(200, cpu // slots),
                "jobs": min(2, len(os.sched_getaffinity(0))),
            }
            for name in CLASSES
        },
    }


def integer(value, low, high, label):
    if type(value) is not int or not low <= value <= high:
        raise ValueError(f"{label} must be an integer in {low}..{high}")


def validate(value):
    if (
        not isinstance(value, dict)
        or value.get("schema_version") not in {1, 2}
        or set(value) != (V2_FIELDS if value.get("schema_version") == 2 else FIELDS)
    ):
        raise ValueError("Unsupported or malformed build policy")
    integer(value["schema_version"], 1, 2, "schema_version")
    integer(value["max_concurrent"], 1, 8, "max_concurrent")
    integer(value["reserved_checks"], 0, value["max_concurrent"] - 1, "reserved_checks")
    memory_budget, cpu_budget = host_budgets()
    integer(value["memory_budget_mib"], 1024, memory_budget, "memory_budget_mib")
    integer(value["cpu_budget_percent"], 100, cpu_budget, "cpu_budget_percent")
    if not isinstance(value["classes"], dict) or set(value["classes"]) != set(CLASSES):
        raise ValueError("Policy must specify every workload class")
    for name, limits in value["classes"].items():
        if not isinstance(limits, dict) or set(limits) != LIMITS:
            raise ValueError(f"Malformed limits for {name}")
        integer(limits["memory_mib"], 256, 32768, "memory_mib")
        integer(limits["cpu_percent"], 100, 800, "cpu_percent")
        integer(limits["jobs"], 1, 8, "jobs")
    for field, budget in (
        ("memory_mib", "memory_budget_mib"),
        ("cpu_percent", "cpu_budget_percent"),
    ):
        if (
            value["max_concurrent"]
            * max(item[field] for item in value["classes"].values())
            > value[budget]
        ):
            raise ValueError(f"Worst-case concurrent {field} exceeds {budget}")
    if value["schema_version"] == 2:
        validate_recipes(value, memory_budget)
    return value


def validate_recipes(value, memory_budget):
    integer(value["minimum_available_mib"], 128, memory_budget, "minimum_available_mib")
    rules = value["light_checks"]
    if not isinstance(rules, list) or len(rules) > 64:
        raise ValueError("light_checks must be a bounded list of measured recipes")
    for rule in rules:
        if not isinstance(rule, dict) or set(rule) != {
            "repository",
            "arguments",
            "evidence",
        }:
            raise ValueError("Malformed measured recipe")
        if not all(
            isinstance(rule[k], str) and 1 <= len(rule[k]) <= 512
            for k in ("repository", "evidence")
        ):
            raise ValueError(
                "Recipe requires repository identity and measurement evidence"
            )
        args = rule["arguments"]
        if (
            not isinstance(args, list)
            or not 3 <= len(args) <= 32
            or not all(isinstance(a, str) and 0 < len(a) <= 256 for a in args)
            or args[:2] != ["check", "-p"]
            or not re.fullmatch(r"[A-Za-z0-9_-]+", args[2])
            or "--lib" not in args
            or any(
                a
                in {
                    "-p",
                    "--package",
                    "--all-targets",
                    "--bins",
                    "--tests",
                    "--examples",
                    "--benches",
                    "--bin",
                    "--test",
                    "--example",
                    "--bench",
                    "--target",
                }
                or a.startswith(
                    (
                        "--package=",
                        "--target=",
                        "--bin=",
                        "--test=",
                        "--example=",
                        "--bench=",
                    )
                )
                for a in args[3:]
            )
            or classify(args) != "check"
        ):
            raise ValueError("Only explicit targeted check recipes can be lightweight")


def classify(arguments):
    args = list(arguments)
    args = args[: args.index("--")] if "--" in args else args
    verb = next(
        (
            a
            for a in args
            if a
            in {
                "check",
                "c",
                "clippy",
                "test",
                "t",
                "build",
                "b",
                "run",
                "r",
                "bench",
                "doc",
                "rustc",
                "rustdoc",
            }
        ),
        "",
    )
    profiles = []
    for index, arg in enumerate(args):
        if arg.startswith("--profile="):
            profiles.append(arg.split("=", 1)[1])
        elif arg == "--profile":
            profiles.append(args[index + 1] if index + 1 < len(args) else "unknown")
    if (
        "--release" in args
        or "-r" in args
        or verb == "bench"
        or any(p not in {"dev", "test"} for p in profiles)
    ):
        return "release"
    if any(a in {"--workspace", "--all", "--all-features"} for a in args):
        return "workspace"
    package = any(
        a in {"-p", "--package"}
        or a.startswith("--package=")
        or (a.startswith("-p") and len(a) > 2)
        for a in args
    )
    if not package:
        return "workspace"
    if verb in {"check", "c", "clippy"}:
        return "check"
    if verb in {"test", "t"}:
        return "targeted-test"
    return "executable"


def runtime_directory():
    value = Path(os.environ.get("XDG_RUNTIME_DIR") or f"/run/user/{os.getuid()}")
    if not value.is_absolute():
        raise ValueError("XDG_RUNTIME_DIR must be absolute")
    return value / "saasy-dev-cargo-admission"


def private_directory(path):
    for parent in (path, *path.parents):
        if parent.is_symlink():
            raise ValueError("Admission paths must not contain symlinks")
    if not path.is_absolute():
        raise ValueError("Admission directory must be absolute")
    path.mkdir(parents=True, mode=0o700, exist_ok=True)
    mode = path.stat()
    if (
        not stat.S_ISDIR(mode.st_mode)
        or mode.st_uid != os.getuid()
        or mode.st_mode & 0o022
    ):
        raise ValueError("Admission directory must be owned and not writable by others")


def open_lock(path):
    descriptor = os.open(path, os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    mode = os.fstat(descriptor)
    if (
        not stat.S_ISREG(mode.st_mode)
        or mode.st_uid != os.getuid()
        or mode.st_mode & 0o022
        or mode.st_nlink != 1
    ):
        os.close(descriptor)
        raise ValueError("Unsafe admission lock")
    return descriptor


def read_policy(directory):
    path = directory / "policy.json"
    try:
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    except FileNotFoundError:
        return validate(default_policy())
    with os.fdopen(fd) as stream:
        mode = os.fstat(stream.fileno())
        if (
            not stat.S_ISREG(mode.st_mode)
            or mode.st_uid != os.getuid()
            or mode.st_mode & 0o022
            or mode.st_nlink != 1
            or mode.st_size > 65536
        ):
            raise ValueError("Unsafe admission policy file")
        return validate(json.load(stream))


def available_memory_mib():
    return (
        int(
            next(
                line.split()[1]
                for line in Path("/proc/meminfo").read_text().splitlines()
                if line.startswith("MemAvailable:")
            )
        )
        // 1024
    )


@contextmanager
def queue_guard(directory):
    descriptor = open_lock(directory / "queue.lock")
    try:
        fcntl.flock(descriptor, fcntl.LOCK_EX)
        yield
    finally:
        os.close(descriptor)


def live_waiters(directory):
    """An owned flock, not a PID or age, proves a waiting request is alive."""
    result = []
    for path in sorted(directory.iterdir()):
        try:
            fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
        except FileNotFoundError:
            continue
        with os.fdopen(fd) as stream:
            info = os.fstat(stream.fileno())
            if (
                not stat.S_ISREG(info.st_mode)
                or info.st_uid != os.getuid()
                or info.st_mode & 0o022
                or info.st_nlink != 1
                or info.st_size > 512
            ):
                raise ValueError("Unsafe queue ticket")
            try:
                fcntl.flock(stream.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError:
                kind = json.load(stream)
                if kind not in ADMISSION_CLASSES:
                    raise ValueError("Invalid live queue class")
                result.append((path.name, kind))
            # Dead tickets are ignored, never reaped by deleting another process's file.
    return result


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


class Session(AbstractContextManager):
    """Hold a policy snapshot and, after acquire(), one global build slot."""

    def __init__(self, arguments, directory=None):
        self.directory = directory or runtime_directory()
        private_directory(self.directory)
        self.policy_fd = open_lock(self.directory / "policy.lock")
        self.slot_fd = None
        self.slot_number = None
        self.pressure_wait_seconds = 0.0
        self.ticket_fd = None
        self.ticket_path = None
        self.receipt = None
        self.receipt_context = None
        self.arguments = list(arguments)
        try:
            fcntl.flock(self.policy_fd, fcntl.LOCK_SH)
            self.policy = read_policy(self.directory)
            self.workload = classify(arguments)
            self.limits = self.policy["classes"][self.workload]
            self.admission_class = (
                "STANDARD_CHECK" if self.workload == "check" else "HEAVY_BUILD"
            )
            self.classification_reason = "No measured warm-target recipe"
            self.fingerprint = hashlib.sha256(
                json.dumps(self.policy, sort_keys=True).encode()
            ).hexdigest()
        except BaseException:
            os.close(self.policy_fd)
            raise

    @property
    def descriptors(self):
        return tuple(fd for fd in (self.policy_fd, self.slot_fd) if fd is not None)

    def properties(self):
        return [
            f"MemoryMax={self.limits['memory_mib']}M",
            "MemorySwapMax=0",
            f"CPUQuota={self.limits['cpu_percent']}%",
        ]

    def configure(self, *, repository, worktree, target, environment):
        if self.slot_fd is not None or self.ticket_fd is not None:
            raise ValueError("Configure workload before queueing")
        self.receipt = self.receipt_context = None
        self.admission_class = (
            "STANDARD_CHECK" if self.workload == "check" else "HEAVY_BUILD"
        )
        self.classification_reason = "Unknown or unmeasured workload"
        if self.workload != "check" or self.policy["schema_version"] != 2:
            return
        recipe = {
            "repository": repository,
            "arguments": recipe_arguments(self.arguments),
        }
        if not any(
            all(rule[k] == recipe[k] for k in recipe)
            for rule in self.policy["light_checks"]
        ):
            return
        try:
            worktree, target = Path(worktree).resolve(), Path(target).resolve()
            fingerprint = warmth_fingerprint(
                self.arguments, repository, worktree, target, environment
            )
            receipts = self.directory / "warm"
            private_directory(receipts)
            self.receipt = receipts / (fingerprint + ".json")
            self.receipt_context = target
            self.classification_reason = (
                "Measured recipe; cold or expired target evidence"
            )
            # Receipt size/ownership checks are the same as policy input checks.
            if self.receipt.is_file() and not self.receipt.is_symlink():
                info = self.receipt.stat()
                if (
                    info.st_uid != os.getuid()
                    or info.st_mode & 0o022
                    or info.st_size > 512
                ):
                    return
                data = json.loads(self.receipt.read_text())
                if 0 <= time.time() - data["success_unix"] <= 3600 and data[
                    "artifact_identity"
                ] == artifact_identity(target):
                    self.admission_class = "LIGHT_CHECK"
                    self.classification_reason = (
                        "Measured exact recipe and compatible warm-target receipt"
                    )
        except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError):
            # Optional scheduling evidence must never make validation unavailable.
            self.classification_reason = (
                "Warm-target evidence unavailable; conservative admission"
            )

    def record_success(self):
        if self.receipt is None or self.receipt_context is None or self.slot_fd is None:
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
                "build-policy: warm receipt unavailable; next check remains conservative",
                file=sys.stderr,
            )
        finally:
            if temporary is not None:
                try:
                    os.unlink(temporary)
                except OSError:
                    pass

    def _remove_ticket(self):
        if self.ticket_path is not None:
            self.ticket_path.unlink(missing_ok=True)
            self.ticket_path = None
        if self.ticket_fd is not None:
            os.close(self.ticket_fd)
            self.ticket_fd = None

    def acquire(self):
        started = time.monotonic()
        slots = self.policy["max_concurrent"]
        ordinary = slots - self.policy["reserved_checks"]
        candidates = list(range(1, ordinary + 1))
        if self.admission_class == "LIGHT_CHECK":
            candidates = list(range(ordinary + 1, slots + 1)) + candidates
        queue = self.directory / "queue"
        private_directory(queue)
        with queue_guard(self.directory):
            self.ticket_fd, name = tempfile.mkstemp(
                prefix=f"{time.monotonic_ns():020d}-", dir=queue
            )
            self.ticket_path = Path(name)
            fcntl.flock(self.ticket_fd, fcntl.LOCK_EX)
            os.write(self.ticket_fd, json.dumps(self.admission_class).encode())
        while self.slot_fd is None:
            iteration_started = time.monotonic()
            with queue_guard(self.directory):
                waiting = live_waiters(queue)
                enough_memory = available_memory_mib() >= self.policy.get(
                    "minimum_available_mib", 0
                )
                for number in candidates if enough_memory else []:
                    earlier = any(
                        name < self.ticket_path.name
                        and (number <= ordinary or kind == "LIGHT_CHECK")
                        for name, kind in waiting
                    )
                    if earlier:
                        continue
                    fd = open_lock(self.directory / f"slot-{number}")
                    try:
                        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    except BlockingIOError:
                        os.close(fd)
                    else:
                        self.slot_fd = fd
                        self.slot_number = number
                        self._remove_ticket()
                        break
            if self.slot_fd is None:
                time.sleep(0.1)
            if not enough_memory:
                self.pressure_wait_seconds += time.monotonic() - iteration_started
        return time.monotonic() - started

    def event(self):
        return {
            "policy_sha256": self.fingerprint,
            "workload_class": self.workload,
            "admission_class": self.admission_class,
            "admission_slot": self.slot_number,
            "pressure_wait_seconds": self.pressure_wait_seconds,
            "classification_reason": self.classification_reason,
            "minimum_available_mib": self.policy.get("minimum_available_mib", 0),
            "max_concurrent": self.policy["max_concurrent"],
            "reserved_checks": self.policy["reserved_checks"],
            "limits": dict(self.limits),
        }

    def __exit__(self, *exc):
        self._remove_ticket()
        for fd in self.descriptors:
            os.close(fd)
        self.policy_fd = self.slot_fd = None


def apply_policy(value, directory=None):
    """Fail rather than mutate capacity while any queue/lease or legacy slot lives."""
    validate(value)
    directory = directory or runtime_directory()
    private_directory(directory)
    gate = open_lock(directory / "policy.lock")
    locks = []
    temporary = None
    try:
        try:
            fcntl.flock(gate, fcntl.LOCK_EX | fcntl.LOCK_NB)
            for number in range(1, 10):
                descriptor = open_lock(directory / f"slot-{number}")
                locks.append(descriptor)
                fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise ValueError(
                "Drain active and queued builds before changing policy"
            ) from None
        # Unique owned file; no overwrite of a prior partial update.
        import tempfile

        descriptor, temporary = tempfile.mkstemp(
            prefix="policy-", suffix=".json", dir=directory
        )
        with os.fdopen(descriptor, "w") as stream:
            json.dump(value, stream, sort_keys=True)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, directory / "policy.json")
        temporary = None
    finally:
        if temporary is not None:
            os.unlink(temporary)
        for descriptor in locks:
            os.close(descriptor)
        os.close(gate)


def main(arguments):
    if arguments == ["example"]:
        print(json.dumps(default_policy(), indent=2))
    elif arguments == ["show"]:
        with Session([]) as session:
            print(json.dumps({"policy": session.policy, **session.event()}, indent=2))
    elif len(arguments) == 2 and arguments[0] == "apply":
        apply_policy(json.loads(Path(arguments[1]).read_text()))
        print(
            "Applied drained host policy; all repositories must use the versioned contract."
        )
    else:
        raise ValueError(
            "Usage: rust_build_policy.py example | show | apply REVIEWED.json"
        )


if __name__ == "__main__":
    try:
        main(sys.argv[1:])
    except (OSError, ValueError) as error:
        print(f"build-policy: {error}", file=sys.stderr)
        sys.exit(64)
