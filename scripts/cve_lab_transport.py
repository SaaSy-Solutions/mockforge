"""Managed CVE lab transport operations."""

import http.client
import ssl
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


def unverified_context():
    """Explicit opt-in context for self-signed test certificates.

    Mirrors the CLI's --skip-tls-verify option: callers must pass this
    deliberately; the default request path verifies certificates.
    """
    context = ssl.create_default_context()
    context.check_hostname = False
    context.verify_mode = ssl.CERT_NONE
    return context


def request(base, path, headers=None, method="GET", body=None, context=None):
    url = target_url(base)
    options = {"timeout": 5}
    if url.scheme == "https":
        cls = http.client.HTTPSConnection
        # HTTPSConnection only; HTTPConnection has no context parameter.
        options["context"] = context
    else:
        cls = http.client.HTTPConnection
    conn = cls(url.hostname, url.port, **options)
    try:
        conn.request(method, path, body=body, headers=headers or {})
        response = conn.getresponse()
        payload = response.read(2 * 1024 * 1024 + 1)
        if len(payload) > 2 * 1024 * 1024:
            raise ValueError("response exceeds 2 MiB")
        return response.status, response.getheaders(), payload
    finally:
        conn.close()


MAX_FORWARDED_BODY = 2 * 1024 * 1024


@contextmanager
def receiver(port, address, backend, signature_header=None, max_body=None):
    limit = MAX_FORWARDED_BODY if max_body is None else max_body
    records = []
    lock = threading.Lock()

    class Handler(BaseHTTPRequestHandler):
        def _forward(self):
            token = parse_qs(urlsplit(self.path).query).get("__mockforge_cve", [None])[
                0
            ]
            header = self.headers.get("X-Mockforge-Cve")
            observation = {
                "method": self.command,
                "path": self.path,
                "tokens": [token, header],
                "signature": (
                    self.headers.get(signature_header) if signature_header else None
                ),
            }
            with lock:
                records.append(observation)
            body = None
            if self.command == "POST":
                length = int(self.headers.get("Content-Length") or 0)
                if length < 0 or length > limit:
                    self.send_error(413)
                    return
                body = self.rfile.read(length) if length else None
            try:
                headers = {
                    k: v
                    for k, v in self.headers.items()
                    if k.lower() not in ("host", "connection", "accept-encoding")
                }
                status, response_headers, payload = request(
                    backend,
                    self.path,
                    headers,
                    method=self.command,
                    body=body,
                )
                observation["status"] = status
                self.send_response(status)
                for name, value in response_headers:
                    if name.lower() not in (
                        "transfer-encoding",
                        "connection",
                        "content-length",
                    ):
                        self.send_header(name, value)
                self.send_header("Content-Length", str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)
            except (OSError, ValueError, http.client.HTTPException) as exc:
                observation["error"] = str(exc)
                self.send_error(502)

        def do_GET(self):
            self._forward()

        def do_POST(self):
            self._forward()

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


def control(pack, base, run_id, phase, context=None):
    """Send a pack's benign and ordinary control requests to one origin.

    The benign control exercises the pack's vulnerable endpoint without the
    injection payload; the ordinary request exercises an everyday route. Their
    expected statuses live in ``pack["controls"]`` so the classifier can tell
    "application answered normally" from "a blanket block answered instead".
    A failed control never raises: it returns what is known - the correlation
    tokens plus an ``error`` - because a control that reached a dying origin
    is evidence a crash-pack classifier must correlate, not an abort.
    """
    controls = pack["controls"]
    token = f"{run_id}-{phase}-{uuid.uuid4().hex}"
    root_token = token + "-root"
    outcome = {"token": token, "root_token": root_token}
    try:
        status, _headers, body = request(
            base,
            controls["benign_path"].replace("{token}", token),
            {"X-Mockforge-Cve": token},
            context=context,
        )
        outcome["status"] = status
        outcome["body_sha256"] = hashlib.sha256(body).hexdigest()
    except (OSError, ValueError, http.client.HTTPException) as exc:
        outcome["error"] = str(exc)
        return outcome
    try:
        outcome["root_status"] = request(
            base,
            controls["ordinary_path"].replace("{token}", root_token),
            {"X-Mockforge-Cve": root_token},
            context=context,
        )[0]
    except (OSError, ValueError, http.client.HTTPException) as exc:
        outcome["error"] = str(exc)
    return outcome


def observations(records, token):
    return [r for r in records if token in r["tokens"]]
