"""Tests for MockServer"""

import pytest
import requests
import shutil
from mockforge_sdk import MockServer


class TestMockServerInit:
    """Test MockServer initialization"""

    def test_creates_with_defaults(self):
        """Should create server with default config"""
        server = MockServer()
        assert server is not None
        assert server.port == 0
        assert server.host == "127.0.0.1"

    def test_creates_with_custom_port(self):
        """Should create server with custom port"""
        server = MockServer(port=3000)
        assert server.port == 3000

    def test_creates_with_custom_host(self):
        """Should create server with custom host"""
        server = MockServer(host="0.0.0.0")
        assert server.host == "0.0.0.0"


class TestMockServerURL:
    """Test URL generation"""

    def test_returns_correct_url(self):
        """Should return correct URL"""
        server = MockServer(port=3000, host="127.0.0.1")
        assert server.url() == "http://127.0.0.1:3000"


class TestMockServerIsRunning:
    """Test isRunning method"""

    def test_returns_false_before_start(self):
        """Should return False before server starts"""
        server = MockServer()
        assert server.is_running() is False


# Integration tests that require MockForge CLI
@pytest.mark.skipif(shutil.which("mockforge") is None, reason="Requires MockForge CLI on PATH")
class TestMockServerIntegration:
    """Integration tests (require MockForge CLI)"""

    def test_start_and_stop(self):
        """Should start and stop server"""
        server = MockServer(port=0)
        server.start()
        assert server.is_running() is True
        server.stop()
        assert server.is_running() is False

    def test_context_manager(self):
        """Should work as context manager"""
        with MockServer(port=0) as server:
            assert server.is_running() is True
        assert server.is_running() is False

    def test_stub_response(self):
        """Should stub a response"""
        with MockServer(port=0) as server:
            server.stub_response('GET', '/test', {'message': 'hello'}, status=202,
                                 headers={'X-SDK-Control': 'python'})
            response = requests.get(server.url() + '/test', timeout=5)
            assert response.status_code == 202
            assert response.json() == {'message': 'hello'}
            assert response.headers['X-SDK-Control'] == 'python'
            server.clear_stubs()
            assert requests.get(server.url() + '/test', timeout=5).status_code == 404


class TestManagementToken:
    """Control-plane credential handling (issue #1011)"""

    def test_explicit_token_wins(self, monkeypatch):
        monkeypatch.setenv("MOCKFORGE_MANAGEMENT_TOKEN", "e" * 40)
        server = MockServer(management_token="x" * 40)
        assert server.management_token == "x" * 40

    def test_env_token_used_when_no_option(self, monkeypatch):
        monkeypatch.setenv("MOCKFORGE_MANAGEMENT_TOKEN", "e" * 40)
        assert MockServer().management_token == "e" * 40

    def test_random_token_generated_per_instance(self, monkeypatch):
        monkeypatch.delenv("MOCKFORGE_MANAGEMENT_TOKEN", raising=False)
        a, b = MockServer().management_token, MockServer().management_token
        assert len(a) >= 32 and all(c in "0123456789abcdef" for c in a)
        assert a != b

    def test_token_passed_to_child_env(self, monkeypatch):
        import subprocess as sp

        captured = {}

        def fake_popen(args, **kwargs):
            captured.update(kwargs)
            raise FileNotFoundError("mockforge")

        monkeypatch.setattr(sp, "Popen", fake_popen)
        server = MockServer(management_token="t" * 40)
        with pytest.raises(Exception):
            server.start()
        assert captured["env"]["MOCKFORGE_MANAGEMENT_TOKEN"] == "t" * 40

    def test_header_sent_on_mock_creation_and_verification(self):
        import json
        import threading
        from http.server import BaseHTTPRequestHandler, HTTPServer
        from mockforge_sdk.types import VerificationRequest
        from mockforge_sdk.verification import VerificationCount

        seen = []

        class Handler(BaseHTTPRequestHandler):
            def do_POST(self):
                length = int(self.headers.get("Content-Length", 0))
                self.rfile.read(length)
                seen.append(
                    (self.path, self.headers.get("X-MockForge-Management-Token"))
                )
                if self.path.startswith("/api/verification"):
                    payload = {"matched": True, "count": 0, "expected": {}, "matches": []}
                else:
                    payload = {"id": "m1"}
                body = json.dumps(payload).encode()
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)

            def log_message(self, *args):
                pass

        httpd = HTTPServer(("127.0.0.1", 0), Handler)
        port = httpd.server_address[1]
        thread = threading.Thread(target=httpd.serve_forever, daemon=True)
        thread.start()
        try:
            token = "k" * 48
            server = MockServer(port=port, management_token=token)
            server.admin_port = port
            server.stub_response("GET", "/x", {"ok": True})
            server.verify(VerificationRequest(path="/x"), VerificationCount.never())
        finally:
            httpd.shutdown()
        assert [p for p, _ in seen] == ["/__mockforge/api/mocks", "/api/verification/verify"]
        assert all(t == token for _, t in seen)
