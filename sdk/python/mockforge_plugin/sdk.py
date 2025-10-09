"""
MockForge Remote Plugin SDK for Python

This SDK allows you to build MockForge plugins in Python that run as remote services.
Your plugin runs as a standalone HTTP service that MockForge can call.

Example usage:

```python
from mockforge_plugin import RemotePlugin, PluginContext, AuthCredentials, AuthResult

class MyAuthPlugin(RemotePlugin):
    async def authenticate(self, ctx: PluginContext, creds: AuthCredentials) -> AuthResult:
        # Use any Python library you want!
        import jwt
        import requests

        # Verify token with external service
        response = requests.post("https://auth.example.com/verify",
                                json={"token": creds.token})

        if response.status_code == 200:
            data = response.json()
            return AuthResult(
                authenticated=True,
                user_id=data["user_id"],
                claims=data["claims"]
            )

        return AuthResult(authenticated=False, user_id="", claims={})

if __name__ == "__main__":
    plugin = MyAuthPlugin()
    plugin.run(host="0.0.0.0", port=8080)
```
"""

from abc import ABC, abstractmethod
from dataclasses import dataclass, field, asdict
from typing import Dict, List, Optional, Any, Callable
from enum import Enum
import json
import logging
from datetime import datetime

# Try to import FastAPI, but make it optional
try:
    from fastapi import FastAPI, HTTPException, Request
    from fastapi.responses import JSONResponse
    import uvicorn
    FASTAPI_AVAILABLE = True
except ImportError:
    FASTAPI_AVAILABLE = False
    print("Warning: FastAPI not installed. Install with: pip install mockforge-plugin[fastapi]")

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("mockforge.plugin")


# ============================================================================
# Data Models
# ============================================================================

@dataclass
class PluginContext:
    """Context information about the current request"""
    method: str
    uri: str
    headers: Dict[str, str] = field(default_factory=dict)
    body: Optional[bytes] = None

    def to_dict(self) -> dict:
        return asdict(self)

    @classmethod
    def from_dict(cls, data: dict) -> 'PluginContext':
        return cls(**data)


@dataclass
class AuthCredentials:
    """Authentication credentials"""
    credential_type: str
    token: Optional[str] = None
    data: Dict[str, str] = field(default_factory=dict)

    def to_dict(self) -> dict:
        return asdict(self)

    @classmethod
    def from_dict(cls, data: dict) -> 'AuthCredentials':
        return cls(
            credential_type=data.get("type", ""),
            token=data.get("token"),
            data=data.get("data", {})
        )


@dataclass
class AuthResult:
    """Result of authentication"""
    authenticated: bool
    user_id: str
    claims: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class ResponseRequest:
    """Request information for response generation"""
    method: str
    path: str
    headers: Dict[str, str] = field(default_factory=dict)
    body: Optional[bytes] = None

    @classmethod
    def from_dict(cls, data: dict) -> 'ResponseRequest':
        return cls(**data)


@dataclass
class ResponseData:
    """Generated response data"""
    status_code: int
    headers: Dict[str, str]
    body: bytes
    content_type: str = "application/json"

    def to_dict(self) -> dict:
        result = asdict(self)
        # Convert bytes to base64 for JSON serialization
        if isinstance(result['body'], bytes):
            import base64
            result['body'] = base64.b64encode(result['body']).decode('utf-8')
        return result


@dataclass
class DataQuery:
    """Query to a data source"""
    query: str
    parameters: Dict[str, Any] = field(default_factory=dict)

    @classmethod
    def from_dict(cls, data: dict) -> 'DataQuery':
        return cls(**data)


@dataclass
class ColumnInfo:
    """Information about a data column"""
    name: str
    data_type: str

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class DataResult:
    """Result of a data query"""
    columns: List[ColumnInfo]
    rows: List[Dict[str, Any]]

    def to_dict(self) -> dict:
        return {
            "columns": [col.to_dict() for col in self.columns],
            "rows": self.rows
        }


@dataclass
class ResolutionContext:
    """Context for template resolution"""
    environment: Dict[str, str] = field(default_factory=dict)
    request_context: Optional[PluginContext] = None

    @classmethod
    def from_dict(cls, data: dict) -> 'ResolutionContext':
        ctx_data = data.get("request_context")
        request_ctx = PluginContext.from_dict(ctx_data) if ctx_data else None
        return cls(
            environment=data.get("environment", {}),
            request_context=request_ctx
        )


