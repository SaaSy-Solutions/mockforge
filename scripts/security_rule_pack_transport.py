"""Owned loopback recorder, explicit target guard and bounded HTTP transport."""

import http.client
import ipaddress
import threading
from contextlib import contextmanager
from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import parse_qs, urlsplit


def target_url(value, allow_external=False):
    url = urlsplit(value)
    if (
        url.scheme not in ("http", "https")
        or not url.hostname
        or url.username is not None
        or url.password is not None
        or url.path not in ("", "/")
        or url.query
        or url.fragment
    ):
        raise ValueError("target must be an HTTP(S) origin without credentials or path")
    # Loopback literals avoid DNS rebinding and accidental discovered-host probing.
    try:
        loopback = ipaddress.ip_address(url.hostname).is_loopback
    except ValueError:
        loopback = False
    if not loopback and not allow_external:
        raise ValueError("non-loopback targets require --allow-external-target")
    if url.port is not None and url.port == 0:
        raise ValueError("target port must be 1..65535")
    return url


@contextmanager
def recorder(port=0):
    """Own synchronous recorder; raw header pairs retain receiver representation."""
    observations = {}
    lock = threading.Lock()

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            query_token = parse_qs(urlsplit(self.path).query).get(
                "__mockforge_correlation", [None]
            )[0]
            header_token = self.headers.get("X-Mockforge-Correlation")
            # Observe either surviving token so removal of one cannot fake absence.
            for token in {query_token, header_token}:
                if token and len(token) <= 100:
                    with lock:
                        observations.setdefault(token, []).append(
                            {
                                "path": self.path,
                                "headers": list(self.headers.raw_items()),
                            }
                        )
            body = b'{"receiver":"mockforge-security-rule-recorder"}'
            self.send_response(200)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *args):
            pass

    class BoundedServer(HTTPServer):
        def get_request(self):
            connection, address = super().get_request()
            connection.settimeout(2)
            return connection, address

    server = BoundedServer(("127.0.0.1", port), Handler)
    server.timeout = 1
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield server.server_address[1], observations, lock
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=2)


def request(url, path, headers, timeout):
    cls = (
        http.client.HTTPSConnection
        if url.scheme == "https"
        else http.client.HTTPConnection
    )
    connection = cls(url.hostname, url.port, timeout=timeout)
    try:
        connection.request("GET", path, headers=headers)
        response = connection.getresponse()
        # Bound response consumption; no retries or redirects.
        response.read(4096)
        return response.status
    finally:
        connection.close()
