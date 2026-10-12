#!/usr/bin/env python3
"""Focused real-Docker same-job isolation regression, not a cross-run proof.

Requires an already built registry server and the production gate prerequisites.
Never fabricates GitHub run IDs, kernel identities, or fixture process receipts.
"""
import argparse
import ctypes
from datetime import datetime
import importlib.util
import json
import os
from pathlib import Path
import re
import select
import signal
import subprocess
import time
from types import SimpleNamespace

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("fixture_evidence", ROOT / "scripts/ci-fixture-evidence.py")
evidence = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(evidence)
owned = evidence.owned
CANCELLED = None


class GateFailure(RuntimeError):
    def __init__(self, command, code):
        super().__init__(f"fixture {command} failed; inspect its retained locator/log")
        self.code = code if code > 0 else 128 - code


def check_cancelled():
    if CANCELLED is not None:
        raise InterruptedError("same-job fixture check cancelled")


def read_state(path):
    result = {}
    for line in path.read_text().splitlines():
        match = re.fullmatch(r'([A-Z_]+)="([a-z0-9_-]+)"', line)
        if not match or match[1] in result:
            raise ValueError("invalid owned fixture state")
        result[match[1]] = match[2]
    if set(result) != {"COMPOSE_PROJECT_NAME", "PG_PORT", "MINIO_PORT", "MINIO_CONSOLE_PORT", "REGISTRY_PORT"}:
        raise ValueError("incomplete owned fixture state")
    for key in ("PG_PORT", "MINIO_PORT", "MINIO_CONSOLE_PORT", "REGISTRY_PORT"):
        if not result[key].isdigit() or not 0 <= int(result[key]) < 65536:
            raise ValueError("invalid owned fixture port")
    return result


def is_alive(identity):
    if identity.get("version") != 1 or identity.get("context") != owned.process_context():
        raise ValueError("wrong fixture process context")
    if (type(identity.get("pid")) is not int or identity["pid"] <= 1
            or type(identity.get("start_time")) is not int or identity["start_time"] <= 0):
        raise ValueError("invalid fixture process identity")
    try:
        descriptor = os.pidfd_open(identity["pid"])
    except ProcessLookupError:
        return False
    try:
        try:
            return (owned.start_time(identity["pid"]) == identity["start_time"]
                    and not select.select([descriptor], [], [], 0)[0])
        except FileNotFoundError:
            return False
    finally:
        os.close(descriptor)


