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
// The main API services (dashboard.ts, workspaces.ts, fixtures.ts, etc.) now use
// /api/v1/ endpoints directly, but some components may still reference /__mockforge/
// paths. Return a generic empty response to prevent 404 errors.
function createCloudStubResponse(_url: string): Response {
  return new Response(JSON.stringify({ success: true, data: {} }),
    { status: 200, headers: { 'Content-Type': 'application/json' } });
}

// Local-only API paths that don't exist on the registry server.
// In cloud mode, return stubs instead of letting them 404.
const LOCAL_ONLY_API_PREFIXES = [
  '/api/chaos/',
  '/api/observability/',
  '/api/resilience/',
  '/api/recorder/',
  '/api/conformance/',
  '/api/performance/',
  '/api/world-state/',
  '/api/v1/consistency/',
  '/api/v1/drift/',
  '/api/v1/scenario-studio/',
  '/api/v1/snapshots',
  '/api/v2/analytics/',
];

// Cached server capabilities (fetched once from GET /api/capabilities)
let cachedCapabilities: string[] | null = null;

/** Fetch available features from the server's capabilities endpoint. */
export async function fetchCapabilities(baseUrl?: string): Promise<string[]> {
  if (cachedCapabilities) return cachedCapabilities;
  try {
    const url = baseUrl
      ? `${baseUrl}/api/capabilities`
      : '/api/capabilities';
    const res = await originalFetch(url);
    if (res.ok) {
      const data = await res.json();
      cachedCapabilities = data.features ?? [];
      return cachedCapabilities;
    }
  } catch {
    // Server may not support capabilities yet — fall back to static list
  }
  return [];
}

/** Check whether a feature is reported as available by the server. */
export function hasCapability(feature: string): boolean {
  return cachedCapabilities?.includes(feature) ?? false;
}

function isLocalOnlyApi(url: string): boolean {
  const path = new URL(url, window.location.origin).pathname;
  return LOCAL_ONLY_API_PREFIXES.some(prefix => path.startsWith(prefix));
}

function createLocalApiStubResponse(_url: string): Response {
  return new Response(JSON.stringify({}),
    { status: 200, headers: { 'Content-Type': 'application/json' } });
}

// Create a fetch wrapper that adds JWT tokens
export function createAuthenticatedFetch() {
  return async (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
    const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;

    // In cloud mode, intercept local-only endpoints with stub responses
    if (isCloud && url.includes('/__mockforge/')) {
      return createCloudStubResponse(url);
    }
    if (isCloud && isLocalOnlyApi(url)) {
      return createLocalApiStubResponse(url);
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

// In cloud mode, override global fetch to intercept /__mockforge/ calls everywhere.
// This catches components that use raw fetch() instead of authenticatedFetch.
if (isCloud) {
  globalThis.fetch = async (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
    const url = typeof input === 'string' ? input : input instanceof URL ? input.href : (input as Request).url;
    if (url.includes('/__mockforge/')) {
      return createCloudStubResponse(url);
    }
    if (isLocalOnlyApi(url)) {
      return createLocalApiStubResponse(url);
    }
    return originalFetch(input, init);
  };
}
