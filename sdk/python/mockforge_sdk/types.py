"""Type definitions for MockForge SDK"""

from typing import Dict, Any, Optional
from dataclasses import dataclass, field


@dataclass
class MockServerConfig:
    """Configuration for MockServer"""

    port: int = 0
    """Port to listen on (default: random available port)"""

    host: str = "127.0.0.1"
    """Host to bind to (default: 127.0.0.1)"""

    config_file: Optional[str] = None
    """Path to MockForge configuration file"""

    openapi_spec: Optional[str] = None
    """Path to OpenAPI specification"""


@dataclass
class ResponseStub:
    """Response stub configuration"""

    method: str
    """HTTP method (GET, POST, PUT, DELETE, etc.)"""

    path: str
    """Path pattern (supports {{path_params}})"""

    body: Any
    """Response body"""

    status: int = 200
    """HTTP status code (default: 200)"""

    headers: Dict[str, str] = field(default_factory=dict)
    """Response headers"""

    latency_ms: Optional[int] = None
    """Latency in milliseconds"""


@dataclass
class VerificationRequest:
    """Pattern for matching requests during verification"""

    method: Optional[str] = None
    """HTTP method to match (e.g., 'GET', 'POST'). Case-insensitive. If None, matches any method."""

    path: Optional[str] = None
    """URL path to match. Supports exact match, wildcards (*, **), and regex. If None, matches any path."""

    query_params: Dict[str, str] = field(default_factory=dict)
    """Query parameters to match (all must be present and match). If empty, query parameters are not checked."""

    headers: Dict[str, str] = field(default_factory=dict)
    """Headers to match (all must be present and match). Case-insensitive header names. If empty, headers are not checked."""

    body_pattern: Optional[str] = None
    """Request body pattern to match. Supports exact match or regex. If None, body is not checked."""


@dataclass
class VerificationResult:
    """Result of a verification operation"""

    matched: bool
    """Whether the verification passed"""

    count: int
    """Actual count of matching requests"""

    expected: Dict[str, Any]
    """Expected count assertion"""

    matches: List[Dict[str, Any]] = field(default_factory=list)
    """All matching request log entries (for inspection)"""

    error_message: Optional[str] = None
    """Error message if verification failed"""
