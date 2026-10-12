#!/usr/bin/env python3
"""A pinned compatibility runtime confined to this trusted Linux x64 UI job.

Node 20 is EOL. This preserves existing CI coverage; it is not a supported-LTS
upgrade. No host tool/cache/default, user configuration, or sibling state changes.
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

NODE_VERSION = "20.20.2"
NPM_VERSION = "10.8.2"
NODE_ARCHIVE = "node-v20.20.2-linux-x64.tar.xz"
NODE_TOP = "node-v20.20.2-linux-x64"
NODE_URL = "https://nodejs.org/dist/v20.20.2/" + NODE_ARCHIVE
# Official release SHASUMS: https://nodejs.org/en/blog/release/v20.20.2
NODE_SHA256 = "df770b2a6f130ed8627c9782c988fda9669fa23898329a61a871e32f965e007d"
MAX_DOWNLOAD = 64 * 1024 * 1024
MAX_EXTRACTED = 256 * 1024 * 1024
OWNER_FILE = ".ui-tool-owner.json"
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
        raise RuntimeError("Python 3.9+ is required for owned UI tool setup")
    if not shutil.rmtree.avoids_symlink_attacks:
        raise RuntimeError("Symlink-safe owned directory cleanup is required")
    if (env.get("RUNNER_OS"), env.get("RUNNER_ARCH"), sys.platform, platform.machine()) != ("Linux", "X64", "linux", "x86_64"):
        raise RuntimeError("Pinned UI runtime supports only the existing Linux x64 runner route")
    identity(env)
    parent = Path(env["RUNNER_TEMP"]).resolve(strict=True)
    if not parent.is_dir() or "\n" in str(parent) or "\r" in str(parent):
        raise RuntimeError("A valid existing RUNNER_TEMP is required")
    return parent


class OfficialRedirects(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        target = urllib.parse.urlsplit(newurl)
        if target.scheme != "https" or target.hostname != "nodejs.org" or target.port not in (None, 443) or target.username or target.password:
            raise RuntimeError("Refusing a redirect outside the official HTTPS Node host")
        return super().redirect_request(req, fp, code, msg, headers, newurl)


def download(destination):
    deadline = time.monotonic() + 180
    opener = urllib.request.build_opener(OfficialRedirects())
    with opener.open(NODE_URL, timeout=20) as response, destination.open("xb") as output:
        length = response.headers.get("Content-Length")
        if length and int(length) > MAX_DOWNLOAD:
            raise RuntimeError("Node archive exceeds the bounded download size")
        size = 0
        while True:
            if time.monotonic() > deadline:
                raise RuntimeError("Node download exceeded its 180-second budget")
            data = response.read(64 * 1024)
            if not data:
                break
            size += len(data)
            if size > MAX_DOWNLOAD:
                raise RuntimeError("Node archive exceeds the bounded download size")
            output.write(data)


def extract_verified(archive, root):
    if archive.stat().st_size > MAX_DOWNLOAD or hashlib.sha256(archive.read_bytes()).hexdigest() != NODE_SHA256:
        raise RuntimeError("Official Node archive SHA256 verification failed")
    # Validate the full member set before writing. Never extract through links;
    # official relative npm/npx links are created only after all regular files.
    with tarfile.open(archive, "r:xz") as source:
        members = source.getmembers()
        if len(members) > 20000 or sum(member.size for member in members) > MAX_EXTRACTED:
            raise RuntimeError("Node archive exceeds bounded extraction limits")
        links, names = set(), set()
        for member in members:
            path = PurePosixPath(member.name)
            if path.is_absolute() or ".." in path.parts or not path.parts or path.parts[0] != NODE_TOP or member.name in names:
                raise RuntimeError("Unsafe or duplicate Node archive path")
            names.add(member.name)
            if not (member.isdir() or member.isfile() or member.issym()):
                raise RuntimeError("Unsupported Node archive member type")
            if member.issym():
                target = PurePosixPath(member.linkname)
                resolved = (root / str(path.parent) / str(target)).resolve()
                if target.is_absolute() or not resolved.is_relative_to(root / NODE_TOP):
                    raise RuntimeError("Node archive link escapes its owned runtime")
                links.add(path)
        for member in members:
            path = PurePosixPath(member.name)
            if any(parent in links for parent in path.parents):
                raise RuntimeError("Node archive member would traverse a symlink")
        for member in sorted(members, key=lambda item: item.issym()):
            destination = root / member.name
            destination.parent.mkdir(parents=True, exist_ok=True)
            if member.isdir():
                destination.mkdir(exist_ok=True)
            elif member.issym():
                destination.symlink_to(member.linkname)
            else:
                with source.extractfile(member) as contents, destination.open("xb") as output:
                    shutil.copyfileobj(contents, output)
                destination.chmod(0o755 if member.mode & 0o111 else 0o644)
    archive.unlink()


def remove_owned(root, token, env):
    parent = preflight(env)
    if root.parent != parent or not re.fullmatch(r"mockforge-ui-" + env["GITHUB_RUN_ID"] + "-" + env["GITHUB_RUN_ATTEMPT"] + r"\.[A-Za-z0-9_-]+", root.name):
        raise RuntimeError("Refusing a UI cleanup path outside this job's owned root")
    info = root.lstat()
    owner_path = root / OWNER_FILE
    if not stat.S_ISDIR(info.st_mode) or info.st_uid != os.getuid() or owner_path.is_symlink():
        raise RuntimeError("Refusing a non-owned or symlinked UI tool root")
    if owner_path.stat().st_size > 4096:
        raise RuntimeError("Invalid UI ownership receipt")
    owner = json.loads(owner_path.read_text())
    expected = {"identity": identity(env), "token": token, "device": info.st_dev, "inode": info.st_ino, "uid": info.st_uid}
    if owner != expected or not re.fullmatch(r"[0-9a-f]{32}", token):
        raise RuntimeError("UI tool ownership receipt does not match this job/root")
    shutil.rmtree(root)
    print("Owned UI tool directory removed.")


def prepare(env):
    parent = preflight(env)
    root = Path(tempfile.mkdtemp(prefix="mockforge-ui-" + env["GITHUB_RUN_ID"] + "-" + env["GITHUB_RUN_ATTEMPT"] + ".", dir=parent))
    token = uuid.uuid4().hex
    info = root.stat()
    committed = False
    owner_ready = False
    try:
        owner = {"identity": identity(env), "token": token, "device": info.st_dev, "inode": info.st_ino, "uid": info.st_uid}
        with (root / OWNER_FILE).open("x") as output:
            json.dump(owner, output)
        owner_ready = True
        for name in ("npm-cache", "pnpm-store", "cache", "config", "data", "state"):
            (root / name).mkdir()
        archive = root / NODE_ARCHIVE
        download(archive)
        extract_verified(archive, root)
        bin_dir = root / NODE_TOP / "bin"
        node_path = bin_dir / "node"
        npm_cli = root / NODE_TOP / "lib/node_modules/npm/bin/npm-cli.js"
        if not node_path.is_file() or node_path.is_symlink() or not npm_cli.is_file() or npm_cli.is_symlink() or (bin_dir / "npm").resolve() != npm_cli:
            raise RuntimeError("Pinned archive lacks the expected owned Node/npm files")
        owned_env = {"UI_TOOL_ROOT": str(root), "UI_TOOL_ID": token, "npm_config_cache": str(root / "npm-cache"),
                     "npm_config_store_dir": str(root / "pnpm-store"), "XDG_CACHE_HOME": str(root / "cache"),
                     "XDG_CONFIG_HOME": str(root / "config"), "XDG_DATA_HOME": str(root / "data"), "XDG_STATE_HOME": str(root / "state")}
        process_env = {**env, **owned_env, "PATH": str(bin_dir) + os.pathsep + env.get("PATH", "")}
        node = subprocess.check_output([str(bin_dir / "node"), "--version"], env=process_env, text=True, timeout=10).strip()
        npm = subprocess.check_output([str(bin_dir / "node"), str(root / NODE_TOP / "lib/node_modules/npm/bin/npm-cli.js"), "--version"], env=process_env, text=True, timeout=10).strip()
        print("Owned UI runtime: node=" + node + " npm=" + npm, flush=True)
        if node != "v" + NODE_VERSION or npm != NPM_VERSION:
            raise RuntimeError("Pinned Node/npm version verification failed")
        with open(env["GITHUB_ENV"], "a") as output:
            output.write("".join(key + "=" + value + "\n" for key, value in owned_env.items()))
        with open(env["GITHUB_PATH"], "a") as output:
            output.write(str(bin_dir) + "\n")
        with open(env["GITHUB_OUTPUT"], "a") as output:
            output.write("pnpm_dest=" + str(root / "setup-pnpm") + "\n")
        committed = True
        return root, token
    finally:
        if not committed:
            # First termination starts cleanup; a repeated TERM must not interrupt
            # the small owned-directory teardown. A hard kill remains a runner
            # temp-cleanup responsibility, not a guarantee made by this helper.
            previous = {sig: signal.signal(sig, signal.SIG_IGN) for sig in (signal.SIGTERM, signal.SIGINT)}
            try:
                if owner_ready:
                    remove_owned(root, token, env)
                else:
                    (root / OWNER_FILE).unlink(missing_ok=True)
                    root.rmdir()  # Only the directory and receipt allocated above.
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
        raise RuntimeError("Usage: ci-owned-ui-tools.py prepare|cleanup")
    if sys.argv[1] == "prepare":
        prepare(os.environ)
    else:
        signal.signal(signal.SIGTERM, signal.SIG_IGN)
        signal.signal(signal.SIGINT, signal.SIG_IGN)
        remove_owned(Path(os.environ["UI_TOOL_ROOT"]), os.environ["UI_TOOL_ID"], os.environ)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print("::error::Owned UI tool setup/cleanup failed: " + str(error), file=sys.stderr)
        sys.exit(1)