@dataclass
class PluginCapabilities:
    """Capabilities that a plugin requires"""
    network: 'NetworkCapabilities' = field(default_factory=lambda: NetworkCapabilities())
    filesystem: 'FilesystemCapabilities' = field(default_factory=lambda: FilesystemCapabilities())
    resources: 'ResourceLimits' = field(default_factory=lambda: ResourceLimits())

    def to_dict(self) -> dict:
        return {
            "network": self.network.to_dict(),
            "filesystem": self.filesystem.to_dict(),
            "resources": self.resources.to_dict()
        }


@dataclass
class NetworkCapabilities:
    """Network access capabilities"""
    allow_http_outbound: bool = False
    allowed_hosts: List[str] = field(default_factory=list)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class FilesystemCapabilities:
    """Filesystem access capabilities"""
    allow_read: bool = False
    allow_write: bool = False
    allowed_paths: List[str] = field(default_factory=list)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class ResourceLimits:
    """Resource limits for the plugin"""
    max_memory_bytes: int = 10 * 1024 * 1024  # 10MB default
    max_cpu_time_ms: int = 1000  # 1 second default

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class TemplateFunction:
    """Description of a template function"""
    name: str
    description: str
    parameters: List['FunctionParameter']
    return_type: str

    def to_dict(self) -> dict:
        return {
            "name": self.name,
            "description": self.description,
            "parameters": [p.to_dict() for p in self.parameters],
            "return_type": self.return_type
        }


@dataclass
class FunctionParameter:
    """Description of a function parameter"""
    name: str
    param_type: str
    required: bool
    description: str

    def to_dict(self) -> dict:
        return {
            "name": self.name,
            "type": self.param_type,
            "required": self.required,
            "description": self.description
        }


# ============================================================================
# Plugin Interfaces
# ============================================================================

