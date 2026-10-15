#!/usr/bin/env python3
"""Checksum-pinned official audit tools in an owned trusted Linux x64 job root.

Release-asset SHA256 digests were verified via each publisher's GitHub API.
No checksum-sidecar verification is claimed; no shared tool/config/cache changes.
"""
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import platform
import re
import shutil
import signal
import stat
import subprocess
import sys
import tarfile
import tempfile
import time
import urllib.parse
import urllib.request
import uuid

SPECS = (
    {"tool": "cargo-audit", "version": "0.22.2", "top": "cargo-audit-x86_64-unknown-linux-musl-v0.22.2",
     "url": "https://github.com/rustsec/rustsec/releases/download/cargo-audit/v0.22.2/cargo-audit-x86_64-unknown-linux-musl-v0.22.2.tgz",
     "sha256": "7fb9497f8594b389e5fce5ef9b92db08432996895b2e0c5a0167a69ed445c428",
     "files": ("cargo-audit", "README.md", "CHANGELOG.md", "LICENSE-APACHE", "LICENSE-MIT")},
    {"tool": "cargo-deny", "version": "0.20.2", "top": "cargo-deny-0.20.2-x86_64-unknown-linux-musl",
     "url": "https://github.com/EmbarkStudios/cargo-deny/releases/download/0.20.2/cargo-deny-0.20.2-x86_64-unknown-linux-musl.tar.gz",
     "sha256": "9f12ed4c49936e09b48bf862b595cde2fe64fcbd9d74dfacac6131ca824c8d5f",
     "files": ("cargo-deny", "README.md", "LICENSE-APACHE", "LICENSE-MIT")},
)
MAX_DOWNLOAD = 32 * 1024 * 1024
MAX_EXTRACTED = 128 * 1024 * 1024
OWNER_FILE = ".security-tool-owner.json"
IDENTITY_FIELDS = ("GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT", "GITHUB_JOB", "RUNNER_NAME")


def identity(env):
    result = {key: env.get(key, "") for key in IDENTITY_FIELDS}
    if not all(result.values()) or any("\n" in value or "\r" in value for value in result.values()):
        raise RuntimeError("Complete single-line job ownership identity is required")
    if not all(re.fullmatch(r"[0-9]+", result[key]) for key in IDENTITY_FIELDS[:2]):
        raise RuntimeError("Numeric run and attempt identity is required")
    return result


def preflight(env):
    if sys.version_info < (3, 9):
        raise RuntimeError("Python 3.9+ is required for owned security tool setup")
    if not shutil.rmtree.avoids_symlink_attacks:
        raise RuntimeError("Symlink-safe owned directory cleanup is required")
    if (env.get("RUNNER_OS"), env.get("RUNNER_ARCH"), sys.platform, platform.machine()) != ("Linux", "X64", "linux", "x86_64"):
        raise RuntimeError("Pinned security tools support only the existing Linux x64 runner route")
    identity(env)
    cargo_home = env.get("CARGO_HOME", "")
    if not Path(cargo_home).is_absolute() or any(char in cargo_home for char in ("\n", "\r", os.pathsep)):
        raise RuntimeError("Existing absolute CARGO_HOME is required; it will not be changed")
    parent = Path(env["RUNNER_TEMP"]).resolve(strict=True)
    if not parent.is_dir() or "\n" in str(parent) or "\r" in str(parent):
        raise RuntimeError("A valid existing RUNNER_TEMP is required")
    return parent


