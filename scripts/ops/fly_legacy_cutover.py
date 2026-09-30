#!/usr/bin/env python3
"""Inventory and quiesce the legacy MockForge Fly Machines after Ash cutover.

This runs only from a protected-main manual workflow with its Fly secret. The
inventory receipt is safe to download; it contains Machine metadata, not env.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
from datetime import datetime, timezone
from pathlib import Path

APPS = (
    "mockforge-demo",
    "mockforge-registry",
    "mockforge-tunnel-relay",
)
INERT_STATES = {"created", "stopped", "suspended"}
LIVE_STATES = {"started"}


def fly(*args: str) -> str:
    if not os.environ.get("FLY_API_TOKEN"):
        raise RuntimeError("FLY_API_TOKEN is required")
    try:
        result = subprocess.run(
            ["flyctl", *args],
            check=True,
            capture_output=True,
            text=True,
            timeout=120,
        )
    except subprocess.CalledProcessError as error:
        # Report only a fixed failure class and app name; flyctl's raw stderr
        # may contain account details and must stay out of Actions logs.
        stderr = (error.stderr or "").lower()
        if "not authorized" in stderr or "permission" in stderr or "forbidden" in stderr:
            reason = "access_denied"
        elif "not found" in stderr or "could not find" in stderr:
            reason = "app_not_found"
        elif "unauthorized" in stderr or "invalid token" in stderr:
            reason = "invalid_token"
        else:
            reason = "flyctl_error"
        app = args[args.index("-a") + 1] if "-a" in args else "unknown"
        raise RuntimeError(
            f"flyctl {args[0]} {args[1]} app={app} exit={error.returncode} reason={reason}"
        ) from error
    return result.stdout


def inventory() -> dict[str, object]:
    apps: dict[str, list[dict[str, object]]] = {}
    ids: set[str] = set()
    for app in APPS:
        rows = json.loads(fly("machine", "list", "-a", app, "--json"))
        if not isinstance(rows, list) or not rows:
            raise RuntimeError(f"unexpected empty Machine inventory: {app}")
        machines = []
        for row in rows:
            machine_id = row["id"]
            if not isinstance(machine_id, str) or machine_id in ids:
                raise RuntimeError("invalid or repeated Machine ID")
            ids.add(machine_id)
            config = row.get("config") or {}
            services = config.get("services") or []
            if not isinstance(services, list):
                raise RuntimeError("invalid Machine services")
            state = row["state"]
            if state not in INERT_STATES | LIVE_STATES:
                raise RuntimeError(f"Machine in transient/unexpected state: {app}")
            machines.append(
                {
                    "id": machine_id,
                    "name": row["name"],
                    "region": row["region"],
                    "state": state,
                    "image_ref": row.get("image_ref"),
                    "service_autostart": [
                        service.get("autostart") for service in services
                    ],
                }
            )
        apps[app] = sorted(machines, key=lambda machine: str(machine["id"]))
    if len(apps) != len(APPS) or len(ids) < len(APPS):
        raise RuntimeError("Fly inventory differs from three populated apps")
    identity = {
        app: [
            {key: machine[key] for key in ("id", "name", "region", "image_ref")}
            for machine in machines
        ]
        for app, machines in apps.items()
    }
    fingerprint = hashlib.sha256(
        json.dumps(identity, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()
    return {
        "captured_at": datetime.now(timezone.utc).isoformat(),
        "identity_sha256": fingerprint,
        "apps": apps,
    }


def write_receipt(path: Path, receipt: dict[str, object]) -> None:
    if path.exists():
        raise RuntimeError("receipt path already exists")
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, "w") as output:
        json.dump(receipt, output, sort_keys=True, indent=2)
        output.write("\n")


def quiesce(receipt: dict[str, object]) -> None:
    apps = receipt["apps"]
    if not isinstance(apps, dict):
        raise RuntimeError("invalid inventory")
    for app, machines in apps.items():
        for machine in machines:
            machine_id = machine["id"]
            if machine["service_autostart"] and any(
                enabled is not False for enabled in machine["service_autostart"]
            ):
                fly(
                    "machine",
                    "update",
                    machine_id,
                    "-a",
                    app,
                    "--autostart=false",
                    "--skip-start",
                    "--yes",
                )
            # The update may itself leave a new Machine version in `created`.
            # Re-read before stopping so an old state cannot drive a bad action.
            current = machine_by_id(app, machine_id)
            if current["state"] == "started":
                fly("machine", "stop", machine_id, "-a", app, "--wait-timeout", "2m")
    after = inventory()
    if after["identity_sha256"] != receipt["identity_sha256"]:
        raise RuntimeError("Machine identity changed during quiescence")
    for app, machines in after["apps"].items():
        for machine in machines:
            if machine["state"] not in INERT_STATES or any(
                enabled is not False for enabled in machine["service_autostart"]
            ):
                raise RuntimeError(f"Machine still active or autostart enabled: {app}")


def machine_by_id(app: str, machine_id: str) -> dict[str, object]:
    rows = json.loads(fly("machine", "list", "-a", app, "--json"))
    matches = [row for row in rows if row.get("id") == machine_id]
    if len(matches) != 1:
        raise RuntimeError("Machine disappeared or duplicated during cutover")
    return matches[0]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=("inventory", "quiesce", "verify"))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expected-sha", default="")
    args = parser.parse_args()
    before = inventory()
    if args.action == "quiesce":
        if (
            len(args.expected_sha) != 64
            or args.expected_sha != before["identity_sha256"]
        ):
            raise RuntimeError(
                "Machine inventory identity differs from approved receipt"
            )
        # Keep the pre-mutation receipt even if a later Fly call fails.
        write_receipt(args.output, before)
        quiesce(before)
        after = inventory()
        write_receipt(args.output.with_name(args.output.stem + "-after.json"), after)
        before = after
    elif args.action == "verify":
        if any(
            machine["state"] not in INERT_STATES
            or any(enabled is not False for enabled in machine["service_autostart"])
            for machines in before["apps"].values()
            for machine in machines
        ):
            raise RuntimeError("legacy Fly Machines are not quiesced")
    if args.action != "quiesce":
        write_receipt(args.output, before)
    print(
        f"fly_cutover_{args.action}_ok apps=3 machines={sum(map(len, before['apps'].values()))} "
        f"identity_sha256={before['identity_sha256']}"
    )


if __name__ == "__main__":
    try:
        main()
    except RuntimeError as error:
        raise SystemExit("Fly cutover refused: " + str(error)) from error
    except (
        OSError,
        ValueError,
        KeyError,
        TypeError,
        subprocess.SubprocessError,
    ) as error:
        raise SystemExit("Fly cutover refused: " + type(error).__name__) from error
