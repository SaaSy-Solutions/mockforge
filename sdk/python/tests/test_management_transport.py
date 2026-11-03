"""Exercise credential-bearing CRUD through actual local HTTP requests."""
import json
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

from mockforge_sdk import MockServer
from mockforge_sdk.verification import MANAGEMENT_TOKEN_HEADER


def test_crud_sends_management_credential():
    expected = "fixture-only-management-credential-1281"
    seen = []

    class Receiver(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def handle_control(self):
            if self.headers.get(MANAGEMENT_TOKEN_HEADER) != expected:
                self.send_response(401)
                self.end_headers()
                return
            seen.append((self.command, self.path))
            if self.command == "DELETE":
                self.send_response(204)
                self.end_headers()
                return
            payload = {"id": "created-control"} if self.command == "POST" else {"mocks": [{"id": "created-control"}]}
            encoded = json.dumps(payload).encode()
            self.send_response(201 if self.command == "POST" else 200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(encoded)))
            self.end_headers()
            self.wfile.write(encoded)

        do_POST = handle_control
        do_GET = handle_control
        do_DELETE = handle_control

    receiver = ThreadingHTTPServer(("127.0.0.1", 0), Receiver)
    worker = threading.Thread(target=receiver.serve_forever, daemon=True)
    worker.start()
    try:
        client = MockServer(port=receiver.server_port, management_token=expected)
        client.stub_response("GET", "/transport-control", {"working": True})
        client.clear_stubs()
        assert seen == [
            ("POST", "/__mockforge/api/mocks"),
            ("GET", "/__mockforge/api/mocks"),
            ("DELETE", "/__mockforge/api/mocks/created-control"),
        ]
    finally:
        receiver.shutdown()
        receiver.server_close()
        worker.join(timeout=5)
