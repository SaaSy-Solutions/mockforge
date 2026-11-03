#!/usr/bin/env python3
"""Exercise the installed CLI against the real pinned lab and three local LB policies."""

import argparse
import http.client
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import socket
import subprocess
import threading
from urllib.parse import urlsplit


def free_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def verify(binary, output, engine):
    output.mkdir(parents=True, exist_ok=False)
    for mode, expected in (
        ("block-signature", "pass"),
        ("forward", "fail"),
        ("block-root", "inconclusive"),
    ):
        origin_port = free_port()

        class Proxy(BaseHTTPRequestHandler):
            def do_GET(self):
                block = (
                    mode == "block-signature"
                    and self.headers.get("x-middleware-subrequest")
                ) or (mode == "block-root" and urlsplit(self.path).path == "/")
                if block:
                    self.send_response(403)
                    self.send_header("Content-Length", "0")
                    self.end_headers()
                    return
                conn = http.client.HTTPConnection("127.0.0.1", origin_port, timeout=10)
                try:
                    conn.request("GET", self.path, headers=dict(self.headers))
                    response = conn.getresponse()
                    body = response.read()
                    self.send_response(response.status)
                    for name, value in response.getheaders():
                        if name.lower() not in (
                            "connection",
                            "transfer-encoding",
                            "content-length",
                        ):
                            self.send_header(name, value)
                    self.send_header("Content-Length", str(len(body)))
                    self.end_headers()
                    self.wfile.write(body)
                finally:
                    conn.close()

            def log_message(self, *_args):
                pass

        server = ThreadingHTTPServer(("127.0.0.1", 0), Proxy)
        worker = threading.Thread(target=server.serve_forever, daemon=True)
        worker.start()
        try:
            result = subprocess.run(
                [
                    str(binary),
                    "cve",
                    "--engine",
                    engine,
                    "run",
                    "CVE-2025-29927",
                    "--target",
                    f"http://127.0.0.1:{server.server_port}",
                    "--origin-port",
                    str(origin_port),
                    "--output-dir",
                    str(output / mode),
                ],
                timeout=420,
            )
            report = json.loads((output / mode / "report.json").read_text())
            assert report["verdict"] == expected, report
            assert (result.returncode == 0) == (expected == "pass"), result.returncode
            print(f"Verified {mode}: {expected}", flush=True)
        finally:
            server.shutdown()
            server.server_close()
            worker.join()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--engine", choices=("docker", "podman"), default="docker")
    args = parser.parse_args()
    verify(args.binary.resolve(), args.output_dir.resolve(), args.engine)


if __name__ == "__main__":
    main()
