"""Fluent builder for creating response stubs"""

from typing import Dict, Any, Optional
from .types import ResponseStub


class StubBuilder:
    """Fluent builder for creating response stubs"""

    def __init__(self, method: str, path: str):
        """
        Initialize a StubBuilder

        Args:
            method: HTTP method
            path: Request path
        """
        self._method = method.upper()
        self._path = path
        self._status = 200
        self._headers: Dict[str, str] = {}
        self._body: Optional[Any] = None
        self._latency_ms: Optional[int] = None

    def status(self, code: int) -> "StubBuilder":
        """Set the response status code"""
        self._status = code
        return self

    def header(self, key: str, value: str) -> "StubBuilder":
        """Set a response header"""
        self._headers[key] = value
        return self

    def headers(self, headers: Dict[str, str]) -> "StubBuilder":
        """Set multiple response headers"""
        self._headers.update(headers)
        return self

    def body(self, body: Any) -> "StubBuilder":
        """Set the response body"""
        self._body = body
        return self

    def latency(self, ms: int) -> "StubBuilder":
        """Set response latency in milliseconds"""
        self._latency_ms = ms
        return self

    def build(self) -> ResponseStub:
        """Build the response stub"""
        if self._body is None:
            raise ValueError("Response body is required")

        return ResponseStub(
            method=self._method,
            path=self._path,
            body=self._body,
            status=self._status,
            headers=self._headers,
            latency_ms=self._latency_ms,
        )
