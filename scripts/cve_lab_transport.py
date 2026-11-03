"""Managed CVE lab transport operations."""

import http.client
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import hashlib
import threading
from contextlib import contextmanager
from urllib.parse import parse_qs, urlsplit
import uuid


def target_url(value):
    url = urlsplit(value)
    if (
        url.scheme not in ("http", "https")
        or not url.hostname
        or url.username
        or url.password
        or url.path not in ("", "/")
        or url.query
        or url.fragment
    ):
        raise ValueError(
            "target must be an HTTP(S) origin without credentials, path or query"
        )
    if url.port == 0:
        raise ValueError("target port must be 1..65535")
    return url


def request(base, path, headers=None):
    url = target_url(base)
    cls = (
        http.client.HTTPSConnection
        if url.scheme == "https"
        else http.client.HTTPConnection
    )
    conn = cls(url.hostname, url.port, timeout=5)
    try:
        conn.request("GET", path, headers=headers or {})
        response = conn.getresponse()
        body = response.read(2 * 1024 * 1024 + 1)
        if len(body) > 2 * 1024 * 1024:
            raise ValueError("response exceeds 2 MiB")
        return response.status, response.getheaders(), body
    finally:
        conn.close()


@contextmanager
def receiver(port, address, backend):
    records = []
    lock = threading.Lock()

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            token = parse_qs(urlsplit(self.path).query).get("__mockforge_cve", [None])[
                0
            ]
            header = self.headers.get("X-Mockforge-Cve")
            observation = {
                "path": self.path,
                "tokens": [token, header],
                "signature": self.headers.get("x-middleware-subrequest"),
            }
            with lock:
                records.append(observation)
            try:
                headers = {
                    k: v
                    for k, v in self.headers.items()
                    if k.lower() not in ("host", "connection", "accept-encoding")
                }
                status, response_headers, body = request(backend, self.path, headers)
                observation["status"] = status
                self.send_response(status)
                for name, value in response_headers:
                    if name.lower() not in (
                        "transfer-encoding",
                        "connection",
                        "content-length",
                    ):
                        self.send_header(name, value)
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)
            except (OSError, ValueError, http.client.HTTPException) as exc:
                observation["error"] = str(exc)
                self.send_error(502)

        def log_message(self, *_args):
            pass

    class Server(ThreadingHTTPServer):
        daemon_threads = True

        def get_request(self):
            sock, addr = super().get_request()
            sock.settimeout(5)
            return sock, addr

    server = Server((address, port), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield records
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=6)


def control(base, run_id, phase):
    token = f"{run_id}-{phase}-{uuid.uuid4().hex}"
    status, _headers, body = request(
        base, f"/login?__mockforge_cve={token}", {"X-Mockforge-Cve": token}
    )
    root_token = token + "-root"
    root_status = request(
        base, f"/?__mockforge_cve={root_token}", {"X-Mockforge-Cve": root_token}
    )[0]
    return {
        "token": token,
        "status": status,
        "body_sha256": hashlib.sha256(body).hexdigest(),
        "root_token": root_token,
        "root_status": root_status,
    }


def observations(records, token):
    return [r for r in records if token in r["tokens"]]
