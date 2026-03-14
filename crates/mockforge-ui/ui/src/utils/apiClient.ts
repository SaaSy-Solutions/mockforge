// API client with automatic JWT token injection
// Intercepts all fetch requests to add Authorization header

import { useAuthStore } from '../stores/useAuthStore';

// Store the original fetch function
const originalFetch = globalThis.fetch;

// Detect cloud mode (same logic as authApi.ts)
const isCloud = (() => {
  const apiBase = import.meta.env.VITE_API_BASE_URL;
  return !!apiBase && apiBase !== '';
})();

// In cloud mode, /__mockforge/ endpoints don't exist on the registry server.
// Return synthetic empty responses to avoid 404 spam.
function createCloudStubResponse(url: string): Response {
  const path = new URL(url, window.location.origin).pathname;

  // Map common local endpoints to sensible empty responses
  if (path === '/__mockforge/dashboard') {
    return new Response(JSON.stringify({
      success: true,
      data: {
        server_info: { version: 'cloud', build_time: '', git_sha: '', api_enabled: true, admin_port: 0 },
        system_info: { os: 'cloud', arch: 'cloud', uptime: 0, memory_usage: 0 },
        metrics: { total_requests: 0, active_requests: 0, average_response_time: 0, error_rate: 0 },
        servers: [],
        recent_logs: [],
        system: { version: 'cloud', uptime_seconds: 0, memory_usage_mb: 0, cpu_usage_percent: 0, active_threads: 0, total_routes: 0, total_fixtures: 0 },
      },
    }), { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path === '/__mockforge/logs' || path.startsWith('/__mockforge/logs?')) {
    return new Response(JSON.stringify({ success: true, data: [] }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path === '/__mockforge/workspaces') {
    return new Response(JSON.stringify({ success: true, data: [] }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path === '/__mockforge/routes') {
    return new Response(JSON.stringify({ success: true, data: [] }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path === '/__mockforge/metrics') {
    return new Response(JSON.stringify({ success: true, data: {} }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path === '/__mockforge/health') {
    return new Response(JSON.stringify({ success: true, data: { status: 'cloud', services: {}, last_check: new Date().toISOString(), issues: [] } }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path === '/__mockforge/server-info') {
    return new Response(JSON.stringify({ success: true, data: { version: 'cloud', build_time: '', git_sha: '' } }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path === '/__mockforge/reality/level') {
    return new Response(JSON.stringify({ success: true, data: {
      level: 1, level_name: 'Cloud', description: 'Cloud-hosted mock',
      chaos: { enabled: false, error_rate: 0, delay_rate: 0 },
      latency: { base_ms: 0, jitter_ms: 0 },
      mockai: { enabled: false },
    }}), { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path === '/__mockforge/fixtures') {
    return new Response(JSON.stringify({ success: true, data: [] }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path === '/__mockforge/config') {
    return new Response(JSON.stringify({ success: true, data: {
      latency: { enabled: false, base_ms: 0, jitter_ms: 0, tag_overrides: {} },
      faults: { enabled: false, failure_rate: 0, status_codes: [] },
      proxy: { enabled: false, upstream_url: null, timeout_seconds: 30 },
      validation: { mode: 'disabled', aggregate_errors: false, validate_responses: false, overrides: {} },
    }}), { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path === '/__mockforge/scenarios') {
    return new Response(JSON.stringify({ success: true, data: { scenarios: [], total: 0 } }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path === '/__mockforge/chains') {
    return new Response(JSON.stringify({ success: true, data: { chains: [], total: 0 } }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path === '/__mockforge/graph') {
    return new Response(JSON.stringify({ success: true, data: { nodes: [], edges: [], clusters: [] } }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path === '/__mockforge/plugins') {
    return new Response(JSON.stringify({ success: true, data: { plugins: [], total: 0 } }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path === '/__mockforge/validation') {
    return new Response(JSON.stringify({ success: true, data: { mode: 'disabled', aggregate_errors: false, validate_responses: false, overrides: {} } }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path.startsWith('/__mockforge/import/history')) {
    return new Response(JSON.stringify({ success: true, data: { imports: [], total: 0 } }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  // Generic fallback: return empty data object (not null, to avoid crashes from .property access)
  return new Response(JSON.stringify({ success: true, data: {} }),
    { status: 200, headers: { 'Content-Type': 'application/json' } });
}

// Create a fetch wrapper that adds JWT tokens
export function createAuthenticatedFetch() {
  return async (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
    const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;

    // In cloud mode, intercept /__mockforge/ calls with stub responses
    if (isCloud && url.includes('/__mockforge/')) {
      return createCloudStubResponse(url);
    }

    const state = useAuthStore.getState();
    const token = state.token;

    // Clone the init object to avoid mutating the original
    const headers = new Headers(init?.headers);

    // Add Authorization header if token exists
    if (token) {
      headers.set('Authorization', `Bearer ${token}`);
    }

    // Create new init with updated headers
    const newInit: RequestInit = {
      ...init,
      headers,
    };

    // Make the request
    let response = await originalFetch(input, newInit);

    // Handle 401 Unauthorized - token might be expired
    if (response.status === 401 && token) {
      try {
        // Try to refresh the token
        await state.refreshTokenAction();

        // Retry the request with new token
        const newToken = useAuthStore.getState().token;
        if (newToken) {
          headers.set('Authorization', `Bearer ${newToken}`);
          response = await originalFetch(input, { ...newInit, headers });
        } else {
          // Refresh failed, logout
          state.logout();
        }
      } catch (error) {
        // Refresh failed, logout
        state.logout();
      }
    }

    return response;
  };
}

// Export a fetch function that can be used throughout the app
export const authenticatedFetch = createAuthenticatedFetch();
