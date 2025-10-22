"""MockServer implementation"""

import subprocess
import time
import requests
from typing import Optional, Dict, Any, List
from .types import MockServerConfig, ResponseStub


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

        # Enable admin API for dynamic stub management
        args.extend(["--admin", "--admin-port", "0"])

        self.process = subprocess.Popen(
            args,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

        # Wait for server to start
        self._wait_for_server()

        return self

    def _wait_for_server(self, timeout: int = 10) -> None:
        """Wait for the server to be ready"""
        start_time = time.time()

        while time.time() - start_time < timeout:
            try:
                response = requests.get(
                    f"http://{self.host}:{self.port}/health",
                    timeout=0.1
                )
                if response.status_code == 200:
                    return
            except requests.exceptions.RequestException:
                time.sleep(0.1)

        raise RuntimeError("Failed to start MockForge server")

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
                requests.post(
                    f"http://{self.host}:{self.admin_port}/api/stubs",
                    json=stub.__dict__,
                    timeout=1.0,
                )
            except requests.exceptions.RequestException:
                pass  # Silently fail, stub is stored locally

    def clear_stubs(self) -> None:
        """Clear all stubs"""
        self.stubs.clear()

        if self.admin_port:
            try:
                requests.delete(
                    f"http://{self.host}:{self.admin_port}/api/stubs",
                    timeout=1.0,
                )
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
