#!/usr/bin/env python3
"""Record and stop only an exact Linux CI fixture process, never a name/port.

The start-time receipt rejects stale/reused PIDs. A pidfd pins the identity while
we validate it and signal it, closing the remaining check-to-kill reuse race.
Missing pidfd support fails closed; no process-name or port fallback is allowed.
"""

import argparse
import json
import os
from pathlib import Path
import select
import signal
import subprocess
import sys


def process_context() -> dict[str, str]:
    proc_pid = int(Path("/proc/self/stat").read_text().split(" ", 1)[0])
    if proc_pid != os.getpid():
        raise RuntimeError("procfs does not match this process PID namespace")
    return {"boot_id": Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
            "pid_namespace": os.readlink("/proc/self/ns/pid")}


def start_time(pid: int) -> int:
    if pid <= 1:
        raise ValueError("A fixture PID must be greater than 1")
    # comm (field 2) may contain spaces or parentheses. Everything after its
    # final ') ' starts at field 3; starttime is field 22.
    stat = Path(f"/proc/{pid}/stat").read_text()
    return int(stat.rsplit(") ", 1)[1].split()[19])


def record(pid: int, receipt: Path) -> None:
    if not hasattr(os, "pidfd_open") or not hasattr(signal, "pidfd_send_signal"):
        raise RuntimeError("Linux pidfd support is required for owned cleanup")
    ticks = start_time(pid)
    fd = os.pidfd_open(pid)
    try:
        if start_time(pid) != ticks or select.select([fd], [], [], 0)[0]:
            raise ProcessLookupError("Fixture exited while recording its identity")
        identity = {"version": 1, "pid": pid, "start_time": ticks,
                    "context": process_context()}
        # The receipt lives in this job's unique fixture directory.
        with receipt.open("x") as stream:
            json.dump(identity, stream)
    finally:
        os.close(fd)


class StartupInterrupted(Exception):
    pass


def start(receipt: Path, log: Path, command: list[str]) -> int:
    """Own spawn and receipt creation together; roll back our child on failure."""
    if not command:
        raise ValueError("A fixture command is required")
    if not hasattr(os, "pidfd_open") or not hasattr(signal, "pidfd_send_signal"):
        raise RuntimeError("Linux pidfd support is required before spawning")
    # Check kernel/permission support before creating any child.
    probe = os.pidfd_open(os.getpid())
    os.close(probe)
    context = process_context()
    process = None
    created_receipt = False
    handlers = {}
    cancelled_signal = None
    launch_read = None
    launch_write = None

    def interrupted(signum, frame):
        # Defer cancellation to an ownership boundary. Raising asynchronously
        # could land after Popen spawned the child but before assignment, or
        # interrupt rollback itself and orphan the child.
        nonlocal cancelled_signal
        cancelled_signal = signum

    def check_cancelled():
        if cancelled_signal is not None:
            raise StartupInterrupted(f"Fixture startup interrupted by signal {cancelled_signal}")

    try:
        for signum in (signal.SIGINT, signal.SIGTERM):
            handlers[signum] = signal.signal(signum, interrupted)
        # Refuse an old receipt before spawn, rather than adopting/replacing it.
        with receipt.open("x") as stream:
            created_receipt = True
            with log.open("wb") as output:
                check_cancelled()
                launch_read, launch_write = os.pipe()
                # The fixture cannot exec before its receipt is durable. If
                # this helper dies even by SIGKILL, closing the only writer
                # makes the waiting child exit rather than launch unowned.
                launcher = (
                    "import os,sys; fd=int(sys.argv[1]); token=os.read(fd,1); "
                    "os.close(fd); "
                    "sys.exit(125) if token != b'G' else "
                    "os.execvpe(sys.argv[2],sys.argv[2:],os.environ)"
                )
                process = subprocess.Popen([sys.executable, "-c", launcher,
                                            str(launch_read), *command],
                                           stdout=output, stderr=subprocess.STDOUT,
                                           start_new_session=True, pass_fds=(launch_read,))
                os.close(launch_read)
                launch_read = None
                check_cancelled()
                ticks = start_time(process.pid)
                fd = os.pidfd_open(process.pid)
                try:
                    if start_time(process.pid) != ticks or select.select([fd], [], [], 0)[0]:
                        raise ProcessLookupError("Fixture exited before receipt was committed")
                    json.dump({"version": 1, "pid": process.pid, "start_time": ticks,
                               "context": context}, stream)
                    stream.flush()
                    os.fsync(stream.fileno())
                    check_cancelled()
                    os.write(launch_write, b"G")
                    check_cancelled()
                finally:
                    os.close(fd)
        return process.pid
    except BaseException:
        # A second TERM/INT must not interrupt the bounded rollback and leave
        # an unrecorded child. SIGKILL cannot be caught by any cleanup handler.
        for signum in handlers:
            signal.signal(signum, signal.SIG_IGN)
        # This is our direct, unreaped child, not a PID discovered from the host.
        # Its PID cannot be reused until we reap it. No process-group/name/port
        # selector or external receipt is used on this rollback path.
        if process is not None:
            try:
                process.terminate()
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=10)
            except ProcessLookupError:
                process.wait()
        if created_receipt:
            receipt.unlink(missing_ok=True)
        raise
    finally:
        for descriptor in (launch_read, launch_write):
            if descriptor is not None:
                os.close(descriptor)
        for signum, previous in handlers.items():
            signal.signal(signum, previous)


