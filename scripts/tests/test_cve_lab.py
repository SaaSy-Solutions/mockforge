"""Fixture contracts and opt-in real CLI/k6 loopback verification.

Set MOCKFORGE_TEST_BINARY to an absolute CLI path and put k6 on PATH for the
live test. Only owned loopback servers and process groups are used.
"""

import http.client
import json
import os
import shutil
import signal
import socket
import subprocess
import tempfile
import threading
import time
import unittest
from collections import Counter
from contextlib import contextmanager
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlsplit

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "examples/cve-lab"
BINARY = os.environ.get("MOCKFORGE_TEST_BINARY")
EXPECTED = {
    "normal-control": {"uri": "/cve-lab/probe?case=normal-control"},
    "nextjs-header": {
        "uri": "/cve-lab/probe?case=nextjs-header",
        "x-middleware-subrequest": "middleware:" * 4 + "middleware",
    },
    "camel-header": {
        "uri": "/cve-lab/probe?case=camel-header",
        "camelbeanmethodname": "mockforgeCanary",
    },
    "camel-parameter": {
        "uri": "/cve-lab/probe?case=camel-parameter&cAmelBeanMethodName=mockforgeCanary",
    },
}


@contextmanager
def owned_process(command, **kwargs):
    """Stop the entire process group, including k6, even on a failed test."""
    process = subprocess.Popen(command, start_new_session=True, **kwargs)
    try:
        yield process
    finally:
        try:
            os.killpg(process.pid, signal.SIGTERM)
            process.wait(timeout=10)
        except ProcessLookupError:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait(timeout=5)


def receiver_ready(port):
    """Require the actual OpenAPI response, not just an open TCP port."""
    connection = http.client.HTTPConnection("127.0.0.1", port, timeout=0.2)
    try:
        connection.request("GET", "/cve-lab/probe")
        response = connection.getresponse()
        return response.status == 200 and json.loads(response.read()) == {
            "receiver": "mockforge-cve-lab"
        }
    except (OSError, ValueError):
        return False
    finally:
        connection.close()


def wait_for_receiver(process, port):
    """Bound startup time and fail immediately if the owned server exits."""
    for _ in range(100):
        if process.poll() is not None:
            return False
        if receiver_ready(port):
            return True
        time.sleep(0.1)
    return False


def recorder_handler(receiver_port, received):
    class Recorder(BaseHTTPRequestHandler):
        def do_GET(self):
            received.append((self.command, self.path, dict(self.headers)))
            connection = http.client.HTTPConnection(
                "127.0.0.1", receiver_port, timeout=5
            )
            try:
                connection.request("GET", self.path, headers=dict(self.headers))
                response = connection.getresponse()
                body = response.read()
                self.send_response(response.status)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)
            finally:
                connection.close()

        def log_message(self, *_args):
            pass

    return Recorder


@contextmanager
def recording_proxy(receiver_port, received):
    proxy = ThreadingHTTPServer(
        ("127.0.0.1", 0), recorder_handler(receiver_port, received)
    )
    thread = threading.Thread(target=proxy.serve_forever, daemon=True)
    thread.start()
    try:
        yield proxy
    finally:
        proxy.shutdown()
        proxy.server_close()
        thread.join(timeout=5)


def run_benchmark(port, output, environment):
    command = [
        BINARY,
        "bench",
        "--target",
        f"http://127.0.0.1:{port}",
        "--wafbench-dir",
        str(FIXTURES / "traffic.yaml"),
        "--wafbench-verbatim",
        "--wafbench-cycle-all",
        "--scenario",
        "constant",
        "--vus",
        "1",
        "--duration",
        "4s",
        "--output",
        str(output / "bench"),
    ]
    with owned_process(
        command,
        cwd=output,
        env=environment,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    ) as process:
        stdout, stderr = process.communicate(timeout=60)
        return process.returncode, stdout + stderr


class FixtureTests(unittest.TestCase):
    def test_receiver_has_no_dynamic_or_vulnerable_behavior(self):
        spec = json.loads((FIXTURES / "receiver.json").read_text())
        self.assertEqual(set(spec["paths"]), {"/cve-lab/probe"})
        response = spec["paths"]["/cve-lab/probe"]["get"]["responses"]["200"][
            "content"
        ]["application/json"]
        self.assertEqual(response["example"], {"receiver": "mockforge-cve-lab"})
        self.assertEqual(
            response["schema"]["properties"]["receiver"]["enum"], ["mockforge-cve-lab"]
        )

    def assert_wire_requests(self, received):
        """Check every fixture field and reject contamination of the control."""
        cases = [item for item in received if item[1] != "/__mockforge/info"]
        counts = Counter()
        for method, path, headers in cases:
            self.assertEqual(method, "GET")
            case = parse_qs(urlsplit(path).query)["case"][0]
            self.assertIn(case, EXPECTED)
            self.assertEqual(path, EXPECTED[case]["uri"])
            headers = {key.lower(): value for key, value in headers.items()}
            self.assertEqual(headers["x-mockforge-lab-case"], case)
            self.assertEqual(
                headers.get("x-middleware-subrequest"),
                EXPECTED[case].get("x-middleware-subrequest"),
            )
            self.assertEqual(
                headers.get("camelbeanmethodname"),
                EXPECTED[case].get("camelbeanmethodname"),
            )
            counts[case] += 1
        self.assertEqual(set(counts), set(EXPECTED))
        print(
            f"Wire observations: {dict(counts)}; discovery probes={len(received) - len(cases)}"
        )

    def assert_k6_summary(self, output):
        metrics = json.loads((output / "bench/summary.json").read_text())["metrics"]
        self.assertEqual(metrics["http_req_failed"]["values"]["rate"], 0)
        self.assertEqual(metrics["checks"]["values"]["fails"], 0)

    @unittest.skipUnless(
        BINARY and shutil.which("k6") and os.name == "posix",
        "Live coverage requires POSIX, MOCKFORGE_TEST_BINARY and k6 on PATH",
    )
    def test_real_cli_sends_all_canaries_to_real_mockforge_receiver(self):
        with socket.socket() as reservation:
            reservation.bind(("127.0.0.1", 0))
            receiver_port = reservation.getsockname()[1]
        received = []
        environment = dict(
            os.environ,
            MOCKFORGE_HTTP_HOST="127.0.0.1",
            MOCKFORGE_RESPONSE_TEMPLATE_EXPAND="false",
            TOKIO_WORKER_THREADS="2",
        )
        with tempfile.TemporaryDirectory(prefix="mockforge-cve-lab-") as temporary:
            output = Path(temporary)
            command = [
                BINARY,
                "serve",
                "--spec",
                str(FIXTURES / "receiver.json"),
                "--http-port",
                str(receiver_port),
            ]
            with (output / "server.log").open("w+") as log:
                with owned_process(
                    command,
                    cwd=output,
                    env=environment,
                    stdout=log,
                    stderr=subprocess.STDOUT,
                ) as server:
                    ready = wait_for_receiver(server, receiver_port)
                    log.seek(0)
                    self.assertTrue(ready, log.read())
                    with recording_proxy(receiver_port, received) as proxy:
                        code, diagnostic = run_benchmark(
                            proxy.server_port, output, environment
                        )
                    self.assertEqual(code, 0, diagnostic)
                    self.assert_wire_requests(received)
                    self.assert_k6_summary(output)


if __name__ == "__main__":
    unittest.main()
