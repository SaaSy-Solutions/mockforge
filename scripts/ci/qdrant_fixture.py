"""Local Qdrant fixture ownership, readiness, restart and resource cleanup."""
import json
import os
import subprocess
import time
import urllib.error
import urllib.request
import uuid

FIXTURE_KEY = "mockforge-qdrant-fixture-not-for-production"
LABEL = "dev.mockforge.qdrant-fixture"


def fixture_environment():
    """Keep native toolchain settings, excluding application/provider credentials."""
    names = ("PATH", "HOME", "CARGO_HOME", "RUSTUP_HOME", "RUSTUP_TOOLCHAIN",
             "LD_LIBRARY_PATH", "LIBRARY_PATH", "PKG_CONFIG_PATH", "CC", "CXX", "AR",
             "CFLAGS", "CXXFLAGS", "CPPFLAGS", "LDFLAGS", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS",
             "RUSTC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER",
             "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER", "CARGO_TARGET_DIR",
             "CARGO_BUILD_JOBS", "CARGO_INCREMENTAL", "CARGO_PROFILE_DEV_DEBUG",
             "CARGO_PROFILE_TEST_DEBUG", "TMPDIR")
    return {key: os.environ[key] for key in names if key in os.environ}


class QdrantFixture:
    """Mutate only resources bearing this instance's unique ownership label."""

    def __init__(self, image, environment):
        self.image = image
        self.environment = environment
        self.owner = "mockforge-qdrant-" + uuid.uuid4().hex
        self.volume = self.owner + "-data"
        self.local_context = False
        self.base = None

    def docker(self, *command):
        return subprocess.check_output(["docker", *command], env=self.environment, text=True).strip()

    def assert_owner(self, resource, name):
        values = json.loads(self.docker(resource, "inspect", name))
        labels = values[0].get("Labels", {}) if resource == "volume" else values[0]["Config"].get("Labels", {})
        if labels.get(LABEL) != self.owner:
            raise RuntimeError("refusing to mutate a fixture with different ownership")

    def wait_ready(self):
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            request = urllib.request.Request(self.base + "/readyz", headers={"api-key": FIXTURE_KEY})
            try:
                with urllib.request.urlopen(request, timeout=2) as response:
                    if response.status == 200:
                        return
            except (urllib.error.URLError, TimeoutError, ConnectionError):
                time.sleep(.2)
        raise RuntimeError("owned Qdrant fixture did not become ready")

    def refresh_address(self):
        address = self.docker("port", self.owner, "6333/tcp")
        if not address.startswith("127.0.0.1:") or "\n" in address:
            raise RuntimeError("fixture must publish exactly one loopback port")
        self.base = "http://" + address
        self.wait_ready()

    def start(self):
        context = json.loads(self.docker("context", "inspect"))
        if context:
            local = context[0].get("Endpoints", {}).get("docker", {}).get("Host", "").startswith("unix://")
        else:
            # Podman's Docker-compatible CLI has no Docker context inventory.
            info = json.loads(self.docker("info", "--format", "{{json .}}"))
            local = info.get("host", {}).get("serviceIsRemote") is False
        if not local:
            raise RuntimeError("Qdrant fixture requires a local Docker or Podman engine")
        self.local_context = True
        self.docker("pull", self.image)
        self.docker("volume", "create", "--label", LABEL + "=" + self.owner, self.volume)
        server_env = dict(self.environment, QDRANT__SERVICE__API_KEY=FIXTURE_KEY)
        subprocess.run(["docker", "run", "--detach", "--name", self.owner, "--label", LABEL + "=" + self.owner,
                        "--publish", "127.0.0.1::6333", "--mount", "type=volume,source=" + self.volume + ",target=/qdrant/storage",
                        "--env", "QDRANT__SERVICE__API_KEY", self.image], env=server_env, check=True,
                       stdout=subprocess.DEVNULL)
        self.refresh_address()

    def restart(self):
        self.assert_owner("container", self.owner)
        self.docker("restart", self.owner)
        self.refresh_address()

    def close(self):
        if not self.local_context:
            return True  # No resource mutation occurred before context validation.
        selector = "label=" + LABEL + "=" + self.owner
        for container in self.docker("ps", "-aq", "--filter", selector).split():
            self.assert_owner("container", container)
            self.docker("rm", "--force", container)
        for volume in self.docker("volume", "ls", "-q", "--filter", selector).split():
            self.assert_owner("volume", volume)
            self.docker("volume", "rm", volume)
        return not self.docker("ps", "-aq", "--filter", selector) and not self.docker("volume", "ls", "-q", "--filter", selector)
