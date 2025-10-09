#!/usr/bin/env python3
"""
OAuth2 Authentication Plugin for MockForge (Remote Plugin)

This plugin demonstrates how to build a remote authentication plugin in Python
that validates OAuth2 tokens by calling an external authorization server.

Features:
- OAuth2 token validation
- Token introspection
- User info retrieval
- Async/await support
- Full Python ecosystem access

Run:
    python plugin.py

or with uvicorn directly:
    uvicorn plugin:app --host 0.0.0.0 --port 8080

Install in MockForge:
    Add to config.yaml:
      plugins:
        - id: auth-python-oauth
          runtime: remote
          endpoint: http://localhost:8080
"""

import os
import sys
from typing import Optional
import logging

# Try to import required packages
try:
    from mockforge_plugin import RemotePlugin, PluginContext, AuthCredentials, AuthResult
    import requests
    from requests.exceptions import RequestException
except ImportError as e:
    print(f"Error: Missing required packages: {e}")
    print("\nInstall with:")
    print("  pip install mockforge-plugin[fastapi] requests")
    sys.exit(1)

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


class OAuth2AuthPlugin(RemotePlugin):
    """
    OAuth2 authentication plugin that validates tokens with an external auth server.

    This plugin demonstrates:
    1. Calling external HTTP services (auth server)
    2. Error handling and retry logic
    3. Token caching (optional)
    4. Async/await patterns
    5. Python-specific features (requests library, etc.)
    """

    def __init__(self):
        super().__init__(
            name="OAuth2 Authentication Plugin",
            version="0.1.0"
        )

        # Load configuration from environment variables
        self.auth_server_url = os.getenv(
            "AUTH_SERVER_URL",
            "https://auth.example.com"
        )
        self.client_id = os.getenv("OAUTH_CLIENT_ID", "mockforge")
        self.client_secret = os.getenv("OAUTH_CLIENT_SECRET", "")
        self.introspection_endpoint = f"{self.auth_server_url}/oauth/introspect"
        self.userinfo_endpoint = f"{self.auth_server_url}/oauth/userinfo"

        # Token cache (simple in-memory cache)
        self._token_cache = {}

        logger.info(f"Initialized OAuth2 plugin with auth server: {self.auth_server_url}")

    async def authenticate(
        self,
        ctx: PluginContext,
        creds: AuthCredentials
    ) -> AuthResult:
        """
        Authenticate user by validating OAuth2 token.

        This method:
        1. Extracts the Bearer token
        2. Validates it with the auth server (token introspection)
        3. Retrieves user information
        4. Returns authentication result

        Args:
            ctx: Request context
            creds: Authentication credentials (Bearer token expected)

        Returns:
            AuthResult with authentication status and user info
        """
        # Validate credential type
        if creds.credential_type.lower() != "bearer":
            logger.warning(f"Unsupported credential type: {creds.credential_type}")
            return AuthResult(
                authenticated=False,
                user_id="",
                claims={"error": "Unsupported credential type"}
            )

        # Extract token
        token = creds.token
        if not token:
            logger.warning("Missing token in credentials")
            return AuthResult(
                authenticated=False,
                user_id="",
                claims={"error": "Missing token"}
            )

        # Check cache first (optional optimization)
        if token in self._token_cache:
            cached_result = self._token_cache[token]
            logger.info(f"Using cached token for user: {cached_result.user_id}")
            return cached_result

        try:
            # Step 1: Validate token with introspection endpoint
            introspection_result = await self._introspect_token(token)

            if not introspection_result.get("active", False):
                logger.info("Token is not active")
                return AuthResult(
                    authenticated=False,
                    user_id="",
                    claims={"error": "Token is not active"}
                )

            # Step 2: Get user information
            user_info = await self._get_user_info(token)

            # Step 3: Build authentication result
            user_id = user_info.get("sub") or introspection_result.get("sub", "unknown")

            claims = {
                "scope": introspection_result.get("scope", ""),
                "client_id": introspection_result.get("client_id", ""),
                "exp": introspection_result.get("exp"),
                "iat": introspection_result.get("iat"),
                **user_info  # Include all user info claims
            }

            result = AuthResult(
                authenticated=True,
                user_id=user_id,
                claims=claims
            )

            # Cache the result
            self._token_cache[token] = result

            logger.info(f"Successfully authenticated user: {user_id}")
            return result

        except RequestException as e:
            logger.error(f"Failed to validate token: {e}", exc_info=True)
            return AuthResult(
                authenticated=False,
                user_id="",
                claims={"error": f"Auth server error: {str(e)}"}
            )
        except Exception as e:
            logger.error(f"Unexpected error during authentication: {e}", exc_info=True)
            return AuthResult(
                authenticated=False,
                user_id="",
                claims={"error": f"Internal error: {str(e)}"}
            )

    async def _introspect_token(self, token: str) -> dict:
        """
        Validate token using OAuth2 token introspection endpoint.

        Args:
            token: Access token to validate

        Returns:
            Introspection result containing token status and metadata

        Raises:
            RequestException: If the introspection request fails
        """
        logger.debug(f"Introspecting token at {self.introspection_endpoint}")

        response = requests.post(
            self.introspection_endpoint,
            data={
                "token": token,
                "token_type_hint": "access_token"
            },
            auth=(self.client_id, self.client_secret),
            timeout=5.0
        )

        response.raise_for_status()
        return response.json()

    async def _get_user_info(self, token: str) -> dict:
        """
        Retrieve user information using the access token.

        Args:
            token: Access token

        Returns:
            User information dictionary

        Raises:
            RequestException: If the user info request fails
        """
        logger.debug(f"Fetching user info from {self.userinfo_endpoint}")

        response = requests.get(
            self.userinfo_endpoint,
            headers={"Authorization": f"Bearer {token}"},
            timeout=5.0
        )

        response.raise_for_status()
        return response.json()

    def get_capabilities(self):
        """
        Return the capabilities this plugin requires.

        Remote plugins need network access to call the auth server.
        """
        from mockforge_plugin import (
            PluginCapabilities,
            NetworkCapabilities,
            FilesystemCapabilities,
            ResourceLimits
        )

        return PluginCapabilities(
            network=NetworkCapabilities(
                allow_http_outbound=True,
                allowed_hosts=[self.auth_server_url]
            ),
            filesystem=FilesystemCapabilities(
                allow_read=False,
                allow_write=False,
                allowed_paths=[]
            ),
            resources=ResourceLimits(
                max_memory_bytes=50 * 1024 * 1024,  # 50MB
                max_cpu_time_ms=5000  # 5 seconds
            )
        )


def create_plugin() -> OAuth2AuthPlugin:
    """Factory function to create the plugin instance."""
    return OAuth2AuthPlugin()


# Create the plugin instance
app = create_plugin().app


if __name__ == "__main__":
    """
    Run the plugin server.

    Configuration via environment variables:
    - AUTH_SERVER_URL: OAuth2 authorization server URL
    - OAUTH_CLIENT_ID: OAuth2 client ID
    - OAUTH_CLIENT_SECRET: OAuth2 client secret
    - HOST: Host to bind to (default: 0.0.0.0)
    - PORT: Port to listen on (default: 8080)
    """
    plugin = create_plugin()

    host = os.getenv("HOST", "0.0.0.0")
    port = int(os.getenv("PORT", "8080"))

    logger.info("=" * 60)
    logger.info("OAuth2 Authentication Plugin for MockForge")
    logger.info("=" * 60)
    logger.info(f"Auth Server: {plugin.auth_server_url}")
    logger.info(f"Client ID: {plugin.client_id}")
    logger.info(f"Listening on: {host}:{port}")
    logger.info("=" * 60)

    plugin.run(host=host, port=port)