class OfficialRedirects(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        target = urllib.parse.urlsplit(newurl)
        if target.scheme != "https" or target.hostname not in ("github.com", "release-assets.githubusercontent.com") or target.port not in (None, 443) or target.username or target.password:
            raise RuntimeError("Refusing a redirect outside the official HTTPS release hosts")
        return super().redirect_request(req, fp, code, msg, headers, newurl)


def download(destination, url):
    deadline = time.monotonic() + 180
    opener = urllib.request.build_opener(OfficialRedirects())
    with opener.open(url, timeout=20) as response, destination.open("xb") as output:
        length = response.headers.get("Content-Length")
        if length and int(length) > MAX_DOWNLOAD:
            raise RuntimeError("Security archive exceeds the bounded download size")
        size = 0
        while True:
            if time.monotonic() > deadline:
                raise RuntimeError("Security download exceeded its 180-second budget")
            data = response.read(64 * 1024)
            if not data:
                break
            size += len(data)
            if size > MAX_DOWNLOAD:
                raise RuntimeError("Security archive exceeds the bounded download size")
            output.write(data)


def extract_verified(archive, root, spec):
    if archive.stat().st_size > MAX_DOWNLOAD or hashlib.sha256(archive.read_bytes()).hexdigest() != spec["sha256"]:
        raise RuntimeError(spec["tool"] + " official archive SHA256 verification failed")
    with tarfile.open(archive, "r:gz") as source:
        members = source.getmembers()
        if len(members) > 32 or sum(member.size for member in members) > MAX_EXTRACTED:
            raise RuntimeError("Security archive exceeds bounded extraction limits")
        names = set()
        for member in members:
            path = PurePosixPath(member.name)
            valid_dir = member.isdir() and path.parts == (spec["top"],)
            valid_file = member.isfile() and len(path.parts) == 2 and path.parts[0] == spec["top"] and path.parts[1] in spec["files"]
            if path.is_absolute() or ".." in path.parts or str(path) in names or not (valid_dir or valid_file):
                raise RuntimeError("Unexpected, unsafe, duplicate or non-regular security archive member")
            names.add(str(path))
        expected_binary = spec["top"] + "/" + spec["tool"]
        if expected_binary not in names:
            raise RuntimeError("Security archive lacks its expected binary")
        for member in members:
            if member.isdir():
                continue
            name = PurePosixPath(member.name).name
            destination = root / "bin" / name if name == spec["tool"] else root / "licenses" / spec["tool"] / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            with source.extractfile(member) as contents, destination.open("xb") as output:
                shutil.copyfileobj(contents, output)
            destination.chmod(0o755 if name == spec["tool"] else 0o644)
    archive.unlink()


def remove_owned(root, token, env):
    parent = preflight(env)
    if root.parent != parent or not re.fullmatch(r"mockforge-security-" + env["GITHUB_RUN_ID"] + "-" + env["GITHUB_RUN_ATTEMPT"] + r"\.[A-Za-z0-9_-]+", root.name):
        raise RuntimeError("Refusing a security cleanup path outside this job's owned root")
    info = root.lstat()
    owner_path = root / OWNER_FILE
    if not stat.S_ISDIR(info.st_mode) or info.st_uid != os.getuid() or owner_path.is_symlink():
        raise RuntimeError("Refusing a non-owned or symlinked security tool root")
    if owner_path.stat().st_size > 4096:
        raise RuntimeError("Invalid security ownership receipt")
    owner = json.loads(owner_path.read_text())
    expected = {"identity": identity(env), "token": token, "device": info.st_dev, "inode": info.st_ino, "uid": info.st_uid}
    if owner != expected or not re.fullmatch(r"[0-9a-f]{32}", token):
        raise RuntimeError("security tool ownership receipt does not match this job/root")
    shutil.rmtree(root)
    print("Owned security tool directory removed.")


def prepare(env):
    parent = preflight(env)
    root = Path(tempfile.mkdtemp(prefix="mockforge-security-" + env["GITHUB_RUN_ID"] + "-" + env["GITHUB_RUN_ATTEMPT"] + ".", dir=parent))
    token = uuid.uuid4().hex
    info = root.stat()
    committed = False
    owner_ready = False
    try:
        owner = {"identity": identity(env), "token": token, "device": info.st_dev, "inode": info.st_ino, "uid": info.st_uid}
        with (root / OWNER_FILE).open("x") as output:
            json.dump(owner, output)
        owner_ready = True
        (root / "bin").mkdir()
        for spec in SPECS:
            archive = root / (spec["tool"] + ".tar.gz")
            download(archive, spec["url"])
            extract_verified(archive, root, spec)
        cargo_bin = str(Path(env["CARGO_HOME"]) / "bin")
        process_env = {**env, "PATH": str(root / "bin") + os.pathsep + cargo_bin + os.pathsep + env.get("PATH", "")}
        for spec in SPECS:
            result = subprocess.run([str(root / "bin" / spec["tool"]), "--version"], env=process_env, capture_output=True, text=True, timeout=10)
            version = result.stdout.strip()
            if result.stderr:
                print(result.stderr.strip()[:2048], file=sys.stderr, flush=True)
            print("Owned security version: " + spec["tool"] + "=" + version + " exit=" + str(result.returncode), flush=True)
            if result.returncode or version != spec["tool"] + " " + spec["version"]:
                raise RuntimeError("Pinned " + spec["tool"] + " version verification failed")
        with open(env["GITHUB_ENV"], "a") as output:
            output.write("SECURITY_TOOL_ROOT=" + str(root) + "\nSECURITY_TOOL_ID=" + token + "\n")
        # Runner2.337 reverses GITHUB_PATH entries. Writing the home bin first
        # leaves our owned bin first. Cargo1.96 sees the explicit home bin later
        # and does not implicitly prepend it over the job-owned tools.
        with open(env["GITHUB_PATH"], "a") as output:
            output.write(cargo_bin + "\n" + str(root / "bin") + "\n")
        committed = True
        return root, token
    finally:
        if not committed:
            previous = {sig: signal.signal(sig, signal.SIG_IGN) for sig in (signal.SIGTERM, signal.SIGINT)}
            try:
                if owner_ready:
                    remove_owned(root, token, env)
                else:
                    (root / OWNER_FILE).unlink(missing_ok=True)
                    root.rmdir()
            finally:
                for sig, handler in previous.items():
                    signal.signal(sig, handler)


def main():
    def terminated(signum, frame):
        signal.signal(signal.SIGTERM, signal.SIG_IGN)
        signal.signal(signal.SIGINT, signal.SIG_IGN)
        raise SystemExit(128 + signum)
    signal.signal(signal.SIGTERM, terminated)
    signal.signal(signal.SIGINT, terminated)
    if len(sys.argv) != 2 or sys.argv[1] not in ("prepare", "cleanup"):
        raise RuntimeError("Usage: ci-owned-security-tools.py prepare|cleanup")
    if sys.argv[1] == "prepare":
        prepare(os.environ)
    else:
        signal.signal(signal.SIGTERM, signal.SIG_IGN)
        signal.signal(signal.SIGINT, signal.SIG_IGN)
        remove_owned(Path(os.environ["SECURITY_TOOL_ROOT"]), os.environ["SECURITY_TOOL_ID"], os.environ)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print("::error::Owned security tool setup/cleanup failed: " + str(error), file=sys.stderr)
        sys.exit(1)
