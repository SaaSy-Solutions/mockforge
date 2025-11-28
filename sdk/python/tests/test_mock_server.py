"""Tests for MockServer"""

import pytest
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
@pytest.mark.skip(reason="Requires MockForge CLI to be installed")
class TestMockServerIntegration:
    """Integration tests (require MockForge CLI)"""

    def test_start_and_stop(self):
        """Should start and stop server"""
        server = MockServer(port=3456)
        server.start()
        assert server.is_running() is True
        server.stop()
        assert server.is_running() is False

    def test_context_manager(self):
        """Should work as context manager"""
        with MockServer(port=3457) as server:
            assert server.is_running() is True
        assert server.is_running() is False

    def test_stub_response(self):
        """Should stub a response"""
        with MockServer(port=3458) as server:
            server.stub_response('GET', '/test', {'message': 'hello'})
            # Would test actual HTTP request here
            assert True
