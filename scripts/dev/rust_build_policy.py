"""Versioned host admission contract shared verbatim by the four Rust repositories.

No daemon, global shell edits or compiler cache policy. All cooperating clients
read one runtime policy; updates require a drained queue and inherited leases.
"""

from __future__ import annotations

from contextlib import AbstractContextManager
import fcntl
import hashlib
import json
import os
from pathlib import Path
import stat
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
    slots = min(2, cpu // 100)
    return {
        "schema_version": 1,
        "max_concurrent": slots,
        "reserved_checks": 0,
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
        or set(value) != FIELDS
        or value["schema_version"] != 1
    ):
        raise ValueError("Unsupported or malformed build policy")
    integer(value["schema_version"], 1, 1, "schema_version")
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
    return value


def classify(arguments):
    args = list(arguments)
    args = args[: args.index("--")] if "--" in args else args
    profiles = []
    for index, arg in enumerate(args):
        if arg.startswith("--profile="):
            profiles.append(arg.split("=", 1)[1])
        elif arg == "--profile":
            profiles.append(args[index + 1] if index + 1 < len(args) else "unknown")
    if (
        "--release" in args
        or "-r" in args
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
    if not package:
        return "workspace"
    if verb in {"check", "c", "clippy"}:
        return "check"
    if verb in {"test", "t"}:
        return "targeted-test"
    if verb == "bench":
        return "release"
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


class Session(AbstractContextManager):
    """Hold a policy snapshot and, after acquire(), one global build slot."""

    def __init__(self, arguments, directory=None):
        self.directory = directory or runtime_directory()
        private_directory(self.directory)
        self.policy_fd = open_lock(self.directory / "policy.lock")
        self.slot_fd = None
        try:
            fcntl.flock(self.policy_fd, fcntl.LOCK_SH)
            self.policy = read_policy(self.directory)
            self.workload = classify(arguments)
            self.limits = self.policy["classes"][self.workload]
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

    def acquire(self):
        started = time.monotonic()
        slots = self.policy["max_concurrent"]
        reserve = self.policy["reserved_checks"]
        candidates = list(range(1, slots - reserve + 1))
        if self.workload == "check":
            candidates = list(range(slots - reserve + 1, slots + 1)) + candidates
        while self.slot_fd is None:
            for number in candidates:
                fd = open_lock(self.directory / f"slot-{number}")
                try:
                    fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
                except BlockingIOError:
                    os.close(fd)
                else:
                    self.slot_fd = fd
                    break
            if self.slot_fd is None:
                time.sleep(0.1)
        return time.monotonic() - started

    def event(self):
        return {
            "policy_sha256": self.fingerprint,
            "workload_class": self.workload,
            "max_concurrent": self.policy["max_concurrent"],
            "reserved_checks": self.policy["reserved_checks"],
            "limits": dict(self.limits),
        }

    def __exit__(self, *exc):
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
