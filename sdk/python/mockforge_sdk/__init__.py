"""
MockForge SDK for Python

Embed MockForge mock servers directly in your tests.

Example:
    >>> from mockforge_sdk import MockServer
    >>>
    >>> # Start a mock server
    >>> with MockServer(port=3000) as server:
    ...     server.stub_response('GET', '/api/users/123', {
    ...         'id': 123,
    ...         'name': 'John Doe'
    ...     })
    ...
    ...     # Make requests to the mock server
    ...     import requests
    ...     response = requests.get('http://localhost:3000/api/users/123')
    ...     assert response.status_code == 200
"""

from .mock_server import MockServer
from .stub_builder import StubBuilder
from .types import ResponseStub, MockServerConfig, VerificationRequest, VerificationResult
from .verification import VerificationCount

__version__ = "0.1.0"
__all__ = [
    "MockServer",
    "StubBuilder",
    "ResponseStub",
    "MockServerConfig",
    "VerificationRequest",
    "VerificationResult",
    "VerificationCount",
]
