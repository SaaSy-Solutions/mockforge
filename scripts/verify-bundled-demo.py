#!/usr/bin/env python3
"""Exercise every bundled JSON response example through the real demo HTTP server.

The caller supplies an already built image. --mount-specs tests sample-only edits
against that image's binary; it does not attest that a new image was built.
"""

import argparse
import datetime
from decimal import Decimal, InvalidOperation
import hashlib
import json
from pathlib import Path
import secrets
import signal
import subprocess
import time
import urllib.parse
import uuid

ROOT = Path(__file__).resolve().parents[1]
SCENARIOS = ("ecommerce-store", "weather-geo", "chat-api")


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def no_markers(value, path="response"):
    if isinstance(value, str):
        require("{{" not in value and "}}" not in value, f"Unexpanded template at {path}: {value}")
    elif isinstance(value, list):
        for index, item in enumerate(value):
            no_markers(item, f"{path}[{index}]")
    elif isinstance(value, dict):
        for key, item in value.items():
            no_markers(key, path + " key")
            no_markers(item, f"{path}.{key}")


def same_shape(expected, actual, path="response"):
    require(type(expected) is type(actual), f"JSON type changed at {path}")
    if isinstance(expected, dict):
        require(expected.keys() == actual.keys(), f"JSON fields changed at {path}")
        for key in expected:
            same_shape(expected[key], actual[key], f"{path}.{key}")
    elif isinstance(expected, list):
        require(len(expected) == len(actual), f"Example array length changed at {path}")
        for index, item in enumerate(expected):
            same_shape(item, actual[index], f"{path}[{index}]")


def within(value, low, high):
    require(low <= Decimal(value) <= high, f"Generated value is outside [{low}, {high}]")


def semantic_values(path, value):
    if path == "/api/products":
        for product in value["products"]:
            uuid.UUID(product["id"])
            require(bool(product["name"] and product["category"] and product["description"]), "Empty product data")
            within(product["price"], 1, 1000)
            within(product["stock"], 0, 100)
        within(value["total"], 10, 100)
    elif path == "/api/cart":
        uuid.UUID(value["id"])
        for item in value["items"]:
            uuid.UUID(item["productId"])
            within(item["quantity"], 1, 5)
            within(item["price"], 1, 1000)
        within(value["total"], 1, 1000)
    elif path == "/api/orders":
        uuid.UUID(value["id"])
        require(value["status"] == "pending", "Order status changed")
        within(value["total"], 1, 1000)
        datetime.datetime.fromisoformat(value["createdAt"])
    elif path == "/api/weather/current":
        within(value["temperature"], -10, 35)
        within(value["humidity"], 30, 90)
        within(value["windSpeed"], 0, 30)
        require(value["condition"] in ("sunny", "cloudy", "rainy", "snowy"), "Invalid weather condition")
        require(bool(value["location"]["city"] and value["location"]["country"]), "Missing location")
        within(value["location"]["coordinates"]["lat"], -90, 90)
        within(value["location"]["coordinates"]["lon"], -180, 180)
    elif path == "/api/weather/forecast":
        for day in value["forecast"]:
            datetime.datetime.fromisoformat(day["date"])
            within(day["temperature"], -10, 35)
            require(day["condition"] in ("sunny", "cloudy", "rainy"), "Invalid forecast condition")
    elif path == "/api/messages":
        for message in value["messages"]:
            uuid.UUID(message["id"])
            uuid.UUID(message["userId"])
            datetime.datetime.fromisoformat(message["timestamp"])
            require(bool(message["text"].strip()), "Empty chat message")


