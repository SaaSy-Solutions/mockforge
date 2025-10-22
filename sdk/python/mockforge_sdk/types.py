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
