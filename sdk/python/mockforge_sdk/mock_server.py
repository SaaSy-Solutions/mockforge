"""MockServer implementation"""

import subprocess
import time
import requests
from typing import Optional, Dict, Any, List
from .types import MockServerConfig, ResponseStub, VerificationRequest, VerificationResult
from .verification import VerificationCount, verify, verify_never, verify_at_least, verify_sequence, count
from .errors import MockServerError


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
        """
        self.config = MockServerConfig(**kwargs)
        self.process: Optional[subprocess.Popen] = None
        self.port = self.config.port
        self.host = self.config.host
        self.admin_port: Optional[int] = None
        self.stubs: List[ResponseStub] = []

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
            )
        except FileNotFoundError as e:
            raise MockServerError.cli_not_found(e)
        except Exception as e:
            raise MockServerError.server_start_failed(f"Failed to spawn process: {str(e)}", e)

        # Start thread to parse stdout for port information
        import threading
        stdout_buffer = []
        stderr_buffer = []

        def read_stdout():
            """Read stdout and parse ports"""
            if self.process and self.process.stdout:
                for line in iter(self.process.stdout.readline, ''):
                    if not line:
                        break
                    stdout_buffer.append(line)
                    self._parse_ports_from_output(''.join(stdout_buffer))

        def read_stderr():
            """Read stderr for error messages"""
            if self.process and self.process.stderr:
                for line in iter(self.process.stderr.readline, ''):
                    if not line:
                        break
                    stderr_buffer.append(line)

        stdout_thread = threading.Thread(target=read_stdout, daemon=True)
        stderr_thread = threading.Thread(target=read_stderr, daemon=True)
        stdout_thread.start()
        stderr_thread.start()

        # Wait for server to start
        self._wait_for_server()

        return self

    def _parse_ports_from_output(self, output: str) -> None:
        """Parse port numbers from MockForge CLI output"""
        import re

        # Parse HTTP server port
        # Pattern: "📡 HTTP server listening on http://localhost:PORT" or "📡 HTTP server on port PORT"
        http_port_match = re.search(
            r'HTTP server (?:listening on http://[^:]+:|on port )(\d+)',
            output
        )
        if http_port_match and self.port == 0:
            detected_port = int(http_port_match.group(1))
            if detected_port > 0:
                self.port = detected_port

        # Parse Admin UI port
        # Pattern: "🎛️ Admin UI listening on http://HOST:PORT" or "🎛️ Admin UI on port PORT"
        admin_port_match = re.search(
            r'Admin UI (?:listening on http://[^:]+:|on port )(\d+)',
            output
        )
        if admin_port_match and self.admin_port is None:
            detected_admin_port = int(admin_port_match.group(1))
            if detected_admin_port > 0:
                self.admin_port = detected_admin_port

    def _wait_for_server(self, timeout: int = 12) -> None:
        """Wait for the server to be ready"""
        start_time = time.time()

        # If port is 0, wait for it to be detected from stdout
        port_detection_attempts = 0
        max_port_detection_attempts = 20

        while time.time() - start_time < timeout:
            # If port is 0, wait for it to be detected from stdout
            if self.port == 0 and port_detection_attempts < max_port_detection_attempts:
                port_detection_attempts += 1
                time.sleep(0.2)
                continue

            # If port is still 0 after detection attempts, raise standardized error
            if self.port == 0:
                raise MockServerError.port_detection_failed()

            try:
                response = requests.get(
                    f"http://{self.host}:{self.port}/health",
                    timeout=0.2
                )
                if response.status_code == 200:
                    return
            except requests.exceptions.RequestException:
                time.sleep(0.2)

        raise MockServerError.health_check_timeout(
            int(timeout * 1000),  # Convert to milliseconds
            self.port
        )

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

        self.stubs.append(stub)

        # If admin API is available, use it to add the stub dynamically
        if self.admin_port:
            try:
                # Convert ResponseStub to MockConfig format expected by Admin API
                mock_config = {
                    "id": "",  # Empty ID - server will generate one
                    "name": f"{stub.method} {stub.path}",  # Generate a name from method and path
                    "method": stub.method,
                    "path": stub.path,
                    "response": {
                        "body": stub.body,
                        "headers": stub.headers if stub.headers else None,
                    },
                    "enabled": True,
                    "latency_ms": stub.latency_ms,
                    "status_code": stub.status if stub.status != 200 else None,
                }
                # Remove None values
                mock_config = {k: v for k, v in mock_config.items() if v is not None}
                if not mock_config["response"]["headers"]:
                    del mock_config["response"]["headers"]

                requests.post(
                    f"http://{self.host}:{self.admin_port}/__mockforge/api/mocks",
                    json=mock_config,
                    timeout=1.0,
                )
            except requests.exceptions.RequestException:
                pass  # Silently fail, stub is stored locally

    def clear_stubs(self) -> None:
        """Clear all stubs"""
        self.stubs.clear()

        if self.admin_port:
            try:
                # Get all mocks and delete them one by one
                response = requests.get(
                    f"http://{self.host}:{self.admin_port}/__mockforge/api/mocks",
                    timeout=1.0,
                )
                mocks = response.json().get("mocks", [])

                # Delete each mock
                for mock in mocks:
                    try:
                        requests.delete(
                            f"http://{self.host}:{self.admin_port}/__mockforge/api/mocks/{mock['id']}",
                            timeout=1.0,
                        )
                    except requests.exceptions.RequestException:
                        pass  # Ignore individual delete errors
            except requests.exceptions.RequestException:
                pass

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
        return verify(self.url(), pattern, expected)

    def verify_never(self, pattern: VerificationRequest) -> VerificationResult:
        """
        Verify that a request was never made

        Args:
            pattern: Pattern to match requests

        Returns:
            VerificationResult with verification outcome
        """
        return verify_never(self.url(), pattern)

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
        return verify_at_least(self.url(), pattern, min_count)

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
        return verify_sequence(self.url(), patterns)

    def count_requests(self, pattern: VerificationRequest) -> int:
        """
        Get count of matching requests

        Args:
            pattern: Pattern to match requests

        Returns:
            Count of matching requests
        """
        return count(self.url(), pattern)