class RemotePlugin(ABC):
    """
    Base class for all MockForge remote plugins.

    Subclass this and implement the methods for the plugin type(s) you want to support.
    """

    def __init__(self, name: str = "MockForge Plugin", version: str = "0.1.0"):
        self.name = name
        self.version = version
        self.app: Optional[FastAPI] = None

        if not FASTAPI_AVAILABLE:
            raise ImportError(
                "FastAPI is required for remote plugins. "
                "Install with: pip install mockforge-plugin[fastapi]"
            )

        self._setup_app()

    def _setup_app(self):
        """Setup FastAPI application with plugin endpoints"""
        self.app = FastAPI(title=self.name, version=self.version)

        # Health check endpoint
        @self.app.get("/health")
        async def health():
            return {"status": "healthy", "plugin": self.name, "version": self.version}

        # Authentication endpoint
        @self.app.post("/plugin/authenticate")
        async def authenticate_endpoint(request: Request):
            try:
                data = await request.json()
                ctx = PluginContext.from_dict(data["context"])
                creds = AuthCredentials.from_dict(data["credentials"])

                result = await self.authenticate(ctx, creds)
                return {"success": True, "result": result.to_dict()}
            except NotImplementedError:
                raise HTTPException(status_code=501, detail="Authentication not implemented")
            except Exception as e:
                logger.error(f"Authentication error: {e}", exc_info=True)
                return JSONResponse(
                    status_code=500,
                    content={"success": False, "error": str(e)}
                )

        # Template function endpoint
        @self.app.post("/plugin/template/execute")
        async def template_execute_endpoint(request: Request):
            try:
                data = await request.json()
                function_name = data["function_name"]
                args = data["args"]
                ctx = ResolutionContext.from_dict(data["context"])

                result = await self.execute_template_function(function_name, args, ctx)
                return {"success": True, "result": result}
            except NotImplementedError:
                raise HTTPException(status_code=501, detail="Template functions not implemented")
            except Exception as e:
                logger.error(f"Template execution error: {e}", exc_info=True)
                return JSONResponse(
                    status_code=500,
                    content={"success": False, "error": str(e)}
                )

        # Response generation endpoint
        @self.app.post("/plugin/response/generate")
        async def response_generate_endpoint(request: Request):
            try:
                data = await request.json()
                ctx = PluginContext.from_dict(data["context"])
                req = ResponseRequest.from_dict(data["request"])

                result = await self.generate_response(ctx, req)
                return {"success": True, "result": result.to_dict()}
            except NotImplementedError:
                raise HTTPException(status_code=501, detail="Response generation not implemented")
            except Exception as e:
                logger.error(f"Response generation error: {e}", exc_info=True)
                return JSONResponse(
                    status_code=500,
                    content={"success": False, "error": str(e)}
                )

        # Data source query endpoint
        @self.app.post("/plugin/datasource/query")
        async def datasource_query_endpoint(request: Request):
            try:
                data = await request.json()
                query = DataQuery.from_dict(data["query"])
                ctx = PluginContext.from_dict(data["context"])

                result = await self.query_datasource(query, ctx)
                return {"success": True, "result": result.to_dict()}
            except NotImplementedError:
                raise HTTPException(status_code=501, detail="Data source not implemented")
            except Exception as e:
                logger.error(f"Data source query error: {e}", exc_info=True)
                return JSONResponse(
                    status_code=500,
                    content={"success": False, "error": str(e)}
                )

    # ========================================================================
    # Plugin Methods (override these in your plugin)
    # ========================================================================

    async def authenticate(
        self, ctx: PluginContext, creds: AuthCredentials
    ) -> AuthResult:
        """
        Implement authentication logic.

        Args:
            ctx: Request context
            creds: Authentication credentials

        Returns:
            AuthResult with authentication status and user info
        """
        raise NotImplementedError("authenticate() must be implemented")

    async def execute_template_function(
        self, function_name: str, args: List[Any], ctx: ResolutionContext
    ) -> Any:
        """
        Execute a template function.

        Args:
            function_name: Name of the function to execute
            args: Function arguments
            ctx: Resolution context

        Returns:
            Function result
        """
        raise NotImplementedError("execute_template_function() must be implemented")

    async def generate_response(
        self, ctx: PluginContext, req: ResponseRequest
    ) -> ResponseData:
        """
        Generate a response for a request.

        Args:
            ctx: Plugin context
            req: Request information

        Returns:
            Generated response data
        """
        raise NotImplementedError("generate_response() must be implemented")

    async def query_datasource(
        self, query: DataQuery, ctx: PluginContext
    ) -> DataResult:
        """
        Query a data source.

        Args:
            query: Query to execute
            ctx: Plugin context

        Returns:
            Query results
        """
        raise NotImplementedError("query_datasource() must be implemented")

    def get_capabilities(self) -> PluginCapabilities:
        """
        Return the capabilities this plugin requires.

        Override this to specify network, filesystem, and resource requirements.
        """
        return PluginCapabilities()

    # ========================================================================
    # Runtime Methods
    # ========================================================================

    def run(self, host: str = "0.0.0.0", port: int = 8080, **kwargs):
        """
        Run the plugin server.

        Args:
            host: Host to bind to (default: 0.0.0.0)
            port: Port to listen on (default: 8080)
            **kwargs: Additional arguments passed to uvicorn
        """
        logger.info(f"Starting {self.name} v{self.version} on {host}:{port}")
        uvicorn.run(self.app, host=host, port=port, **kwargs)


# ============================================================================
# Utility Functions
# ============================================================================

def create_success_response(data: Any) -> Dict[str, Any]:
    """Create a successful response"""
    return {"success": True, "result": data}


def create_error_response(error: str, code: int = 500) -> Dict[str, Any]:
    """Create an error response"""
    return {"success": False, "error": error, "code": code}


# ============================================================================
# Example Plugin (for documentation/testing)
# ============================================================================

class ExampleAuthPlugin(RemotePlugin):
    """Example authentication plugin"""

    async def authenticate(
        self, ctx: PluginContext, creds: AuthCredentials
    ) -> AuthResult:
        """Simple token-based authentication"""
        # In a real plugin, you'd validate against a database or external service
        if creds.token == "valid-token-123":
            return AuthResult(
                authenticated=True,
                user_id="user123",
                claims={"role": "admin", "permissions": ["read", "write"]}
            )
        else:
            return AuthResult(
                authenticated=False,
                user_id="",
                claims={}
            )

    def get_capabilities(self) -> PluginCapabilities:
        """This plugin doesn't need any special capabilities"""
        return PluginCapabilities(
            network=NetworkCapabilities(allow_http_outbound=False),
            filesystem=FilesystemCapabilities(allow_read=False),
            resources=ResourceLimits(
                max_memory_bytes=10 * 1024 * 1024,  # 10MB
                max_cpu_time_ms=1000  # 1 second
            )
        )


if __name__ == "__main__":
    # Example usage
    plugin = ExampleAuthPlugin(name="Example Auth Plugin", version="1.0.0")
    plugin.run()
