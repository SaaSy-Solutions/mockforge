"""Versioned shared Cargo admission Session and command-line interface.

Configuration, private runtime IO and warm evidence are cohesive sibling modules.
All cooperating clients retain the same policy lock, queue and inherited leases.
"""

from contextlib import AbstractContextManager
import fcntl
import hashlib
import json
import os
from pathlib import Path
import sys
import tempfile
import time

sys.path.insert(0, str(Path(__file__).resolve().parent))

# Public helpers remain importable for existing policy tooling.
from rust_policy_config import (
    classify,
    default_policy,
    host_budgets as host_budgets,
    validate as validate,
)
import rust_policy_runtime as runtime
from rust_policy_runtime import (
    apply_policy,
    live_waiters,
    open_lock,
    private_directory,
    queue_guard,
    read_policy,
    runtime_directory,
)
from rust_policy_warmth import WarmEvidence


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
        self.warmth = WarmEvidence()
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

    @property
    def receipt(self):
        return self.warmth.receipt

    def configure(self, *, repository, worktree, target, environment):
        if self.slot_fd is not None or self.ticket_fd is not None:
            raise ValueError("Configure workload before queueing")
        self.warmth = WarmEvidence()
        self.admission_class = (
            "STANDARD_CHECK" if self.workload == "check" else "HEAVY_BUILD"
        )
        self.classification_reason = "Unknown or unmeasured workload"
        if self.workload == "check" and self.policy["schema_version"] == 2:
            light, self.classification_reason = self.warmth.configure(
                self.directory,
                self.policy,
                self.arguments,
                repository,
                worktree,
                target,
                environment,
            )
            if light:
                self.admission_class = "LIGHT_CHECK"

    def record_success(self):
        if self.slot_fd is not None:
            self.warmth.record_success()

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
                enough_memory = runtime.available_memory_mib() >= self.policy.get(
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


def main(arguments):
    if arguments == ["example"]:
        print(json.dumps(default_policy(), indent=2))
    elif arguments == ["show"]:
        with Session([]) as session:
            print(json.dumps({"policy": session.policy, **session.event()}, indent=2))
    elif len(arguments) == 2 and arguments[0] == "apply":
        apply_policy(json.loads(Path(arguments[1]).read_text()))
        print(
            "Applied drained host policy; all repositories must use "
            "the versioned contract."
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
