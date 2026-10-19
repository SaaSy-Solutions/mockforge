#!/usr/bin/env python3
"""Run SDK tests against an explicit, version-matched local MockForge binary."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import queue
import secrets
import signal
import shlex
import shutil
import subprocess
import tarfile
import tempfile
import threading
import time
import tomllib
import urllib.request
import re

MAVEN_VERSION = "3.10.0"
MAVEN_SHA512 = "908b1501bfb420bf7c8affb855534a9c407fd6099367bfb9f2f2dcb8e9799102bffb84518cde74c679bd76870247c6528683abdd620581bffa90f95d92d175aa"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--maven", type=Path)
    parser.add_argument("--temp-root", type=Path)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    binary = args.binary.resolve(strict=True)
    expected = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    env = {key: os.environ[key] for key in ("PATH", "HOME", "LANG", "JAVA_HOME") if key in os.environ}
    version = subprocess.check_output([str(binary), "--version"], env=env, text=True).strip()
    if version != f"mockforge {expected}":
        raise SystemExit("SDK tests require a binary matching the workspace version")
    report = {"binary_version": expected, "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "phases": [], "complete": False, "all_passed": False}
    args.report.parent.mkdir(parents=True, exist_ok=True)
    logs = args.report.parent / "sdk-runtime-logs"
    logs.mkdir(exist_ok=True)

    def save():
        args.report.write_text(json.dumps(report, indent=2) + "\n")

    def run(name, command, cwd, child_env=None):
        log = logs / f"{name}.log"
        started = time.monotonic()
        with log.open("w") as out:
            process = subprocess.Popen(command, cwd=cwd, env=child_env or env, stdout=out, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                exit_code = process.wait(timeout=600)
            except subprocess.TimeoutExpired:
                exit_code = 124
            finally:
                # Only this phase's fresh process group: reap leaked fixture children.
                try:
                    os.killpg(process.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait(timeout=5)
        report["phases"].append({"name": name, "exit_code": exit_code, "seconds": round(time.monotonic() - started, 2), "log": str(log)})
        save()
        print(f"{name}: {'passed' if exit_code == 0 else 'failed'}", flush=True)
        if exit_code:
            raise SystemExit(f"SDK phase failed; see {log}")
        return log.read_text()

    if args.temp_root:
        args.temp_root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="mockforge-sdk-runtime-", dir=args.temp_root) as temporary:
        temp = Path(temporary)
        server_cwd = temp / "server-cwd"
        server_cwd.mkdir()
        (server_cwd / ".env").write_text("")
        shim = temp / "bin"
        shim.mkdir()
        wrapper = shim / "mockforge"
        wrapper.write_text(f"#!/bin/sh\ncd {shlex.quote(str(server_cwd))} || exit 1\nexec {shlex.quote(str(binary))} \"$@\"\n")
        wrapper.chmod(0o700)
        user_npm = temp / "user.npmrc"
        global_npm = temp / "global.npmrc"
        maven_settings = temp / "maven-settings.xml"
        user_npm.write_text("")
        global_npm.write_text("")
        maven_settings.write_text('<settings xmlns="http://maven.apache.org/SETTINGS/1.0.0"/>\n')
        env.update(PATH=str(shim) + os.pathsep + env["PATH"], GOENV="off", GOCACHE=str(temp / "go-cache"), GOMODCACHE=str(temp / "go-modules"), npm_config_cache=str(temp / "npm-cache"), npm_config_userconfig=str(user_npm), npm_config_globalconfig=str(global_npm))
        run("python-environment", ["python3", "-m", "venv", str(temp / "python")], root)
        python = str(temp / "python/bin/python")
        run("python-install", [python, "-m", "pip", "--isolated", "install", "--disable-pip-version-check", "-e", str(root / "sdk/python") + "[dev]"], root)
        run("python-tests", [python, "-m", "pytest", "tests", "-q"], root / "sdk/python")
        go_output = run("go-tests", ["go", "test", "-v", "./...", "-timeout", "120s"], root / "sdk/go")
        if "--- PASS: TestDynamicMockRuntime" not in go_output:
            raise SystemExit("Go runtime control did not execute")
        maven = str(args.maven.resolve(strict=True)) if args.maven else shutil.which("mvn", path=env["PATH"])
        if not maven:
            archive = temp / "maven.tar.gz"
            url = f"https://archive.apache.org/dist/maven/maven-3/{MAVEN_VERSION}/binaries/apache-maven-{MAVEN_VERSION}-bin.tar.gz"
            with urllib.request.urlopen(url, timeout=60) as response:
                archive.write_bytes(response.read())
            if hashlib.sha512(archive.read_bytes()).hexdigest() != MAVEN_SHA512:
                raise SystemExit("Maven distribution checksum mismatch")
            with tarfile.open(archive) as package:
                if not all(member.name.startswith(f"apache-maven-{MAVEN_VERSION}/") for member in package.getmembers()):
                    raise SystemExit("Unexpected Maven archive paths")
                package.extractall(temp, filter="data")
            maven = str(temp / f"apache-maven-{MAVEN_VERSION}/bin/mvn")
        java_output = run("java-tests", [maven, "-B", "-ntp", "--settings", str(maven_settings), "--global-settings", str(maven_settings), "-Dmaven.repo.local=" + str(temp / "maven-repository"), "test"], root / "sdk/java")
        if "Running com.mockforge.sdk.MockServerTest" not in java_output:
            raise SystemExit("Java runtime controls did not execute")
        browser = root / "sdk/browser"
        run("browser-install", ["npm", "ci", "--ignore-scripts", "--registry", "https://registry.npmjs.org"], browser)
        run("browser-tests", ["npm", "run", "test:unit", "--", "--runInBand"], browser)
        run("browser-build", ["npm", "run", "build"], browser)
        browser_env = dict(env, MOCKFORGE_MANAGEMENT_TOKEN=secrets.token_hex(32))
        child = subprocess.Popen([str(wrapper), "serve", "--http-port", "0"], cwd=server_cwd, env=browser_env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        ports = queue.Queue()

        def read_ports():
            for line in child.stdout:
                for port in re.findall(r"HTTP server listening on https?://[^:/]+:(\d+)", line):
                    ports.put(int(port))

        threading.Thread(target=read_ports, daemon=True).start()
        try:
            port = ports.get(timeout=20)
            run("browser-runtime", ["node", "tests/runtime.cjs", f"http://127.0.0.1:{port}"], browser, browser_env)
        finally:
            child.terminate()
            try:
                child.wait(timeout=10)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait(timeout=5)
        report["complete"] = True
        report["all_passed"] = True
        save()


if __name__ == "__main__":
    main()