class Fixture:
    def __init__(self, root, name, environment):
        self.root = root / name
        self.root.mkdir(mode=0o700)
        self.env = dict(environment)
        # Let the production gate allocate fresh projects and ports. Never
        # inherit a caller's fixed project, state, endpoint, or server receipt.
        for key in ("COMPOSE_PROJECT_NAME", "PG_PORT", "MINIO_PORT", "MINIO_CONSOLE_PORT", "REGISTRY_PORT"):
            self.env.pop(key, None)
        self.env.update(STATE_FILE=str(self.root / "state"), STORAGE_PATH=str(self.root / "storage"),
                        SERVER_LOG=str(self.root / "server.log"), SERVER_PID_FILE=str(self.root / "server.json"),
                        EVIDENCE_OUTPUT=str(self.root / "evidence.jsonl"),
                        COMPOSE_FILE_GATE="docker-compose.rls-gate.yml")
        self.project = None
        self.identity = None

    def command(self, command):
        if command == "up":
            check_cancelled()
        # Own only this newly spawned invocation group. GNU timeout KILL avoids
        # the direct-client-exits-before-plugin TERM orphan race. Servers are
        # deliberately separate sessions with their own durable pidfd receipts.
        parent = os.getpid()
        def bind_parent():
            libc = ctypes.CDLL(None, use_errno=True)
            if libc.prctl(1, signal.SIGALRM, 0, 0, 0) != 0 or os.getppid() != parent:
                os._exit(125)
        seconds = 180 if command == "up" else 120
        with (self.root / f"{command}.log").open("ab") as log:
            # The handler only records cancellation, so a signal cannot raise
            # between Popen's spawn and assignment. This is our unreaped direct
            # child; its PID cannot be reused while opening the pidfd.
            process = subprocess.Popen(["timeout", "--signal=KILL", f"{seconds}s", "bash",
                                        str(ROOT / "scripts/rls-e2e-gate.sh"), command],
                                       env=self.env, stdout=log, stderr=subprocess.STDOUT,
                                       preexec_fn=bind_parent)
            descriptor = None
            try:
                descriptor = os.pidfd_open(process.pid)
                while not select.select([descriptor], [], [], .1)[0]:
                    if command == "up" and CANCELLED is not None:
                        # ALRM asks the known GNU supervisor to KILL only its
                        # own invocation group immediately, then the outer
                        # finally tears down exact committed fixture state.
                        signal.pidfd_send_signal(descriptor, signal.SIGALRM)
                        break
                code = process.wait()
            except BaseException:
                # Also roll back if opening/polling the pidfd fails. Popen owns
                # this direct child and has not reaped it; no adopted PID/group.
                process.send_signal(signal.SIGALRM)
                process.wait()
                raise
            finally:
                if descriptor is not None:
                    os.close(descriptor)
        if command == "up":
            check_cancelled()
        if code:
            raise GateFailure(command, code)

    def state(self):
        state = read_state(Path(self.env["STATE_FILE"]))
        slug = re.sub(r"[^a-z0-9_-]", "-", self.env["RUNNER_NAME"].lower())
        expected = ("rlsgate-" + self.env["GITHUB_RUN_ID"] + "-" + self.env["GITHUB_RUN_ATTEMPT"]
                    + r"-[a-f0-9]{32}-" + re.escape(slug))
        if not re.fullmatch(expected, state["COMPOSE_PROJECT_NAME"]):
            raise ValueError("refusing a foreign fixture project")
        if self.project is not None and self.project != state["COMPOSE_PROJECT_NAME"]:
            raise ValueError("fixture project identity changed")
        self.project = state["COMPOSE_PROJECT_NAME"]
        return state

    def args(self):
        state = self.state()
        cids = {}
        for service in ("db", "minio"):
            # Exact own-project AND service selectors; no global enumeration.
            value = evidence.docker("container", "ls", "--all", "--filter",
                "label=com.docker.compose.project=" + self.project, "--filter",
                "label=com.docker.compose.service=" + service, "--format", "{{.ID}}")
            if not re.fullmatch(r"[a-f0-9]{12,64}", value):
                raise ValueError("missing or ambiguous owned service")
            cids[service] = value
        return SimpleNamespace(project=self.project, compose="docker-compose.rls-gate.yml",
            run_id=self.env["GITHUB_RUN_ID"], output=Path(self.env["EVIDENCE_OUTPUT"]),
            port=int(state["REGISTRY_PORT"]), receipt=Path(self.env["SERVER_PID_FILE"]),
            db_cid=cids["db"], minio_cid=cids["minio"])

    def up(self):
        self.command("up")
        check_cancelled()
        args = self.args()
        self.identity = json.loads(args.receipt.read_text())
        if not is_alive(self.identity):
            raise RuntimeError("fixture server is not alive")
        evidence.sample(args, "started")
        return args

    def down(self):
        path = Path(self.env["STATE_FILE"])
        if not path.exists():
            if self.identity is not None and is_alive(self.identity):
                raise RuntimeError("live fixture lost its owned project locator")
            return
        # Failed setup cleans itself. A KILL may leave a valid owned locator;
        # never synthesize a different project or source arbitrary shell text.
        self.state()
        self.command("down")
        if path.exists() or (self.identity is not None and is_alive(self.identity)):
            raise RuntimeError("owned fixture cleanup was incomplete")


def verify_same_job(first_path, second_path, first_identity, second_identity, run_id, attempt):
    """Separate meaning from the unchanged cross-run evidence comparator."""
    if not re.fullmatch(r"[0-9]+", run_id) or not re.fullmatch(r"[1-9][0-9]*", attempt):
        raise ValueError("invalid real CI identity")
    for identity in (first_identity, second_identity):
        if (identity.get("version") != 1 or type(identity.get("pid")) is not int or identity["pid"] <= 1
                or type(identity.get("start_time")) is not int or identity["start_time"] <= 0
                or set(identity.get("context", {})) != {"boot_id", "pid_namespace"}
                or not all(isinstance(value, str) and value for value in identity["context"].values())):
            raise ValueError("invalid kernel identity receipt")
    if first_identity == second_identity or first_identity["pid"] == second_identity["pid"]:
        raise ValueError("fixtures must own different real processes")
    if first_identity["context"] != second_identity["context"]:
        raise ValueError("fixtures must share the same process context")
    groups = []
    for path in (first_path, second_path):
        rows = evidence.events(path)
        groups.append(evidence.verify(path, rows[0]["project"]))
    first, second = groups
    if first[0]["project"] == second[0]["project"]:
        raise ValueError("fixtures must own different projects")
    for rows in groups:
        if rows[0]["run_id"] != run_id or not rows[0]["project"].startswith(f"rlsgate-{run_id}-{attempt}-"):
            raise ValueError("evidence does not belong to this actual run/attempt")
        if rows[0]["compose"] != "docker-compose.rls-gate.yml" or any(
            row["phase"] not in {"started", "healthy", "cleaned"} or
            (row["phase"] != "cleaned" and row.get("registry_healthy") is not True) for row in rows):
            raise ValueError("invalid same-job fixture health evidence")
    if first[0]["daemon"] != second[0]["daemon"]:
        raise ValueError("different daemons do not prove shared-host isolation")
    if set(first[0]["ports"].values()) & set(second[0]["ports"].values()):
        raise ValueError("fixtures share an endpoint")
    a_cids = {service["container"] for service in first[0]["services"].values()}
    b_cids = {service["container"] for service in second[0]["services"].values()}
    if a_cids & b_cids:
        raise ValueError("fixtures share a container")
    cleaned = datetime.fromisoformat(first[-1]["at"])
    moments = [datetime.fromisoformat(row["at"]) for row in second if row["phase"] == "healthy"]
    if not any(0 <= (cleaned - point).total_seconds() <= 30 for point in moments):
        raise RuntimeError("missing pre-cleanup survivor health")
    if not any(5 <= (point - cleaned).total_seconds() <= 30 for point in moments):
        raise RuntimeError("missing post-cleanup survivor health")
    return {"status": "same_job_fixture_isolation_proven", "run_id": run_id, "run_attempt": attempt,
            "daemon": first[0]["daemon"], "cleaned_project": first[0]["project"],
            "surviving_project": second[0]["project"], "cleanup_at": first[-1]["at"]}


