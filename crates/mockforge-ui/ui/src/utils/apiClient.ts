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

  if (path.startsWith('/__mockforge/community/')) {
    return new Response(JSON.stringify({ success: true, data: [] }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path.startsWith('/__mockforge/environments')) {
    return new Response(JSON.stringify({ success: true, data: [] }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path.startsWith('/__mockforge/data-explorer')) {
    return new Response(JSON.stringify({ success: true, data: { tables: [], queries: [] } }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path.startsWith('/__mockforge/testing')) {
    return new Response(JSON.stringify({ success: true, data: { suites: [], results: [] } }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  // State machines
  if (path.startsWith('/__mockforge/api/state-machines/instances')) {
    return new Response(JSON.stringify({ success: true, data: { instances: [], total: 0 } }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }
  if (path.startsWith('/__mockforge/api/state-machines/export')) {
    return new Response(JSON.stringify({ success: true, data: { state_machines: [], visual_layouts: {} } }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }
  if (path.startsWith('/__mockforge/api/state-machines')) {
    return new Response(JSON.stringify({ success: true, data: { state_machines: [], total: 0 } }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  // Proxy inspector
  if (path.startsWith('/__mockforge/api/proxy/rules')) {
    return new Response(JSON.stringify({ success: true, data: { rules: [], total: 0 } }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }
  if (path.startsWith('/__mockforge/api/proxy/inspect')) {
    return new Response(JSON.stringify({ success: true, data: { message: 'Not available in cloud mode', requests: [], responses: [] } }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  // MockAI
  if (path.startsWith('/__mockforge/api/mockai/rules')) {
    return new Response(JSON.stringify({ success: true, data: { rules: [], explanations: [], total: 0 } }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }
  if (path.startsWith('/__mockforge/api/mockai')) {
    return new Response(JSON.stringify({ success: true, data: {} }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  // Playground
  if (path.startsWith('/__mockforge/playground')) {
    return new Response(JSON.stringify({ success: true, data: { endpoints: [], schemas: [] } }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  // Time travel
  if (path.startsWith('/__mockforge/time-travel')) {
    return new Response(JSON.stringify({ success: true, data: { enabled: false, current_time: new Date().toISOString(), scale: 1.0, mutations: [], cron_jobs: [] } }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  // Reality presets
  if (path.startsWith('/__mockforge/reality/presets')) {
    return new Response(JSON.stringify({ success: true, data: [] }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  // Verification
  if (path.startsWith('/__mockforge/verification')) {
    return new Response(JSON.stringify({ success: true, data: { verified: true, results: [], count: 0 } }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  // Contract diff
  if (path.startsWith('/__mockforge/contract-diff')) {
    return new Response(JSON.stringify({ success: true, data: { statistics: { total_captures: 0, endpoints: [], method_distribution: {}, status_distribution: {} }, diffs: [] } }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  // Plugins status
  if (path.startsWith('/__mockforge/plugins')) {
    return new Response(JSON.stringify({ success: true, data: { plugins: [], total: 0 } }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  // Smoke tests
  if (path.startsWith('/__mockforge/smoke')) {
    return new Response(JSON.stringify({ success: true, data: [] }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  // Import
  if (path.startsWith('/__mockforge/import')) {
    return new Response(JSON.stringify({ success: true, data: { imports: [], total: 0 } }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  // Files
  if (path.startsWith('/__mockforge/files')) {
    return new Response(JSON.stringify({ success: true, data: { content: '', files: [] } }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  // Env
  if (path === '/__mockforge/env') {
    return new Response(JSON.stringify({ success: true, data: {} }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  // Fixtures bulk
  if (path.startsWith('/__mockforge/fixtures')) {
    return new Response(JSON.stringify({ success: true, data: [] }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  // Config sub-paths
  if (path.startsWith('/__mockforge/config/')) {
    return new Response(JSON.stringify({ success: true, data: {} }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  // Servers
  if (path.startsWith('/__mockforge/servers')) {
    return new Response(JSON.stringify({ success: true, data: { status: 'cloud', servers: [] } }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  // Generic fallback: return empty object (safer than array — pages using .property get undefined instead of crash)
  return new Response(JSON.stringify({ success: true, data: {} }),
    { status: 200, headers: { 'Content-Type': 'application/json' } });
}

// Local-only API paths that don't exist on the registry server.
// In cloud mode, return stubs instead of letting them 404.
// This list is a static fallback — the server also exposes GET /api/capabilities
// which the UI can query at startup to dynamically determine available features.
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
  '/api/v1/plugins/',
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

function createLocalApiStubResponse(url: string): Response {
  const path = new URL(url, window.location.origin).pathname;

  if (path.includes('/chaos/config')) {
    return new Response(JSON.stringify({
      latency: { enabled: false, fixed_delay_ms: 0, probability: 0 },
      fault_injection: { enabled: false, http_errors: [], http_error_probability: 0 },
      traffic_shaping: { enabled: false, bandwidth_limit_kbps: 0, packet_loss_rate: 0, corruption_rate: 0, corruption_type: 'none' },
    }), { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path.includes('/chaos/status')) {
    return new Response(JSON.stringify({ active: false, scenarios: [] }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path.includes('/chaos/scenarios')) {
    return new Response(JSON.stringify({ scenarios: [] }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path.includes('/chaos/profiles')) {
    return new Response(JSON.stringify([]),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path.includes('/observability/')) {
    return new Response(JSON.stringify({ data: [], stats: {} }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path.includes('/resilience/')) {
    return new Response(JSON.stringify([]),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path.includes('/recorder/')) {
    return new Response(JSON.stringify({ recordings: [], status: 'idle' }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path.includes('/conformance/')) {
    return new Response(JSON.stringify({ runs: [], total: 0 }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path.includes('/performance/')) {
    return new Response(JSON.stringify({ status: 'idle', profiles: [], results: [] }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path.includes('/world-state/')) {
    return new Response(JSON.stringify({ layers: [], graph: { nodes: [], edges: [] }, entities: [] }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path.includes('/consistency/')) {
    return new Response(JSON.stringify({ entities: [], total: 0 }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path.includes('/drift/')) {
    return new Response(JSON.stringify({ incidents: [], stats: { total: 0, open: 0, resolved: 0 }, total: 0 }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path.includes('/scenario-studio/')) {
    return new Response(JSON.stringify({ flows: [], total: 0 }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path.includes('/snapshots')) {
    return new Response(JSON.stringify({ snapshots: [], total: 0 }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path.includes('/analytics/')) {
    return new Response(JSON.stringify({ data: [], overview: { total_requests: 0, error_rate: 0, avg_latency: 0 }, traffic_patterns: [], requests: [], errors: [], latency: [] }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

  if (path.includes('/plugins/')) {
    return new Response(JSON.stringify({ plugins: [], total: 0 }),
      { status: 200, headers: { 'Content-Type': 'application/json' } });
  }

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
