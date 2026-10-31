"""Private runtime files, live FIFO tickets and drained policy publication."""

from contextlib import contextmanager
import fcntl
import json
import os
from pathlib import Path
import stat
from rust_policy_config import ADMISSION_CLASSES, default_policy, validate


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
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
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
            fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
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
