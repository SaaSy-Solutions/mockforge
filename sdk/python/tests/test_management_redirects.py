"""Real loopback transport regressions; all credentials and endpoints are synthetic."""
import json
import threading
import unittest
from contextlib import contextmanager
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

from mockforge_sdk.mock_server import MockServer
from mockforge_sdk.verification import _make_verification_request
from mockforge_sdk.errors import MockServerError

TOKEN = "synthetic-redirect-regression-only"

@contextmanager
def server(handler):
    httpd = ThreadingHTTPServer(("127.0.0.1", 0), handler)
    thread = threading.Thread(target=httpd.serve_forever, kwargs={"poll_interval": 0.01}, daemon=True)
    thread.start()
    try:
        yield httpd
    finally:
        httpd.shutdown()
        httpd.server_close()
        thread.join(timeout=2)
        if thread.is_alive():
            raise AssertionError("owned fixture did not stop")

class ManagementRedirectTests(unittest.TestCase):
    def exercise(self, code, operation):
        received = []
        class Sink(BaseHTTPRequestHandler):
            def handle_request(self):
                received.append(self.headers.get("X-MockForge-Management-Token"))
                self.send_response(200)
                self.end_headers()
                payload = ({"matched": True, "count": 1, "expected": {}, "matches": []}
                           if operation == "verify" else {"mocks": []})
                self.wfile.write(json.dumps(payload).encode())
            do_GET = do_POST = do_DELETE = handle_request
            def log_message(self, *_):
                pass
        with server(Sink) as sink:
            class Redirect(BaseHTTPRequestHandler):
                def handle_request(self):
                    length = int(self.headers.get("Content-Length", 0))
                    self.rfile.read(length)
                    if operation == "delete" and self.command == "GET":
                        self.send_response(200)
                        self.end_headers()
                        self.wfile.write(b'{"mocks": [{"id": "owned"}]}')
                    else:
                        self.send_response(code)
                        self.send_header("Location", f"http://127.0.0.1:{sink.server_port}/sink")
                        self.end_headers()
                do_GET = do_POST = do_DELETE = handle_request
                def log_message(self, *_):
                    pass
            with server(Redirect) as origin:
                client = MockServer(management_token=TOKEN)
                client.host, client.port = "127.0.0.1", origin.server_port
                if operation == "verify":
                    result = _make_verification_request(client.url(), "verify", {}, management_token=TOKEN)
                    self.assertFalse(result.matched)
                    self.assertIn("redirect", result.error_message.lower())
                else:
                    with self.assertRaises(MockServerError):
                        if operation == "create":
                            client.stub_response("GET", "/example", {})
                        else:
                            client.clear_stubs()
                self.assertEqual([], received, "credentialed request followed a redirect")
                self.assertEqual([], client.stubs)

    def test_non_redirected_crud_retains_credential_transport(self):
        from test_management_transport import test_crud_sends_management_credential
        test_crud_sends_management_credential()

    def test_management_redirects_are_refused(self):
        for code in (301, 302, 303, 307, 308):
            for operation in ("create", "list", "delete", "verify"):
                with self.subTest(status=code, operation=operation):
                    self.exercise(code, operation)

if __name__ == "__main__":
    unittest.main()