def stop(receipt: Path, timeout: float = 10.0) -> None:
    try:
        identity = json.loads(receipt.read_text())
    except FileNotFoundError:
        return
    if identity.get("version") != 1:
        raise ValueError("Unknown process receipt version; refusing to signal")
    pid = identity.get("pid")
    ticks = identity.get("start_time")
    if type(pid) is not int or pid <= 1 or type(ticks) is not int or ticks <= 0:
        raise ValueError("Invalid process identity; refusing to signal")
    if identity.get("context") != process_context():
        raise ValueError("Process receipt belongs to another boot/PID namespace")
    if not hasattr(os, "pidfd_open") or not hasattr(signal, "pidfd_send_signal"):
        raise RuntimeError("Linux pidfd support is required for owned cleanup")
    try:
        fd = os.pidfd_open(pid)
    except ProcessLookupError:
        receipt.unlink(missing_ok=True)
        return
    try:
        try:
            same_process = start_time(pid) == ticks
        except FileNotFoundError:
            same_process = False
        if not same_process:
            # The original process is gone. Never signal whoever has its PID.
            receipt.unlink(missing_ok=True)
            return
        try:
            signal.pidfd_send_signal(fd, signal.SIGTERM)
            if not select.select([fd], [], [], timeout)[0]:
                signal.pidfd_send_signal(fd, signal.SIGKILL)
                if not select.select([fd], [], [], timeout)[0]:
                    raise RuntimeError("Owned fixture process did not stop")
        except ProcessLookupError:
            pass
        receipt.unlink(missing_ok=True)
    finally:
        os.close(fd)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    remember = commands.add_parser("record")
    remember.add_argument("pid", type=int)
    remember.add_argument("receipt", type=Path)
    spawn = commands.add_parser("start")
    spawn.add_argument("--receipt", required=True, type=Path)
    spawn.add_argument("--log", required=True, type=Path)
    spawn.add_argument("argv", nargs=argparse.REMAINDER)
    terminate = commands.add_parser("stop")
    terminate.add_argument("receipt", type=Path)
    args = parser.parse_args()
    if args.command == "record":
        record(args.pid, args.receipt)
    elif args.command == "start":
        command = args.argv[1:] if args.argv[:1] == ["--"] else args.argv
        print(start(args.receipt, args.log, command))
    else:
        stop(args.receipt)


if __name__ == "__main__":
    main()
