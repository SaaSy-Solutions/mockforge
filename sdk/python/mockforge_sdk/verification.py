"""Verification API for MockForge SDK"""

from typing import List, Dict, Any, Optional
from .types import VerificationRequest, VerificationResult
import requests


class VerificationCount:
    """Count assertion for verification"""

    @staticmethod
    def exactly(n: int) -> Dict[str, Any]:
        """Request must be made exactly N times"""
        return {"type": "exactly", "value": n}

    @staticmethod
    def at_least(n: int) -> Dict[str, Any]:
        """Request must be made at least N times"""
        return {"type": "at_least", "value": n}

    @staticmethod
    def at_most(n: int) -> Dict[str, Any]:
        """Request must be made at most N times"""
        return {"type": "at_most", "value": n}

    @staticmethod
    def never() -> Dict[str, Any]:
        """Request must never be made (count must be 0)"""
        return {"type": "never"}

    @staticmethod
    def at_least_once() -> Dict[str, Any]:
        """Request must be made at least once (count >= 1)"""
        return {"type": "at_least_once"}


def _make_verification_request(
    base_url: str,
    endpoint: str,
    data: Dict[str, Any],
    timeout: float = 5.0,
) -> VerificationResult:
    """Make a verification API request"""
    try:
        response = requests.post(
            f"{base_url}/api/verification/{endpoint}",
            json=data,
            timeout=timeout,
        )
        response.raise_for_status()
        result_data = response.json()
        return VerificationResult(**result_data)
    except requests.exceptions.RequestException as e:
        return VerificationResult(
            matched=False,
            count=0,
            expected={},
            matches=[],
            error_message=f"Verification API request failed: {str(e)}",
        )


def verify(
    base_url: str,
    pattern: VerificationRequest,
    expected: Dict[str, Any],
    timeout: float = 5.0,
) -> VerificationResult:
    """
    Verify requests against a pattern and count assertion

    Args:
        base_url: Base URL of the mock server (e.g., "http://localhost:3000")
        pattern: Pattern to match requests
        expected: Expected count assertion (from VerificationCount)
        timeout: Request timeout in seconds

    Returns:
        VerificationResult with verification outcome
    """
    data = {
        "pattern": {
            "method": pattern.method,
            "path": pattern.path,
            "query_params": pattern.query_params,
            "headers": pattern.headers,
            "body_pattern": pattern.body_pattern,
        },
        "expected": expected,
    }
    return _make_verification_request(base_url, "verify", data, timeout)


def verify_never(
    base_url: str,
    pattern: VerificationRequest,
    timeout: float = 5.0,
) -> VerificationResult:
    """
    Verify that a request was never made

    Args:
        base_url: Base URL of the mock server
        pattern: Pattern to match requests
        timeout: Request timeout in seconds

    Returns:
        VerificationResult with verification outcome
    """
    data = {
        "method": pattern.method,
        "path": pattern.path,
        "query_params": pattern.query_params,
        "headers": pattern.headers,
        "body_pattern": pattern.body_pattern,
    }
    return _make_verification_request(base_url, "never", data, timeout)


def verify_at_least(
    base_url: str,
    pattern: VerificationRequest,
    min_count: int,
    timeout: float = 5.0,
) -> VerificationResult:
    """
    Verify that a request was made at least N times

    Args:
        base_url: Base URL of the mock server
        pattern: Pattern to match requests
        min_count: Minimum count
        timeout: Request timeout in seconds

    Returns:
        VerificationResult with verification outcome
    """
    data = {
        "pattern": {
            "method": pattern.method,
            "path": pattern.path,
            "query_params": pattern.query_params,
            "headers": pattern.headers,
            "body_pattern": pattern.body_pattern,
        },
        "min": min_count,
    }
    return _make_verification_request(base_url, "at-least", data, timeout)


def verify_sequence(
    base_url: str,
    patterns: List[VerificationRequest],
    timeout: float = 5.0,
) -> VerificationResult:
    """
    Verify that requests occurred in a specific sequence

    Args:
        base_url: Base URL of the mock server
        patterns: List of patterns to match in sequence
        timeout: Request timeout in seconds

    Returns:
        VerificationResult with verification outcome
    """
    data = {
        "patterns": [
            {
                "method": p.method,
                "path": p.path,
                "query_params": p.query_params,
                "headers": p.headers,
                "body_pattern": p.body_pattern,
            }
            for p in patterns
        ],
    }
    return _make_verification_request(base_url, "sequence", data, timeout)


def count(
    base_url: str,
    pattern: VerificationRequest,
    timeout: float = 5.0,
) -> int:
    """
    Get count of matching requests

    Args:
        base_url: Base URL of the mock server
        pattern: Pattern to match requests
        timeout: Request timeout in seconds

    Returns:
        Count of matching requests
    """
    try:
        data = {
            "pattern": {
                "method": pattern.method,
                "path": pattern.path,
                "query_params": pattern.query_params,
                "headers": pattern.headers,
                "body_pattern": pattern.body_pattern,
            },
        }
        response = requests.post(
            f"{base_url}/api/verification/count",
            json=data,
            timeout=timeout,
        )
        response.raise_for_status()
        result = response.json()
        return result.get("count", 0)
    except requests.exceptions.RequestException:
        return 0