def run(output, environment):
    global CANCELLED
    CANCELLED = None
    for key in ("GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT"):
        if not re.fullmatch(r"[1-9][0-9]*", environment.get(key, "")):
            raise ValueError("an actual CI run and attempt are required")
    if not environment.get("RUNNER_NAME"):
        raise ValueError("an actual runner identity is required")
    binary = Path(environment.get("CARGO_TARGET_DIR", str(ROOT / "target"))) / "debug/mockforge-registry-server"
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise RuntimeError("build the exact checked-out registry server before this focused test")
    version = subprocess.check_output(["timeout", "--version"], text=True, timeout=5,
                                      env={**environment, "LC_ALL": "C"})
    if not version.startswith("timeout (GNU coreutils)"):
        raise RuntimeError("GNU timeout is required for owned invocation bounds")
    output.mkdir(mode=0o700)
    fixtures = [Fixture(output, name, environment) for name in ("first", "survivor")]
    previous = {}
    def cancelled(signum, frame):
        global CANCELLED
        CANCELLED = signum
    for signum in (signal.SIGINT, signal.SIGTERM):
        previous[signum] = signal.signal(signum, cancelled)
    error = None
    result = None
    try:
        first, second = fixtures
        first_args = first.up()
        second_args = second.up()
        check_cancelled()
        evidence.sample(first_args, "healthy")
        evidence.sample(second_args, "healthy")
        if not is_alive(first.identity) or not is_alive(second.identity):
            raise RuntimeError("both fixtures must be live before cleanup")
        (output / "process-receipts.json").write_text(json.dumps({
            "run_id": environment["GITHUB_RUN_ID"], "run_attempt": environment["GITHUB_RUN_ATTEMPT"],
            "first": first.identity, "survivor": second.identity}, sort_keys=True) + "\n")
        first.down()
        deadline = time.monotonic() + 6
        while time.monotonic() < deadline:
            check_cancelled()
            time.sleep(.1)
        if json.loads(second_args.receipt.read_text()) != second.identity or not is_alive(second.identity):
            raise RuntimeError("first cleanup changed the survivor process")
        evidence.sample(second_args, "healthy")
        second.down()
        check_cancelled()
        result = verify_same_job(first_args.output, second_args.output, first.identity, second.identity,
                                 environment["GITHUB_RUN_ID"], environment["GITHUB_RUN_ATTEMPT"])
    except BaseException as caught:
        error = caught
    finally:
        # Repeated TERM cannot interrupt cleanup of the second owned fixture.
        for signum in previous: signal.signal(signum, signal.SIG_IGN)
        for fixture in reversed(fixtures):
            try: fixture.down()
            except BaseException as cleanup_error:
                if error is None: error = cleanup_error
        for signum, handler in previous.items(): signal.signal(signum, handler)
    if error is None and CANCELLED is not None:
        error = InterruptedError("same-job fixture check cancelled")
    if error is not None:
        raise error
    (output / "result.json").write_text(json.dumps(result, sort_keys=True) + "\n")
    print(json.dumps(result, sort_keys=True))
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, required=True, help="new job-owned directory; must not already exist")
    options = parser.parse_args()
    try:
        run(options.output_dir, os.environ)
    except GateFailure as error:
        raise SystemExit(error.code)
    except InterruptedError:
        raise SystemExit(128 + (CANCELLED or signal.SIGTERM))