def examples():
    cases = []
    files = {}
    for name in SCENARIOS:
        source = ROOT / "examples/scenarios" / name / "openapi.json"
        raw = source.read_bytes()
        files[name] = {"path": source, "sha256": hashlib.sha256(raw).hexdigest()}
        spec = json.loads(raw)
        for path, methods in spec["paths"].items():
            for method, operation in methods.items():
                if method not in ("get", "post", "put", "patch", "delete"):
                    continue
                query = {"city": "London"} if path.startswith("/api/weather/") else {}
                for parameter in operation.get("parameters", []):
                    if parameter.get("required") and parameter.get("in") == "query":
                        query[parameter["name"]] = "demo-fixture"
                for status, response in operation["responses"].items():
                    content = response.get("content", {}).get("application/json", {})
                    if "example" in content:
                        cases.append((method.upper(), path, query, int(status), content["example"]))
    require(len(cases) == 6, "Bundled JSON response-example inventory changed; review coverage")
    return cases, files


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--image", required=True)
    parser.add_argument("--docker-host", default="unix:///var/run/docker.sock")
    parser.add_argument("--mount-specs", action="store_true")
    parser.add_argument("--receipt", required=True, type=Path)
    args = parser.parse_args()
    require(not args.receipt.exists(), "Receipt already exists; preserve prior evidence")
    docker = ["/usr/bin/docker", "--host", args.docker_host]

    def run(*arguments, timeout=30):
        result = subprocess.run(docker + list(arguments), capture_output=True, timeout=timeout)
        require(result.returncode == 0, "Docker operation failed: " + result.stderr.decode()[-1000:])
        return result.stdout

    cases, files = examples()
    image = json.loads(run("image", "inspect", args.image))[0]
    require(image["Os"] == "linux" and image["Architecture"] == "amd64", "Expected Linux amd64 image")
    owner = "mockforge-demo-check-" + secrets.token_hex(8)
    receipt = {"image_id": image["Id"], "mounted_checkout_specs": args.mount_specs,
               "spec_sha256": {name: row["sha256"] for name, row in files.items()},
               "cases": [], "cleanup_verified": False}
    identity = None
    def interrupted(*_):
        raise InterruptedError("Demo verification interrupted; cleaning owned fixture")

    for signum in (signal.SIGINT, signal.SIGTERM):
        signal.signal(signum, interrupted)
    try:
        command = ["run", "-d", "--name", owner, "--label", "saasy.fixture.owner=" + owner,
                   "--network", "none", "--read-only", "--cap-drop", "ALL",
                   "--security-opt", "no-new-privileges", "--memory", "768m",
                   "--memory-swap", "768m", "--cpus", "1", "--pids-limit", "128",
                   "--tmpfs", "/tmp:rw,nosuid,nodev,size=64m,mode=1777",
                   "--tmpfs", "/home/mockforge:rw,nosuid,nodev,size=32m,mode=1777",
                   "--env", "MOCKFORGE_LATENCY_ENABLED=false",
                   "--env", "MOCKFORGE_MANAGEMENT_TOKEN=" + secrets.token_urlsafe(32)]
        if args.mount_specs:
            for name, row in files.items():
                command += ["--mount", f"type=bind,src={row['path']},dst=/app/examples/scenarios/{name}/openapi.json,readonly"]
        command += [image["Id"], "serve"]
        for name in SCENARIOS:
            command += ["--spec", f"/app/examples/scenarios/{name}/openapi.json"]
        command += ["--admin"]
        identity = run(*command).decode().strip()
        require(len(identity) == 64, "Expected exact created container identity")
        live = json.loads(run("inspect", identity))[0]
        host = live["HostConfig"]
        require(host["Memory"] == host["MemorySwap"] == 768 * 1024**2
                and host["NanoCpus"] == 1_000_000_000 and host["PidsLimit"] == 128
                and host["NetworkMode"] == "none" and host["ReadonlyRootfs"]
                and host["CapDrop"] == ["ALL"] and "no-new-privileges" in host["SecurityOpt"]
                and not host["PortBindings"], "Owned fixture resource/network limits differ")
        receipt["resource_limits_verified"] = True
        for _ in range(60):
            ready = subprocess.run(docker + ["exec", identity, "curl", "--silent", "--fail",
                                   "--max-time", "2", "http://127.0.0.1:3000/health"], capture_output=True, timeout=5)
            if ready.returncode == 0:
                break
            time.sleep(0.25)
        else:
            raise AssertionError("Owned demo failed to become ready")
        receipt["binary_sha256"] = run("exec", identity, "sha256sum", "/proc/1/exe").decode().split()[0]
        for method, path, query, expected_status, expected in cases:
            url = "http://127.0.0.1:3000" + path
            if query:
                url += "?" + urllib.parse.urlencode(query)
            raw = run("exec", identity, "curl", "--silent", "--show-error", "--max-time", "5",
                      "--request", method, "--write-out", "\n%{http_code}", url)
            body, status = raw.rsplit(b"\n", 1)
            result = {"method": method, "path": path, "status": int(status), "passed": False}
            try:
                require(int(status) == expected_status, "HTTP status differs from bundled example")
                actual = json.loads(body)
                same_shape(expected, actual)
                no_markers(actual)
                semantic_values(path, actual)
                result["passed"] = True
            except (AssertionError, ValueError, InvalidOperation) as error:
                result["error"] = str(error)
            receipt["cases"].append(result)
        require(all(case["passed"] for case in receipt["cases"]), "Bundled demo response checks failed")
        require(all(hashlib.sha256(row["path"].read_bytes()).hexdigest() == row["sha256"]
                    for row in files.values()), "Scenario changed during verification")
    finally:
        # A second cancellation must not interrupt cleanup of our one fixture.
        for signum in (signal.SIGINT, signal.SIGTERM):
            signal.signal(signum, signal.SIG_IGN)
        if identity is None:
            # Docker can accept creation even if its client times out before
            # returning an ID. Reconcile only the exact random owned name.
            probe = subprocess.run(docker + ["inspect", owner], capture_output=True, timeout=30)
            if probe.returncode == 0:
                candidate = json.loads(probe.stdout)[0]
                require(candidate["Config"].get("Labels", {}).get("saasy.fixture.owner") == owner
                        and candidate["Image"] == image["Id"], "Unknown fixture generation; no cleanup attempted")
                identity = candidate["Id"]
        if identity:
            current = json.loads(run("inspect", identity))[0]
            require(current["Id"] == identity and current["Config"]["Labels"].get("saasy.fixture.owner") == owner,
                    "Owned fixture identity changed; no cleanup attempted")
            run("rm", "--force", identity)
            listed = run("ps", "-a", "--no-trunc", "--filter", "id=" + identity, "--format", "{{.ID}}")
            receipt["cleanup_verified"] = not bool(listed.strip())
        args.receipt.write_text(json.dumps(receipt, indent=2) + "\n")
        print(json.dumps(receipt, indent=2))


if __name__ == "__main__":
    main()
