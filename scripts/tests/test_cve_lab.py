"""Fixture contracts and opt-in real CLI/k6 wire verification for CVE canaries.

MOCKFORGE_TEST_BINARY=/absolute/path/to/mockforge python3 scripts/tests/test_cve_lab.py
The live test also requires k6 on PATH; no service outside loopback is contacted.
"""

import http.client
import json
import os
import shutil
import socket
import subprocess
import tempfile
import threading
import time
import unittest
from collections import Counter
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlsplit

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "examples/cve-lab"
BINARY = os.environ.get("MOCKFORGE_TEST_BINARY")


class FixtureTests(unittest.TestCase):
    def test_receiver_has_no_dynamic_or_vulnerable_behavior(self):
        spec = json.loads((FIXTURES / "receiver.json").read_text())
        self.assertEqual(set(spec["paths"]), {"/cve-lab/probe"})
        operation = spec["paths"]["/cve-lab/probe"]["get"]
        response = operation["responses"]["200"]["content"]["application/json"]
        self.assertEqual(response["example"], {"receiver": "mockforge-cve-lab"})
        self.assertEqual(
            response["schema"]["properties"]["receiver"]["enum"], ["mockforge-cve-lab"]
        )

    @unittest.skipUnless(
        BINARY and shutil.which("k6"),
        "Set MOCKFORGE_TEST_BINARY and put k6 on PATH for live coverage",
    )
    def test_real_cli_sends_all_canaries_to_real_mockforge_receiver(self):
        received = []
        with socket.socket() as reservation:
            reservation.bind(("127.0.0.1", 0))
            receiver_port = reservation.getsockname()[1]

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

        proxy = ThreadingHTTPServer(("127.0.0.1", 0), Recorder)
        proxy_thread = threading.Thread(target=proxy.serve_forever, daemon=True)
        proxy_thread.start()
        self.addCleanup(proxy.server_close)
        self.addCleanup(proxy.shutdown)
        with tempfile.TemporaryDirectory(prefix="mockforge-cve-lab-") as temporary:
            output = Path(temporary)
            with (output / "server.log").open("w+") as server_log:
                environment = dict(
                    os.environ,
                    MOCKFORGE_HTTP_HOST="127.0.0.1",
                    MOCKFORGE_RESPONSE_TEMPLATE_EXPAND="false",
                    TOKIO_WORKER_THREADS="2",
                )
                server = subprocess.Popen(
                    [
                        BINARY,
                        "serve",
                        "--spec",
                        str(FIXTURES / "receiver.json"),
                        "--http-port",
                        str(receiver_port),
                    ],
                    cwd=output,
                    env=environment,
                    stdout=server_log,
                    stderr=subprocess.STDOUT,
                )
                try:
                    ready = False
                    for _ in range(100):
                        if server.poll() is not None:
                            break
                        connection = http.client.HTTPConnection(
                            "127.0.0.1", receiver_port, timeout=0.2
                        )
                        try:
                            connection.request("GET", "/cve-lab/probe")
                            response = connection.getresponse()
                            body = response.read()
                            if response.status == 200 and json.loads(body) == {
                                "receiver": "mockforge-cve-lab"
                            }:
                                ready = True
                                break
                        except (OSError, ValueError):
                            pass
                        finally:
                            connection.close()
                        time.sleep(0.1)
                    server_log.flush()
                    server_log.seek(0)
                    self.assertTrue(ready, server_log.read())
                    result = subprocess.run(
                        [
                            BINARY,
                            "bench",
                            "--target",
                            f"http://127.0.0.1:{proxy.server_port}",
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
                        ],
                        cwd=output,
                        env=environment,
                        capture_output=True,
                        text=True,
                        timeout=60,
                    )
                    self.assertEqual(
                        result.returncode, 0, result.stdout + result.stderr
                    )
                    self.assertTrue(received, result.stdout)
                    counts = Counter()
                    discovery = 0
                    camel_header_spellings = set()
                    for method, path, headers in received:
                        self.assertEqual(method, "GET")
                        if path == "/__mockforge/info":
                            discovery += 1
                            continue
                        uri = urlsplit(path)
                        self.assertEqual(uri.path, "/cve-lab/probe")
                        case = parse_qs(uri.query)["case"][0]
                        camel_header_spellings.update(
                            key
                            for key in headers
                            if key.lower() == "camelbeanmethodname"
                        )
                        headers = {key.lower(): value for key, value in headers.items()}
                        self.assertEqual(headers["x-mockforge-lab-case"], case)
                        counts[case] += 1
                        if case == "nextjs-header":
                            self.assertEqual(
                                headers["x-middleware-subrequest"],
                                "middleware:" * 4 + "middleware",
                            )
                        elif case == "camel-header":
                            self.assertEqual(
                                headers["camelbeanmethodname"], "mockforgeCanary"
                            )
                        elif case == "camel-parameter":
                            self.assertEqual(
                                parse_qs(uri.query)["cAmelBeanMethodName"],
                                ["mockforgeCanary"],
                            )
                        else:
                            self.assertEqual(case, "normal-control")
                            self.assertNotIn("x-middleware-subrequest", headers)
                            self.assertNotIn("camelbeanmethodname", headers)
                    self.assertEqual(
                        set(counts),
                        {
                            "normal-control",
                            "nextjs-header",
                            "camel-header",
                            "camel-parameter",
                        },
                    )
                    summary = json.loads((output / "bench/summary.json").read_text())
                    self.assertEqual(
                        summary["metrics"]["http_req_failed"]["values"]["rate"], 0
                    )
                    self.assertEqual(summary["metrics"]["checks"]["values"]["fails"], 0)
                    print(
                        f"Wire observations: {dict(counts)}; discovery probes={discovery}; "
                        f"Camel header spellings={sorted(camel_header_spellings)}; all HTTP requests and k6 checks passed"
                    )
                finally:
                    server.terminate()
                    try:
                        server.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        server.kill()
                        server.wait(timeout=5)


if __name__ == "__main__":
    unittest.main()
