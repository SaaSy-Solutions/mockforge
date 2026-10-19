"""MockServer implementation"""

import os
import secrets
import subprocess
import time
import requests
from typing import Optional, Dict, Any, List
from .types import MockServerConfig, ResponseStub, VerificationRequest, VerificationResult
from .verification import _auth_headers, VerificationCount, verify, verify_never, verify_at_least, verify_sequence, count
from .errors import MockServerError


def resolve_management_token(explicit: Optional[str] = None) -> str:
    """Explicit token, else $MOCKFORGE_MANAGEMENT_TOKEN, else a random hex token."""
    if explicit:
        return explicit
    from_env = os.environ.get("MOCKFORGE_MANAGEMENT_TOKEN")
    if from_env:
        return from_env
    return secrets.token_hex(32)


class MockServer:
    """Embedded mock server for testing"""

    def __init__(self, **kwargs):
        """
        Initialize a MockServer

        Args:
            port: Port to listen on (default: random)
            host: Host to bind to (default: 127.0.0.1)
            config_file: Path to MockForge config file
            openapi_spec: Path to OpenAPI specification
            management_token: Control-plane credential passed to the CLI as
                MOCKFORGE_MANAGEMENT_TOKEN and sent as
                X-MockForge-Management-Token (default: env var, else random)
        """
        self.config = MockServerConfig(**kwargs)
        self.process: Optional[subprocess.Popen] = None
        self.port = self.config.port
        self.host = self.config.host
        self.admin_port: Optional[int] = None
        self.stubs: List[ResponseStub] = []
        self.management_token = resolve_management_token(self.config.management_token)
        self._headers = _auth_headers(self.management_token)

    def start(self) -> "MockServer":
        """Start the mock server"""
        args = ["mockforge", "serve"]

        if self.config.config_file:
            args.extend(["--config", self.config.config_file])

        if self.config.openapi_spec:
            args.extend(["--spec", self.config.openapi_spec])

        if self.port:
            args.extend(["--http-port", str(self.port)])
        else:
            # Use port 0 to let OS assign a random port
            args.extend(["--http-port", "0"])

        # Enable admin API for dynamic stub management
        args.extend(["--admin", "--admin-port", "0"])

        try:
            self.process = subprocess.Popen(
                args,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,  # Enable text mode for string operations
                env={**os.environ, "MOCKFORGE_MANAGEMENT_TOKEN": self.management_token},
            )
        except FileNotFoundError as e:
            raise MockServerError.cli_not_found(e)
        except Exception as e:
            raise MockServerError.server_start_failed(f"Failed to spawn process: {str(e)}", e)

        # Start thread to parse stdout for port information
        import threading

        def read_stdout():
            """Read stdout and parse ports"""
            if self.process and self.process.stdout:
                for line in iter(self.process.stdout.readline, ''):
                    if not line:
                        break
                    self._parse_ports_from_output(line)

        def read_stderr():
            """Read stderr for error messages"""
            if self.process and self.process.stderr:
                for line in iter(self.process.stderr.readline, ''):
                    if not line:
                        break
                    self._parse_ports_from_output(line)

        stdout_thread = threading.Thread(target=read_stdout, daemon=True)
        stderr_thread = threading.Thread(target=read_stderr, daemon=True)
        stdout_thread.start()
        stderr_thread.start()

        # A failed start must not leave the subprocess running.
        try:
            self._wait_for_server()
        except BaseException:
            self.stop()
            raise

        return self

    def _parse_ports_from_output(self, output: str) -> None:
        """Parse port numbers from MockForge CLI output"""
        import re

        http_ports = re.findall(
            r'HTTP server (?:listening on https?://[^:/]+:|on port )(\d+)', output
        )
        bound = [int(port) for port in http_ports if int(port) > 0]
        if self.port == 0 and bound:
            self.port = bound[-1]
        admin_ports = re.findall(
            r'Admin UI (?:listening on https?://[^:/]+:|on port )(\d+)', output
        )
        bound = [int(port) for port in admin_ports if int(port) > 0]
        if self.admin_port is None and bound:
            self.admin_port = bound[-1]

    def _wait_for_server(self, timeout: int = 12) -> None:
        """Wait for a bound HTTP port and a successful health response."""
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if self.process is not None and self.process.poll() is not None:
                raise MockServerError.server_start_failed("MockForge exited before becoming ready")
            if self.port:
                try:
                    response = requests.get(f"{self.url()}/health", timeout=0.2)
                    if response.status_code == 200:
                        return
                except requests.exceptions.RequestException:
                    pass
            time.sleep(0.1)
        if not self.port:
            raise MockServerError.port_detection_failed()
        raise MockServerError.health_check_timeout(int(timeout), self.port)

    def stub_response(
        self,
        method: str,
        path: str,
        body: Any,
        status: int = 200,
        headers: Optional[Dict[str, str]] = None,
        latency_ms: Optional[int] = None,
    ) -> None:
        """
        Stub a response

        Args:
            method: HTTP method (GET, POST, etc.)
            path: Request path
            body: Response body
            status: HTTP status code (default: 200)
            headers: Response headers
            latency_ms: Response latency in milliseconds
        """
        stub = ResponseStub(
            method=method.upper(),
            path=path,
            body=body,
            status=status,
            headers=headers or {},
            latency_ms=latency_ms,
        )

        if not self.port:
            raise MockServerError.admin_api_error("create mock", "HTTP port is not available")
        mock_config = {
            "method": stub.method,
            "path": stub.path,
            "response": {"body": stub.body, "headers": stub.headers},
            "enabled": True,
            "status_code": stub.status,
        }
        if stub.latency_ms is not None:
            mock_config["latency_ms"] = stub.latency_ms
        try:
            response = requests.post(
                f"{self.url()}/__mockforge/api/mocks",
                json=mock_config,
                headers=self._headers,
                timeout=5.0,
            )
            response.raise_for_status()
        except requests.exceptions.RequestException as error:
            raise MockServerError.admin_api_error("create mock", str(error), error) from error
        self.stubs.append(stub)

    def clear_stubs(self) -> None:
        """Clear stubs through the HTTP management API, reporting failures."""
        from urllib.parse import quote

        if not self.port:
            raise MockServerError.admin_api_error("clear mocks", "HTTP port is not available")
        try:
            response = requests.get(f"{self.url()}/__mockforge/api/mocks", headers=self._headers, timeout=5.0)
            response.raise_for_status()
            mocks = response.json()["mocks"]
            for mock in mocks:
                response = requests.delete(
                    f"{self.url()}/__mockforge/api/mocks/{quote(str(mock['id']), safe='')}",
                    headers=self._headers,
                    timeout=5.0,
                )
                response.raise_for_status()
        except (requests.exceptions.RequestException, ValueError, KeyError, TypeError) as error:
            raise MockServerError.admin_api_error("clear mocks", str(error), error) from error
        self.stubs.clear()

    def url(self) -> str:
        """Get the server URL"""
        return f"http://{self.host}:{self.port}"

    def get_port(self) -> int:
        """Get the server port"""
        return self.port

    def is_running(self) -> bool:
        """Check if the server is running"""
        return self.process is not None and self.process.poll() is None

    def stop(self) -> None:
        """Stop the mock server"""
        if self.process:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
            self.process = None

    def __enter__(self) -> "MockServer":
        """Context manager entry"""
        return self.start()

    def __exit__(self, exc_type, exc_val, exc_tb) -> None:
        """Context manager exit"""
        self.stop()

    def __del__(self) -> None:
        """Destructor - ensure server is stopped"""
        self.stop()

    def verify(
        self,
        pattern: VerificationRequest,
        expected: Dict[str, Any],
    ) -> VerificationResult:
        """
        Verify requests against a pattern and count assertion

        Args:
            pattern: Pattern to match requests
            expected: Expected count assertion (from VerificationCount)

        Returns:
            VerificationResult with verification outcome

        Example:
            >>> from mockforge_sdk import MockServer
            >>> from mockforge_sdk.types import VerificationRequest
            >>> from mockforge_sdk.verification import VerificationCount
            >>>
            >>> server = MockServer(port=3000).start()
            >>> pattern = VerificationRequest(method="GET", path="/api/users")
            >>> result = server.verify(pattern, VerificationCount.exactly(3))
            >>> assert result.matched
        """
        return verify(self.url(), pattern, expected, management_token=self.management_token)

    def verify_never(self, pattern: VerificationRequest) -> VerificationResult:
        """
        Verify that a request was never made

        Args:
            pattern: Pattern to match requests

        Returns:
            VerificationResult with verification outcome
        """
        return verify_never(self.url(), pattern, management_token=self.management_token)

    def verify_at_least(
        self,
        pattern: VerificationRequest,
        min_count: int,
    ) -> VerificationResult:
        """
        Verify that a request was made at least N times

        Args:
            pattern: Pattern to match requests
            min_count: Minimum count

        Returns:
            VerificationResult with verification outcome
        """
        return verify_at_least(self.url(), pattern, min_count, management_token=self.management_token)

    def verify_sequence(
        self,
        patterns: List[VerificationRequest],
    ) -> VerificationResult:
        """
        Verify that requests occurred in a specific sequence

        Args:
            patterns: List of patterns to match in sequence

        Returns:
            VerificationResult with verification outcome
        """
        return verify_sequence(self.url(), patterns, management_token=self.management_token)

    def count_requests(self, pattern: VerificationRequest) -> int:
        """
        Get count of matching requests

        Args:
            pattern: Pattern to match requests

        Returns:
            Count of matching requests
        """
        return count(self.url(), pattern, management_token=self.management_token)
