"""Standardized error types for MockForge SDK"""

from typing import Optional, Dict, Any


class MockServerErrorCode:
    """Error codes for MockServer operations"""
    CLI_NOT_FOUND = "CLI_NOT_FOUND"
    SERVER_START_FAILED = "SERVER_START_FAILED"
    PORT_DETECTION_FAILED = "PORT_DETECTION_FAILED"
    ADMIN_API_ERROR = "ADMIN_API_ERROR"
    HEALTH_CHECK_TIMEOUT = "HEALTH_CHECK_TIMEOUT"
    INVALID_CONFIG = "INVALID_CONFIG"
    STUB_NOT_FOUND = "STUB_NOT_FOUND"
    NETWORK_ERROR = "NETWORK_ERROR"
    UNKNOWN_ERROR = "UNKNOWN_ERROR"


class MockServerError(Exception):
    """Standardized error class for MockServer operations"""

    def __init__(
        self,
        code: str,
        message: str,
        cause: Optional[Exception] = None,
        details: Optional[Dict[str, Any]] = None,
    ):
        """
        Initialize a MockServerError

        Args:
            code: Error code from MockServerErrorCode
            message: Human-readable error message
            cause: Underlying exception that caused this error
            details: Additional error details
        """
        super().__init__(message)
        self.code = code
        self.cause = cause
        self.details = details or {}

    @classmethod
    def cli_not_found(cls, cause: Optional[Exception] = None) -> "MockServerError":
        """Create an error for CLI not found"""
        return cls(
            MockServerErrorCode.CLI_NOT_FOUND,
            "MockForge CLI not found. Install with: cargo install mockforge-cli",
            cause,
            {"hint": "Ensure mockforge is in your PATH"},
        )

    @classmethod
    def server_start_failed(
        cls, message: str, cause: Optional[Exception] = None
    ) -> "MockServerError":
        """Create an error for server start failure"""
        return cls(
            MockServerErrorCode.SERVER_START_FAILED,
            f"Failed to start MockForge server: {message}",
            cause,
        )

    @classmethod
    def port_detection_failed(
        cls, cause: Optional[Exception] = None
    ) -> "MockServerError":
        """Create an error for port detection failure"""
        return cls(
            MockServerErrorCode.PORT_DETECTION_FAILED,
            "Failed to detect server port from MockForge output. "
            "The server may have failed to start.",
            cause,
            {"hint": "Check that mockforge CLI is installed and the server started successfully"},
        )

    @classmethod
    def admin_api_error(
        cls, operation: str, message: str, cause: Optional[Exception] = None
    ) -> "MockServerError":
        """Create an error for Admin API operations"""
        return cls(
            MockServerErrorCode.ADMIN_API_ERROR,
            f"Admin API {operation} failed: {message}",
            cause,
            {"operation": operation},
        )

    @classmethod
    def health_check_timeout(cls, timeout: int, port: int) -> "MockServerError":
        """Create an error for health check timeout"""
        return cls(
            MockServerErrorCode.HEALTH_CHECK_TIMEOUT,
            f"Health check timed out after {timeout}s. "
            f"Could not connect to http://127.0.0.1:{port}/health",
            None,
            {"timeout": timeout, "port": port, "hint": "Check that the server started successfully"},
        )

    @classmethod
    def invalid_config(
        cls, message: str, details: Optional[Dict[str, Any]] = None
    ) -> "MockServerError":
        """Create an error for invalid configuration"""
        return cls(
            MockServerErrorCode.INVALID_CONFIG,
            f"Invalid configuration: {message}",
            None,
            details,
        )

    @classmethod
    def stub_not_found(cls, method: str, path: str) -> "MockServerError":
        """Create an error for stub not found"""
        return cls(
            MockServerErrorCode.STUB_NOT_FOUND,
            f"Stub not found: {method} {path}",
            None,
            {"method": method, "path": path},
        )

    @classmethod
    def network_error(
        cls, message: str, cause: Optional[Exception] = None
    ) -> "MockServerError":
        """Create an error for network operations"""
        return cls(
            MockServerErrorCode.NETWORK_ERROR,
            f"Network error: {message}",
            cause,
        )
